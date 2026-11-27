//! An adapter: Media Foundation capture (09 §6).

use std::sync::mpsc::SyncSender;

pub use faraday_camera::{Failure, Frame};

/// The widest frame kept, in pixels.
pub const MAX_WIDTH: usize = 1280;

/// The most frames a second a `Camera` sends on.
pub const MAX_FPS: u8 = 10;

/// A frame from a plane Media Foundation gave.
#[must_use]
pub fn from_plane(
    _plane: &[u8],
    _width: usize,
    _height: usize,
    _stride: usize,
    _chroma: Option<(&[u8], usize)>,
) -> Option<Frame> {
    todo!()
}

#[must_use]
pub fn failure_of_hresult(_hr: u32) -> Option<Failure> {
    todo!()
}

#[must_use]
pub fn failure_of_status(_status: i32) -> Option<Failure> {
    todo!()
}

/// What a running `Camera` sends.
#[derive(Debug, PartialEq, Eq)]
pub enum Message {
    Frame(Frame),
    Unavailable(Option<Failure>),
}

/// A video capture device, as the scan sheet offers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub name: String,
    pub link: String,
}

#[must_use]
pub fn devices() -> Vec<Device> {
    todo!()
}

pub struct Camera {
    _private: (),
}

impl Camera {
    pub fn start(_link: Option<&str>, _tx: SyncSender<Message>) -> Camera {
        todo!()
    }
}
