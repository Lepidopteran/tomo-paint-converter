use rfd::{AsyncFileDialog, FileHandle};
use slint::WindowHandle;

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

#[derive(Debug, Default)]
pub struct FileDialog {
    inner: AsyncFileDialog,
}

impl FileDialog {
    pub fn new() -> Self {
        Self {
            inner: AsyncFileDialog::new(),
        }
    }

    pub async fn pick_folder(self) -> Option<FileHandle> {
        self.inner.pick_folder().await
    }

    pub async fn pick_file(self) -> Option<FileHandle> {
        let mut dialog = self.inner;
        if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
            dialog = dialog.add_filter("All files", &["*"]);
        }

        dialog.pick_file().await
    }

    pub fn add_supported_formats(mut self) -> Self {
        self.inner = self
            .inner
            .add_filter("Supported formats", ALL_SUPPORTED_FORMATS);

        self.inner = self.inner.add_filter("Images", SUPPORTED_IMAGE_FORMATS);
        self.inner = self.inner.add_filter("Textures", SUPPORTED_TEXTURE_FORMATS);

        self
    }

    pub fn set_parent(mut self, parent: &WindowHandle) -> Self {
        self.inner = self.inner.set_parent(parent);
        self
    }

    pub fn add_filter(mut self, name: &str, extensions: &[&str]) -> Self {
        self.inner = self.inner.add_filter(name, extensions);
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.inner = self.inner.set_title(title);
        self
    }
}
