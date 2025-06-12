use bevy::prelude::*;
use bevy::window::{WindowCloseRequested, ExitCondition};

mod physics;
mod player;
mod render;
mod terrain;
mod ui;
mod voxel;
mod world;

use physics::PhysicsPlugin;
use player::PlayerPlugin;
use render::RenderPlugin;
use ui::UiPlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gecynd".into(),
                ..default()
            }),
            exit_condition: ExitCondition::OnPrimaryClosed,
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.98)))
        .add_plugins((WorldPlugin, PlayerPlugin, PhysicsPlugin, RenderPlugin, UiPlugin))
        .add_systems(Update, handle_window_close)
        .run();
}

fn handle_window_close(
    mut close_events: EventReader<WindowCloseRequested>,
    mut exit: EventWriter<AppExit>,
) {
    for _ in close_events.read() {
        exit.write(AppExit::Success);
    }
}
