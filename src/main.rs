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
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.98))) // 天空蓝色
        .add_plugins((WorldPlugin, PlayerPlugin, PhysicsPlugin, RenderPlugin, UiPlugin))
        .run();
}
