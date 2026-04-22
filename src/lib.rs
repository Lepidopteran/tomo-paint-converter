use std::io::Write;

pub const TEXTURE_SIZE: u32 = 512;
pub const FOOD_SIZE: u32 = 384;
pub const THUMBNAIL_SIZE: u32 = 256;
pub const CANVAS_SIZE: u32 = 256;

pub mod texture;
pub use texture::Texture;

pub fn zstd_compress_bytes(input: &[u8], level: i32) -> color_eyre::Result<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), level)?;

    encoder.set_pledged_src_size(Some(input.len() as u64))?;
    encoder.include_contentsize(true)?;

    encoder.write_all(input)?;

    Ok(encoder.finish()?)
}
