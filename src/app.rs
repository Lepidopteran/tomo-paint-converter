use std::fmt::Display;

use slint::{SharedString, VecModel};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

mod cli;
mod state;

mod loading;
mod processing;

pub use loading::*;
pub use processing::*;

slint::include_modules!();

/// Output type of texture
#[derive(clap::ValueEnum, Debug, Clone, Copy, Eq, PartialEq, EnumString, EnumIter, Display)]
#[strum(serialize_all = "title_case")]
pub enum PaintType {
    Food,
    FacePaint,
    Interior,
    Exterior,
    Treasure,
    Cloth,
    Terrain,
    Object,
}

impl PaintType {
    pub fn file_name(&self) -> &'static str {
        match self {
            PaintType::Food => "UgcFood",
            PaintType::FacePaint => "UgcFacePaint",
            PaintType::Interior => "UgcInterior",
            PaintType::Exterior => "UgcExterior",
            PaintType::Treasure => "UgcGoods",
            PaintType::Cloth => "UgcCloth",
            PaintType::Terrain => "UgcMapFloor",
            PaintType::Object => "UgcMapObject",
        }
    }

    pub fn exclude_thumbnail(&self) -> bool {
        matches!(self, Self::FacePaint)
    }
}

pub trait VecEnumModel: IntoEnumIterator + Display {
    fn model() -> VecModel<SharedString> {
        VecModel::from_iter(Self::iter().map(|v| SharedString::from(v.to_string())))
    }
}

impl<T> VecEnumModel for T where T: IntoEnumIterator + Display {}

pub fn run() -> color_eyre::eyre::Result<()> {
    if let Some(command) = cli::parse_command() {
        command.run()
    } else {
        let app = AppWindow::new()?;
        state::setup(&app)?;

        Ok(app.run()?)
    }
}
