use rfd::{AsyncFileDialog, FileHandle};

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
    "png", "avif", "bmp", "exr", "ff", "gif", "hdr", "ico", "jpeg", "pnm", "qoi", "tga", "tiff",
    "webp",
];

pub const SUPPORTED_TEXTURE_FORMATS: &[&str] = &["canvas", "ugctex", "ugctex.zs", "canvas.zs"];

#[derive(Debug, Default)]
pub struct FileDialogBuilder {
    title: Option<String>,
    filters: Vec<(String, Vec<String>)>,
}

impl FileDialogBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn add_filter(mut self, name: impl Into<String>, extensions: &[&str]) -> Self {
        self.filters.push((
            name.into(),
            extensions.iter().map(|s| s.to_string()).collect(),
        ));

        self
    }

    pub fn formats_filter(self) -> Self {
        self.add_filter("Supported formats", ALL_SUPPORTED_FORMATS)
    }

    pub fn image_formats_filter(self) -> Self {
        self.add_filter("Supported image formats", SUPPORTED_IMAGE_FORMATS)
    }

    pub fn texture_formats_filter(self) -> Self {
        self.add_filter("Supported texture formats", SUPPORTED_TEXTURE_FORMATS)
    }

    pub fn encodable_formats_filter(self) -> Self {
        let mut builder = self;
        for format in ENCODEABLE_IMAGE_FORMATS {
            builder = builder.add_filter(format.to_uppercase().as_str(), &[format]);
        }
        builder
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

        for (name, extensions) in self.filters {
            inner = inner.add_filter(&name, &extensions);
        }

        inner = inner.set_parent(&super::window_handle());

        if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
            inner = inner.add_filter("All files", &["*"]);
        }

        inner
    }
}
