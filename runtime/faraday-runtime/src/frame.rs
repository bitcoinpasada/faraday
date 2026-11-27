//! The wiping frame buffer, painting the call list, the last `FrameOut` (05 §2.5).

use core::borrow::Borrow;
use std::collections::VecDeque;

use faraday_atoms::{
    Anchor, ComponentId, FrameOut, KeptFrost, Laid, Pan, Pointer, Theme, Ui,
    motion::{FIRST_STEP_MS, MAX_STEP_MS},
};
use faraday_crypto::Secret;
use faraday_draw::canvas::{Calls, Canvas};
use faraday_draw::geom::Rect as Target;
use faraday_kernel::{Note, NoteUp, Running, ViewOrder, session, shared, vaults};
use faraday_port as port;
use faraday_port::event::{Input, Key, TouchPhase};
use faraday_port::values::{Pace, Point};

use crate::facts::{Lent, build_view, show_of};
use crate::queue::Item;
use crate::root::{Root, Tops};
use crate::runtime::{Phase, Runtime};
use crate::seen::Mark;

/// How long a note stays up before the next one comes up, by the runtime's clock (04 §10).
const TOAST_SHOW_MS: u64 = 4000;

/// How far a wheel notch scrolls, in design units, and how many pixels the adapters send a notch
/// (`Input::Wheel`): a notch covers the same part of the page on every screen.
const WHEEL_NOTCH_UNITS: f32 = 56.0;
const WHEEL_NOTCH_PX: f32 = 48.0;
/// How far a finger moves, in design units, before it scrolls rather than presses.
const SLOP_UNITS: f32 = 8.0;
/// How long two-finger scrolling may pause before it is taken to have ended, for a shell that
/// sends no end.
const SCROLL_QUIET_MS: u64 = 300;

/// The region a pan moves, as it stood when the pan began (units).
#[derive(Clone, Copy)]
pub(crate) struct PanState {
    id: ComponentId,
    max: f32,
    view: f32,
    began: bool,
    last_ms: u64,
}

/// A frosted page kept under a sheet (04 K16), in a wiping buffer (02 P13).
pub(crate) struct KeptCopy {
    pub(crate) sheet: Anchor,
    pub(crate) page: Anchor,
    pub(crate) taken_at: u64,
    /// The `secret` flag of the frame it was copied from: V19 refuses a frame that is set, so it
    /// is clear in every copy kept.
    pub(crate) secret: bool,
    pub(crate) pixels: Secret<Vec<u8>>,
}

impl KeptCopy {
    fn wipe(&mut self) {
        let pixels = self.pixels.expose_mut();
        pixels.fill(0);
        pixels.clear();
    }
}

/// What the last frame keeps until the next (§2.5): the buffer, the output, the waiting orders.
pub(crate) struct Painted<R: Root> {
    /// P13: wiping.
    buffer: Secret<Vec<u8>>,
    /// The size the buffer was last made for.
    size: (u16, u16),
    calls: Option<Calls>,
    changed: Vec<port::frame::Rect>,
    pub(crate) out: Option<FrameOut<R::Press>>,
    /// The number of the last frame drawn.
    number: u64,
    /// `Fx::View` orders waiting for their anchor to be drawn.
    pub(crate) orders: Vec<ViewOrder>,
    /// The last laid fact sent for each `Anchor` (V17).
    sent: Vec<(Anchor, Laid)>,
    /// The pointer and the focus as the last frame left them (04 §7.1).
    pub(crate) pointer: Pointer<R::Press>,
    /// The press under the pointer when it went down.
    down: Option<R::Press>,
    /// A frame is wanted by something other than a dispatch: a tick, a note.
    pub(crate) wanted: bool,
    /// The frosted pages kept under the sheets drawn (K16); none outlives its sheet, a lock, a
    /// hide or the page.
    pub(crate) frosts: Vec<KeptCopy>,
    /// A hold was held to its end this event and its press is not yet sent.
    pub(crate) hold_ready: Option<R::Press>,
    /// The press of the hold sent for the contact now down: a contact sends it once.
    hold_sent: bool,
    /// The focused hold Enter or Space is held down on: it fills as a held pointer fills it.
    key_hold: Option<R::Press>,
    /// The region the fingers are moving.
    pan: Option<PanState>,
    /// Where a finger dragging a region last was, pixels: set while a drag goes on.
    drag_last: Option<Point>,
    /// Where the contact now down went down, pixels, while it has not moved past the slop.
    touch_from: Option<Point>,
    /// The frame clock at the last advance of the regions' own motion.
    motion_ms: Option<u64>,
    /// A frame was drawn that `Runtime::frame` has not handed out.
    fresh: bool,
    /// The ink record is on (04 V21).
    pub(crate) ink: bool,
    /// Tests that do not draw (the kit's drawing is not built when this is written).
    #[cfg(test)]
    pub(crate) skip: bool,
}

