use crate::player::{Player, PlayerInteraction};
use bevy::prelude::*;
use bevy_egui::{EguiContextPass, EguiContexts, EguiPlugin, egui};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_systems(EguiContextPass, ui_system);
    }
}

fn ui_system(
    mut contexts: EguiContexts, 
    player_query: Query<&Transform, With<Player>>,
    interaction: Res<PlayerInteraction>,
) {
    if let Ok(player_transform) = player_query.single() {
        let pos = player_transform.translation;

        egui::Area::new(egui::Id::new("player_info"))
            .fixed_pos(egui::pos2(10.0, 10.0))
            .show(contexts.ctx_mut(), |ui| {
                ui.label(format!(
                    "Position: ({:.1}, {:.1}, {:.1})",
                    pos.x, pos.y, pos.z
                ));
                
                if let Some((chunk_coord, x, y, z)) = interaction.selected_voxel {
                    ui.label(format!(
                        "Selected: Chunk({}, {}) Voxel({}, {}, {})",
                        chunk_coord.x, chunk_coord.z, x, y, z
                    ));
                } else {
                    ui.label("Selected: None");
                }
                
                ui.label("Left Click: Break Block");
                ui.label("Right Click: Place Block");
            });
    }
}
