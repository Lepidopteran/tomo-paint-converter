use std::{
    fs::{read, write},
    io::Write,
    path::PathBuf,
};
use tomo_image_converter::*;

use clap::{Parser, Subcommand};

const COMPRESSION_LEVEL: i32 = 19;

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
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum ResizeType {
    /// Preserve image aspect ratio
    Preserve,
    /// Fill image preserving aspect ratio and cropping
    Fill,
    /// Resize image to exact size, ignoring aspect ratio
    Exact,
}

#[derive(Subcommand, Debug)]
enum Command {
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
        #[arg(short = 'l', long, default_value_t = COMPRESSION_LEVEL)]
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
        #[arg(short = 'l', long, default_value_t = COMPRESSION_LEVEL)]
        compression_level: i32,
        /// Skip compression
        #[arg(long, default_value_t = false)]
        skip_compression: bool,
    },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct App {
    #[command(subcommand)]
    command: Command,
}

fn main() {
    let App { command } = App::parse();

    match command {
        Command::Compress {
            input,
            output,
            compression_level: level,
        } => {
            let input_bytes = read(&input).expect("Failed to read input");
            let compressed_bytes = compress(&input_bytes, level).expect("Failed to compress file");

            write(&output, compressed_bytes).expect("Failed to write output");
        }
        Command::Decode { input, output } => {
            let file_name = input
                .file_name()
                .expect("Failed to get file name")
                .to_string_lossy();
            let mut file_parts = file_name.split('.').collect::<Vec<_>>();
            let mut extension = file_parts.pop().expect("Failed to get extension");
            let prefix = file_parts.first().cloned().expect("Failed to get prefix");

            let input_bytes = read(&input).expect("Failed to read input");
            let buffer: Vec<u8> = if extension == "zs" {
                extension = file_parts.pop().expect("Failed to get extension");
                zstd::decode_all(input_bytes.as_slice()).expect("Failed to decompress input")
            } else {
                input_bytes
            };

            let image = match extension {
                "canvas" => image_from_canvas(&buffer).expect("Failed to read canvas"),
                "ugctex" => {
                    if prefix.ends_with("Thumb") {
                        image_from_thumbnail(&buffer).expect("Failed to read ugctex")
                    } else {
                        image_from_texture(&buffer, prefix.contains("Food"))
                            .expect("Failed to read ugctex")
                    }
                }
                _ => panic!("Unsupported file type"),
            };

            image.save(output).expect("Failed to write output");
        }
        Command::Encode {
            input,
            output_dir,
            output_type,
            number,
            skip_compression,
            compression_level,
            resize_method: resize,
        } => {
            let image = image::open(&input).expect("Failed to read input");

            let file_stem = format!("{}{number:0>3}", output_type.file_name());
            if output_type.has_texture() {
                let size = if output_type == PaintType::Food {
                    FOOD_SIZE
                } else {
                    TEXTURE_SIZE
                };

                let resized = match resize {
                    ResizeType::Preserve => {
                        image.resize(size, size, image::imageops::FilterType::Nearest)
                    }
                    ResizeType::Fill => {
                        image.resize_to_fill(size, size, image::imageops::FilterType::Nearest)
                    }
                    ResizeType::Exact => {
                        image.resize_exact(size, size, image::imageops::FilterType::Nearest)
                    }
                };

                let bytes = texture_from_image(&resized, output_type == PaintType::Food)
                    .expect("Failed to encode texture");

                let mut texture_path = output_dir.clone();
                texture_path.push(&file_stem);

                if skip_compression {
                    texture_path.set_extension("ugctex");
                    let mut file =
                        std::fs::File::create(&texture_path).expect("Failed to create file");
                    file.write_all(&bytes).expect("Failed to write texture");

                    return;
                }

                let compressed = compress(&bytes, compression_level).expect("Failed to compress");

                texture_path.set_extension("ugctex.zs");

                let mut file = std::fs::File::create(&texture_path).expect("Failed to create file");
                file.write_all(&compressed)
                    .expect("Failed to write texture");
            }

            if output_type.has_canvas() {
                let resized = match resize {
                    ResizeType::Preserve => image.resize(
                        CANVAS_SIZE,
                        CANVAS_SIZE,
                        image::imageops::FilterType::Nearest,
                    ),
                    ResizeType::Fill => image.resize_to_fill(
                        CANVAS_SIZE,
                        CANVAS_SIZE,
                        image::imageops::FilterType::Nearest,
                    ),
                    ResizeType::Exact => image.resize_exact(
                        CANVAS_SIZE,
                        CANVAS_SIZE,
                        image::imageops::FilterType::Nearest,
                    ),
                };

                let bytes = canvas_from_image(&resized).expect("Failed to encode canvas");
                if skip_compression {
                    let mut canvas_path = output_dir.clone();
                    canvas_path.push(&file_stem);
                    canvas_path.set_extension("canvas");

                    let mut file =
                        std::fs::File::create(&canvas_path).expect("Failed to create file");
                    file.write_all(&bytes).expect("Failed to write canvas");
                    return;
                }

                let compressed = compress(&bytes, compression_level).expect("Failed to compress");

                let mut canvas_path = output_dir.clone();
                canvas_path.push(&file_stem);
                canvas_path.set_extension("canvas.zs");

                let mut file = std::fs::File::create(&canvas_path).expect("Failed to create file");
                file.write_all(&compressed).expect("Failed to write canvas");
            }

            if output_type.has_thumbnail() {
                let mut thumbnail_path = output_dir.clone();
                let mut stem = file_stem.clone();
                stem.push_str("_Thumb");

                thumbnail_path.push(stem);

                let resized = match resize {
                    ResizeType::Preserve => image.resize(
                        THUMBNAIL_SIZE,
                        THUMBNAIL_SIZE,
                        image::imageops::FilterType::Nearest,
                    ),
                    ResizeType::Fill => image.resize_to_fill(
                        THUMBNAIL_SIZE,
                        THUMBNAIL_SIZE,
                        image::imageops::FilterType::Nearest,
                    ),
                    ResizeType::Exact => image.resize_exact(
                        THUMBNAIL_SIZE,
                        THUMBNAIL_SIZE,
                        image::imageops::FilterType::Nearest,
                    ),
                };

                let bytes = thumbnail_from_image(&resized).expect("Failed to encode thumbnail");

                if skip_compression {
                    thumbnail_path.set_extension("ugctex");
                    let mut file =
                        std::fs::File::create(&thumbnail_path).expect("Failed to create file");
                    file.write_all(&bytes).expect("Failed to write thumbnail");
                    return;
                }

                let compressed = compress(&bytes, compression_level).expect("Failed to compress");

                thumbnail_path.set_extension("ugctex.zs");
                let mut file =
                    std::fs::File::create(&thumbnail_path).expect("Failed to create file");

                file.write_all(&compressed)
                    .expect("Failed to write thumbnail");
            }
        }
    }
}

fn compress(input: &[u8], level: i32) -> color_eyre::Result<Vec<u8>> {
    let mut encoder = zstd::Encoder::new(Vec::new(), level)?;

    encoder.set_pledged_src_size(Some(input.len() as u64))?;
    encoder.include_contentsize(true)?;

    encoder.write_all(input)?;

    Ok(encoder.finish()?)
}
