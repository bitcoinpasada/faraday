//! Where the pointer or finger is and what it has hold of: the controls
//! the last frame offered and where each takes a press, the one
//! pressed, the drag, the held scrollbar, the hover, the control the
//! keyboard is on, and the tooltip they bring up.

use faraday_draw::Rect;
use faraday_shell_api::Key as KeyIn;
use faraday_ui::tooltip::{TipAnchor, TipBy, Tips};

use crate::Action;
use crate::{Faraday, Screen, Sheet, motion, slow, ui, vault, work};
use faraday_shell_api::Command;
use faraday_shell_api::TouchPhase;

/// Where a control the last frame drew takes a press (`faraday_ui::Reach`).
pub(crate) type Reach = faraday_ui::Reach<Action>;

/// The pointer's state. One
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
    /// The controls the last frame drew past a side of their clip, each
    /// with its box uncut ([`faraday_ui::Ui::cut`]).
    cut: Vec<(Rect, Action)>,
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
            cut: Vec::new(),
        }
    }

    /// What the last frame offered to a press, in drawing order.
    pub(crate) fn hits(&self) -> &[(Rect, Action)] {
        &self.hits
    }

    /// Where each control the last frame drew takes a press, in drawing
    /// order.
    pub(crate) fn reach(&self) -> &[Reach] {
        &self.reach
    }

    /// The controls the last frame drew past a side of their clip.
    pub(crate) fn cut(&self) -> &[(Rect, Action)] {
        &self.cut
    }

    /// Keeps the controls a frame drew past a side of their clip.
    pub(crate) fn set_cut(&mut self, cut: Vec<(Rect, Action)>) {
        self.cut = cut;
    }

    /// Lets go of the buffers the last frames' controls were kept in, not
    /// only their contents: a buffer keeps what its entries once held
    /// past their end, and a screen that held a secret may have left some.
    pub(crate) fn forget_drawn(&mut self) {
        self.hits = Vec::new();
        self.cut = Vec::new();
        self.reach = Vec::new();
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
    /// controls in the order the last frame drew them, past every hold,
    /// which no key sets off; Enter or Space returns the control it is
    /// on, to be pressed.
    pub(crate) fn focus_key(&mut self, key: KeyIn) -> Option<Action> {
        match key {
            KeyIn::Tab | KeyIn::BackTab => {
                let order: Vec<Action> = self
                    .reach
                    .iter()
                    .map(|t| t.action)
                    .filter(|a| !crate::vault::is_hold(*a) && *a != Action::BootNoticeClose)
                    .collect();
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
/// was pressed.
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
                | V::FocusPromptAgain
                | V::Dice(_)
        ) | Action::EntryPassphrase
            | Action::EntryPassphraseAgain
            | Action::KPassField(_)
            | Action::CPassField(_)
            | Action::SeedRead(seeds::read::Choice::Field(_))
            | Action::KefRead(crate::seeds::kef::KefAct::Field)
            | Action::Rename
            | Action::KTyping(true)
            | Action::Seeds(seeds::SeedsAction::Focus(_))
            | Action::Vault(V::Search(crate::vault::SearchAct::Focus(_)))
            | Action::Vault(V::Roller(crate::fresh::roller::Act::Count))
            | Action::BName(_)
            | Action::RegisterField
            | Action::Sweep(seeds::sweep::SweepAction::Field)
            | Action::SetTimeFocus(_)
            | Action::TypeHere(_)
            | crate::edit::GPG_LINE
            | Action::PotentialType
            | Action::PotentialTypeAgain
            | Action::LPassphrase
    ) || a == crate::file_menu::FIELD
}

impl Faraday {
    /// The wheel, a trackpad or a finger starts at (x, y): the region
    /// under it is the one they move (the innermost, where one region
    /// stands inside another), the page's when none is.
    pub(crate) fn point_region(&mut self, x: i32, y: i32) {
        let slot = self
            .regions()
            .filter(|e| e.view.contains(x, y))
            .min_by_key(|e| i64::from(e.view.w) * i64::from(e.view.h))
            .map_or(ui::Slot::Page, |e| e.slot);
        self.scroll.went_to(slot);
    }

    /// Finds what can be pressed under the pointer in the frame on
    /// screen. Returns whether that changed.
    pub(crate) fn find_hovered(&mut self) -> bool {
        self.pointer.find_hovered()
    }

    pub(crate) fn touch(&mut self, x: u16, y: u16, phase: TouchPhase) {
        let (x, y) = (i32::from(x), i32::from(y));
        let found = self.pointer.hit_at(x, y);
        match phase {
            TouchPhase::Down => {
                self.pointer.set_down_at((x, y));
                self.pointer.tip_down((x, y), self.now_ms);
                self.point_region(x, y);
                // At the scrolled region's right edge a press takes the
                // scrollbar: held on its thumb it follows from where it was
                // taken, anywhere else on the track the thumb's middle
                // comes to the press.
                if let Some((e, g)) = self.bar_at(x, y) {
                    let offset = self.scroll_offset().unwrap_or(0.0);
                    let at = g.top
                        + ((g.track - g.thumb) as f32 * (offset / e.max).clamp(0.0, 1.0)).round()
                            as i32;
                    let grab = if (at..at + g.thumb).contains(&y) {
                        y - at
                    } else {
                        g.thumb / 2
                    };
                    self.pointer.set_bar_held(Some(grab));
                    self.scroll.stop();
                    self.pointer.set_pressed(None);
                    self.pointer.set_drag(None);
                    self.pointer.set_swallow(false);
                    self.bar_drag(y);
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                // A finger on a scrolled region may be the start of a
                // drag; it is a press until it moves. Not through a sheet
                // that does not scroll: the page under it is out of reach.
                let slot = self.slot();
                let dragging = self
                    .regions()
                    .find(|e| e.slot == slot && e.view.contains(x, y))
                    .map(|_| (y, false));
                self.pointer.set_drag(dragging);
                // A touch on content that is still flying only stops it.
                self.pointer.set_swallow(self.scroll.flung(self.f));
                if self.pointer.swallow() {
                    self.scroll.stop();
                    self.pointer.set_pressed(None);
                    return;
                }
                self.pointer.set_pressed(found);
                self.vaults.pressed(self.now_ms);
                // A press on a board chip may be its drag: no scroll (§19.2).
                if self.board_press_down(found.map(|f| f.0), x, y) {
                    self.pointer.set_drag(None);
                }
                if let Some((Action::VisitBar(c), _)) = found {
                    self.visit_drag(c, y);
                }
            }
            TouchPhase::Move => {
                self.pointer.tip_moved((x, y));
                if self.pointer.bar_held().is_some() {
                    self.bar_drag(y);
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                // A board chip past the slop follows the finger.
                if self.board_press_move(x, y) {
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                let held_bar = match self.pointer.pressed() {
                    Some((Action::VisitBar(c), _)) => Some(c),
                    _ => None,
                };
                // A slider held follows the finger along its track.
                if let Some((Action::Slide(id, _), r)) = self.pointer.pressed() {
                    let row = r.y + r.h / 2;
                    let under = self
                        .pointer
                        .hits()
                        .iter()
                        .rev()
                        .find(|(h, a)| {
                            matches!(a, Action::Slide(i, _) if *i == id) && h.contains(x, row)
                        })
                        .map(|(h, a)| (*a, *h));
                    if let Some((a, h)) = under
                        && Some(a) != self.pointer.pressed().map(|p| p.0)
                    {
                        self.pointer.set_pressed(Some((a, h)));
                        self.act(a);
                        self.out.redraw();
                        self.out.commands.push_back(Command::Draw);
                    }
                    return;
                }
                if let Some((last, dragging)) = self.pointer.drag()
                    && held_bar.is_none()
                {
                    let slop = motion::SLOP * self.f;
                    let (dx, dy) = (x - self.pointer.down_at().0, y - self.pointer.down_at().1);
                    let start = !dragging && self.drag_starts(dx, dy, slop);
                    if dragging || start {
                        // Past the slop the press is a drag: nothing
                        // under the finger is pressed, the page follows.
                        self.pointer.set_pressed(None);
                        self.pointer.set_drag(Some((y, true)));
                        self.pan_drag((x, y), last, start);
                        self.out.moved();
                        self.out.commands.push_back(Command::Draw);
                        return;
                    }
                }
                // The scrollbar follows the finger anywhere once held.
                if let Some(c) = held_bar {
                    self.visit_drag(c, y);
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                } else if let Some((_, r)) = self.pointer.pressed()
                    && !r.contains(x, y)
                {
                    self.pointer.set_pressed(None);
                }
            }
            TouchPhase::Up => {
                self.pointer.tip_up();
                if self.pointer.take_bar_held().is_some() {
                    return;
                }
                // A board chip dragged is dropped where it is lifted.
                if self.board_press_up(x, y) {
                    self.pointer.set_drag(None);
                    self.out.redraw();
                    self.out.commands.push_back(Command::Draw);
                    return;
                }
                let dragged = self.pointer.take_drag().is_some_and(|(_, d)| d);
                if dragged {
                    self.pointer.set_pressed(None);
                    self.scroll.release(self.f);
                }
                if dragged || self.pointer.take_swallow() {
                    return;
                }
                let pressed = self.pointer.take_pressed();
                // A hold acts on the tick that completes it, never on release.
                if let (Some((a, _)), Some((b, _))) = (pressed, found)
                    && a == b
                    && !vault::is_hold(a)
                {
                    // On a field: the caret where it was pressed, a
                    // second press selecting all of it, a drag what it
                    // went over.
                    let double = self
                        .pointer
                        .last_tap()
                        .is_some_and(|(l, t)| l == a && self.now_ms.saturating_sub(t) <= 450);
                    if field_action(a)
                        && let Some(dragged) =
                            self.field_press(a, self.pointer.down_at(), (x, y), double)
                    {
                        self.pointer
                            .set_last_tap((!double && !dragged).then_some((a, self.now_ms)));
                        return;
                    }
                    self.pointer.set_last_tap(Some((a, self.now_ms)));
                    // A press on the screen's way out is Back (F77).
                    let a = self.as_back_at(a, found.map(|f| f.1));
                    self.press_in(a);
                    if field_action(a) && !matches!(a, Action::BName(_)) {
                        self.field_press(a, (x, y), (x, y), false);
                    }
                }
            }
        }
    }

    /// The overlay scrollbar a press at (x, y) would take: the scrolled
    /// region's, when the press is within [`ui::BAR_GRAB`] of its right
    /// edge and the region scrolls, with where its bar runs. A small
    /// panel draws none, so a press there is on the content.
    pub(crate) fn bar_at(&self, x: i32, y: i32) -> Option<(ui::Scrolled, ui::BarGeometry)> {
        if self.compact {
            return None;
        }
        // Not through a sheet that does not scroll.
        let e = self.regions().find(|e| e.view.contains(x, y))?;
        if e.own_bar || e.max <= 0.0 {
            return None;
        }
        let f = self.f.max(0.1);
        if x < e.view.right() - (ui::BAR_GRAB * f).round() as i32 {
            return None;
        }
        let g = ui::BarGeometry::of(e.view, e.max, f);
        (g.track > g.thumb).then_some((e, g))
    }

    /// The scrolled region moved to where its held scrollbar is, at pixel
    /// row `y`.
    fn bar_drag(&mut self, y: i32) {
        let Some(grab) = self.pointer.bar_held() else {
            return;
        };
        let Some(e) = self.region_now() else {
            return;
        };
        let g = ui::BarGeometry::of(e.view, e.max, self.f.max(0.1));
        let room = (g.track - g.thumb).max(1) as f32;
        let at = ((y - g.top - grab) as f32).clamp(0.0, room);
        self.set_scroll_offset(at / room * e.max);
    }

    /// Brings whatever is moving to rest at once: a glide lands, a
    /// stretch ends, a toast is fully up, the overlay scrollbar is gone
    /// and a text caret shows. For drivers that take pictures of the screens,
    /// such as the snapshot tool, so a picture shows where things end up.
    pub fn settle(&mut self) {
        self.typing.blink_from(self.now_ms);
        self.settle_regions();
        self.scroll.stop();
        self.pointer.set_scroll_at(None);
        if let Some(d) = self.disclosed.as_mut() {
            d.moving = false;
        }
        self.fresh.own_settle();
        let whole = self.whole_offset();
        let offset = self.scroll_offset().map_or(0.0, whole);
        self.scroll
            .set_bar_seen((self.region_key(), self.slot()), offset, 0);
        if self.out.toast.is_some() {
            self.out.toast_at = Some(self.now_ms.saturating_sub(motion::TOAST_MS));
        }
        // The next frame is the screen at rest, not on the way from the last.
        self.fade.restart();
        self.out.redraw();
    }

    /// Acts as if `action` had been pressed: for drivers that script the
    /// screens, such as the snapshot tool.
    pub fn press(&mut self, action: Action) {
        self.press_as(action, true);
    }

    /// [`Faraday::press`], its jobs run inline or, as a person's press
    /// has them, on the worker.
    pub(crate) fn press_as(&mut self, action: Action, inline: bool) {
        // An action may change the words typed: the status line's key is
        // worked out again, once typing pauses when the action is a
        // character from the on-screen keyboard.
        self.entry.state().forget_probe();
        if matches!(
            action,
            Action::EntryKey(_) | Action::EntryKeyBack | Action::EntryCandidate(_)
        ) && self.entry_typing_here()
        {
            self.inbox.set_typed_at(Some(self.now_ms));
        }
        self.note_secrets();
        self.note_here();
        // A script does not wait for the worker either: its jobs run here.
        let was_inline = self.worker.set_inline(inline);
        self.press_in(action);
        // A script does not wait for frames: what a press leaves for the tick is done now.
        self.later_all(inline);
        self.worker.set_inline(was_inline);
        self.backup_settle();
        self.after_input();
        self.out.redraw();
        self.out.commands.push_back(Command::Draw);
    }

    /// A person's press: done now, whatever its work (`docs/WORKERS.md`
    /// §2.1: under the line it runs here, over it it starts a job). A
    /// vault stretch or key left waiting goes first.
    pub(crate) fn press_in(&mut self, action: Action) {
        // A slider moved again replaces the files it asked to be made.
        if !(matches!(action, Action::Slide(..)) && self.backup.omit_remake().is_some()) {
            self.later_now();
        }
        let before = work::snapshot();
        self.act(action);
        // The rule `slow` keeps, held in debug builds and every test: a
        // press on neither list stretches no key and makes at most one
        // picture.
        let done = work::since(&before);
        slow::guard(action, &done);
        // A slider let go starts the files it changed.
        self.omit_remake_tick();
    }

    /// Whether New key, opened from tool `s` to add the key it needs, is
    /// on screen: the tool keeps what it held for the way back.
    pub(crate) fn adding_key_for(&self, s: Screen) -> bool {
        matches!(self.screen, Screen::KeyGen | Screen::Randomness)
            && self.keygen.state().is_some_and(|k| k.back == s)
    }

    /// What every press does once it has acted, however it came (a tap,
    /// a key, the mouse's back button, a scripted press): a screen left
    /// drops what it held, a detour not taken ends, a spent round goes
    /// from its vault, and a sheet waiting comes up. Lookups, unless
    /// something is dropped.
    pub(crate) fn after_input(&mut self) {
        self.stick.note_visit(self.stick_pane());
        vault::settle(self);
        self.trail_note();
        // A vault being made or unlocked for another flow: the way back
        // ends when the person leaves for anything else, and a secret
        // kept for it goes, wiped. While on the way, the flow keeps what
        // it held.
        let detour = matches!(
            self.screen,
            Screen::CreateVault | Screen::Vaults | Screen::Unlock | Screen::BackupFlow
        );
        if !detour {
            self.vaults.set_back_to(None);
        }
        let waiting = detour && self.vaults.back_to.is_some();
        if !waiting && !matches!(self.sheet, Some(Sheet::SecretOut | Sheet::Pull)) {
            self.secret_cancel();
        }
        let kept = |s: Screen| {
            self.screen == s
                || (waiting && self.vaults.back_to == Some(s))
                || self.adding_key_for(s)
        };
        let (bip85, lightning, silent, vanity) = (
            kept(Screen::Bip85),
            kept(Screen::Lightning),
            kept(Screen::Silent),
            kept(Screen::Vanity),
        );
        // A vault's passphrase, typed or half typed, goes once nothing that
        // takes it is up, however it was left; an unlock under way keeps it
        // until it is done.
        let typing = matches!(
            self.screen,
            Screen::Unlock | Screen::BackupFlow | Screen::SpendBackup
        ) || self.sheet == Some(Sheet::Import);
        if !typing && self.vaults.working != Some(vault::Work::Unlock) {
            self.vaults.clear_unlock();
        }
        // Leaving New key drops what it held (Add randomness, opened from it, keeps it).
        // (The making on Make a new wallet's Keys card stays while Create
        // shows: `keygen_card_tidy` ends it.)
        if !matches!(self.screen, Screen::KeyGen | Screen::Randomness)
            && self.keygen.state().is_some_and(|k| !k.on_card)
        {
            self.keygen_drop();
        }
        if !bip85 {
            self.bip85.clear();
        }
        self.shown_tidy();
        self.tool_tidy();
        let (screen, wallet) = (self.screen, self.wallets.wallet());
        self.tools
            .tidy(screen, |i| screen == Screen::Wallets && wallet == i);
        if self.sheet != Some(Sheet::WordList) {
            self.wordlist.clear();
        }
        // Leaving the vanity screen stops a search and drops a find.
        if !vanity {
            self.vanity.clear();
        }
        if !lightning && self.sheet != Some(Sheet::SecretOut) {
            self.lightning.clear();
        }
        if !silent && self.sheet != Some(Sheet::Qr) {
            self.silent.clear();
        }
        self.cameras_noise();
        self.keygen_device();
        self.release_held_stick();
        // A device still waiting comes back up once no other sheet is.
        self.input_sheet();
        self.device_sheet();
        self.drop_forged_wallets();
    }

    /// Whether the last frame drawn offers `action` to a press: for
    /// tests that a control is on screen.
    pub fn offers(&self, action: Action) -> bool {
        self.pointer.offers(action)
    }

    /// Where the last frame drawn offers `action`, in pixels: the middle
    /// of where it can be pressed, for tests that touch it.
    pub fn where_offered(&self, action: Action) -> Option<(u16, u16)> {
        let r = self.pointer.rect_of(action)?;
        Some(((r.x + r.w / 2) as u16, (r.y + r.h / 2) as u16))
    }

    /// Every control the last frame drawn offers, for the tours' check of
    /// what a press opened (`docs/SECRET-LEAKS.md` §5).
    #[cfg(feature = "testkit")]
    pub fn offered_actions(&self) -> Vec<Action> {
        self.pointer.hits().iter().map(|(_, a)| *a).collect()
    }

    /// The box the last frame drawn offers `action` in, in pixels
    /// (x, y, w, h): for a test that two controls do not overlap.
    pub fn hit_box(&self, action: Action) -> Option<(u16, u16, u16, u16)> {
        let r = self.pointer.rect_of(action)?;
        Some((r.x as u16, r.y as u16, r.w as u16, r.h as u16))
    }
}
