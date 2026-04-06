use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

use crate::voxel::{Voxel, VOXEL_SIZE};
use crate::terrain::TerrainGenerator;
use crate::player::{spawn_player, Player};

pub const CHUNK_SIZE: usize = 32;
pub const CHUNK_HEIGHT: usize = 256;
pub const VISIBLE_RADIUS_METERS: f32 = 16.0;
pub const INITIAL_LOAD_RADIUS_CHUNKS: i32 = 3;

pub const CHUNK_VOXELS_SIZE: usize = CHUNK_SIZE;
pub const CHUNK_VOXELS_HEIGHT: usize = CHUNK_HEIGHT;

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
    Vec3::new(8.0, chunk_world_height() + 2.0, 8.0)
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DebugViewMode {
    pub render_wireframe: bool,
    pub physics_wireframe: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

#[derive(Component)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub voxels: Vec<Vec<Vec<Voxel>>>,
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        let mut voxels = Vec::with_capacity(CHUNK_VOXELS_SIZE);
        for _ in 0..CHUNK_VOXELS_SIZE {
            let mut y_vec = Vec::with_capacity(CHUNK_VOXELS_HEIGHT);
            for _ in 0..CHUNK_VOXELS_HEIGHT {
                let z_vec = vec![Voxel::default(); CHUNK_VOXELS_SIZE];
                y_vec.push(z_vec);
            }
            voxels.push(y_vec);
        }
        
        Self {
            coord,
            voxels,
        }
    }
    
    pub fn get_voxel(&self, x: usize, y: usize, z: usize) -> Option<&Voxel> {
        if x < CHUNK_VOXELS_SIZE && y < CHUNK_VOXELS_HEIGHT && z < CHUNK_VOXELS_SIZE {
            Some(&self.voxels[x][y][z])
        } else {
            None
        }
    }
    
    pub fn set_voxel(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) {
        if x < CHUNK_VOXELS_SIZE && y < CHUNK_VOXELS_HEIGHT && z < CHUNK_VOXELS_SIZE {
            self.voxels[x][y][z] = voxel;
        }
    }
    
}

#[derive(Resource)]
pub struct World {
    pub chunks: HashMap<ChunkCoord, Entity>,
    pub pending_chunks: HashMap<ChunkCoord, Arc<Mutex<Option<Chunk>>>>,
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
    pub result: Arc<Mutex<Option<Vec<Chunk>>>>,
}

