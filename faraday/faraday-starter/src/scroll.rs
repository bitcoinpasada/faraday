//! The page and its sheet under a finger, the mouse and the wheel
//! (`docs/STARTER.md` §3.1d): a press is a press until it moves past the
//! slop, when the page, or the sheet open over it, follows the finger and
//! nothing under it is pressed; lifted, it coasts and slows, and springs
//! back from an end, as Faraday's own pages do (`faraday_core::motion`,
//! `docs/MOTION.md` §3.3). A touch on content still coasting only stops
//! it. The motion asks for frames only while it moves (`docs/SPEED.md`
//! §12). An open sheet scrolls by its own offset, from its top each time
//! it opens, so the page under it keeps its place and its frosted copy.
//! A hold (`docs/DESIGN.md`'s hold, F136) acts on the tick that has seen
//! it held to the end, never on release: Continue anyway's and keep
//! data's write over a newer stick.
//! A page whose content changes in place keeps what the person last
//! pressed on it where it was on screen (or, where that is gone, the
//! nearest control drawn before it, so that what followed moves up into
//! its place), within the page's new length; only a new page opens at
//! its top.

use faraday_core::Action;
use faraday_core::motion::{Motion, Region, SLOP};
use faraday_draw::Rect;
use faraday_draw::tokens::HOLD_MS;
use faraday_shell_api::TouchPhase;

use crate::{Page, Sheet, Starter, StarterAction, draw};

/// Faraday's drawing, a press being the desktop app's action.
type Ui<'a> = faraday_ui::Ui<'a, Action>;

/// The scrolled region's state between events: one field of [`Starter`].
#[derive(Debug, Default)]
pub(crate) struct Scroller {
    /// How the page or the sheet moves under a finger and after it.
    motion: Motion,
    /// How far the open sheet is scrolled, units, and the furthest it
    /// scrolls, as last drawn.
    sheet_offset: f32,
    sheet_max: f32,
    /// The sheet the last frame drew: a new one opens at its top.
    shown_sheet: Option<Sheet>,
    /// Where the region that scrolls now stood in the last frame, in
    /// pixels: the page's, or the open sheet's.
    view: Option<Rect>,
    /// What the motion moves: the page shown, the sheet over it and
    /// whether the held app is shown. Any change stops it.
    on: Option<(Page, Option<Sheet>, bool)>,
    /// A press on the region: where it went down, the row it was last
    /// at, and whether it has moved past the slop, so that it scrolls and
    /// presses nothing.
    drag: Option<((i32, i32), i32, bool)>,
    /// The press under way only stopped a coast, and does nothing else.
    swallow: bool,
    /// The last tick's time, ms.
    now_ms: u64,
    /// When the hold pressed now went down, ms.
    held_since: Option<u64>,
    /// What was last pressed on the page, until the page is scrolled or
    /// left.
    anchor: Option<Anchor>,
}

/// A control on the page: what it does, and its top and its foot down
/// the window, units; an edge the view cut off is not known.
type Mark = (Action, Option<f32>, Option<f32>);

/// The control last pressed on a page and those drawn before it in
/// view, as they stood then: the pressed one first, then the others,
/// nearest first. With none of them drawn any more, the page keeps its
/// scroll.
#[derive(Debug)]
struct Anchor {
    page: Page,
    marks: Vec<Mark>,
}

impl Starter {
    /// How far what scrolls now is scrolled, units: the open sheet, or
    /// the page. A lookup.
    pub fn scrolled(&self) -> f32 {
        if self.sheet.is_some() {
            self.scroller.sheet_offset
        } else {
            self.offset
        }
    }

    /// How far the open sheet is scrolled, units: what drawing it reads.
    /// A lookup.
    pub(crate) fn sheet_offset(&self) -> f32 {
        self.scroller.sheet_offset
    }

    /// What a press at (x, y), in pixels, does as the last frame drew
    /// it, for tests that touch it. A pass over the frame's presses.
    pub fn offered_at(&self, x: i32, y: i32) -> Option<StarterAction> {
        match self.hit_at(x, y)? {
            Action::Host(code) => StarterAction::from_code(code),
            _ => None,
        }
    }

