//! An adapter: framebuffer, evdev and touch, held-back input, the arrow, turns, sticks attached,
//! the clean marker, the read-only root, the dev reports, the capture thread
//! (`ReadMode::Reduced`) (09 §6).

use std::sync::Arc;

use faraday_port::{Adapters, Effect, Event, Failure, Waker};
use faraday_storage::Storage;
use faraday_text::Text;

/// The stick shell's command line, as the adapter takes it.
pub struct StickOptions {
    pub fb: Text,
    pub size: Option<(u16, u16)>,
    pub depth: u8,
    pub dpi: u16,
    pub native: bool,
    pub mount: Text,
    pub pointer_speed: u8,
    pub touch_grid: bool,
    pub files: Text,
    pub verbose: bool,
    pub timings: bool,
    pub boot_report: bool,
}

/// Every device adapter but storage.
pub struct Stick {
    _private: (),
}

impl Stick {
    pub fn open(_o: StickOptions, _waker: Arc<dyn Waker>) -> Result<Self, Failure> {
        todo!()
    }
}

impl Adapters for Stick {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}

/// The device's adapters behind one value: the stick's and the storage adapter's.
pub struct DeviceAdapters {
    pub stick: Stick,
    pub storage: Storage,
}

impl Adapters for DeviceAdapters {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}
