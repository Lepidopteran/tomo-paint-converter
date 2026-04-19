use std::{fmt::Display, io::Write, str::FromStr};

use image::{DynamicImage, ImageBuffer, RgbaImage};
use slint::{SharedString, VecModel};
use tegra_swizzle::{
    block_height_mip0, div_round_up,
    swizzle::{deswizzle_block_linear, swizzle_block_linear},
};
use texpresso::Params;

mod resize;
pub use resize::*;

pub const TEXTURE_SIZE: u32 = 512;
pub const FOOD_SIZE: u32 = 384;
pub const THUMBNAIL_SIZE: u32 = 256;
pub const CANVAS_SIZE: u32 = 256;

const DEPTH: u32 = 1;
const UNCOMPRESSED_BLOCK_SIZE: u32 = 4;
const BC1_BLOCK_SIZE: u32 = 8;
const BC3_BLOCK_SIZE: u32 = 16;

/// Output type of texture
#[derive(clap::ValueEnum, Debug, Clone, Copy, Eq, PartialEq)]
pub enum PaintType {
    Food,
    FacePaint,
    Interior,
    Exterior,
    Treasure,
    Cloth,
    Terrain,
    Object,
}

impl PaintType {
    pub fn file_name(&self) -> &'static str {
        match self {
            PaintType::Food => "UgcFood",
            PaintType::FacePaint => "UgcFacePaint",
            PaintType::Interior => "UgcInterior",
            PaintType::Exterior => "UgcExterior",
            PaintType::Treasure => "UgcGoods",
            PaintType::Cloth => "UgcCloth",
            PaintType::Terrain => "UgcMapFloor",
            PaintType::Object => "UgcMapObject",
        }
    }

    pub fn has_thumbnail(&self) -> bool {
        !matches!(self, Self::FacePaint)
    }

    pub fn has_canvas(&self) -> bool {
        true
    }

    pub fn has_texture(&self) -> bool {
        true
    }

    pub fn model() -> VecModel<SharedString> {
        VecModel::from(
            vec![
                PaintType::FacePaint.to_string(),
                PaintType::Food.to_string(),
                PaintType::Interior.to_string(),
                PaintType::Exterior.to_string(),
                PaintType::Treasure.to_string(),
                PaintType::Cloth.to_string(),
                PaintType::Terrain.to_string(),
                PaintType::Object.to_string(),
            ]
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<_>>(),
        )
    }
}

impl Display for PaintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PaintType::Food => "Food",
            PaintType::FacePaint => "Face Paint",
            PaintType::Interior => "Interior",
            PaintType::Exterior => "Exterior",
            PaintType::Treasure => "Treasure",
            PaintType::Cloth => "Cloth",
            PaintType::Terrain => "Terrain",
            PaintType::Object => "Object",
        })
    }
}

impl FromStr for PaintType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(" ", "").as_str() {
            "food" => Ok(PaintType::Food),
            "facepaint" => Ok(PaintType::FacePaint),
            "interior" => Ok(PaintType::Interior),
            "exterior" => Ok(PaintType::Exterior),
            "treasure" => Ok(PaintType::Treasure),
            "cloth" => Ok(PaintType::Cloth),
            "terrain" => Ok(PaintType::Terrain),
            "object" => Ok(PaintType::Object),
            _ => Err(format!("Invalid paint type: {}", s)),
        }
    }
}

pub fn bc1_compress_bytes(size: u32, bytes: &[u8]) -> color_eyre::Result<Vec<u8>> {
    let bc1 = texpresso::Format::Bc1;
    let mut encoded_bytes: Vec<u8> = vec![0u8; bc1.compressed_size(size as usize, size as usize)];

    bc1.compress(
        bytes,
        size as usize,
        size as usize,
        Params::default(),
        &mut encoded_bytes,
    );

    Ok(encoded_bytes)
}

pub fn bc3_compress_bytes(size: u32, bytes: &[u8]) -> color_eyre::Result<Vec<u8>> {
    let bc3 = texpresso::Format::Bc3;
    let mut encoded_bytes: Vec<u8> = vec![0u8; bc3.compressed_size(size as usize, size as usize)];

    bc3.compress(
        bytes,
        size as usize,
        size as usize,
        Params::default(),
        &mut encoded_bytes,
    );

    Ok(encoded_bytes)
}

