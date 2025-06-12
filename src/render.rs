use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

use crate::voxel::VOXEL_SIZE;
use crate::world::{CHUNK_SIZE, CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, Chunk};
use crate::player::PlayerInteraction;

#[derive(Component)]
pub struct ChunkMesh;

#[derive(Component)]
pub struct VoxelHighlight;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_lighting)
            .add_systems(Update, (chunk_rendering_system, chunk_rerendering_system, voxel_highlight_system));
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

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 60.0,
        affects_lightmapped_meshes: false,
    });
}

fn chunk_rendering_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_query: Query<(Entity, &Chunk), (Without<Mesh3d>, Without<ChunkMesh>)>,
) {
    for (entity, chunk) in chunk_query.iter() {
        if let Some(mesh) = generate_chunk_mesh(chunk) {
            let mesh_handle = meshes.add(mesh);
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 0.3),
                metallic: 0.0,
                perceptual_roughness: 0.8,
                reflectance: 0.1,
                ..default()
            });

            commands.entity(entity).insert((
                ChunkMesh,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::from_xyz(
                    chunk.coord.x as f32 * CHUNK_SIZE as f32,
                    0.0,
                    chunk.coord.z as f32 * CHUNK_SIZE as f32,
                ),
                GlobalTransform::default(),
            ));
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
        // 移除旧的渲染组件
        commands.entity(entity).remove::<Mesh3d>();
        commands.entity(entity).remove::<MeshMaterial3d<StandardMaterial>>();
        
        if let Some(mesh) = generate_chunk_mesh(chunk) {
            let mesh_handle = meshes.add(mesh);
            let material_handle = materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 0.3),
                metallic: 0.0,
                perceptual_roughness: 0.8,
                reflectance: 0.1,
                ..default()
            });

            commands.entity(entity).insert((
                ChunkMesh,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
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
    _world: Res<crate::world::World>,
) {
    for entity in highlight_query.iter() {
        commands.entity(entity).despawn();
    }
    
    if let Some((chunk_coord, x, y, z)) = interaction.selected_voxel {
        if let Some(chunk_entity) = _world.chunks.get(&chunk_coord) {
            if let Ok(chunk) = chunk_query.get(*chunk_entity) {
                if let Some(voxel) = chunk.get_voxel(x, y, z) {
                    if voxel.is_solid() {
                        let world_pos = Vec3::new(
                            chunk_coord.x as f32 * CHUNK_SIZE as f32 + x as f32 * VOXEL_SIZE,
                            y as f32 * VOXEL_SIZE,
                            chunk_coord.z as f32 * CHUNK_SIZE as f32 + z as f32 * VOXEL_SIZE,
                        );
                        
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
                            Transform::from_translation(world_pos),
                            GlobalTransform::default(),
                            Name::new("Voxel Highlight"),
                        ));
                    }
                }
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
        0, 1, 1, 2, 2, 3, 3, 0,
        4, 5, 5, 6, 6, 7, 7, 4,
        0, 4, 1, 5, 2, 6, 3, 7,
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
    
    mesh
}

fn generate_chunk_mesh(chunk: &Chunk) -> Option<Mesh> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    let mut face_count = 0;

    for x in 0..CHUNK_VOXELS_SIZE {
        for y in 0..CHUNK_VOXELS_HEIGHT {
            for z in 0..CHUNK_VOXELS_SIZE {
                if let Some(voxel) = chunk.get_voxel(x, y, z) {
                    if voxel.is_solid() {
                        let old_vertex_count = vertices.len();
                        add_voxel_faces(
                            &mut vertices,
                            &mut indices,
                            &mut normals,
                            &mut uvs,
                            Vec3::new(
                                x as f32 * VOXEL_SIZE,
                                y as f32 * VOXEL_SIZE,
                                z as f32 * VOXEL_SIZE,
                            ),
                            chunk,
                            x,
                            y,
                            z,
                        );
                        let new_faces = (vertices.len() - old_vertex_count) / 4;
                        face_count += new_faces;
                    }
                }
            }
        }
    }

    if vertices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));

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
        (should_render_face(chunk, x, y, z, -1, 0, 0), 0),
        (should_render_face(chunk, x, y, z, 1, 0, 0), 1),
        (should_render_face(chunk, x, y, z, 0, -1, 0), 2),
        (should_render_face(chunk, x, y, z, 0, 1, 0), 3),
        (should_render_face(chunk, x, y, z, 0, 0, -1), 4),
        (should_render_face(chunk, x, y, z, 0, 0, 1), 5),
    ];

    for (should_render, face_index) in faces.iter() {
        if *should_render {
            add_face(vertices, indices, normals, uvs, pos, *face_index);
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
    face_index: usize,
) {
    let start_vertex = vertices.len() as u32;
    let size = VOXEL_SIZE;

    let (face_vertices, face_normal) = match face_index {
        0 => (
            [
                // Left face (-X)
                [pos.x, pos.y, pos.z],
                [pos.x, pos.y, pos.z + size],
                [pos.x, pos.y + size, pos.z + size],
                [pos.x, pos.y + size, pos.z],
            ],
            [-1.0, 0.0, 0.0],
        ),
        1 => (
            [
                // Right face (+X)
                [pos.x + size, pos.y, pos.z],
                [pos.x + size, pos.y + size, pos.z],
                [pos.x + size, pos.y + size, pos.z + size],
                [pos.x + size, pos.y, pos.z + size],
            ],
            [1.0, 0.0, 0.0],
        ),
        2 => (
            [
                // Bottom face (-Y)
                [pos.x, pos.y, pos.z],
                [pos.x + size, pos.y, pos.z],
                [pos.x + size, pos.y, pos.z + size],
                [pos.x, pos.y, pos.z + size],
            ],
            [0.0, -1.0, 0.0],
        ),
        3 => (
            [
                // Top face (+Y)
                [pos.x, pos.y + size, pos.z],
                [pos.x, pos.y + size, pos.z + size],
                [pos.x + size, pos.y + size, pos.z + size],
                [pos.x + size, pos.y + size, pos.z],
            ],
            [0.0, 1.0, 0.0],
        ),
        4 => (
            [
                // Back face (-Z)
                [pos.x, pos.y, pos.z],
                [pos.x, pos.y + size, pos.z],
                [pos.x + size, pos.y + size, pos.z],
                [pos.x + size, pos.y, pos.z],
            ],
            [0.0, 0.0, -1.0],
        ),
        5 => (
            [
                // Front face (+Z)
                [pos.x, pos.y, pos.z + size],
                [pos.x + size, pos.y, pos.z + size],
                [pos.x + size, pos.y + size, pos.z + size],
                [pos.x, pos.y + size, pos.z + size],
            ],
            [0.0, 0.0, 1.0],
        ),
        _ => return,
    };

    vertices.extend_from_slice(&face_vertices);
    normals.extend_from_slice(&[face_normal; 4]);
    uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

    // 确保逆时针绕序
    indices.extend_from_slice(&[
        start_vertex,
        start_vertex + 1,
        start_vertex + 2,
        start_vertex,
        start_vertex + 2,
        start_vertex + 3,
    ]);
}
