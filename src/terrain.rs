use noise::{NoiseFn, Perlin};
use crate::world::{chunk_world_origin, Chunk, CHUNK_VOXELS_SIZE, CHUNK_VOXELS_HEIGHT};
use crate::voxel::{Voxel, VoxelType, VOXEL_SIZE};

const TERRAIN_BASE_HEIGHT_METERS: f32 = 4.0;
const TERRAIN_VARIATION_METERS: f32 = 4.0;
const DIRT_LAYER_THICKNESS_METERS: f32 = 0.5;

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
        let chunk_origin = chunk_world_origin(chunk.coord);
        let dirt_voxels = (DIRT_LAYER_THICKNESS_METERS / VOXEL_SIZE).max(1.0).round() as i32;
        
        for x in 0..CHUNK_VOXELS_SIZE {
            for z in 0..CHUNK_VOXELS_SIZE {
                let world_x = chunk_origin.x + x as f32 * VOXEL_SIZE;
                let world_z = chunk_origin.z + z as f32 * VOXEL_SIZE;
                
                let height = self.get_height(world_x as f64, world_z as f64) as f32;
                let surface_voxel_y = (height / VOXEL_SIZE).floor() as i32;
                
                for y in 0..CHUNK_VOXELS_HEIGHT {
                    let yi = y as i32;
                    let world_y = y as f32 * VOXEL_SIZE;
                    let voxel_type = if yi > surface_voxel_y {
                        VoxelType::Air
                    } else if yi == surface_voxel_y {
                        VoxelType::Grass
                    } else if yi >= surface_voxel_y - dirt_voxels {
                        VoxelType::Dirt
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
        let scale = 0.08;
        let height = self.height_noise.get([x * scale, z * scale]);
        (TERRAIN_BASE_HEIGHT_METERS + (height as f32 + 1.0) * TERRAIN_VARIATION_METERS) as f64
    }
}
