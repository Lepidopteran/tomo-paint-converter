use std::{fmt::Display, str::FromStr};

use image::{DynamicImage, GenericImage, ImageBuffer, Rgba, imageops::FilterType};

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum ResizeType {
    /// Preserve image aspect ratio
    Preserve,
    /// Fill image preserving aspect ratio and cropping
    Fill,
    /// Resize image to exact size, ignoring aspect ratio
    Exact,
}

impl Display for ResizeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ResizeType::Preserve => "Preserve Ratio",
            ResizeType::Fill => "Fill",
            ResizeType::Exact => "Exact",
        })
    }
}

impl FromStr for ResizeType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "preserveratio" => Ok(Self::Preserve),
            "fill" => Ok(Self::Fill),
            "exact" => Ok(Self::Exact),
            _ => Err(format!("Unknown resize type {}", s)),
        }
    }
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
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

impl Display for ResizeFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Nearest => "Nearest",
                Self::Bilinear => "Bilinear",
                Self::CatmullRom => "CatmullRom",
                Self::Gaussian => "Gaussian",
                Self::Lanczos3 => "Lanczos3",
            }
        )
    }
}

impl FromStr for ResizeFilter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(" ", "").as_str() {
            "nearest" => Ok(ResizeFilter::Nearest),
            "bilinear" => Ok(ResizeFilter::Bilinear),
            "catmullrom" => Ok(ResizeFilter::CatmullRom),
            "gaussian" => Ok(ResizeFilter::Gaussian),
            "lanczos3" => Ok(ResizeFilter::Lanczos3),
            _ => Err(format!("Invalid resize filter: {}", s)),
        }
    }
}

pub fn resize_image(
    img: &DynamicImage,
    size: u32,
    method: ResizeType,
    filter: ResizeFilter,
) -> DynamicImage {
    match method {
        ResizeType::Preserve => {
            let original_width = img.width();
            let original_height = img.height();

            let scale = f32::min(
                size as f32 / original_width as f32,
                size as f32 / original_height as f32,
            );

            let new_w = (original_width as f32 * scale) as u32;
            let new_h = (original_height as f32 * scale) as u32;

            let resized = img.resize(size, size, filter.into());

            let mut canvas = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));

            let x = (size - new_w) / 2;
            let y = (size - new_h) / 2;

            canvas
                .copy_from(&resized.to_rgba8(), x, y)
                .expect("Failed to copy image");

            DynamicImage::ImageRgba8(canvas)
        }
        ResizeType::Fill => img.resize_to_fill(size, size, filter.into()),
        ResizeType::Exact => img.resize_exact(size, size, filter.into()),
    }
}
