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

    pub fn resize_type(&self) -> ResizeType {
        self.resize_type
    }

    pub fn resize_filter(&self) -> ResizeFilter {
        self.resize_filter
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
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

    pub fn paint_type(&self) -> PaintType {
        self.paint_type
    }

    pub fn resize_type(&self) -> ResizeType {
        self.resize_type
    }

    pub fn resize_filter(&self) -> ResizeFilter {
        self.resize_filter
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

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug)]
pub enum CachedTexture<'t> {
    Texture(&'t TextureCache),
    UgcTexture(&'t UgcTextureCache),
}

impl<'t> From<&'t TextureCache> for CachedTexture<'t> {
    fn from(value: &'t TextureCache) -> Self {
        CachedTexture::Texture(value)
    }
}

impl<'t> From<&'t UgcTextureCache> for CachedTexture<'t> {
    fn from(value: &'t UgcTextureCache) -> Self {
        CachedTexture::UgcTexture(value)
    }
}

impl CachedTexture<'_> {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            CachedTexture::Texture(texture) => texture.as_bytes(),
            CachedTexture::UgcTexture(ugc_texture) => ugc_texture.as_bytes(),
        }
    }

    pub fn resize_type(&self) -> ResizeType {
        match self {
            CachedTexture::Texture(texture) => texture.resize_type(),
            CachedTexture::UgcTexture(ugc_texture) => ugc_texture.resize_type(),
        }
    }

    pub fn resize_filter(&self) -> ResizeFilter {
        match self {
            CachedTexture::Texture(texture) => texture.resize_filter(),
            CachedTexture::UgcTexture(ugc_texture) => ugc_texture.resize_filter(),
        }
    }

    pub fn texture_type(&self) -> Option<PaintType> {
        match self {
            CachedTexture::UgcTexture(ugc_texture) => Some(ugc_texture.paint_type()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CacheKey {
    Texture,
    Canvas,
    Thumbnail,
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

    pub fn get(&self, key: CacheKey) -> Option<CachedTexture<'_>> {
        match key {
            CacheKey::Texture => self.texture.as_ref().map(CachedTexture::from),
            CacheKey::Canvas => self.canvas.as_ref().map(CachedTexture::from),
            CacheKey::Thumbnail => self.thumbnail.as_ref().map(CachedTexture::from),
        }
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
