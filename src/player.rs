use crate::voxel::{VOXEL_SIZE, Voxel, VoxelFace, VoxelType};
use crate::world::{InitialWorldGeneration, World, initial_player_spawn_position};
use bevy::input::mouse::MouseMotion;
use bevy::platform::collections::HashSet;
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
const MIN_BRUSH_SIZE: i32 = 4;
const MAX_BRUSH_SIZE: i32 = 16;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component, Default)]
pub struct PlayerMotor {
    pub vertical_velocity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushShape {
    Single,
    Cube,
    Sphere,
}

#[derive(Resource)]
pub struct PlayerInteraction {
    pub selected_voxel_world_pos: Option<Vec3>,
    pub hit_face: Option<VoxelFace>,
    pub interaction_range: f32,
    pub selected_material: VoxelType,
    pub brush_shape: BrushShape,
    pub brush_size: i32,
}

impl Default for PlayerInteraction {
    fn default() -> Self {
        Self {
            selected_voxel_world_pos: None,
            hit_face: None,
            interaction_range: 10.0,
            selected_material: VoxelType::Stone,
            brush_shape: BrushShape::Single,
            brush_size: 4,
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

#[derive(Resource)]
pub struct PlacementCooldown {
    pub last_place_time: f32,
}

impl Default for PlacementCooldown {
    fn default() -> Self {
        Self {
            last_place_time: -f32::INFINITY,
        }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInteraction>()
            .init_resource::<CursorState>()
            .init_resource::<PlacementCooldown>()
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
                    material_selection_input,
                    brush_selection_input,
                    voxel_interaction,
                    voxel_selection,
                ),
            );
        app.add_systems(
            FixedUpdate,
            (player_movement, player_unstuck.after(player_movement)),
        );
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
    mut player_query: Query<
        (
            &mut KinematicCharacterController,
            &Transform,
            Option<&KinematicCharacterControllerOutput>,
            &mut PlayerMotor,
        ),
        With<Player>,
    >,
    time: Res<Time<Fixed>>,
) {
    if let Ok((mut controller, transform, controller_output, mut motor)) = player_query.single_mut()
    {
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

fn setup_cursor_grab(mut window_cursor_query: Query<&mut CursorOptions, With<PrimaryWindow>>) {
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

fn material_selection_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut interaction: ResMut<PlayerInteraction>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        interaction.selected_material = VoxelType::Grass;
    } else if keyboard_input.just_pressed(KeyCode::Digit2) {
        interaction.selected_material = VoxelType::Dirt;
    } else if keyboard_input.just_pressed(KeyCode::Digit3) {
        interaction.selected_material = VoxelType::Stone;
    }
}

fn brush_selection_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut interaction: ResMut<PlayerInteraction>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit4) {
        interaction.brush_shape = BrushShape::Single;
    } else if keyboard_input.just_pressed(KeyCode::Digit5) {
        interaction.brush_shape = BrushShape::Cube;
    } else if keyboard_input.just_pressed(KeyCode::Digit6) {
        interaction.brush_shape = BrushShape::Sphere;
    }

    if keyboard_input.just_pressed(KeyCode::KeyZ) {
        interaction.brush_size = (interaction.brush_size - 1).clamp(MIN_BRUSH_SIZE, MAX_BRUSH_SIZE);
    } else if keyboard_input.just_pressed(KeyCode::KeyX) {
        interaction.brush_size = (interaction.brush_size + 1).clamp(MIN_BRUSH_SIZE, MAX_BRUSH_SIZE);
    }
}

fn calculate_placement_position(voxel_center: Vec3, face: VoxelFace) -> Vec3 {
    let (dx, dy, dz) = face.get_offset();
    voxel_center + Vec3::new(dx as f32, dy as f32, dz as f32) * VOXEL_SIZE
}

pub fn brush_world_size(brush_shape: BrushShape, brush_size: i32) -> f32 {
    match brush_shape {
        BrushShape::Single => VOXEL_SIZE,
        BrushShape::Cube | BrushShape::Sphere => brush_size as f32 * VOXEL_SIZE,
    }
}

fn build_brush_positions(center: Vec3, brush_shape: BrushShape, brush_size: i32) -> Vec<Vec3> {
    if brush_shape == BrushShape::Single {
        return vec![center];
    }

    let mut positions = Vec::new();
    let start = -(brush_size / 2);
    let end = start + brush_size - 1;
    let center_bias = if brush_size % 2 == 0 { 0.5 } else { 0.0 };
    let radius = brush_size as f32 * 0.5;

    for x in start..=end {
        for y in start..=end {
            for z in start..=end {
                if brush_shape == BrushShape::Sphere {
                    let sample_x = x as f32 + center_bias;
                    let sample_y = y as f32 + center_bias;
                    let sample_z = z as f32 + center_bias;
                    if sample_x * sample_x + sample_y * sample_y + sample_z * sample_z
                        > radius * radius
                    {
                        continue;
                    }
                }

                let offset = Vec3::new(x as f32, y as f32, z as f32) * VOXEL_SIZE;
                positions.push(center + offset);
            }
        }
    }

    positions
}

pub fn brush_preview_origin(center: Vec3, brush_shape: BrushShape, brush_size: i32) -> Vec3 {
    match brush_shape {
        BrushShape::Single => center - Vec3::splat(VOXEL_SIZE / 2.0),
        BrushShape::Cube | BrushShape::Sphere => {
            let half_extent = brush_world_size(brush_shape, brush_size) * 0.5;
            center - Vec3::splat(half_extent)
        }
    }
}

pub fn placement_center(selected_voxel_center: Vec3, hit_face: VoxelFace) -> Vec3 {
    calculate_placement_position(selected_voxel_center, hit_face)
}

pub fn brush_center_for_edit(
    selected_voxel_center: Vec3,
    hit_face: Option<VoxelFace>,
    brush_shape: BrushShape,
) -> Option<Vec3> {
    match brush_shape {
        BrushShape::Single => hit_face.map(|face| placement_center(selected_voxel_center, face)),
        BrushShape::Cube | BrushShape::Sphere => Some(selected_voxel_center),
    }
}

fn player_overlaps_voxel(player_pos: Vec3, voxel_center: Vec3) -> bool {
    let player_min = player_pos + Vec3::new(-0.25, 0.0, -0.25);
    let player_max = player_pos + Vec3::new(0.25, 2.0, 0.25);
    let voxel_min = voxel_center - Vec3::splat(VOXEL_SIZE / 2.0);
    let voxel_max = voxel_center + Vec3::splat(VOXEL_SIZE / 2.0);

    player_min.x < voxel_max.x
        && player_max.x > voxel_min.x
        && player_min.y < voxel_max.y
        && player_max.y > voxel_min.y
        && player_min.z < voxel_max.z
        && player_max.z > voxel_min.z
}

fn voxel_interaction(
    mut commands: Commands,
    world: Res<World>,
    interaction: Res<PlayerInteraction>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    cursor_query: Query<&CursorOptions, With<PrimaryWindow>>,
    mut chunk_query_set: ParamSet<(Query<&crate::world::Chunk>, Query<&mut crate::world::Chunk>)>,
    player_query: Query<&Transform, With<Player>>,
    time: Res<Time>,
    mut placement_cooldown: ResMut<PlacementCooldown>,
) {
    if let Ok(cursor_options) = cursor_query.single() {
        if cursor_options.grab_mode != CursorGrabMode::Locked {
            return;
        }
    }

    if let Some(selected_voxel_pos) = interaction.selected_voxel_world_pos {
        if mouse_input.just_pressed(MouseButton::Left) {
            let target_positions = build_brush_positions(
                selected_voxel_pos,
                interaction.brush_shape,
                interaction.brush_size,
            );

            for target_pos in target_positions {
                if world.set_voxel_at_world(
                    target_pos,
                    Voxel::new(VoxelType::Air),
                    &mut chunk_query_set.p1(),
                ) {
                    mark_chunk_for_update(&mut commands, &world, target_pos);
                }
            }
        }

        if mouse_input.pressed(MouseButton::Right) {
            let current_time = time.elapsed_secs();

            if current_time - placement_cooldown.last_place_time > 0.1 {
                if let Some(place_center) = brush_center_for_edit(
                    selected_voxel_pos,
                    interaction.hit_face,
                    interaction.brush_shape,
                ) {
                    let target_positions = build_brush_positions(
                        place_center,
                        interaction.brush_shape,
                        interaction.brush_size,
                    );
                    let mut placed_any = false;

                    if let Ok(player_transform) = player_query.single() {
                        let player_pos = player_transform.translation;
                        for target_pos in target_positions {
                            if player_overlaps_voxel(player_pos, target_pos) {
                                continue;
                            }

                            if let Some(existing_voxel) =
                                world.get_voxel_at_world(target_pos, &chunk_query_set.p0())
                            {
                                if !existing_voxel.is_solid() {
                                    let selected_material = interaction.selected_material;
                                    if world.set_voxel_at_world(
                                        target_pos,
                                        Voxel::new(selected_material),
                                        &mut chunk_query_set.p1(),
                                    ) {
                                        mark_chunk_for_update(&mut commands, &world, target_pos);
                                        placed_any = true;
                                    }
                                }
                            }
                        }
                    }

                    if placed_any {
                        placement_cooldown.last_place_time = current_time;
                    }
                }
            }
        }
    }
}

fn mark_chunk_for_update(commands: &mut Commands, world: &World, world_pos: Vec3) {
    if let Some((chunk_coord, voxel_x, _, voxel_z)) = world.world_to_voxel(world_pos) {
        let mut dirty_chunks = HashSet::from([chunk_coord]);

        if voxel_x == 0 {
            dirty_chunks.insert(crate::world::ChunkCoord::new(
                chunk_coord.x - 1,
                chunk_coord.z,
            ));
        }
        if voxel_x + 1 == crate::world::CHUNK_VOXELS_SIZE {
            dirty_chunks.insert(crate::world::ChunkCoord::new(
                chunk_coord.x + 1,
                chunk_coord.z,
            ));
        }
        if voxel_z == 0 {
            dirty_chunks.insert(crate::world::ChunkCoord::new(
                chunk_coord.x,
                chunk_coord.z - 1,
            ));
        }
        if voxel_z + 1 == crate::world::CHUNK_VOXELS_SIZE {
            dirty_chunks.insert(crate::world::ChunkCoord::new(
                chunk_coord.x,
                chunk_coord.z + 1,
            ));
        }

        for dirty_chunk in dirty_chunks {
            if let Some(chunk_entity) = world.chunks.get(&dirty_chunk) {
                commands
                    .entity(*chunk_entity)
                    .insert((NeedsRenderRefresh, NeedsPhysicsRefresh));
            }
        }
    }
}

#[derive(Component)]
pub struct NeedsRenderRefresh;

#[derive(Component)]
pub struct NeedsPhysicsRefresh;

fn player_unstuck(
    mut player_query: Query<&mut Transform, With<Player>>,
    world: Res<World>,
    chunk_query: Query<&crate::world::Chunk>,
) {
    let Ok(mut player_transform) = player_query.single_mut() else {
        return;
    };

    let current_position = player_transform.translation;
    if !player_intersects_solid_voxel(&world, &chunk_query, current_position) {
        return;
    }

    for up_steps in 1..=16 {
        let vertical_offset = Vec3::Y * (up_steps as f32 * VOXEL_SIZE);
        for horizontal_offset in unstuck_horizontal_offsets() {
            let candidate_position = current_position + vertical_offset + horizontal_offset;
            if !player_intersects_solid_voxel(&world, &chunk_query, candidate_position) {
                player_transform.translation = candidate_position;
                return;
            }
        }
    }
}

fn player_intersects_solid_voxel(
    world: &World,
    chunk_query: &Query<&crate::world::Chunk>,
    player_position: Vec3,
) -> bool {
    let player_min = player_position + Vec3::new(-0.25, 0.0, -0.25);
    let player_max = player_position + Vec3::new(0.25, 2.0, 0.25);

    let min_x = (player_min.x / VOXEL_SIZE).floor() as i32;
    let min_y = (player_min.y / VOXEL_SIZE).floor() as i32;
    let min_z = (player_min.z / VOXEL_SIZE).floor() as i32;
    let max_x = ((player_max.x - f32::EPSILON) / VOXEL_SIZE).floor() as i32;
    let max_y = ((player_max.y - f32::EPSILON) / VOXEL_SIZE).floor() as i32;
    let max_z = ((player_max.z - f32::EPSILON) / VOXEL_SIZE).floor() as i32;

    for voxel_x in min_x..=max_x {
        for voxel_y in min_y..=max_y {
            for voxel_z in min_z..=max_z {
                let voxel_center = voxel_center_from_indices(voxel_x, voxel_y, voxel_z);
                if world
                    .get_voxel_at_world(voxel_center, chunk_query)
                    .is_some_and(|voxel| voxel.is_solid())
                {
                    return true;
                }
            }
        }
    }

    false
}

fn unstuck_horizontal_offsets() -> [Vec3; 5] {
    [
        Vec3::ZERO,
        Vec3::new(VOXEL_SIZE, 0.0, 0.0),
        Vec3::new(-VOXEL_SIZE, 0.0, 0.0),
        Vec3::new(0.0, 0.0, VOXEL_SIZE),
        Vec3::new(0.0, 0.0, -VOXEL_SIZE),
    ]
}
