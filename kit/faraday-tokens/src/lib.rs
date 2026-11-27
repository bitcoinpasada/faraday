//! The kit's tokens level: core/faraday-draw/src/tokens.rs; faraday-ui's pal, theme.
//! Constants and their helpers only (K2). Section 09 §3.2.

#![no_std]

extern crate alloc;

pub mod dims;
pub mod elevation;
pub mod motion;
pub mod palette;
pub mod type_roles;

mod theme;

pub use theme::{Contrast, Mode, TextSize, Theme};
