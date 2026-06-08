use bevy::asset::RenderAssetUsages;
use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures_lite::future};

use crate::player::{PlayerInteraction, brush_center_for_edit, brush_preview_origin, brush_world_size};
use crate::voxel::{VOXEL_SIZE, VoxelFace, VoxelType};
use crate::world::{
    CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, Chunk, ChunkCoord, DebugViewMode, World,
    chunk_world_height, chunk_world_origin,
};

#[derive(Component)]
pub struct ChunkMesh;

#[derive(Component)]
pub struct VoxelHighlight;

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct DebugAabb;

#[derive(Component)]
struct PendingRenderMesh(Task<(u64, Option<Mesh>)>);

#[derive(Clone)]
struct ChunkRenderInput {
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

pub struct RenderPlugin;

#[derive(Resource)]
struct ChunkMaterial {
    handle: Handle<StandardMaterial>,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WireframePlugin::default())
            .add_systems(
                Startup,
                (
                    setup_lighting,
                    setup_chunk_material,
                    setup_crosshair,
                    setup_voxel_highlight,
                ),
            )
            .add_systems(
                Update,
                (
                    sync_render_wireframe_mode,
                    queue_chunk_render_builds.before(process_chunk_render_builds),
                    process_chunk_render_builds.before(debug_aabb_system),
                    voxel_highlight_system,
                    debug_aabb_system,
                ),
            );
    }
}

fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 9000.0,
            color: Color::srgb(1.0, 0.97, 0.9),
            shadows_enabled: false,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -std::f32::consts::FRAC_PI_4,
            std::f32::consts::FRAC_PI_4,
            0.0,
        )),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.62, 0.69, 0.8),
        brightness: 100.0,
        affects_lightmapped_meshes: false,
    });
}

fn setup_chunk_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 0.0,
        perceptual_roughness: 0.9,
        reflectance: 0.08,
        ..default()
    });

    commands.insert_resource(ChunkMaterial { handle: material });
}

fn setup_crosshair(mut commands: Commands) {
    commands
        .spawn((
            Crosshair,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                width: Val::Px(20.0),
                height: Val::Px(20.0),
                margin: UiRect {
                    left: Val::Px(-10.0),
                    top: Val::Px(-10.0),
                    ..default()
                },
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(8.0),
                    top: Val::Px(9.0),
                    width: Val::Px(4.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));

            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(9.0),
                    top: Val::Px(8.0),
                    width: Val::Px(2.0),
                    height: Val::Px(4.0),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
            ));
        });
}

fn setup_voxel_highlight(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh_handle = meshes.add(create_single_voxel_wireframe());
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        ..default()
    });

    commands.spawn((
        VoxelHighlight,
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Hidden,
        Name::new("Voxel Highlight"),
    ));
}

fn queue_chunk_render_builds(
    mut commands: Commands,
    world: Res<World>,
    chunk_query: Query<
        (Entity, &Chunk),
        (
            With<crate::player::NeedsRenderRefresh>,
            Without<PendingRenderMesh>,
        ),
    >,
    all_chunks: Query<&Chunk>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    for (entity, chunk) in chunk_query.iter() {
        let input = ChunkRenderInput {
            chunk: chunk.clone(),
            neighbors: gather_horizontal_neighbors(chunk.coord, &world, &all_chunks),
        };
        let revision = input.chunk.revision;
        let task = task_pool.spawn(async move { (revision, generate_chunk_mesh(&input)) });
        commands.entity(entity).insert(PendingRenderMesh(task));
    }
}

fn process_chunk_render_builds(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_material: Res<ChunkMaterial>,
    mut chunk_query: Query<(Entity, &mut PendingRenderMesh, &Chunk)>,
    debug_state: Res<crate::world::DebugAabbState>,
    debug_view_mode: Res<DebugViewMode>,
) {
    for (entity, mut pending_mesh, chunk) in chunk_query.iter_mut() {
        let Some((revision, mesh)) = future::block_on(future::poll_once(&mut pending_mesh.0))
        else {
            continue;
        };

        if revision != chunk.revision {
            commands.entity(entity).remove::<PendingRenderMesh>();
            continue;
        }

        if let Some(mesh) = mesh {
            let mesh_handle = meshes.add(mesh);
            let chunk_world_pos = chunk_world_origin(chunk.coord);
            let wireframe = if debug_view_mode.render_wireframe {
                Some(Wireframe)
            } else {
                None
            };

            let mut entity_commands = commands.entity(entity);
            entity_commands.remove::<PendingRenderMesh>();
            entity_commands.remove::<crate::player::NeedsRenderRefresh>();
            entity_commands.insert((
                ChunkMesh,
                Mesh3d(mesh_handle),
                MeshMaterial3d(chunk_material.handle.clone()),
                Transform::from_translation(chunk_world_pos),
                GlobalTransform::default(),
                Visibility::Visible,
            ));

            if let Some(wireframe) = wireframe {
                entity_commands.insert(wireframe);
            }

            if debug_state.enabled {
                create_debug_aabb_for_chunk(&mut commands, &mut meshes, &mut materials, entity);
            }
        } else {
            commands
                .entity(entity)
                .remove::<PendingRenderMesh>()
                .remove::<crate::player::NeedsRenderRefresh>()
                .remove::<ChunkMesh>()
                .remove::<Mesh3d>()
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .remove::<Wireframe>();
        }

        commands.entity(entity).remove::<PendingRenderMesh>();
    }
}