impl Default for InitialWorldGeneration {
    fn default() -> Self {
        Self {
            started: false,
            finished: false,
            result: Arc::new(Mutex::new(None)),
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
        
        if voxel_x < CHUNK_VOXELS_SIZE && 
           voxel_y < CHUNK_VOXELS_HEIGHT && 
           voxel_z < CHUNK_VOXELS_SIZE {
            Some((chunk_coord, voxel_x, voxel_y, voxel_z))
        } else {
            None
        }
    }
    
    /// Returns the voxel at a world position.
    pub fn get_voxel_at_world(&self, world_pos: Vec3, chunk_query: &Query<&Chunk>) -> Option<Voxel> {
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
        chunk_query: &mut Query<&mut Chunk>
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
    
    /// Returns the world-space center of the voxel at a world position.
    pub fn get_voxel_center_at_world(&self, world_pos: Vec3) -> Option<Vec3> {
        if let Some((chunk_coord, x, y, z)) = self.world_to_voxel(world_pos) {
            if let Some(_chunk_entity) = self.chunks.get(&chunk_coord) {
                let chunk_origin = chunk_world_origin(chunk_coord);
                
                return Some(Vec3::new(
                    chunk_origin.x + x as f32 * VOXEL_SIZE + VOXEL_SIZE / 2.0,
                    y as f32 * VOXEL_SIZE + VOXEL_SIZE / 2.0,
                    chunk_origin.z + z as f32 * VOXEL_SIZE + VOXEL_SIZE / 2.0,
                ));
            }
        }
        None
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
        app
            .init_resource::<World>()
            .init_resource::<DebugAabbState>()
            .init_resource::<DebugViewMode>()
            .init_resource::<InitialWorldGeneration>()
            .add_systems(Startup, start_initial_world_generation)
            .add_systems(
                Update,
                (
                    complete_initial_world_generation,
                    complete_pending_chunk_generation_system,
                    chunk_loading_system,
                    chunk_unloading_system,
                    debug_view_mode_system,
                    debug_state_system,
                ),
            );
    }
}

fn queue_chunk_generation(world: &mut World, coord: ChunkCoord) {
    if world.chunks.contains_key(&coord) || world.pending_chunks.contains_key(&coord) {
        return;
    }

    let result = Arc::new(Mutex::new(None));
    world.pending_chunks.insert(coord, Arc::clone(&result));

    std::thread::spawn(move || {
        let terrain_generator = TerrainGenerator::new();
        let mut chunk = Chunk::new(coord);
        terrain_generator.generate_chunk(&mut chunk);

        if let Ok(mut guard) = result.lock() {
            *guard = Some(chunk);
        }
    });
}

fn start_initial_world_generation(mut generation_state: ResMut<InitialWorldGeneration>) {
    if generation_state.started {
        return;
    }

    generation_state.started = true;
    let result = Arc::clone(&generation_state.result);
    let spawn_chunk = ChunkCoord::from_world_pos(initial_player_spawn_position());

    std::thread::spawn(move || {
        let terrain_generator = TerrainGenerator::new();
        let mut chunks = Vec::new();

        for x in (spawn_chunk.x - INITIAL_LOAD_RADIUS_CHUNKS)..=(spawn_chunk.x + INITIAL_LOAD_RADIUS_CHUNKS) {
            for z in (spawn_chunk.z - INITIAL_LOAD_RADIUS_CHUNKS)..=(spawn_chunk.z + INITIAL_LOAD_RADIUS_CHUNKS) {
                let dx = x - spawn_chunk.x;
                let dz = z - spawn_chunk.z;
                if dx * dx + dz * dz > INITIAL_LOAD_RADIUS_CHUNKS * INITIAL_LOAD_RADIUS_CHUNKS {
                    continue;
                }

                let coord = ChunkCoord::new(x, z);
                let mut chunk = Chunk::new(coord);
                terrain_generator.generate_chunk(&mut chunk);
                chunks.push(chunk);
            }
        }

        if let Ok(mut guard) = result.lock() {
            *guard = Some(chunks);
        }
    });
}

fn complete_initial_world_generation(
    mut commands: Commands,
    mut world: ResMut<World>,
    mut generation_state: ResMut<InitialWorldGeneration>,
) {
    if generation_state.finished {
        return;
    }

    let mut result_guard = match generation_state.result.try_lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    let Some(chunks) = result_guard.take() else {
        return;
    };
    drop(result_guard);

    for chunk in chunks {
        let coord = chunk.coord;
        let entity = commands.spawn(chunk).id();
        world.chunks.insert(coord, entity);
    }

    spawn_player(&mut commands);
    generation_state.finished = true;
}

fn complete_pending_chunk_generation_system(
    mut commands: Commands,
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
) {
    let mut ready_chunks: Vec<(ChunkCoord, Chunk)> = world
        .pending_chunks
        .iter()
        .filter_map(|(coord, result)| {
            let Ok(mut guard) = result.try_lock() else {
                return None;
            };

            guard.take().map(|chunk| (*coord, chunk))
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
        let entity = commands.spawn(chunk).id();
        world.chunks.insert(coord, entity);
    }
}

fn chunk_loading_system(
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
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
                queue_chunk_generation(&mut world, coord);
            }
        }
    }
}

fn chunk_unloading_system(
    mut commands: Commands,
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_chunk = ChunkCoord::from_world_pos(player_transform.translation);
        let unload_distance = render_distance_chunks() + 2;
        
        let mut chunks_to_unload = Vec::new();
        
        for (&chunk_coord, &chunk_entity) in world.chunks.iter() {
            let distance_x = (chunk_coord.x - player_chunk.x).abs();
            let distance_z = (chunk_coord.z - player_chunk.z).abs();
            
            if distance_x * distance_x + distance_z * distance_z > unload_distance * unload_distance {
                chunks_to_unload.push((chunk_coord, chunk_entity));
            }
        }
        
        for (coord, entity) in chunks_to_unload {
            commands.entity(entity).despawn();
            world.chunks.remove(&coord);
        }
    }
}

fn debug_state_system(
    mut debug_state: ResMut<DebugAabbState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
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
