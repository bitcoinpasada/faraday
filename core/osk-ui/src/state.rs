//! Interaction state that persists across frames, and the event-to-action
//! translation.
//!
//! The application owns its data; [`UiState`] owns only what the widgets
//! need to feel right between two frames: which target is pressed, how far
//! a hold has progressed, scroll offsets, keyboard modifiers, pager pages.
//! Feed it every shell [`Event`] together with the current [`Layout`] and
//! it returns at most one [`Action`].

use osk_shell_api::{ButtonId, Event, Key, TouchPhase};

use crate::geom::{Dp, Rect};
use crate::layout::{Id, Layout, ScrollOffsets};
use crate::widgets::keyboard::{self, KeyInput, KeyboardKind, Modifiers};
use crate::widgets::{HitTarget, tokens};

/// What the user did, in widget terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A button, tile, chip, toggle or list row was tapped.
    Tap(Id),
    /// A hold button was held for the full duration.
    HoldCompleted(Id),
    /// A scroll region moved.
    Scrolled(Id),
    /// An on-screen keyboard (or a physical key routed to it) produced
    /// input.
    KeyboardInput(Id, KeyInput),
    /// A candidate in a strip was tapped.
    Candidate(Id, u8),
    /// A paged chunked string advanced to `page`.
    PageChanged(Id, u8),
    /// The back gesture: physical Back button or Escape.
    Back,
    /// Something visual changed (hold progress, modifier toggle); redraw.
    Redraw,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    target: HitTarget,
    x: i32,
    y: i32,
    moved: bool,
    scroll: Option<Id>,
    /// The key under the press point on a keyboard, found when the
    /// finger went down as the release finds it. `None` on a dead key
    /// and on every other target.
    key: Option<KeyInput>,
    /// The candidate cell under the press point on a strip, found the
    /// same way. `None` on a cell that carries no word.
    cell: Option<u8>,
}

/// How strongly a press is drawn (`docs/PLANNING.md` §16.124): at full
/// strength while the finger is down and for
/// [`tokens::PRESS_LINGER_FULL_MS`] after it lifts, at half strength for
/// the rest of [`tokens::PRESS_LINGER_MS`], and not at all after that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressLook {
    /// The whole of the mix.
    Full,
    /// Half of it: the after-image on its way out.
    Half,
}

impl PressLook {
    /// How much of `fraction` a surface mixes at this strength.
    pub fn of(self, fraction: f32) -> f32 {
        match self {
            PressLook::Full => fraction,
            PressLook::Half => fraction / 2.0,
        }
    }
}

/// Cross-frame interaction state.
#[derive(Debug, Default)]
pub struct UiState {
    now_ms: u64,
    press: Option<Press>,
    /// A press that started on a scroll region but not on a widget.
    drag: Option<(Id, i32, i32, bool)>,
    hold: Option<(Id, u64)>,
    /// The press a finger has lifted and the moment it lifted: its
    /// widget keeps the pressed look until the window closes
    /// (`docs/PLANNING.md` §16.120).
    linger: Option<(Press, u64)>,
    /// The newest entry of a masked run and the moment it arrived
    /// (`docs/DESIGN.md` §4.3).
    flash: Option<(Id, u64)>,
    scroll: ScrollOffsets,
    mods: Modifiers,
    /// The item a key last moved focus to (`docs/DESIGN.md` §4.15).
    focus: Option<Id>,
    /// Whether the focus ring is on screen: a key moving focus shows it,
    /// a touch takes it away.
    focus_visible: bool,
    pages: alloc::vec::Vec<(Id, u8)>,
    /// Touch slop in pixels; a move beyond it cancels the tap.
    slop: i32,
    /// Pixels per Up/Down button or arrow press.
    line: i32,
    /// Whether the event being handled changed anything a widget draws
    /// besides a scroll offset.
    scroll_only: bool,
}

impl UiState {
    /// A fresh state for a display at `scale`.
    pub fn new(scale: crate::geom::Scale) -> Self {
        UiState {
            slop: scale.px(Dp(8.0)),
            line: scale.px(Dp(48.0)),
            ..Default::default()
        }
    }

    /// Whether the [`Action::Scrolled`] just reported is all the last
    /// event did, so the frame it asks for is the frame on the panel
    /// with the region's pixels moved. False when the same event also
    /// cancelled a press or a hold, whose widgets are drawn differently
    /// now.
    pub fn scroll_only(&self) -> bool {
        self.scroll_only
    }

    /// Scroll offsets, for [`crate::layout::solve_with`].
    pub fn scroll_offsets(&self) -> &ScrollOffsets {
        &self.scroll
    }

    /// Scrolls `id` to `offset` pixels (clamped at the next solve).
    pub fn set_scroll(&mut self, id: Id, offset: i32) {
        self.scroll.set(id, offset.max(0));
    }

    /// Keyboard modifiers.
    pub fn modifiers(&self) -> Modifiers {
        self.mods
    }

    /// Sets the keyboard modifiers. The address keyboard's layer
    /// follows the characters typed — `bc1` is bech32, anything else is
    /// base58 — so the application sets it as the address grows.
    pub fn set_modifiers(&mut self, mods: Modifiers) {
        self.mods = mods;
    }

    /// The press whose look is on the screen and how strongly it is
    /// drawn: the finger that is down and has not moved past the slop,
    /// or the one that lifted less than [`tokens::PRESS_LINGER_MS`] ago.
    fn drawn(&self) -> Option<(&Press, PressLook)> {
        match &self.press {
            Some(p) if !p.moved => Some((p, PressLook::Full)),
            Some(_) => None,
            None => self.lingering(),
        }
    }

    /// The lingering press and the strength it is drawn at, while its
    /// window is open: the whole mix for the first
    /// [`tokens::PRESS_LINGER_FULL_MS`] after the lift, half of it for
    /// the rest of [`tokens::PRESS_LINGER_MS`].
    fn lingering(&self) -> Option<(&Press, PressLook)> {
        let (press, at) = self.linger.as_ref()?;
        match self.now_ms.saturating_sub(*at) {
            since if since < tokens::PRESS_LINGER_FULL_MS => Some((press, PressLook::Full)),
            since if since < tokens::PRESS_LINGER_MS => Some((press, PressLook::Half)),
            _ => None,
        }
    }

    /// How strongly `id` is drawn pressed, if it is: a finger is on it,
    /// or one lifted from it within the last
    /// [`tokens::PRESS_LINGER_MS`].
    pub fn pressed(&self, id: Id) -> Option<PressLook> {
        match self.drawn() {
            Some((
                Press {
                    target: HitTarget::Tap(t),
                    ..
                },
                look,
            )) if *t == id => Some(look),
            _ => None,
        }
    }