fn sync_render_wireframe_mode(
    debug_view_mode: Res<DebugViewMode>,
    mut commands: Commands,
    chunk_query: Query<Entity, With<ChunkMesh>>,
    wireframe_query: Query<(), With<Wireframe>>,
) {
    let render_mode_enabled = debug_view_mode.render_wireframe;

    for entity in chunk_query.iter() {
        let has_wireframe = wireframe_query.get(entity).is_ok();
        if render_mode_enabled && !has_wireframe {
            commands.entity(entity).insert(Wireframe);
        } else if !render_mode_enabled && has_wireframe {
            commands.entity(entity).remove::<Wireframe>();
        }
    }
}

fn voxel_highlight_system(
    interaction: Res<PlayerInteraction>,
    mut highlight_query: Query<
        (&mut Transform, &mut Visibility, &mut Mesh3d),
        With<VoxelHighlight>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    chunk_query: Query<&crate::world::Chunk>,
    world: Res<crate::world::World>,
) {
    let Ok((mut highlight_transform, mut highlight_visibility, mut highlight_mesh)) =
        highlight_query.single_mut()
    else {
        return;
    };

    if let Some(selected_voxel_pos) = interaction.selected_voxel_world_pos {
        if let Some(voxel) = world.get_voxel_at_world(selected_voxel_pos, &chunk_query) {
            if voxel.is_solid() {
                let Some(preview_center) = brush_center_for_edit(
                    selected_voxel_pos,
                    interaction.hit_face,
                ) else {
                    *highlight_visibility = Visibility::Hidden;
                    return;
                };

                let mesh = create_box_wireframe(Vec3::splat(brush_world_size()));

                highlight_mesh.0 = meshes.add(mesh);
                highlight_transform.translation = brush_preview_origin(preview_center);
                *highlight_visibility = Visibility::Visible;
                return;
            }
        }
    }

    *highlight_visibility = Visibility::Hidden;
}

fn create_single_voxel_wireframe() -> Mesh {
    create_box_wireframe(Vec3::splat(VOXEL_SIZE))
}

fn create_box_wireframe(size: Vec3) -> Mesh {
    let min = Vec3::ZERO;
    let max = size;
    let vertices = vec![
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [max.x, min.y, max.z],
        [min.x, min.y, max.z],
        [min.x, max.y, min.z],
        [max.x, max.y, min.z],
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
    ];

    let indices = vec![
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    let normals = vec![[0.0, 1.0, 0.0]; 8];
    let uvs = vec![[0.0, 0.0]; 8];

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    mesh
}

fn debug_aabb_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keys: Res<ButtonInput<KeyCode>>,
    chunk_query: Query<Entity, With<ChunkMesh>>,
    debug_query: Query<Entity, With<DebugAabb>>,
    children_query: Query<&Children>,
    debug_state: Res<crate::world::DebugAabbState>,
) {
    if keys.just_pressed(KeyCode::F1) {
        if debug_state.enabled {
            for chunk_entity in chunk_query.iter() {
                let has_debug_aabb = if let Ok(children) = children_query.get(chunk_entity) {
                    children.iter().any(|child| debug_query.get(child).is_ok())
                } else {
                    false
                };

                if !has_debug_aabb {
                    create_debug_aabb_for_chunk(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        chunk_entity,
                    );
                }
            }
        } else {
            for entity in debug_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn create_debug_aabb_for_chunk(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    chunk_entity: Entity,
) {
    let chunk_size_world = CHUNK_VOXELS_SIZE as f32 * VOXEL_SIZE;
    let chunk_height_world = chunk_world_height();

    let min = Vec3::new(0.0, 0.0, 0.0);
    let max = Vec3::new(chunk_size_world, chunk_height_world, chunk_size_world);

    let vertices = vec![
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [max.x, min.y, max.z],
        [min.x, min.y, max.z],
        [min.x, max.y, min.z],
        [max.x, max.y, min.z],
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
    ];

    let indices = vec![
        0, 1, 1, 2, 2, 3, 3, 0, 4, 5, 5, 6, 6, 7, 7, 4, 0, 4, 1, 5, 2, 6, 3, 7,
    ];

    let normals = vec![[0.0, 1.0, 0.0]; 8];
    let uvs = vec![[0.0, 0.0]; 8];

    let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::RENDER_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

    let mesh_handle = meshes.add(mesh);
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.0),
        unlit: true,
        cull_mode: None,
        ..default()
    });

    let debug_aabb_entity = commands
        .spawn((
            DebugAabb,
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::from_translation(Vec3::ZERO),
            GlobalTransform::default(),
            Name::new("Debug AABB"),
            NotShadowCaster,
            NotShadowReceiver,
        ))
        .id();

    commands.entity(chunk_entity).add_child(debug_aabb_entity);
}

