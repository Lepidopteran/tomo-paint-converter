use std::env::{self, var};

use rfd::{AsyncFileDialog, FileDialog, FileHandle};
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

pub const ENCODEABLE_IMAGE_FORMATS: &[&str] = &[
    "avif", "bmp", "exr", "ff", "gif", "hdr", "ico", "jpeg", "png", "pnm", "qoi", "tga", "tiff",
    "webp",
];

pub const SUPPORTED_TEXTURE_FORMATS: &[&str] = &["canvas", "ugctex", "ugctex.zs", "canvas.zs"];

#[derive(Debug, Default)]
pub struct FileDialogBuilder {
    title: Option<String>,
    supported_formats_filter: Option<bool>,
    supported_image_formats_filter: Option<bool>,
    supported_texture_formats_filter: Option<bool>,
}

impl FileDialogBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn formats_filter(mut self, enabled: bool) -> Self {
        self.supported_formats_filter = Some(enabled);
        self
    }

    pub fn image_formats_filter(mut self, enabled: bool) -> Self {
        self.supported_image_formats_filter = Some(enabled);
        self
    }

    pub fn texture_formats_filter(mut self, enabled: bool) -> Self {
        self.supported_texture_formats_filter = Some(enabled);
        self
    }

    pub async fn pick_file(self) -> Option<FileHandle> {
        self.build().pick_file().await
    }

    pub async fn pick_folder(self) -> Option<FileHandle> {
        self.build().pick_folder().await
    }

    pub async fn save_file(self) -> Option<FileHandle> {
        self.build().save_file().await
    }

    pub fn build(self) -> AsyncFileDialog {
        let mut inner = AsyncFileDialog::new();
        if let Some(title) = self.title {
            inner = inner.set_title(&title);
        }

        if self.supported_formats_filter.is_none()
            && self.supported_image_formats_filter.is_none()
            && self.supported_texture_formats_filter.is_none()
        {
            inner = inner.add_filter("Supported formats", ALL_SUPPORTED_FORMATS);
            inner = inner.add_filter("Supported image formats", SUPPORTED_IMAGE_FORMATS);
            inner = inner.add_filter("Supported texture formats", SUPPORTED_TEXTURE_FORMATS);
        } else {
            if let Some(supported_formats_filter) = self.supported_formats_filter {
                if supported_formats_filter {
                    inner = inner.add_filter("Supported formats", ALL_SUPPORTED_FORMATS);
                }
            }
            if let Some(supported_image_formats_filter) = self.supported_image_formats_filter {
                if supported_image_formats_filter {
                    inner = inner.add_filter("Supported image formats", SUPPORTED_IMAGE_FORMATS);
                }
            }
            if let Some(supported_texture_formats_filter) = self.supported_texture_formats_filter {
                if supported_texture_formats_filter {
                    inner =
                        inner.add_filter("Supported texture formats", SUPPORTED_TEXTURE_FORMATS);
                }
            }
        }

        inner = inner.set_parent(&super::window_handle());

        if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
            inner = inner.add_filter("All files", &["*"]);
        }

        inner
    }
}

pub async fn save_image(title: impl Into<String>) -> Option<FileHandle> {
    FileDialogBuilder::new()
        .title(title)
        .build()
        .add_filter("Supported image formats", ENCODEABLE_IMAGE_FORMATS)
        .save_file()
        .await
}
