//! An adapter: sticks as folders (the folder `Place` from `faraday-files`), `ToggleStick`,
//! Downloads, opening, the print folder, the camera list (09 §6).

use std::sync::Arc;

use faraday_files::{Place, Wiped, Zeroed};
use faraday_port::{Adapters, Effect, Event, Waker};
use faraday_text::Text;

/// The desktop's adapter: folder sticks and the host.
pub struct Desktop {
    _private: (),
}

impl Desktop {
    #[must_use]
    pub fn new(_sticks: Text, _downloads: Option<Text>, _waker: Arc<dyn Waker>) -> Self {
        todo!()
    }
}

impl Adapters for Desktop {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}

/// A folder standing for a stick: `Place` over `std::fs`.
pub struct FolderPlace {
    _private: (),
}

impl Place for FolderPlace {
    fn files(&mut self) -> Result<Vec<(Text, u64)>, Text> {
        todo!()
    }

    fn read(&mut self, _name: &str, _max: u64) -> Result<Vec<u8>, Text> {
        todo!()
    }

    fn create(&mut self, _name: &str, _bytes: &[u8]) -> Result<(), Text> {
        todo!()
    }

    fn remove(&mut self, _name: &str) -> Result<(), Text> {
        todo!()
    }

    fn wipe(&mut self, _name: &str) -> Result<Wiped, Text> {
        todo!()
    }

    fn rename(&mut self, _from: &str, _to: &str) -> Result<(), Text> {
        todo!()
    }

    fn space(&mut self) -> Result<(u64, u64), Text> {
        todo!()
    }

    fn zero_free(&mut self, _from: u32) -> Result<Zeroed, Text> {
        todo!()
    }

    fn make_empty(&mut self) -> Result<(), Text> {
        todo!()
    }
}
