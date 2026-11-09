//! What the camera shells share, in safe code: the captured [`Frame`],
//! the copy of an NV12 buffer's two planes into one ([`Frame::from_nv12`]
//! with [`nv12`] and [`scale`]), the reasons a camera gave no frames
//! ([`Failure`]), and the list of Linux cameras ([`list`]).
//!
//! The capture itself, which is `unsafe`, stays in `faraday-v4l2`,
//! `faraday-avfoundation` and `faraday-mediafoundation`; each passes the
//! widest frame it hands the core.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod nv12;
pub mod scale;

/// One captured frame: 8-bit luma, row-major, `width × height` bytes,
/// and the NV12 chroma plane for it where the device gave colour, which
/// is exactly the payload of `faraday_shell_api::Event::CameraFrame`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Frame width in pixels.
    pub width: u16,
    /// Frame height in pixels.
    pub height: u16,
    /// The pixels, black 0 to white 255.
    pub luma: Vec<u8>,
    /// `⌈width / 2⌉ × ⌈height / 2⌉` pairs of interleaved Cb and Cr (U
    /// and V), or `None` when the device gave no colour.
    pub chroma: Option<Vec<u8>>,
}

impl Frame {
    /// Copies an NV12 buffer's two planes into a frame: the `stride` the
    /// device lays each one's rows out at is honoured, and a frame wider
    /// than `max_width` pixels is shrunk by an integer factor.
    ///
    /// `chroma` is plane 1 and its own bytes per row. `None`, or a plane
    /// shorter than the rows it claims, leaves the preview grey.
    ///
    /// `None` when the luma plane is shorter than the rows it claims, or
    /// when the result would not fit a `u16`.
    #[must_use]
    pub fn from_nv12(
        plane: &[u8],
        width: usize,
        height: usize,
        stride: usize,
        chroma: Option<(&[u8], usize)>,
        max_width: usize,
    ) -> Option<Frame> {
        let luma = nv12::luma(plane, width, height, stride)?;
        let factor = scale::scale_factor(width, max_width);
        let uv = chroma
            .and_then(|(plane, stride)| nv12::chroma(plane, width, height, stride))
            .map(|uv| scale::downscale_chroma(uv, width, height, factor));
        let (width, height, luma) = scale::downscale(luma, width, height, factor);
        Some(Frame {
            width: u16::try_from(width).ok()?,
            height: u16::try_from(height).ok()?,
            luma,
            chroma: uv,
        })
    }
}

/// Why a camera gave no frames, when it is clearly one of three things.
/// The macOS and Windows shells use the same type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// Camera access is denied or restricted, or the person said no.
    Refused,
    /// Another app holds the camera.
    InUse,
    /// There is no camera, or the one asked for is gone.
    NotFound,
}

/// The cameras this machine has, as `(path, name)`: every
/// `/dev/videoN` whose sysfs node is its device's first (`index` 0), which
/// is how a UVC camera's metadata node and a Pi's ISP nodes are told from
/// the capture node without opening any of them. The name is the one the
/// device gives.
pub fn list() -> Vec<(String, String)> {
    let mut out: Vec<(u32, String, String)> = Vec::new();
    let Ok(dir) = std::fs::read_dir("/sys/class/video4linux") else {
        return Vec::new();
    };
    for e in dir.flatten() {
        let node = e.file_name().to_string_lossy().into_owned();
        let Some(n) = node
            .strip_prefix("video")
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let read = |f: &str| {
            std::fs::read_to_string(e.path().join(f))
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        if read("index").parse::<u32>().unwrap_or(0) != 0 {
            continue;
        }
        let name = match read("name") {
            n if n.is_empty() => node.clone(),
            n => n,
        };
        out.push((n, format!("/dev/{node}"), name));
    }
    out.sort();
    out.into_iter().map(|(_, p, n)| (p, n)).collect()
}
