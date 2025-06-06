use crate::physics::{
    PLAYER_DEPTH, PLAYER_HALF_DEPTH, PLAYER_HALF_HEIGHT, PLAYER_HALF_WIDTH, PLAYER_HEIGHT,
    PLAYER_WIDTH, Player,
};
use crate::voxel::{VoxelType, VoxelWorld};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_rapier3d::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WireframeState::default())
            .add_systems(Startup, setup_camera)
            .add_systems(Startup, setup_cursor)
            .add_systems(Update, player_movement)
            .add_systems(Update, mouse_look)
            .add_systems(Update, toggle_cursor)
            .add_systems(Update, toggle_wireframes)
            .add_systems(Update, read_character_controller_output);
    }
}

#[derive(Component)]
pub struct FreeCamera {
    pub sensitivity: f32,
    pub speed: f32,
    pub jump_force: f32,
    pub on_ground: bool,
}

#[derive(Resource)]
pub struct WireframeState {
    pub render_wireframe_enabled: bool,  // 渲染网格显示状态
    pub physics_wireframe_enabled: bool, // 物理网格显示状态
}

impl Default for WireframeState {
    fn default() -> Self {
        Self {
            render_wireframe_enabled: false,
            physics_wireframe_enabled: false,
        }
    }
}

fn setup_camera(mut commands: Commands, voxel_world: Res<VoxelWorld>) {
    // 找到安全的生成位置
    let safe_pos = find_safe_spawn_position(&voxel_world);

    // 创建玩家物理实体，使用角色控制器
    let player_entity = commands
        .spawn((
            Transform::from_translation(safe_pos),
            GlobalTransform::default(),
            RigidBody::KinematicPositionBased, // 使用动力学位置控制
            Collider::capsule_y(PLAYER_HALF_HEIGHT - PLAYER_HALF_WIDTH, PLAYER_HALF_WIDTH), // 胶囊体形状，更适合角色控制器
            KinematicCharacterController {
                offset: CharacterLength::Absolute(0.01), // 小间隙避免数值问题
                max_slope_climb_angle: 0.6,              // 约34度
                min_slope_slide_angle: 0.4,              // 约23度
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(0.5), // 最大台阶高度
                    min_width: CharacterLength::Absolute(0.2),  // 最小台阶宽度
                    include_dynamic_bodies: true,
                }),
                snap_to_ground: Some(CharacterLength::Absolute(0.1)), // 贴地效果
                apply_impulse_to_dynamic_bodies: true,                // 推动动态物体
                up: Vec3::Y,                                          // 向上方向
                ..default()
            },
            Friction::coefficient(0.0), // 避免地面摩擦导致滑动异常
            Player,
            FreeCamera {
                sensitivity: 0.002,
                speed: 5.0,
                jump_force: 8.0,
                on_ground: false,
            },
        ))
        .id();

    // 添加摄像机作为玩家的子实体
    let camera_offset = Vec3::new(0.0, 0.5, 0.0); // 摄像机位于玩家头部
    commands.spawn((
        Camera::default(),
        Camera3d::default(),
        Transform::from_translation(camera_offset),
        GlobalTransform::default(),
        ChildOf(player_entity),
    ));
}

fn find_safe_spawn_position(voxel_world: &VoxelWorld) -> Vec3 {
    // 从原点向外搜索安全位置
    for distance in 0..10i32 {
        for x in -distance..=distance {
            for z in -distance..=distance {
                if (x.abs() != distance && z.abs() != distance) && distance > 0 {
                    continue; // 只检查当前"环"
                }

                // 找到该位置的地面高度
                let mut ground_y = -1;
                for y in 0..30 {
                    let pos = IVec3::new(x, y, z);
                    if voxel_world.get_voxel(pos) != VoxelType::Air {
                        ground_y = y;
                    }
                }

                if ground_y >= 0 {
                    // 检查上方是否有足够空间
                    let mut has_space = true;
                    for y in 1..3 {
                        // 需要至少2格高的空间
                        if voxel_world.get_voxel(IVec3::new(x, ground_y + y, z)) != VoxelType::Air {
                            has_space = false;
                            break;
                        }
                    }

                    if has_space {
                        return Vec3::new(
                            x as f32 + 0.5,
                            (ground_y + 2) as f32, // 放在地面上方2个单位
                            z as f32 + 0.5,
                        );
                    }
                }
            }
        }
    }

    // 默认返回高位置
    Vec3::new(0.5, 10.0, 0.5) // 简化返回，实际实现保持不变
}