impl<R: Root> Painted<R> {
    pub(crate) fn new() -> Self {
        Painted {
            buffer: Secret::new(Vec::new()),
            size: (0, 0),
            calls: None,
            changed: Vec::new(),
            out: None,
            number: 0,
            orders: Vec::new(),
            sent: Vec::new(),
            pointer: Pointer {
                at: None,
                down: false,
                focused: None,
                hovered: None,
                pressed: None,
            },
            down: None,
            wanted: false,
            frosts: Vec::new(),
            hold_ready: None,
            hold_sent: false,
            key_hold: None,
            pan: None,
            drag_last: None,
            touch_from: None,
            motion_ms: None,
            fresh: false,
            ink: false,
            #[cfg(test)]
            skip: false,
        }
    }

    /// The frame, once, when one was drawn since the last call.
    pub(crate) fn handed(&mut self) -> Option<port::Frame<'_>> {
        if !self.fresh {
            return None;
        }
        self.fresh = false;
        Some(port::Frame {
            width: self.size.0,
            height: self.size.1,
            rgba: self.buffer.expose().as_slice(),
            changed: &self.changed,
            moved: None,
            regions: &[],
            pace: self.pace(),
        })
    }

    /// What the last frame said about the next tick.
    pub(crate) fn pace(&self) -> Pace {
        self.out.as_ref().map_or(Pace::Rest, |out| out.pace)
    }

    /// Wipes and drops every kept frost (V19: a lock, a hide, a page left, a sheet closed).
    pub(crate) fn drop_frosts(&mut self) {
        for kept in &mut self.frosts {
            kept.wipe();
        }
        self.frosts.clear();
    }

    /// After a frame: the copies whose sheet is no longer drawn, whose page was left or whose
    /// size is not the display's are wiped; the frosts the frame made are kept unless the frame
    /// showed a secret (V19) or the scope is going (`keeping` clear). `len` is the buffer's size.
    fn keep_frosts(
        &mut self,
        out: &mut FrameOut<R::Press>,
        now_ms: u64,
        keeping: bool,
        len: usize,
    ) {
        let mut i = 0;
        while i < self.frosts.len() {
            let k = &self.frosts[i];
            let stays = keeping
                && k.page == out.page
                && k.pixels.expose().len() == len
                && out.sheets.contains(&k.sheet);
            if stays {
                i += 1;
            } else {
                let mut gone = self.frosts.swap_remove(i);
                gone.wipe();
            }
        }
        for made in out.frosts.drain(..) {
            let mut pixels = made.pixels;
            let new = keeping
                && !out.secret
                && pixels.len() == len
                && made.page == out.page
                && out.sheets.contains(&made.sheet)
                && !self.frosts.iter().any(|k| k.sheet == made.sheet);
            if new {
                self.frosts.push(KeptCopy {
                    sheet: made.sheet,
                    page: made.page,
                    taken_at: now_ms,
                    secret: false,
                    pixels: Secret::new(pixels),
                });
            } else {
                pixels.fill(0);
            }
        }
    }

    /// The press of a hold that is full under the contact now down, once per contact.
    fn finish_hold(&mut self) {
        let Some(down) = self.down.as_ref().or(self.key_hold.as_ref()) else {
            return;
        };
        let Some(out) = self.out.as_ref() else {
            return;
        };
        let top = out.hits.iter().map(|hit| hit.layer).max();
        let on_top = out
            .hits
            .iter()
            .any(|hit| Some(hit.layer) == top && hit.press == *down);
        if !self.hold_sent && on_top && out.held.contains(down) {
            self.hold_sent = true;
            self.hold_ready = Some(down.clone());
        }
    }

    /// Whether `press` is a hold's: one presses it by holding, never by a lift or a key.
    fn is_hold(&self, press: &R::Press) -> bool {
        self.out
            .as_ref()
            .is_some_and(|out| out.holds.contains(press))
    }

    /// The page the last frame drew, the sheets over it and the ink record, for `Seen`.
    pub(crate) fn last(&self) -> Option<&FrameOut<R::Press>> {
        self.out.as_ref()
    }

    /// The press under a point: the top layer drawn takes presses, and of its hits the last
    /// drawn wins (04 §7.1).
    fn hit_at(&self, at: Point) -> Option<R::Press> {
        let out = self.out.as_ref()?;
        let top = out.hits.iter().map(|hit| hit.layer).max()?;
        out.hits
            .iter()
            .rev()
            .find(|hit| hit.layer == top && inside(&hit.target, at))
            .map(|hit| hit.press.clone())
    }
}

