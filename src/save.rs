use std::fs;
use std::path::PathBuf;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::player::{Inventory, Player};
use crate::voxel::VoxelType;
use crate::world::{Chunk, ChunkCoord};

const SAVE_VERSION: u32 = 1;
const DEFAULT_WORLD_SEED: u32 = 12345;

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedChunk {
    pub coord: ChunkCoord,
    pub voxels: Vec<VoxelType>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedPlayer {
    pub translation: [f32; 3],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedInventoryEntry {
    pub voxel_type: VoxelType,
    pub count: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WorldSaveFile {
    pub version: u32,
    pub seed: u32,
    pub player: SavedPlayer,
    pub inventory: Vec<SavedInventoryEntry>,
    pub edited_chunks: Vec<SavedChunk>,
}

#[derive(Resource)]
pub struct SaveState {
    pub path: PathBuf,
    pub version: u32,
    pub seed: u32,
    pub edited_chunks: HashMap<ChunkCoord, SavedChunk>,
    pub loaded_player_translation: Vec3,
    pub dirty: bool,
    pub autosave_timer: Timer,
}

impl Default for SaveState {
    fn default() -> Self {
        let path = PathBuf::from("saves/world.json");
        let loaded = load_world_save(&path);

        Self {
            path,
            version: SAVE_VERSION,
            seed: loaded.as_ref().map_or(DEFAULT_WORLD_SEED, |save| save.version_seed().1),
            edited_chunks: loaded
                .as_ref()
                .map(|save| {
                    save.edited_chunks
                        .iter()
                        .cloned()
                        .map(|chunk| (chunk.coord, chunk))
                        .collect()
                })
                .unwrap_or_default(),
            loaded_player_translation: loaded
                .as_ref()
                .map(WorldSaveFile::player_translation)
                .unwrap_or(Vec3::ZERO),
            dirty: false,
            autosave_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        }
    }
}

impl SaveState {
    pub fn initial_player_translation(&self) -> Option<Vec3> {
        if self.loaded_player_translation == Vec3::ZERO {
            None
        } else {
            Some(self.loaded_player_translation)
        }
    }

    pub fn record_chunk_snapshot(&mut self, chunk: &Chunk) {
        if chunk.modified {
            self.edited_chunks.insert(
                chunk.coord,
                SavedChunk {
                    coord: chunk.coord,
                    voxels: chunk.voxels.iter().map(|voxel| voxel.voxel_type).collect(),
                },
            );
            self.dirty = true;
        }
    }
}

impl WorldSaveFile {
    fn player_translation(&self) -> Vec3 {
        Vec3::from_array(self.player.translation)
    }

    fn version_seed(&self) -> (u32, u32) {
        (self.version, self.seed)
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveState>()
            .add_systems(Update, autosave_world_system);
    }
}

pub fn load_inventory_from_save(save_state: &SaveState) -> Inventory {
    let Some(save_file) = load_world_save(&save_state.path) else {
        return Inventory::default();
    };

    let mut inventory = Inventory::default();
    for entry in save_file.inventory {
        inventory.add(entry.voxel_type, entry.count);
    }

    inventory
}

fn load_world_save(path: &PathBuf) -> Option<WorldSaveFile> {
    let bytes = fs::read(path).ok()?;
    let save = serde_json::from_slice::<WorldSaveFile>(&bytes).ok()?;
    if save.version != SAVE_VERSION {
        warn!(
            "Unsupported save version {} in {:?}; starting a new world.",
            save.version, path
        );
        return None;
    }

    Some(save)
}

fn autosave_world_system(
    time: Res<Time>,
    mut save_state: ResMut<SaveState>,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<&Chunk>,
    inventory: Res<Inventory>,
) {
    save_state.autosave_timer.tick(time.delta());
    if !save_state.dirty || !save_state.autosave_timer.just_finished() {
        return;
    }

    if let Err(error) = write_world_save(&save_state, &player_query, &chunk_query, &inventory) {
        warn!("Failed to save world: {error}");
    } else {
        save_state.dirty = false;
    }
}

pub fn write_world_save(
    save_state: &SaveState,
    player_query: &Query<&Transform, With<Player>>,
    chunk_query: &Query<&Chunk>,
    inventory: &Inventory,
) -> Result<(), String> {
    if let Some(parent) = save_state.path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let player_translation = player_query
        .single()
        .map(|transform| transform.translation)
        .unwrap_or(Vec3::ZERO);

    let mut edited_chunks: Vec<SavedChunk> = save_state.edited_chunks.values().cloned().collect();
    for chunk in chunk_query.iter() {
        if chunk.modified {
            edited_chunks.retain(|saved| saved.coord != chunk.coord);
            edited_chunks.push(SavedChunk {
                coord: chunk.coord,
                voxels: chunk.voxels.iter().map(|voxel| voxel.voxel_type).collect(),
            });
        }
    }

    edited_chunks.sort_by_key(|chunk| (chunk.coord.x, chunk.coord.z));

    let save_file = WorldSaveFile {
        version: save_state.version,
        seed: save_state.seed,
        player: SavedPlayer {
            translation: player_translation.to_array(),
        },
        inventory: inventory
            .entries()
            .into_iter()
            .map(|(voxel_type, count)| SavedInventoryEntry { voxel_type, count })
            .collect(),
        edited_chunks,
    };

    let payload = serde_json::to_vec_pretty(&save_file).map_err(|error| error.to_string())?;
    fs::write(&save_state.path, payload).map_err(|error| error.to_string())?;
    Ok(())
}
