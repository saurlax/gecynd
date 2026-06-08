use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use serde::{Deserialize, Serialize};

use crate::player::{
    EditMode, EditRequest, Inventory, NeedsPhysicsRefresh, NeedsRenderRefresh, Player, spawn_player,
};
use crate::save::{SaveState, SavedChunk};
use crate::terrain::{TERRAIN_MAX_HEIGHT_METERS, TerrainGenerator};
use crate::voxel::{VOXEL_SIZE, Voxel, VoxelType};
use crate::AppState;

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_HEIGHT: usize = 256;
pub const VISIBLE_RADIUS_METERS: f32 = 16.0;
pub const INITIAL_LOAD_RADIUS_CHUNKS: i32 = 3;

pub const CHUNK_VOXELS_SIZE: usize = CHUNK_SIZE;
pub const CHUNK_VOXELS_HEIGHT: usize = CHUNK_HEIGHT;
const PLAYER_SPAWN_CLEARANCE_METERS: f32 = 3.0;

pub fn chunk_world_size() -> f32 {
    CHUNK_VOXELS_SIZE as f32 * VOXEL_SIZE
}

pub fn chunk_world_height() -> f32 {
    CHUNK_VOXELS_HEIGHT as f32 * VOXEL_SIZE
}

pub fn chunk_world_origin(coord: ChunkCoord) -> Vec3 {
    Vec3::new(
        coord.x as f32 * chunk_world_size(),
        0.0,
        coord.z as f32 * chunk_world_size(),
    )
}

pub fn render_distance_chunks() -> i32 {
    ((VISIBLE_RADIUS_METERS / chunk_world_size()).ceil() as i32).max(1)
}

pub fn initial_player_spawn_position() -> Vec3 {
    Vec3::new(
        chunk_world_size() * 0.25,
        TERRAIN_MAX_HEIGHT_METERS + PLAYER_SPAWN_CLEARANCE_METERS,
        chunk_world_size() * 0.25,
    )
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DebugViewMode {
    pub render_wireframe: bool,
    pub physics_wireframe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn from_world_pos(world_pos: Vec3) -> Self {
        let chunk_size_world = chunk_world_size();
        Self {
            x: (world_pos.x / chunk_size_world).floor() as i32,
            z: (world_pos.z / chunk_size_world).floor() as i32,
        }
    }
}

#[derive(Component, Clone)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub voxels: Vec<Voxel>,
    pub revision: u64,
    pub modified: bool,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        Self {
            coord,
            voxels: vec![
                Voxel::default();
                CHUNK_VOXELS_SIZE * CHUNK_VOXELS_HEIGHT * CHUNK_VOXELS_SIZE
            ],
            revision: 0,
            modified: false,
        }
    }

    fn voxel_index(x: usize, y: usize, z: usize) -> usize {
        x + z * CHUNK_VOXELS_SIZE + y * CHUNK_VOXELS_SIZE * CHUNK_VOXELS_SIZE
    }

    pub fn get_voxel(&self, x: usize, y: usize, z: usize) -> Option<&Voxel> {
        if x < CHUNK_VOXELS_SIZE && y < CHUNK_VOXELS_HEIGHT && z < CHUNK_VOXELS_SIZE {
            Some(&self.voxels[Self::voxel_index(x, y, z)])
        } else {
            None
        }
    }

    pub fn set_voxel(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        if x < CHUNK_VOXELS_SIZE && y < CHUNK_VOXELS_HEIGHT && z < CHUNK_VOXELS_SIZE {
            let index = Self::voxel_index(x, y, z);
            if self.voxels[index] != voxel {
                self.voxels[index] = voxel;
                self.revision += 1;
                self.modified = true;
            }
        }
    }
}

#[derive(Resource)]
pub struct World {
    pub chunks: HashMap<ChunkCoord, Entity>,
    pub pending_chunks: HashMap<ChunkCoord, Task<Chunk>>,
}

impl Default for World {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            pending_chunks: HashMap::new(),
        }
    }
}

#[derive(Resource)]
pub struct InitialWorldGeneration {
    pub started: bool,
    pub finished: bool,
    pub task: Option<Task<Vec<Chunk>>>,
}

impl Default for InitialWorldGeneration {
    fn default() -> Self {
        Self {
            started: false,
            finished: false,
            task: None,
        }
    }
}

