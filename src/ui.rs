use bevy::prelude::*;

use crate::player::{
    HOTBAR_MATERIALS, Inventory, Player, PlayerInteraction, selected_material_index,
};
use crate::save::{SaveState, flush_pending_save, queue_manual_save};
use crate::world::Chunk;
use crate::voxel::VoxelType;
use crate::world::{DebugInfoState, InitialWorldGeneration};
use crate::AppState;

const NORMAL_BUTTON: Color = Color::srgb(0.30, 0.30, 0.30);
const HOVERED_BUTTON: Color = Color::srgb(0.42, 0.42, 0.42);
const PRESSED_BUTTON: Color = Color::srgb(0.56, 0.56, 0.56);
const DISABLED_BUTTON: Color = Color::srgb(0.16, 0.16, 0.16);
const HOTBAR_SLOT: Color = Color::srgba(0.08, 0.10, 0.14, 0.82);
const HOTBAR_SLOT_SELECTED: Color = Color::srgb(0.86, 0.76, 0.34);
const HOTBAR_BORDER: Color = Color::srgba(0.72, 0.76, 0.82, 0.55);
const HOTBAR_BORDER_SELECTED: Color = Color::srgb(0.98, 0.95, 0.82);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui_camera)
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu)
            .add_systems(
                Update,
                (main_menu_button_visuals, main_menu_actions).run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnEnter(AppState::LoadingWorld), setup_loading_screen)
            .add_systems(OnExit(AppState::LoadingWorld), cleanup_loading_screen)
            .add_systems(
                Update,
                update_loading_screen.run_if(in_state(AppState::LoadingWorld)),
            )
            .add_systems(OnEnter(AppState::InGame), setup_hud)
            .add_systems(OnExit(AppState::InGame), cleanup_hud)
            .add_systems(OnEnter(AppState::Paused), setup_pause_menu)
            .add_systems(OnExit(AppState::Paused), cleanup_pause_menu)
            .add_systems(
                Update,
                (update_hud_text, sync_debug_info_visibility).run_if(in_state(AppState::InGame)),
            )
            .add_systems(
                Update,
                (pause_menu_button_visuals, pause_menu_actions, pause_input_system)
                    .run_if(in_state(AppState::Paused)),
            );
    }
}

#[derive(Component)]
struct UiCameraRoot;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct PauseMenuRoot;

#[derive(Component)]
struct PlayerInfoText;

#[derive(Component)]
struct SelectedBlockText;

#[derive(Component)]
struct DebugInfoRoot;

#[derive(Component)]
struct HotbarRoot;

#[derive(Component)]
struct HotbarSlot {
    index: usize,
}

#[derive(Component)]
struct HotbarSlotLabel {
    material: VoxelType,
}

#[derive(Component)]
struct HotbarSlotCount {
    material: VoxelType,
}

#[derive(Component)]
struct LoadingRoot;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct LoadingProgressFill;

#[derive(Component)]
struct MainMenuButton {
    action: MainMenuAction,
}

#[derive(Component)]
struct PauseMenuButton {
    action: PauseMenuAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MainMenuAction {
    NewSave,
    LoadSave,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PauseMenuAction {
    Resume,
    ReturnToMainMenu,
}

fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
        UiCameraRoot,
    ));
}

fn setup_main_menu(mut commands: Commands, save_state: Res<SaveState>) {
    commands
        .spawn((
            MainMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(24.0), Val::Px(28.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.36, 0.62, 0.92)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(560.0),
                        max_width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("Gecynd"),
                        TextFont {
                            font_size: 64.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    card.spawn((
                        Text::new("Prototype Sandbox Build"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.96, 0.94, 0.56)),
                        Node {
                            margin: UiRect::bottom(Val::Px(16.0)),
                            ..default()
                        },
                    ));

                    spawn_menu_button(card, "New Save", MainMenuAction::NewSave, true);
                    spawn_menu_button(
                        card,
                        "Load Save",
                        MainMenuAction::LoadSave,
                        save_state.save_exists(),
                    );

                });

            parent.spawn((
                Text::new("Closed Alpha Build 0.1.0"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.0),
                    bottom: Val::Px(10.0),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Copyright 2026 Gecynd Project"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(12.0),
                    bottom: Val::Px(10.0),
                    ..default()
                },
            ));
        });
}

