mod image;

use bevy::{
    app::{App, Plugin},
    asset::{Asset, AssetApp, Assets, Handle},
    reflect::Reflect,
};

use self::image::ImageSamplerDescriptor;

pub const TRANSPARENT_IMAGE_HANDLE: Handle<Image> =
    Handle::weak_from_u128(154728948001857810431816125397303024160);

pub struct ImagePlugin {
    pub default_sampler: ImageSamplerDescriptor,
}

impl Default for ImagePlugin {
    fn default() -> Self {
        ImagePlugin::default_linear()
    }
}

impl ImagePlugin {
    pub fn default_linear() -> ImagePlugin {
        ImagePlugin {
            default_sampler: ImageSamplerDescriptor::linear(),
        }
    }

    pub fn default_nearest() -> ImagePlugin {
        ImagePlugin {
            default_sampler: ImageSamplerDescriptor::nearest(),
        }
    }
}

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Image>()
            .init_asset::<Image>()
            .register_asset_reflect::<Image>();

        let mut image_assets = app.world_mut().resource_mut::<Assets<Image>>();

        image_assets.insert(&Handle::default(), Image::default());
        image_assets.insert(&TRANSPARENT_IMAGE_HANDLE, Image::transparent());

        #[cfg(feature = "basis-universal")]
        if let Some(processor) = app
            .world()
            .get_resource::<bevy_asset::processor::AssetProcessor>()
        {
            processor.register_processor::<bevy_asset::processor::LoadAndSave<ImageLoader, CompressedImageSaver>>(
                CompressedImageSaver.into(),
            );
            processor
                .set_default_processor::<bevy_asset::processor::LoadAndSave<ImageLoader, CompressedImageSaver>>("png");
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TextureCache>().add_systems(
                Render,
                update_texture_cache_system.in_set(RenderSet::Cleanup),
            );
        }

        #[cfg(any(
            feature = "png",
            feature = "dds",
            feature = "tga",
            feature = "jpeg",
            feature = "bmp",
            feature = "basis-universal",
            feature = "ktx2",
            feature = "webp",
            feature = "pnm"
        ))]
        app.preregister_asset_loader::<ImageLoader>(IMG_FILE_EXTENSIONS);
    }

    fn finish(&self, app: &mut App) {
        #[cfg(any(
            feature = "png",
            feature = "dds",
            feature = "tga",
            feature = "jpeg",
            feature = "bmp",
            feature = "basis-universal",
            feature = "ktx2",
            feature = "webp",
            feature = "pnm"
        ))]
        {
            app.init_asset_loader::<ImageLoader>();
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let default_sampler = {
                let device = render_app.world().resource::<RenderDevice>();
                device.create_sampler(&self.default_sampler.as_wgpu())
            };
            render_app
                .insert_resource(DefaultImageSampler(default_sampler))
                .init_resource::<FallbackImage>()
                .init_resource::<FallbackImageZero>()
                .init_resource::<FallbackImageCubemap>()
                .init_resource::<FallbackImageFormatMsaaCache>();
        }
    }
}
