//! The kit's atoms level: the rest of core/faraday-draw. Section 09 §3.2.

#![no_std]

extern crate alloc;

pub mod canvas;
pub mod ease;
pub mod fonts;
pub mod geom;
pub mod icons;
pub mod picture;
pub mod qr;
pub mod shapes;
mod skia;
pub mod text;

pub use canvas::{BlendBand, Canvas};
pub use faraday_tokens::palette::Color;
pub use geom::{Dp, Edges, Point, Rect, Scale, Size, SizeClass};
pub use text::{Font, TextAlign};
