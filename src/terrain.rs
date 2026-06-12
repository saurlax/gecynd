use crate::voxel::{VOXEL_SIZE, Voxel, VoxelType};
use crate::world::{CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, Chunk, chunk_world_origin};
use noise::{NoiseFn, Perlin};

pub const TERRAIN_MIN_HEIGHT_METERS: f32 = 3.5;
pub const TERRAIN_MAX_HEIGHT_METERS: f32 = 9.5;
const TERRAIN_BASE_HEIGHT_METERS: f32 = 5.8;
const TERRAIN_PRIMARY_VARIATION_METERS: f32 = 2.1;
const TERRAIN_SECONDARY_VARIATION_METERS: f32 = 0.9;
const TERRAIN_DETAIL_VARIATION_METERS: f32 = 0.35;
const DIRT_LAYER_THICKNESS_METERS: f32 = 0.7;
const STONE_CAP_NOISE_THRESHOLD: f64 = 0.58;

pub struct TerrainGenerator {
    broad_noise: Perlin,
    rolling_noise: Perlin,
    detail_noise: Perlin,
    surface_noise: Perlin,
    cave_noise: Perlin,
    ore_noise: Perlin,
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            broad_noise: Perlin::new(seed),
            rolling_noise: Perlin::new(seed.wrapping_add(67890)),
            detail_noise: Perlin::new(seed.wrapping_add(24680)),
            surface_noise: Perlin::new(seed.wrapping_add(13579)),
            cave_noise: Perlin::new(seed.wrapping_add(424242)),
            ore_noise: Perlin::new(seed.wrapping_add(919191)),
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
                let surface_noise = self
                    .surface_noise
                    .get([world_x as f64 * 0.045, world_z as f64 * 0.045]);

                for y in 0..CHUNK_VOXELS_HEIGHT {
                    let yi = y as i32;
                    let world_y = y as f32 * VOXEL_SIZE;
                    let cave_noise = self.cave_noise.get([
                        world_x as f64 * 0.055,
                        world_y as f64 * 0.055,
                        world_z as f64 * 0.055,
                    ]);
                    let ore_noise = self.ore_noise.get([
                        world_x as f64 * 0.09,
                        world_y as f64 * 0.09,
                        world_z as f64 * 0.09,
                    ]);

                    let voxel_type = if yi > surface_voxel_y {
                        VoxelType::Air
                    } else if yi < surface_voxel_y - 6 && cave_noise > 0.42 {
                        VoxelType::Air
                    } else if yi == surface_voxel_y {
                        if surface_noise > STONE_CAP_NOISE_THRESHOLD {
                            VoxelType::Stone
                        } else {
                            VoxelType::Grass
                        }
                    } else if yi >= surface_voxel_y - dirt_voxels {
                        VoxelType::Dirt
                    } else if yi < surface_voxel_y - dirt_voxels - 8 && ore_noise > 0.63 {
                        VoxelType::Grass
                    } else {
                        VoxelType::Stone
                    };

                    chunk.set_voxel(x, y, z, Voxel::new(voxel_type));
                }
            }
        }
    }

    fn get_height(&self, x: f64, z: f64) -> f64 {
        let broad = self.broad_noise.get([x * 0.010, z * 0.010]) as f32;
        let rolling = self.rolling_noise.get([x * 0.026, z * 0.026]) as f32;
        let detail = self.detail_noise.get([x * 0.060, z * 0.060]) as f32;

        let height = TERRAIN_BASE_HEIGHT_METERS
            + broad * TERRAIN_PRIMARY_VARIATION_METERS
            + rolling * TERRAIN_SECONDARY_VARIATION_METERS
            + detail * TERRAIN_DETAIL_VARIATION_METERS;

        height.clamp(TERRAIN_MIN_HEIGHT_METERS, TERRAIN_MAX_HEIGHT_METERS) as f64
    }
}
