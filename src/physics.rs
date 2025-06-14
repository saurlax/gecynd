use crate::voxel::{VOXEL_SIZE, VoxelFace};
use crate::world::{CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, Chunk, World};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RapierPhysicsPlugin::<NoUserData>::default(),
            // RapierDebugRenderPlugin::default(),
        ))
        .add_systems(Update, update_chunk_physics);
    }
}

#[derive(Component)]
pub struct ChunkPhysics;

fn update_chunk_physics(
    mut commands: Commands,
    _world: Res<World>,
    chunk_query: Query<(Entity, &Chunk), Without<ChunkPhysics>>,
    chunk_requery: Query<(Entity, &Chunk), (With<RigidBody>, Without<ChunkPhysics>)>,
) {
    // 处理新区块
    for (entity, chunk) in chunk_query.iter() {
        let collider = generate_chunk_collider(chunk);
        if let Some(collider) = collider {
            commands
                .entity(entity)
                .insert((ChunkPhysics, RigidBody::Fixed, collider));
        }
    }
    
    // 处理需要重新生成物理的区块
    for (entity, chunk) in chunk_requery.iter() {
        commands.entity(entity).remove::<RigidBody>();
        commands.entity(entity).remove::<Collider>();
        
        let collider = generate_chunk_collider(chunk);
        if let Some(collider) = collider {
            commands
                .entity(entity)
                .insert((ChunkPhysics, RigidBody::Fixed, collider));
        }
    }
}

fn generate_chunk_collider(chunk: &Chunk) -> Option<Collider> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for x in 0..CHUNK_VOXELS_SIZE {
        for y in 0..CHUNK_VOXELS_HEIGHT {
            for z in 0..CHUNK_VOXELS_SIZE {
                if let Some(voxel) = chunk.get_voxel(x, y, z) {
                    if voxel.is_solid() {
                        // 使用与渲染系统相同的坐标计算方式
                        let local_x = x as f32 * VOXEL_SIZE;
                        let local_y = y as f32 * VOXEL_SIZE;
                        let local_z = z as f32 * VOXEL_SIZE;

                        add_voxel_geometry(
                            &mut vertices,
                            &mut indices,
                            Vec3::new(local_x, local_y, local_z),
                            chunk,
                            x,
                            y,
                            z,
                        );
                    }
                }
            }
        }
    }

    if vertices.is_empty() {
        return None;
    }

    Collider::trimesh(vertices, indices).ok()
}

fn add_voxel_geometry(
    vertices: &mut Vec<Vec3>,
    indices: &mut Vec<[u32; 3]>,
    pos: Vec3,
    chunk: &Chunk,
    x: usize,
    y: usize,
    z: usize,
) {
    let faces = [
        (should_render_face_physics(chunk, x, y, z, -1, 0, 0), VoxelFace::NegativeX),
        (should_render_face_physics(chunk, x, y, z, 1, 0, 0), VoxelFace::PositiveX),
        (should_render_face_physics(chunk, x, y, z, 0, -1, 0), VoxelFace::NegativeY),
        (should_render_face_physics(chunk, x, y, z, 0, 1, 0), VoxelFace::PositiveY),
        (should_render_face_physics(chunk, x, y, z, 0, 0, -1), VoxelFace::NegativeZ),
        (should_render_face_physics(chunk, x, y, z, 0, 0, 1), VoxelFace::PositiveZ),
    ];

    for (should_render, face) in faces.iter() {
        if *should_render {
            add_face_geometry(vertices, indices, pos, VOXEL_SIZE, *face);
        }
    }
}

fn should_render_face_physics(
    chunk: &Chunk,
    x: usize,
    y: usize,
    z: usize,
    dx: i32,
    dy: i32,
    dz: i32,
) -> bool {
    let nx = x as i32 + dx;
    let ny = y as i32 + dy;
    let nz = z as i32 + dz;

    // 如果相邻位置超出区块边界，则渲染该面
    if nx < 0
        || nx >= CHUNK_VOXELS_SIZE as i32
        || ny < 0
        || ny >= CHUNK_VOXELS_HEIGHT as i32
        || nz < 0
        || nz >= CHUNK_VOXELS_SIZE as i32
    {
        return true;
    }

    // 如果相邻位置是空气或不存在，则渲染该面
    if let Some(neighbor_voxel) = chunk.get_voxel(nx as usize, ny as usize, nz as usize) {
        !neighbor_voxel.is_solid()
    } else {
        true
    }
}

fn add_face_geometry(
    vertices: &mut Vec<Vec3>,
    indices: &mut Vec<[u32; 3]>,
    pos: Vec3,
    size: f32,
    face: VoxelFace,
) {
    let start_vertex = vertices.len() as u32;
    let face_vertices = face.get_vertices(pos, size);
    
    // Convert to Vec3
    for vertex in face_vertices.iter() {
        vertices.push(Vec3::new(vertex[0], vertex[1], vertex[2]));
    }

    indices.push([start_vertex, start_vertex + 1, start_vertex + 2]);
    indices.push([start_vertex, start_vertex + 2, start_vertex + 3]);
}