    /// The key of keyboard `id` drawn pressed and how strongly, if any.
    pub fn pressed_key(&self, id: Id) -> Option<(KeyInput, PressLook)> {
        match self.drawn() {
            Some((
                Press {
                    target: HitTarget::Keyboard { id: t, .. },
                    key,
                    ..
                },
                look,
            )) if *t == id => key.map(|k| (k, look)),
            _ => None,
        }
    }

    /// The cell of candidate strip `id` drawn pressed and how strongly,
    /// if any.
    pub fn pressed_cell(&self, id: Id) -> Option<(u8, PressLook)> {
        match self.drawn() {
            Some((
                Press {
                    target: HitTarget::Candidates { id: t, .. },
                    cell,
                    ..
                },
                look,
            )) if *t == id => cell.map(|c| (c, look)),
            _ => None,
        }
    }

    /// Drops the pressed look a release left behind. A screen change
    /// calls this, since ids repeat from screen to screen and a linger
    /// belongs to the screen it was pressed on.
    pub fn clear_linger(&mut self) {
        self.linger = None;
    }

    /// Whether `id` is under a finger right now, from the press until
    /// the release: a secret panel's surface, or a hold button,
    /// including after its hold has completed. A secret shows exactly
    /// while this is true (`docs/PLANNING.md` §4.6: re-mask on release).
    pub fn is_held(&self, id: Id) -> bool {
        matches!(
            self.press,
            Some(Press {
                target: HitTarget::Hold(t) | HitTarget::Reveal(t),
                moved: false,
                ..
            }) if t == id
        )
    }

    /// Marks the moment a die or a coin was entered into the run `id`.
    ///
    /// §4.3: "the newest entry shows for half a second, then masks."
    /// The window is the state's, not the caller's, so every pad in
    /// every flow shows an entry for the same time.
    pub fn flash(&mut self, id: Id) {
        self.flash = Some((id, self.now_ms));
    }

    /// Whether the newest entry of `id` is still inside its window.
    pub fn flashing(&self, id: Id) -> bool {
        matches!(self.flash, Some((f, at)) if f == id
            && self.now_ms.saturating_sub(at) < tokens::REVEAL_FLASH_MS)
    }

