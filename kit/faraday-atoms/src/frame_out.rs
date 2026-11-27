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
    /// The presses of the typed lines the frame drew: a key that edits or moves a caret is
    /// theirs when one is focused.
    pub typed: Vec<R>,
    /// The tooltip anchors the frame drew and the screen template did not take (`Ui::take_tips`):
    /// where each control with a tooltip is, its press and its text.
    pub tips: Vec<crate::icon_button::TipAnchor<R>>,
    /// The presses of the holds the frame drew (a hold sends its press when it is held to its
    /// end, never when the contact lifts).
    pub holds: Vec<R>,
    /// The holds among them that are full this frame.
    pub held: Vec<R>,
    /// The frost made under each sheet this frame for the runtime to keep (K16): none where the
    /// runtime gave one back.
    pub frosts: Vec<crate::keeps::MadeFrost>,
    /// The rows taken of each region that moves, for the runtime to keep (K16).
    pub rows: Vec<crate::keeps::MadeRows>,
    /// How many regions were drawn as moved frames: their rows put back, only the strip painted.
    pub moved: usize,
    /// Where each of them moved: the region's view and the rows its kept rows now stand lower
    /// by (higher when negative), for the shell (05 §9).
    pub slid: Vec<(Rect, i32)>,
    /// A caret was drawn, shown or in the off half of its blink: nothing kept of this frame
    /// stands for the next while it blinks.
    pub caret: bool,
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
