use crate::AppState;
use crate::voxel::{VOXEL_SIZE, VoxelFace};
use crate::world::{CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, Chunk, ChunkCoord, World};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};
use bevy_rapier3d::prelude::*;

#[derive(Component)]
pub(crate) struct PendingPhysicsCollider(Task<(u64, Option<Collider>)>);

#[derive(Clone)]
struct ChunkPhysicsInput {
    chunk: Chunk,
    neighbors: HorizontalChunkNeighbors,
}

#[derive(Clone, Default)]
struct HorizontalChunkNeighbors {
    negative_x: Option<Chunk>,
    positive_x: Option<Chunk>,
    negative_z: Option<Chunk>,
    positive_z: Option<Chunk>,
}

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RapierPhysicsPlugin::<NoUserData>::default(),))
            .add_systems(
                Update,
                queue_chunk_physics_builds
                    .run_if(in_state(AppState::LoadingWorld).or(in_state(AppState::InGame))),
            )
            .add_systems(
                Update,
                process_chunk_physics_builds
                    .run_if(in_state(AppState::LoadingWorld).or(in_state(AppState::InGame))),
            );
    }
}

#[derive(Component)]
pub struct ChunkPhysics;

fn queue_chunk_physics_builds(
    mut commands: Commands,
    world: Res<World>,
    chunk_query: Query<
        (Entity, &Chunk),
        (
            With<crate::player::NeedsPhysicsRefresh>,
            Without<PendingPhysicsCollider>,
        ),
    >,
    all_chunks: Query<&Chunk>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    for (entity, chunk) in chunk_query.iter() {
        let input = ChunkPhysicsInput {
            chunk: chunk.clone(),
            neighbors: gather_horizontal_neighbors(chunk.coord, &world, &all_chunks),
        };
        let revision = input.chunk.revision;
        let task = task_pool.spawn(async move { (revision, generate_chunk_collider(&input)) });
        commands.entity(entity).insert(PendingPhysicsCollider(task));
    }
}

fn process_chunk_physics_builds(
    mut commands: Commands,
    mut chunk_query: Query<(Entity, &mut PendingPhysicsCollider, &Chunk)>,
) {
    for (entity, mut pending_collider, chunk) in chunk_query.iter_mut() {
        let Some((revision, collider)) =
            future::block_on(future::poll_once(&mut pending_collider.0))
        else {
            continue;
        };

        if revision != chunk.revision {
            commands.entity(entity).remove::<PendingPhysicsCollider>();
            continue;
        }

        if let Some(collider) = collider {
            commands
                .entity(entity)
                .remove::<crate::player::NeedsPhysicsRefresh>()
                .insert((ChunkPhysics, RigidBody::Fixed, collider));
        } else {
            commands
                .entity(entity)
                .remove::<PendingPhysicsCollider>()
                .remove::<crate::player::NeedsPhysicsRefresh>()
                .remove::<ChunkPhysics>()
                .remove::<Collider>();
        }

        commands.entity(entity).remove::<PendingPhysicsCollider>();
    }
}

fn generate_chunk_collider(input: &ChunkPhysicsInput) -> Option<Collider> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for x in 0..CHUNK_VOXELS_SIZE {
        for y in 0..CHUNK_VOXELS_HEIGHT {
            for z in 0..CHUNK_VOXELS_SIZE {
                if let Some(voxel) = input.chunk.get_voxel(x, y, z) {
                    if voxel.is_solid() {
                        let local_x = x as f32 * VOXEL_SIZE;
                        let local_y = y as f32 * VOXEL_SIZE;
                        let local_z = z as f32 * VOXEL_SIZE;

                        add_voxel_geometry(
                            &mut vertices,
                            &mut indices,
                            Vec3::new(local_x, local_y, local_z),
                            input,
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
    input: &ChunkPhysicsInput,
    x: usize,
    y: usize,
    z: usize,
) {
    let faces = [
        (
            should_render_face_physics(input, x, y, z, -1, 0, 0),
            VoxelFace::NegativeX,
        ),
        (
            should_render_face_physics(input, x, y, z, 1, 0, 0),
            VoxelFace::PositiveX,
        ),
        (
            should_render_face_physics(input, x, y, z, 0, -1, 0),
            VoxelFace::NegativeY,
        ),
        (
            should_render_face_physics(input, x, y, z, 0, 1, 0),
            VoxelFace::PositiveY,
        ),
        (
            should_render_face_physics(input, x, y, z, 0, 0, -1),
            VoxelFace::NegativeZ,
        ),
        (
            should_render_face_physics(input, x, y, z, 0, 0, 1),
            VoxelFace::PositiveZ,
        ),
    ];

    for (should_render, face) in faces.iter() {
        if *should_render {
            add_face_geometry(vertices, indices, pos, VOXEL_SIZE, *face);
        }
    }
}

fn should_render_face_physics(
    input: &ChunkPhysicsInput,
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

    neighbor_voxel_for_face(input, nx, ny, nz).is_none_or(|voxel| !voxel.is_solid())
}

fn neighbor_voxel_for_face(
    input: &ChunkPhysicsInput,
    nx: i32,
    ny: i32,
    nz: i32,
) -> Option<crate::voxel::Voxel> {
    if ny < 0 || ny >= CHUNK_VOXELS_HEIGHT as i32 {
        return None;
    }

    if (0..CHUNK_VOXELS_SIZE as i32).contains(&nx) && (0..CHUNK_VOXELS_SIZE as i32).contains(&nz) {
        return input
            .chunk
            .get_voxel(nx as usize, ny as usize, nz as usize)
            .copied();
    }

    if nx < 0 {
        return input
            .neighbors
            .negative_x
            .as_ref()
            .and_then(|chunk| chunk.get_voxel(CHUNK_VOXELS_SIZE - 1, ny as usize, nz as usize))
            .copied();
    }

    if nx >= CHUNK_VOXELS_SIZE as i32 {
        return input
            .neighbors
            .positive_x
            .as_ref()
            .and_then(|chunk| chunk.get_voxel(0, ny as usize, nz as usize))
            .copied();
    }

    if nz < 0 {
        return input
            .neighbors
            .negative_z
            .as_ref()
            .and_then(|chunk| chunk.get_voxel(nx as usize, ny as usize, CHUNK_VOXELS_SIZE - 1))
            .copied();
    }

    if nz >= CHUNK_VOXELS_SIZE as i32 {
        return input
            .neighbors
            .positive_z
            .as_ref()
            .and_then(|chunk| chunk.get_voxel(nx as usize, ny as usize, 0))
            .copied();
    }

    None
}

fn gather_horizontal_neighbors(
    coord: ChunkCoord,
    world: &World,
    chunk_query: &Query<&Chunk>,
) -> HorizontalChunkNeighbors {
    HorizontalChunkNeighbors {
        negative_x: get_chunk_clone(ChunkCoord::new(coord.x - 1, coord.z), world, chunk_query),
        positive_x: get_chunk_clone(ChunkCoord::new(coord.x + 1, coord.z), world, chunk_query),
        negative_z: get_chunk_clone(ChunkCoord::new(coord.x, coord.z - 1), world, chunk_query),
        positive_z: get_chunk_clone(ChunkCoord::new(coord.x, coord.z + 1), world, chunk_query),
    }
}

fn get_chunk_clone(coord: ChunkCoord, world: &World, chunk_query: &Query<&Chunk>) -> Option<Chunk> {
    world
        .chunks
        .get(&coord)
        .and_then(|entity| chunk_query.get(*entity).ok())
        .cloned()
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