fn setup_cursor(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = window_query.single_mut().unwrap();
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
}

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<
        (
            &mut KinematicCharacterController,
            &Transform,
            &mut FreeCamera,
        ),
        With<Player>,
    >,
    rapier_context: ReadRapierContext,
    time: Res<Time>,
) {
    for (mut character_controller, transform, mut camera) in player_query.iter_mut() {
        // 计算移动方向
        let mut movement_direction = Vec3::ZERO;
        let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);

        let forward = -Vec3::new(yaw.sin(), 0.0, yaw.cos()).normalize();
        let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin()).normalize();

        if keyboard_input.pressed(KeyCode::KeyW) {
            movement_direction += forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            movement_direction -= forward;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            movement_direction -= right;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            movement_direction += right;
        }

        movement_direction = movement_direction.normalize_or_zero();

        // 计算水平移动速度
        let horizontal_speed = camera.speed * time.delta_secs();
        let horizontal_velocity = movement_direction * horizontal_speed;

        // 计算垂直移动（重力和跳跃）
        let gravity = -9.81 * time.delta_secs();
        let mut vertical_velocity = gravity;

        // 跳跃逻辑
        if keyboard_input.just_pressed(KeyCode::Space) && camera.on_ground {
            vertical_velocity = camera.jump_force * time.delta_secs();
        }

        // 设置角色控制器的移动
        let translation = Vec3::new(
            horizontal_velocity.x,
            vertical_velocity,
            horizontal_velocity.z,
        );

        character_controller.translation = Some(translation);
    }
}

// 读取角色控制器输出
fn read_character_controller_output(
    mut player_query: Query<(&KinematicCharacterControllerOutput, &mut FreeCamera), With<Player>>,
) {
    for (controller_output, mut camera) in player_query.iter_mut() {
        // 更新地面状态
        camera.on_ground = controller_output.grounded;

        // 可以在这里处理碰撞信息
        for collision in &controller_output.collisions {
            // 可以在这里添加碰撞反馈效果，如声音或粒子效果
        }
    }
}

fn mouse_look(
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut camera_parent_query: Query<(&mut Transform, &FreeCamera), With<Player>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    let window = window_query.single().unwrap();

    // 仅在光标锁定时处理鼠标视角
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }

    for motion in mouse_motion_events.read() {
        for (mut transform, camera) in camera_parent_query.iter_mut() {
            let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

            yaw -= motion.delta.x * camera.sensitivity;
            pitch -= motion.delta.y * camera.sensitivity;
            pitch = pitch.clamp(-1.54, 1.54);

            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
        }
    }
}

