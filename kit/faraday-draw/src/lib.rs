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
use faraday_tokens::motion::{Easing, EasingPath};
// The colour type the tests paint with (any colour, not a token's value).
#[cfg(test)]
use faraday_tokens::palette::Color;
pub use geom::{Dp, Edges, Point, Rect, Scale, Size};
pub use text::{Font, TextAlign};