fn inside(target: &Target, at: Point) -> bool {
    at.x >= target.x && at.x < target.x + target.w && at.y >= target.y && at.y < target.y + target.h
}

/// The notes waiting and the one on show (04 §10).
pub(crate) struct Notes {
    waiting: VecDeque<Note>,
    up: Option<Up>,
    next_id: u32,
}

struct Up {
    note: Note,
    id: u32,
    since_ms: u64,
}

impl Notes {
    pub(crate) fn new() -> Self {
        Notes {
            waiting: VecDeque::new(),
            up: None,
            next_id: 0,
        }
    }

    /// A note kept: it comes up at once when none is up. True when the note on show changed.
    pub(crate) fn queue(&mut self, note: Note, now_ms: u64) -> bool {
        if self.up.is_none() {
            self.raise(note, now_ms);
            true
        } else {
            self.waiting.push_back(note);
            false
        }
    }

    /// The next note comes up once the one before has been up for `TOAST_SHOW_MS`. True when the
    /// note on show changed.
    pub(crate) fn tick(&mut self, now_ms: u64) -> bool {
        let due = self
            .up
            .as_ref()
            .is_some_and(|up| now_ms >= up.since_ms.saturating_add(TOAST_SHOW_MS));
        if !due {
            return false;
        }
        match self.waiting.pop_front() {
            Some(next) => {
                self.raise(next, now_ms);
                true
            }
            None => false,
        }
    }

    fn raise(&mut self, note: Note, now_ms: u64) {
        self.next_id = self.next_id.wrapping_add(1);
        self.up = Some(Up {
            note,
            id: self.next_id,
            since_ms: now_ms,
        });
    }

    pub(crate) fn up(&self) -> Option<NoteUp<'_>> {
        self.up.as_ref().map(|up| NoteUp {
            note: &up.note,
            id: up.id,
        })
    }

    /// At a lock and an end: no note outlives the scope (§7.2).
    pub(crate) fn drop_all(&mut self) {
        self.waiting.clear();
        self.up = None;
    }
}

impl<R: Root> Runtime<R> {
    /// After an event: a frame when anything was dispatched or one is wanted (§2.5), and the
    /// laid facts it made handled as one event after, with at most one more frame.
    pub(crate) fn draw_after(&mut self) {
        if self.mode.is_none() {
            return;
        }
        if self.queue.dispatched == 0 && !self.frame.wanted {
            return;
        }
        let laid = self.draw(true);
        // A hold held to its end sends its press as any press is sent (V24's mark included).
        let held = self.frame.hold_ready.take();
        if laid.is_empty() && held.is_none() {
            return;
        }
        for press in laid {
            self.queue.push(Item::Msg(R::press(press)));
        }
        if let Some(press) = held {
            self.pressed(press);
        }
        let before = self.queue.dispatched;
        self.drain();
        if self.queue.dispatched > before {
            self.draw(false);
        }
    }

