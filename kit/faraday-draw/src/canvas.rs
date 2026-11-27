//! The canvas, its call list and the ink record (09 §3.2).

/// A drawing surface the size of the display.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/canvas.rs`.
pub struct Canvas(pub ());

/// The call list a frame records and replays in bands.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/canvas/list.rs` (`List`).
pub struct Calls(pub ());

/// Every string a frame drew, each with its box; kept only when the record is on (04 §9).
// Spec: 09 §3.2 names this without a shape.
pub struct Ink(pub ());
