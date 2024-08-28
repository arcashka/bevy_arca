use bevy::{app::App, DefaultPlugins};
use bevy_arca::ArcaPlugin;

fn main() {
    App::new().add_plugins((DefaultPlugins, ArcaPlugin)).run();
}
