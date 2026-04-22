use slint::{Rgba8Pixel, SharedPixelBuffer};

use super::OutputType;

pub type Rgba8Buffer = SharedPixelBuffer<Rgba8Pixel>;

#[derive(Default)]
pub struct ImageDataCache {
    source: Option<Rgba8Buffer>,
    texture: Option<Rgba8Buffer>,
    canvas: Option<Rgba8Buffer>,
    thumbnail: Option<Rgba8Buffer>,
}

impl ImageDataCache {
    pub fn clear(&mut self) {
        self.source = None;
        self.texture = None;
        self.canvas = None;
        self.thumbnail = None;
    }

    pub fn replace(&mut self, output_type: OutputType, image: Rgba8Buffer) {
        match output_type {
            OutputType::Source => self.source = Some(image),
            OutputType::Texture => self.texture = Some(image),
            OutputType::Canvas => self.canvas = Some(image),
            OutputType::Thumbnail => self.thumbnail = Some(image),
        }
    }

    pub fn get(&self, output_type: OutputType) -> Option<Rgba8Buffer> {
        match output_type {
            OutputType::Source => self.source.as_ref().cloned(),
            OutputType::Texture => self.texture.as_ref().cloned(),
            OutputType::Canvas => self.canvas.as_ref().cloned(),
            OutputType::Thumbnail => self.thumbnail.as_ref().cloned(),
        }
    }
}
