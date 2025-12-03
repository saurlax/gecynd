use crate::player::{Player, PlayerInteraction};
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
struct ControlsText;

fn setup_ui(mut commands: Commands) {
    // 创建UI根节点
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
            // 玩家位置信息
            parent.spawn((
                Text::new("Position: (0.0, 0.0, 0.0)"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                PlayerInfoText,
            ));
            
            // 选中方块信息
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
            
            // 控制说明
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
                        "Shift: Sprint",
                        "F1: Toggle AABB Debug",
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
}

fn update_ui_text(
    player_query: Query<&Transform, With<Player>>,
    interaction: Res<PlayerInteraction>,
    world: Res<crate::world::World>,
    mut player_info_query: Query<&mut Text, (With<PlayerInfoText>, Without<SelectedBlockText>)>,
    mut selected_block_query: Query<&mut Text, (With<SelectedBlockText>, Without<PlayerInfoText>)>,
) {
    // 更新玩家位置
    if let Ok(player_transform) = player_query.single() {
        if let Ok(mut text) = player_info_query.single_mut() {
            let pos = player_transform.translation;
            **text = format!(
                "Position: ({:.1}, {:.1}, {:.1})",
                pos.x, pos.y, pos.z
            );
        }
    }
    
    // 更新选中方块信息
    if let Ok(mut text) = selected_block_query.single_mut() {
        if let Some(selected_pos) = interaction.selected_voxel_world_pos {
            if let Some((chunk_coord, x, y, z)) = world.world_to_voxel(selected_pos) {
                let mut info = format!(
                    "Selected: Chunk({}, {}) Voxel({}, {}, {})\nWorld Pos: ({:.1}, {:.1}, {:.1})",
                    chunk_coord.x, chunk_coord.z, x, y, z,
                    selected_pos.x, selected_pos.y, selected_pos.z
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
}