fn spawn_menu_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: MainMenuAction,
    enabled: bool,
) {
    parent
        .spawn((
            Button,
            MainMenuButton { action },
            Node {
                width: Val::Px(320.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(if enabled { NORMAL_BUTTON } else { DISABLED_BUTTON }),
            BorderColor::all(if enabled {
                Color::srgb(0.08, 0.08, 0.08)
            } else {
                Color::srgb(0.16, 0.16, 0.16)
            }),
            InteractionDisabled(!enabled),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(if enabled {
                    Color::WHITE
                } else {
                    Color::srgb(0.55, 0.55, 0.55)
                }),
            ));
        });
}

#[derive(Component)]
struct InteractionDisabled(bool);

fn cleanup_main_menu(mut commands: Commands, root_query: Query<Entity, With<MainMenuRoot>>) {
    for entity in root_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn main_menu_button_visuals(
    mut button_query: Query<
        (&Interaction, &InteractionDisabled, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>, With<MainMenuButton>),
    >,
) {
    for (interaction, disabled, mut background, mut border) in &mut button_query {
        if disabled.0 {
            *background = DISABLED_BUTTON.into();
            *border = BorderColor::all(Color::srgb(0.16, 0.16, 0.16));
            continue;
        }

        match *interaction {
            Interaction::Pressed => {
                *background = PRESSED_BUTTON.into();
                *border = BorderColor::all(Color::srgb(0.95, 0.95, 0.95));
            }
            Interaction::Hovered => {
                *background = HOVERED_BUTTON.into();
                *border = BorderColor::all(Color::srgb(0.95, 0.95, 0.95));
            }
            Interaction::None => {
                *background = NORMAL_BUTTON.into();
                *border = BorderColor::all(Color::srgb(0.08, 0.08, 0.08));
            }
        }
    }
}

fn main_menu_actions(
    mut interaction_query: Query<
        (&Interaction, &MainMenuButton, &InteractionDisabled),
        (Changed<Interaction>, With<Button>),
    >,
    mut save_state: ResMut<SaveState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button, disabled) in &mut interaction_query {
        if *interaction != Interaction::Pressed || disabled.0 {
            continue;
        }

        match button.action {
            MainMenuAction::NewSave => {
                save_state.start_new_world();
                next_state.set(AppState::LoadingWorld);
            }
            MainMenuAction::LoadSave => {
                if save_state.load_existing_world() {
                    next_state.set(AppState::LoadingWorld);
                }
            }
        }
    }
}

fn pause_input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::InGame);
    }
}

fn pause_menu_button_visuals(
    mut button_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>, With<PauseMenuButton>),
    >,
) {
    for (interaction, mut background, mut border) in &mut button_query {
        match *interaction {
            Interaction::Pressed => {
                *background = PRESSED_BUTTON.into();
                *border = BorderColor::all(Color::srgb(0.95, 0.95, 0.95));
            }
            Interaction::Hovered => {
                *background = HOVERED_BUTTON.into();
                *border = BorderColor::all(Color::srgb(0.95, 0.95, 0.95));
            }
            Interaction::None => {
                *background = NORMAL_BUTTON.into();
                *border = BorderColor::all(Color::srgb(0.08, 0.08, 0.08));
            }
        }
    }
}

