use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

mod camera;
mod input;
mod physics;
mod voxel;
mod world;

use camera::CameraPlugin;
use input::InputPlugin;
use physics::PhysicsPlugin;
use voxel::VoxelPlugin;
use world::WorldPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins((
            VoxelPlugin,
            InputPlugin,
            CameraPlugin,
            WorldPlugin,
            PhysicsPlugin,
        ))
        .run();
}
