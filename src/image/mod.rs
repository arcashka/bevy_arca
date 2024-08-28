mod sampler;

use bevy::prelude::*;

use bevy::asset::Handle;
use image::DynamicImage;
use sampler::Sampler;
use windows::Win32::Graphics::{
    Direct3D12::{
        D3D12_MIP_REGION, D3D12_RESOURCE_DESC1, D3D12_RESOURCE_DIMENSION,
        D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_NONE,
        D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_SAMPLE_DESC},
};

use crate::win_types::WinHandle;

pub const TRANSPARENT_IMAGE_HANDLE: Handle<Image> =
    Handle::weak_from_u128(154728948001857810431816125397303024160);

pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Image>()
            .init_asset::<Image>()
            .register_asset_reflect::<Image>();

        let mut image_assets = app.world_mut().resource_mut::<Assets<Image>>();

        image_assets.insert(&Handle::default(), Image::default());
        image_assets.insert(&TRANSPARENT_IMAGE_HANDLE, Image::new());
    }
}

#[derive(Asset, Reflect, Debug, Clone, Default)]
#[reflect_value(Default)]
pub struct Image {
    pub data: Vec<u8>,
    pub texture_descriptor: D3D12_RESOURCE_DESC1,
    pub sampler: Sampler,
    pub texture_view_descriptor: Option<WinHandle>,
}

impl Image {
    pub fn new() -> Self {
        let format = DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;
        let data = vec![255, 255, 255, 0];
        Self {
            data,
            texture_descriptor: D3D12_RESOURCE_DESC1 {
                Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                Alignment: 0,
                Width: 1,
                Height: 1,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: format,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                Flags: D3D12_RESOURCE_FLAG_NONE,
                SamplerFeedbackMipRegion: D3D12_MIP_REGION {
                    Width: 1,
                    Height: 1,
                    Depth: 1,
                },
            },
            sampler: Sampler::default(),
            texture_view_descriptor: None,
        }
    }

    pub fn from_dynamic(image: DynamicImage) -> Self {
        let image = image.into_rgba8();
        let width = image.width();
        let height = image.height();
        let data = image.into_raw();

        Self::from_buffer(
            Size { width, height },
            D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            &data,
        )
    }

    pub fn from_buffer(size: Size, dimension: D3D12_RESOURCE_DIMENSION, pixel: &[u8]) -> Self {
        debug_assert_eq!(pixel.len(), (size.width * size.height * 4) as usize);
        Image {
            data: pixel.to_vec(),
            texture_descriptor: D3D12_RESOURCE_DESC1 {
                Dimension: dimension,
                Alignment: 0,
                Width: size.width as u64,
                Height: size.height,
                DepthOrArraySize: 1,
                MipLevels: 1,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
                Flags: D3D12_RESOURCE_FLAG_NONE,
                SamplerFeedbackMipRegion: D3D12_MIP_REGION {
                    Width: 1,
                    Height: 1,
                    Depth: 1,
                },
            },
            sampler: Sampler::default(),
            texture_view_descriptor: None,
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.texture_descriptor.Width as u32
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.texture_descriptor.Height
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl Size {
    pub fn volume(&self) -> usize {
        (self.width * self.height) as usize
    }
}
