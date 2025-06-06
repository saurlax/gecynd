use crate::voxel::{VOXEL_SIZE, VoxelType, VoxelWorld};
use crate::world::{CHUNK_HEIGHT, CHUNK_SIZE};
use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (update_voxel_colliders, respawn_fallen_player));
    }
}

#[derive(Component)]
pub struct VoxelCollider;

#[derive(Component)]
pub struct Player;

/// 更新加载的区块中的体素碰撞器
fn update_voxel_colliders(
    mut commands: Commands,
    voxel_world: Res<VoxelWorld>,
    colliders: Query<Entity, With<VoxelCollider>>,
) {
    if !voxel_world.is_changed() {
        return;
    }

    // 移除旧的碰撞器
    for entity in colliders.iter() {
        commands.entity(entity).despawn();
    }

    // 为每个加载的区块创建碰撞器
    for &chunk_pos in voxel_world.loaded_chunks.iter() {
        let start_x = chunk_pos.x * CHUNK_SIZE;
        let start_z = chunk_pos.z * CHUNK_SIZE;

        // 合并相邻的相同类型体素为更大的碰撞体
        let mut processed = vec![
            vec![vec![false; CHUNK_SIZE as usize]; CHUNK_HEIGHT as usize];
            CHUNK_SIZE as usize
        ];

        for local_x in 0..CHUNK_SIZE {
            for local_y in 0..CHUNK_HEIGHT {
                for local_z in 0..CHUNK_SIZE {
                    let world_x = start_x + local_x;
                    let world_y = local_y;
                    let world_z = start_z + local_z;

                    let voxel_pos = IVec3::new(world_x, world_y, world_z);
                    let lx = local_x as usize;
                    let ly = local_y as usize;
                    let lz = local_z as usize;

                    if processed[lx][ly][lz] || voxel_world.get_voxel(voxel_pos) == VoxelType::Air {
                        continue;
                    }

                    // 尝试向X方向扩展
                    let mut width = 1;
                    while lx + width < CHUNK_SIZE as usize
                        && !processed[lx + width][ly][lz]
                        && voxel_world.get_voxel(IVec3::new(
                            world_x + width as i32,
                            world_y,
                            world_z,
                        )) != VoxelType::Air
                    {
                        width += 1;
                    }

                    // 尝试向Z方向扩展
                    let mut depth = 1;
                    let mut can_expand_z = true;
                    while can_expand_z && lz + depth < CHUNK_SIZE as usize {
                        for wx in 0..width {
                            if processed[lx + wx][ly][lz + depth]
                                || voxel_world.get_voxel(IVec3::new(
                                    world_x + wx as i32,
                                    world_y,
                                    world_z + depth as i32,
                                )) == VoxelType::Air
                            {
                                can_expand_z = false;
                                break;
                            }
                        }
                        if can_expand_z {
                            depth += 1;
                        }
                    }

                    // 标记已处理的体素
                    for wx in 0..width {
                        for wz in 0..depth {
                            processed[lx + wx][ly][lz + wz] = true;
                        }
                    }

                    // 创建碰撞器
                    let world_pos = Vec3::new(
                        (world_x as f32 + width as f32 * 0.5 - 0.5) * VOXEL_SIZE,
                        (world_y as f32 + 0.5) * VOXEL_SIZE,
                        (world_z as f32 + depth as f32 * 0.5 - 0.5) * VOXEL_SIZE,
                    );

                    let collider_size = Vec3::new(
                        width as f32 * VOXEL_SIZE,
                        VOXEL_SIZE,
                        depth as f32 * VOXEL_SIZE,
                    );

                    commands.spawn((
                        Transform::from_translation(world_pos),
                        GlobalTransform::default(),
                        Collider::cuboid(
                            collider_size.x * 0.5,
                            collider_size.y * 0.5,
                            collider_size.z * 0.5,
                        ),
                        VoxelCollider,
                    ));
                }
            }
        }
    }
}

/// 如果玩家掉落到虚空，将其传送回安全位置
fn respawn_fallen_player(
    mut player_query: Query<(&mut Transform, &mut Velocity), With<Player>>,
    voxel_world: Res<VoxelWorld>,
) {
    for (mut transform, mut velocity) in player_query.iter_mut() {
        if transform.translation.y < -100.0 {
            // 寻找安全的生成位置
            for x in -5..=5 {
                for z in -5..=5 {
                    let mut highest_y = 0;

                    // 找到最高的实体方块
                    for y in 0..20 {
                        let pos = IVec3::new(x, y, z);
                        if voxel_world.get_voxel(pos) != VoxelType::Air {
                            highest_y = y;
                        }
                    }

                    // 检查上方是否有足够空间
                    let mut has_space = true;
                    for y in 1..3 {
                        // 检查2个方块高度的空间
                        if voxel_world.get_voxel(IVec3::new(x, highest_y + y, z)) != VoxelType::Air
                        {
                            has_space = false;
                            break;
                        }
                    }

                    if has_space {
                        transform.translation = Vec3::new(
                            x as f32 + 0.5,
                            (highest_y + 3) as f32, // 在地面上方3个方块
                            z as f32 + 0.5,
                        );
                        *velocity = Velocity::zero();
                        return;
                    }
                }
            }

            // 如果没找到安全位置，放在默认高度
            transform.translation = Vec3::new(0.5, 20.0, 0.5);
            *velocity = Velocity::zero();
        }
    }
}

// 玩家尺寸常量
pub const PLAYER_WIDTH: f32 = 0.5; // 宽度为0.5个世界单位
pub const PLAYER_HEIGHT: f32 = 1.0 + 26.0 / 32.0; // 高度为1+26/32个世界单位
pub const PLAYER_DEPTH: f32 = 10.0 / 32.0; // 深度为10/32个世界单位

// 玩家尺寸的半值（在碰撞检测和碰撞器创建中常用）
pub const PLAYER_HALF_WIDTH: f32 = PLAYER_WIDTH / 2.0;
pub const PLAYER_HALF_HEIGHT: f32 = PLAYER_HEIGHT / 2.0;
pub const PLAYER_HALF_DEPTH: f32 = PLAYER_DEPTH / 2.0;
