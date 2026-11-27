//! The camera's asks and value types (02 §3.7, §4.4).

use faraday_text::Text;

use crate::ask::{AskId, request};
use crate::bytes::Bytes;

/// The adapter's name for a camera.
// Spec: 02 §4.4 leaves this open (the id's type).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CameraId(pub Text);

/// A camera an adapter can open: its id and the name it gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraInfo {
    pub id: CameraId,
    pub name: Text,
}

/// Starts a stream on the chosen camera, or the adapter's first.
pub struct On {
    pub camera: Option<CameraId>,
}
request!(On => Result<Started, Failure>);

/// A stream is on: named by the `AskId` of the `On` that started it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Started {
    pub stream: AskId,
}

/// Why the camera gives no frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Refused,
    InUse,
    NotFound,
    Unavailable,
    Unsupported,
}

pub struct CameraFrame {
    pub width: u16,
    pub height: u16,
    pub luma: Bytes,
    /// NV12's plane; a grey preview without it.
    pub chroma: Option<Bytes>,
}

/// What one scan pass saw of a code.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QrSeen {
    /// The frame's size.
    pub width: u16,
    pub height: u16,
    /// The code's corners in the frame: top-left, top-right, bottom-right, bottom-left.
    pub corners: [(u16, u16); 4],
    /// One module's side, in frame pixels.
    pub module_px: f32,
    /// The pass read it.
    pub read: bool,
    /// The pass looked at the whole frame, not a reduced copy of it.
    pub whole: bool,
}
