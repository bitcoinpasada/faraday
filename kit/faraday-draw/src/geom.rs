//! Geometry (09 §3.2): `Rect` and `Size`; `Point` is the port's.

pub use faraday_port::values::Point;

/// A size in whole pixels.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/geom.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub w: i32,
    pub h: i32,
}

/// An axis-aligned rectangle in whole pixels.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/geom.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