impl World {
    /// Converts world coordinates to chunk coordinates and voxel indices.
    pub fn world_to_voxel(&self, world_pos: Vec3) -> Option<(ChunkCoord, usize, usize, usize)> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);

        if !self.chunks.contains_key(&chunk_coord) {
            return None;
        }

        let chunk_origin = chunk_world_origin(chunk_coord);

        let local_x = world_pos.x - chunk_origin.x;
        let local_y = world_pos.y;
        let local_z = world_pos.z - chunk_origin.z;

        if local_x < 0.0 || local_y < 0.0 || local_z < 0.0 {
            return None;
        }

        let voxel_x = (local_x / VOXEL_SIZE).floor() as usize;
        let voxel_y = (local_y / VOXEL_SIZE).floor() as usize;
        let voxel_z = (local_z / VOXEL_SIZE).floor() as usize;

        if voxel_x < CHUNK_VOXELS_SIZE
            && voxel_y < CHUNK_VOXELS_HEIGHT
            && voxel_z < CHUNK_VOXELS_SIZE
        {
            Some((chunk_coord, voxel_x, voxel_y, voxel_z))
        } else {
            None
        }
    }

    /// Returns the voxel at a world position.
    pub fn get_voxel_at_world(
        &self,
        world_pos: Vec3,
        chunk_query: &Query<&Chunk>,
    ) -> Option<Voxel> {
        if let Some((chunk_coord, x, y, z)) = self.world_to_voxel(world_pos) {
            if let Some(chunk_entity) = self.chunks.get(&chunk_coord) {
                if let Ok(chunk) = chunk_query.get(*chunk_entity) {
                    return chunk.get_voxel(x, y, z).copied();
                }
            }
        }
        None
    }

    /// Sets the voxel at a world position.
    pub fn set_voxel_at_world(
        &self,
        world_pos: Vec3,
        voxel: Voxel,
        chunk_query: &mut Query<&mut Chunk>,
    ) -> bool {
        if let Some((chunk_coord, x, y, z)) = self.world_to_voxel(world_pos) {
            if let Some(chunk_entity) = self.chunks.get(&chunk_coord) {
                if let Ok(mut chunk) = chunk_query.get_mut(*chunk_entity) {
                    chunk.set_voxel(x, y, z, voxel);
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Resource)]
pub struct DebugAabbState {
    pub enabled: bool,
}

impl Default for DebugAabbState {
    fn default() -> Self {
        Self { enabled: false }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<World>()
            .init_resource::<DebugAabbState>()
            .init_resource::<DebugViewMode>()
            .init_resource::<InitialWorldGeneration>()
            .add_message::<EditRequest>()
            .add_systems(OnEnter(AppState::InGame), prepare_world_session)
            .add_systems(OnEnter(AppState::InGame), start_initial_world_generation)
            .add_systems(
                Update,
                (
                    complete_initial_world_generation,
                    complete_pending_chunk_generation_system,
                    chunk_loading_system,
                    chunk_unloading_system,
                    apply_edit_requests_system,
                    debug_view_mode_system,
                    debug_state_system,
                )
                    .run_if(in_state(AppState::InGame)),
            );
    }
}

fn prepare_world_session(
    mut world: ResMut<World>,
    mut generation_state: ResMut<InitialWorldGeneration>,
) {
    world.chunks.clear();
    world.pending_chunks.clear();
    generation_state.started = false;
    generation_state.finished = false;
    generation_state.task = None;
}

fn queue_chunk_generation(
    world: &mut World,
    coord: ChunkCoord,
    saved_chunks: HashMap<ChunkCoord, SavedChunk>,
    seed: u32,
) {
    if world.chunks.contains_key(&coord) || world.pending_chunks.contains_key(&coord) {
        return;
    }

    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move {
        let terrain_generator = TerrainGenerator::new(seed);
        let mut chunk = Chunk::new(coord);
        terrain_generator.generate_chunk(&mut chunk);
        if let Some(saved_chunk) = saved_chunks.get(&coord) {
            for (index, voxel_type) in saved_chunk.voxels.iter().copied().enumerate() {
                if index < chunk.voxels.len() {
                    chunk.voxels[index] = Voxel::new(voxel_type);
                }
            }
            chunk.modified = true;
        }
        chunk
    });

    world.pending_chunks.insert(coord, task);
}

fn start_initial_world_generation(
    mut generation_state: ResMut<InitialWorldGeneration>,
    save_state: Res<SaveState>,
) {
    if generation_state.started {
        return;
    }

    generation_state.started = true;
    let spawn_position = save_state
        .initial_player_translation()
        .unwrap_or_else(initial_player_spawn_position);
    let spawn_chunk = ChunkCoord::from_world_pos(spawn_position);
    let seed = save_state.seed;
    let saved_chunks = save_state.edited_chunks.clone();
    let task_pool = AsyncComputeTaskPool::get();

    generation_state.task = Some(task_pool.spawn(async move {
        let terrain_generator = TerrainGenerator::new(seed);
        let mut chunks = Vec::new();

        for x in (spawn_chunk.x - INITIAL_LOAD_RADIUS_CHUNKS)
            ..=(spawn_chunk.x + INITIAL_LOAD_RADIUS_CHUNKS)
        {
            for z in (spawn_chunk.z - INITIAL_LOAD_RADIUS_CHUNKS)
                ..=(spawn_chunk.z + INITIAL_LOAD_RADIUS_CHUNKS)
            {
                let dx = x - spawn_chunk.x;
                let dz = z - spawn_chunk.z;
                if dx * dx + dz * dz > INITIAL_LOAD_RADIUS_CHUNKS * INITIAL_LOAD_RADIUS_CHUNKS {
                    continue;
                }

                let coord = ChunkCoord::new(x, z);
                let mut chunk = Chunk::new(coord);
                terrain_generator.generate_chunk(&mut chunk);
                if let Some(saved_chunk) = saved_chunks.get(&coord) {
                    for (index, voxel_type) in saved_chunk.voxels.iter().copied().enumerate() {
                        if index < chunk.voxels.len() {
                            chunk.voxels[index] = Voxel::new(voxel_type);
                        }
                    }
                    chunk.modified = true;
                }
                chunks.push(chunk);
            }
        }

        chunks
    }));
}

fn complete_initial_world_generation(
    mut commands: Commands,
    mut world: ResMut<World>,
    mut generation_state: ResMut<InitialWorldGeneration>,
    save_state: Res<SaveState>,
    mut inventory: ResMut<Inventory>,
) {
    if generation_state.finished {
        return;
    }

    let Some(task) = generation_state.task.as_mut() else {
        return;
    };
    let Some(chunks) = future::block_on(future::poll_once(task)) else {
        return;
    };
    generation_state.task = None;

    for chunk in chunks {
        let coord = chunk.coord;
        let entity = commands
            .spawn((chunk, NeedsRenderRefresh, NeedsPhysicsRefresh))
            .id();
        world.chunks.insert(coord, entity);
    }

    *inventory = crate::save::load_inventory_from_save(&save_state);
    spawn_player(
        &mut commands,
        save_state
            .initial_player_translation()
            .unwrap_or_else(initial_player_spawn_position),
    );
    generation_state.finished = true;
}

fn complete_pending_chunk_generation_system(
    mut commands: Commands,
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
) {
    let mut ready_chunks: Vec<(ChunkCoord, Chunk)> = world
        .pending_chunks
        .iter_mut()
        .filter_map(|(coord, task)| {
            future::block_on(future::poll_once(task)).map(|chunk| (*coord, chunk))
        })
        .collect();

    if let Ok(player_transform) = player_query.single() {
        let player_chunk = ChunkCoord::from_world_pos(player_transform.translation);
        ready_chunks.sort_by_key(|(coord, _)| {
            let dx = coord.x - player_chunk.x;
            let dz = coord.z - player_chunk.z;
            dx * dx + dz * dz
        });
    }

    for (coord, chunk) in ready_chunks {
        world.pending_chunks.remove(&coord);
        let entity = commands
            .spawn((chunk, NeedsRenderRefresh, NeedsPhysicsRefresh))
            .id();
        world.chunks.insert(coord, entity);
    }
}

fn chunk_loading_system(
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
    save_state: Res<SaveState>,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_chunk = ChunkCoord::from_world_pos(player_transform.translation);
        let render_distance = render_distance_chunks();

        for x in (player_chunk.x - render_distance)..=(player_chunk.x + render_distance) {
            for z in (player_chunk.z - render_distance)..=(player_chunk.z + render_distance) {
                let dx = x - player_chunk.x;
                let dz = z - player_chunk.z;
                if dx * dx + dz * dz > render_distance * render_distance {
                    continue;
                }

                let coord = ChunkCoord::new(x, z);
                let saved_chunks = save_state
                    .edited_chunks
                    .get(&coord)
                    .cloned()
                    .map(|chunk| HashMap::from([(coord, chunk)]))
                    .unwrap_or_default();
                queue_chunk_generation(
                    &mut world,
                    coord,
                    saved_chunks,
                    save_state.seed,
                );
            }
        }
    }
}

