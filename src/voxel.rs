use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub const VOXEL_SIZE: f32 = 1.0 / 32.0;

pub struct VoxelPlugin;

impl Plugin for VoxelPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VoxelWorld::new())
            .add_systems(Startup, setup_voxel_materials)
            .add_systems(Update, (update_chunk_meshes, highlight_selected_voxel));
    }
}

#[derive(Resource)]
pub struct VoxelWorld {
    pub voxels: HashMap<IVec3, VoxelType>,
    pub loaded_chunks: HashSet<IVec3>,
    pub dirty_chunks: HashSet<IVec3>,
    pub chunk_size: i32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum VoxelType {
    Air,
    Stone,
    Dirt,
    Grass,
}

#[derive(Resource)]
pub struct VoxelMaterials {
    pub stone: Handle<StandardMaterial>,
    pub dirt: Handle<StandardMaterial>,
    pub grass: Handle<StandardMaterial>,
    pub highlight: Handle<StandardMaterial>,
}

#[derive(Component)]
pub struct VoxelChunk {
    pub position: IVec3,
    pub needs_update: bool,
}

#[derive(Component)]
pub struct HighlightBox;

#[derive(Component)]
pub struct VoxelEntity;

#[derive(Component)]
pub struct ChunkMesh {
    pub chunk_pos: IVec3,
}

impl VoxelWorld {
    pub fn new() -> Self {
        Self {
            voxels: HashMap::new(),
            loaded_chunks: HashSet::new(),
            dirty_chunks: HashSet::new(),
            chunk_size: 32,
        }
    }

    pub fn get_voxel(&self, pos: IVec3) -> VoxelType {
        self.voxels.get(&pos).copied().unwrap_or(VoxelType::Air)
    }

    pub fn set_voxel(&mut self, pos: IVec3, voxel_type: VoxelType) {
        if voxel_type == VoxelType::Air {
            self.voxels.remove(&pos);
        } else {
            self.voxels.insert(pos, voxel_type);
        }

        // Mark chunk as dirty
        let chunk_pos = IVec3::new(
            pos.x.div_euclid(self.chunk_size),
            0,
            pos.z.div_euclid(self.chunk_size),
        );
        self.dirty_chunks.insert(chunk_pos);
    }

    pub fn mark_chunk_dirty(&mut self, chunk_pos: IVec3) {
        self.dirty_chunks.insert(chunk_pos);
    }

    pub fn world_to_voxel_pos(&self, world_pos: Vec3) -> IVec3 {
        IVec3::new(
            (world_pos.x / VOXEL_SIZE).floor() as i32,
            (world_pos.y / VOXEL_SIZE).floor() as i32,
            (world_pos.z / VOXEL_SIZE).floor() as i32,
        )
    }

    pub fn voxel_to_world_pos(&self, voxel_pos: IVec3) -> Vec3 {
        Vec3::new(
            voxel_pos.x as f32 * VOXEL_SIZE,
            voxel_pos.y as f32 * VOXEL_SIZE,
            voxel_pos.z as f32 * VOXEL_SIZE,
        )
    }

    fn is_face_visible(&self, pos: IVec3, face_dir: IVec3) -> bool {
        let neighbor_pos = pos + face_dir;
        self.get_voxel(neighbor_pos) == VoxelType::Air
    }
}

fn setup_voxel_materials(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    let voxel_materials = VoxelMaterials {
        stone: materials.add(StandardMaterial {
            base_color: Color::srgb(0.6, 0.6, 0.6),
            ..default()
        }),
        dirt: materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.2, 0.1),
            ..default()
        }),
        grass: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.8, 0.2),
            ..default()
        }),
        highlight: materials.add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.3),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
    };

    commands.insert_resource(voxel_materials);
}

fn update_chunk_meshes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut voxel_world: ResMut<VoxelWorld>,
    materials: Res<VoxelMaterials>,
    existing_chunks: Query<(Entity, &ChunkMesh)>,
) {
    if voxel_world.dirty_chunks.is_empty() {
        return;
    }

    // Remove old chunk meshes
    for (entity, chunk_mesh) in existing_chunks.iter() {
        if voxel_world.dirty_chunks.contains(&chunk_mesh.chunk_pos) {
            commands.entity(entity).despawn();
        }
    }

    // Generate new chunk meshes
    let dirty_chunks: Vec<IVec3> = voxel_world.dirty_chunks.drain().collect();

    for chunk_pos in dirty_chunks {
        if let Some(mesh) = generate_chunk_mesh(&voxel_world, chunk_pos) {
            // 使用 Mesh3d 和 MeshMaterial3d 组件
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.stone.clone()),
                Transform::from_translation(Vec3::ZERO),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
                ChunkMesh { chunk_pos },
            ));
        }
    }
}

