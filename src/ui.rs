use bevy::prelude::*;

use crate::player::{Inventory, Player, PlayerInteraction};
use crate::save::{DEFAULT_SAVE_PATH, SaveState};
use crate::voxel::VoxelType;
use crate::world::InitialWorldGeneration;
use crate::AppState;

const NORMAL_BUTTON: Color = Color::srgb(0.30, 0.30, 0.30);
const HOVERED_BUTTON: Color = Color::srgb(0.42, 0.42, 0.42);
const PRESSED_BUTTON: Color = Color::srgb(0.56, 0.56, 0.56);
const DISABLED_BUTTON: Color = Color::srgb(0.16, 0.16, 0.16);

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
            .add_systems(OnEnter(AppState::InGame), setup_hud)
            .add_systems(OnExit(AppState::InGame), cleanup_hud)
            .add_systems(Update, update_hud_text.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component)]
struct UiCameraRoot;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct PlayerInfoText;

#[derive(Component)]
struct SelectedBlockText;

#[derive(Component)]
struct SelectedMaterialText;

#[derive(Component)]
struct ModeText;

#[derive(Component)]
struct InventoryText;

#[derive(Component)]
struct LoadingRoot;

#[derive(Component)]
struct MainMenuButton {
    action: MainMenuAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MainMenuAction {
    NewSave,
    LoadSave,
}

fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((Camera2d, UiCameraRoot));
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
                        Text::new("Rusty blocks. Survival first."),
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

                    card.spawn((
                        Text::new(if save_state.save_exists() {
                            "Existing save found."
                        } else {
                            "No save file found yet."
                        }),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.92, 0.92)),
                        Node {
                            margin: UiRect::top(Val::Px(12.0)),
                            ..default()
                        },
                    ));

                    card.spawn((
                        Text::new(format!("Default save path: {DEFAULT_SAVE_PATH}")),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.88, 0.88, 0.88)),
                    ));
                });

            parent.spawn((
                Text::new("Bevy native UI prototype"),
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
                next_state.set(AppState::InGame);
            }
            MainMenuAction::LoadSave => {
                if save_state.load_existing_world() {
                    next_state.set(AppState::InGame);
                }
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
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
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
                Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Material: Stone [1 Grass, 2 Dirt, 3 Stone]"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                SelectedMaterialText,
                Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Mode: Survival block interaction | F5 to save"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                ModeText,
                Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("Inventory: Grass 0 | Dirt 0 | Stone 0"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                InventoryText,
                Node {
                    margin: UiRect::top(Val::Px(5.0)),
                    ..default()
                },
            ));

            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|parent| {
                    let controls = [
                        "Controls:",
                        "Left Click: Break Block",
                        "Right Click: Place Block",
                        "1/2/3: Select Grass/Dirt/Stone",
                        "F5: Save World",
                        "Shift: Sprint",
                        "F1: Toggle AABB Debug",
                        "F2: Toggle Render Wireframe",
                        "F3: Toggle Physics Wireframe",
                    ];

                    for control_text in controls {
                        parent.spawn((
                            Text::new(control_text),
                            TextFont {
                                font_size: 16.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        ));
                    }
                });
        });

    commands
        .spawn((
            LoadingRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Generating terrain..."),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn cleanup_hud(
    mut commands: Commands,
    hud_query: Query<Entity, Or<(With<HudRoot>, With<LoadingRoot>)>>,
) {
    for entity in hud_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn update_hud_text(
    player_query: Query<&Transform, With<Player>>,
    interaction: Res<PlayerInteraction>,
    inventory: Res<Inventory>,
    world: Res<crate::world::World>,
    generation_state: Res<InitialWorldGeneration>,
    loading_root_query: Query<Entity, With<LoadingRoot>>,
    mut commands: Commands,
    mut text_queries: ParamSet<(
        Query<&mut Text, With<PlayerInfoText>>,
        Query<&mut Text, With<SelectedBlockText>>,
        Query<&mut Text, With<SelectedMaterialText>>,
        Query<&mut Text, With<ModeText>>,
        Query<&mut Text, With<InventoryText>>,
    )>,
) {
    if generation_state.finished {
        if let Ok(entity) = loading_root_query.single() {
            commands.entity(entity).despawn_children();
            commands.entity(entity).despawn();
        }
    }

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

    if let Ok(mut text) = text_queries.p2().single_mut() {
        let material_name = match interaction.selected_material {
            VoxelType::Grass => "Grass",
            VoxelType::Dirt => "Dirt",
            VoxelType::Stone => "Stone",
            VoxelType::Air => "Air",
        };
        **text = format!("Material: {material_name} [1 Grass, 2 Dirt, 3 Stone]");
    }

    if let Ok(mut text) = text_queries.p3().single_mut() {
        **text = format!(
            "Mode: Survival block interaction | F5 to save | Save path: {}",
            DEFAULT_SAVE_PATH
        );
    }

    if let Ok(mut text) = text_queries.p4().single_mut() {
        **text = format!(
            "Inventory: Grass {} | Dirt {} | Stone {}",
            inventory.count(VoxelType::Grass),
            inventory.count(VoxelType::Dirt),
            inventory.count(VoxelType::Stone)
        );
    }
}
