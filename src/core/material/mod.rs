use bevy::{math::Affine2, prelude::*};

use super::Image;

pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Material>()
            .register_asset_reflect::<Material>();

        let mut material_assets = app.world_mut().resource_mut::<Assets<Material>>();

        material_assets.insert(&Handle::default(), Material::default());
    }
}

#[derive(Asset, Debug, Reflect, Clone)]
pub struct Material {
    pub base_color: Color,
    pub base_color_texture: Option<Handle<Image>>,
    pub normal_map_texture: Option<Handle<Image>>,
    pub occlusion_texture: Option<Handle<Image>>,
    pub uv_transform: Affine2,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            base_color: Color::WHITE,
            base_color_texture: None,
            normal_map_texture: None,
            occlusion_texture: None,
            uv_transform: Affine2::IDENTITY,
        }
    }
}
