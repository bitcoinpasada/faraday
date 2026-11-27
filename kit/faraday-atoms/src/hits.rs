//! Hits and the Tab order (04 §7.1).

use faraday_draw::geom::Rect;

/// A press recorded where it was drawn.
pub struct Hit<R> {
    /// The control as drawn.
    pub rect: Rect,
    /// Where a press is taken: the control grown to the touch floor (`main`'s `Reach`).
    pub target: Rect,
    /// What a press sends, lifted to the root's press.
    pub press: R,
    /// 0 for the page, then 1 for the lowest sheet over it and up; only the top layer drawn
    /// takes presses.
    pub layer: u8,
}
