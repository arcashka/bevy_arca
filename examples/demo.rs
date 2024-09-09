use bevy::prelude::*;
use bevy_arca::core::Camera;
use bevy_arca::plugins::{CameraController, CameraControllerPlugin};
use bevy_arca::ArcaPlugin;

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            fov: 120.0,
            aspect_ratio: 16.0 / 9.0,
        },
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::ONE, Vec3::Y),
        CameraController::default(),
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ArcaPlugin, CameraControllerPlugin))
        .add_systems(Startup, setup)
        .run();
}