fn pause_menu_actions(
    mut interaction_query: Query<
        (&Interaction, &PauseMenuButton),
        (Changed<Interaction>, With<Button>),
    >,
    player_query: Query<&Transform, With<Player>>,
    chunk_query: Query<&Chunk>,
    inventory: Res<Inventory>,
    mut save_state: ResMut<SaveState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut interaction_query {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button.action {
            PauseMenuAction::Resume => next_state.set(AppState::InGame),
            PauseMenuAction::ReturnToMainMenu => {
                if save_state.pending_write.is_none() {
                    queue_manual_save(&mut save_state, &player_query, &chunk_query, &inventory);
                }

                if let Err(error) = flush_pending_save(&mut save_state) {
                    warn!("Failed to save world before returning to main menu: {error}");
                }

                next_state.set(AppState::MainMenu);
            }
        }
    }
}

fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    DebugInfoRoot,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(5.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    Visibility::Hidden,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Position: (0.0, 0.0, 0.0)"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        PlayerInfoText,
                    ));

                    parent.spawn((
                        Text::new("Selected: None"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        SelectedBlockText,
                    ));
                });

            parent
                .spawn((
                    HotbarRoot,
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::End,
                        padding: UiRect::bottom(Val::Px(14.0)),
                        column_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|hotbar| {
                    for (index, material) in HOTBAR_MATERIALS.iter().copied().enumerate() {
                        hotbar
                            .spawn((
                                HotbarSlot { index },
                                Node {
                                    width: Val::Px(92.0),
                                    height: Val::Px(92.0),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Stretch,
                                    padding: UiRect::all(Val::Px(10.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(HOTBAR_SLOT),
                                BorderColor::all(HOTBAR_BORDER),
                            ))
                            .with_children(|slot| {
                                slot.spawn((
                                    Text::new(format!("{}", index + 1)),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.84, 0.86, 0.90)),
                                ));

                                slot.spawn((
                                    Text::new(hotbar_material_name(material)),
                                    TextFont {
                                        font_size: 18.0,
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                    HotbarSlotLabel { material },
                                ));

                                slot.spawn((
                                    Text::new("x0"),
                                    TextFont {
                                        font_size: 15.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.92, 0.94, 0.96)),
                                    HotbarSlotCount { material },
                                ));
                            });
                    }
                });
        });

}

fn cleanup_hud(
    mut commands: Commands,
    hud_query: Query<Entity, With<HudRoot>>,
) {
    for entity in hud_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn hotbar_material_name(material: VoxelType) -> &'static str {
    match material {
        VoxelType::Grass => "Grass",
        VoxelType::Dirt => "Dirt",
        VoxelType::Stone => "Stone",
        VoxelType::Air => "Air",
    }
}

fn setup_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.06, 0.08, 0.72)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(360.0),
                        max_width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.10, 0.14, 0.94)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Paused"),
                        TextFont {
                            font_size: 40.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    spawn_pause_menu_button(panel, "Resume", PauseMenuAction::Resume);
                    spawn_pause_menu_button(
                        panel,
                        "Return To Main Menu",
                        PauseMenuAction::ReturnToMainMenu,
                    );
                });
        });
}

fn spawn_pause_menu_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: PauseMenuAction,
) {
    parent
        .spawn((
            Button,
            PauseMenuButton { action },
            Node {
                width: Val::Px(320.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
            BorderColor::all(Color::srgb(0.08, 0.08, 0.08)),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn cleanup_pause_menu(mut commands: Commands, root_query: Query<Entity, With<PauseMenuRoot>>) {
    for entity in root_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn setup_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            LoadingRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.36, 0.62, 0.92)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(420.0),
                        max_width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.10, 0.14, 0.92)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Preparing World... 0 / 0"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        LoadingText,
                    ));

                    panel.spawn((
                        Text::new("Initializing terrain and loading nearby regions."),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.82, 0.86, 0.90)),
                    ));

                    panel
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(20.0),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.14, 0.16, 0.20)),
                            BorderColor::all(Color::srgb(0.48, 0.52, 0.58)),
                        ))
                        .with_children(|bar| {
                            bar.spawn((
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.78, 0.84, 0.32)),
                                LoadingProgressFill,
                            ));
                    });

                    panel.spawn((
                        Text::new("Please wait while world data is assembled."),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.72, 0.76, 0.82)),
                    ));
                });
        });
}

