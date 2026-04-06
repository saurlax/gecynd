use bevy::light::{NotShadowCaster, NotShadowReceiver};
use bevy::prelude::*;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;

use crate::player::PlayerInteraction;
use crate::voxel::{VOXEL_SIZE, VoxelFace};
use crate::world::{chunk_world_height, chunk_world_origin, CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, Chunk};

#[derive(Component)]
pub struct ChunkMesh;

#[derive(Component)]
pub struct VoxelHighlight;

#[derive(Component)]
pub struct Crosshair;

#[derive(Component)]
pub struct DebugAabb;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_lighting, setup_crosshair))
            .add_systems(
                Update,
                (
                    chunk_rendering_system.before(debug_aabb_system),
                    chunk_rerendering_system.before(debug_aabb_system),
                    voxel_highlight_system,
                    force_rerender_system,
                    debug_aabb_system,
                ),
            );
    }
}

fn setup_lighting(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
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
        color: Color::WHITE,
        brightness: 60.0,
        affects_lightmapped_meshes: false,
    });
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

fn chunk_rendering_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_query: Query<(Entity, &Chunk), (Without<Mesh3d>, Without<ChunkMesh>)>,
    debug_state: Res<crate::world::DebugAabbState>,
) {
    for (entity, chunk) in chunk_query.iter() {
        if let Some(mesh) = generate_chunk_mesh(chunk) {
            let mesh_handle = meshes.add(mesh);
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 0.3),
                metallic: 0.0,
                perceptual_roughness: 0.8,
                reflectance: 0.1,
                cull_mode: None,
                double_sided: true,
                ..default()
            });

            let chunk_world_pos = chunk_world_origin(chunk.coord);

            commands.entity(entity).insert((
                ChunkMesh,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::from_translation(chunk_world_pos),
                GlobalTransform::default(),
                Visibility::Visible,
            ));

            if debug_state.enabled {
                create_debug_aabb_for_chunk(&mut commands, &mut meshes, &mut materials, entity);
            }
        }
    }
}

fn chunk_rerendering_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_query: Query<(Entity, &Chunk), (With<Mesh3d>, Without<ChunkMesh>)>,
) {
    for (entity, chunk) in chunk_query.iter() {
        commands.entity(entity).remove::<Mesh3d>();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>();

        if let Some(mesh) = generate_chunk_mesh(chunk) {
            let mesh_handle = meshes.add(mesh);
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 0.3),
                metallic: 0.0,
                perceptual_roughness: 0.8,
                reflectance: 0.1,
                cull_mode: None,
                double_sided: true,
                ..default()
            });

            commands.entity(entity).insert((
                ChunkMesh,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Visibility::Visible,
            ));
        }
    }
}

fn voxel_highlight_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    interaction: Res<PlayerInteraction>,
    highlight_query: Query<Entity, With<VoxelHighlight>>,
    chunk_query: Query<&crate::world::Chunk>,
    world: Res<crate::world::World>,
) {
    for entity in highlight_query.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(selected_voxel_pos) = interaction.selected_voxel_world_pos {
        if let Some(voxel) = world.get_voxel_at_world(selected_voxel_pos, &chunk_query) {
            if voxel.is_solid() {
                let highlight_pos = selected_voxel_pos - Vec3::splat(VOXEL_SIZE / 2.0);

                let highlight_mesh = create_highlight_wireframe();
                let mesh_handle = meshes.add(highlight_mesh);
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
                    Transform::from_translation(highlight_pos),
                    GlobalTransform::default(),
                    Name::new("Voxel Highlight"),
                ));
            }
        }
    }
}

fn create_highlight_wireframe() -> Mesh {
    let size = VOXEL_SIZE;

    let vertices = vec![
        [0.0, 0.0, 0.0],
        [size, 0.0, 0.0],
        [size, 0.0, size],
        [0.0, 0.0, size],
        [0.0, size, 0.0],
        [size, size, 0.0],
        [size, size, size],
        [0.0, size, size],
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
                    children.iter().any(|child| {
                        debug_query.get(child).is_ok()
                    })
                } else {
                    false
                };

                if !has_debug_aabb {
                    create_debug_aabb_for_chunk(&mut commands, &mut meshes, &mut materials, chunk_entity);
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

    let mut mesh = Mesh::new(
        PrimitiveTopology::LineList,
        RenderAssetUsages::RENDER_WORLD,
    );
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

fn generate_chunk_mesh(chunk: &Chunk) -> Option<Mesh> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for x in 0..CHUNK_VOXELS_SIZE {
        for y in 0..CHUNK_VOXELS_HEIGHT {
            for z in 0..CHUNK_VOXELS_SIZE {
                if let Some(voxel) = chunk.get_voxel(x, y, z) {
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
                            local_pos,
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
    extended_normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 8]);
    extended_uvs.extend_from_slice(&[[0.0, 0.0]; 8]);

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
    mesh.insert_indices(Indices::U32(extended_indices));

    Some(mesh)
}

fn add_voxel_faces(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    pos: Vec3,
    chunk: &Chunk,
    x: usize,
    y: usize,
    z: usize,
) {
    let faces = [
        (
            should_render_face(chunk, x, y, z, -1, 0, 0),
            VoxelFace::NegativeX,
        ),
        (
            should_render_face(chunk, x, y, z, 1, 0, 0),
            VoxelFace::PositiveX,
        ),
        (
            should_render_face(chunk, x, y, z, 0, -1, 0),
            VoxelFace::NegativeY,
        ),
        (
            should_render_face(chunk, x, y, z, 0, 1, 0),
            VoxelFace::PositiveY,
        ),
        (
            should_render_face(chunk, x, y, z, 0, 0, -1),
            VoxelFace::NegativeZ,
        ),
        (
            should_render_face(chunk, x, y, z, 0, 0, 1),
            VoxelFace::PositiveZ,
        ),
    ];

    for (should_render, face) in faces.iter() {
        if *should_render {
            add_face(vertices, indices, normals, uvs, pos, *face);
        }
    }
}

fn should_render_face(
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

fn add_face(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    pos: Vec3,
    face: VoxelFace,
) {
    let start_vertex = vertices.len() as u32;
    let face_vertices = face.get_vertices(pos, VOXEL_SIZE);
    let face_normal = face.get_normal();

    vertices.extend_from_slice(&face_vertices);
    normals.extend_from_slice(&[[face_normal.x, face_normal.y, face_normal.z]; 4]);
    uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

    indices.extend_from_slice(&[
        start_vertex,
        start_vertex + 1,
        start_vertex + 2,
        start_vertex,
        start_vertex + 2,
        start_vertex + 3,
    ]);
}

fn force_rerender_system(
    mut commands: Commands,
    rerender_query: Query<Entity, With<crate::player::NeedsRerender>>,
) {
    for entity in rerender_query.iter() {
        commands
            .entity(entity)
            .remove::<crate::player::NeedsRerender>();
    }
}
