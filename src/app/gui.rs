use std::{cell::RefCell, rc::Rc};

use color_eyre::eyre::Result;
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer};
use strum::{Display, EnumIter, EnumString};
use tomo_image_converter::{
    Texture,
    texture::resize::{ResizeFilter, ResizeType},
};

use super::*;

mod file_dialog;
use file_dialog::*;

slint::include_modules!();

#[derive(Debug, Clone, Copy, Display, EnumIter, EnumString)]
enum PreviewType {
    Source,
    Texture,
    Canvas,
    Thumbnail,
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
        .set_parent(&app.window().window_handle())
        .add_filter("Supported formats", ALL_SUPPORTED_FORMATS)
        .add_filter("Images", SUPPORTED_IMAGE_FORMATS)
        .add_filter("Textures", SUPPORTED_TEXTURE_FORMATS)
        .pick_file()
        .await;

    if let Some(file) = file {
        let path = file.path();

        app.set_input_path(path.to_string_lossy().to_string().into());

        let texture = open_file(path).expect("Failed to open texture or image");
        let img = texture.as_image();

        let buffer =
            Rgba8Buffer::clone_from_slice(img.to_rgba8().as_raw(), img.width(), img.height());

        app.set_viewer_image(Image::from_rgba8(buffer.clone()));
        state.borrow_mut().cache.source.replace(buffer);

        app.set_file_dialog_opened(false);
        app.set_image_loaded(true);
    }
}