    /// Hold progress of `id` in `0..=1`; zero when not held.
    pub fn hold_progress(&self, id: Id) -> f32 {
        match self.hold {
            Some((h, start)) if h == id => {
                let elapsed = self.now_ms.saturating_sub(start);
                (elapsed as f32 / tokens::HOLD_MS as f32).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }

    /// Whether anything is animating — a hold in progress, an entry
    /// still inside its reveal window — so the application should keep
    /// redrawing on ticks.
    pub fn animating(&self) -> bool {
        self.hold.is_some()
            || self.lingering().is_some()
            || matches!(self.flash, Some((id, _)) if self.flashing(id))
    }

    /// Current page of a paged chunked string.
    pub fn page(&self, id: Id) -> u8 {
        self.pages
            .iter()
            .find(|(i, _)| *i == id)
            .map_or(0, |(_, p)| *p)
    }

    /// Sets the page of a chunked string.
    pub fn set_page(&mut self, id: Id, page: u8) {
        match self.pages.iter_mut().find(|(i, _)| *i == id) {
            Some(e) => e.1 = page,
            None => self.pages.push((id, page)),
        }
    }

    /// The item the focus ring is on, if a key has moved focus and no
    /// touch has taken the ring away since.
    pub fn focused(&self) -> Option<Id> {
        self.focus.filter(|_| self.focus_visible)
    }

    /// Whether the focus ring belongs on `id`.
    pub fn is_focused(&self, id: Id) -> bool {
        self.focused() == Some(id)
    }

    /// Drops a focus that `layout` has no stop for, which is what a
    /// screen change does: the application calls this with the layout it
    /// is about to draw, so the ring is never left on an id another
    /// screen happens to use.
    pub fn retain_focus(&mut self, layout: &Layout) {
        if let Some(id) = self.focus
            && !layout.items.iter().any(|p| p.focus == Some(id))
        {
            self.focus = None;
            self.focus_visible = false;
        }
    }

    /// Feeds one event. `layout` must be the layout the frame on screen was
    /// drawn from.
    pub fn event(&mut self, layout: &Layout, event: Event) -> Option<Action> {
        self.scroll_only = true;
        match event {
            Event::Display(_)
            | Event::File { .. }
            | Event::FileUnavailable { .. }
            | Event::FileCancelled { .. }
            | Event::FileList { .. }
            | Event::FileWritten { .. }
            | Event::FileNotWritten { .. }
            | Event::Clipboard { .. }
            | Event::ClipboardUnavailable { .. }
            | Event::ClipboardWritten { .. }
            | Event::ClipboardNotWritten { .. }
            | Event::CameraFrame { .. }
            | Event::Scanned { .. }
            | Event::CameraUnavailable
            | Event::Entropy(_)
            | Event::Lock
            | Event::Settings { .. }
            | Event::SecretKept { .. }
            | Event::SecureMac { .. }
            | Event::SecureUnavailable
            | Event::SecretStored
            | Event::SecretNotStored
            | Event::Secret { .. }
            | Event::SecretUnavailable
            | Event::SecretForgotten
            | Event::ScrollEnd { .. }
            | Event::Hover { .. }
            | Event::HoverEnd => None,
            Event::Tick { now_ms } => self.tick(now_ms),
            Event::Touch { x, y, phase } => self.touch(layout, i32::from(x), i32::from(y), phase),
            // No glide here: a wheel moves the content as a scroll does.
            Event::Scroll { x, y, dy } | Event::Wheel { x, y, dy } => {
                let p = layout.scroll_at(i32::from(x), i32::from(y))?;
                self.scroll_by(p.id?, p, i32::from(dy))
            }
            Event::Key(key) => self.key(layout, key),
            Event::KeyUp(key) => self.key_up(key),
            Event::Button(b) => match b {
                ButtonId::Back => Some(Action::Back),
                ButtonId::Up => self.scroll_first(layout, -self.line),
                ButtonId::Down => self.scroll_first(layout, self.line),
                _ => None,
            },
        }
    }

    fn tick(&mut self, now_ms: u64) -> Option<Action> {
        let was_flashing = matches!(self.flash, Some((id, _)) if self.flashing(id));
        let look_was = self.lingering().map(|(_, look)| look);
        self.now_ms = now_ms;
        let look_now = self.lingering().map(|(_, look)| look);
        // §16.124: the linger has two steps, and the tick that ends
        // either of them asks for the frame that shows the next one.
        let linger_stepped = look_was.is_some() && look_now != look_was;
        if look_now.is_none() {
            self.linger = None;
        }
        // A hold that completes on this tick is reported whatever else
        // the tick ends: the frame its action leads to shows the rest.
        if let Some((id, start)) = self.hold
            && now_ms.saturating_sub(start) >= tokens::HOLD_MS
        {
            // The press stays so that `is_held` holds until the finger
            // lifts; the release then reports a redraw.
            self.hold = None;
            return Some(Action::HoldCompleted(id));
        }
        // The masking window closed (the entry masks itself), a linger
        // step ended (the widget halves its pressed look, or stops
        // looking pressed), or a hold is still filling.
        let flash_closed =
            matches!(self.flash, Some((id, _)) if was_flashing && !self.flashing(id));
        (flash_closed || linger_stepped || self.hold.is_some()).then_some(Action::Redraw)
    }

    fn touch(&mut self, layout: &Layout, x: i32, y: i32, phase: TouchPhase) -> Option<Action> {
        match phase {
            TouchPhase::Down => {
                self.press = None;
                self.drag = None;
                self.hold = None;
                // §4.15: a touch or a pointer press takes the ring away,
                // as a desktop hides its focus ring until a key is used.
                if self.focus.is_some() {
                    self.focus = None;
                    self.focus_visible = false;
                    self.scroll_only = false;
                }
                let scroll = layout.scroll_at(x, y).and_then(|p| p.id);
                if let Some(hit) = layout.hit_test(x, y) {
                    let target = hit.hit?;
                    // The key and the cell under the finger are found
                    // now, as the release finds them, so a finger that
                    // slides off one stops showing it (§16.120).
                    let (key, cell) = match target {
                        HitTarget::Keyboard {
                            kind,
                            enabled,
                            scramble,
                            ..
                        } => {
                            let caps = keyboard::keys(
                                kind,
                                hit.rect,
                                &layout.ctx,
                                enabled,
                                scramble,
                                self.mods,
                            );
                            (keyboard::key_at(&caps, x, y), None)
                        }
                        HitTarget::Candidates { count, cells, .. } => {
                            (None, Self::cell_at(hit.rect, count, cells, x))
                        }
                        _ => (None, None),
                    };
                    self.press = Some(Press {
                        target,
                        x,
                        y,
                        moved: false,
                        scroll,
                        key,
                        cell,
                    });
                    if let HitTarget::Hold(id) = target {
                        self.hold = Some((id, self.now_ms));
                    }
                    Some(Action::Redraw)
                } else if let Some(id) = scroll {
                    self.drag = Some((id, x, y, false));
                    None
                } else {
                    None
                }
            }
            TouchPhase::Move => {
                if let Some(p) = &mut self.press
                    && !p.moved
                    && ((x - p.x).abs() > self.slop || (y - p.y).abs() > self.slop)
                {
                    p.moved = true;
                    self.hold = None;
                    // The row under the finger stops looking pressed, so
                    // this frame is more than the pixels moved.
                    self.scroll_only = false;
                    if let Some(id) = p.scroll {
                        self.drag = Some((id, p.x, p.y, true));
                    }
                }
                if let Some((id, _, ly, active)) = self.drag {
                    let moved = active || (y - ly).abs() > self.slop;
                    if moved {
                        self.drag = Some((id, x, y, true));
                        let placed = layout.placed(id)?;
                        return self.scroll_by(id, placed, ly - y);
                    }
                }
                None
            }
            TouchPhase::Up => {
                let press = self.press.take();
                self.hold = None;
                self.drag = None;
                let p = press?;
                if p.moved {
                    return Some(Action::Redraw);
                }
                let hit = layout.hit_test(x, y)?;
                if hit.hit != Some(p.target) {
                    return Some(Action::Redraw);
                }
                // §16.120: the pressed look outlasts the lift on a tap,
                // a key and a candidate cell. A hold completes under the
                // finger and a reveal re-masks on the lift, so neither
                // lingers.
                if matches!(
                    p.target,
                    HitTarget::Tap(_) | HitTarget::Keyboard { .. } | HitTarget::Candidates { .. }
                ) {
                    self.linger = Some((p, self.now_ms));
                }
                match p.target {
                    HitTarget::Tap(id) => Some(Action::Tap(id)),
                    HitTarget::Hold(_) | HitTarget::Reveal(_) => Some(Action::Redraw),
                    HitTarget::Keyboard {
                        id,
                        kind,
                        enabled,
                        scramble,
                    } => {
                        let caps = keyboard::keys(
                            kind,
                            hit.rect,
                            &layout.ctx,
                            enabled,
                            scramble,
                            self.mods,
                        );
                        let input = keyboard::key_at(&caps, x, y)?;
                        self.apply_modifier(id, kind, input)
                    }
                    HitTarget::Candidates { id, count, cells } => {
                        Self::cell_at(hit.rect, count, cells, x)
                            .map(|index| Action::Candidate(id, index))
                    }
                    HitTarget::Pager { id, pages } => {
                        let pages = pages.max(1);
                        let next = (self.page(id) + 1) % pages;
                        self.set_page(id, next);
                        Some(Action::PageChanged(id, next))
                    }
                }
            }
        }
    }

    /// The candidate cell of a strip at `rect` under `x`, and `None`
    /// when the cell carries no candidate. The cell keeps its width
    /// whether or not a candidate is in it, so a one-character strip
    /// with three characters left still has them under the first three
    /// keys' worth of width, and a tap on an empty cell does nothing.
    fn cell_at(rect: Rect, count: u8, cells: u8, x: i32) -> Option<u8> {
        let cells = cells.max(count);
        let cell = (rect.w / i32::from(cells)).max(1);
        let index = ((x - rect.x) / cell).clamp(0, i32::from(cells) - 1);
        (index < i32::from(count)).then_some(index as u8)
    }

    fn apply_modifier(&mut self, id: Id, kind: KeyboardKind, input: KeyInput) -> Option<Action> {
        match input {
            KeyInput::Shift => {
                self.mods.shift = !self.mods.shift;
                Some(Action::Redraw)
            }
            KeyInput::Symbols => {
                self.mods.symbols = !self.mods.symbols;
                Some(Action::Redraw)
            }
            KeyInput::Char(c) => {
                // Shift is one-shot, as on phone keyboards: it releases
                // on the first key it changed, which is an upper-case
                // letter on the passphrase keyboard and a doubled
                // consonant or a shifted vowel on the jamo one.
                // On the address keyboard shift is the layer the
                // address is typed in and not a case for one letter, so
                // it stays down until it is pressed again.
                if kind != KeyboardKind::Address
                    && self.mods.shift
                    && (c.is_ascii_uppercase() || !c.is_ascii())
                {
                    self.mods.shift = false;
                }
                Some(Action::KeyboardInput(id, KeyInput::Char(c)))
            }
            other => Some(Action::KeyboardInput(id, other)),
        }
    }

    fn scroll_by(&mut self, id: Id, placed: &crate::layout::Placed, dy: i32) -> Option<Action> {
        let info = placed.scroll?;
        let max = info.max_offset();
        let new = (info.offset + dy).clamp(0, max);
        if new == info.offset {
            return None;
        }
        self.scroll.set(id, new);
        Some(Action::Scrolled(id))
    }

    fn scroll_first(&mut self, layout: &Layout, dy: i32) -> Option<Action> {
        let p = layout.items.iter().find(|p| p.scroll.is_some())?;
        self.scroll_by(p.id?, p, dy)
    }

    /// The items a key can move focus to, in the order the layout placed
    /// them, which is reading order.
    fn stops(layout: &Layout) -> impl Iterator<Item = &crate::layout::Placed> {
        layout
            .items
            .iter()
            .filter(|p| p.focus.is_some() && !p.rect.is_empty())
    }

    /// Moves focus `step` stops on, wrapping at both ends, and scrolls
    /// the item it lands on into view.
    fn move_focus(&mut self, layout: &Layout, forward: bool) -> Option<Action> {
        let ids: alloc::vec::Vec<Id> = Self::stops(layout).filter_map(|p| p.focus).collect();
        if ids.is_empty() {
            return None;
        }
        let next = match self.focus.and_then(|f| ids.iter().position(|i| *i == f)) {
            Some(i) if forward => (i + 1) % ids.len(),
            Some(i) => (i + ids.len() - 1) % ids.len(),
            None if forward => 0,
            None => ids.len() - 1,
        };
        self.set_focus(layout, ids[next]);
        Some(Action::Redraw)
    }

    /// Moves focus to the nearest stop to the left or the right inside
    /// the same row: two stops are in one row when their rectangles
    /// overlap vertically by more than half the shorter of the two.
    fn focus_sideways(&mut self, layout: &Layout, right: bool) -> Option<Action> {
        let id = self.focus?;
        let here = Self::stops(layout).find(|p| p.focus == Some(id))?;
        // A pager turns its page, as its chevrons do (§4.15).
        if let Some(HitTarget::Pager { id, pages }) = here.hit {
            let pages = pages.max(1);
            let page = self.page(id);
            let next = if right {
                (page + 1) % pages
            } else {
                (page + pages - 1) % pages
            };
            self.set_page(id, next);
            self.scroll_only = false;
            return Some(Action::PageChanged(id, next));
        }
        let rect = here.rect;
        let mut best: Option<(i32, Id)> = None;
        for p in Self::stops(layout) {
            let Some(other) = p.focus else { continue };
            if other == id {
                continue;
            }
            let overlap = rect.bottom().min(p.rect.bottom()) - rect.y.max(p.rect.y);
            if 2 * overlap <= rect.h.min(p.rect.h) {
                continue;
            }
            let d = if right {
                p.rect.x - rect.x
            } else {
                rect.x - p.rect.x
            };
            if d > 0 && best.is_none_or(|(b, _)| d < b) {
                best = Some((d, other));
            }
        }
        let (_, next) = best?;
        self.set_focus(layout, next);
        Some(Action::Redraw)
    }

    /// Puts the ring on `id` and scrolls it wholly into view.
    fn set_focus(&mut self, layout: &Layout, id: Id) {
        self.focus = Some(id);
        self.focus_visible = true;
        self.scroll_only = false;
        let Some(p) = Self::stops(layout).find(|p| p.focus == Some(id)) else {
            return;
        };
        let (rect, clip) = (p.rect, p.clip);
        let dy = if rect.bottom() > clip.bottom() {
            rect.bottom() - clip.bottom()
        } else if rect.y < clip.y {
            rect.y - clip.y
        } else {
            return;
        };
        let centre = clip.center();
        let Some(region) = layout.scroll_at(centre.x, centre.y) else {
            return;
        };
        let Some(region_id) = region.id else {
            return;
        };
        self.scroll_by(region_id, region, dy);
    }

    /// Acts on the focused item as a tap does.
    fn activate(&mut self, layout: &Layout) -> Option<Action> {
        let id = self.focus?;
        let here = Self::stops(layout).find(|p| p.focus == Some(id))?;
        let rect = here.rect;
        let under_key = |target| {
            let c = rect.center();
            Press {
                target,
                x: c.x,
                y: c.y,
                moved: false,
                scroll: None,
                key: None,
                cell: None,
            }
        };
        match here.hit {
            Some(HitTarget::Tap(id)) => {
                self.scroll_only = false;
                Some(Action::Tap(id))
            }
            Some(HitTarget::Hold(id)) => {
                // Auto-repeat while the key is down is the same key
                // still down: the fill carries on from where it is.
                if matches!(self.hold, Some((h, _)) if h == id) {
                    return None;
                }
                self.press = Some(under_key(HitTarget::Hold(id)));
                self.hold = Some((id, self.now_ms));
                self.scroll_only = false;
                Some(Action::Redraw)
            }
            Some(HitTarget::Reveal(id)) => {
                if self.is_held(id) {
                    return None;
                }
                self.press = Some(under_key(HitTarget::Reveal(id)));
                self.scroll_only = false;
                Some(Action::Redraw)
            }
            Some(HitTarget::Pager { id, pages }) => {
                let pages = pages.max(1);
                let next = (self.page(id) + 1) % pages;
                self.set_page(id, next);
                self.scroll_only = false;
                Some(Action::PageChanged(id, next))
            }
            // A dimmed control does nothing, as a tap on it does.
            _ => None,
        }
    }

    /// A key coming up: the end of a hold or of a reveal, as a finger
    /// lifting is. A shell that never sends it cannot hold with a key.
    fn key_up(&mut self, key: Key) -> Option<Action> {
        if !matches!(key, Key::Enter | Key::Char(' ')) {
            return None;
        }
        if !matches!(
            self.press,
            Some(Press {
                target: HitTarget::Hold(_) | HitTarget::Reveal(_),
                ..
            })
        ) {
            return None;
        }
        self.press = None;
        self.hold = None;
        self.scroll_only = false;
        Some(Action::Redraw)
    }

    /// The first on-screen keyboard in `layout`, if it has one.
    fn keyboard_in(layout: &Layout) -> Option<(Id, KeyboardKind, keyboard::KeyMask)> {
        layout.items.iter().find_map(|p| match p.hit {
            Some(HitTarget::Keyboard {
                id, kind, enabled, ..
            }) => Some((id, kind, enabled)),
            _ => None,
        })
    }

    /// Routes a physical key: focus and its ring (§4.15), then the first
    /// keyboard in the layout (§4.5).
    fn key(&mut self, layout: &Layout, key: Key) -> Option<Action> {
        if key == Key::Escape {
            return Some(Action::Back);
        }
        self.retain_focus(layout);
        match key {
            Key::Tab => return self.move_focus(layout, true),
            Key::BackTab => return self.move_focus(layout, false),
            Key::Up if self.focus.is_some() => return self.move_focus(layout, false),
            Key::Down if self.focus.is_some() => return self.move_focus(layout, true),
            Key::Up => return self.scroll_first(layout, -self.line),
            Key::Down => return self.scroll_first(layout, self.line),
            Key::Left => return self.focus_sideways(layout, false),
            Key::Right => return self.focus_sideways(layout, true),
            _ => {}
        }
        // Enter acts on the focused item; Space does too, except where
        // the on-screen keyboard takes a space, where typing wins.
        if self.focus.is_some() {
            let typed_space = key == Key::Char(' ')
                && Self::keyboard_in(layout)
                    .is_some_and(|(_, kind, _)| keyboard::accepts_char(kind, ' ').is_some());
            if matches!(key, Key::Enter | Key::Char(' ')) && !typed_space {
                return self.activate(layout);
            }
        }
        let (id, kind, enabled) = Self::keyboard_in(layout)?;
        match key {
            Key::Char(c) => {
                let c = keyboard::accepts_char(kind, c)?;
                // Respect the enabled set of a predictive keyboard: the
                // layout carries it. A character the keyboard has no key
                // for — a small kana, which the 小 key makes — is left to
                // the application.
                let bit = keyboard::key_bit(kind, c);
                if bit != 0
                    && matches!(
                        kind,
                        KeyboardKind::Bip39
                            | KeyboardKind::Kana
                            | KeyboardKind::Jamo
                            | KeyboardKind::Address
                            | KeyboardKind::Codex32
                            | KeyboardKind::Pin
                    )
                    && enabled & bit == 0
                {
                    return None;
                }
                Some(Action::KeyboardInput(id, KeyInput::Char(c)))
            }
            Key::Backspace => Some(Action::KeyboardInput(id, KeyInput::Backspace)),
            // A dead ✓ does nothing when it is tapped, and Enter is the
            // same key: an incomplete PIN is never submitted, so it
            // never costs an attempt (§16.99).
            Key::Enter
                if kind == KeyboardKind::Pin
                    && enabled & (keyboard::DONE_DISABLED | keyboard::DONE_ABSENT) != 0 =>
            {
                None
            }
            Key::Enter => Some(Action::KeyboardInput(id, KeyInput::Done)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::{Rect, Scale, SizeClass};
    use crate::layout::{LayoutCtx, Node, solve, solve_with};
    use crate::widgets::{ButtonStyle, Icon, Widget};

    fn ctx() -> LayoutCtx {
        LayoutCtx::new(Scale::IDENTITY, SizeClass::Mobile)
    }

    fn touch(x: i32, y: i32, phase: TouchPhase) -> Event {
        Event::Touch {
            x: x as u16,
            y: y as u16,
            phase,
        }
    }

    fn tree() -> Node {
        Node::column()
            .child(Node::widget(Widget::button(
                Id(1),
                "Tap",
                ButtonStyle::Primary,
            )))
            .child(Node::widget(Widget::hold_button(
                Id(2),
                "Hold",
                ButtonStyle::Danger,
            )))
            .child(Node::widget(Widget::keyboard(Id(3), KeyboardKind::Bip39)).height(150.0))
    }

    #[test]
    fn tap_requires_down_and_up_on_the_same_target() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        assert_eq!(
            s.event(&layout, touch(10, 10, TouchPhase::Down)),
            Some(Action::Redraw)
        );
        assert!(s.pressed(Id(1)).is_some());
        assert_eq!(
            s.event(&layout, touch(12, 12, TouchPhase::Up)),
            Some(Action::Tap(Id(1)))
        );
        s.clear_linger();
        assert!(s.pressed(Id(1)).is_none());
        // Down on one, up on another: no tap.
        s.event(&layout, touch(10, 10, TouchPhase::Down));
        assert_eq!(
            s.event(&layout, touch(10, 60, TouchPhase::Up)),
            Some(Action::Redraw)
        );
        // Moving past the slop cancels.
        s.event(&layout, touch(10, 10, TouchPhase::Down));
        s.event(&layout, touch(40, 10, TouchPhase::Move));
        assert_eq!(
            s.event(&layout, touch(10, 10, TouchPhase::Up)),
            Some(Action::Redraw)
        );
    }

    /// A tap just before a hold leaves its pressed look lingering; the
    /// tick that ends the linger's last step and the tick that ends the
    /// hold can be one tick, and the hold still completes.
    #[test]
    fn a_hold_completes_on_the_tick_a_linger_ends() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, Event::Tick { now_ms: 1000 });
        s.event(&layout, touch(10, 10, TouchPhase::Down));
        assert_eq!(
            s.event(&layout, touch(10, 10, TouchPhase::Up)),
            Some(Action::Tap(Id(1)))
        );
        s.event(&layout, touch(10, 60, TouchPhase::Down));
        s.event(
            &layout,
            Event::Tick {
                now_ms: 1000 + tokens::PRESS_LINGER_FULL_MS,
            },
        );
        assert_eq!(
            s.event(
                &layout,
                Event::Tick {
                    now_ms: 1000 + tokens::PRESS_LINGER_FULL_MS + tokens::HOLD_MS,
                },
            ),
            Some(Action::HoldCompleted(Id(2)))
        );
    }

    #[test]
    fn hold_completes_after_the_duration() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, Event::Tick { now_ms: 1000 });
        s.event(&layout, touch(10, 60, TouchPhase::Down));
        assert_eq!(s.hold_progress(Id(2)), 0.0);
        assert_eq!(
            s.event(&layout, Event::Tick { now_ms: 1750 }),
            Some(Action::Redraw)
        );
        assert!((s.hold_progress(Id(2)) - 0.5).abs() < 0.01);
        assert!(s.animating());
        assert_eq!(
            s.event(&layout, Event::Tick { now_ms: 2500 }),
            Some(Action::HoldCompleted(Id(2)))
        );
        assert!(!s.animating());
        assert!(s.is_held(Id(2)), "still under the finger");
        assert_eq!(
            s.event(&layout, touch(10, 60, TouchPhase::Up)),
            Some(Action::Redraw)
        );
        assert!(!s.is_held(Id(2)));
        // Releasing early cancels.
        s.event(&layout, touch(10, 60, TouchPhase::Down));
        s.event(&layout, Event::Tick { now_ms: 3000 });
        assert_eq!(
            s.event(&layout, touch(10, 60, TouchPhase::Up)),
            Some(Action::Redraw)
        );
        assert_eq!(s.event(&layout, Event::Tick { now_ms: 9000 }), None);
    }

