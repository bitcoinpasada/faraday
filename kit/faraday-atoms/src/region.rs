//! Scrolled regions (04 §7.1).

use faraday_draw::geom::{Rect, Size};
use faraday_port::values::Anchor;

/// What a wheel or a finger moves.
pub struct Scrolled {
    /// The region's anchor: its entry in the motion store.
    pub id: Anchor,
    /// The part of the window it shows.
    pub view: Rect,
    /// The size of all it holds.
    pub extent: Size,
}
