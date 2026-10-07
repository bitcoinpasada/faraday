//! Layout engine, widgets and pixel renderer for OpenSignerKit
//! (`docs/PLANNING.md` §4.3, §4.5, §13; UX.md §2, §5, §6).
//!
//! The crate turns a widget tree into a premultiplied RGBA framebuffer and
//! turns shell events into widget actions. It has no flows and no wording
//! of its own; `opensigner-core` builds trees from its state each frame.
//! See `README.md` in this crate for the architecture.

#![no_std]

extern crate alloc;

pub mod audit;
pub mod canvas;
pub mod color;
pub mod components;
pub mod descriptor;
pub mod fonts;
pub mod geom;
pub mod layout;
pub mod organisms;
pub mod screens;
pub mod state;
pub mod text;
pub mod tokens;
pub mod widgets;

#[cfg(feature = "gallery")]
pub mod gallery;

pub use canvas::Canvas;
pub use color::{Color, Theme};
pub use geom::{Dp, Edges, Point, Rect, Scale, Size, SizeClass};
pub use layout::{Align, Id, Justify, Layout, Node};
pub use state::{Action, PressLook, UiState};
pub use text::{Font, TextAlign};
pub use widgets::Widget;