fn chunk_unloading_system(
    mut commands: Commands,
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<&Chunk>,
    mut save_state: ResMut<SaveState>,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_chunk = ChunkCoord::from_world_pos(player_transform.translation);
        let unload_distance = render_distance_chunks() + 2;

        let mut chunks_to_unload = Vec::new();

        for (&chunk_coord, &chunk_entity) in world.chunks.iter() {
            let distance_x = (chunk_coord.x - player_chunk.x).abs();
            let distance_z = (chunk_coord.z - player_chunk.z).abs();

            if distance_x * distance_x + distance_z * distance_z > unload_distance * unload_distance
            {
                chunks_to_unload.push((chunk_coord, chunk_entity));
            }
        }

        for (coord, entity) in chunks_to_unload {
            if let Ok(chunk) = chunk_query.get(entity) {
                save_state.record_chunk_snapshot(chunk);
            }
            commands.entity(entity).despawn();
            world.chunks.remove(&coord);
        }
    }
}

fn apply_edit_requests_system(
    mut commands: Commands,
    world: Res<World>,
    mut chunk_query: Query<&mut Chunk>,
    mut edit_requests: MessageReader<EditRequest>,
    mut save_state: ResMut<SaveState>,
    mut inventory: ResMut<Inventory>,
) {
    for request in edit_requests.read() {
        let changed_positions = apply_edit_request(&world, &mut chunk_query, &mut inventory, request);
        for world_pos in changed_positions {
            mark_chunk_for_update(&mut commands, &world, world_pos);
        }
        if !request.positions.is_empty() {
            save_state.dirty = true;
        }
    }
}

