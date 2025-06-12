use crate::voxel::{VOXEL_SIZE, Voxel, VoxelType};
use crate::world::{CHUNK_SIZE, CHUNK_VOXELS_HEIGHT, CHUNK_VOXELS_SIZE, ChunkCoord, World};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow, WindowFocused};
use bevy_rapier3d::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Resource)]
pub struct PlayerInteraction {
    pub selected_voxel: Option<(ChunkCoord, usize, usize, usize)>,
    pub hit_normal: Option<Vec3>,
    pub interaction_range: f32,
}

impl Default for PlayerInteraction {
    fn default() -> Self {
        Self {
            selected_voxel: None,
            hit_normal: None,
            interaction_range: 10.0,
        }
    }
}

#[derive(Resource)]
pub struct CursorState {
    pub was_locked_before_focus_loss: bool,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            was_locked_before_focus_loss: false,
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInteraction>()
            .init_resource::<CursorState>()
            .add_systems(Startup, (spawn_player, setup_cursor_grab))
            .add_systems(
                Update,
                (
                    player_movement,
                    player_look,
                    handle_cursor_grab,
                    handle_window_focus_events,
                    voxel_interaction,
                    voxel_selection,
                ),
            );
    }
}

fn spawn_player(mut commands: Commands) {
    let player = commands
        .spawn((
            Player,
            RigidBody::KinematicPositionBased,
            Collider::cuboid(0.25, 1.0, 0.25),
            KinematicCharacterController {
                translation: Some(Vec3::ZERO),
                ..default()
            },
            Transform::from_xyz(8.0, 80.0, 8.0),
            GlobalTransform::default(),
        ))
        .id();

    // 将相机作为玩家的子组件
    let camera = commands
        .spawn((
            PlayerCamera,
            Camera3d::default(),
            Transform::from_xyz(0.0, 1.6, 0.0), // 相对于玩家的位置
            GlobalTransform::default(),
        ))
        .id();

    commands.entity(player).add_child(camera);
}

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(&mut KinematicCharacterController, &Transform), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((mut controller, transform)) = player_query.single_mut() {
        let mut movement = Vec3::ZERO;
        let speed = 8.0; // 保持正常移动速度

        // 获取玩家的前进方向（基于Y轴旋转）
        let forward = -*transform.local_z();
        let right = *transform.local_x();

        if keyboard_input.pressed(KeyCode::KeyW) {
            movement += forward;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            movement -= forward;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            movement -= right;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            movement += right;
        }
        if keyboard_input.pressed(KeyCode::Space) {
            movement.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::ControlLeft) {
            movement.y -= 1.0;
        }

        // 归一化水平移动向量
        let horizontal = Vec3::new(movement.x, 0.0, movement.z);
        let normalized_horizontal = if horizontal.length() > 0.0 {
            horizontal.normalize()
        } else {
            Vec3::ZERO
        };

        let final_movement = Vec3::new(
            normalized_horizontal.x * speed,
            movement.y * speed,
            normalized_horizontal.z * speed,
        ) * time.delta_secs();

        controller.translation = Some(final_movement);
    }
}

fn player_look(
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(window) = window_query.single() {
        if window.cursor_options.grab_mode != CursorGrabMode::Locked {
            for _ in mouse_motion.read() {}
            return;
        }
    }

    if let (Ok(mut player_transform), Ok(mut camera_transform)) =
        (player_query.single_mut(), camera_query.single_mut())
    {
        let mut delta = Vec2::ZERO;
        for motion in mouse_motion.read() {
            delta += motion.delta;
        }

        if delta != Vec2::ZERO {
            let sensitivity = 0.002;

            let yaw = -delta.x * sensitivity;
            player_transform.rotate_y(yaw);

            let pitch = -delta.y * sensitivity;
            camera_transform.rotate_local_x(pitch);

            let euler = camera_transform.rotation.to_euler(EulerRot::XYZ);
            let clamped_pitch = euler.0.clamp(-1.5, 1.5);
            camera_transform.rotation = Quat::from_euler(EulerRot::XYZ, clamped_pitch, 0.0, 0.0);
        }
    }
}

