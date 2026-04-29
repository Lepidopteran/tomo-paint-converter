use std::{
    cell::RefCell,
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
    thread,
};

use color_eyre::eyre::Result;
use image::{DynamicImage, GenericImageView, RgbaImage};
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

mod cache;
use cache::*;

type Rgba8Buffer = SharedPixelBuffer<Rgba8Pixel>;

#[derive(Debug, Clone, Copy, Display, EnumIter, EnumString, Eq, PartialEq)]
enum PreviewType {
    Source,
    Texture,
    Canvas,
    Thumbnail,
}

impl From<&AppWindow> for PreviewType {
    fn from(value: &AppWindow) -> Self {
        PreviewType::from_str(&value.get_viewer_mode().as_str()).expect("Invalid PreviewType")
    }
}

type StateHandle = Arc<State>;

#[derive(Default)]
struct State {
    source_image: RwLock<Option<RgbaImage>>,
    texture: RwLock<Option<TextureCache>>,
    cache: RwLock<Cache>,
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
        let resize_type = ResizeType::from(&app);
        let resize_filter = ResizeFilter::from(&app);
        let app_ref = app.as_weak();
        thread::spawn(move || {
            if let Ok(texture) = loading::open_file(path) {
                state
                    .texture
                    .write()
                    .expect("Failed to get input texture")
                    .replace(TextureCache::new(
                        texture
                            .resize(TEXTURE_SIZE, TEXTURE_SIZE, resize_type, resize_filter)
                            .into_bytes(),
                        resize_type,
                        resize_filter,
                    ));

                state
                    .source_image
                    .write()
                    .expect("Failed to get source image")
                    .replace(texture.into_image().into_rgba8());

                app_ref
                    .upgrade_in_event_loop(move |handle| {
                        handle.set_image_loaded(true);
                        handle.invoke_update_preview();
                        handle.set_loading(false);
                    })
                    .expect("Couldn't get app");

                let mut cache = state.cache.write().expect("Failed to get cache");
                cache.canvas.take();
                cache.thumbnail.take();
                cache.texture.take();
            };
        });
    }

    app.set_file_dialog_opened(false);
}

