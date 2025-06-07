use bevy::prelude::*;

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
        .add_plugins(DefaultPlugins)
        .add_plugins((WorldPlugin, PlayerPlugin, PhysicsPlugin, RenderPlugin, UiPlugin))
        .run();
}
