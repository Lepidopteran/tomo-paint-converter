use std::{
    fs::{read, write},
    io::Write,
    path::PathBuf,
};
use tomo_image_converter::*;

use clap::{Parser, Subcommand};

const COMPRESSION_LEVEL: i32 = 3;

/// Output type of texture
#[derive(clap::ValueEnum, Debug, Clone, Copy, Eq, PartialEq)]
pub enum PaintType {
    Food,
    FacePaint,
    Interior,
    Exterior,
    Good,
    Cloth,
}

impl PaintType {
    pub fn file_name(&self) -> &'static str {
        match self {
            PaintType::Food => "UgcFood",
            PaintType::FacePaint => "UgcFacePaint",
            PaintType::Interior => "UgcInterior",
            PaintType::Exterior => "UgcExterior",
            PaintType::Good => "UgcGoods",
            PaintType::Cloth => "UgcCloth",
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

#[derive(Subcommand, Debug)]
enum Command {
    /// Decode canvas or ugctex to image file.
    Decode {
        /// Input image e.g UgcFacePaint000.ugctex.zs
        #[arg(short, long)]
        input: PathBuf,
        /// Output image e.g mycat.png
        /// supports avif, bmp, exr, ff, gif, hdr, ico, jpeg, png, pnm, qoi, tga, tiff, and webp.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Compress file using zstd
    Compress {
        /// Input image
        #[arg(short, long)]
        input: PathBuf,
        /// Output image
        #[arg(short, long)]
        output: PathBuf,
        /// Compression level
        #[arg(short, long, default_value_t = COMPRESSION_LEVEL)]
        level: i32,
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
            level,
        } => {
            let input_bytes = read(&input).expect("Failed to read input");
            let compressed_bytes = zstd::encode_all(input_bytes.as_slice(), level).unwrap();
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
    }
}
