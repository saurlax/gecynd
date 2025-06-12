use crate::voxel::{VOXEL_SIZE, Voxel, VoxelType, VoxelFace};
use crate::world::World;
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
    pub selected_voxel_world_pos: Option<Vec3>,
    pub hit_face: Option<VoxelFace>,
    pub interaction_range: f32,
}

impl Default for PlayerInteraction {
    fn default() -> Self {
        Self {
            selected_voxel_world_pos: None,
            hit_face: None,
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
        let mut speed = 8.0;
        
        // Sprint when holding shift
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            speed *= 2.0;
        }

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
) -> Option<(Vec3, Vec3)> {
    let normalized_dir = direction.normalize();
    let step_size = 0.01; // 更小的步长以获得更精确的交点
    let max_steps = (max_distance / step_size) as i32;

    for i in 1..max_steps {
        let current_pos = start + normalized_dir * (i as f32 * step_size);
        
        if let Some(voxel) = world.get_voxel_at_world(current_pos, chunk_query) {
            if voxel.is_solid() {
                // 找到第一个固体方块，现在需要精确计算射线与方块的交点
                let voxel_center = world.get_voxel_center_at_world(current_pos).unwrap_or(current_pos);
                
                // 使用精确的射线-立方体相交算法
                if let Some(hit_info) = raycast_cube(start, normalized_dir, voxel_center, VOXEL_SIZE) {
                    return Some((voxel_center, hit_info.normal));
                }
            }
        }
    }

    None
}

#[derive(Debug)]
struct RayHitInfo {
    point: Vec3,
    normal: Vec3,
    distance: f32,
}

fn raycast_cube(ray_origin: Vec3, ray_dir: Vec3, cube_center: Vec3, cube_size: f32) -> Option<RayHitInfo> {
    let half_size = cube_size / 2.0;
    let cube_min = cube_center - Vec3::splat(half_size);
    let cube_max = cube_center + Vec3::splat(half_size);
    
    // 计算射线与立方体各面的交点参数t
    let inv_dir = Vec3::new(
        if ray_dir.x != 0.0 { 1.0 / ray_dir.x } else { f32::INFINITY },
        if ray_dir.y != 0.0 { 1.0 / ray_dir.y } else { f32::INFINITY },
        if ray_dir.z != 0.0 { 1.0 / ray_dir.z } else { f32::INFINITY },
    );
    
    let t1 = (cube_min - ray_origin) * inv_dir;
    let t2 = (cube_max - ray_origin) * inv_dir;
    
    let t_min = t1.min(t2);
    let t_max = t1.max(t2);
    
    let t_near = t_min.x.max(t_min.y).max(t_min.z);
    let t_far = t_max.x.min(t_max.y).min(t_max.z);
    
    // 检查是否有交点
    if t_near > t_far || t_far < 0.0 {
        return None;
    }
    
    // 选择合适的t值（如果射线起点在立方体内部，使用t_far；否则使用t_near）
    let t = if t_near < 0.0 { t_far } else { t_near };
    
    if t < 0.0 {
        return None;
    }
    
    let hit_point = ray_origin + ray_dir * t;
    
    // 确定击中的面
    let relative_pos = hit_point - cube_center;
    let abs_pos = relative_pos.abs();
    
    let normal = if abs_pos.x >= abs_pos.y && abs_pos.x >= abs_pos.z {
        // X面
        if relative_pos.x > 0.0 {
            Vec3::new(1.0, 0.0, 0.0)  // +X面
        } else {
            Vec3::new(-1.0, 0.0, 0.0) // -X面
        }
    } else if abs_pos.y >= abs_pos.z {
        // Y面
        if relative_pos.y > 0.0 {
            Vec3::new(0.0, 1.0, 0.0)  // +Y面
        } else {
            Vec3::new(0.0, -1.0, 0.0) // -Y面
        }
    } else {
        // Z面
        if relative_pos.z > 0.0 {
            Vec3::new(0.0, 0.0, 1.0)  // +Z面
        } else {
            Vec3::new(0.0, 0.0, -1.0) // -Z面
        }
    };
    
    Some(RayHitInfo {
        point: hit_point,
        normal,
        distance: t,
    })
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
            Some((voxel_pos, normal)) => {
                interaction.selected_voxel_world_pos = Some(voxel_pos);
                interaction.hit_face = VoxelFace::from_normal(normal);
            }
            None => {
                interaction.selected_voxel_world_pos = None;
                interaction.hit_face = None;
            }
        }
    }
}

