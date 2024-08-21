use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::HANDLE;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handle(pub HANDLE);

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

#[derive(Debug, Clone)]
pub struct SampleDesc {
    pub count: u32,
    pub quality: u32,
}

#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub size: Extent3d,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub sample_desc: SampleDesc,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Extent3d {
    pub width: u32,
    pub height: u32,
    pub depth_or_array_layers: u32,
}

impl Default for Extent3d {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        }
    }
}

impl Extent3d {
    pub fn physical_size(&self, format: TextureFormat) -> Self {
        let (block_width, block_height) = format.block_dimensions();

        let width = ((self.width + block_width - 1) / block_width) * block_width;
        let height = ((self.height + block_height - 1) / block_height) * block_height;

        Self {
            width,
            height,
            depth_or_array_layers: self.depth_or_array_layers,
        }
    }

    /// Calculates the maximum possible count of mipmaps.
    ///
    /// Treats the depth as part of the mipmaps. If calculating
    /// for a 2DArray texture, which does not mipmap depth, set depth to 1.
    pub fn max_mips(&self, dim: TextureDimension) -> u32 {
        match dim {
            TextureDimension::D1 => 1,
            TextureDimension::D2 => {
                let max_dim = self.width.max(self.height);
                32 - max_dim.leading_zeros()
            }
            TextureDimension::D3 => {
                let max_dim = self.width.max(self.height.max(self.depth_or_array_layers));
                32 - max_dim.leading_zeros()
            }
        }
    }

    /// Calculates the extent at a given mip level.
    /// Does *not* account for memory size being a multiple of block size.
    ///
    /// <https://gpuweb.github.io/gpuweb/#logical-miplevel-specific-texture-extent>
    pub fn mip_level_size(&self, level: u32, dim: TextureDimension) -> Self {
        Self {
            width: u32::max(1, self.width >> level),
            height: match dim {
                TextureDimension::D1 => 1,
                _ => u32::max(1, self.height >> level),
            },
            depth_or_array_layers: match dim {
                TextureDimension::D1 => 1,
                TextureDimension::D2 => self.depth_or_array_layers,
                TextureDimension::D3 => u32::max(1, self.depth_or_array_layers >> level),
            },
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureDimension {
    D1,
    D2,
    D3,
}

pub struct TextureViewDescriptor;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub enum ImageSampler {
    #[default]
    Default,
    Descriptor(ImageSamplerDescriptor),
}

impl ImageSampler {
    #[inline]
    pub fn linear() -> ImageSampler {
        ImageSampler::Descriptor(ImageSamplerDescriptor::linear())
    }

    #[inline]
    pub fn nearest() -> ImageSampler {
        ImageSampler::Descriptor(ImageSamplerDescriptor::nearest())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Sampler {
    id: Handle,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum ImageAddressMode {
    #[default]
    ClampToEdge,
    Repeat,
    MirrorRepeat,
    ClampToBorder,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum ImageFilterMode {
    #[default]
    Nearest,
    Linear,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ImageCompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ImageSamplerBorderColor {
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
    Zero,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageSamplerDescriptor {
    pub label: Option<String>,
    pub address_mode_u: ImageAddressMode,
    pub address_mode_v: ImageAddressMode,
    pub address_mode_w: ImageAddressMode,
    pub mag_filter: ImageFilterMode,
    pub min_filter: ImageFilterMode,
    pub mipmap_filter: ImageFilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<ImageCompareFunction>,
    pub anisotropy_clamp: u16,
    pub border_color: Option<ImageSamplerBorderColor>,
}

impl Default for ImageSamplerDescriptor {
    fn default() -> Self {
        Self {
            address_mode_u: Default::default(),
            address_mode_v: Default::default(),
            address_mode_w: Default::default(),
            mag_filter: Default::default(),
            min_filter: Default::default(),
            mipmap_filter: Default::default(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
            label: None,
        }
    }
}

impl ImageSamplerDescriptor {
    #[inline]
    pub fn linear() -> ImageSamplerDescriptor {
        ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            mipmap_filter: ImageFilterMode::Linear,
            ..Default::default()
        }
    }

    #[inline]
    pub fn nearest() -> ImageSamplerDescriptor {
        ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            mipmap_filter: ImageFilterMode::Nearest,
            ..Default::default()
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum TextureFormat {
    R8Unorm,
    R8Snorm,
    R8Uint,
    R8Sint,
    R16Uint,
    R16Sint,
    R16Unorm,
    R16Snorm,
    R16Float,
    Rg8Unorm,
    Rg8Snorm,
    Rg8Uint,
    Rg8Sint,
    R32Uint,
    R32Sint,
    R32Float,
    Rg16Uint,
    Rg16Sint,
    Rg16Unorm,
    Rg16Snorm,
    Rg16Float,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba8Snorm,
    Rgba8Uint,
    Rgba8Sint,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    Rgb9e5Ufloat,
    Rgb10a2Uint,
    Rgb10a2Unorm,
    Rg11b10Float,
    Rg32Uint,
    Rg32Sint,
    Rg32Float,
    Rgba16Uint,
    Rgba16Sint,
    Rgba16Unorm,
    Rgba16Snorm,
    Rgba16Float,
    Rgba32Uint,
    Rgba32Sint,
    Rgba32Float,
    Stencil8,
    Depth16Unorm,
    Depth24Plus,
    Depth24PlusStencil8,
    Depth32Float,
    Depth32FloatStencil8,
    NV12,
    Bc1RgbaUnorm,
    Bc1RgbaUnormSrgb,
    Bc2RgbaUnorm,
    Bc2RgbaUnormSrgb,
    Bc3RgbaUnorm,
    Bc3RgbaUnormSrgb,
    Bc4RUnorm,
    Bc4RSnorm,
    Bc5RgUnorm,
    Bc5RgSnorm,
    Bc6hRgbUfloat,
    Bc6hRgbFloat,
    Bc7RgbaUnorm,
    Bc7RgbaUnormSrgb,
    Etc2Rgb8Unorm,
    Etc2Rgb8UnormSrgb,
    Etc2Rgb8A1Unorm,
    Etc2Rgb8A1UnormSrgb,
    Etc2Rgba8Unorm,
    Etc2Rgba8UnormSrgb,
    EacR11Unorm,
    EacR11Snorm,
    EacRg11Unorm,
    EacRg11Snorm,
}
