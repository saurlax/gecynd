use bevy::prelude::*;
use bevy::window::{ExitCondition, WindowCloseRequested};

mod physics;
mod player;
mod render;
mod save;
mod terrain;
mod ui;
mod voxel;
mod world;

#[cfg(debug_assertions)]
mod debug_remote;

#[cfg(debug_assertions)]
use debug_remote::DebugRemotePlugin;
use physics::PhysicsPlugin;
use player::PlayerPlugin;
use render::RenderPlugin;
use save::SavePlugin;
use ui::UiPlugin;
use world::WorldPlugin;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    LoadingWorld,
    InGame,
    Paused,
}

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
        .init_state::<AppState>()
        .insert_resource(ClearColor(Color::srgb(0.58, 0.76, 0.90)))
        .add_plugins((
            SavePlugin,
            WorldPlugin,
            PlayerPlugin,
            PhysicsPlugin,
            RenderPlugin,
            UiPlugin,
        ))
        .add_plugins(debug_plugins())
        .add_systems(Update, handle_window_close)
        .run();
}

#[cfg(debug_assertions)]
fn debug_plugins() -> DebugRemotePlugin {
    DebugRemotePlugin
}

#[cfg(not(debug_assertions))]
fn debug_plugins() {}

fn handle_window_close(
    mut close_events: MessageReader<WindowCloseRequested>,
    mut exit: MessageWriter<AppExit>,
) {
    for _ in close_events.read() {
        exit.write(AppExit::Success);
    }
}
