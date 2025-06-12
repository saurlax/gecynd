use crate::voxel::VOXEL_SIZE;
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
        (should_render_face_physics(chunk, x, y, z, -1, 0, 0), 0), // Left
        (should_render_face_physics(chunk, x, y, z, 1, 0, 0), 1),  // Right
        (should_render_face_physics(chunk, x, y, z, 0, -1, 0), 2), // Bottom
        (should_render_face_physics(chunk, x, y, z, 0, 1, 0), 3),  // Top
        (should_render_face_physics(chunk, x, y, z, 0, 0, -1), 4), // Back
        (should_render_face_physics(chunk, x, y, z, 0, 0, 1), 5),  // Front
    ];

    for (should_render, face_index) in faces.iter() {
        if *should_render {
            add_face_geometry(vertices, indices, pos, VOXEL_SIZE, *face_index);
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
    face_index: usize,
) {
    let start_vertex = vertices.len() as u32;

    match face_index {
        0 => {
            // Left face (-X) - 与渲染系统保持一致的顶点顺序
            vertices.extend_from_slice(&[
                pos + Vec3::new(0.0, 0.0, 0.0),
                pos + Vec3::new(0.0, 0.0, size),
                pos + Vec3::new(0.0, size, size),
                pos + Vec3::new(0.0, size, 0.0),
            ]);
        }
        1 => {
            // Right face (+X)
            vertices.extend_from_slice(&[
                pos + Vec3::new(size, 0.0, 0.0),
                pos + Vec3::new(size, size, 0.0),
                pos + Vec3::new(size, size, size),
                pos + Vec3::new(size, 0.0, size),
            ]);
        }
        2 => {
            // Bottom face (-Y)
            vertices.extend_from_slice(&[
                pos + Vec3::new(0.0, 0.0, 0.0),
                pos + Vec3::new(size, 0.0, 0.0),
                pos + Vec3::new(size, 0.0, size),
                pos + Vec3::new(0.0, 0.0, size),
            ]);
        }
        3 => {
            // Top face (+Y)
            vertices.extend_from_slice(&[
                pos + Vec3::new(0.0, size, 0.0),
                pos + Vec3::new(0.0, size, size),
                pos + Vec3::new(size, size, size),
                pos + Vec3::new(size, size, 0.0),
            ]);
        }
        4 => {
            // Back face (-Z)
            vertices.extend_from_slice(&[
                pos + Vec3::new(0.0, 0.0, 0.0),
                pos + Vec3::new(0.0, size, 0.0),
                pos + Vec3::new(size, size, 0.0),
                pos + Vec3::new(size, 0.0, 0.0),
            ]);
        }
        5 => {
            // Front face (+Z)
            vertices.extend_from_slice(&[
                pos + Vec3::new(0.0, 0.0, size),
                pos + Vec3::new(size, 0.0, size),
                pos + Vec3::new(size, size, size),
                pos + Vec3::new(0.0, size, size),
            ]);
        }
        _ => {}
    }

    // 确保与渲染系统相同的三角形绕序
    indices.push([start_vertex, start_vertex + 1, start_vertex + 2]);
    indices.push([start_vertex, start_vertex + 2, start_vertex + 3]);
}
