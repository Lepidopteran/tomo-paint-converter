use color_eyre::eyre::Result;
use image::{DynamicImage, EncodableLayout, ImageBuffer};

pub mod codecs;
pub mod resize;
pub mod tegra;

#[derive(Debug, Clone)]
pub struct Texture {
    inner_image: DynamicImage,
}

impl Texture {
    pub fn from_decoder<T: TextureDecoder>(
        bytes: Vec<u8>,
        decoder: T,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            inner_image: DynamicImage::ImageRgba8(
                ImageBuffer::from_raw(
                    width,
                    height,
                    decoder
                        .decode_bytes(&bytes, width, height)
                        .expect("Failed to decode texture"),
                )
                .expect("Failed to create image buffer"),
            ),
        }
    }

    pub fn from_bytes(bytes: Vec<u8>, width: u32, height: u32) -> Self {
        Self {
            inner_image: DynamicImage::ImageRgba8(
                ImageBuffer::from_raw(width, height, bytes).expect("Failed to create image buffer"),
            ),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        self.inner_image.to_rgba8().as_bytes().to_vec()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.into_image().to_rgba8().as_bytes().to_vec()
    }

    pub fn encode<T: TextureEncoder>(self, encoder: T) -> Result<Vec<u8>> {
        encoder.encode_texture(&self.as_bytes(), self.width(), self.height())
    }

    pub fn from_image(image: DynamicImage) -> Self {
        Self { inner_image: image }
    }

    pub fn into_image(self) -> DynamicImage {
        self.inner_image
    }

    pub fn as_image(&self) -> &DynamicImage {
        &self.inner_image
    }

    pub fn width(&self) -> u32 {
        self.inner_image.width()
    }

    pub fn height(&self) -> u32 {
        self.inner_image.height()
    }

    pub fn resize(
        &self,
        nwidth: u32,
        nheight: u32,
        method: resize::ResizeType,
        filter: resize::ResizeFilter,
    ) -> Texture {
        resize::resize(self, nwidth, nheight, method, filter)
    }
}

pub trait TextureEncoder {
    fn encode_texture(&self, buf: &[u8], width: u32, height: u32) -> Result<Vec<u8>>;
}

pub trait TextureDecoder {
    fn decode_bytes(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>>;
}
