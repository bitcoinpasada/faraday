//! An adapter: frames, NV12, scaling, the list (09 §6).

pub mod nv12;
pub mod scale;

/// One captured frame: luma, and the NV12 chroma plane when the device gives one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: u16,
    pub height: u16,
    pub luma: Vec<u8>,
    pub chroma: Option<Vec<u8>>,
}

impl Frame {
    #[must_use]
    pub fn new(_width: u16, _height: u16, _luma: Vec<u8>, _chroma: Option<Vec<u8>>) -> Frame {
        todo!()
    }

    /// A frame from an NV12 plane, scaled down to `max_width` at most.
    #[must_use]
    pub fn from_nv12(
        _plane: &[u8],
        _width: usize,
        _height: usize,
        _stride: usize,
        _chroma: Option<(&[u8], usize)>,
        _max_width: usize,
    ) -> Option<Frame> {
        todo!()
    }
}

/// Why a camera gave no frames, when it is clearly one of three things.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Camera access is denied or restricted, or the person said no.
    Refused,
    /// Another app holds the camera.
    InUse,
    /// There is no camera, or the one asked for is gone.
    NotFound,
}

/// The cameras this machine has, as `(path, name)`.
#[must_use]
pub fn list() -> Vec<(String, String)> {
    todo!()
}
