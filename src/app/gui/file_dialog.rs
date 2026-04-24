use rfd::AsyncFileDialog;

pub const ALL_SUPPORTED_FORMATS: &[&str] = &[
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

pub const SUPPORTED_IMAGE_FORMATS: &[&str] = &[
    "avif", "bmp", "dds", "exr", "ff", "gif", "hdr", "ico", "jpeg", "png", "pnm", "qoi", "tga",
    "tiff", "webp",
];

pub const SUPPORTED_TEXTURE_FORMATS: &[&str] = &["canvas", "ugctex", "ugctex.zs", "canvas.zs"];

pub fn file_dialog(title: &str) -> AsyncFileDialog {
    let mut file_dialog = AsyncFileDialog::new().set_title(title);

    if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        file_dialog = file_dialog.add_filter("All files", &["*"]);
    }

    file_dialog
}
