mod camera;
mod image;
mod material;
mod mesh;
mod mesh_data;
mod shader;
mod vertex_buffer;

use bevy::prelude::*;
use camera::CameraPlugin;

pub use camera::Camera;
pub use image::Image;
pub use material::Material;
pub use mesh::Mesh;
pub use mesh_data::{MeshBuffer, MeshData};
pub use shader::Shader;
pub use vertex_buffer::VertexBuffer;

use shader::ShaderLoader;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<Material>()
            .init_asset::<Shader>()
            .register_type::<Image>()
            .register_type::<Material>()
            .register_asset_reflect::<Image>()
            .register_asset_reflect::<Material>()
            .register_asset_loader(ShaderLoader);

        app.add_plugins(CameraPlugin);

        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(&Handle::default(), Image::new());

        app.world_mut()
            .resource_mut::<Assets<Material>>()
            .insert(&Handle::default(), Material::default());
    }
}
