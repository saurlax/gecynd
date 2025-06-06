use crate::voxel::{SelectedVoxel, VoxelType, VoxelWorld};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use bevy_rapier3d::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_voxel_selection)
            .add_systems(Update, handle_mouse_input);
    }
}

fn handle_voxel_selection(
    mut commands: Commands,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    voxel_world: Res<VoxelWorld>,
) {
    let (camera, camera_transform) = match camera_query.single() {
        Ok(result) => result,
        Err(_) => return,
    };

    let window = match window_query.single() {
        Ok(result) => result,
        Err(_) => return,
    };

    // 仅在光标锁定时进行射线检测
    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        commands.remove_resource::<SelectedVoxel>();
        return;
    }

    // 使用屏幕中心进行射线检测
    let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);

    match camera.viewport_to_world(camera_transform, screen_center) {
        Ok(ray) => {
            let ray_dir: Vec3 = ray.direction.into();
            let ray_origin = ray.origin;

            // 使用体素光线投射算法 (DDA)
            if let Some((hit_voxel, hit_normal)) =
                voxel_ray_cast(&voxel_world, ray_origin, ray_dir, 10.0)
            {
                let place_position = hit_voxel + hit_normal;

                commands.insert_resource(SelectedVoxel {
                    position: hit_voxel,
                    place_position,
                });
            } else {
                commands.remove_resource::<SelectedVoxel>();
            }
        }
        Err(_) => {
            commands.remove_resource::<SelectedVoxel>();
        }
    }
}

fn handle_mouse_input(
    mut voxel_world: ResMut<VoxelWorld>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    selected_voxel: Option<Res<SelectedVoxel>>,
    window_query: Query<&Window, With<PrimaryWindow>>,
) {
    // 仅在光标锁定时处理鼠标输入
    let window = match window_query.single() {
        Ok(result) => result,
        Err(_) => return,
    };

    if window.cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }

    if let Some(selected) = selected_voxel {
        if mouse_button_input.just_pressed(MouseButton::Left) {
            // 破坏方块
            voxel_world.set_voxel(selected.position, VoxelType::Air);
        } else if mouse_button_input.just_pressed(MouseButton::Right) {
            // 放置方块
            voxel_world.set_voxel(selected.place_position, VoxelType::Stone);
        }
    }
}

/// 使用DDA算法进行体素光线投射
/// 返回命中的体素坐标和命中面的法线向量
fn voxel_ray_cast(
    voxel_world: &VoxelWorld,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(IVec3, IVec3)> {
    // 初始体素坐标
    let mut current_voxel = voxel_world.world_to_voxel_pos(origin);

    // 计算光线步进的方向
    let step_x = if direction.x > 0.0 {
        1
    } else if direction.x < 0.0 {
        -1
    } else {
        0
    };
    let step_y = if direction.y > 0.0 {
        1
    } else if direction.y < 0.0 {
        -1
    } else {
        0
    };
    let step_z = if direction.z > 0.0 {
        1
    } else if direction.z < 0.0 {
        -1
    } else {
        0
    };

    // 避免除以零
    let dir_x = if direction.x.abs() < 1e-6 {
        1e-6
    } else {
        direction.x
    };
    let dir_y = if direction.y.abs() < 1e-6 {
        1e-6
    } else {
        direction.y
    };
    let dir_z = if direction.z.abs() < 1e-6 {
        1e-6
    } else {
        direction.z
    };

    // 计算到下一个体素边界的距离
    let voxel_size = crate::voxel::VOXEL_SIZE;
    let next_voxel_boundary_x = if step_x > 0 {
        (current_voxel.x as f32 + 1.0) * voxel_size
    } else {
        current_voxel.x as f32 * voxel_size
    };
    let next_voxel_boundary_y = if step_y > 0 {
        (current_voxel.y as f32 + 1.0) * voxel_size
    } else {
        current_voxel.y as f32 * voxel_size
    };
    let next_voxel_boundary_z = if step_z > 0 {
        (current_voxel.z as f32 + 1.0) * voxel_size
    } else {
        current_voxel.z as f32 * voxel_size
    };

    // 计算到下一个体素边界所需的参数t
    let mut t_max_x = if dir_x != 0.0 {
        (next_voxel_boundary_x - origin.x) / dir_x
    } else {
        f32::MAX
    };
    let mut t_max_y = if dir_y != 0.0 {
        (next_voxel_boundary_y - origin.y) / dir_y
    } else {
        f32::MAX
    };
    let mut t_max_z = if dir_z != 0.0 {
        (next_voxel_boundary_z - origin.z) / dir_z
    } else {
        f32::MAX
    };

    // 计算t在各个轴上的增量
    let t_delta_x = if dir_x != 0.0 {
        voxel_size / dir_x.abs()
    } else {
        f32::MAX
    };
    let t_delta_y = if dir_y != 0.0 {
        voxel_size / dir_y.abs()
    } else {
        f32::MAX
    };
    let t_delta_z = if dir_z != 0.0 {
        voxel_size / dir_z.abs()
    } else {
        f32::MAX
    };

    // 记录从哪个面进入体素
    let mut face_normal = IVec3::ZERO;

    // 限制最大步数以避免无限循环
    let max_steps = (max_distance / voxel_size) as i32 + 1;

    // 如果起点已经在体素内，先检查当前体素
    if voxel_world.get_voxel(current_voxel) != VoxelType::Air {
        // 假设我们从-Z方向进入(向上看)
        return Some((current_voxel, IVec3::new(0, -1, 0)));
    }

    // 主循环，沿着光线步进
    for _ in 0..max_steps {
        // 确定下一步沿哪个轴移动
        if t_max_x < t_max_y && t_max_x < t_max_z {
            current_voxel.x += step_x;
            t_max_x += t_delta_x;
            face_normal = IVec3::new(-step_x, 0, 0);
        } else if t_max_y < t_max_z {
            current_voxel.y += step_y;
            t_max_y += t_delta_y;
            face_normal = IVec3::new(0, -step_y, 0);
        } else {
            current_voxel.z += step_z;
            t_max_z += t_delta_z;
            face_normal = IVec3::new(0, 0, -step_z);
        }

        // 检查当前体素是否命中
        if voxel_world.get_voxel(current_voxel) != VoxelType::Air {
            return Some((current_voxel, face_normal));
        }
    }

    None
}
