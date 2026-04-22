use color_eyre::eyre::Result;
use tegra_swizzle::{
    block_height_mip0,
    swizzle::{deswizzle_block_linear, swizzle_block_linear},
};

const UNCOMPRESSED_BYTE_SIZE: u32 = 4;

use super::{TextureDecoder, TextureEncoder};

pub trait TegraSwizzle {
    fn swizzle_bytes(&self, width: u32, height: u32, bytes: &[u8]) -> Result<Vec<u8>>;
}

pub trait TegraDeswizzle {
    fn deswizzle_bytes(&self, width: u32, height: u32, bytes: &[u8]) -> Result<Vec<u8>>;
}

pub trait TegraEncoder: TextureEncoder + TegraSwizzle {
    fn encode_swizzled_texture(&self, buf: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        self.swizzle_bytes(width, height, &self.encode_texture(buf, width, height)?)
    }
}

impl<T> TegraEncoder for T where T: TextureEncoder + TegraSwizzle {}

pub trait TegraDecoder: TextureDecoder + TegraDeswizzle {
    fn decode_swizzled_texture(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        self.decode_bytes(&self.deswizzle_bytes(width, height, data)?, width, height)
    }
}

impl<T> TegraDecoder for T where T: TextureDecoder + TegraDeswizzle {}

pub struct TegraTextureEncoder<'e> {
    inner: Box<dyn TegraEncoder + 'e>,
}

impl<'e> TegraTextureEncoder<'e> {
    pub fn new(encoder: impl TegraEncoder + 'e) -> Self {
        Self {
            inner: Box::new(encoder),
        }
    }

    pub fn new_with_boxed_encoder(encoder: Box<dyn TegraEncoder>) -> Self {
        Self { inner: encoder }
    }
}

impl TextureEncoder for TegraTextureEncoder<'_> {
    fn encode_texture(&self, buf: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        self.inner.encode_swizzled_texture(buf, width, height)
    }
}

pub struct TegraTextureDecoder<'d> {
    inner: Box<dyn TegraDecoder + 'd>,
}

impl<'d> TegraTextureDecoder<'d> {
    pub fn new(decoder: impl TegraDecoder + 'd) -> Self {
        Self {
            inner: Box::new(decoder),
        }
    }

    pub fn new_with_boxed_decoder(decoder: Box<dyn TegraDecoder>) -> Self {
        Self { inner: decoder }
    }
}

impl TextureDecoder for TegraTextureDecoder<'_> {
    fn decode_bytes(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        self.inner.decode_swizzled_texture(data, width, height)
    }
}

pub fn swizzle_uncompressed_bytes(width: u32, height: u32, bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(swizzle_block_linear(
        width,
        height,
        1,
        bytes,
        block_height_mip0(height),
        UNCOMPRESSED_BYTE_SIZE,
    )?)
}

pub fn deswizzle_uncompressed_bytes(width: u32, height: u32, bytes: &[u8]) -> Result<Vec<u8>> {
    Ok(deswizzle_block_linear(
        width,
        height,
        1,
        bytes,
        block_height_mip0(height),
        UNCOMPRESSED_BYTE_SIZE,
    )?)
}