    /// What the motion moves now. A lookup.
    fn place(&self) -> (Page, Option<Sheet>, bool) {
        (self.page, self.sheet, self.showing)
    }

    /// Runs `go` on the region that scrolls now: the open sheet, or the
    /// page.
    fn region<R>(&mut self, go: impl FnOnce(&mut Motion, &mut Region) -> R) -> R {
        let f = self.f.max(0.1);
        let s = &mut self.scroller;
        let view = s.view.map_or(self.h, |r| r.h as f32 / f);
        let (offset, max) = if self.sheet.is_some() {
            (&mut s.sheet_offset, s.sheet_max)
        } else {
            (&mut self.offset, self.max)
        };
        let mut region = Region {
            offset,
            max: Some(max),
            view,
            f,
        };
        go(&mut s.motion, &mut region)
    }

    /// The page moved: its frosted copy under a sheet is no longer it
    /// (`Starter::frost`). A sheet's own scroll keeps it.
    fn page_moved(&mut self) {
        if self.sheet.is_none() {
            self.frost = None;
        }
    }

    /// The wheel or two fingers moved what scrolls now by `dy` pixels.
    /// Returns whether it moved: at an end, a turn further moves nothing.
    pub(crate) fn wheel(&mut self, dy: i16) -> bool {
        self.scroller.motion.stop();
        self.scroller.anchor = None;
        let step = f32::from(dy) / self.f.max(0.1);
        let moved = self.region(|_, r| {
            let to = (*r.offset + step).clamp(0.0, r.max.unwrap_or(0.0).max(0.0));
            let moved = to != *r.offset;
            *r.offset = to;
            moved
        });
        if moved {
            self.page_moved();
        }
        moved
    }

    /// Whether `action` acts only when held to the end (F136): Continue
    /// anyway's answer, and Write on Confirm when keep data writes an
    /// older release over a newer stick. A lookup.
    pub(crate) fn is_hold(&self, action: StarterAction) -> bool {
        match action {
            StarterAction::LetThrough => true,
            StarterAction::Write => self.writes_older(),
            _ => false,
        }
    }

    /// The hold pressed now, if what is pressed is one. A lookup.
    fn pressed_hold(&self) -> Option<StarterAction> {
        match self.pressed {
            Some(Action::Host(code)) => StarterAction::from_code(code).filter(|a| self.is_hold(*a)),
            _ => None,
        }
    }

    /// How far `action`'s hold has been held, 0 to 1: what its button
    /// fills. A lookup.
    pub(crate) fn held_part(&self, action: StarterAction) -> f32 {
        match self.scroller.held_since {
            Some(since) if self.pressed_hold() == Some(action) => {
                self.scroller.now_ms.saturating_sub(since).min(HOLD_MS) as f32 / HOLD_MS as f32
            }
            _ => 0.0,
        }
    }

    /// A tick for the hold pressed now: done once held to the end.
    /// Returns whether a frame is wanted for it.
    fn hold_tick(&mut self, now_ms: u64) -> bool {
        let Some(since) = self.scroller.held_since else {
            return false;
        };
        let Some(action) = self.pressed_hold() else {
            self.scroller.held_since = None;
            return true;
        };
        if now_ms.saturating_sub(since) >= HOLD_MS {
            self.scroller.held_since = None;
            self.pressed = None;
            self.act(action);
        }
        true
    }

    /// A tick: what coasts or springs back moves on, and a hold fills.
    /// Returns whether a frame is wanted for it.
    pub(crate) fn scroll_tick(&mut self, now_ms: u64) -> bool {
        self.scroller.now_ms = now_ms;
        if self.hold_tick(now_ms) {
            return true;
        }
        if self.scroller.on.is_some_and(|on| on != self.place()) {
            self.scroller.motion.stop();
            self.scroller.on = None;
        }
        let moved = self.region(|m, r| m.tick(r, now_ms));
        if moved {
            self.page_moved();
        }
        moved
    }

