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

#[derive(Debug, Clone, Copy, Display, EnumIter, EnumString)]
enum PreviewType {
    Source,
    Texture,
    Canvas,
    Thumbnail,
}

type Rgba8Buffer = SharedPixelBuffer<Rgba8Pixel>;
type ImageBuffer = RefCell<Option<Rgba8Buffer>>;

#[derive(Default)]
struct ImageData {
    source: ImageBuffer,
    texture: ImageBuffer,
    canvas: ImageBuffer,
    thumbnail: ImageBuffer,
}

type StateHandle = Rc<State>;

#[derive(Default)]
struct State {
    // NOTE: Reduces the amount of resizing.
    proxy_texture: RefCell<Option<Texture>>,
    images: ImageData,
}

pub fn setup(app: &AppWindow) -> Result<()> {
    let state = Rc::new(State::default());

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

    app.set_texture_type_model(ModelRc::new(PaintType::model()));
    app.set_resize_filter_model(ModelRc::new(ResizeFilter::model()));
    app.set_resize_method_model(ModelRc::new(ResizeType::model()));
    app.set_viewer_mode_model(ModelRc::new(PreviewType::model()));

    Ok(())
}

async fn handle_output_folder(app: AppWindow) {
    app.set_file_dialog_opened(true);
    let file = FileDialog::new()
        .with_title("Select output folder")
        .pick_folder()
        .await;

    if let Some(file) = file {
        app.set_output_folder_path(file.path().to_string_lossy().to_string().into());
    }
}

async fn handle_file_input(app: AppWindow, state: StateHandle) {
    app.set_file_dialog_opened(true);
    let file = FileDialog::new()
        .with_title("Select file to convert")
        .add_supported_formats()
        .pick_file()
        .await;

    if let Some(file) = file {
        let path = file.path();

        app.set_input_path(path.to_string_lossy().to_string().into());

        let texture = open_file(path).expect("Failed to open texture or image");
        let img = texture.as_image();
        let buffer =
            Rgba8Buffer::clone_from_slice(img.to_rgba8().as_raw(), img.width(), img.height());

        state.proxy_texture.borrow_mut().replace(texture);
        state.images.source.borrow_mut().replace(buffer.clone());

        app.set_viewer_image(Image::from_rgba8(buffer));
        app.set_image_loaded(true);
    }

    app.set_file_dialog_opened(false);
}
