use crate::voxel::{VOXEL_SIZE, VoxelFace};
use crate::world::{Chunk, DebugViewMode, CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

#[derive(Component)]
struct PendingPhysicsCollider;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default().disabled(),
        ))
        .add_systems(Update, sync_physics_debug_mode)
        .add_systems(Update, queue_chunk_physics_builds)
        .add_systems(Update, process_chunk_physics_builds);
    }
}

#[derive(Component)]
pub struct ChunkPhysics;

fn sync_physics_debug_mode(
    debug_view_mode: Res<DebugViewMode>,
    mut render_context: ResMut<DebugRenderContext>,
) {
    render_context.enabled = debug_view_mode.physics_wireframe;
}

fn queue_chunk_physics_builds(
    mut commands: Commands,
    chunk_query: Query<(Entity, &Chunk), (Without<Collider>, Without<PendingPhysicsCollider>)>,
) {
    for (entity, chunk) in chunk_query.iter() {
        if chunk.get_voxel(0, 0, 0).is_some() {
            commands.entity(entity).insert(PendingPhysicsCollider);
        }
    }
}

fn process_chunk_physics_builds(
    mut commands: Commands,
    chunk_query: Query<(Entity, &Chunk), With<PendingPhysicsCollider>>,
) {
    for (entity, chunk) in chunk_query.iter() {
        let collider = generate_chunk_collider(chunk);
        commands.entity(entity).remove::<PendingPhysicsCollider>();

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

    if nx < 0
        || nx >= CHUNK_VOXELS_SIZE as i32
        || ny < 0
        || ny >= CHUNK_VOXELS_HEIGHT as i32
        || nz < 0
        || nz >= CHUNK_VOXELS_SIZE as i32
    {
        return true;
    }

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
    
    for vertex in face_vertices.iter() {
        vertices.push(Vec3::new(vertex[0], vertex[1], vertex[2]));
    }

    indices.push([start_vertex, start_vertex + 1, start_vertex + 2]);
    indices.push([start_vertex, start_vertex + 2, start_vertex + 3]);
}