    /// A contact at (x, y): whether the frame changes.
    pub(crate) fn touch(&mut self, x: i32, y: i32, phase: TouchPhase) -> bool {
        let f = self.f.max(0.1);
        match phase {
            TouchPhase::Down => {
                let s = &mut self.scroller;
                s.drag = None;
                s.swallow = false;
                if !self.menu && s.view.is_some_and(|r| r.contains(x, y)) {
                    if s.motion.flung(f) {
                        // A touch on content still flying only stops it.
                        s.motion.stop();
                        s.swallow = true;
                        self.pressed = None;
                        return true;
                    }
                    s.drag = Some(((x, y), y, false));
                }
                self.pressed = self.hit_at(x, y);
                let now = self.scroller.now_ms;
                self.scroller.held_since = self.pressed_hold().map(|_| now);
            }
            TouchPhase::Move => {
                if let Some(((x0, y0), last, dragging)) = self.scroller.drag {
                    let (dx, dy) = (x - x0, y - y0);
                    if dragging || ((dy.abs() as f32) > SLOP * f && dy.abs() > dx.abs()) {
                        // Past the slop the press is a drag: nothing under
                        // the finger is pressed, the page follows.
                        self.pressed = None;
                        self.scroller.anchor = None;
                        self.scroller.drag = Some(((x0, y0), y, true));
                        self.scroller.on = Some(self.place());
                        let dy = (last - y) as f32 / f;
                        self.region(|m, r| m.pan(r, dy));
                        self.page_moved();
                        return true;
                    }
                }
                // A move that keeps the press as it was changes nothing.
                if self.pressed.is_none() || self.hit_at(x, y) == self.pressed {
                    return false;
                }
                self.pressed = None;
            }
            TouchPhase::Up => {
                if self.scroller.drag.take().is_some_and(|d| d.2) {
                    self.pressed = None;
                    self.scroller.motion.release(f);
                    return true;
                }
                if std::mem::take(&mut self.scroller.swallow) {
                    return false;
                }
                let pressed = self.pressed.take();
                self.scroller.held_since = None;
                // A hold let go before its end does nothing.
                if let Some(a) = pressed
                    && self.hit_at(x, y) == Some(a)
                    && let Action::Host(code) = a
                    && let Some(action) = StarterAction::from_code(code)
                    && !self.is_hold(action)
                {
                    self.mark(a);
                    self.act(action);
                    if action == StarterAction::Name {
                        self.name_pressed(action, Some(x));
                    }
                }
            }
        }
        true
    }

    /// Costs the frame's drawing. Draws the frame ([`draw::frame`]) with
    /// what scrolls stretched where a finger pulls it past an end, a new
    /// sheet from its top, and notes where the region that scrolls now
    /// stands and how far the sheet goes. Returns how far the page can
    /// scroll, units.
    fn frame_scrolled(&mut self, ui: &mut Ui) -> f32 {
        if self.sheet != self.scroller.shown_sheet {
            self.scroller.shown_sheet = self.sheet;
            self.scroller.sheet_offset = 0.0;
        }
        ui.stretch = (self.scroller.motion.stretch() * ui.f).round() as i32;
        ui.stretch_in_sheet = self.sheet.is_some();
        let max = draw::frame(self, ui);
        // The sheet's region stands in for the page's while it is open.
        let region = ui
            .scrolled
            .iter()
            .find(|r| r.slot == faraday_ui::Slot::Page)
            .map(|r| (r.view, r.max));
        let s = &mut self.scroller;
        s.view = region.map(|r| r.0).filter(|_| !self.showing);
        if self.sheet.is_some()
            && let Some((_, m)) = region
        {
            s.sheet_max = m;
            s.sheet_offset = s.sheet_offset.min(m);
        }
        max
    }