fn handle_preview_update(app: AppWindow, state: StateHandle) {
    let preview_type = PreviewType::from(&app);
    let paint_type = PaintType::from(&app);
    let resize_type = ResizeType::from(&app);
    let resize_filter = ResizeFilter::from(&app);

    tracing::debug!(
        "Updating preview with type: {:?}, paint_type: {:?}, resize_type: {:?}, resize_filter: {:?}",
        preview_type,
        paint_type,
        resize_type,
        resize_filter
    );

    if preview_type == PreviewType::Source {
        let source = state.source_image.read().expect("Couldn't read image");

        let (image, width, height) = {
            let image = source.as_ref().expect("No source image");

            (image.as_raw(), image.width(), image.height())
        };

        app.set_viewer_image(Image::from_rgba8(Rgba8Buffer::clone_from_slice(
            image, width, height,
        )));

        return;
    }

    let state_cache = state.cache.read().expect("Couldn't read cache");
    let cache: Option<CachedTexture> = match preview_type {
        PreviewType::Texture => state_cache.get(CacheKey::Texture),
        PreviewType::Canvas => state_cache.get(CacheKey::Canvas),
        PreviewType::Thumbnail => state_cache.get(CacheKey::Thumbnail),
        PreviewType::Source => unreachable!("Source should be handled already"),
    };

    let size = match preview_type {
        PreviewType::Texture => {
            if paint_type == PaintType::Food {
                FOOD_SIZE
            } else {
                TEXTURE_SIZE
            }
        }
        PreviewType::Canvas => CANVAS_SIZE,
        PreviewType::Thumbnail => THUMBNAIL_SIZE,
        PreviewType::Source => unreachable!("Source should be handled already"),
    };

    if let Some(cache) = cache
        && !match cache {
            CachedTexture::UgcTexture(texture) => {
                texture.is_invalid(paint_type, resize_type, resize_filter)
            }
            CachedTexture::Texture(texture) => texture.is_invalid(resize_type, resize_filter),
        }
    {
        let bytes = match preview_type {
            PreviewType::Texture => &BcTextureDecoder::new(BcFormat::Bc1)
                .decode_bytes(cache.as_bytes(), size, size)
                .expect("Failed to decode texture"),
            PreviewType::Thumbnail => &BcTextureDecoder::new(BcFormat::Bc3)
                .decode_bytes(cache.as_bytes(), size, size)
                .expect("Failed to decode thumbnail"),
            _ => cache.as_bytes(),
        };

        let buffer = Rgba8Buffer::clone_from_slice(bytes, size, size);
        app.set_viewer_image(Image::from_rgba8(buffer));

        return;
    }

    drop(state_cache);

    app.set_processing_image(true);
    let state = state.clone();
    let app_ref = app.as_weak();
    thread::spawn(move || {
        let texture = if state
            .texture
            .read()
            .expect("Couldn't read input texture")
            .as_ref()
            .expect("No input texture")
            .is_invalid(resize_type, resize_filter)
        {
            tracing::debug!("Invalidating resized texture");

            let image = DynamicImage::ImageRgba8(
                state
                    .source_image
                    .read()
                    .expect("Couldn't read source image")
                    .clone()
                    .expect("No source image"),
            );

            let texture = Texture::from_image(image).resize(
                TEXTURE_SIZE,
                TEXTURE_SIZE,
                resize_type,
                resize_filter,
            );

            state
                .texture
                .write()
                .expect("Couldn't write texture")
                .replace(TextureCache::new(
                    texture.as_bytes().clone(),
                    resize_type,
                    resize_filter,
                ));

            texture
        } else {
            Texture::from_bytes(
                state
                    .texture
                    .read()
                    .expect("Couldn't read texture")
                    .clone()
                    .expect("No texture")
                    .into_bytes(),
                TEXTURE_SIZE,
                TEXTURE_SIZE,
            )
            .expect("Failed to decode texture")
        };

        let bytes = match preview_type {
            PreviewType::Canvas => texture
                .resize(size, size, resize_type, resize_filter)
                .into_bytes(),
            PreviewType::Thumbnail => {
                let encoder = BcTextureEncoder::new(BcFormat::Bc3);
                texture
                    .resize(size, size, resize_type, resize_filter)
                    .encode(encoder)
                    .expect("Failed to encode thumbnail")
            }
            PreviewType::Texture => {
                let encoder = BcTextureEncoder::new(BcFormat::Bc1);
                if size == TEXTURE_SIZE {
                    texture.encode(encoder).expect("Failed to encode texture")
                } else {
                    texture
                        .resize(size, size, resize_type, resize_filter)
                        .encode(encoder)
                        .expect("Failed to encode texture")
                }
            }
            PreviewType::Source => unreachable!("Source should be handled already"),
        };
        let mut state_cache = state.cache.write().expect("Couldn't read cache");

        match preview_type {
            PreviewType::Texture => {
                state_cache.texture.replace(UgcTextureCache::new(
                    bytes.to_vec(),
                    paint_type,
                    resize_type,
                    resize_filter,
                ));
            }
            PreviewType::Canvas => {
                state_cache.canvas.replace(TextureCache::new(
                    bytes.to_vec(),
                    resize_type,
                    resize_filter,
                ));
            }
            PreviewType::Thumbnail => {
                state_cache.thumbnail.replace(TextureCache::new(
                    bytes.to_vec(),
                    resize_type,
                    resize_filter,
                ));
            }
            PreviewType::Source => unreachable!("Source should be handled already"),
        };

        let decoded_bytes = match preview_type {
            PreviewType::Texture => BcTextureDecoder::new(BcFormat::Bc1)
                .decode_bytes(&bytes, size, size)
                .expect("Failed to decode texture"),
            PreviewType::Thumbnail => BcTextureDecoder::new(BcFormat::Bc3)
                .decode_bytes(&bytes, size, size)
                .expect("Failed to decode thumbnail"),
            _ => bytes,
        };

        let buffer = Rgba8Buffer::clone_from_slice(&decoded_bytes, size, size);
        app_ref
            .upgrade_in_event_loop(move |handle| {
                handle.set_viewer_image(Image::from_rgba8(buffer));
                handle.set_processing_image(false);
            })
            .expect("Couldn't get app");
    });
}
