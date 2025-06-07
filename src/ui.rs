use crate::player::Player;
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

fn ui_system(mut contexts: EguiContexts, player_query: Query<&Transform, With<Player>>) {
    if let Ok(player_transform) = player_query.single() {
        let pos = player_transform.translation;

        egui::Area::new(egui::Id::new("player_pos"))
            .fixed_pos(egui::pos2(10.0, 10.0))
            .show(contexts.ctx_mut(), |ui| {
                ui.label(format!(
                    "Position: ({:.1}, {:.1}, {:.1})",
                    pos.x, pos.y, pos.z
                ));
            });
    }
}
