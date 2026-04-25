use std::{
    fs::{File, read, write},
    io::Write,
    ops::Not,
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use tomo_image_converter::{
    texture::{
        codecs::bcn::{BcFormat, BcTextureEncoder},
        resize::{ResizeFilter, ResizeType},
        tegra::{TegraTextureEncoder, swizzle_uncompressed_bytes},
    },
    *,
};

use super::{PaintType, open_texture};

const DEFAULT_COMPRESSION_LEVEL: i32 = 19;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Decode canvas or ugctex to image.
    Decode {
        /// Input image e.g UgcFacePaint000.ugctex.zs
        #[arg(short, long)]
        input: PathBuf,
        /// Output image e.g mycat.png
        /// supports avif, bmp, exr, ff, gif, hdr, ico, jpeg, png, pnm, qoi, tga, tiff, and webp.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Compress file using zstd compression
    Compress {
        /// Input image
        #[arg(short, long)]
        input: PathBuf,
        /// Output image
        #[arg(short, long)]
        output: PathBuf,
        /// Compression level
        #[arg(short = 'l', long, default_value_t = DEFAULT_COMPRESSION_LEVEL)]
        compression_level: i32,
    },
    /// Encode image to switch compatible format
    Encode {
        /// Input image
        #[arg(short, long)]
        input: PathBuf,
        /// Resize type
        #[arg(long, value_enum, default_value_t = ResizeType::Exact)]
        resize_method: ResizeType,
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output_dir: PathBuf,
        /// Output type of texture, Default: FacePaint
        #[arg(short = 't', long, value_enum, default_value_t = PaintType::FacePaint)]
        output_type: PaintType,
        /// Texture ID suffix
        #[arg(short, long, default_value_t = 0)]
        number: u32,
        /// Compression level of output textures
        #[arg(short = 'l', long, default_value_t = DEFAULT_COMPRESSION_LEVEL)]
        compression_level: i32,
        /// Skip compression
        #[arg(long, default_value_t = false)]
        skip_compression: bool,
    },
}

impl Command {
    pub fn run(self) -> color_eyre::eyre::Result<()> {
        match self {
            Self::Compress {
                input,
                output,
                compression_level: level,
            } => {
                let input_bytes = read(&input)?;
                let compressed_bytes = zstd_compress_bytes(&input_bytes, level)?;

                write(&output, compressed_bytes)?;
            }
            Self::Decode { input, output } => {
                let texture = open_texture(&input)?;

                texture.as_image().save(&output)?;
            }
            Self::Encode {
                input,
                output_dir,
                output_type,
                number,
                skip_compression,
                compression_level,
                resize_method,
            } => {
                let texture = Texture::from_image(image::open(&input)?);

                let file_stem = format!("{}{number:0>3}", output_type.file_name());
                let size = if output_type == PaintType::Food {
                    FOOD_SIZE
                } else {
                    TEXTURE_SIZE
                };

                let item_texture = texture.resize(size, size, resize_method, ResizeFilter::Nearest);
                let canvas_texture = item_texture.resize(
                    CANVAS_SIZE,
                    CANVAS_SIZE,
                    resize_method,
                    ResizeFilter::Nearest,
                );

                let bc1_encoder = BcTextureEncoder::new(BcFormat::Bc1);
                let texture_bytes = item_texture.encode(TegraTextureEncoder::new(bc1_encoder))?;

                let canvas_bytes = swizzle_uncompressed_bytes(
                    canvas_texture.width(),
                    canvas_texture.height(),
                    &canvas_texture.as_bytes(),
                )?;

                let thumbnail_bytes = output_type.exclude_thumbnail().not().then_some({
                    let bc3_encoder = BcTextureEncoder::new(BcFormat::Bc3);
                    canvas_texture
                        .clone()
                        .encode(TegraTextureEncoder::new(bc3_encoder))?
                });

                let texture_path = output_dir
                    .with_file_name(&file_stem)
                    .with_extension("ugctex");
                let canvas_path = texture_path
                    .with_file_name(&file_stem)
                    .with_extension("canvas");
                let thumbnail_path =
                    texture_path.with_file_name(format!("{file_stem}_Thumb.ugctex"));

                if skip_compression {
                    let mut texture_file = File::create(texture_path)?;

                    texture_file.write_all(&texture_bytes)?;

                    let mut canvas_file = File::create(canvas_path)?;

                    canvas_file.write_all(&canvas_bytes)?;

                    if let Some(thumbnail_bytes) = thumbnail_bytes {
                        let mut thumbnail_file = File::create(thumbnail_path)?;

                        thumbnail_file.write_all(&thumbnail_bytes)?;
                    }
                } else {
                    let mut texture_file = File::create(texture_path.with_added_extension("zs"))?;

                    texture_file
                        .write_all(&zstd_compress_bytes(&texture_bytes, compression_level)?)?;

                    let mut canvas_file = File::create(canvas_path.with_added_extension("zs"))?;

                    canvas_file
                        .write_all(&zstd_compress_bytes(&canvas_bytes, compression_level)?)?;

                    if let Some(thumbnail_bytes) = thumbnail_bytes {
                        let mut thumbnail_file =
                            File::create(thumbnail_path.with_added_extension("zs"))?;

                        thumbnail_file.write_all(&zstd_compress_bytes(
                            &thumbnail_bytes,
                            compression_level,
                        )?)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct App {
    #[command(subcommand)]
    command: Option<Command>,
}

pub fn parse_command() -> Option<Command> {
    App::parse().command
}
