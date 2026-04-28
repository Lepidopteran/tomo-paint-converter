use std::{
    cell::RefCell,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
    thread,
};

use color_eyre::eyre::Result;
use image::GenericImageView;
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer};
use strum::{Display, EnumIter, EnumString};
use tomo_image_converter::{
    CANVAS_SIZE, FOOD_SIZE, TEXTURE_SIZE, THUMBNAIL_SIZE, Texture,
    texture::{
        TextureDecoder,
        codecs::bcn::{BcFormat, BcTextureDecoder, BcTextureEncoder},
        resize::{ResizeFilter, ResizeType},
    },
};

use super::*;

mod file_dialog;
use file_dialog::*;

type Rgba8Buffer = SharedPixelBuffer<Rgba8Pixel>;

#[derive(Debug, Clone, Copy, Display, EnumIter, EnumString)]
enum PreviewType {
    Source,
    Texture,
    Canvas,
    Thumbnail,
}

#[derive(Debug)]
struct Cache {
    bytes: Vec<u8>,
    resize_type: ResizeType,
    resize_filter: ResizeFilter,
}

#[derive(Debug)]
struct TextureCache {
    bytes: Vec<u8>,
    paint_type: PaintType,
    resize_type: ResizeType,
    resize_filter: ResizeFilter,
}

#[derive(Default, Debug)]
struct OutputCache {
    texture: Option<TextureCache>,
    canvas: Option<Cache>,
    thumbnail: Option<Cache>,
}

type StateHandle = Arc<State>;

#[derive(Default)]
struct State {
    input_texture: RwLock<Option<Texture>>,
    images: RwLock<OutputCache>,
}

impl State {
    // fn output_texture(
    //     &self,
    //     paint_type: PaintType,
    //     resize_type: ResizeType,
    //     resize_filter: ResizeFilter,
    // ) -> Vec<u8> {
    //     if let Some(cached) = self.images.texture.borrow().as_ref()
    //         && cached.resize_type == resize_type
    //         && cached.resize_filter == resize_filter
    //         && cached.paint_type == paint_type
    //     {
    //         return cached.bytes.to_vec();
    //     }
    //
    //     let nsize = if paint_type == PaintType::Food {
    //         FOOD_SIZE
    //     } else {
    //         TEXTURE_SIZE
    //     };
    //
    //     let bytes = self
    //         .input_texture
    //         .borrow()
    //         .as_ref()
    //         .expect("No input texture")
    //         .resize(nsize, nsize, resize_type, resize_filter)
    //         .encode(BcTextureEncoder::new(BcFormat::Bc1))
    //         .expect("Failed to encode texture");
    //
    //     self.images.texture.replace(
    //         TextureCache {
    //             bytes: bytes.clone(),
    //             paint_type,
    //             resize_type,
    //             resize_filter,
    //         }
    //         .into(),
    //     );
    //
    //     bytes
    // }
    //
    // fn output_canvas(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> Vec<u8> {
    //     if let Some(cached) = self.images.canvas.borrow().as_ref()
    //         && cached.resize_type == resize_type
    //         && cached.resize_filter == resize_filter
    //     {
    //         return cached.bytes.to_vec();
    //     }
    //
    //     let bytes = self
    //         .input_texture
    //         .borrow()
    //         .as_ref()
    //         .expect("No input texture")
    //         .resize(CANVAS_SIZE, CANVAS_SIZE, resize_type, resize_filter)
    //         .into_bytes();
    //
    //     self.images.canvas.replace(
    //         Cache {
    //             bytes: bytes.clone(),
    //             resize_type,
    //             resize_filter,
    //         }
    //         .into(),
    //     );
    //
    //     bytes
    // }
    //
    // fn output_thumbnail(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> Vec<u8> {
    //     if let Some(cached) = self.images.thumbnail.borrow().as_ref()
    //         && cached.resize_type == resize_type
    //         && cached.resize_filter == resize_filter
    //     {
    //         return cached.bytes.to_vec();
    //     }
    //
    //     let bytes = self
    //         .input_texture
    //         .borrow()
    //         .as_ref()
    //         .expect("No input texture")
    //         .resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, resize_type, resize_filter)
    //         .encode(BcTextureEncoder::new(BcFormat::Bc3))
    //         .expect("Failed to encode texture");
    //
    //     self.images.thumbnail.replace(
    //         Cache {
    //             bytes: bytes.clone(),
    //             resize_type,
    //             resize_filter,
    //         }
    //         .into(),
    //     );
    //
    //     bytes
    // }
}

