use crate::player::{Inventory, Player, PlayerInteraction};
use crate::voxel::VoxelType;
use crate::world::InitialWorldGeneration;
use bevy::prelude::*;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
            .add_systems(Update, update_ui_text);
    }
}

#[derive(Component)]
struct PlayerInfoText;

#[derive(Component)]
struct SelectedBlockText;

#[derive(Component)]
struct SelectedMaterialText;

#[derive(Component)]
struct BrushSettingsText;

#[derive(Component)]
struct InventoryText;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct LoadingRoot;

fn setup_ui(mut commands: Commands) {
    commands
        .spawn((
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
                Text::new("Mode: Survival block interaction"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                BrushSettingsText,
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
                LoadingText,
                Text::new("Generating terrain..."),
                TextFont {
                    font_size: 36.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn update_ui_text(
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
        Query<&mut Text, With<BrushSettingsText>>,
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
        **text = "Mode: Survival block interaction".to_string();
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