    /// §2.5, steps 1 to 7: build `View` and `Show`, call `R::frame`, finish, paint, send the
    /// laid facts, keep the frame. Returns the presses of the laid facts that changed, when
    /// `send` is set; the second frame of an event sends none (step 6).
    pub(crate) fn draw(&mut self, send: bool) -> Vec<R::Press> {
        // §5.3: the jobs' progress is read at each frame, so a job that reports none still shows.
        self.running = self.pool.running();
        #[cfg(test)]
        if self.frame.skip {
            self.frame.wanted = false;
            return Vec::new();
        }
        let Some(mode) = self.mode else {
            return Vec::new();
        };
        let started = self.started;
        let keeping = !matches!(self.phase, Phase::Locking | Phase::Ending);
        let Runtime {
            root,
            tops,
            facts,
            clock,
            running,
            readers,
            compute: _,
            motion,
            frame,
            memo,
            notes,
            ..
        } = &mut *self;
        let Some(lent_tops) = Tops::lend(tops) else {
            return Vec::new();
        };
        let fallback = shared::Settings::default();
        let settings: &shared::Settings = tops
            .settings
            .as_ref()
            .map_or(&fallback, |s| Borrow::<shared::Settings>::borrow(s));
        let readers: &shared::Empty = readers;
        let running: &[Running] = running;
        let lent = Lent {
            settings,
            session: tops
                .session
                .as_ref()
                .map_or(readers as &dyn session::Read, |s| s as &dyn session::Read),
            vaults: tops
                .vaults
                .as_ref()
                .map_or(readers as &dyn vaults::Read, |v| v as &dyn vaults::Read),
        };
        let scale = f32::from(settings.scale_pct) / 100.0;
        let view = build_view(lent, &facts.facts, mode, clock, running, started);
        let show = show_of(memo, notes.up());
        let mut canvas = Canvas::new(&facts.facts.display);
        // The kit's `Theme` is mapped from `Settings::appearance` (04 §12.2) once the kit holds
        // the scheme table; until then every frame is drawn in its default.
        // The press under the pointer and the press a button or a finger is down on, from the
        // frame seen: what a control asks to draw its hovered and pressed state (04 §7.1).
        let under = frame.pointer.at.and_then(|at| frame.hit_at(at));
        frame.pointer.pressed = frame.down.clone().or_else(|| frame.key_hold.clone());
        frame.pointer.hovered = if frame.pointer.down { None } else { under };
        let size =
            usize::from(facts.facts.display.width) * usize::from(facts.facts.display.height) * 4;
        let lent_frosts: Vec<KeptFrost<'_>> = frame
            .frosts
            .iter()
            .filter(|k| k.pixels.expose().len() == size)
            .map(|k| KeptFrost {
                sheet: k.sheet,
                page: k.page,
                pixels: k.pixels.expose().as_slice(),
            })
            .collect();
        let mut ui = Ui::new(
            &mut canvas,
            Theme::default(),
            scale,
            motion,
            &frame.pointer,
            frame.ink,
        );
        ui.set_kept(&lent_frosts);
        ui.set_now_ms(clock.now_ms);
        ui.size_class();
        root.frame(&lent_tops, &view, &show, &mut ui);
        let mut out = ui.finish();
        drop(lent_frosts);
        frame.keep_frosts(&mut out, clock.now_ms, keeping, size);

        let mut pending = Vec::new();
        for (anchor, laid, press) in &out.laid {
            let sent = frame.sent.iter().position(|(a, _)| a == anchor);
            if sent.is_some_and(|i| frame.sent[i].1 == *laid) {
                continue;
            }
            pending.push(press.clone());
            if send {
                match sent {
                    Some(i) => frame.sent[i].1 = *laid,
                    None => frame.sent.push((*anchor, *laid)),
                }
            }
        }
        debug_assert!(send || pending.is_empty(), "layout settles in one frame");
        frame.out = Some(out);
        frame.finish_hold();
        frame.wanted = false;
        frame.number += 1;
        frame.fresh = true;
        let display = &facts.facts.display;
        frame.paint(display.width, display.height, canvas.pixels().rgba);
        self.memo_frame_end();
        if send { pending } else { Vec::new() }
    }