fn cleanup_loading_screen(mut commands: Commands, root_query: Query<Entity, With<LoadingRoot>>) {
    for entity in root_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn update_loading_screen(
    generation_state: Res<InitialWorldGeneration>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<LoadingText>>,
        Query<&mut Node, With<LoadingProgressFill>>,
    )>,
) {
    if let Ok(mut text) = text_queries.p0().single_mut() {
        **text = format!(
            "Preparing World... {} / {}",
            generation_state.completed_chunks,
            generation_state.total_chunks
        );
    }

    if let Ok(mut node) = text_queries.p1().single_mut() {
        let progress = if generation_state.total_chunks == 0 {
            0.0
        } else {
            generation_state.completed_chunks as f32 / generation_state.total_chunks as f32
        };
        node.width = Val::Percent((progress * 100.0).clamp(0.0, 100.0));
    }
}

fn update_hud_text(
    player_query: Query<&Transform, With<Player>>,
    interaction: Res<PlayerInteraction>,
    inventory: Res<Inventory>,
    world: Res<crate::world::World>,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<PlayerInfoText>>,
        Query<&mut Text, With<SelectedBlockText>>,
        Query<(&HotbarSlot, &mut BackgroundColor, &mut BorderColor)>,
        Query<(&HotbarSlotCount, &mut Text)>,
        Query<(&HotbarSlotLabel, &mut TextColor)>,
    )>,
) {
    if let Ok(player_transform) = player_query.single() {
        if let Ok(mut text) = text_queries.p0().single_mut() {
            let pos = player_transform.translation;
            **text = format!("Position: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
        }
    }

    if let Ok(mut text) = text_queries.p1().single_mut() {
        if let Some(selected_pos) = interaction.selected_voxel_world_pos {
            if let Some((chunk_coord, x, y, z)) = world.world_to_voxel(selected_pos) {
                let mut info = format!(
                    "Selected: Chunk({}, {}) Voxel({}, {}, {})\nWorld Pos: ({:.1}, {:.1}, {:.1})",
                    chunk_coord.x,
                    chunk_coord.z,
                    x,
                    y,
                    z,
                    selected_pos.x,
                    selected_pos.y,
                    selected_pos.z
                );

                if let Some(face) = interaction.hit_face {
                    info.push_str(&format!("\nHit Face: {:?}", face));
                }

                **text = info;
            } else {
                **text = "Selected: None".to_string();
            }
        } else {
            **text = "Selected: None".to_string();
        }
    }

    let selected_index = selected_material_index(interaction.selected_material);
    for (slot, mut background, mut border) in &mut text_queries.p2() {
        let is_selected = slot.index == selected_index;
        *background = if is_selected {
            HOTBAR_SLOT_SELECTED.into()
        } else {
            HOTBAR_SLOT.into()
        };
        *border = BorderColor::all(if is_selected {
            HOTBAR_BORDER_SELECTED
        } else {
            HOTBAR_BORDER
        });
    }

    for (slot_count, mut text) in &mut text_queries.p3() {
        **text = format!("x{}", inventory.count(slot_count.material));
    }

    for (slot_label, mut text_color) in &mut text_queries.p4() {
        *text_color = if slot_label.material == interaction.selected_material {
            TextColor(Color::srgb(0.14, 0.10, 0.02))
        } else {
            TextColor(Color::WHITE)
        };
    }
}

fn sync_debug_info_visibility(
    debug_info_state: Res<DebugInfoState>,
    mut debug_info_query: Query<&mut Visibility, With<DebugInfoRoot>>,
) {
    if let Ok(mut visibility) = debug_info_query.single_mut() {
        *visibility = if debug_info_state.enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
