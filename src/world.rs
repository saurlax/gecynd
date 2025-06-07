use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::voxel::{Voxel, VOXEL_SIZE, VOXEL_PRECISION};
use crate::terrain::TerrainGenerator;
use crate::player::Player;

pub const CHUNK_SIZE: usize = 16; // 区块在世界中的大小（单位）
pub const CHUNK_HEIGHT: usize = 256; // 区块在世界中的高度（单位）
pub const RENDER_DISTANCE: i32 = 5;

// 实际的体素数组大小
pub const CHUNK_VOXELS_SIZE: usize = CHUNK_SIZE * VOXEL_PRECISION as usize;
pub const CHUNK_VOXELS_HEIGHT: usize = CHUNK_HEIGHT * VOXEL_PRECISION as usize;

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
        Self {
            x: (world_pos.x / CHUNK_SIZE as f32).floor() as i32,
            z: (world_pos.z / CHUNK_SIZE as f32).floor() as i32,
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
        // 使用Vec来避免栈分配
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
    pub terrain_generator: TerrainGenerator,
}

impl Default for World {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            terrain_generator: TerrainGenerator::new(),
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<World>()
            .add_systems(Update, chunk_loading_system);
    }
}

fn chunk_loading_system(
    mut commands: Commands,
    mut world: ResMut<World>,
    player_query: Query<&Transform, With<Player>>,
) {
    if let Ok(player_transform) = player_query.single() {
        let player_chunk = ChunkCoord::from_world_pos(player_transform.translation);
        let mut chunks_to_generate = HashSet::new();
        
        // 检查玩家周围的区块
        for x in (player_chunk.x - RENDER_DISTANCE)..=(player_chunk.x + RENDER_DISTANCE) {
            for z in (player_chunk.z - RENDER_DISTANCE)..=(player_chunk.z + RENDER_DISTANCE) {
                let coord = ChunkCoord::new(x, z);
                if !world.chunks.contains_key(&coord) {
                    chunks_to_generate.insert(coord);
                }
            }
        }
        
        // 生成新区块
        for coord in chunks_to_generate {
            let mut chunk = Chunk::new(coord);
            world.terrain_generator.generate_chunk(&mut chunk);
            
            let entity = commands.spawn(chunk).id();
            world.chunks.insert(coord, entity);
        }
    }
}