    /// A press of the last frame becomes its top state's message; a hiding press (`Pressed::hides`)
    /// is recorded as `Mark::Hides` for the event (04 §11.1, 05 §9).
    fn pressed(&mut self, press: R::Press) {
        if faraday_atoms::Pressed::hides(&press) {
            self.marks.push(Mark::Hides);
        }
        self.queue.push(Item::Msg(R::press(press)));
    }

    /// A touch: the press goes down on the hit under it, and is sent when the contact lifts on
    /// the same hit (R9: against the last frame handed out, the one the person saw). A contact
    /// that moves past the slop is a pan of the region it went down in and presses nothing.
    fn touch(&mut self, at: Point, phase: TouchPhase) {
        self.frame.pointer.at = Some(at);
        match phase {
            TouchPhase::Down => {
                self.frame.pointer.down = true;
                self.frame.down = self.frame.hit_at(at);
                self.frame.hold_sent = false;
                self.frame.touch_from = Some(at);
                // A touch stops a coast where it is.
                if let Some((id, _, _)) = self.region_at(at)
                    && self.motion.stop_coast(id)
                {
                    self.frame.touch_from = None;
                    self.frame.down = None;
                }
                // A frame shows the press down: a hold begins to fill.
                self.frame.wanted = true;
            }
            TouchPhase::Move => {
                if !self.frame.pointer.down {
                    return;
                }
                if let Some(from) = self.frame.touch_from {
                    let slop = SLOP_UNITS * self.scale();
                    let (dx, dy) = ((at.x - from.x) as f32, (at.y - from.y) as f32);
                    if dx.hypot(dy) < slop {
                        return;
                    }
                    // A drag, not a press: the region it went down in follows the finger.
                    self.frame.touch_from = None;
                    self.frame.down = None;
                    self.frame.hold_sent = false;
                    self.frame.hold_ready = None;
                    self.frame.pan = None;
                    self.frame.drag_last = Some(from);
                    self.frame.wanted = true;
                }
                if let Some(last) = self.frame.drag_last {
                    self.frame.drag_last = Some(at);
                    self.pan(last, (last.y - at.y) as f32);
                }
            }
            TouchPhase::Up => {
                self.frame.pointer.down = false;
                self.frame.hold_sent = false;
                self.frame.hold_ready = None;
                self.frame.touch_from = None;
                self.frame.wanted = true;
                if self.frame.drag_last.take().is_some() {
                    self.release_pan();
                    self.frame.down = None;
                    return;
                }
                let down = self.frame.down.take();
                if let (Some(down), Some(up)) = (down, self.frame.hit_at(at))
                    && down == up
                    && !self.frame.is_hold(&up)
                {
                    self.pressed(up);
                }
            }
        }
    }

    /// Tab and Back-tab move the focus along the last frame's `reach` order and Enter presses
    /// the focused control; any other key is the root's (§2.2).
    fn key(&mut self, key: Key) {
        match key {
            Key::Tab | Key::BackTab => {
                self.frame.key_hold = None;
                self.frame.hold_sent = false;
                self.move_focus(key == Key::BackTab);
            }
            Key::Enter | Key::Char(' ') => match self.frame.pointer.focused.clone() {
                // A hold is pressed by holding it (F136): the key held down fills it over the
                // hold's time; a repeat of the key does not begin it again.
                Some(press) if self.frame.is_hold(&press) => {
                    if self.frame.key_hold.is_none() {
                        self.frame.key_hold = Some(press);
                        self.frame.hold_sent = false;
                        self.frame.wanted = true;
                    }
                }
                Some(press) if key == Key::Enter => self.pressed(press),
                _ => self.root_input(&Input::Key(key)),
            },
            _ => self.root_input(&Input::Key(key)),
        }
    }