fn handle_cursor_grab(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut cursor_state: ResMut<CursorState>,
) {
    if let Ok(mut window) = window_query.single_mut() {
        if keys.just_pressed(KeyCode::Escape) {
            cursor_state.was_locked_before_focus_loss = false;
            release_cursor(&mut window);
        } else if mouse_input.just_pressed(MouseButton::Left) {
            if window.cursor_options.grab_mode == CursorGrabMode::None && window.focused {
                lock_cursor(&mut window);
            }
        }
    }
}

fn handle_window_focus_events(
    mut focus_events: EventReader<WindowFocused>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mut cursor_state: ResMut<CursorState>,
) {
    for event in focus_events.read() {
        if let Ok(mut window) = window_query.single_mut() {
            if event.focused {
                if cursor_state.was_locked_before_focus_loss {
                    lock_cursor(&mut window);
                    cursor_state.was_locked_before_focus_loss = false;
                }
            } else {
                if window.cursor_options.grab_mode == CursorGrabMode::Locked {
                    cursor_state.was_locked_before_focus_loss = true;
                }
                release_cursor(&mut window);
            }
        }
    }
}

fn setup_cursor_grab(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.single_mut() {
        lock_cursor(&mut window);
    }
}

fn lock_cursor(window: &mut Window) {
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
    window.cursor_options.visible = false;
    
    // Move cursor to center of window
    window.set_cursor_position(Some(Vec2::new(window.width() / 2.0, window.height() / 2.0)));
}

fn release_cursor(window: &mut Window) {
    window.cursor_options.grab_mode = CursorGrabMode::None;
    window.cursor_options.visible = true;
}