fn calculate_placement_position(voxel_center: Vec3, face: VoxelFace) -> Vec3 {
    let (dx, dy, dz) = face.get_offset();
    voxel_center + Vec3::new(dx as f32, dy as f32, dz as f32) * VOXEL_SIZE
}

fn voxel_interaction(
    mut commands: Commands,
    world: Res<World>,
    interaction: Res<PlayerInteraction>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut chunk_query_set: ParamSet<(
        Query<&crate::world::Chunk>,
        Query<&mut crate::world::Chunk>,
    )>,
    player_query: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    if let Ok(window) = window_query.single() {
        if window.cursor_options.grab_mode != CursorGrabMode::Locked {
            return;
        }
    }

    if let Some(selected_voxel_pos) = interaction.selected_voxel_world_pos {
        if mouse_input.just_pressed(MouseButton::Left) {
            // Break block at selected position
            if world.set_voxel_at_world(selected_voxel_pos, Voxel::new(VoxelType::Air), &mut chunk_query_set.p1()) {
                mark_chunk_for_update(&mut commands, &world, selected_voxel_pos);
            }
        }
        
        if mouse_input.pressed(MouseButton::Right) {
            static mut LAST_PLACE_TIME: f32 = 0.0;
            let current_time = time.elapsed_secs();
            
            unsafe {
                if current_time - LAST_PLACE_TIME > 0.1 {
                    if let Some(hit_face) = interaction.hit_face {
                        let place_pos = calculate_placement_position(selected_voxel_pos, hit_face);
                        
                        // Check collision with player
                        if let Ok(player_transform) = player_query.single() {
                            let player_pos = player_transform.translation;
                            let player_min = player_pos + Vec3::new(-0.25, 0.0, -0.25);
                            let player_max = player_pos + Vec3::new(0.25, 2.0, 0.25);
                            let voxel_min = place_pos - Vec3::splat(VOXEL_SIZE / 2.0);
                            let voxel_max = place_pos + Vec3::splat(VOXEL_SIZE / 2.0);
                            
                            let overlaps = player_min.x < voxel_max.x && player_max.x > voxel_min.x &&
                                          player_min.y < voxel_max.y && player_max.y > voxel_min.y &&
                                          player_min.z < voxel_max.z && player_max.z > voxel_min.z;
                            
                            if overlaps {
                                return;
                            }
                        }
                        
                        // Check if placement position is valid and place block
                        if let Some(existing_voxel) = world.get_voxel_at_world(place_pos, &chunk_query_set.p0()) {
                            if !existing_voxel.is_solid() {
                                if world.set_voxel_at_world(place_pos, Voxel::new(VoxelType::Stone), &mut chunk_query_set.p1()) {
                                    mark_chunk_for_update(&mut commands, &world, place_pos);
                                    LAST_PLACE_TIME = current_time;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn mark_chunk_for_update(commands: &mut Commands, world: &World, world_pos: Vec3) {
    if let Some((chunk_coord, _, _, _)) = world.world_to_voxel(world_pos) {
        if let Some(chunk_entity) = world.chunks.get(&chunk_coord) {
            // 强制重新生成网格，确保立即可见
            commands.entity(*chunk_entity).remove::<crate::render::ChunkMesh>();
            commands.entity(*chunk_entity).remove::<crate::physics::ChunkPhysics>();
            
            // 添加一个标记确保在下一帧重新渲染
            commands.entity(*chunk_entity).insert(NeedsRerender);
        }
    }
}

#[derive(Component)]
pub struct NeedsRerender;
