//! Hits and the Tab order (04 §7.1).

use faraday_draw::geom::Rect;

/// A press recorded where it was drawn.
// Spec: 09 §3.2 gives `Hit` as "rect, touch target, press, sheet layer" without a shape.
pub struct Hit<R> {
    pub rect: Rect,
    pub target: Rect,
    pub press: R,
    pub layer: u8,
}
