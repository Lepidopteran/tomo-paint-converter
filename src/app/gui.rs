use std::{cell::RefCell, fmt::Display, io::Cursor, rc::Rc, str::FromStr};

use color_eyre::eyre::Result;
use image::ImageReader;
use rfd::AsyncFileDialog;
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use tomo_image_converter::{
    CANVAS_SIZE, FOOD_SIZE, TEXTURE_SIZE, THUMBNAIL_SIZE, Texture,
    texture::{
        codecs::bcn::{BcFormat, BcTextureDecoder},
        resize::{ResizeFilter, ResizeType},
        tegra::{TegraTextureDecoder, deswizzle_uncompressed_bytes},
    },
};

use crate::app::{PaintType, VecEnumModel};

const ALL_FORMATS: &[&str] = &[
    "avif",
    "bmp",
    "dds",
    "exr",
    "ff",
    "gif",
    "hdr",
    "ico",
    "jpeg",
    "png",
    "pnm",
    "qoi",
    "tga",
    "tiff",
    "webp",
    "canvas",
    "ugctex",
    "ugctex.zs",
    "canvas.zs",
];

const IMAGE_FORMATS: &[&str] = &[
    "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpeg", "png", "pnm", "qoi", "tga",
    "tiff", "webp",
];

const TEXTURE_FORMATS: &[&str] = &["canvas", "ugctex", "ugctex.zs", "canvas.zs"];

slint::include_modules!();

#[derive(Debug, Clone, Copy)]
enum PreviewType {
    Source,
    Texture,
    Canvas,
    Thumbnail,
}

impl PreviewType {
    fn model() -> VecModel<SharedString> {
        VecModel::from(vec![
            PreviewType::Texture.to_string().into(),
            PreviewType::Canvas.to_string().into(),
            PreviewType::Thumbnail.to_string().into(),
            PreviewType::Source.to_string().into(),
        ])
    }
}

impl Display for PreviewType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PreviewType::Source => "Source",
            PreviewType::Texture => "Texture",
            PreviewType::Canvas => "Canvas",
            PreviewType::Thumbnail => "Thumbnail",
        })
    }
}

impl FromStr for PreviewType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(" ", "").as_str() {
            "source" => Ok(PreviewType::Source),
            "texture" => Ok(PreviewType::Texture),
            "canvas" => Ok(PreviewType::Canvas),
            "thumbnail" => Ok(PreviewType::Thumbnail),
            _ => Err(format!("Invalid preview type: {}", s)),
        }
    }
}

type Rgba8Buffer = SharedPixelBuffer<Rgba8Pixel>;

#[derive(Default)]
struct ImageDataCache {
    source: Option<Rgba8Buffer>,
    texture: Option<Rgba8Buffer>,
    canvas: Option<Rgba8Buffer>,
    thumbnail: Option<Rgba8Buffer>,
}

#[derive(Default)]
struct State {
    source_bytes: Vec<u8>,
    // NOTE: Reduces the amount of resizing.
    proxy_texture: Option<Texture>,
    cache: ImageDataCache,
}

pub fn run() -> Result<()> {
    let app = AppWindow::new()?;
    let state = Rc::new(RefCell::new(State::default()));
    app.set_texture_type_model(ModelRc::new(PaintType::model()));
    app.set_resize_filter_model(ModelRc::new(ResizeFilter::model()));
    app.set_resize_method_model(ModelRc::new(ResizeType::model()));
    app.set_viewer_mode_model(ModelRc::new(PreviewType::model()));

    let app_ref = app.as_weak();
    let state_ref = state.clone();

    app.on_pick_file_input(move || {
        let app_ref = app_ref.clone();
        let state = state_ref.clone();
        slint::spawn_local(async move {
            handle_file_input(app_ref.upgrade().expect("Couldn't get app"), state).await;
        })
        .unwrap();
    });

    let app_ref = app.as_weak();

    app.on_pick_folder_output(move || {
        let weak_app = app_ref.clone();
        slint::spawn_local(async move {
            handle_output_folder(weak_app.upgrade().expect("Couldn't get app")).await;
        })
        .unwrap();
    });

    Ok(app.run()?)
}