pub fn setup(app: &AppWindow) -> Result<()> {
    let state = Arc::new(State::default());

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
    let state_ref = state.clone();
    app.on_update_preview(move || {
        handle_preview_update(
            app_ref.upgrade().expect("Couldn't get app"),
            state_ref.clone(),
        );
    });

    let app_ref = app.as_weak();
    let state_ref = state.clone();
    app.on_export_button_clicked(move || {
        let app_ref = app_ref.clone();
        let state = state_ref.clone();
    });

    app.set_texture_type_model(ModelRc::new(PaintType::model()));
    app.set_resize_filter_model(ModelRc::new(ResizeFilter::model()));
    app.set_resize_method_model(ModelRc::new(ResizeType::model()));
    app.set_viewer_mode_model(ModelRc::new(PreviewType::model()));

    Ok(())
}

async fn handle_file_input(app: AppWindow, state: StateHandle) {
    app.set_file_dialog_opened(true);
    let file = FileDialog::new()
        .with_title("Select file to convert")
        .set_parent(&app.window().window_handle())
        .add_supported_formats()
        .pick_file()
        .await;

    if let Some(file) = file {
        let path = file.path();

        app.set_input_path(path.to_string_lossy().to_string().into());
        app.set_loading(true);

        let path = path.to_path_buf();
        let app_ref = app.as_weak();
        thread::spawn(move || {
            if let Ok(texture) = loading::open_file(path) {
                state
                    .input_texture
                    .write()
                    .expect("Failed to get input texture")
                    .replace(texture);

                app_ref
                    .upgrade_in_event_loop(move |handle| {
                        handle.set_image_loaded(true);
                        handle.invoke_update_preview();
                        handle.set_loading(false);
                    })
                    .expect("Couldn't get app");

                let mut images = state.images.write().expect("Failed to get images");
                images.canvas.take();
                images.thumbnail.take();
                images.texture.take();
            };
        });
    }

    app.set_file_dialog_opened(false);
}

fn handle_preview_update(app: AppWindow, state: StateHandle) {
    let preview_type = PreviewType::from_str(app.get_viewer_mode().as_str()).expect("Invalid type");
    let paint_type = PaintType::from_str(app.get_texture_type().as_str()).expect("Invalid type");
    let resize_type = ResizeType::from_str(app.get_resize_method().as_str()).expect("Invalid type");
    let resize_filter =
        ResizeFilter::from_str(app.get_resize_filter().as_str()).expect("Invalid type");

    tracing::debug!(
        "Updating preview with type: {:?}, paint_type: {:?}, resize_type: {:?}, resize_filter: {:?}",
        preview_type,
        paint_type,
        resize_type,
        resize_filter
    );

    let (width, height) = match preview_type {
        PreviewType::Canvas => (CANVAS_SIZE, CANVAS_SIZE),
        PreviewType::Thumbnail => (THUMBNAIL_SIZE, THUMBNAIL_SIZE),
        PreviewType::Texture => {
            if paint_type == PaintType::Food {
                (FOOD_SIZE, FOOD_SIZE)
            } else {
                (TEXTURE_SIZE, TEXTURE_SIZE)
            }
        }

        PreviewType::Source => state
            .input_texture
            .read()
            .expect("Couldn't read state")
            .as_ref()
            .expect("No input texture")
            .as_image()
            .dimensions(),
    };

    let bytes = match preview_type {
        PreviewType::Source => state
            .input_texture
            .read()
            .expect("Couldn't read state")
            .as_ref()
            .expect("No input texture")
            .as_bytes()
            .clone(),
        _ => vec![0; (width * height * 4) as usize],
    };

    let buffer = Rgba8Buffer::clone_from_slice(&bytes, width, height);

    app.set_viewer_image(Image::from_rgba8(buffer));
}
