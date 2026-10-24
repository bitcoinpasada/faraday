//! The drawing base of Faraday's screens: the
//! pixel canvas, text and the fonts, geometry, the design tokens, the layout
//! context, and the icon and keyboard widgets.
//!
//! The crate draws into a premultiplied RGBA framebuffer. It has no flows
//! and no wording of its own. See `README.md` in this crate.

#![no_std]

extern crate alloc;

pub mod canvas;
mod color;
pub mod easing;
pub mod fonts;
pub mod geom;
pub mod layout;
pub mod text;
pub mod tokens;
pub mod widgets;

pub use canvas::Canvas;
pub use color::Color;
pub use geom::{Dp, Edges, Point, Rect, Scale, Size, SizeClass};
pub use text::{Font, TextAlign};
