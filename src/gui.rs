use color_eyre::eyre::Result;
use rfd::AsyncFileDialog;
use slint::{Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use tomo_image_converter::PaintType;

slint::include_modules!();

struct State {
    file_bytes: Vec<u8>,
}

impl State {}

pub fn run() -> Result<()> {
    let app = AppWindow::new()?;

    app.set_texture_type_model(ModelRc::new(VecModel::from(
        PaintType::variants()
            .into_iter()
            .map(|s| s.into())
            .collect::<Vec<_>>(),
    )));

    let weak_app = app.as_weak();
    app.on_pick_file_input(move || {
        let weak_app = weak_app.clone();
        slint::spawn_local(async move {
            let app = weak_app.upgrade().unwrap();
            app.set_file_dialog_opened(true);
            let file = file_dialog().pick_file().await;

            if let Some(file) = file
                && let Some(app) = weak_app.upgrade()
            {
                app.set_input_path(file.path().to_string_lossy().to_string().into());
                app.set_viewer_image(
                    Image::load_from_path(file.path()).expect("Failed to load image"),
                );
                app.set_file_dialog_opened(false);
                app.set_image_loaded(true);
            }
        })
        .unwrap();
    });

    Ok(app.run()?)
}

fn file_dialog() -> AsyncFileDialog {
    AsyncFileDialog::new()
        .add_filter(
            "Supported Files",
            &[
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
            ],
        )
        .add_filter(
            "Image Files",
            &[
                "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpeg", "png", "pnm",
                "qoi", "tga", "tiff", "webp",
            ],
        )
        .add_filter(
            "Texture Files",
            &["canvas", "ugctex", "ugctex.zs", "canvas.zs"],
        )
        .add_filter("All Files", &["*"])
}
