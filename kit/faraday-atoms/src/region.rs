//! Scrolled regions (04 §7.1).

use faraday_draw::geom::{Rect, Size};
use faraday_port::values::Anchor;

/// What a wheel or a finger moves.
// Spec: 09 §3.2 gives `Scrolled` as "id, view, extent" without a shape.
pub struct Scrolled {
    pub id: Anchor,
    pub view: Rect,
    pub extent: Size,
}
