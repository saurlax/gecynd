use crate::voxel::{VoxelType, VoxelWorld};
use bevy::prelude::*;
use noise::{NoiseFn, Perlin};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_initial_world)
            .add_systems(Update, update_chunks_around_player)
            .add_systems(Startup, setup_lighting);
    }
}

pub const CHUNK_SIZE: i32 = 32;
pub const CHUNK_HEIGHT: i32 = 16;
const RENDER_DISTANCE: i32 = 1; // 3x3x3 chunks around player

fn generate_initial_world(mut voxel_world: ResMut<VoxelWorld>) {
    // Generate initial chunks around origin
    for x in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for z in -RENDER_DISTANCE..=RENDER_DISTANCE {
            let chunk_key = IVec3::new(x, 0, z);
            generate_chunk(&mut voxel_world, chunk_key);
            voxel_world.loaded_chunks.insert(chunk_key);
        }
    }

    // Clear a safe spawn area around origin (only above ground level)
    let spawn_radius = 2;
    for x in -spawn_radius..=spawn_radius {
        for z in -spawn_radius..=spawn_radius {
            // Find the ground level at this position first
            let mut ground_level = 0;
            for y in 0..CHUNK_HEIGHT {
                let pos = IVec3::new(x, y, z);
                if voxel_world.get_voxel(pos) != VoxelType::Air {
                    ground_level = y + 1; // One block above the highest solid block
                }
            }

            // Only clear blocks above ground level + 1 (keep the ground and one layer above)
            for y in (ground_level + 1)..10 {
                let pos = IVec3::new(x, y, z);
                voxel_world.set_voxel(pos, VoxelType::Air);
            }
        }
    }
}

fn update_chunks_around_player(
    mut voxel_world: ResMut<VoxelWorld>,
    camera_query: Query<&Transform, With<Camera>>,
) {
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };

    let camera_pos = camera_transform.translation;
    let chunk_pos = IVec3::new(
        (camera_pos.x / (CHUNK_SIZE as f32 * crate::voxel::VOXEL_SIZE)).floor() as i32,
        0, // Y is always 0 for now
        (camera_pos.z / (CHUNK_SIZE as f32 * crate::voxel::VOXEL_SIZE)).floor() as i32,
    );

    // Generate chunks around player
    for x in (chunk_pos.x - RENDER_DISTANCE)..=(chunk_pos.x + RENDER_DISTANCE) {
        for z in (chunk_pos.z - RENDER_DISTANCE)..=(chunk_pos.z + RENDER_DISTANCE) {
            let chunk_key = IVec3::new(x, 0, z);
            if !voxel_world.loaded_chunks.contains(&chunk_key) {
                generate_chunk(&mut voxel_world, chunk_key);
                voxel_world.loaded_chunks.insert(chunk_key);
            }
        }
    }

    // Remove far chunks
    let chunks_to_remove: Vec<IVec3> = voxel_world
        .loaded_chunks
        .iter()
        .filter(|&&chunk| {
            (chunk.x - chunk_pos.x).abs() > RENDER_DISTANCE
                || (chunk.z - chunk_pos.z).abs() > RENDER_DISTANCE
        })
        .copied()
        .collect();

    for chunk in chunks_to_remove {
        unload_chunk(&mut voxel_world, chunk);
        voxel_world.loaded_chunks.remove(&chunk);
    }
}

fn generate_chunk(voxel_world: &mut VoxelWorld, chunk_pos: IVec3) {
    // Check if chunk already has voxels (player modifications), don't overwrite
    let start_x = chunk_pos.x * CHUNK_SIZE;
    let start_z = chunk_pos.z * CHUNK_SIZE;

    // Check if this chunk already has any voxels
    let mut has_existing_voxels = false;
    for (&pos, _) in voxel_world.voxels.iter() {
        if pos.x >= start_x
            && pos.x < start_x + CHUNK_SIZE
            && pos.z >= start_z
            && pos.z < start_z + CHUNK_SIZE
        {
            has_existing_voxels = true;
            break;
        }
    }

    // If chunk already has voxels, just mark it dirty and return
    if has_existing_voxels {
        voxel_world.mark_chunk_dirty(chunk_pos);
        return;
    }

    let perlin = Perlin::new(42);

    for local_x in 0..CHUNK_SIZE {
        for local_z in 0..CHUNK_SIZE {
            let world_x = start_x + local_x;
            let world_z = start_z + local_z;

            // 使用多层噪声生成更丰富的地形
            let base_height_noise = perlin.get([world_x as f64 * 0.02, world_z as f64 * 0.02]);
            let detail_noise = perlin.get([world_x as f64 * 0.1, world_z as f64 * 0.1]) * 0.2;

            // 增加地形起伏，高度范围约为2-10
            let height = 4.0 + (base_height_noise + 1.0) * 3.0 + detail_noise * 3.0;
            let height = height as i32;

            for y in 0..=height.min(CHUNK_HEIGHT - 1) {
                let pos = IVec3::new(world_x, y, world_z);

                let voxel_type = if y == height {
                    VoxelType::Grass
                } else if y > height - 2 {
                    VoxelType::Dirt
                } else {
                    VoxelType::Stone
                };

                voxel_world.set_voxel(pos, voxel_type);
            }
        }
    }

    // Mark chunk as dirty to generate mesh
    voxel_world.mark_chunk_dirty(chunk_pos);
}

fn unload_chunk(voxel_world: &mut VoxelWorld, chunk_pos: IVec3) {
    let start_x = chunk_pos.x * CHUNK_SIZE;
    let start_z = chunk_pos.z * CHUNK_SIZE;

    // Remove all voxels in this chunk
    let mut voxels_to_remove = Vec::new();
    for (&pos, _) in voxel_world.voxels.iter() {
        if pos.x >= start_x
            && pos.x < start_x + CHUNK_SIZE
            && pos.z >= start_z
            && pos.z < start_z + CHUNK_SIZE
        {
            voxels_to_remove.push(pos);
        }
    }

    for pos in voxels_to_remove {
        voxel_world.voxels.remove(&pos);
    }
}

fn setup_lighting(mut commands: Commands) {
    // 添加方向光源
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4)),
    ));

    // 添加环境光
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.3, 0.3, 0.3),
        brightness: 0.3,
        affects_lightmapped_meshes: false,
    });
}
