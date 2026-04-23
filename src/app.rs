use std::fmt::Display;

use slint::{SharedString, VecModel};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

pub mod cli;
pub mod gui;

pub trait VecEnumModel: IntoEnumIterator + Display {
    fn model() -> VecModel<SharedString> {
        VecModel::from_iter(Self::iter().map(|v| SharedString::from(v.to_string())))
    }
}

impl<T> VecEnumModel for T where T: IntoEnumIterator + Display {}

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
