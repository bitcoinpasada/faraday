//! Whether Faraday on the stick could use this computer's camera
//! (`docs/HARDWARE.md` §4, `docs/STARTER.md` §7.1): a video capture device
//! driven by `uvcvideo` whose USB descriptors offer an uncompressed
//! format. Read from sysfs only, no ioctl.

use faraday_starter::api::Camera;
use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The driver the stick's kernel uses for USB video (the name
/// `faraday/shells/v4l2` expects).
const DRIVER: &str = "uvcvideo";

/// USB descriptor types and the USB Video Class values read here (USB
/// Video Class 1.5, §3.9.2.3 and Table A-6).
const TYPE_INTERFACE: u8 = 0x04;
const TYPE_CS_INTERFACE: u8 = 0x24;
const CLASS_VIDEO: u8 = 0x0e;
const SUBCLASS_STREAMING: u8 = 0x02;
const VS_FORMAT_UNCOMPRESSED: u8 = 0x04;

/// The camera as the stick would find it, with `/` at `root`. Each
/// camera counts once, however many video nodes it has: only a node with
/// index 0 is a capture device (a UVC metadata node has a higher one).
pub fn camera(root: &Path) -> Camera {
    let class = root.join("sys/class/video4linux");
    let nodes = match std::fs::read_dir(&class) {
        Ok(d) => d,
        // No video4linux class: no camera, if sysfs itself is there.
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return if root.join("sys/class").is_dir() {
                Camera::None
            } else {
                Camera::Unknown
            };
        }
        Err(_) => return Camera::Unknown,
    };
    let mut cameras = BTreeSet::new();
    let mut usable = false;
    for node in nodes {
        let Ok(node) = node else {
            return Camera::Unknown;
        };
        let dir = node.path();
        if !node.file_name().to_string_lossy().starts_with("video") || !is_capture(&dir) {
            continue;
        }
        // The same camera under another node is not counted again.
        let device = std::fs::canonicalize(dir.join("device")).unwrap_or(dir);
        if cameras.insert(device.clone()) && offers_uncompressed(&device) {
            usable = true;
        }
    }
    match (usable, cameras.is_empty()) {
        (true, _) => Camera::Usable,
        (false, true) => Camera::None,
        (false, false) => Camera::NotUsable,
    }
}

/// A node with index 0 (or no index file) is a capture device.
fn is_capture(node: &Path) -> bool {
    std::fs::read_to_string(node.join("index")).map_or(true, |s| s.trim() == "0")
}

/// The camera's interface is driven by `uvcvideo` and its USB device
/// (the interface's parent) lists an uncompressed format.
fn offers_uncompressed(interface: &Path) -> bool {
    let driven = std::fs::read_link(interface.join("driver"))
        .ok()
        .and_then(|d| d.file_name().map(|n| n == DRIVER))
        .unwrap_or(false);
    let descriptors: Option<PathBuf> = interface.parent().map(|p| p.join("descriptors"));
    driven
        && descriptors
            .and_then(|p| std::fs::read(p).ok())
            .is_some_and(|d| has_uncompressed(&d))
}

/// Whether USB descriptors (`descriptors` in sysfs: the device descriptor,
/// then each configuration with its interfaces) hold, in a VideoStreaming
/// interface, a VS_FORMAT_UNCOMPRESSED descriptor (subtype 0x04, 27 bytes,
/// with at least one frame descriptor). MJPEG (0x06), frame-based (0x10)
/// and the rest do not count. A descriptor that is shorter than its
/// header or runs past the end stops the walk with `false`.
pub fn has_uncompressed(descriptors: &[u8]) -> bool {
    let mut at = 0usize;
    let mut streaming = false;
    while at < descriptors.len() {
        let len = usize::from(descriptors[at]);
        let Some(d) = at
            .checked_add(len)
            .and_then(|end| descriptors.get(at..end))
            .filter(|d| d.len() >= 2)
        else {
            return false;
        };
        match d[1] {
            TYPE_INTERFACE => {
                streaming = d.len() >= 9 && d[5] == CLASS_VIDEO && d[6] == SUBCLASS_STREAMING;
            }
            TYPE_CS_INTERFACE
                if streaming && d.len() >= 27 && d[2] == VS_FORMAT_UNCOMPRESSED && d[4] > 0 =>
            {
                return true;
            }
            _ => {}
        }
        at += len;
    }
    false
}