fn toggle_cursor(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        let mut window = window_query.single_mut().unwrap();

        match window.cursor_options.grab_mode {
            CursorGrabMode::Locked => {
                window.cursor_options.grab_mode = CursorGrabMode::None;
                window.cursor_options.visible = true;
            }
            _ => {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
    }
}

fn toggle_wireframes(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut wireframe_state: ResMut<WireframeState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(Entity, &Mesh3d), With<crate::voxel::ChunkMesh>>,
    wireframe_query: Query<Entity, With<RenderWireframeEntity>>,
    mut commands: Commands,
    colliders: Query<Entity, With<Collider>>,
    collider_debug: Query<Entity, With<ColliderDebug>>,
) {
    // 按F1切换渲染网格
    if keyboard_input.just_pressed(KeyCode::F1) {
        wireframe_state.render_wireframe_enabled = !wireframe_state.render_wireframe_enabled;

        // 根据状态显示或隐藏渲染网格
        if wireframe_state.render_wireframe_enabled {
            // 为每个区块创建渲染线框
            for (chunk_entity, mesh_handle) in query.iter() {
                // 创建一个临时作用域来分离借用
                let wireframe_mesh = {
                    // 这个作用域内只有不可变借用
                    if let Some(mesh) = meshes.get(&mesh_handle.0) {
                        create_wireframe_from_mesh(mesh)
                    } else {
                        continue;
                    }
                }; // 不可变借用在这里结束

                // 创建材质
                let wireframe_material = materials.add(StandardMaterial {
                    base_color: Color::srgba(0.0, 1.0, 0.0, 0.3),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                });

                // 现在可以安全地可变借用 meshes
                let mesh_handle = meshes.add(wireframe_mesh);

                commands.spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(wireframe_material),
                    Transform::from_xyz(0.0, 0.001, 0.0),
                    GlobalTransform::default(),
                    Visibility::default(),
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                    RenderWireframeEntity,
                    ChildOf(chunk_entity),
                ));
            }
        } else {
            // 移除所有渲染线框
            for entity in wireframe_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }

    // 按F2切换物理网格
    if keyboard_input.just_pressed(KeyCode::F2) {
        wireframe_state.physics_wireframe_enabled = !wireframe_state.physics_wireframe_enabled;

        // 使用 ColliderDebug 组件来控制碰撞器的可见性
        if wireframe_state.physics_wireframe_enabled {
            // 为所有碰撞器添加 ColliderDebug::AlwaysRender
            for entity in colliders.iter() {
                commands.entity(entity).insert(ColliderDebug::AlwaysRender);
            }
        } else {
            // 为所有碰撞器添加 ColliderDebug::NeverRender
            for entity in colliders.iter() {
                commands.entity(entity).insert(ColliderDebug::NeverRender);
            }
        }
    }
}

// 标记渲染线框实体的组件
#[derive(Component)]
struct RenderWireframeEntity;

// 从网格创建线框
fn create_wireframe_from_mesh(mesh: &Mesh) -> Mesh {
    // 获取原始网格的数据
    let vertices: Vec<[f32; 3]> = if let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        match positions {
            VertexAttributeValues::Float32x3(positions) => positions.clone(),
            _ => return Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default()),
        }
    } else {
        return Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
    };
    
    // 保存顶点数量，因为后面vertices会被移动
    let vertex_count = vertices.len();
    
    // 创建新的线框网格，直接指定拓扑为LineList
    let mut new_mesh = Mesh::new(
        PrimitiveTopology::LineList,
        RenderAssetUsages::default(),
    );
    
    // 复制原始顶点位置
    new_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    
    // 设置法线（可选）
    let default_normal = vec![[0.0, 1.0, 0.0]; vertex_count];
    new_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, default_normal);
    
    // 设置UV（可选）
    let default_uv = vec![[0.0, 0.0]; vertex_count];
    new_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, default_uv);
    
    // 从三角形索引转换为线段索引
    if let Some(indices) = mesh.indices() {
        let triangle_indices = match indices {
            bevy::render::mesh::Indices::U16(indices) => {
                indices.iter().map(|&i| i as u32).collect::<Vec<_>>()
            }
            bevy::render::mesh::Indices::U32(indices) => indices.clone(),
        };

        let mut line_indices = Vec::new();
        for chunk in triangle_indices.chunks(3) {
            if chunk.len() == 3 {
                // 添加三角形的边
                line_indices.extend_from_slice(&[chunk[0], chunk[1]]);
                line_indices.extend_from_slice(&[chunk[1], chunk[2]]);
                line_indices.extend_from_slice(&[chunk[2], chunk[0]]);
            }
        }

        new_mesh.insert_indices(bevy::render::mesh::Indices::U32(line_indices));
    }

    new_mesh
}