pub fn image_from_canvas(bytes: &[u8]) -> color_eyre::Result<DynamicImage> {
    let deswizzled_bytes = deswizzle_block_linear(
        CANVAS_SIZE,
        CANVAS_SIZE,
        DEPTH,
        bytes,
        block_height_mip0(CANVAS_SIZE),
        UNCOMPRESSED_BLOCK_SIZE,
    )?;

    let buffer = ImageBuffer::from_raw(CANVAS_SIZE, CANVAS_SIZE, deswizzled_bytes)
        .expect("Failed to create image buffer");

    Ok(DynamicImage::ImageRgba8(buffer))
}

pub fn canvas_from_image(image: &DynamicImage) -> color_eyre::Result<Vec<u8>> {
    let rgba_image = image.to_rgba8();

    Ok(swizzle_block_linear(
        CANVAS_SIZE,
        CANVAS_SIZE,
        1,
        rgba_image.as_raw(),
        block_height_mip0(CANVAS_SIZE),
        UNCOMPRESSED_BLOCK_SIZE,
    )?)
}

pub fn thumbnail_from_image(image: &DynamicImage) -> color_eyre::Result<Vec<u8>> {
    let compressed_size = div_round_up(THUMBNAIL_SIZE, 4);
    Ok(swizzle_block_linear(
        compressed_size,
        compressed_size,
        DEPTH,
        &bc3_compress_bytes(THUMBNAIL_SIZE, image.to_rgba8().as_raw())?,
        block_height_mip0(compressed_size),
        BC3_BLOCK_SIZE,
    )?)
}

pub fn image_from_thumbnail(bytes: &[u8]) -> color_eyre::Result<DynamicImage> {
    let size = div_round_up(THUMBNAIL_SIZE, 4);

    let block_height = block_height_mip0(size);
    let deswizzled_bytes =
        deswizzle_block_linear(size, size, 1, bytes, block_height, BC3_BLOCK_SIZE)?;

    let bc3 = texpresso::Format::Bc3;
    let mut rgba_buffer: Vec<u8> = vec![0; THUMBNAIL_SIZE as usize * THUMBNAIL_SIZE as usize * 4];

    bc3.decompress(
        &deswizzled_bytes,
        THUMBNAIL_SIZE as usize,
        THUMBNAIL_SIZE as usize,
        &mut rgba_buffer,
    );

    Ok(DynamicImage::ImageRgba8(
        RgbaImage::from_raw(THUMBNAIL_SIZE, THUMBNAIL_SIZE, rgba_buffer)
            .expect("Failed to create rgba image"),
    ))
}

pub fn image_from_texture(bytes: &[u8], food: bool) -> color_eyre::Result<DynamicImage> {
    let size = if food { FOOD_SIZE } else { TEXTURE_SIZE };

    let compressed_size = div_round_up(size, 4);

    let block_height = block_height_mip0(compressed_size);
    let deswizzled_bytes = deswizzle_block_linear(
        compressed_size,
        compressed_size,
        1,
        bytes,
        block_height,
        BC1_BLOCK_SIZE,
    )?;

    let bc1 = texpresso::Format::Bc1;
    let mut rgba_buffer: Vec<u8> = vec![0; size as usize * size as usize * 4];

    bc1.decompress(
        &deswizzled_bytes,
        size as usize,
        size as usize,
        &mut rgba_buffer,
    );

    Ok(DynamicImage::ImageRgba8(
        RgbaImage::from_raw(size, size, rgba_buffer).expect("Failed to create rgba image"),
    ))
}

pub fn texture_from_image(image: &DynamicImage, food: bool) -> color_eyre::Result<Vec<u8>> {
    let size = if food { FOOD_SIZE } else { TEXTURE_SIZE };

    let compressed_size = div_round_up(size, 4);
    Ok(swizzle_block_linear(
        compressed_size,
        compressed_size,
        1,
        &bc1_compress_bytes(size, image.to_rgba8().as_raw())?,
        block_height_mip0(compressed_size),
        BC1_BLOCK_SIZE,
    )?)
}

pub fn compress(input: &[u8], level: i32) -> color_eyre::Result<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), level)?;

    encoder.set_pledged_src_size(Some(input.len() as u64))?;
    encoder.include_contentsize(true)?;

    encoder.write_all(input)?;

    Ok(encoder.finish()?)
}
