use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};

use crate::AppState;
use crate::player::{Inventory, Player};
use crate::voxel::VoxelType;
use crate::world::{Chunk, ChunkCoord};

const SAVE_VERSION: u32 = 2;
const DEFAULT_WORLD_SEED: u32 = 12345;
const SAVE_MAGIC: &[u8; 4] = b"GECY";
const CHUNK_MAGIC: &[u8; 4] = b"GCHK";
pub const DEFAULT_SAVE_ROOT: &str = "saves/default_world";
pub const DEFAULT_WORLD_META_PATH: &str = "saves/default_world/world.meta";

#[derive(Clone)]
pub struct SavedChunk {
    pub coord: ChunkCoord,
    pub voxels: Vec<VoxelType>,
}

#[derive(Clone)]
struct WorldMetadata {
    version: u32,
    seed: u32,
    player_translation: [f32; 3],
    inventory: Vec<(VoxelType, u32)>,
}

#[derive(Resource)]
pub struct SaveState {
    pub root: PathBuf,
    pub version: u32,
    pub seed: u32,
    pub edited_chunks: HashMap<ChunkCoord, SavedChunk>,
    pub dirty_chunks: HashMap<ChunkCoord, SavedChunk>,
    pub loaded_player_translation: Vec3,
    pub loaded_inventory: Inventory,
    pub dirty: bool,
    pub pending_write: Option<Task<Result<(), String>>>,
}

impl Default for SaveState {
    fn default() -> Self {
        let root = PathBuf::from(DEFAULT_SAVE_ROOT);
        let (metadata, edited_chunks) = load_world_directory(&root).unwrap_or_else(|| {
            (
                WorldMetadata {
                    version: SAVE_VERSION,
                    seed: DEFAULT_WORLD_SEED,
                    player_translation: Vec3::ZERO.to_array(),
                    inventory: Inventory::default().entries(),
                },
                HashMap::default(),
            )
        });

        let mut inventory = Inventory::default();
        inventory.clear();
        for (voxel_type, count) in &metadata.inventory {
            inventory.add(*voxel_type, *count);
        }

        Self {
            root,
            version: metadata.version,
            seed: metadata.seed,
            edited_chunks,
            dirty_chunks: HashMap::default(),
            loaded_player_translation: Vec3::from_array(metadata.player_translation),
            loaded_inventory: inventory,
            dirty: false,
            pending_write: None,
        }
    }
}

impl SaveState {
    pub fn save_exists(&self) -> bool {
        self.meta_path().is_file()
    }

    pub fn initial_player_translation(&self) -> Option<Vec3> {
        if self.loaded_player_translation == Vec3::ZERO {
            None
        } else {
            Some(self.loaded_player_translation)
        }
    }

    pub fn start_new_world(&mut self) {
        self.version = SAVE_VERSION;
        self.seed = fresh_world_seed();
        self.edited_chunks.clear();
        self.dirty_chunks.clear();
        self.loaded_player_translation = Vec3::ZERO;
        self.loaded_inventory = Inventory::default();
        self.dirty = false;
        self.pending_write = None;
    }

    pub fn load_existing_world(&mut self) -> bool {
        let Some((metadata, edited_chunks)) = load_world_directory(&self.root) else {
            return false;
        };

        let mut inventory = Inventory::default();
        inventory.clear();
        for (voxel_type, count) in metadata.inventory {
            inventory.add(voxel_type, count);
        }

        self.version = metadata.version;
        self.seed = metadata.seed;
        self.edited_chunks = edited_chunks;
        self.dirty_chunks.clear();
        self.loaded_player_translation = Vec3::from_array(metadata.player_translation);
        self.loaded_inventory = inventory;
        self.dirty = false;
        self.pending_write = None;
        true
    }

    pub fn record_chunk_snapshot(&mut self, chunk: &Chunk) {
        if chunk.modified {
            let snapshot = SavedChunk {
                coord: chunk.coord,
                voxels: chunk.voxels.iter().map(|voxel| voxel.voxel_type).collect(),
            };
            self.edited_chunks.insert(chunk.coord, snapshot.clone());
            self.dirty_chunks.insert(chunk.coord, snapshot);
            self.dirty = true;
        }
    }

    fn meta_path(&self) -> PathBuf {
        self.root.join("world.meta")
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveState>()
            .add_systems(Update, process_pending_save_task.run_if(in_state(AppState::InGame)))
            .add_systems(Update, manual_save_input_system.run_if(in_state(AppState::InGame)));
    }
}

