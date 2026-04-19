use std::{cell::RefCell, io::Cursor, rc::Rc};

use color_eyre::eyre::Result;
use image::{GenericImageView, ImageReader};
use rfd::AsyncFileDialog;
use slint::{Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use tomo_image_converter::PaintType;

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
    cache: ImageDataCache,
}

pub fn run() -> Result<()> {
    let app = AppWindow::new()?;
    let state = Rc::new(RefCell::new(State::default()));

    app.set_texture_type_model(ModelRc::new(VecModel::from(
        PaintType::variants()
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<_>>(),
    )));

    let weak_app = app.as_weak();
    let state_ref = state.clone();
    app.on_pick_file_input(move || {
        let weak_app = weak_app.clone();
        let state = state_ref.clone();
        slint::spawn_local(async move {
            let app = weak_app.upgrade().unwrap();
            app.set_file_dialog_opened(true);
            let file = file_dialog("Pick a file to import")
                .add_filter("Supported formats", ALL_FORMATS)
                .add_filter("Images", IMAGE_FORMATS)
                .add_filter("Textures", TEXTURE_FORMATS)
                .pick_file()
                .await;

            if let Some(file) = file
                && let Some(app) = weak_app.upgrade()
            {
                let path = file.path();
                let bytes = file.read().await;
                let file_name = path
                    .file_name()
                    .expect("Path doesn't contain a file name")
                    .to_string_lossy()
                    .to_string();

                app.set_input_path(file.path().to_string_lossy().to_string().into());

                state.borrow_mut().source_bytes = bytes.clone();

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
                }

                app.set_file_dialog_opened(false);
                app.set_image_loaded(true);
            }
        })
        .unwrap();
    });

    Ok(app.run()?)
}

fn file_dialog(title: &str) -> AsyncFileDialog {
    AsyncFileDialog::new()
        .set_title(title)
        .add_filter("All Files", &["*"])
}
