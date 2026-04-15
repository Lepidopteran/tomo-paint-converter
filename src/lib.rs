use image::{DynamicImage, ImageBuffer, RgbaImage};
use tegra_swizzle::{block_height_mip0, div_round_up, swizzle::deswizzle_block_linear};

const TEXTURE_SIZE: u32 = 512;
const FOOD_SIZE: u32 = 384;
const THUMB_SIZE: u32 = 256;
const CANVAS_SIZE: u32 = 256;

pub fn image_from_canvas(bytes: &[u8]) -> color_eyre::Result<DynamicImage> {
    let deswizzled_bytes = deswizzle_block_linear(
        CANVAS_SIZE,
        CANVAS_SIZE,
        1,
        bytes,
        block_height_mip0(CANVAS_SIZE),
        4,
    )?;

    let buffer = ImageBuffer::from_raw(CANVAS_SIZE, CANVAS_SIZE, deswizzled_bytes)
        .expect("Failed to create image buffer");

    Ok(DynamicImage::ImageRgba8(buffer))
}

pub fn image_from_ugctex_thumb(bytes: &[u8]) -> color_eyre::Result<DynamicImage> {
    let size = div_round_up(THUMB_SIZE, 4);

    let block_height = block_height_mip0(size);
    let deswizzled_bytes = deswizzle_block_linear(size, size, 1, bytes, block_height, 16)?;

    let bc3 = texpresso::Format::Bc3;
    let mut rgba_buffer: Vec<u8> = vec![0; THUMB_SIZE as usize * THUMB_SIZE as usize * 4];

    bc3.decompress(
        &deswizzled_bytes,
        THUMB_SIZE as usize,
        THUMB_SIZE as usize,
        &mut rgba_buffer,
    );

    Ok(DynamicImage::ImageRgba8(
        RgbaImage::from_raw(THUMB_SIZE, THUMB_SIZE, rgba_buffer)
            .expect("Failed to create rgba image"),
    ))
}

pub fn image_from_ugctex(bytes: &[u8], food: bool) -> color_eyre::Result<DynamicImage> {
    let size = if food { FOOD_SIZE } else { TEXTURE_SIZE };

    let compressed_size = div_round_up(size, 4);

    let block_height = block_height_mip0(compressed_size);
    let deswizzled_bytes =
        deswizzle_block_linear(compressed_size, compressed_size, 1, bytes, block_height, 8)?;

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