pub fn load_inventory_from_save(save_state: &SaveState) -> Inventory {
    save_state.loaded_inventory.clone()
}

fn load_world_directory(root: &Path) -> Option<(WorldMetadata, HashMap<ChunkCoord, SavedChunk>)> {
    let metadata = read_world_metadata(&root.join("world.meta")).ok()?;
    let mut chunks = HashMap::default();
    let chunks_dir = root.join("chunks");
    if chunks_dir.is_dir() {
        for entry in fs::read_dir(chunks_dir).ok()? {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let chunk = read_chunk_file(&path).ok()?;
            chunks.insert(chunk.coord, chunk);
        }
    }

    Some((metadata, chunks))
}

fn process_pending_save_task(mut save_state: ResMut<SaveState>) {
    let Some(task) = save_state.pending_write.as_mut() else {
        return;
    };

    if let Some(result) = future::block_on(future::poll_once(task)) {
        match result {
            Ok(()) => {
                save_state.dirty = false;
                save_state.dirty_chunks.clear();
            }
            Err(error) => warn!("Failed to save world: {error}"),
        }
        save_state.pending_write = None;
    }
}

fn manual_save_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut save_state: ResMut<SaveState>,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<&Chunk>,
    inventory: Res<Inventory>,
) {
    if !keyboard_input.just_pressed(KeyCode::F5) || save_state.pending_write.is_some() {
        return;
    }

    queue_manual_save(&mut save_state, &player_query, &chunk_query, &inventory);
}

fn build_save_snapshot(
    save_state: &SaveState,
    player_query: &Query<&Transform, With<Player>>,
    chunk_query: &Query<&Chunk>,
    inventory: &Inventory,
) -> (WorldMetadata, Vec<SavedChunk>) {
    let player_translation = player_query
        .single()
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::ZERO);

    let mut dirty_chunks = save_state.dirty_chunks.values().cloned().collect::<Vec<_>>();
    for chunk in chunk_query.iter() {
        if chunk.modified {
            dirty_chunks.retain(|saved| saved.coord != chunk.coord);
            dirty_chunks.push(SavedChunk {
                coord: chunk.coord,
                voxels: chunk.voxels.iter().map(|voxel| voxel.voxel_type).collect(),
            });
        }
    }

    dirty_chunks.sort_by_key(|chunk| (chunk.coord.x, chunk.coord.z));

    (
        WorldMetadata {
            version: save_state.version,
            seed: save_state.seed,
            player_translation: player_translation.to_array(),
            inventory: inventory.entries(),
        },
        dirty_chunks,
    )
}

pub fn queue_manual_save(
    save_state: &mut SaveState,
    player_query: &Query<&Transform, With<Player>>,
    chunk_query: &Query<&Chunk>,
    inventory: &Inventory,
) {
    let root = save_state.root.clone();
    let snapshot = build_save_snapshot(save_state, player_query, chunk_query, inventory);
    let task_pool = AsyncComputeTaskPool::get();
    save_state.pending_write = Some(task_pool.spawn(async move {
        write_world_directory(&root, snapshot.0, snapshot.1)
    }));
}

fn write_world_directory(
    root: &Path,
    metadata: WorldMetadata,
    dirty_chunks: Vec<SavedChunk>,
) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let chunks_dir = root.join("chunks");
    fs::create_dir_all(&chunks_dir).map_err(|error| error.to_string())?;

    write_world_metadata(&root.join("world.meta"), &metadata)?;

    for chunk in dirty_chunks {
        let chunk_path = chunk_file_path(&chunks_dir, chunk.coord);
        write_chunk_file(&chunk_path, &chunk)?;
    }

    Ok(())
}

fn chunk_file_path(chunks_dir: &Path, coord: ChunkCoord) -> PathBuf {
    chunks_dir.join(format!("chunk_{}_{}.bin", coord.x, coord.z))
}

