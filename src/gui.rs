use std::{cell::RefCell, io::Cursor, rc::Rc};

use color_eyre::eyre::Result;
use image::ImageReader;
use rfd::AsyncFileDialog;
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use tomo_image_converter::{
    PaintType, image_from_canvas, image_from_texture, image_from_thumbnail,
};

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
            handle_file_input(weak_app.upgrade().expect("Couldn't get app"), state).await;
        })
        .unwrap();
    });

    Ok(app.run()?)
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
                    image_from_thumbnail(data).expect("Failed to read ugctex thumbnail")
                } else {
                    image_from_texture(data, file_name.contains("Food"))
                        .expect("Failed to read ugctex")
                };

                let buffer = Rgba8Buffer::clone_from_slice(
                    img.to_rgba8().as_raw(),
                    img.width(),
                    img.height(),
                );

                app.set_viewer_image(Image::from_rgba8(buffer.clone()));
                state.borrow_mut().cache.source.replace(buffer);
            } else if file_name.contains(".canvas") {
                let img = image_from_canvas(data).expect("Failed to read canvas");

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
    AsyncFileDialog::new()
        .set_title(title)
        .add_filter("All Files", &["*"])
}
