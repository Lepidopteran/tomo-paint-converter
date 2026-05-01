use std::{fs::read, io::Cursor, path::Path};

use color_eyre::eyre::{OptionExt, Result};
use image::{ImageError, ImageFormat, ImageReader, guess_format};
use tomo_image_converter::{
    CANVAS_SIZE, FOOD_SIZE, TEXTURE_SIZE, THUMBNAIL_SIZE, Texture,
    texture::{
        codecs::bcn::{BC1_BYTE_SIZE, BC3_BYTE_SIZE, BLOCK_SIZE, BcFormat, BcTextureDecoder},
        tegra::{TegraTextureDecoder, deswizzle_uncompressed_bytes},
    },
};
use tracing::{debug, info};

pub fn is_zstd_compressed(bytes: &[u8]) -> bool {
    bytes.len() > 4 && bytes[0..4] == [0x28, 0xb5, 0x2f, 0xfd]
}

pub fn is_item_texture(bytes: &[u8]) -> bool {
    const VALID_SIZES: &[usize; 3] = &[
        (FOOD_SIZE.div_ceil(BLOCK_SIZE) * FOOD_SIZE.div_ceil(BLOCK_SIZE) * BC1_BYTE_SIZE) as usize,
        // Not sure why this validates to correct
        (FOOD_SIZE.div_ceil(BLOCK_SIZE) * TEXTURE_SIZE.div_ceil(BLOCK_SIZE) * BC1_BYTE_SIZE)
            as usize,
        (TEXTURE_SIZE.div_ceil(BLOCK_SIZE) * TEXTURE_SIZE.div_ceil(BLOCK_SIZE) * BC1_BYTE_SIZE)
            as usize,
    ];

    VALID_SIZES.contains(&bytes.len())
}

pub fn is_item_thumbnail(bytes: &[u8]) -> bool {
    bytes.len()
        == (THUMBNAIL_SIZE.div_ceil(BLOCK_SIZE)
            * THUMBNAIL_SIZE.div_ceil(BLOCK_SIZE)
            * BC3_BYTE_SIZE) as usize
}

pub fn is_canvas(bytes: &[u8]) -> bool {
    bytes.len() == (CANVAS_SIZE * CANVAS_SIZE * 4) as usize
}

pub fn open_file(path: impl AsRef<Path>) -> Result<Texture> {
    let path = path.as_ref();
    let bytes = read(path)?;

    let format = guess_format(&bytes).or_else(|err| match err {
        ImageError::Unsupported(_) => {
            let extension = path
                .extension()
                .ok_or_eyre("Could not determine file type")?;

            ImageFormat::from_extension(extension).ok_or_eyre("Could not determine file type")
        }
        _ => Err(err.into()),
    });

    if let Ok(format) = format {
        let mut reader = ImageReader::new(Cursor::new(bytes));
        reader.set_format(format);

        Ok(reader.decode().map(Texture::from_image)?)
    } else {
        load_texture(&bytes)
    }
}

pub fn open_texture(path: impl AsRef<Path>) -> Result<Texture> {
    let path = path.as_ref();
    let bytes = read(path)?;

    load_texture(&bytes)
}

pub fn load_texture(bytes: &[u8]) -> Result<Texture> {
    let mut bytes = bytes.to_vec();

    debug!("Texture size: {}", bytes.len());

    if is_zstd_compressed(&bytes) {
        let cursor = Cursor::new(bytes);
        bytes = zstd::decode_all(cursor)?;
        debug!("Uncompressed Texture size: {}", bytes.len());
    }

    let texture = if is_item_texture(&bytes) {
        let decoder = BcTextureDecoder::new(BcFormat::Bc1);
        info!("Decoding texture");

        Texture::from_decoder(
            bytes.to_vec(),
            TegraTextureDecoder::new(decoder),
            TEXTURE_SIZE,
            TEXTURE_SIZE,
        )
        .or_else(|_| {
            let decoder = BcTextureDecoder::new(BcFormat::Bc1);
            Texture::from_decoder(
                bytes.to_vec(),
                TegraTextureDecoder::new(decoder),
                FOOD_SIZE,
                FOOD_SIZE,
            )
        })?
    } else if is_item_thumbnail(&bytes) {
        let decoder = BcTextureDecoder::new(BcFormat::Bc3);
        info!("Decoding thumbnail");

        Texture::from_decoder(
            bytes.to_vec(),
            TegraTextureDecoder::new(decoder),
            THUMBNAIL_SIZE,
            THUMBNAIL_SIZE,
        )?
    } else if is_canvas(&bytes) {
        info!("Deswizzling canvas");
        Texture::from_bytes(
            deswizzle_uncompressed_bytes(CANVAS_SIZE, CANVAS_SIZE, &bytes)?,
            CANVAS_SIZE,
            CANVAS_SIZE,
        )?
    } else {
        Err(color_eyre::eyre::eyre!(
            "Could't determine texture type for file",
        ))?
    };

    Ok(texture)
}
