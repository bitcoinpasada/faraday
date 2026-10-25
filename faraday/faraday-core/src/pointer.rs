//! Where the pointer or finger is and what it has hold of: the controls
//! the last frame offered and where each takes a press, the one
//! pressed, the drag, the held scrollbar, the hover, the control the
//! keyboard is on, and the tooltip they bring up.

use faraday_draw::Rect;
use faraday_shell_api::Key as KeyIn;
use faraday_ui::tooltip::{TipAnchor, TipBy, Tips};

use crate::Action;

/// Where a control the last frame drew takes a press (`faraday_ui::Reach`).
pub(crate) type Reach = faraday_ui::Reach<Action>;

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
    /// Where each control the last frame drew takes a press, as large as
    /// the touch floor, in drawing order: the order Tab moves through.
    reach: Vec<Reach>,
    /// The control the keyboard is on (`docs/DESIGN.md` §4.15): Tab and
    /// Shift+Tab move it, Enter or Space presses it, a press of the
    /// pointer or a finger takes it off.
    focus: Option<Action>,
    /// The tooltip showing and the one coming (`faraday_ui::tooltip`).
    tips: Tips<Action>,
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
            reach: Vec::new(),
            focus: None,
            tips: Tips::default(),
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

    /// Puts back the hits a frame drew, with where each takes a press;
    /// a target whose control is no longer among the hits as it was
    /// drawn (cut back by a clip, taken off by a sheet) is let go.
    pub(crate) fn set_hits(&mut self, hits: Vec<(Rect, Action)>, mut reach: Vec<Reach>) {
        reach.retain(|t| hits.contains(&(t.drawn, t.action)));
        self.hits = hits;
        self.reach = reach;
    }

    /// The topmost control offered at (x, y), with where it was drawn;
    /// beside every control, the target (x, y) falls in.
    pub(crate) fn hit_at(&self, x: i32, y: i32) -> Option<(Action, Rect)> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| r.contains(x, y))
            .map(|(r, a)| (*a, *r))
            .or_else(|| faraday_ui::reach_at(&self.reach, x, y))
    }

    /// The control the keyboard is on, while the last frame offered it.
    pub(crate) fn focused(&self) -> Option<Action> {
        self.focus.filter(|a| self.offers(*a))
    }

    /// What a key left over by every screen does to the keyboard's place
    /// (`docs/DESIGN.md` §4.15): Tab and Shift+Tab move it through the
    /// controls in the order the last frame drew them; Enter or Space
    /// returns the control it is on, to be pressed.
    pub(crate) fn focus_key(&mut self, key: KeyIn) -> Option<Action> {
        match key {
            KeyIn::Tab | KeyIn::BackTab => {
                let order: Vec<Action> = self.reach.iter().map(|t| t.action).collect();
                let at = self.focused();
                self.focus = faraday_ui::focus_step(&order, at, key == KeyIn::BackTab);
                None
            }
            KeyIn::Enter | KeyIn::Char(' ') => self.focused(),
            _ => None,
        }
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

    /// How many places the last frame offered `action` in.
    pub(crate) fn offered_times(&self, action: Action) -> usize {
        self.hits.iter().filter(|(_, a)| *a == action).count()
    }

    pub(crate) fn pressed(&self) -> Option<(Action, Rect)> {
        self.pressed
    }

    /// A press is down, or let go: a press takes the keyboard's place
    /// off its control.
    pub(crate) fn set_pressed(&mut self, pressed: Option<(Action, Rect)>) {
        if pressed.is_some() {
            self.focus = None;
        }
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

    /// The owner whose tooltip shows.
    pub(crate) fn tip_shown(&self) -> Option<Action> {
        self.tips.shown()
    }

    /// Takes the tooltip anchors a frame drew: a tooltip whose anchor
    /// the frame did not draw goes (it was not drawn either).
    pub(crate) fn set_tip_anchors(&mut self, anchors: Vec<TipAnchor<Action>>) {
        self.tips.set_anchors(anchors);
    }

    /// The pointer moved or left at `now` ms: the tooltip under it is on
    /// its way, one it left goes. Returns whether what shows changed.
    pub(crate) fn tip_hover(&mut self, now: u64) -> bool {
        self.tips.hover(self.hover, now)
    }

    /// A finger or a button went down at `at`, at `now` ms. Returns
    /// whether a tooltip went.
    pub(crate) fn tip_down(&mut self, at: (i32, i32), now: u64) -> bool {
        self.tips.press_down(at, now)
    }

    /// The finger moved to `at`: off the anchor, or scrolling the page,
    /// it is no long press.
    pub(crate) fn tip_moved(&mut self, at: (i32, i32)) {
        let scrolling = self.drag.is_some_and(|(_, d)| d);
        self.tips.press_moved(at, scrolling);
    }

    /// The finger lifted.
    pub(crate) fn tip_up(&mut self) {
        self.tips.press_up();
    }

    /// A key moved the keyboard, at `now` ms: a tooltip of the control
    /// it left goes, one of the control it is on is on its way.
    pub(crate) fn tip_focus(&mut self, now: u64) {
        let focused = self.focused();
        self.tips.focus(focused, now);
    }

    /// A press of `owner`'s own button: its tooltip shows at once.
    pub(crate) fn tip_show(&mut self, owner: Action) {
        self.tips.show(owner);
    }

    /// Escape: the tooltip goes. Returns whether one showed.
    pub(crate) fn tip_dismiss(&mut self) -> bool {
        self.tips.dismiss()
    }

    /// Time moved on to `now` ms, the keyboard where it is: a tooltip
    /// whose wait is over shows, one timed out goes. A long press that
    /// brought one up lets go of its control, so the lift presses
    /// nothing. Returns whether what shows changed.
    pub(crate) fn tip_tick(&mut self, now: u64) -> bool {
        let focused = self.focused();
        let moved = self.tips.focus(focused, now);
        let changed = self.tips.tick(now);
        if changed && self.tips.shown_by() == Some(TipBy::Touch) {
            self.pressed = None;
            self.drag = None;
        }
        moved || changed
    }
}

/// Whether a press on `a` is on a text field: the caret goes where it
/// was pressed (`docs/history/NEW-WALLET.md` §13.2).
pub(crate) fn field_action(a: Action) -> bool {
    use crate::seeds;
    use crate::vault::VaultAction as V;
    matches!(
        a,
        Action::Vault(
            V::FocusPassphrase
                | V::CFocus(..)
                | V::CName
                | V::FocusField(_)
                | V::FocusPrompt
                | V::Dice(_)
        ) | Action::EntryPassphrase
            | Action::KPassField(_)
            | Action::CPassField(_)
            | Action::SeedRead(seeds::read::Choice::Field(_))
            | Action::Rename
            | Action::MType
            | Action::KTyping(true)
            | Action::Seeds(seeds::SeedsAction::Focus(_))
    )
}