fn apply_edit_request(
    world: &World,
    chunk_query: &mut Query<&mut Chunk>,
    inventory: &mut Inventory,
    request: &EditRequest,
) -> Vec<Vec3> {
    let mut changed_positions = Vec::new();

    for operation in &request.operations {
        match operation.mode {
            EditMode::Place => {
                if operation.voxel_type == VoxelType::Air {
                    continue;
                }

                if !inventory.try_remove(operation.voxel_type, 1) {
                    continue;
                }

                if world.set_voxel_at_world(
                    operation.position,
                    Voxel::new(operation.voxel_type),
                    chunk_query,
                ) {
                    changed_positions.push(operation.position);
                } else {
                    inventory.add(operation.voxel_type, 1);
                }
            }
            EditMode::Break => {
                if let Some(previous) = world.get_voxel_at_world(operation.position, &chunk_query.as_readonly()) {
                    if previous.is_solid()
                        && world.set_voxel_at_world(operation.position, Voxel::new(VoxelType::Air), chunk_query)
                    {
                        inventory.add(previous.voxel_type, 1);
                        changed_positions.push(operation.position);
                    }
                }
            }
        }
    }

    changed_positions
}

pub fn mark_chunk_for_update(commands: &mut Commands, world: &World, world_pos: Vec3) {
    if let Some((chunk_coord, voxel_x, _, voxel_z)) = world.world_to_voxel(world_pos) {
        let mut dirty_chunks = bevy::platform::collections::HashSet::from([chunk_coord]);

        if voxel_x == 0 {
            dirty_chunks.insert(ChunkCoord::new(chunk_coord.x - 1, chunk_coord.z));
        }
        if voxel_x + 1 == CHUNK_VOXELS_SIZE {
            dirty_chunks.insert(ChunkCoord::new(chunk_coord.x + 1, chunk_coord.z));
        }
        if voxel_z == 0 {
            dirty_chunks.insert(ChunkCoord::new(chunk_coord.x, chunk_coord.z - 1));
        }
        if voxel_z + 1 == CHUNK_VOXELS_SIZE {
            dirty_chunks.insert(ChunkCoord::new(chunk_coord.x, chunk_coord.z + 1));
        }

        for dirty_chunk in dirty_chunks {
            if let Some(chunk_entity) = world.chunks.get(&dirty_chunk) {
                commands
                    .entity(*chunk_entity)
                    .insert((NeedsRenderRefresh, NeedsPhysicsRefresh));
            }
        }
    }
}

fn debug_state_system(mut debug_state: ResMut<DebugAabbState>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F1) {
        debug_state.enabled = !debug_state.enabled;
    }
}

fn debug_view_mode_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut debug_view_mode: ResMut<DebugViewMode>,
) {
    if keys.just_pressed(KeyCode::F2) {
        debug_view_mode.render_wireframe = !debug_view_mode.render_wireframe;
    }

    if keys.just_pressed(KeyCode::F3) {
        debug_view_mode.physics_wireframe = !debug_view_mode.physics_wireframe;
    }
}
