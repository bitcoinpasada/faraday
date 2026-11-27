//! An adapter: the window, mouse, touch, keys, clipboard, `MovesWindow`, OS randomness, the lens
//! (`ReadMode::Full`) (09 §6).

use std::sync::Arc;

use faraday_port::{Adapters, Effect, Event, Failure, Frame, Waker};
use faraday_text::Text;

/// The window's command line, as the adapter takes it.
pub struct WindowOptions {
    pub size: Option<(u16, u16)>,
    pub panel: Option<Text>,
    pub aspect: Option<Text>,
    pub ppi: Option<u16>,
    pub timings: bool,
    pub no_camera: bool,
}

/// The window, its input, the clipboard, the randomness and the lens.
pub struct Window {
    _private: (),
}

impl Window {
    pub fn open(_o: WindowOptions, _waker: Arc<dyn Waker>) -> Result<Self, Failure> {
        todo!()
    }

    /// Shows a frame of the runtime's.
    pub fn paint(&mut self, _f: &Frame<'_>) {
        todo!()
    }
}

impl Adapters for Window {
    fn carry(&mut self, _effect: Effect) {
        todo!()
    }

    fn poll(&mut self) -> Option<Event> {
        todo!()
    }
}