    /// A key let go: the hold it was filling empties, and sends nothing if it was not full.
    fn key_up(&mut self, key: Key) {
        if matches!(key, Key::Enter | Key::Char(' ')) && self.frame.key_hold.take().is_some() {
            self.frame.hold_sent = false;
            self.frame.hold_ready = None;
            self.frame.wanted = true;
            return;
        }
        self.root_input(&Input::KeyUp(key));
    }

    fn move_focus(&mut self, back: bool) {
        let Some(out) = self.frame.out.as_ref() else {
            return;
        };
        let count = out.reach.len();
        if count == 0 {
            return;
        }
        let at = self.frame.pointer.focused.as_ref().and_then(|focused| {
            out.reach
                .iter()
                .position(|&i| out.hits.get(i).is_some_and(|hit| hit.press == *focused))
        });
        let next = match (at, back) {
            (None, false) => 0,
            (None, true) => count - 1,
            (Some(i), false) => (i + 1) % count,
            (Some(i), true) => (i + count - 1) % count,
        };
        let focus = out
            .reach
            .get(next)
            .and_then(|&i| out.hits.get(i))
            .map(|hit| hit.press.clone());
        self.frame.pointer.focused = focus;
        self.frame.wanted = true;
    }

    /// Input the last frame did not take is `Root::input`'s.
    fn root_input(&mut self, input: &Input) {
        let Some(tops) = Tops::lend(&self.tops) else {
            return;
        };
        if let Some(addr) = self.root.input(&tops, input) {
            self.queue.push(Item::Msg(addr));
        }
    }

    /// §2.2: what an `Input` becomes. Every `Input` is dropped while the scope is locking or
    /// ending.
    pub(crate) fn input(&mut self, input: Input) {
        if matches!(self.phase, Phase::Locking | Phase::Ending) {
            return;
        }
        self.clock.last_input_ms = self.clock.now_ms;
        match input {
            Input::Touch { at, phase } => self.touch(at, phase),
            Input::Hover(at) => self.frame.pointer.at = at,
            // The motion store moves against the last frame's regions (04 §8.1): no message.
            Input::Scroll { at, dy } => self.pan(at, f32::from(dy)),
            Input::Wheel { at, dy } => {
                self.scroll(
                    at,
                    f32::from(dy) / WHEEL_NOTCH_PX * WHEEL_NOTCH_UNITS * self.scale(),
                );
            }
            Input::ScrollEnd { .. } => self.release_pan(),
            Input::Key(key) => self.key(key),
            Input::KeyUp(key) => self.key_up(key),
            other => self.root_input(&other),
        }
    }

    /// Pixels per design unit, as the person's setting has it.
    fn scale(&self) -> f32 {
        let pct = self
            .tops
            .settings
            .as_ref()
            .map_or(100, |s| Borrow::<shared::Settings>::borrow(s).scale_pct);
        f32::from(pct) / 100.0
    }

    /// The region of the top layer under `at`: its entry, how far it scrolls and how tall it is
    /// seen, in units.
    fn region_at(&self, at: Point) -> Option<(ComponentId, f32, f32)> {
        let f = self.scale().max(0.1);
        let out = self.frame.out.as_ref()?;
        let top = out.hits.iter().map(|hit| hit.layer).max().unwrap_or(0);
        let region = out
            .regions
            .iter()
            .rev()
            .find(|region| region.layer == top && region.view.contains(at.x, at.y))?;
        let max = (region.extent.h - region.view.h).max(0) as f32 / f;
        Some((
            ComponentId(out.page, region.id),
            max,
            region.view.h as f32 / f,
        ))
    }

    /// Moves the region of the top layer under `at` by `dy` pixels (down is positive): the motion
    /// store's offset and the time its scrollbar shows from are set from the frame's clock when
    /// the offset changes (04 §8.1). A wheel: it takes the content there and stops any coast.
    fn scroll(&mut self, at: Point, dy: f32) {
        let f = self.scale().max(0.1);
        let Some((id, max, _)) = self.region_at(at) else {
            return;
        };
        if self
            .motion
            .scroll_region(id, dy / f, max, self.clock.now_ms)
        {
            self.frame.wanted = true;
        }
    }