fn raycast_solid_voxel(
    world: &World,
    chunk_query: &Query<&crate::world::Chunk>,
    start: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<((ChunkCoord, usize, usize, usize), Vec3)> {
    let normalized_dir = direction.normalize();
    let step_size = 0.05;
    let max_steps = (max_distance / step_size) as i32;

    let mut last_pos = start;

    for i in 1..max_steps {
        let current_pos = start + normalized_dir * (i as f32 * step_size);
        let chunk_coord = ChunkCoord::from_world_pos(current_pos);

        if let Some(chunk_entity) = world.chunks.get(&chunk_coord) {
            if let Ok(chunk) = chunk_query.get(*chunk_entity) {
                let chunk_world_x = chunk_coord.x as f32 * CHUNK_SIZE as f32;
                let chunk_world_z = chunk_coord.z as f32 * CHUNK_SIZE as f32;

                let local_x = current_pos.x - chunk_world_x;
                let local_y = current_pos.y;
                let local_z = current_pos.z - chunk_world_z;

                let voxel_x = (local_x / VOXEL_SIZE) as usize;
                let voxel_y = (local_y / VOXEL_SIZE) as usize;
                let voxel_z = (local_z / VOXEL_SIZE) as usize;

                // 确保坐标在有效范围内
                if voxel_x < CHUNK_VOXELS_SIZE
                    && voxel_y < CHUNK_VOXELS_HEIGHT
                    && voxel_z < CHUNK_VOXELS_SIZE
                {
                    // 检查这个体素是否为固体
                    if let Some(voxel) = chunk.get_voxel(voxel_x, voxel_y, voxel_z) {
                        if voxel.is_solid() {
                            // 计算击中的面法线
                            let hit_normal = calculate_hit_normal(
                                last_pos,
                                current_pos,
                                chunk_world_x,
                                chunk_world_z,
                                voxel_x,
                                voxel_y,
                                voxel_z,
                            );
                            return Some(((chunk_coord, voxel_x, voxel_y, voxel_z), hit_normal));
                        }
                    }
                }
            }
        }

        last_pos = current_pos;
    }

    None
}

// 计算射线击中的面法线
fn calculate_hit_normal(
    last_pos: Vec3,
    current_pos: Vec3,
    chunk_world_x: f32,
    chunk_world_z: f32,
    voxel_x: usize,
    voxel_y: usize,
    voxel_z: usize,
) -> Vec3 {
    // 计算方块的六个面的世界坐标
    let block_min_x = chunk_world_x + voxel_x as f32 * VOXEL_SIZE;
    let block_min_y = voxel_y as f32 * VOXEL_SIZE;
    let block_min_z = chunk_world_z + voxel_z as f32 * VOXEL_SIZE;
    let block_max_x = block_min_x + VOXEL_SIZE;
    let block_max_y = block_min_y + VOXEL_SIZE;
    let block_max_z = block_min_z + VOXEL_SIZE;

    // 计算射线方向
    let ray_dir = (current_pos - last_pos).normalize();

    // 确定射线从哪个面进入方块
    // 通过比较离射线起点最近的面来确定
    let t_x_min = if ray_dir.x != 0.0 {
        (block_min_x - last_pos.x) / ray_dir.x
    } else {
        f32::MAX
    };
    let t_x_max = if ray_dir.x != 0.0 {
        (block_max_x - last_pos.x) / ray_dir.x
    } else {
        f32::MAX
    };
    let t_y_min = if ray_dir.y != 0.0 {
        (block_min_y - last_pos.y) / ray_dir.y
    } else {
        f32::MAX
    };
    let t_y_max = if ray_dir.y != 0.0 {
        (block_max_y - last_pos.y) / ray_dir.y
    } else {
        f32::MAX
    };
    let t_z_min = if ray_dir.z != 0.0 {
        (block_min_z - last_pos.z) / ray_dir.z
    } else {
        f32::MAX
    };
    let t_z_max = if ray_dir.z != 0.0 {
        (block_max_z - last_pos.z) / ray_dir.z
    } else {
        f32::MAX
    };

    // 找出最小的正t值对应的面
    let mut min_t = f32::MAX;
    let mut normal = Vec3::ZERO;

    if t_x_min > 0.0 && t_x_min < min_t {
        min_t = t_x_min;
        normal = Vec3::new(-1.0, 0.0, 0.0); // -X面
    }
    if t_x_max > 0.0 && t_x_max < min_t {
        min_t = t_x_max;
        normal = Vec3::new(1.0, 0.0, 0.0); // +X面
    }
    if t_y_min > 0.0 && t_y_min < min_t {
        min_t = t_y_min;
        normal = Vec3::new(0.0, -1.0, 0.0); // -Y面
    }
    if t_y_max > 0.0 && t_y_max < min_t {
        min_t = t_y_max;
        normal = Vec3::new(0.0, 1.0, 0.0); // +Y面
    }
    if t_z_min > 0.0 && t_z_min < min_t {
        min_t = t_z_min;
        normal = Vec3::new(0.0, 0.0, -1.0); // -Z面
    }
    if t_z_max > 0.0 && t_z_max < min_t {
        min_t = t_z_max;
        normal = Vec3::new(0.0, 0.0, 1.0); // +Z面
    }

    normal
}

fn voxel_selection(
    mut interaction: ResMut<PlayerInteraction>,
    world: Res<World>,
    _player_query: Query<&Transform, With<Player>>,
    camera_query: Query<&GlobalTransform, (With<PlayerCamera>, Without<Player>)>,
    chunk_query: Query<&crate::world::Chunk>,
) {
    if let Ok(camera_transform) = camera_query.single() {
        let camera_pos = camera_transform.translation();
        let camera_forward = camera_transform.forward();

        match raycast_solid_voxel(
            &world,
            &chunk_query,
            camera_pos,
            *camera_forward,
            interaction.interaction_range,
        ) {
            Some((voxel, normal)) => {
                interaction.selected_voxel = Some(voxel);
                interaction.hit_normal = Some(normal);
            }
            None => {
                interaction.selected_voxel = None;
                interaction.hit_normal = None;
            }
        }
    }
}

fn get_placement_position(
    chunk_coord: ChunkCoord,
    x: usize,
    y: usize,
    z: usize,
    normal: Vec3,
) -> Option<(ChunkCoord, usize, usize, usize)> {
    // 根据命中面的法线确定放置方向
    let (dx, dy, dz) = if normal.x < -0.5 {
        (-1, 0, 0) // -X方向
    } else if normal.x > 0.5 {
        (1, 0, 0) // +X方向
    } else if normal.y < -0.5 {
        (0, -1, 0) // -Y方向
    } else if normal.y > 0.5 {
        (0, 1, 0) // +Y方向
    } else if normal.z < -0.5 {
        (0, 0, -1) // -Z方向
    } else if normal.z > 0.5 {
        (0, 0, 1) // +Z方向
    } else {
        (0, 1, 0) // 默认上方
    };

    let new_x = x as i32 + dx;
    let new_y = y as i32 + dy;
    let new_z = z as i32 + dz;

    // 检查是否在当前区块内
    if new_x >= 0
        && new_x < CHUNK_VOXELS_SIZE as i32
        && new_y >= 0
        && new_y < CHUNK_VOXELS_HEIGHT as i32
        && new_z >= 0
        && new_z < CHUNK_VOXELS_SIZE as i32
    {
        return Some((chunk_coord, new_x as usize, new_y as usize, new_z as usize));
    }

    // TODO: 处理跨区块的情况
    None
}

fn voxel_interaction(
    mut commands: Commands,
    world: Res<World>,
    interaction: Res<PlayerInteraction>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut chunk_query: Query<(Entity, &mut crate::world::Chunk)>,
) {
    if let Ok(window) = window_query.single() {
        if window.cursor_options.grab_mode != CursorGrabMode::Locked {
            return;
        }
    }

    if let Some((chunk_coord, x, y, z)) = interaction.selected_voxel {
        if mouse_input.just_pressed(MouseButton::Left) {
            // 破坏方块
            if let Some(chunk_entity) = world.chunks.get(&chunk_coord) {
                for (entity, mut chunk) in chunk_query.iter_mut() {
                    if entity == *chunk_entity {
                        // 确保是实心方块才能破坏
                        if let Some(voxel) = chunk.get_voxel(x, y, z) {
                            if voxel.is_solid() {
                                chunk.set_voxel(x, y, z, Voxel::new(VoxelType::Air));
                                // 重新生成网格
                                commands.entity(entity).remove::<crate::render::ChunkMesh>();
                                commands
                                    .entity(entity)
                                    .remove::<crate::physics::ChunkPhysics>();
                            }
                        }
                        break;
                    }
                }
            }
        } else if mouse_input.just_pressed(MouseButton::Right) {
            // 放置方块（在选中方块的相邻位置）
            if let Some(normal) = interaction.hit_normal {
                if let Some(place_pos) = get_placement_position(chunk_coord, x, y, z, normal) {
                    let (place_chunk_coord, place_x, place_y, place_z) = place_pos;
                    if let Some(chunk_entity) = world.chunks.get(&place_chunk_coord) {
                        for (entity, mut chunk) in chunk_query.iter_mut() {
                            if entity == *chunk_entity {
                                // 确保目标位置是空气才能放置
                                if let Some(voxel) = chunk.get_voxel(place_x, place_y, place_z) {
                                    if !voxel.is_solid() {
                                        chunk.set_voxel(
                                            place_x,
                                            place_y,
                                            place_z,
                                            Voxel::new(VoxelType::Stone),
                                        );
                                        // 重新生成网格
                                        commands
                                            .entity(entity)
                                            .remove::<crate::render::ChunkMesh>();
                                        commands
                                            .entity(entity)
                                            .remove::<crate::physics::ChunkPhysics>();
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}
