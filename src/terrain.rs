use noise::{NoiseFn, Perlin};
use crate::world::{Chunk, CHUNK_SIZE, CHUNK_VOXELS_SIZE, CHUNK_VOXELS_HEIGHT};
use crate::voxel::{Voxel, VoxelType, VOXEL_SIZE};

pub struct TerrainGenerator {
    height_noise: Perlin,
    cave_noise: Perlin,
}

impl TerrainGenerator {
    pub fn new() -> Self {
        Self {
            height_noise: Perlin::new(12345),
            cave_noise: Perlin::new(54321),
        }
    }
    
    pub fn generate_chunk(&self, chunk: &mut Chunk) {
        // 使用统一的VOXEL_SIZE坐标计算
        let chunk_world_x = chunk.coord.x as f32 * (CHUNK_SIZE as f32 * VOXEL_SIZE);
        let chunk_world_z = chunk.coord.z as f32 * (CHUNK_SIZE as f32 * VOXEL_SIZE);
        
        for x in 0..CHUNK_VOXELS_SIZE {
            for z in 0..CHUNK_VOXELS_SIZE {
                let world_x = chunk_world_x + x as f32 * VOXEL_SIZE;
                let world_z = chunk_world_z + z as f32 * VOXEL_SIZE;
                
                let height = self.get_height(world_x as f64, world_z as f64);
                let grass_height = height;
                let dirt_height = height - 3.0;
                
                for y in 0..CHUNK_VOXELS_HEIGHT {
                    let world_y = y as f32 * VOXEL_SIZE;
                    let voxel_type = if world_y > grass_height as f32 {
                        VoxelType::Air
                    } else if world_y > dirt_height as f32 {
                        if world_y == (grass_height.floor() as f32) {
                            VoxelType::Grass
                        } else {
                            VoxelType::Dirt
                        }
                    } else {
                        let cave_noise = self.cave_noise.get([
                            world_x as f64 * 0.02,
                            world_y as f64 * 0.02,
                            world_z as f64 * 0.02,
                        ]);
                        
                        if cave_noise > 0.3 {
                            VoxelType::Air
                        } else {
                            VoxelType::Stone
                        }
                    };
                    
                    chunk.set_voxel(x, y, z, Voxel::new(voxel_type));
                }
            }
        }
    }
    
    fn get_height(&self, x: f64, z: f64) -> f64 {
        let scale = 0.01;
        let height = self.height_noise.get([x * scale, z * scale]);
        // 将噪声值从[-1, 1]映射到[32, 96]
        32.0 + (height + 1.0) * 32.0
    }
}
