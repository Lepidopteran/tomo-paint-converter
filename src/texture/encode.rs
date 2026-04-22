use color_eyre::eyre::Result;

pub trait TextureEncoder {
    fn encode_texture(&self, buf: &[u8], width: u32, height: u32) -> Result<Vec<u8>>;
}