    /// Costs the frame's drawing, twice where the page changed under
    /// what was last pressed on it. Draws the frame, then, where the
    /// page's content moved what was last pressed (or the control
    /// before it, where it is gone) or the page grew shorter than how far
    /// it is scrolled, draws it again scrolled to keep that control where
    /// it was, within the page's new length.
    pub(crate) fn render(&mut self) {
        let Some(mut canvas) = self.canvas.take() else {
            return;
        };
        let mut hits = std::mem::take(&mut self.hits);
        let mut reach = Vec::new();
        self.offset = self.offset.clamp(0.0, self.max.max(0.0));
        for again in [false, true] {
            hits.clear();
            let mut ui = faraday_ui::Ui::new(&mut canvas, self.f, &mut hits, self.pressed);
            ui.theme = self.theme;
            ui.set_text_size(self.text);
            ui.hovered = self.hovered;
            ui.compact = self.compact();
            ui.primaries = 0;
            ui.offset = self.offset;
            ui.frost = self.sheet.and(self.frost.take());
            ui.edit = self.name_edit();
            self.max = self.frame_scrolled(&mut ui);
            self.fields = std::mem::take(&mut ui.fields);
            self.frost = ui.frost.take();
            self.primaries = ui.primaries;
            reach = ui.take_presses();
            match self.kept_at(&hits) {
                Some(to) if !again => self.offset = to,
                _ => break,
            }
        }
        self.hits = hits;
        self.reach = reach;
        self.canvas = Some(canvas);
    }

    /// Notes `pressed`, pressed on the page, and the controls drawn
    /// before it in view, where they stand: what a change to the page keeps
    /// in place. A pass over the frame's presses.
    fn mark(&mut self, pressed: Action) {
        self.scroller.anchor = None;
        let Some(view) = self.page_view() else {
            return;
        };
        let marks = self.marks(&self.hits, view);
        let Some(i) = marks.iter().position(|m| m.0 == pressed) else {
            return;
        };
        let mut order = vec![marks[i]];
        order.extend(marks[..i].iter().rev());
        self.scroller.anchor = Some(Anchor {
            page: self.page,
            marks: order,
        });
    }

    /// Where the page shows, pixels, while it alone is drawn and still: no
    /// sheet, menu or held app over it, nothing coasting. A lookup.
    fn page_view(&self) -> Option<Rect> {
        let still = !self.scroller.motion.moving() && self.scroller.drag.is_none();
        let alone = self.sheet.is_none() && !self.menu && !self.showing && !self.showing_demo();
        self.scroller.view.filter(|_| still && alone)
    }

    /// The page's controls in `hits` that show in `view`, each with its
    /// edges down the window. A pass over the frame's presses.
    fn marks(&self, hits: &[(Rect, Action)], view: Rect) -> Vec<Mark> {
        let f = self.f.max(0.1);
        let at = |px: i32| px as f32 / f;
        hits.iter()
            .filter(|(r, _)| !r.intersect(&view).is_empty())
            .map(|(r, a)| {
                let top = (r.y > view.y).then(|| at(r.y));
                let foot = (r.bottom() < view.bottom()).then(|| at(r.bottom()));
                (*a, top, foot)
            })
            .collect()
    }

    /// How far the page is to be scrolled after the frame just drawn
    /// into `hits`, when not where it is: moved by as far as the first of
    /// the anchor's controls still drawn has moved on screen, within the
    /// page's length. A pass over the frame's presses.
    fn kept_at(&mut self, hits: &[(Rect, Action)]) -> Option<f32> {
        if let Some(a) = &self.scroller.anchor
            && a.page != self.page
        {
            self.scroller.anchor = None;
        }
        let view = self.page_view()?;
        let now = self.marks(hits, view);
        let moved = self.scroller.anchor.as_ref().and_then(|a| {
            a.marks.iter().find_map(|(act, top, foot)| {
                let (_, t, b) = now.iter().find(|m| m.0 == *act)?;
                match (top, t, foot, b) {
                    (Some(was), Some(is), ..) | (_, _, Some(was), Some(is)) => Some(is - was),
                    _ => None,
                }
            })
        });
        let to = (self.offset + moved.unwrap_or(0.0)).clamp(0.0, self.max.max(0.0));
        ((to - self.offset).abs() * self.f >= 0.5).then_some(to)
    }
}
