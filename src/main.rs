use std::{fs::read, path::PathBuf};
use tomo_image_converter::{image_from_canvas, image_from_ugctex, image_from_ugctex_thumb};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
}

fn main() {
    let Args { input, output } = Args::parse();

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
                image_from_ugctex_thumb(&buffer).expect("Failed to read ugctex")
            } else {
                image_from_ugctex(&buffer, prefix.contains("Food")).expect("Failed to read ugctex")
            }
        }
        _ => panic!("Unsupported file type"),
    };

    image.save(output).expect("Failed to write output");
}
