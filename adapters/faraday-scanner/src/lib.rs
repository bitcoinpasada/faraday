//! An adapter: reading codes from frames on its own thread (09 §6; 02 §9).

use std::time::Duration;

mod scan;

pub use scan::{reduce, reduce_chroma, reduce_factor};

/// What the worker reads of each frame it takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// The reduced copy every pass, the full frame after a code seen and not read: the stick.
    Reduced,
    /// The full frame every pass: the desktop roots.
    Full,
}

/// What one pass saw of a code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seen {
    pub width: u16,
    pub height: u16,
    pub corners: [(u16, u16); 4],
    pub module_px: f32,
    pub read: bool,
    pub whole: bool,
}

/// The reading thread: frames offered, codes and sightings polled.
pub struct Scanner {
    _private: (),
}

impl Scanner {
    #[must_use]
    pub fn new(_mode: ReadMode) -> Scanner {
        todo!()
    }

    /// Offers a frame's luma; a frame still waiting is replaced.
    pub fn offer(&self, _width: u16, _height: u16, _luma: Vec<u8>) {
        todo!()
    }

    /// The bytes of a code the thread read, if one waits.
    #[must_use]
    pub fn poll(&self) -> Option<Vec<u8>> {
        todo!()
    }

    #[must_use]
    pub fn poll_seen(&self) -> Option<Seen> {
        todo!()
    }

    #[must_use]
    pub fn take_pass_times(&self) -> Vec<Duration> {
        todo!()
    }

    pub fn stop(&mut self) {
        todo!()
    }

    #[must_use]
    pub fn running(&self) -> bool {
        todo!()
    }
}

/// One frame's first code, and what was seen of it, at `scale` frame pixels a pixel of `luma`.
#[must_use]
pub fn look(
    _width: usize,
    _height: usize,
    _luma: &[u8],
    _scale: usize,
    _frame: (u16, u16),
) -> (Option<Vec<u8>>, Option<Seen>) {
    todo!()
}
