//! The wiping frame buffer, painting the call list, the last `FrameOut` (05 §2.5).

use core::borrow::Borrow;
use std::collections::VecDeque;

use faraday_atoms::{Anchor, FrameOut, Laid, Pointer, Theme, Ui};
use faraday_crypto::Secret;
use faraday_draw::canvas::{Calls, Canvas};
use faraday_draw::geom::Rect as Target;
use faraday_kernel::{Note, NoteUp, Running, ViewOrder, session, shared, vaults};
use faraday_port as port;
use faraday_port::event::{Input, Key, TouchPhase};
use faraday_port::values::{Pace, Point};

use crate::facts::{Lent, build_view, default_settings, show_of};
use crate::queue::Item;
use crate::root::{Root, Tops};
use crate::runtime::{Phase, Runtime};

/// How long a note stays up before the next one comes up, by the runtime's clock (04 §10).
const TOAST_SHOW_MS: u64 = 4000;

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
            },
            down: None,
            wanted: false,
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
        if laid.is_empty() {
            return;
        }
        for press in laid {
            self.queue.push(Item::Msg(R::press(press)));
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
        #[cfg(test)]
        if self.frame.skip {
            self.frame.wanted = false;
            return Vec::new();
        }
        let Some(mode) = self.mode else {
            return Vec::new();
        };
        let started = self.started;
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
        let fallback = default_settings();
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
        let mut canvas = Canvas(());
        // The kit's `Theme` is mapped from `Settings::appearance` (04 §12.2) once the kit holds
        // the scheme table; until then every frame is drawn in its default.
        let mut ui = Ui::new(
            &mut canvas,
            Theme::default(),
            scale,
            motion,
            &frame.pointer,
            frame.ink,
        );
        root.frame(&lent_tops, &view, &show, &mut ui);
        let out = ui.finish();

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
        frame.wanted = false;
        frame.number += 1;
        frame.fresh = true;
        let display = &facts.facts.display;
        frame.paint(display.width, display.height);
        self.memo_frame_end();
        if send { pending } else { Vec::new() }
    }

    /// A touch: the press goes down on the hit under it, and is sent when the contact lifts on
    /// the same hit (R9: against the last frame handed out, the one the person saw).
    fn touch(&mut self, at: Point, phase: TouchPhase) {
        self.frame.pointer.at = Some(at);
        match phase {
            TouchPhase::Down => {
                self.frame.pointer.down = true;
                self.frame.down = self.frame.hit_at(at);
            }
            TouchPhase::Move => {}
            TouchPhase::Up => {
                self.frame.pointer.down = false;
                let down = self.frame.down.take();
                if let (Some(down), Some(up)) = (down, self.frame.hit_at(at))
                    && down == up
                {
                    self.queue.push(Item::Msg(R::press(up)));
                }
            }
        }
    }

    /// Tab and Back-tab move the focus along the last frame's `reach` order and Enter presses
    /// the focused control; any other key is the root's (§2.2).
    fn key(&mut self, key: Key) {
        match key {
            Key::Tab | Key::BackTab => self.move_focus(key == Key::BackTab),
            Key::Enter => match self.frame.pointer.focused.clone() {
                Some(press) => self.queue.push(Item::Msg(R::press(press))),
                None => self.root_input(&Input::Key(key)),
            },
            _ => self.root_input(&Input::Key(key)),
        }
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
            Input::Scroll { .. } | Input::Wheel { .. } | Input::ScrollEnd { .. } => {}
            Input::Key(key) => self.key(key),
            other => self.root_input(&other),
        }
    }

    /// §7.2 step 7: the buffer zeroed, the call list and the frame's output dropped.
    pub(crate) fn wipe_frame(&mut self) {
        self.frame.buffer.expose_mut().fill(0);
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
    fn paint(&mut self, width: u16, height: u16) {
        let len = usize::from(width) * usize::from(height) * 4;
        if self.size != (width, height) {
            let buffer = self.buffer.expose_mut();
            buffer.fill(0);
            buffer.clear();
            buffer.resize(len, 0);
            self.size = (width, height);
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
