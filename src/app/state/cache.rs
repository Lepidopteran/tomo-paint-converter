use tomo_image_converter::texture::resize::{ResizeFilter, ResizeType};

use crate::app::PaintType;

#[derive(Debug, Clone)]
pub struct TextureCache {
    bytes: Vec<u8>,
    resize_type: ResizeType,
    resize_filter: ResizeFilter,
}

impl TextureCache {
    pub fn new(bytes: Vec<u8>, resize_type: ResizeType, resize_filter: ResizeFilter) -> Self {
        Self {
            bytes,
            resize_type,
            resize_filter,
        }
    }

    pub fn is_invalid(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> bool {
        self.resize_type != resize_type || self.resize_filter != resize_filter
    }

    pub fn is_valid(&self, resize_type: ResizeType, resize_filter: ResizeFilter) -> bool {
        !self.is_invalid(resize_type, resize_filter)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug, Clone)]
pub struct UgcTextureCache {
    bytes: Vec<u8>,
    paint_type: PaintType,
    resize_type: ResizeType,
    resize_filter: ResizeFilter,
}

impl UgcTextureCache {
    pub fn new(
        bytes: Vec<u8>,
        paint_type: PaintType,
        resize_type: ResizeType,
        resize_filter: ResizeFilter,
    ) -> Self {
        Self {
            bytes,
            paint_type,
            resize_type,
            resize_filter,
        }
    }

    pub fn is_invalid(
        &self,
        paint_type: PaintType,
        resize_type: ResizeType,
        resize_filter: ResizeFilter,
    ) -> bool {
        resize_type != self.resize_type
            || resize_filter != self.resize_filter
            || (paint_type != self.paint_type
                && (paint_type == PaintType::Food || self.paint_type == PaintType::Food))
    }

    pub fn is_valid(
        &self,
        paint_type: PaintType,
        resize_type: ResizeType,
        resize_filter: ResizeFilter,
    ) -> bool {
        !self.is_invalid(paint_type, resize_type, resize_filter)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Default, Debug)]
pub struct Cache {
    pub texture: Option<UgcTextureCache>,
    pub canvas: Option<TextureCache>,
    pub thumbnail: Option<TextureCache>,
}

impl Cache {
    pub fn clear(&mut self) {
        self.texture.take();
        self.canvas.take();
        self.thumbnail.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ugc_texture_needs_update() {
        let resize_type = ResizeType::Preserve;
        let resize_filter = ResizeFilter::Bilinear;

        let cache = UgcTextureCache::new(vec![], PaintType::Food, resize_type, resize_filter);
        assert!(cache.is_invalid(PaintType::Interior, resize_type, resize_filter));
        assert!(!cache.is_invalid(PaintType::Food, resize_type, resize_filter));

        let cache = UgcTextureCache::new(vec![], PaintType::Interior, resize_type, resize_filter);
        assert!(cache.is_invalid(PaintType::Food, resize_type, resize_filter));
        assert!(!cache.is_invalid(PaintType::Interior, resize_type, resize_filter));
        assert!(!cache.is_invalid(PaintType::Object, resize_type, resize_filter));
    }
}
