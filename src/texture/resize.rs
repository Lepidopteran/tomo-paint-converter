use image::{DynamicImage, GenericImage, ImageBuffer, Rgba, imageops::FilterType};
use strum::{Display, EnumIter, EnumString};

use super::Texture;

#[derive(clap::ValueEnum, Display, EnumString, EnumIter, Debug, Clone, Copy, PartialEq, Eq)]
#[strum(serialize_all = "title_case")]
pub enum ResizeType {
    /// Preserve image aspect ratio
    #[strum(serialize = "Preserve Ratio")]
    Preserve,
    /// Fill image preserving aspect ratio and cropping
    Fill,
    /// Resize image to exact size, ignoring aspect ratio
    Exact,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy, Display, EnumIter, EnumString, PartialEq, Eq)]
pub enum ResizeFilter {
    /// Nearest neighbor
    Nearest,
    /// Bilinear
    Bilinear,
    /// Catmull-Rom
    CatmullRom,
    /// Gaussian
    Gaussian,
    /// Lanczos
    Lanczos3,
}

impl From<FilterType> for ResizeFilter {
    fn from(value: FilterType) -> Self {
        match value {
            FilterType::Nearest => Self::Nearest,
            FilterType::Triangle => Self::Bilinear,
            FilterType::CatmullRom => Self::CatmullRom,
            FilterType::Gaussian => Self::Gaussian,
            FilterType::Lanczos3 => Self::Lanczos3,
        }
    }
}

impl From<ResizeFilter> for FilterType {
    fn from(value: ResizeFilter) -> Self {
        match value {
            ResizeFilter::Nearest => FilterType::Nearest,
            ResizeFilter::Bilinear => FilterType::Triangle,
            ResizeFilter::CatmullRom => FilterType::CatmullRom,
            ResizeFilter::Gaussian => FilterType::Gaussian,
            ResizeFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

pub fn resize(
    input: &Texture,
    nwidth: u32,
    nheight: u32,
    method: ResizeType,
    filter: ResizeFilter,
) -> Texture {
    let img = &input.inner_image;
    let resized = match method {
        ResizeType::Preserve => {
            let original_width = img.width();
            let original_height = img.height();

            let scale = f32::min(
                nwidth as f32 / original_width as f32,
                nheight as f32 / original_height as f32,
            );

            let new_w = (original_width as f32 * scale) as u32;
            let new_h = (original_height as f32 * scale) as u32;

            let resized = img.resize(nwidth, nwidth, filter.into());

            let mut canvas = ImageBuffer::from_pixel(nwidth, nheight, Rgba([0, 0, 0, 0]));

            let x = (nwidth - new_w) / 2;
            let y = (nheight - new_h) / 2;

            canvas
                .copy_from(&resized.to_rgba8(), x, y)
                .expect("Failed to copy image");

            DynamicImage::ImageRgba8(canvas)
        }
        ResizeType::Fill => img.resize_to_fill(nwidth, nheight, filter.into()),
        ResizeType::Exact => img.resize_exact(nwidth, nheight, filter.into()),
    };

    Texture::from_image(resized)
}
