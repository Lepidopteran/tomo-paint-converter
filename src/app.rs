use std::fmt::Display;

use slint::{SharedString, VecModel};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

pub mod cli;
pub mod gui;

fn enum_to_model<T>() -> VecModel<SharedString>
where
    T: IntoEnumIterator + Display,
{
    VecModel::from_iter(T::iter().map(|v| SharedString::from(v.to_string())))
}

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
