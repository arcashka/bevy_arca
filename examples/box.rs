use bevy::{asset::AssetServer, prelude::*, DefaultPlugins};
use bevy_arca::GraphicsPlugin;

fn load_cube(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("cube.gltf")),
        ..default()
    });
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GraphicsPlugin))
        .add_systems(Startup, load_cube)
        .run();
}
