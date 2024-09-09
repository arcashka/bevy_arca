pub mod core;
pub mod gltf;
pub mod plugins;
pub mod render;
mod win_types;

use bevy::prelude::*;

use core::CorePlugin;
use render::RenderPlugin;

pub struct ArcaPlugin;

impl Plugin for ArcaPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((CorePlugin, RenderPlugin));
    }
}
