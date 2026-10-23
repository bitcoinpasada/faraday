//! Where the pointer or finger is and what it has hold of: the controls
//! the last frame offered, the one pressed, the drag, the held scrollbar
//! and the hover.

use faraday_draw::Rect;

use crate::Action;

/// The pointer's state (`docs/STRUCTURE-PASS.md` §7.4.4, step 47). One
/// field on `Faraday`; the fields are read and written through these
/// methods.
pub(crate) struct Pointer {
    /// What the last frame offered to a press, in drawing order.
    hits: Vec<(Rect, Action)>,
    /// The control a press is down on, and where it was drawn.
    pressed: Option<(Action, Rect)>,
    /// The last tap that acted and when, for a double-click.
    last_tap: Option<(Action, u64)>,
    /// Where the finger or pointer went down, in pixels.
    down_at: (i32, i32),
    /// A finger on the scrolled region: the row it was last at, and
    /// whether it has moved far enough to be scrolling rather than
    /// pressing.
    drag: Option<(i32, bool)>,
    /// The touch now down only stopped a coast, and does nothing else.
    swallow: bool,
    /// The overlay scrollbar is held: how far down its thumb, in pixels,
    /// the finger or pointer took it.
    bar_held: Option<i32>,
    /// The tick the last two-finger scroll came in at, until its end.
    scroll_at: Option<u64>,
    /// Where the pointer is, with no button down, in pixels.
    hover: Option<(i32, i32)>,
    /// What can be pressed under it.
    hovered: Option<Action>,
}

impl Pointer {
    pub(crate) fn new() -> Self {
        Self {
            hits: Vec::new(),
            pressed: None,
            last_tap: None,
            down_at: (0, 0),
            drag: None,
            swallow: false,
            bar_held: None,
            scroll_at: None,
            hover: None,
            hovered: None,
        }
    }

    /// What the last frame offered to a press, in drawing order.
    pub(crate) fn hits(&self) -> &[(Rect, Action)] {
        &self.hits
    }

    /// Takes the hits out to be filled by the next frame.
    pub(crate) fn take_hits(&mut self) -> Vec<(Rect, Action)> {
        std::mem::take(&mut self.hits)
    }

    /// Puts back the hits a frame drew.
    pub(crate) fn set_hits(&mut self, hits: Vec<(Rect, Action)>) {
        self.hits = hits;
    }

    /// The topmost control offered at (x, y), with where it was drawn.
    pub(crate) fn hit_at(&self, x: i32, y: i32) -> Option<(Action, Rect)> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| r.contains(x, y))
            .map(|(r, a)| (*a, *r))
    }

    /// Where the topmost offer of `action` was drawn.
    pub(crate) fn rect_of(&self, action: Action) -> Option<Rect> {
        self.hits
            .iter()
            .rev()
            .find(|(_, a)| *a == action)
            .map(|(r, _)| *r)
    }

    /// Whether the last frame offered `action`.
    pub(crate) fn offers(&self, action: Action) -> bool {
        self.hits.iter().any(|(_, a)| *a == action)
    }

    pub(crate) fn pressed(&self) -> Option<(Action, Rect)> {
        self.pressed
    }

    pub(crate) fn set_pressed(&mut self, pressed: Option<(Action, Rect)>) {
        self.pressed = pressed;
    }

    pub(crate) fn take_pressed(&mut self) -> Option<(Action, Rect)> {
        self.pressed.take()
    }

    pub(crate) fn last_tap(&self) -> Option<(Action, u64)> {
        self.last_tap
    }

    pub(crate) fn set_last_tap(&mut self, tap: Option<(Action, u64)>) {
        self.last_tap = tap;
    }

    pub(crate) fn down_at(&self) -> (i32, i32) {
        self.down_at
    }

    pub(crate) fn set_down_at(&mut self, at: (i32, i32)) {
        self.down_at = at;
    }

    pub(crate) fn drag(&self) -> Option<(i32, bool)> {
        self.drag
    }

    pub(crate) fn set_drag(&mut self, drag: Option<(i32, bool)>) {
        self.drag = drag;
    }

    pub(crate) fn take_drag(&mut self) -> Option<(i32, bool)> {
        self.drag.take()
    }

    pub(crate) fn swallow(&self) -> bool {
        self.swallow
    }

    pub(crate) fn set_swallow(&mut self, swallow: bool) {
        self.swallow = swallow;
    }

    pub(crate) fn take_swallow(&mut self) -> bool {
        std::mem::take(&mut self.swallow)
    }

    pub(crate) fn bar_held(&self) -> Option<i32> {
        self.bar_held
    }

    pub(crate) fn set_bar_held(&mut self, grab: Option<i32>) {
        self.bar_held = grab;
    }

    pub(crate) fn take_bar_held(&mut self) -> Option<i32> {
        self.bar_held.take()
    }

    pub(crate) fn scroll_at(&self) -> Option<u64> {
        self.scroll_at
    }

    pub(crate) fn set_scroll_at(&mut self, at: Option<u64>) {
        self.scroll_at = at;
    }

    pub(crate) fn hover(&self) -> Option<(i32, i32)> {
        self.hover
    }

    pub(crate) fn set_hover(&mut self, at: Option<(i32, i32)>) {
        self.hover = at;
    }

    pub(crate) fn hovered(&self) -> Option<Action> {
        self.hovered
    }

    /// Finds what can be pressed under the pointer in the frame on
    /// screen. Returns whether that changed.
    pub(crate) fn find_hovered(&mut self) -> bool {
        let now = self
            .hover
            .and_then(|(x, y)| self.hit_at(x, y).map(|(a, _)| a));
        std::mem::replace(&mut self.hovered, now) != now
    }
}