fn write_world_metadata(path: &Path, metadata: &WorldMetadata) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend_from_slice(SAVE_MAGIC);
    bytes.extend_from_slice(&metadata.version.to_le_bytes());
    bytes.extend_from_slice(&metadata.seed.to_le_bytes());
    for value in metadata.player_translation {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let inventory_len: u32 = metadata
        .inventory
        .len()
        .try_into()
        .map_err(|_| "Inventory is too large to save".to_string())?;
    bytes.extend_from_slice(&inventory_len.to_le_bytes());
    for (voxel_type, count) in &metadata.inventory {
        bytes.push(voxel_type_to_u8(*voxel_type));
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn read_world_metadata(path: &Path) -> Result<WorldMetadata, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut cursor = Cursor::new(bytes);

    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic).map_err(|error| error.to_string())?;
    if &magic != SAVE_MAGIC {
        return Err("Invalid world metadata header".to_string());
    }

    let version = read_u32(&mut cursor)?;
    if version != SAVE_VERSION {
        return Err(format!("Unsupported save version: {version}"));
    }

    let seed = read_u32(&mut cursor)?;
    let mut player_translation = [0.0; 3];
    for value in &mut player_translation {
        *value = read_f32(&mut cursor)?;
    }

    let inventory_len = read_u32(&mut cursor)? as usize;
    let mut inventory = Vec::with_capacity(inventory_len);
    for _ in 0..inventory_len {
        let voxel_type = voxel_type_from_u8(read_u8(&mut cursor)?)?;
        let count = read_u32(&mut cursor)?;
        inventory.push((voxel_type, count));
    }

    Ok(WorldMetadata {
        version,
        seed,
        player_translation,
        inventory,
    })
}

fn write_chunk_file(path: &Path, chunk: &SavedChunk) -> Result<(), String> {
    let mut bytes = Vec::with_capacity(32 + chunk.voxels.len());
    bytes.extend_from_slice(CHUNK_MAGIC);
    bytes.extend_from_slice(&chunk.coord.x.to_le_bytes());
    bytes.extend_from_slice(&chunk.coord.z.to_le_bytes());
    let voxel_count: u32 = chunk
        .voxels
        .len()
        .try_into()
        .map_err(|_| "Chunk voxel count exceeds u32".to_string())?;
    bytes.extend_from_slice(&voxel_count.to_le_bytes());
    bytes.extend(chunk.voxels.iter().map(|voxel| voxel_type_to_u8(*voxel)));
    fs::write(path, bytes).map_err(|error| error.to_string())
}

fn read_chunk_file(path: &Path) -> Result<SavedChunk, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let mut cursor = Cursor::new(bytes);

    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic).map_err(|error| error.to_string())?;
    if &magic != CHUNK_MAGIC {
        return Err(format!("Invalid chunk header in {}", path.display()));
    }

    let x = read_i32(&mut cursor)?;
    let z = read_i32(&mut cursor)?;
    let voxel_count = read_u32(&mut cursor)? as usize;
    let mut voxels = Vec::with_capacity(voxel_count);
    for _ in 0..voxel_count {
        voxels.push(voxel_type_from_u8(read_u8(&mut cursor)?)?);
    }

    Ok(SavedChunk {
        coord: ChunkCoord::new(x, z),
        voxels,
    })
}

fn voxel_type_to_u8(voxel_type: VoxelType) -> u8 {
    match voxel_type {
        VoxelType::Air => 0,
        VoxelType::Stone => 1,
        VoxelType::Dirt => 2,
        VoxelType::Grass => 3,
    }
}

fn voxel_type_from_u8(value: u8) -> Result<VoxelType, String> {
    match value {
        0 => Ok(VoxelType::Air),
        1 => Ok(VoxelType::Stone),
        2 => Ok(VoxelType::Dirt),
        3 => Ok(VoxelType::Grass),
        _ => Err(format!("Unknown voxel type id: {value}")),
    }
}

fn read_u8(cursor: &mut Cursor<Vec<u8>>) -> Result<u8, String> {
    let mut bytes = [0u8; 1];
    cursor.read_exact(&mut bytes).map_err(|error| error.to_string())?;
    Ok(bytes[0])
}

fn read_u32(cursor: &mut Cursor<Vec<u8>>) -> Result<u32, String> {
    let mut bytes = [0u8; 4];
    cursor.read_exact(&mut bytes).map_err(|error| error.to_string())?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i32(cursor: &mut Cursor<Vec<u8>>) -> Result<i32, String> {
    let mut bytes = [0u8; 4];
    cursor.read_exact(&mut bytes).map_err(|error| error.to_string())?;
    Ok(i32::from_le_bytes(bytes))
}

fn read_f32(cursor: &mut Cursor<Vec<u8>>) -> Result<f32, String> {
    let mut bytes = [0u8; 4];
    cursor.read_exact(&mut bytes).map_err(|error| error.to_string())?;
    Ok(f32::from_le_bytes(bytes))
}

fn fresh_world_seed() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as u32 ^ duration.subsec_nanos())
        .unwrap_or(DEFAULT_WORLD_SEED)
}
