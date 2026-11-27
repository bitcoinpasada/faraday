//! The canvas, its call list and the ink record (09 §3.2).

use alloc::vec::Vec;

use crate::geom::Rect;

/// A drawing surface the size of the display.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/canvas.rs`.
pub struct Canvas(pub ());

/// The call list a frame records and replays in bands.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/canvas/list.rs` (`List`).
pub struct Calls(pub ());

/// What one frame drew, in drawing order; kept only while the record is on (04 §9, V21). Its
/// texts are `Text`, wiped when the record is dropped or replaced.
pub struct Ink(pub Vec<Inked>);

/// One thing a frame drew: `main`'s `canvas::Ink`, with the secret mark of 04 K12.
pub struct Inked {
    pub kind: InkKind,
    /// The box its visible ink covers inside the clip it was drawn in.
    pub rect: Rect,
    /// It was drawn from a `Shown::secret`.
    pub secret: bool,
}

/// What an `Inked` is: a line of text with its face, colour and whether it runs past its clip;
/// an icon; a progress indicator; a badge's dot; the wordmark.
// Placeholder until the domain copy: `main`'s `core/faraday-draw/src/canvas.rs` (`InkKind`).
pub struct InkKind(pub ());
