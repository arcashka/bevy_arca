use bevy::{asset::AssetServer, prelude::*};
use bevy_arca::{gltf::GltfAssetLabel, ArcaPlugin};

fn load_cube(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("cube.glb")),
        ..default()
    });
}

fn main() {
    App::new()
        .add_plugins(ArcaPlugin)
        .add_systems(Startup, load_cube)
        .run();
}
