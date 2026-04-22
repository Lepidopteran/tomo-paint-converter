use color_eyre::eyre::Result;

pub trait TextureDecoder {
    fn decode_bytes(&self, data: &[u8], width: u32, height: u32) -> Result<Vec<u8>>;
}
