use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::voxel::VOXEL_SIZE;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_player, setup_cursor_grab))
           .add_systems(Update, (player_movement, player_look, cursor_grab, handle_window_focus));
    }
}

fn spawn_player(mut commands: Commands) {
    let player = commands.spawn((
        Player,
        RigidBody::KinematicPositionBased,
        Collider::cuboid(0.25, 1.0, 0.25),
        KinematicCharacterController {
            translation: Some(Vec3::ZERO),
            ..default()
        },
        Transform::from_xyz(8.0, 80.0, 8.0),
        GlobalTransform::default(),
    )).id();

    // 将相机作为玩家的子组件
    let camera = commands.spawn((
        PlayerCamera,
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.6, 0.0), // 相对于玩家的位置
        GlobalTransform::default(),
    )).id();

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
    // 只有在鼠标被锁定时才处理鼠标移动
    if let Ok(window) = window_query.single() {
        if window.cursor_options.grab_mode != CursorGrabMode::Locked {
            // 清空鼠标事件以防止累积
            for _ in mouse_motion.read() {}
            return;
        }
    }

    if let (Ok(mut player_transform), Ok(mut camera_transform)) = 
        (player_query.single_mut(), camera_query.single_mut()) {
        
        let mut delta = Vec2::ZERO;
        for motion in mouse_motion.read() {
            delta += motion.delta;
        }
        
        if delta != Vec2::ZERO {
            let sensitivity = 0.002;
            
            // 水平旋转（Y轴）- 影响玩家
            let yaw = -delta.x * sensitivity;
            player_transform.rotate_y(yaw);
            
            // 垂直旋转（X轴）- 影响相机
            let pitch = -delta.y * sensitivity;
            camera_transform.rotate_local_x(pitch);
            
            // 限制相机俯仰角
            let euler = camera_transform.rotation.to_euler(EulerRot::XYZ);
            let clamped_pitch = euler.0.clamp(-1.5, 1.5); // 约85度
            camera_transform.rotation = Quat::from_euler(EulerRot::XYZ, clamped_pitch, 0.0, 0.0);
        }
    }
}

fn cursor_grab(
    keys: Res<ButtonInput<KeyCode>>,
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = window_query.single_mut() {
        if keys.just_pressed(KeyCode::Escape) {
            match window.cursor_options.grab_mode {
                CursorGrabMode::None => {
                    window.cursor_options.grab_mode = CursorGrabMode::Locked;
                    window.cursor_options.visible = false;
                }
                _ => {
                    window.cursor_options.grab_mode = CursorGrabMode::None;
                    window.cursor_options.visible = true;
                }
            }
        }
    }
}

fn setup_cursor_grab(mut window_query: Query<&mut Window, With<PrimaryWindow>>) {
    if let Ok(mut window) = window_query.single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    }
}

fn handle_window_focus(
    mut window_query: Query<&mut Window, With<PrimaryWindow>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    if let Ok(mut window) = window_query.single_mut() {
        // 当鼠标点击且当前未锁定时，锁定鼠标
        if mouse_input.just_pressed(MouseButton::Left) && window.cursor_options.grab_mode == CursorGrabMode::None {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
    }
}
