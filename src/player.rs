use crate::voxel::{VOXEL_SIZE, Voxel, VoxelType, VoxelFace};
use crate::world::{initial_player_spawn_position, InitialWorldGeneration, World};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::time::Fixed;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowFocused};
use bevy_rapier3d::prelude::*;

const PLAYER_WALK_SPEED: f32 = 4.5;
const PLAYER_SPRINT_MULTIPLIER: f32 = 1.8;
const PLAYER_GRAVITY: f32 = 25.0;
const PLAYER_MAX_FALL_SPEED: f32 = 40.0;
const PLAYER_JUMP_SPEED: f32 = 6.5;
const PLAYER_STEP_HEIGHT: f32 = 0.5;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component, Default)]
pub struct PlayerMotor {
    pub vertical_velocity: f32,
}

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
            .add_systems(Startup, setup_cursor_grab)
            .add_systems(
                Update,
                (
                    handle_window_focus_events,
                    sync_cursor_with_window_focus,
                    handle_cursor_grab,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    player_look,
                    voxel_interaction,
                    voxel_selection,
                ),
            );
        app.add_systems(FixedUpdate, player_movement);
    }
}

pub fn spawn_player(commands: &mut Commands) {
    let player = commands
        .spawn((
            Player,
            PlayerMotor::default(),
            RigidBody::KinematicPositionBased,
            Collider::cuboid(0.25, 1.0, 0.25),
            KinematicCharacterController {
                translation: Some(Vec3::ZERO),
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(PLAYER_STEP_HEIGHT),
                    min_width: CharacterLength::Absolute(0.2),
                    include_dynamic_bodies: false,
                }),
                snap_to_ground: Some(CharacterLength::Absolute(0.1)),
                ..default()
            },
            Transform::from_translation(initial_player_spawn_position()),
            GlobalTransform::default(),
        ))
        .id();

    let camera = commands
        .spawn((
            PlayerCamera,
            Camera3d::default(),
            Transform::from_xyz(0.0, 1.6, 0.0),
            GlobalTransform::default(),
        ))
        .id();

    commands.entity(player).add_child(camera);
}