    /// The fingers moved the region under `at` by `dy` pixels: it follows them at once and
    /// stretches past an end; the region the pan began in keeps it.
    fn pan(&mut self, at: Point, dy: f32) {
        let f = self.scale().max(0.1);
        let now_ms = self.clock.now_ms;
        let state = match self.frame.pan {
            Some(state) => state,
            None => {
                let Some((id, max, view)) = self.region_at(at) else {
                    return;
                };
                PanState {
                    id,
                    max,
                    view,
                    began: true,
                    last_ms: now_ms,
                }
            }
        };
        let pan = Pan {
            by: dy / f,
            view: state.view,
            began: state.began,
        };
        self.frame.pan = Some(PanState {
            began: false,
            last_ms: now_ms,
            ..state
        });
        if self.motion.pan_region(state.id, pan, state.max, now_ms) {
            self.frame.wanted = true;
        }
    }

    /// The fingers lifted: a stretch springs back, a flick coasts on.
    fn release_pan(&mut self) {
        let f = self.scale().max(0.1);
        if let Some(state) = self.frame.pan.take()
            && self.motion.release_region(state.id, f, self.clock.now_ms)
        {
            self.frame.wanted = true;
        }
    }

    /// At each tick of the frame clock: a pan that went quiet has ended, and what coasts or
    /// springs moves by the time since the last tick. A frame is wanted while any does.
    pub(crate) fn advance_motion(&mut self, now_ms: u64) {
        if self
            .frame
            .pan
            .is_some_and(|p| now_ms.saturating_sub(p.last_ms) > SCROLL_QUIET_MS)
        {
            self.release_pan();
        }
        let last = self.frame.motion_ms.replace(now_ms);
        let dt = match last {
            Some(t) if now_ms.saturating_sub(t) <= 4 * MAX_STEP_MS => now_ms.saturating_sub(t),
            _ => FIRST_STEP_MS,
        };
        let f = self.scale().max(0.1);
        let panning = self.frame.pan.map(|p| p.id);
        if self.motion.advance(dt, f, panning) {
            self.frame.wanted = true;
        }
    }

    /// §7.2 step 7: the buffer zeroed, the call list and the frame's output dropped.
    pub(crate) fn wipe_frame(&mut self) {
        self.frame.buffer.expose_mut().fill(0);
        self.frame.drop_frosts();
        self.frame.hold_ready = None;
        self.frame.hold_sent = false;
        self.frame.key_hold = None;
        self.frame.pan = None;
        self.frame.drag_last = None;
        self.frame.touch_from = None;
        self.frame.calls = None;
        self.frame.out = None;
        self.frame.changed.clear();
        self.frame.orders.clear();
        self.frame.sent.clear();
        self.frame.down = None;
        self.frame.pointer.focused = None;
        self.frame.pointer.down = false;
    }
}

impl<R: Root> Painted<R> {
    /// The canvas replays its call list into the buffer, recording the changed rectangles. The
    /// replay is the kit's; the buffer is made the display's size and the whole of it is
    /// recorded as changed.
    fn paint(&mut self, width: u16, height: u16, drawn: &[u8]) {
        let len = usize::from(width) * usize::from(height) * 4;
        if self.size != (width, height) {
            let buffer = self.buffer.expose_mut();
            buffer.fill(0);
            buffer.clear();
            buffer.resize(len, 0);
            self.size = (width, height);
        }
        // The canvas paints as it is drawn on: its pixels are the frame, premultiplied.
        if drawn.len() == len {
            self.buffer.expose_mut().copy_from_slice(drawn);
        }
        self.changed.clear();
        self.changed.push(port::frame::Rect {
            x: 0,
            y: 0,
            w: i32::from(width),
            h: i32::from(height),
        });
    }
}
