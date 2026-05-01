use std::{
    fs::File,
    io::Write,
    ops::Not,
    str::FromStr,
    sync::{Arc, RwLock},
    thread,
};

use color_eyre::eyre::Result;
use image::{DynamicImage, RgbaImage};
use slint::{Image, ModelRc, Rgba8Pixel, SharedPixelBuffer};
use strum::{Display, EnumIter, EnumString};
use tomo_image_converter::{
    CANVAS_SIZE, FOOD_SIZE, TEXTURE_SIZE, THUMBNAIL_SIZE, Texture,
    texture::{
        TextureDecoder,
        codecs::bcn::{BcFormat, BcTextureDecoder, BcTextureEncoder},
        resize::{ResizeFilter, ResizeType},
        tegra::{TegraSwizzle, swizzle_uncompressed_bytes},
    },
    zstd_compress_bytes,
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
        PreviewType::from_str(value.get_viewer_mode().as_str()).expect("Invalid PreviewType")
    }
}

type StateHandle = Arc<State>;

#[derive(Default)]
struct State {
    source_image: RwLock<Option<RgbaImage>>,
    texture: RwLock<Option<TextureCache>>,
    cache: RwLock<Cache>,
}

impl State {
    fn resized_source(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> Texture {
        if let Some(texture) = self
            .texture
            .read()
            .expect("Couldn't read input texture")
            .as_ref()
            .and_then(|texture| {
                texture
                    .is_valid(resize_type, resize_filter)
                    .then_some(texture.as_bytes())
            })
        {
            return Texture::from_bytes(texture.to_vec(), TEXTURE_SIZE, TEXTURE_SIZE)
                .expect("Failed to decode texture");
        }

        tracing::debug!("Invalidating resized texture");

        let image = DynamicImage::ImageRgba8(
            self.source_image
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

        self.texture
            .write()
            .expect("Couldn't write texture")
            .replace(TextureCache::new(
                texture.as_bytes().clone(),
                resize_type,
                resize_filter,
            ));

        texture
    }

    fn canvas(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> Vec<u8> {
        if let Some(bytes) = self
            .cache
            .read()
            .expect("Couldn't read cache")
            .canvas
            .as_ref()
            .and_then(|c| {
                c.is_valid(resize_type, resize_filter)
                    .then_some(c.as_bytes())
            })
        {
            return bytes.to_vec();
        }

        tracing::debug!("Invalidating canvas");

        let texture = self.resized_source(resize_type, resize_filter).resize(
            CANVAS_SIZE,
            CANVAS_SIZE,
            resize_type,
            resize_filter,
        );

        self.cache
            .write()
            .expect("Couldn't write cache")
            .canvas
            .replace(TextureCache::new(
                texture.as_bytes().clone(),
                resize_type,
                resize_filter,
            ));

        texture.into_bytes()
    }

    fn encoded_thumbnail(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> Vec<u8> {
        if let Some(bytes) = self
            .cache
            .read()
            .expect("Couldn't read cache")
            .thumbnail
            .as_ref()
            .and_then(|c| {
                c.is_valid(resize_type, resize_filter)
                    .then_some(c.as_bytes())
            })
        {
            return bytes.to_vec();
        }

        tracing::debug!("Invalidating thumbnail");

        let encoded_bytes = self
            .resized_source(resize_type, resize_filter)
            .resize(THUMBNAIL_SIZE, THUMBNAIL_SIZE, resize_type, resize_filter)
            .encode(BcTextureEncoder::new(BcFormat::Bc3))
            .expect("Failed to encode thumbnail");

        self.cache
            .write()
            .expect("Couldn't write cache")
            .thumbnail
            .replace(TextureCache::new(
                encoded_bytes.to_vec(),
                resize_type,
                resize_filter,
            ));

        encoded_bytes
    }

    fn decoded_thumbnail(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> Vec<u8> {
        BcTextureDecoder::new(BcFormat::Bc3)
            .decode_bytes(
                &self.encoded_thumbnail(resize_type, resize_filter),
                THUMBNAIL_SIZE,
                THUMBNAIL_SIZE,
            )
            .expect("Failed to decode thumbnail")
    }

    fn encoded_texture(
        &self,
        paint_type: PaintType,
        resize_type: ResizeType,
        resize_filter: ResizeFilter,
    ) -> Vec<u8> {
        if let Some(bytes) = self
            .cache
            .read()
            .expect("Couldn't read cache")
            .texture
            .as_ref()
            .and_then(|c| {
                c.is_valid(paint_type, resize_type, resize_filter)
                    .then_some(c.as_bytes())
            })
        {
            return bytes.to_vec();
        }

        tracing::debug!("Invalidating texture");
        let texture = if paint_type == PaintType::Food {
            self.resized_source(resize_type, resize_filter).resize(
                FOOD_SIZE,
                FOOD_SIZE,
                resize_type,
                resize_filter,
            )
        } else {
            self.resized_source(resize_type, resize_filter)
        };

        let encoded_bytes = texture
            .encode(BcTextureEncoder::new(BcFormat::Bc1))
            .expect("Failed to encode texture");

        self.cache
            .write()
            .expect("Couldn't write cache")
            .texture
            .replace(UgcTextureCache::new(
                encoded_bytes.clone(),
                paint_type,
                resize_type,
                resize_filter,
            ));

        encoded_bytes
    }

    fn decoded_texture(
        &self,
        paint_type: PaintType,
        resize_type: ResizeType,
        resize_filter: ResizeFilter,
    ) -> Vec<u8> {
        let size = if paint_type == PaintType::Food {
            FOOD_SIZE
        } else {
            TEXTURE_SIZE
        };

        BcTextureDecoder::new(BcFormat::Bc1)
            .decode_bytes(
                &self.encoded_texture(paint_type, resize_type, resize_filter),
                size,
                size,
            )
            .expect("Failed to decode texture")
    }
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
        handle_export_button_clicked(
            app_ref.upgrade().expect("Couldn't get app"),
            state_ref.clone(),
        );
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
    let cache: Option<&[u8]> = match preview_type {
        PreviewType::Texture => state_cache.texture.as_ref().and_then(|texture| {
            texture
                .is_valid(paint_type, resize_type, resize_filter)
                .then_some(texture.as_bytes())
        }),
        PreviewType::Canvas => state_cache.canvas.as_ref().and_then(|texture| {
            texture
                .is_valid(resize_type, resize_filter)
                .then_some(texture.as_bytes())
        }),
        PreviewType::Thumbnail => state_cache.thumbnail.as_ref().and_then(|texture| {
            texture
                .is_valid(resize_type, resize_filter)
                .then_some(texture.as_bytes())
        }),
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

    if let Some(cache) = cache {
        let bytes = match preview_type {
            PreviewType::Texture => &BcTextureDecoder::new(BcFormat::Bc1)
                .decode_bytes(cache, size, size)
                .expect("Failed to decode texture"),
            PreviewType::Thumbnail => &BcTextureDecoder::new(BcFormat::Bc3)
                .decode_bytes(cache, size, size)
                .expect("Failed to decode thumbnail"),
            _ => cache,
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
        let bytes = match preview_type {
            PreviewType::Canvas => state.canvas(resize_type, resize_filter),
            PreviewType::Thumbnail => state.decoded_thumbnail(resize_type, resize_filter),
            PreviewType::Texture => state.decoded_texture(paint_type, resize_type, resize_filter),
            PreviewType::Source => unreachable!("Source should be handled already"),
        };

        let buffer = Rgba8Buffer::clone_from_slice(&bytes, size, size);
        app_ref
            .upgrade_in_event_loop(move |handle| {
                handle.set_viewer_image(Image::from_rgba8(buffer));
                handle.set_processing_image(false);
            })
            .expect("Couldn't get app");
    });
}

fn handle_export_button_clicked(app: AppWindow, state: StateHandle) {
    app.set_file_dialog_opened(true);

    let (tx, rx) = std::sync::mpsc::sync_channel(1);

    let app_ref = app.as_weak();
    slint::spawn_local(async move {
        let app = app_ref.upgrade().expect("Couldn't get app");
        let window_handle = app.window().window_handle();
        let response = FileDialog::new()
            .set_parent(&window_handle)
            .pick_folder()
            .await
            .map(|folder| {
                (
                    folder.path().to_path_buf(),
                    PaintType::from(&app),
                    app.get_item_index(),
                    ResizeType::from(&app),
                    ResizeFilter::from(&app),
                    app.get_compression_settings(),
                )
            });

        if response.is_some() {
            app.set_saving(true);
        }

        app.set_file_dialog_opened(false);
        tx.send(response)
    })
    .expect("Couldn't spawn thread");

    let app_ref = app.as_weak();
    let state = state.clone();
    thread::spawn(move || {
        if let Some((
            output_folder,
            paint_type,
            item_index,
            resize_type,
            resize_filter,
            compression_settings,
        )) = rx.recv().expect("Failed to receive folder")
        {
            tracing::debug!(
                "Settings: {paint_type:?}, {resize_type:?}, {resize_filter:?}, {compression_settings:?}"
            );

            let state_handle = state.clone();
            let canvas_thread =
                thread::spawn(move || state_handle.canvas(resize_type, resize_filter));

            let state_handle = state.clone();
            let thumbnail_thread =
                thread::spawn(move || state_handle.encoded_thumbnail(resize_type, resize_filter));

            let state_handle = state.clone();
            let texture_thread = thread::spawn(move || {
                state_handle.encoded_texture(paint_type, resize_type, resize_filter)
            });

            let canvas_bytes = swizzle_uncompressed_bytes(
                CANVAS_SIZE,
                CANVAS_SIZE,
                &canvas_thread.join().expect("Failed to join canvas thread"),
            )
            .expect("Failed to swizzle bytes");

            let texture_size = if paint_type == PaintType::Food {
                FOOD_SIZE
            } else {
                TEXTURE_SIZE
            };

            let texture_bytes = BcTextureEncoder::new(BcFormat::Bc1)
                .swizzle_bytes(
                    texture_size,
                    texture_size,
                    &texture_thread
                        .join()
                        .expect("Failed to join texture thread"),
                )
                .expect("Failed to swizzle bytes");

            let thumbnail_bytes = BcTextureEncoder::new(BcFormat::Bc3)
                .swizzle_bytes(
                    THUMBNAIL_SIZE,
                    THUMBNAIL_SIZE,
                    &thumbnail_thread
                        .join()
                        .expect("Failed to join thumbnail thread"),
                )
                .expect("Failed to swizzle bytes");

            tracing::info!("Exporting to {}", output_folder.display());

            let file_stem = format!("{}{item_index:0>3}", paint_type.file_name());

            if compression_settings.enabled {
                let mut canvas_file =
                    File::create(output_folder.join(&file_stem).with_extension("canvas.zs"))
                        .expect("Failed to create canvas file");
                let mut texture_file =
                    File::create(output_folder.join(&file_stem).with_extension("ugctex.zs"))
                        .expect("Failed to create texture file");
                let mut thumbnail_file = File::create(
                    output_folder
                        .join(format!("{file_stem}_Thumb"))
                        .with_extension("ugctex.zs"),
                )
                .expect("Failed to create thumbnail file");

                canvas_file
                    .write_all(
                        &zstd_compress_bytes(&canvas_bytes, compression_settings.level)
                            .expect("Failed to compress bytes"),
                    )
                    .expect("Failed to write canvas bytes");
                texture_file
                    .write_all(
                        &zstd_compress_bytes(&texture_bytes, compression_settings.level)
                            .expect("Failed to compress bytes"),
                    )
                    .expect("Failed to write texture bytes");
                thumbnail_file
                    .write_all(
                        &zstd_compress_bytes(&thumbnail_bytes, compression_settings.level)
                            .expect("Failed to compress bytes"),
                    )
                    .expect("Failed to write thumbnail bytes");
            } else {
                let mut canvas_file =
                    File::create(output_folder.join(&file_stem).with_extension("canvas"))
                        .expect("Failed to create canvas file");
                let mut texture_file =
                    File::create(output_folder.join(&file_stem).with_extension("ugctex"))
                        .expect("Failed to create texture file");
                let mut thumbnail_file = File::create(
                    output_folder
                        .join(format!("{file_stem}_Thumb"))
                        .with_extension("ugctex"),
                )
                .expect("Failed to create thumbnail file");

                canvas_file
                    .write_all(&canvas_bytes)
                    .expect("Failed to write canvas bytes");
                texture_file
                    .write_all(&texture_bytes)
                    .expect("Failed to write texture bytes");
                thumbnail_file
                    .write_all(&thumbnail_bytes)
                    .expect("Failed to write thumbnail bytes");
            };

            app_ref
                .upgrade_in_event_loop(|handle| {
                    handle.set_saving(false);
                })
                .expect("Couldn't get app");
        }
    });
}