fn generate_chunk_mesh(voxel_world: &VoxelWorld, chunk_pos: IVec3) -> Option<Mesh> {
    let start_x = chunk_pos.x * voxel_world.chunk_size;
    let start_z = chunk_pos.z * voxel_world.chunk_size;

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    // Face directions: +X, -X, +Y, -Y, +Z, -Z
    let face_dirs = [
        IVec3::new(1, 0, 0),  // Right
        IVec3::new(-1, 0, 0), // Left
        IVec3::new(0, 1, 0),  // Top
        IVec3::new(0, -1, 0), // Bottom
        IVec3::new(0, 0, 1),  // Front
        IVec3::new(0, 0, -1), // Back
    ];

    let face_normals = [
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];

    for local_x in 0..voxel_world.chunk_size {
        for local_y in 0..16 {
            // Height limit
            for local_z in 0..voxel_world.chunk_size {
                let world_pos = IVec3::new(start_x + local_x, local_y, start_z + local_z);

                if voxel_world.get_voxel(world_pos) == VoxelType::Air {
                    continue;
                }

                // Check each face
                for (face_idx, &face_dir) in face_dirs.iter().enumerate() {
                    if voxel_world.is_face_visible(world_pos, face_dir) {
                        add_face_to_mesh(
                            &mut vertices,
                            &mut indices,
                            &mut normals,
                            &mut uvs,
                            world_pos,
                            face_idx,
                            &face_normals[face_idx],
                        );
                    }
                }
            }
        }
    }

    if vertices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    Some(mesh)
}

fn add_face_to_mesh(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    voxel_pos: IVec3,
    face_idx: usize,
    normal: &Vec3,
) {
    let world_pos = Vec3::new(
        voxel_pos.x as f32 * VOXEL_SIZE,
        voxel_pos.y as f32 * VOXEL_SIZE,
        voxel_pos.z as f32 * VOXEL_SIZE,
    );

    let half_size = VOXEL_SIZE * 0.5;
    let base_vertex_idx = vertices.len() as u32;

    // Define face vertices based on face direction
    let face_vertices = match face_idx {
        0 => [
            // +X (Right)
            [
                world_pos.x + half_size,
                world_pos.y - half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y + half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y + half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y - half_size,
                world_pos.z + half_size,
            ],
        ],
        1 => [
            // -X (Left)
            [
                world_pos.x - half_size,
                world_pos.y - half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y + half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y + half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y - half_size,
                world_pos.z - half_size,
            ],
        ],
        2 => [
            // +Y (Top)
            [
                world_pos.x - half_size,
                world_pos.y + half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y + half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y + half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y + half_size,
                world_pos.z - half_size,
            ],
        ],
        3 => [
            // -Y (Bottom)
            [
                world_pos.x - half_size,
                world_pos.y - half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y - half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y - half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y - half_size,
                world_pos.z + half_size,
            ],
        ],
        4 => [
            // +Z (Front)
            [
                world_pos.x - half_size,
                world_pos.y - half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y - half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y + half_size,
                world_pos.z + half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y + half_size,
                world_pos.z + half_size,
            ],
        ],
        5 => [
            // -Z (Back)
            [
                world_pos.x + half_size,
                world_pos.y - half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y - half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x - half_size,
                world_pos.y + half_size,
                world_pos.z - half_size,
            ],
            [
                world_pos.x + half_size,
                world_pos.y + half_size,
                world_pos.z - half_size,
            ],
        ],
        _ => unreachable!(),
    };

    // Add vertices
    for vertex in face_vertices {
        vertices.push(vertex);
        normals.push([normal.x, normal.y, normal.z]);
    }

    // Add UVs
    uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);

    // Add indices (two triangles per face)
    indices.extend_from_slice(&[
        base_vertex_idx,
        base_vertex_idx + 1,
        base_vertex_idx + 2,
        base_vertex_idx,
        base_vertex_idx + 2,
        base_vertex_idx + 3,
    ]);
}

#[derive(Resource)]
pub struct SelectedVoxel {
    pub position: IVec3,
    pub place_position: IVec3,
}

// 在高亮函数中使用正确的组件集合
fn highlight_selected_voxel(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<VoxelMaterials>,
    highlight_boxes: Query<Entity, With<HighlightBox>>,
    selected_voxel: Option<Res<SelectedVoxel>>,
    voxel_world: Res<VoxelWorld>,
) {
    // Remove existing highlight boxes
    for entity in highlight_boxes.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(selected) = selected_voxel {
        let world_pos = voxel_world.voxel_to_world_pos(selected.position);

        // 创建MC风格的黑色线框，使用 Mesh3d 和 MeshMaterial3d 组件
        commands.spawn((
            Mesh3d(meshes.add(create_wireframe_cube(VOXEL_SIZE * 1.001))),
            MeshMaterial3d(materials.highlight.clone()),
            Transform::from_translation(world_pos),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            HighlightBox,
        ));
    }
}

fn create_wireframe_cube(size: f32) -> Mesh {
    let half_size = size * 0.5;

    // 定义立方体的8个顶点
    let vertices = vec![
        // 底面
        [-half_size, -half_size, -half_size], // 0
        [half_size, -half_size, -half_size],  // 1
        [half_size, -half_size, half_size],   // 2
        [-half_size, -half_size, half_size],  // 3
        // 顶面
        [-half_size, half_size, -half_size], // 4
        [half_size, half_size, -half_size],  // 5
        [half_size, half_size, half_size],   // 6
        [-half_size, half_size, half_size],  // 7
    ];

    // 定义边为线段
    let indices = vec![
        // 底面边
        0, 1, 1, 2, 2, 3, 3, 0, // 顶面边
        4, 5, 5, 6, 6, 7, 7, 4, // 竖直边
        0, 4, 1, 5, 2, 6, 3, 7,
    ];

    let normals = vec![[0.0, 1.0, 0.0]; vertices.len()];
    let uvs = vec![[0.0, 0.0]; vertices.len()];

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::LineList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}
