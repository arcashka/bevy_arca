use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_arca::core::Camera;
use bevy_arca::gltf::{GltfAssetLabel, GltfPlugin};
use bevy_arca::plugins::{CameraController, CameraControllerPlugin};
use bevy_arca::ArcaPlugin;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera {
            fov: PI / 4.0,
            aspect_ratio: 16.0 / 9.0,
        },
        Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, -1.0), Vec3::Y),
        GlobalTransform::default(),
        CameraController::default(),
    ));
    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("cube.glb")),
        transform: Transform::from_xyz(0.0, 0.0, -5.0),
        ..default()
    });
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            ArcaPlugin,
            GltfPlugin,
            CameraControllerPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}
