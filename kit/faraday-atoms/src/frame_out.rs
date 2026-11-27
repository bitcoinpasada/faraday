//! What a frame gives back (04 §7.1, §7.3).

use faraday_draw::canvas::Ink;
use faraday_draw::geom::Rect;
use faraday_port::values::{Anchor, Pace};

use crate::Pressed;
use crate::hits::Hit;
use crate::region::Scrolled;

/// What the runtime keeps of the last frame. Generic over the root's press type.
pub struct FrameOut<R: Pressed> {
    /// Rect, touch target, press, sheet layer; last drawn wins.
    pub hits: Vec<Hit<R>>,
    /// The Tab order: indices into hits, in drawing order.
    pub reach: Vec<usize>,
    /// The page drawn, set by the screen template.
    pub page: Anchor,
    /// The sheets drawn over it, lowest first, set by the same.
    pub sheets: Vec<Anchor>,
    /// The screen's back link and the index in `hits` of its own press: the mouse's back button.
    pub back: Option<(R, usize)>,
    pub anchors: Vec<(Anchor, Rect)>,
    /// Id, view, extent: what a wheel or a finger moves.
    pub regions: Vec<Scrolled>,
    pub laid: Vec<(Anchor, Laid, R)>,
    /// The tooltip anchors the frame drew and the screen template did not take (`Ui::take_tips`):
    /// where each control with a tooltip is, its press and its text.
    pub tips: Vec<crate::icon_button::TipAnchor<R>>,
    /// A secret was drawn (04 §11).
    pub secret: bool,
    /// Rest or Moving: motion wants another frame.
    pub pace: Pace,
    /// Only when the record is on (04 §9, V21).
    pub ink: Option<Ink>,
}

/// A layout answer, never a coordinate (04 §7.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Laid {
    Placed(Placement),
    Room(bool),
    Fits(bool),
    Rows(u16),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Placement {
    Stacked,
    Beside,
    OneColumn,
}
