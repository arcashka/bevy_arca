use bevy::{app::App, DefaultPlugins};
use bevy_rtx::RtxPlugin;

fn main() {
    App::new().add_plugins((DefaultPlugins, RtxPlugin)).run();
}
