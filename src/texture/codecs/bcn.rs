use strum::{Display, EnumIter, EnumString};
use tegra_swizzle::{
    block_height_mip0, div_round_up,
    swizzle::{deswizzle_block_linear, swizzle_block_linear},
};
use texpresso::{Format, Params};

use crate::texture::{
    TextureDecoder, TextureEncoder,
    tegra::{TegraDeswizzle, TegraSwizzle},
};

use super::*;

pub const BLOCK_SIZE: u32 = 4;
pub const BC1_BYTE_SIZE: u32 = 8;
pub const BC3_BYTE_SIZE: u32 = 16;

const DEPTH: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, EnumIter, EnumString, Display)]
#[strum(serialize_all = "UPPERCASE")]
#[non_exhaustive]
pub enum BcFormat {
    Bc1,
    Bc3,
}

pub struct BcTextureEncoder {
    format: BcFormat,
}

impl BcTextureEncoder {
    pub fn new(format: BcFormat) -> Self {
        Self { format }
    }
}

impl TextureEncoder for BcTextureEncoder {
    fn encode_texture(&self, buf: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        Ok(match self.format {
            BcFormat::Bc1 => {
                let bc1 = Format::Bc1;
                let mut compressed_bytes: Vec<u8> =
                    vec![0u8; width as usize * height as usize * BLOCK_SIZE as usize];

                bc1.compress(
                    buf,
                    width as usize,
                    height as usize,
                    Params::default(),
                    &mut compressed_bytes,
                );

                compressed_bytes
            }
            BcFormat::Bc3 => {
                let bc3 = Format::Bc3;
                let mut compressed_bytes: Vec<u8> =
                    vec![0u8; width as usize * height as usize * BLOCK_SIZE as usize];

                bc3.compress(
                    buf,
                    width as usize,
                    height as usize,
                    Params::default(),
                    &mut compressed_bytes,
                );

                compressed_bytes
            }
        })
    }
}

impl TegraSwizzle for BcTextureEncoder {
    fn swizzle_bytes(&self, width: u32, height: u32, bytes: &[u8]) -> Result<Vec<u8>> {
        let width = width.div_ceil(BLOCK_SIZE);
        let height = height.div_ceil(BLOCK_SIZE);
        let mip = block_height_mip0(height);

        let bytes = match self.format {
            BcFormat::Bc1 => swizzle_block_linear(width, height, DEPTH, bytes, mip, BC1_BYTE_SIZE),
            BcFormat::Bc3 => swizzle_block_linear(width, height, DEPTH, bytes, mip, BC3_BYTE_SIZE),
        }?;

        Ok(bytes)
    }
}

#[derive(Debug)]
pub struct BcTextureDecoder {
    format: BcFormat,
}

impl BcTextureDecoder {
    pub fn new(format: BcFormat) -> Self {
        Self { format }
    }
}

impl TextureDecoder for BcTextureDecoder {
    fn decode_bytes(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        Ok(match self.format {
            BcFormat::Bc1 => {
                let bc1 = Format::Bc1;
                let mut decoded_bytes: Vec<u8> = vec![0u8; width as usize * height as usize * 4];

                bc1.decompress(data, width as usize, height as usize, &mut decoded_bytes);

                decoded_bytes
            }
            BcFormat::Bc3 => {
                let bc3 = Format::Bc3;
                let mut decoded_bytes: Vec<u8> = vec![0u8; width as usize * height as usize * 4];

                bc3.decompress(data, width as usize, height as usize, &mut decoded_bytes);

                decoded_bytes
            }
        })
    }
}

impl TegraDeswizzle for BcTextureDecoder {
    fn deswizzle_bytes(
        &self,
        width: u32,
        height: u32,
        bytes: &[u8],
    ) -> color_eyre::Result<Vec<u8>> {
        let width = width.div_ceil(BLOCK_SIZE);
        let height = height.div_ceil(BLOCK_SIZE);
        let mip = block_height_mip0(height);

        let bytes = match self.format {
            BcFormat::Bc1 => {
                deswizzle_block_linear(width, height, DEPTH, bytes, mip, BC1_BYTE_SIZE)
            }
            BcFormat::Bc3 => {
                deswizzle_block_linear(width, height, DEPTH, bytes, mip, BC3_BYTE_SIZE)
            }
        }?;

        Ok(bytes)
    }
}
