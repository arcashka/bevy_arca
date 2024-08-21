use bevy::{
    asset::Asset,
    ecs::system::Resource,
    prelude::{Deref, DerefMut, ReflectDefault},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::gpu_types::{
    Extent3d, Handle, ImageSampler, Sampler, TextureDescriptor, TextureDimension, TextureFormat,
};

#[derive(Asset, Reflect, Debug, Clone)]
#[reflect_value(Default)]
pub struct Image {
    pub data: Vec<u8>,
    pub texture_descriptor: TextureDescriptor,
    pub sampler: ImageSampler,
    pub texture_view_descriptor: Option<Handle>,
}

#[derive(Resource, Debug, Clone, Deref, DerefMut)]
pub struct DefaultImageSampler(pub(crate) Sampler);

impl Image {
    /// Creates a new image from raw binary data and the corresponding metadata.
    ///
    /// # Panics
    /// Panics if the length of the `data`, volume of the `size` and the size of the `format`
    /// do not match.
    pub fn new(
        size: Extent3d,
        dimension: TextureDimension,
        data: Vec<u8>,
        format: TextureFormat,
    ) -> Self {
        debug_assert_eq!(
            size.volume() * format.pixel_size(),
            data.len(),
            "Pixel data, size and format have to match",
        );
        let mut image = Self {
            data,
            ..Default::default()
        };
        image.texture_descriptor.dimension = dimension;
        image.texture_descriptor.size = size;
        image.texture_descriptor.format = format;
        image
    }

    pub fn transparent() -> Image {
        let format = TextureFormat::Rgba8UnormSrgb;
        debug_assert!(format.pixel_size() == 4);
        let data = vec![255, 255, 255, 0];
        Image {
            data,
            texture_descriptor: wgpu::TextureDescriptor {
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                format,
                dimension: TextureDimension::D2,
                label: None,
                mip_level_count: 1,
                sample_count: 1,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            sampler: ImageSampler::Default,
            texture_view_descriptor: None,
        }
    }

    /// Creates a new image from raw binary data and the corresponding metadata, by filling
    /// the image data with the `pixel` data repeated multiple times.
    ///
    /// # Panics
    /// Panics if the size of the `format` is not a multiple of the length of the `pixel` data.
    pub fn new_fill(
        size: Extent3d,
        dimension: TextureDimension,
        pixel: &[u8],
        format: TextureFormat,
        asset_usage: RenderAssetUsages,
    ) -> Self {
        let mut value = Image::default();
        value.texture_descriptor.format = format;
        value.texture_descriptor.dimension = dimension;
        value.asset_usage = asset_usage;
        value.resize(size);

        debug_assert_eq!(
            pixel.len() % format.pixel_size(),
            0,
            "Must not have incomplete pixel data (pixel size is {}B).",
            format.pixel_size(),
        );
        debug_assert!(
            pixel.len() <= value.data.len(),
            "Fill data must fit within pixel buffer (expected {}B).",
            value.data.len(),
        );

        for current_pixel in value.data.chunks_exact_mut(pixel.len()) {
            current_pixel.copy_from_slice(pixel);
        }
        value
    }

    /// Returns the width of a 2D image.
    #[inline]
    pub fn width(&self) -> u32 {
        self.texture_descriptor.size.width
    }

    /// Returns the height of a 2D image.
    #[inline]
    pub fn height(&self) -> u32 {
        self.texture_descriptor.size.height
    }

    /// Returns the aspect ratio (width / height) of a 2D image.
    #[inline]
    pub fn aspect_ratio(&self) -> AspectRatio {
        AspectRatio::from_pixels(self.width(), self.height())
    }

    /// Returns the size of a 2D image as f32.
    #[inline]
    pub fn size_f32(&self) -> Vec2 {
        Vec2::new(self.width() as f32, self.height() as f32)
    }

    /// Returns the size of a 2D image.
    #[inline]
    pub fn size(&self) -> UVec2 {
        UVec2::new(self.width(), self.height())
    }

    /// Resizes the image to the new size, by removing information or appending 0 to the `data`.
    /// Does not properly resize the contents of the image, but only its internal `data` buffer.
    pub fn resize(&mut self, size: Extent3d) {
        self.texture_descriptor.size = size;
        self.data.resize(
            size.volume() * self.texture_descriptor.format.pixel_size(),
            0,
        );
    }

    /// Changes the `size`, asserting that the total number of data elements (pixels) remains the
    /// same.
    ///
    /// # Panics
    /// Panics if the `new_size` does not have the same volume as to old one.
    pub fn reinterpret_size(&mut self, new_size: Extent3d) {
        assert_eq!(
            new_size.volume(),
            self.texture_descriptor.size.volume(),
            "Incompatible sizes: old = {:?} new = {:?}",
            self.texture_descriptor.size,
            new_size
        );

        self.texture_descriptor.size = new_size;
    }

    /// Takes a 2D image containing vertically stacked images of the same size, and reinterprets
    /// it as a 2D array texture, where each of the stacked images becomes one layer of the
    /// array. This is primarily for use with the `texture2DArray` shader uniform type.
    ///
    /// # Panics
    /// Panics if the texture is not 2D, has more than one layers or is not evenly dividable into
    /// the `layers`.
    pub fn reinterpret_stacked_2d_as_array(&mut self, layers: u32) {
        // Must be a stacked image, and the height must be divisible by layers.
        assert_eq!(self.texture_descriptor.dimension, TextureDimension::D2);
        assert_eq!(self.texture_descriptor.size.depth_or_array_layers, 1);
        assert_eq!(self.height() % layers, 0);

        self.reinterpret_size(Extent3d {
            width: self.width(),
            height: self.height() / layers,
            depth_or_array_layers: layers,
        });
    }

    /// Convert a texture from a format to another. Only a few formats are
    /// supported as input and output:
    /// - `TextureFormat::R8Unorm`
    /// - `TextureFormat::Rg8Unorm`
    /// - `TextureFormat::Rgba8UnormSrgb`
    ///
    /// To get [`Image`] as a [`image::DynamicImage`] see:
    /// [`Image::try_into_dynamic`].
    pub fn convert(&self, new_format: TextureFormat) -> Option<Self> {
        self.clone()
            .try_into_dynamic()
            .ok()
            .and_then(|img| match new_format {
                TextureFormat::R8Unorm => {
                    Some((image::DynamicImage::ImageLuma8(img.into_luma8()), false))
                }
                TextureFormat::Rg8Unorm => Some((
                    image::DynamicImage::ImageLumaA8(img.into_luma_alpha8()),
                    false,
                )),
                TextureFormat::Rgba8UnormSrgb => {
                    Some((image::DynamicImage::ImageRgba8(img.into_rgba8()), true))
                }
                _ => None,
            })
            .map(|(dyn_img, is_srgb)| Self::from_dynamic(dyn_img, is_srgb, self.asset_usage))
    }

    /// Load a bytes buffer in a [`Image`], according to type `image_type`, using the `image`
    /// crate
    pub fn from_buffer(
        #[cfg(all(debug_assertions, feature = "dds"))] name: String,
        buffer: &[u8],
        image_type: ImageType,
        #[allow(unused_variables)] supported_compressed_formats: CompressedImageFormats,
        is_srgb: bool,
        image_sampler: ImageSampler,
        asset_usage: RenderAssetUsages,
    ) -> Result<Image, TextureError> {
        let format = image_type.to_image_format()?;

        // Load the image in the expected format.
        // Some formats like PNG allow for R or RG textures too, so the texture
        // format needs to be determined. For RGB textures an alpha channel
        // needs to be added, so the image data needs to be converted in those
        // cases.

        let mut image = match format {
            #[cfg(feature = "basis-universal")]
            ImageFormat::Basis => {
                basis_buffer_to_image(buffer, supported_compressed_formats, is_srgb)?
            }
            #[cfg(feature = "dds")]
            ImageFormat::Dds => dds_buffer_to_image(
                #[cfg(debug_assertions)]
                name,
                buffer,
                supported_compressed_formats,
                is_srgb,
            )?,
            #[cfg(feature = "ktx2")]
            ImageFormat::Ktx2 => {
                ktx2_buffer_to_image(buffer, supported_compressed_formats, is_srgb)?
            }
            _ => {
                let image_crate_format = format
                    .as_image_crate_format()
                    .ok_or_else(|| TextureError::UnsupportedTextureFormat(format!("{format:?}")))?;
                let mut reader = image::ImageReader::new(std::io::Cursor::new(buffer));
                reader.set_format(image_crate_format);
                reader.no_limits();
                let dyn_img = reader.decode()?;
                Self::from_dynamic(dyn_img, is_srgb, asset_usage)
            }
        };
        image.sampler = image_sampler;
        Ok(image)
    }

    /// Whether the texture format is compressed or uncompressed
    pub fn is_compressed(&self) -> bool {
        let format_description = self.texture_descriptor.format;
        format_description
            .required_features()
            .contains(wgpu::Features::TEXTURE_COMPRESSION_ASTC)
            || format_description
                .required_features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC)
            || format_description
                .required_features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_ETC2)
    }
}