fn player_movement(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(
        &mut KinematicCharacterController,
        &Transform,
        Option<&KinematicCharacterControllerOutput>,
        &mut PlayerMotor,
    ), With<Player>>,
    time: Res<Time<Fixed>>,
) {
    if let Ok((mut controller, transform, controller_output, mut motor)) = player_query.single_mut() {
        let mut movement = Vec3::ZERO;
        let mut speed = PLAYER_WALK_SPEED;
        
        if keyboard_input.pressed(KeyCode::ShiftLeft) {
            speed *= PLAYER_SPRINT_MULTIPLIER;
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

        let grounded = controller_output.is_some_and(|output| output.grounded);
        if grounded && motor.vertical_velocity < 0.0 {
            motor.vertical_velocity = 0.0;
        }

        if keyboard_input.just_pressed(KeyCode::Space) && grounded {
            motor.vertical_velocity = PLAYER_JUMP_SPEED;
        } else {
            motor.vertical_velocity -= PLAYER_GRAVITY * time.delta_secs();
            motor.vertical_velocity = motor
                .vertical_velocity
                .clamp(-PLAYER_MAX_FALL_SPEED, PLAYER_JUMP_SPEED);
        }

        let horizontal = Vec3::new(movement.x, 0.0, movement.z);
        let normalized_horizontal = if horizontal.length() > 0.0 {
            horizontal.normalize()
        } else {
            Vec3::ZERO
        };

        let final_movement = Vec3::new(
            normalized_horizontal.x * speed,
            motor.vertical_velocity,
            normalized_horizontal.z * speed,
        ) * time.delta_secs();

        controller.translation = Some(final_movement);
    }
}

fn player_look(
    mut mouse_motion: MessageReader<MouseMotion>,
    mut player_query: Query<&mut Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    cursor_query: Query<&CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(cursor_options) = cursor_query.single() {
        if cursor_options.grab_mode != CursorGrabMode::Locked {
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
    mut window_cursor_query: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor_state: ResMut<CursorState>,
    generation_state: Res<InitialWorldGeneration>,
) {
    if !generation_state.finished {
        if let Ok((_window, mut cursor_options)) = window_cursor_query.single_mut() {
            release_cursor(&mut cursor_options);
        }
        return;
    }

    if let Ok((mut window, mut cursor_options)) = window_cursor_query.single_mut() {
        if keys.just_pressed(KeyCode::Escape)
            || keys.just_pressed(KeyCode::SuperLeft)
            || keys.just_pressed(KeyCode::SuperRight)
        {
            cursor_state.was_locked_before_focus_loss = false;
            release_cursor(&mut cursor_options);
        } else if mouse_input.just_pressed(MouseButton::Left) {
            if cursor_options.grab_mode == CursorGrabMode::None && window.focused {
                lock_cursor(&mut window, &mut cursor_options);
            }
        }
    }
}

fn handle_window_focus_events(
    mut focus_events: MessageReader<WindowFocused>,
    mut window_cursor_query: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor_state: ResMut<CursorState>,
) {
    for event in focus_events.read() {
        if let Ok((mut window, mut cursor_options)) = window_cursor_query.single_mut() {
            if event.focused {
                if cursor_state.was_locked_before_focus_loss {
                    lock_cursor(&mut window, &mut cursor_options);
                    cursor_state.was_locked_before_focus_loss = false;
                }
            } else {
                if cursor_options.grab_mode == CursorGrabMode::Locked {
                    cursor_state.was_locked_before_focus_loss = true;
                }
                release_cursor(&mut cursor_options);
            }
        }
    }
}

fn sync_cursor_with_window_focus(
    mut window_cursor_query: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut cursor_state: ResMut<CursorState>,
) {
    if let Ok((mut window, mut cursor_options)) = window_cursor_query.single_mut() {
        if !window.focused {
            if cursor_options.grab_mode == CursorGrabMode::Locked {
                cursor_state.was_locked_before_focus_loss = true;
            }

            if cursor_options.grab_mode != CursorGrabMode::None || !cursor_options.visible {
                release_cursor(&mut cursor_options);
            }
        } else if cursor_state.was_locked_before_focus_loss
            && cursor_options.grab_mode != CursorGrabMode::Locked
        {
            lock_cursor(&mut window, &mut cursor_options);
            cursor_state.was_locked_before_focus_loss = false;
        }
    }
}

fn setup_cursor_grab(
    mut window_cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if let Ok(mut cursor_options) = window_cursor_query.single_mut() {
        release_cursor(&mut cursor_options);
    }
}

fn lock_cursor(window: &mut Window, cursor_options: &mut CursorOptions) {
    let center = Vec2::new(window.width() * 0.5, window.height() * 0.5);
    window.set_cursor_position(Some(center));
    cursor_options.grab_mode = CursorGrabMode::Locked;
    cursor_options.visible = false;
}

fn release_cursor(cursor_options: &mut CursorOptions) {
    cursor_options.grab_mode = CursorGrabMode::None;
    cursor_options.visible = true;
}

fn raycast_solid_voxel(
    world: &World,
    chunk_query: &Query<&crate::world::Chunk>,
    start: Vec3,
    direction: Vec3,
    max_distance: f32,
) -> Option<(Vec3, Vec3)> {
    let ray_dir = direction.normalize_or_zero();
    if ray_dir == Vec3::ZERO {
        return None;
    }

    let mut voxel_x = (start.x / VOXEL_SIZE).floor() as i32;
    let mut voxel_y = (start.y / VOXEL_SIZE).floor() as i32;
    let mut voxel_z = (start.z / VOXEL_SIZE).floor() as i32;

    let step_x = ray_dir.x.signum() as i32;
    let step_y = ray_dir.y.signum() as i32;
    let step_z = ray_dir.z.signum() as i32;

    let delta_x = axis_delta_distance(ray_dir.x);
    let delta_y = axis_delta_distance(ray_dir.y);
    let delta_z = axis_delta_distance(ray_dir.z);

    let mut t_max_x = initial_axis_distance(start.x, ray_dir.x, voxel_x);
    let mut t_max_y = initial_axis_distance(start.y, ray_dir.y, voxel_y);
    let mut t_max_z = initial_axis_distance(start.z, ray_dir.z, voxel_z);

    let mut last_hit_normal = None;
    let mut distance_traveled = 0.0;

    while distance_traveled <= max_distance {
        let sample_pos = voxel_center_from_indices(voxel_x, voxel_y, voxel_z);
        if let Some(voxel) = world.get_voxel_at_world(sample_pos, chunk_query) {
            if voxel.is_solid() {
                return Some((sample_pos, last_hit_normal.unwrap_or(-ray_dir.signum())));
            }
        }

        if t_max_x <= t_max_y && t_max_x <= t_max_z {
            voxel_x += step_x;
            distance_traveled = t_max_x;
            t_max_x += delta_x;
            last_hit_normal = Some(Vec3::new(-(step_x as f32), 0.0, 0.0));
        } else if t_max_y <= t_max_z {
            voxel_y += step_y;
            distance_traveled = t_max_y;
            t_max_y += delta_y;
            last_hit_normal = Some(Vec3::new(0.0, -(step_y as f32), 0.0));
        } else {
            voxel_z += step_z;
            distance_traveled = t_max_z;
            t_max_z += delta_z;
            last_hit_normal = Some(Vec3::new(0.0, 0.0, -(step_z as f32)));
        }
    }

    None
}

fn axis_delta_distance(direction: f32) -> f32 {
    if direction == 0.0 {
        f32::INFINITY
    } else {
        VOXEL_SIZE / direction.abs()
    }
}

fn initial_axis_distance(origin: f32, direction: f32, voxel_index: i32) -> f32 {
    if direction > 0.0 {
        (((voxel_index + 1) as f32 * VOXEL_SIZE) - origin) / direction
    } else if direction < 0.0 {
        (origin - (voxel_index as f32 * VOXEL_SIZE)) / -direction
    } else {
        f32::INFINITY
    }
}

fn voxel_center_from_indices(x: i32, y: i32, z: i32) -> Vec3 {
    Vec3::new(
        (x as f32 + 0.5) * VOXEL_SIZE,
        (y as f32 + 0.5) * VOXEL_SIZE,
        (z as f32 + 0.5) * VOXEL_SIZE,
    )
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
    cursor_query: Query<&CursorOptions, With<PrimaryWindow>>,
    mut chunk_query_set: ParamSet<(
        Query<&crate::world::Chunk>,
        Query<&mut crate::world::Chunk>,
    )>,
    player_query: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    if let Ok(cursor_options) = cursor_query.single() {
        if cursor_options.grab_mode != CursorGrabMode::Locked {
            return;
        }
    }

    if let Some(selected_voxel_pos) = interaction.selected_voxel_world_pos {
        if mouse_input.just_pressed(MouseButton::Left) {
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
            commands
                .entity(*chunk_entity)
                .remove::<crate::render::ChunkMesh>()
                .remove::<Mesh3d>()
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .remove::<crate::physics::ChunkPhysics>()
                .remove::<Collider>()
                .insert(NeedsRerender);
        }
    }
}

#[derive(Component)]
pub struct NeedsRerender;