async fn handle_output_folder(app: AppWindow) {
    app.set_file_dialog_opened(true);
    let file = file_dialog("Pick a folder").pick_folder().await;

    if let Some(file) = file {
        app.set_output_folder_path(file.path().to_string_lossy().to_string().into());
    }
}

async fn handle_file_input(app: AppWindow, state: Rc<RefCell<State>>) {
    app.set_file_dialog_opened(true);
    let file = file_dialog("Pick a file to import")
        .add_filter("Supported formats", ALL_FORMATS)
        .add_filter("Images", IMAGE_FORMATS)
        .add_filter("Textures", TEXTURE_FORMATS)
        .pick_file()
        .await;

    if let Some(file) = file {
        let path = file.path();
        let bytes = file.read().await;
        let file_name = path
            .file_name()
            .expect("Path doesn't contain a file name")
            .to_string_lossy()
            .to_string();

        app.set_input_path(file.path().to_string_lossy().to_string().into());

        if IMAGE_FORMATS
            .iter()
            .any(|file_type| file_name.ends_with(&format!(".{file_type}")))
        {
            let input_image = ImageReader::new(Cursor::new(&bytes))
                .with_guessed_format()
                .expect("Failed to guess image format")
                .decode()
                .expect("Failed to decode image");
            let rgba = input_image.to_rgba8();

            let buffer = Rgba8Buffer::clone_from_slice(&rgba, rgba.width(), rgba.height());
            app.set_viewer_image(Image::from_rgba8(buffer.clone()));
            state.borrow_mut().cache.source.replace(buffer);
        } else if TEXTURE_FORMATS
            .iter()
            .any(|file_type| file_name.ends_with(&format!(".{file_type}")))
        {
            let decompressed;
            let data = if file_name.ends_with(".zs") {
                decompressed = zstd::decode_all(&bytes[..]).expect("Failed to decompress");
                &decompressed
            } else {
                &bytes
            };

            if file_name.contains(".ugctex") {
                let img = if file_name.contains("Thumb") {
                    let decoder = BcTextureDecoder::new(BcFormat::Bc3);

                    Texture::from_decoder(
                        data.to_vec(),
                        TegraTextureDecoder::new(decoder),
                        THUMBNAIL_SIZE,
                        THUMBNAIL_SIZE,
                    )
                    .into_image()
                } else {
                    let size = if file_name.contains("Texture") {
                        TEXTURE_SIZE
                    } else {
                        FOOD_SIZE
                    };

                    let decoder = BcTextureDecoder::new(BcFormat::Bc1);

                    Texture::from_decoder(
                        data.to_vec(),
                        TegraTextureDecoder::new(decoder),
                        size,
                        size,
                    )
                    .into_image()
                };

                let buffer = Rgba8Buffer::clone_from_slice(
                    img.to_rgba8().as_raw(),
                    img.width(),
                    img.height(),
                );

                app.set_viewer_image(Image::from_rgba8(buffer.clone()));
                state.borrow_mut().cache.source.replace(buffer);
            } else if file_name.contains(".canvas") {
                let img = Texture::from_bytes(
                    deswizzle_uncompressed_bytes(CANVAS_SIZE, CANVAS_SIZE, data)
                        .expect("Failed to deswizzle"),
                    CANVAS_SIZE,
                    CANVAS_SIZE,
                )
                .into_image();

                let buffer = Rgba8Buffer::clone_from_slice(
                    img.to_rgba8().as_raw(),
                    img.width(),
                    img.height(),
                );

                app.set_viewer_image(Image::from_rgba8(buffer.clone()));
                state.borrow_mut().cache.source.replace(buffer);
            }
        };

        state.borrow_mut().source_bytes = bytes.clone();

        app.set_file_dialog_opened(false);
        app.set_image_loaded(true);
    }
}

fn file_dialog(title: &str) -> AsyncFileDialog {
    let mut file_dialog = AsyncFileDialog::new().set_title(title);

    if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        file_dialog = file_dialog.add_filter("All files", &["*"]);
    }

    file_dialog
}
