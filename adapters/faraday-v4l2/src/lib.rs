//! An adapter: V4L2 capture (09 §6).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;
use std::time::Duration;

pub use faraday_camera::Frame;

/// The most frames a second a `Camera` sends on.
pub const MAX_FPS: u8 = 10;

/// A capture size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub const VGA: Size = Size {
        width: 640,
        height: 480,
    };
    pub const HD: Size = Size {
        width: 1280,
        height: 720,
    };
    pub const FULL_HD: Size = Size {
        width: 1920,
        height: 1080,
    };
}

/// Why a camera did not open, as far as the kernel's error says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Failure {
    NotFound,
    InUse,
    Refused,
}

impl Failure {
    /// The reason an I/O error from opening or setting up a camera gives.
    #[must_use]
    pub fn of_io(_error: &io::Error) -> Option<Failure> {
        todo!()
    }
}

/// What a camera counted since it opened.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub dequeued: u32,
    pub flagged: u32,
    pub short: u32,
}

/// An open capture device; dropping it wipes and unmaps its buffers (P28).
pub struct Camera {
    _private: (),
}

impl Camera {
    pub fn open(_path: &Path, _want: Size) -> io::Result<Camera> {
        todo!()
    }

    #[must_use]
    pub fn size(&self) -> Size {
        todo!()
    }

    pub fn set_max_fps(&mut self, _fps: u8) {
        todo!()
    }

    #[must_use]
    pub fn stats(&self) -> Stats {
        todo!()
    }

    #[must_use]
    pub fn format(&self) -> String {
        todo!()
    }

    pub fn frame(&mut self, _timeout: Duration) -> io::Result<Option<Frame>> {
        todo!()
    }
}

/// The first capture device, if there is one.
#[must_use]
pub fn find() -> Option<PathBuf> {
    todo!()
}

pub fn find_or_failure() -> Result<PathBuf, Failure> {
    todo!()
}

/// Runs a camera on its own thread, sending each frame.
pub fn spawn(_camera: Camera, _tx: SyncSender<Frame>, _stop: Arc<AtomicBool>) -> JoinHandle<()> {
    todo!()
}