fn generate_chunk_mesh(input: &ChunkRenderInput) -> Option<Mesh> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut colors = Vec::new();

    for x in 0..CHUNK_VOXELS_SIZE {
        for y in 0..CHUNK_VOXELS_HEIGHT {
            for z in 0..CHUNK_VOXELS_SIZE {
                if let Some(voxel) = input.chunk.get_voxel(x, y, z) {
                    if voxel.is_solid() {
                        let local_pos = Vec3::new(
                            x as f32 * VOXEL_SIZE,
                            y as f32 * VOXEL_SIZE,
                            z as f32 * VOXEL_SIZE,
                        );

                        add_voxel_faces(
                            &mut vertices,
                            &mut indices,
                            &mut normals,
                            &mut uvs,
                            &mut colors,
                            local_pos,
                            voxel.voxel_type,
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

    let chunk_size_world = CHUNK_VOXELS_SIZE as f32 * VOXEL_SIZE;
    let chunk_height_world = chunk_world_height();

    let mut extended_vertices = vertices;
    let dummy_indices_start = extended_vertices.len() as u32;

    extended_vertices.extend_from_slice(&[
        [0.0, 0.0, 0.0],
        [chunk_size_world, 0.0, 0.0],
        [0.0, 0.0, chunk_size_world],
        [chunk_size_world, 0.0, chunk_size_world],
        [0.0, chunk_height_world, 0.0],
        [chunk_size_world, chunk_height_world, 0.0],
        [0.0, chunk_height_world, chunk_size_world],
        [chunk_size_world, chunk_height_world, chunk_size_world],
    ]);

    let mut extended_normals = normals;
    let mut extended_uvs = uvs;
    let mut extended_colors = colors;
    extended_normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 8]);
    extended_uvs.extend_from_slice(&[[0.0, 0.0]; 8]);
    extended_colors.extend_from_slice(&[[1.0, 1.0, 1.0, 1.0]; 8]);

    let mut extended_indices = indices;
    for i in 0..8 {
        let idx = dummy_indices_start + i;
        extended_indices.extend_from_slice(&[idx, idx, idx]);
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, extended_vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, extended_normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, extended_uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, extended_colors);
    mesh.insert_indices(Indices::U32(extended_indices));

    Some(mesh)
}

fn add_voxel_faces(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    pos: Vec3,
    voxel_type: VoxelType,
    input: &ChunkRenderInput,
    x: usize,
    y: usize,
    z: usize,
) {
    let faces = [
        (
            should_render_face(input, x, y, z, -1, 0, 0),
            VoxelFace::NegativeX,
        ),
        (
            should_render_face(input, x, y, z, 1, 0, 0),
            VoxelFace::PositiveX,
        ),
        (
            should_render_face(input, x, y, z, 0, -1, 0),
            VoxelFace::NegativeY,
        ),
        (
            should_render_face(input, x, y, z, 0, 1, 0),
            VoxelFace::PositiveY,
        ),
        (
            should_render_face(input, x, y, z, 0, 0, -1),
            VoxelFace::NegativeZ,
        ),
        (
            should_render_face(input, x, y, z, 0, 0, 1),
            VoxelFace::PositiveZ,
        ),
    ];

    for (should_render, face) in faces.iter() {
        if *should_render {
            add_face(
                vertices, indices, normals, uvs, colors, pos, *face, voxel_type,
            );
        }
    }
}

fn should_render_face(
    input: &ChunkRenderInput,
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
    input: &ChunkRenderInput,
    nx: i32,
    ny: i32,
    nz: i32,
) -> Option<crate::voxel::Voxel> {
    if ny < 0 || ny >= CHUNK_VOXELS_HEIGHT as i32 {
        return None;
    }

    if (0..CHUNK_VOXELS_SIZE as i32).contains(&nx) && (0..CHUNK_VOXELS_SIZE as i32).contains(&nz) {
        return input.chunk.get_voxel(nx as usize, ny as usize, nz as usize).copied();
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

fn add_face(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    colors: &mut Vec<[f32; 4]>,
    pos: Vec3,
    face: VoxelFace,
    voxel_type: VoxelType,
) {
    let start_vertex = vertices.len() as u32;
    let face_vertices = face.get_vertices(pos, VOXEL_SIZE);
    let face_normal = face.get_normal();
    let linear = voxel_type.color().to_linear();
    let color = [linear.red, linear.green, linear.blue, linear.alpha];

    vertices.extend_from_slice(&face_vertices);
    normals.extend_from_slice(&[[face_normal.x, face_normal.y, face_normal.z]; 4]);
    uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    colors.extend_from_slice(&[color; 4]);

    indices.extend_from_slice(&[
        start_vertex,
        start_vertex + 1,
        start_vertex + 2,
        start_vertex,
        start_vertex + 2,
        start_vertex + 3,
    ]);
}
