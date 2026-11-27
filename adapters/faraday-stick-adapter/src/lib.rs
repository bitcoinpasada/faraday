//! An adapter: framebuffer, evdev and touch, held-back input, the arrow, turns, sticks attached,
//! the clean marker, the read-only root, the dev reports, the capture thread
//! (`ReadMode::Reduced`) (09 §6).
//!
//! The stick root builds a [`Stick`] and a storage adapter and puts them behind one
//! [`DeviceAdapters`]. Beside the port's `carry` and `poll` the root calls two seams the port does
//! not name: [`Stick::present`], which puts a frame on the panel, and [`Stick::ended`], which says
//! the status the process ends with.
//!
//! Held-back input (`DevicesFact`, `AimInput`, `DecideInput`, `WatchDevices`) fails closed: a
//! device that is not part of the machine and was not plugged in while the start window was open
//! sends nothing, and the three orders are not acted on.
#![allow(
    dead_code,
    reason = "the copied files hold the frame grid, the clock boost and the timing hooks whose \
              caller is the runtime's pass"
)]

mod attached;
mod clock;
mod cursor;
mod devnodes;
mod evdev;
mod fb;
mod files;
mod hid;
mod keyboard;
mod machine;
mod notice;
mod orient;
mod pace;
mod panel;
mod pointer;
mod present;
mod refresh;
mod start_window;
mod stick;
mod timings;
mod touch;
mod usb;
mod wakechan;
mod writer;

use std::sync::{Arc, Mutex};

use faraday_port::event::Key;
use faraday_port::{Adapters, Ask, Effect, Event, Waker};
use faraday_storage::Storage;
use faraday_text::Text;

use crate::cursor::{Cursor, Pointed};
use crate::panel::{Panel, lock};

pub use crate::stick::Stick;

/// The stick shell's command line, as the adapter takes it.
///
/// Where an option cannot say "not given" (`depth` 0, `dpi` 0, `pointer_speed` 0 for the default
/// of 10, `mount` empty) it is the unsaid value; `native` true is a panel that is portrait
/// natively, false is "read it from the framebuffer"; `touch_grid` true is the default grid.
pub struct StickOptions {
    pub fb: Text,
    pub size: Option<(u16, u16)>,
    pub depth: u8,
    pub dpi: u16,
    pub native: bool,
    pub mount: Text,
    /// Tenths of a device pixel per unit of pointer motion; 0 is 10.
    pub pointer_speed: u8,
    pub touch_grid: bool,
    pub files: Text,
    pub verbose: bool,
    pub timings: bool,
    pub boot_report: bool,
}

/// What woke the loop.
///
/// One ring carries all of it, so the loop is woken by whichever source is ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wake {
    /// One contact from a touchscreen, already in panel pixels.
    Touch(touch::Touch),
    /// One motion from a mouse or a touchpad, in device units: the cursor turns it into a touch.
    Pointer(pointer::Motion),
    /// What a pointer move the input thread applied as it read it meant ([`Arrow::moved`]): the
    /// arrow is already where it is.
    Pointed(Option<Pointed>),
    /// One key from a keyboard, going down.
    Key(Key),
    /// One key from a keyboard, coming up.
    KeyUp(Key),
    /// An input reader has ended: the device is gone, or a file has been read to the end.
    InputEnded(u32),
    /// A frame was queued by the capture thread (the camera, which this adapter does not carry).
    Frame,
    /// A worker has an answer.
    Work,
    /// What a device not yet believed sent, held back.
    Held(u32, HeldInput),
    /// The panic chord, Super and S, closed (`true`) or opened again.
    Panic(bool),
}

/// What a held-back device sends: the input [`Wake`]s, by themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeldInput {
    Touch(touch::Touch),
    Pointer(pointer::Motion),
    Key(Key),
    KeyUp(Key),
    Panic(bool),
}

impl HeldInput {
    /// The input a reader produced, held; anything else is not input.
    pub(crate) fn of(wake: Wake) -> Option<HeldInput> {
        match wake {
            Wake::Touch(t) => Some(HeldInput::Touch(t)),
            Wake::Pointer(m) => Some(HeldInput::Pointer(m)),
            Wake::Key(k) => Some(HeldInput::Key(k)),
            Wake::KeyUp(k) => Some(HeldInput::KeyUp(k)),
            Wake::Panic(on) => Some(HeldInput::Panic(on)),
            _ => None,
        }
    }
}

/// What an input thread needs to move the arrow itself: the cursor it moves and the panel it paints
/// on, both shared with the loop (`docs/MOTION.md` §3.2).
#[derive(Clone)]
pub(crate) struct Arrow {
    pub(crate) cursor: Arc<Mutex<Cursor>>,
    pub(crate) panel: Arc<Mutex<Panel>>,
}

impl Arrow {
    /// Applies one pointer move to the cursor and paints the arrow where it now is, on the thread
    /// that read the move, so the arrow follows the hand at the mouse's own rate. What the move
    /// meant (a hover, a drag) is the wake the loop gets in its place.
    pub(crate) fn moved(&self, motion: pointer::Motion) -> Wake {
        let (pointed, at) = {
            let mut c = lock(&self.cursor);
            let pointed = c.apply(motion);
            (pointed, c.visible().then(|| c.at()))
        };
        // A write that fails is noticed by the next frame, which writes the panel whole.
        let _ = lock(&self.panel).move_arrow(at);
        Wake::Pointed(pointed)
    }
}

/// The device's adapters behind one value: the stick's and the storage adapter's.
pub struct DeviceAdapters {
    stick: Stick,
    storage: Storage,
    started: bool,
}

impl DeviceAdapters {
    /// The two adapters as one. `waker` is the root's: the storage adapter's thread wakes the
    /// loop with it.
    #[must_use]
    pub fn new(stick: Stick, mut storage: Storage, waker: Arc<dyn Waker>) -> Self {
        storage.wake_with(waker);
        DeviceAdapters {
            stick,
            storage,
            started: false,
        }
    }

    /// The stick adapter, for the root's `present` and `ended`.
    pub fn stick(&mut self) -> &mut Stick {
        &mut self.stick
    }
}

impl Adapters for DeviceAdapters {
    /// `Storage`, `Keep`, `Forget` and `Memory` go to the storage adapter; everything else to the
    /// stick.
    fn carry(&mut self, effect: Effect) {
        match &effect {
            Effect::Ask(_, Ask::Storage(_) | Ask::Keep(_) | Ask::Forget | Ask::Memory) => {
                self.storage.carry(effect);
            }
            _ => self.stick.carry(effect),
        }
    }

    /// The stick's `Start` first, with what the last process carried across a lock; then the
    /// stick's events, then the storage adapter's.
    fn poll(&mut self) -> Option<Event> {
        if let Some(event) = self.stick.poll() {
            return Some(match event {
                Event::Start(mut start) => {
                    start.carried = self.storage.take_carried();
                    self.started = true;
                    Event::Start(start)
                }
                other => other,
            });
        }
        if self.started {
            self.storage.poll()
        } else {
            None
        }
    }
}
