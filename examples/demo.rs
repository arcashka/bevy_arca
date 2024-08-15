use bevy::{app::App, DefaultPlugins};
use bevy_arca::GraphicsPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, GraphicsPlugin))
        .run();
}