    #[test]
    fn keyboard_taps_and_physical_keys_produce_input() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        let kb_rect = layout.rect(Id(3)).unwrap();
        // Top-left key of QWERTY is q.
        let (x, y) = (kb_rect.x + 10, kb_rect.y + 10);
        s.event(&layout, touch(x, y, TouchPhase::Down));
        assert_eq!(
            s.event(&layout, touch(x, y, TouchPhase::Up)),
            Some(Action::KeyboardInput(Id(3), KeyInput::Char('q')))
        );
        assert_eq!(
            s.event(&layout, Event::Key(Key::Char('Z'))),
            Some(Action::KeyboardInput(Id(3), KeyInput::Char('z')))
        );
        assert_eq!(s.event(&layout, Event::Key(Key::Char('1'))), None);
        assert_eq!(
            s.event(&layout, Event::Key(Key::Backspace)),
            Some(Action::KeyboardInput(Id(3), KeyInput::Backspace))
        );
        assert_eq!(
            s.event(&layout, Event::Key(Key::Enter)),
            Some(Action::KeyboardInput(Id(3), KeyInput::Done))
        );
        assert_eq!(
            s.event(&layout, Event::Key(Key::Escape)),
            Some(Action::Back)
        );
        assert_eq!(
            s.event(&layout, Event::Button(ButtonId::Back)),
            Some(Action::Back)
        );
    }

    /// A PIN pad takes the keyboard's digits, and Enter submits only
    /// what the ✓ would submit: while the entry is short the ✓ is dead
    /// and tapping it does nothing, so Enter does nothing either
    /// (`docs/PLANNING.md` §16.99).
    #[test]
    fn a_pin_pad_takes_typed_digits_and_enter_only_once_the_tick_is_live() {
        let pad = |done| {
            Node::column()
                .child(Node::widget(Widget::pad(Id(4), KeyboardKind::Pin, done)).height(200.0))
        };
        let rect = Rect::new(0, 0, 360, 400);

        let layout = solve(&pad(false), rect, &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        assert_eq!(
            s.event(&layout, Event::Key(Key::Char('7'))),
            Some(Action::KeyboardInput(Id(4), KeyInput::Char('7')))
        );
        assert_eq!(s.event(&layout, Event::Key(Key::Char('a'))), None);
        assert_eq!(
            s.event(&layout, Event::Key(Key::Backspace)),
            Some(Action::KeyboardInput(Id(4), KeyInput::Backspace))
        );
        assert_eq!(s.event(&layout, Event::Key(Key::Enter)), None);

        let layout = solve(&pad(true), rect, &ctx());
        assert_eq!(
            s.event(&layout, Event::Key(Key::Enter)),
            Some(Action::KeyboardInput(Id(4), KeyInput::Done))
        );
    }

    /// `docs/DESIGN.md` §4.2: "Tap checks (the list keeps its scroll
    /// position)." A choice list is rebuilt with the check moved, and
    /// the offset it was scrolled to has to survive that, or the row
    /// under the finger jumps away from it.
    #[test]
    fn tapping_a_row_leaves_the_scroll_offset_alone() {
        let rows = Node::column().children((0..20).map(|i| {
            Node::widget(Widget::button(Id(100 + i), "Row", ButtonStyle::Secondary)).height(50.0)
        }));
        let root = Node::scroll(Id(9), rows);
        let area = Rect::new(0, 0, 200, 200);
        let mut s = UiState::new(Scale::IDENTITY);
        s.set_scroll(Id(9), 120);
        let layout = solve_with(&root, area, &ctx(), s.scroll_offsets());
        // A row well inside the viewport, at the offset it was left at.
        let row = layout.rect(Id(104)).expect("a row");
        let (x, y) = (row.center().x, row.center().y);
        s.event(&layout, touch(x, y, TouchPhase::Down));
        assert_eq!(
            s.event(&layout, touch(x, y, TouchPhase::Up)),
            Some(Action::Tap(Id(104)))
        );
        assert_eq!(s.scroll_offsets().get(Id(9)), 120, "the tap moved the list");
    }

    /// §4.3: "the newest entry shows for half a second, then masks."
    #[test]
    fn a_flashed_entry_masks_when_its_window_closes() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, Event::Tick { now_ms: 1_000 });
        s.flash(Id(5));
        assert!(s.flashing(Id(5)));
        assert!(s.animating());
        assert_eq!(s.event(&layout, Event::Tick { now_ms: 1_200 }), None);
        assert!(s.flashing(Id(5)), "still inside the window");
        // The window closes: the pad redraws with the entry masked.
        assert_eq!(
            s.event(
                &layout,
                Event::Tick {
                    now_ms: 1_000 + tokens::REVEAL_FLASH_MS
                }
            ),
            Some(Action::Redraw)
        );
        assert!(!s.flashing(Id(5)));
        assert!(!s.animating());
        // Another entry opens a new window; a different run is never
        // flashing on this one's account.
        s.flash(Id(6));
        assert!(s.flashing(Id(6)) && !s.flashing(Id(5)));
    }

    /// A strip of three candidates over a secret surface.
    fn strip() -> Node {
        Node::column()
            .child(
                Node::widget(Widget::CandidateStrip {
                    id: Id(7),
                    words: ["abandon", "ability", "able"]
                        .iter()
                        .map(|w| alloc::string::String::from(*w))
                        .collect(),
                    more: false,
                    selected: None,
                    outlined: None,
                })
                .height(48.0),
            )
            .child(Node::widget(Widget::SecretSurface { id: Id(8) }).height(100.0))
    }

    /// §16.120, §16.124: a key, a cell and a button keep the pressed
    /// look for a moment after the finger lifts, so the person sees what
    /// they hit once their finger is off it, and the look fades in two
    /// steps.
    #[test]
    fn what_was_pressed_keeps_its_look_for_a_moment_after_the_lift() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, Event::Tick { now_ms: 1_000 });
        s.event(&layout, touch(10, 10, TouchPhase::Down));
        assert!(s.pressed(Id(1)).is_some());
        assert_eq!(
            s.event(&layout, touch(12, 12, TouchPhase::Up)),
            Some(Action::Tap(Id(1)))
        );
        assert_eq!(
            s.pressed(Id(1)),
            Some(PressLook::Full),
            "the finger is off it and it shows"
        );
        assert!(s.animating());
        assert_eq!(s.event(&layout, Event::Tick { now_ms: 1_050 }), None);
        assert_eq!(
            s.pressed(Id(1)),
            Some(PressLook::Full),
            "still inside the first step"
        );
        assert_eq!(
            s.event(
                &layout,
                Event::Tick {
                    now_ms: 1_000 + tokens::PRESS_LINGER_FULL_MS
                }
            ),
            Some(Action::Redraw),
            "the first step ends and the frame it asks for is the half one"
        );
        assert_eq!(s.pressed(Id(1)), Some(PressLook::Half));
        assert!(s.animating());
        assert_eq!(
            s.event(
                &layout,
                Event::Tick {
                    now_ms: 1_000 + tokens::PRESS_LINGER_MS
                }
            ),
            Some(Action::Redraw)
        );
        assert!(s.pressed(Id(1)).is_none());
        assert!(!s.animating());
    }

    #[test]
    fn the_key_under_the_finger_is_the_one_the_release_reports() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        let kb = layout.rect(Id(3)).unwrap();
        // Top-left key of QWERTY is q.
        let (x, y) = (kb.x + 10, kb.y + 10);
        s.event(&layout, Event::Tick { now_ms: 1_000 });
        s.event(&layout, touch(x, y, TouchPhase::Down));
        assert_eq!(
            s.pressed_key(Id(3)),
            Some((KeyInput::Char('q'), PressLook::Full))
        );
        assert_eq!(
            s.event(&layout, touch(x, y, TouchPhase::Up)),
            Some(Action::KeyboardInput(Id(3), KeyInput::Char('q')))
        );
        assert_eq!(
            s.pressed_key(Id(3)),
            Some((KeyInput::Char('q'), PressLook::Full)),
            "the key shows after the finger has left it"
        );
        s.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::PRESS_LINGER_MS,
            },
        );
        assert_eq!(s.pressed_key(Id(3)), None);
        // A finger that slides off the key it went down on stops
        // showing it, and leaves nothing behind.
        s.event(&layout, touch(x, y, TouchPhase::Down));
        assert_eq!(
            s.pressed_key(Id(3)),
            Some((KeyInput::Char('q'), PressLook::Full))
        );
        s.event(&layout, touch(x + 60, y, TouchPhase::Move));
        assert_eq!(s.pressed_key(Id(3)), None);
        s.event(&layout, touch(x + 60, y, TouchPhase::Up));
        assert_eq!(s.pressed_key(Id(3)), None);
    }

    #[test]
    fn the_candidate_cell_under_the_finger_shows_and_lingers() {
        let layout = solve(&strip(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        let rect = layout.rect(Id(7)).unwrap();
        // The strip is a fixed grid, so cell 1 is the second of them
        // whatever the count.
        let cells = tokens::candidates_per_row(SizeClass::Mobile, false) as i32;
        let (x, y) = (rect.x + rect.w * 3 / (2 * cells), rect.y + rect.h / 2);
        s.event(&layout, Event::Tick { now_ms: 1_000 });
        s.event(&layout, touch(x, y, TouchPhase::Down));
        assert_eq!(s.pressed_cell(Id(7)), Some((1, PressLook::Full)));
        assert_eq!(
            s.event(&layout, touch(x, y, TouchPhase::Up)),
            Some(Action::Candidate(Id(7), 1))
        );
        assert_eq!(s.pressed_cell(Id(7)), Some((1, PressLook::Full)));
        s.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::PRESS_LINGER_MS,
            },
        );
        assert_eq!(s.pressed_cell(Id(7)), None);
    }

    /// A hold completes under the finger and a reveal re-masks on the
    /// lift: neither is a tap, so neither leaves a look behind.
    #[test]
    fn a_hold_and_a_reveal_leave_nothing_behind() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, Event::Tick { now_ms: 1_000 });
        s.event(&layout, touch(10, 60, TouchPhase::Down));
        s.event(
            &layout,
            Event::Tick {
                now_ms: 1_000 + tokens::HOLD_MS,
            },
        );
        s.event(&layout, touch(10, 60, TouchPhase::Up));
        assert!(!s.is_held(Id(2)));
        assert!(!s.animating(), "nothing is waiting to stop looking held");

        let layout = solve(&strip(), Rect::new(0, 0, 360, 400), &ctx());
        let secret = layout.rect(Id(8)).unwrap();
        let (x, y) = (secret.x + 10, secret.y + 10);
        s.event(&layout, touch(x, y, TouchPhase::Down));
        assert!(s.is_held(Id(8)));
        s.event(&layout, touch(x, y, TouchPhase::Up));
        assert!(!s.is_held(Id(8)), "the secret re-masks on the lift");
        assert!(!s.animating());
    }

    /// Ids repeat from screen to screen, so the look a release left
    /// behind never shows on the screen that replaces it.
    #[test]
    fn a_screen_change_drops_the_pressed_look() {
        let layout = solve(&tree(), Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, touch(10, 10, TouchPhase::Down));
        s.event(&layout, touch(10, 10, TouchPhase::Up));
        assert!(s.pressed(Id(1)).is_some());
        s.clear_linger();
        assert!(s.pressed(Id(1)).is_none());
    }

    /// A menu: three rows in a scrolling list, and a Continue under it.
    fn menu() -> Node {
        let rows = Node::column().children(
            (1..=3).map(|i| Node::widget(Widget::menu_row(Id(i), Icon::Keys, "Row", None))),
        );
        Node::column()
            .child(Node::scroll(Id(9), rows).weight(1.0))
            .child(
                Node::widget(Widget::button(Id(4), "Continue", ButtonStyle::Primary)).height(48.0),
            )
    }

    fn key(k: Key) -> Event {
        Event::Key(k)
    }

    /// Tab walks the rows and the action in reading order and comes back
    /// round; Shift+Tab walks them backwards; the arrows do the same
    /// once something has focus, and scroll the list while nothing has.
    #[test]
    fn tab_and_the_arrows_walk_a_menu_and_enter_opens_a_row() {
        let layout = solve(&menu(), Rect::new(0, 0, 360, 640), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        assert_eq!(s.focused(), None, "nothing has focus when a screen opens");

        for id in [Id(1), Id(2), Id(3), Id(4), Id(1)] {
            assert_eq!(s.event(&layout, key(Key::Tab)), Some(Action::Redraw));
            assert_eq!(s.focused(), Some(id));
        }
        for id in [Id(4), Id(3)] {
            s.event(&layout, key(Key::BackTab));
            assert_eq!(s.focused(), Some(id));
        }
        s.event(&layout, key(Key::Down));
        assert_eq!(s.focused(), Some(Id(4)));
        s.event(&layout, key(Key::Up));
        assert_eq!(s.focused(), Some(Id(3)));

        assert_eq!(s.event(&layout, key(Key::Enter)), Some(Action::Tap(Id(3))));
        // Space is the same key on a screen whose keyboard has no space.
        assert_eq!(
            s.event(&layout, key(Key::Char(' '))),
            Some(Action::Tap(Id(3)))
        );
    }

    /// A touch takes the ring away, and the arrows go back to scrolling
    /// the list.
    #[test]
    fn a_touch_takes_the_focus_ring_away() {
        let tree = menu();
        let area = Rect::new(0, 0, 360, 200);
        let mut s = UiState::new(Scale::IDENTITY);
        let layout = solve(&tree, area, &ctx());
        s.event(&layout, key(Key::Tab));
        assert!(s.is_focused(Id(1)));

        let row = layout.rect(Id(1)).expect("a row");
        s.event(
            &layout,
            touch(row.center().x, row.center().y, TouchPhase::Down),
        );
        assert_eq!(s.focused(), None);
        assert_eq!(
            s.event(
                &layout,
                touch(row.center().x, row.center().y, TouchPhase::Up)
            ),
            Some(Action::Tap(Id(1)))
        );
        assert_eq!(
            s.event(&layout, key(Key::Down)),
            Some(Action::Scrolled(Id(9))),
            "with nothing focused the arrows scroll"
        );
    }

    /// A row below the fold is scrolled wholly into view when it takes
    /// focus.
    #[test]
    fn focus_scrolls_a_row_below_the_fold_into_view() {
        let rows = Node::column().children(
            (0..20).map(|i| Node::widget(Widget::menu_row(Id(100 + i), Icon::Keys, "Row", None))),
        );
        let tree = Node::scroll(Id(9), rows);
        let area = Rect::new(0, 0, 360, 200);
        let mut s = UiState::new(Scale::IDENTITY);
        let layout = solve(&tree, area, &ctx());
        let below = layout.placed(Id(104)).expect("a row");
        assert!(
            below.rect.bottom() > below.clip.bottom(),
            "the row starts below the fold"
        );
        for _ in 0..5 {
            s.event(&layout, key(Key::Tab));
        }
        assert_eq!(s.focused(), Some(Id(104)));
        assert!(s.scroll_offsets().get(Id(9)) > 0, "the list moved");
        let layout = solve_with(&tree, area, &ctx(), s.scroll_offsets());
        let now = layout.placed(Id(104)).expect("a row");
        assert!(
            now.rect.y >= now.clip.y && now.rect.bottom() <= now.clip.bottom(),
            "the focused row is wholly in view"
        );
    }

    /// Left and Right move between the two buttons of a pair and do
    /// nothing where the focused item is alone in its row. A dimmed
    /// action takes focus and does nothing on Enter, as it does nothing
    /// under a finger.
    #[test]
    fn the_arrows_move_across_a_pair_and_a_dimmed_action_does_nothing() {
        let tree = Node::column()
            .child(Node::widget(Widget::menu_row(
                Id(1),
                Icon::Keys,
                "Row",
                None,
            )))
            .child(
                Node::row()
                    .child(Node::widget(Widget::button(
                        Id(2),
                        "Numbers",
                        ButtonStyle::Secondary,
                    )))
                    .child(Node::widget(Widget::button(
                        Id(3),
                        "Done",
                        ButtonStyle::Primary,
                    )))
                    .height(48.0),
            )
            .child(
                Node::widget(Widget::disabled_button(
                    Id(4),
                    "Export",
                    ButtonStyle::Primary,
                ))
                .height(48.0),
            );
        let layout = solve(&tree, Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);

        s.event(&layout, key(Key::Tab));
        assert_eq!(s.focused(), Some(Id(1)));
        assert_eq!(s.event(&layout, key(Key::Right)), None, "the row is alone");
        assert_eq!(s.focused(), Some(Id(1)));

        s.event(&layout, key(Key::Tab));
        assert_eq!(s.focused(), Some(Id(2)));
        assert_eq!(s.event(&layout, key(Key::Right)), Some(Action::Redraw));
        assert_eq!(s.focused(), Some(Id(3)));
        s.event(&layout, key(Key::Left));
        assert_eq!(s.focused(), Some(Id(2)));
        assert_eq!(
            s.event(&layout, key(Key::Left)),
            None,
            "nothing to its left"
        );

        s.event(&layout, key(Key::Tab));
        s.event(&layout, key(Key::Tab));
        assert_eq!(s.focused(), Some(Id(4)), "a dimmed action takes focus");
        assert_eq!(s.event(&layout, key(Key::Enter)), None);
    }

    /// A hold is held under Enter as it is under a finger: the key going
    /// down starts the fill, the key coming up before it completes
    /// cancels it, and holding through the duration completes it.
    #[test]
    fn a_hold_is_held_with_enter_and_cancelled_when_the_key_comes_up() {
        let tree = Node::column().child(
            Node::widget(Widget::hold_button(Id(2), "Hold", ButtonStyle::Danger)).height(56.0),
        );
        let layout = solve(&tree, Rect::new(0, 0, 360, 400), &ctx());
        let mut s = UiState::new(Scale::IDENTITY);
        s.event(&layout, Event::Tick { now_ms: 1_000 });
        s.event(&layout, key(Key::Tab));
        assert_eq!(s.focused(), Some(Id(2)));

        assert_eq!(s.event(&layout, key(Key::Enter)), Some(Action::Redraw));
        assert!(s.is_held(Id(2)));
        s.event(&layout, Event::Tick { now_ms: 1_750 });
        assert!((s.hold_progress(Id(2)) - 0.5).abs() < 0.01);
        assert_eq!(
            s.event(&layout, Event::KeyUp(Key::Enter)),
            Some(Action::Redraw)
        );
        assert!(!s.is_held(Id(2)));
        assert_eq!(s.event(&layout, Event::Tick { now_ms: 9_000 }), None);

        s.event(&layout, key(Key::Enter));
        s.event(&layout, Event::Tick { now_ms: 9_100 });
        assert_eq!(
            s.event(
                &layout,
                Event::Tick {
                    now_ms: 9_000 + tokens::HOLD_MS
                }
            ),
            Some(Action::HoldCompleted(Id(2)))
        );
        assert!(s.is_held(Id(2)), "still under the key");
        s.event(&layout, Event::KeyUp(Key::Enter));
        assert!(!s.is_held(Id(2)));
    }

    #[test]
    fn drag_and_wheel_scroll_a_region() {
        let content = Node::column().children((0..20).map(|_| Node::column().height(50.0)));
        let root = Node::scroll(Id(9), content);
        let mut s = UiState::new(Scale::IDENTITY);
        let layout = solve(&root, Rect::new(0, 0, 200, 200), &ctx());
        assert_eq!(
            s.event(
                &layout,
                Event::Scroll {
                    x: 10,
                    y: 10,
                    dy: 30
                }
            ),
            Some(Action::Scrolled(Id(9)))
        );
        assert_eq!(s.scroll_offsets().get(Id(9)), 30);
        let layout = solve_with(&root, Rect::new(0, 0, 200, 200), &ctx(), s.scroll_offsets());
        assert_eq!(s.event(&layout, touch(10, 150, TouchPhase::Down)), None);
        assert_eq!(
            s.event(&layout, touch(10, 100, TouchPhase::Move)),
            Some(Action::Scrolled(Id(9)))
        );
        assert_eq!(s.scroll_offsets().get(Id(9)), 80);
        s.event(&layout, touch(10, 100, TouchPhase::Up));
        // Clamped at the end of the content.
        assert_eq!(
            s.event(
                &layout,
                Event::Scroll {
                    x: 10,
                    y: 10,
                    dy: 5000
                }
            ),
            Some(Action::Scrolled(Id(9)))
        );
        assert_eq!(s.scroll_offsets().get(Id(9)), 800);
        let layout = solve_with(&root, Rect::new(0, 0, 200, 200), &ctx(), s.scroll_offsets());
        assert_eq!(
            s.event(
                &layout,
                Event::Scroll {
                    x: 10,
                    y: 10,
                    dy: 1
                }
            ),
            None
        );
    }
}
