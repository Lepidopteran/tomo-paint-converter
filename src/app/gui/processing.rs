use super::*;

#[derive(Debug, Clone, Copy)]
pub enum OutputType {
    Texture,
    Canvas,
    Thumbnail,
}

impl OutputType {
    pub fn model() -> VecModel<SharedString> {
        VecModel::from(vec![
            OutputType::Texture.to_string().into(),
            OutputType::Canvas.to_string().into(),
            OutputType::Thumbnail.to_string().into(),
        ])
    }
}

impl Display for OutputType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            OutputType::Texture => "Texture",
            OutputType::Canvas => "Canvas",
            OutputType::Thumbnail => "Thumbnail",
        })
    }
}

impl FromStr for OutputType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(" ", "").as_str() {
            "texture" => Ok(OutputType::Texture),
            "canvas" => Ok(OutputType::Canvas),
            "thumbnail" => Ok(OutputType::Thumbnail),
            _ => Err(format!("Invalid output type: {}", s)),
        }
    }
}
