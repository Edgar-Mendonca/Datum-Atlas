//! Datum Atlas core: materials and their referenced values, units, property-chart geometry
//! (bubbles and family hulls), selection stages with reasons and ranking, edits and .datlas files.
//! No UI or OS code lives here.
pub mod chart;
pub mod compare;
pub mod db;
pub mod formula;
pub mod doc;
pub mod eco;
pub mod elements;
pub mod export;
pub mod geometry;
pub mod goals;
pub mod micro;
pub mod model;
pub mod ops;
pub mod process;
pub mod select;
pub mod sheet;
pub mod templates;
pub mod units;

pub use model::*;

#[cfg(test)]
mod hardening;
#[cfg(test)]
mod tests;
