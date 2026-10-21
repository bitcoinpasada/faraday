//! Media Foundation capture for the Windows shells (the desktop app and
//! the Starter): list the cameras, start one, and hand back 8-bit luma
//! frames with the colour beside them. The Windows twin of
//! `faraday-avfoundation`, with the same [`Frame`], [`Message`] and
//! [`MAX_FPS`].
//!
//! # Why this crate may use `unsafe`
//!
//! The workspace sets `unsafe_code = "forbid"`. This crate is one of the
//! exceptions, with `faraday-v4l2` and `faraday-avfoundation`, and
//! its `Cargo.toml` says so rather than inheriting the workspace lints.
//! The reason is `docs/PLANNING.md` §16.33: each platform reaches its
//! camera through the operating system's own interface with code we
//! write and can read, and on Windows that is Media Foundation, "the same
//! one-small-file pattern" as AVFoundation on macOS.
//!
//! Media Foundation is COM, which Rust cannot call without either a
//! binding crate (`windows`, tens of thousands of generated lines) or C.
//! The capture is therefore `src/camera.c`, a few hundred lines with its
//! comments, behind a two-function C ABI. Calling it needs two
//! `extern "C"` declarations and two callbacks, which have no safe form
//! in `std`. Every `unsafe` block below carries a `// SAFETY:` comment
//! naming the invariant it relies on, and the public API — [`Camera`],
//! [`devices`], [`Device`], [`Frame`], [`Message`] — is entirely safe: no
//! raw pointer and no pixel buffer crosses it.
//!
//! # Platforms
//!
//! Everything that touches Media Foundation is behind
//! `#[cfg(target_os = "windows")]`, and the build script compiles
//! `src/camera.c` only when the target is Windows. On Linux and macOS
//! this crate is [`Device`], [`Frame`], [`Message`] and their tests, and
//! the build runs no compiler but `rustc`.
//!
//! # Shape
//!
//! ```no_run
//! # #[cfg(target_os = "windows")] {
//! use std::sync::mpsc::sync_channel;
//! use faraday_mediafoundation::{Camera, Message};
//!
//! let (tx, rx) = sync_channel(2);
//! let camera = Camera::start(None, tx);
//! // ... rx.try_recv() for frames, until:
//! drop(camera);
//! # }
//! ```
//!
//! Capture runs on the camera's own thread, so `Camera::start` cannot
//! fail synchronously: when there is no camera, or Windows' privacy
//! setting refuses this program the camera, the first message is
//! [`Message::Unavailable`]; otherwise it is a [`Message::Frame`].

// The NV12 conversion is faraday-avfoundation's: the same buffer
// layout on both systems.
use faraday_avfoundation::convert;

/// Why a camera gave no frames, when it is clearly one of three things:
/// the type `faraday-avfoundation` has, shared by both crates.
pub use faraday_avfoundation::Failure;

/// `osk_mf_capture`'s `OSK_MF_NO_CAMERA`.
const STATUS_NO_CAMERA: i32 = 1;

/// Windows' camera privacy setting refused the program
/// (`E_ACCESSDENIED`).
const E_ACCESSDENIED: u32 = 0x8007_0005;
/// A source or reader that could not start streaming, as when another
/// program has the camera (`MF_E_HW_MFT_FAILED_START_STREAMING`), or a
/// sharing violation or busy device from the driver
/// (`HRESULT_FROM_WIN32` of `ERROR_SHARING_VIOLATION`, `ERROR_BUSY`).
const IN_USE: [u32; 3] = [0xC00D_3704, 0x8007_0020, 0x8007_00AA];
/// The device is not there (`HRESULT_FROM_WIN32` of
/// `ERROR_FILE_NOT_FOUND`, `ERROR_DEVICE_NOT_CONNECTED`).
const GONE: [u32; 2] = [0x8007_0002, 0x8007_048F];

/// The reason an HRESULT gives for a camera that did not open or stream,
/// or `None` for any other.
#[must_use]
pub fn failure_of_hresult(hr: u32) -> Option<Failure> {
    if hr == E_ACCESSDENIED {
        Some(Failure::Refused)
    } else if IN_USE.contains(&hr) {
        Some(Failure::InUse)
    } else if GONE.contains(&hr) {
        Some(Failure::NotFound)
    } else {
        None
    }
}

/// The reason `osk_mf_capture`'s status gives: 1 is no camera, a
/// negative status is an HRESULT, and any other (a failure with no
/// HRESULT) has none.
#[must_use]
pub fn failure_of_status(status: i32) -> Option<Failure> {
    if status == STATUS_NO_CAMERA {
        Some(Failure::NotFound)
    } else if status < 0 {
        failure_of_hresult(status as u32)
    } else {
        None
    }
}

/// One captured frame: 8-bit luma, row-major, `width × height` bytes,
/// and the NV12 chroma plane for it, which is exactly the payload of
/// `faraday_shell_api::Event::CameraFrame`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Frame width in pixels.
    pub width: u16,
    /// Frame height in pixels.
    pub height: u16,
    /// The pixels, black 0 to white 255.
    pub luma: Vec<u8>,
    /// `width / 2 × height / 2` pairs of interleaved Cb and Cr, or
    /// `None` when the device gave no usable second plane.
    pub chroma: Option<Vec<u8>>,
}

impl Frame {
    /// Copies an NV12 buffer's two planes into a frame: the `stride` the
    /// device lays each one's rows out at is honoured, and a frame wider
    /// than 1280 pixels is shrunk by an integer factor.
    ///
    /// `chroma` is plane UV and its own bytes per row. `None`, or a plane
    /// shorter than the rows it claims, leaves the preview grey.
    ///
    /// `None` when the width or height is odd (NV12 has neither), when
    /// the luma plane is shorter than the rows it claims, or when the
    /// result would not fit a `u16`.
    #[must_use]
    pub fn from_plane(
        plane: &[u8],
        width: usize,
        height: usize,
        stride: usize,
        chroma: Option<(&[u8], usize)>,
    ) -> Option<Frame> {
        if !width.is_multiple_of(2) || !height.is_multiple_of(2) {
            return None;
        }
        let luma = convert::luma(plane, width, height, stride)?;
        let factor = convert::scale_factor(width);
        let uv = chroma
            .and_then(|(plane, stride)| convert::chroma(plane, width, height, stride))
            .map(|uv| convert::downscale_chroma(uv, width, height, factor));
        let (width, height, luma) = convert::downscale(luma, width, height, factor);
        Some(Frame {
            width: u16::try_from(width).ok()?,
            height: u16::try_from(height).ok()?,
            luma,
            chroma: uv,
        })
    }
}

/// What a running `Camera` sends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message {
    /// One captured frame.
    Frame(Frame),
    /// No frames will come: no camera, the chosen one is gone, Windows
    /// refused this program the camera, or the device failed. Sent at
    /// most once, with the reason when it is clearly one of
    /// [`Failure`]'s.
    Unavailable(Option<Failure>),
}

/// A video capture device, as the scan sheet offers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// The name Windows shows for it.
    pub name: String,
    /// Its symbolic link, which names it to `Camera::start`.
    pub link: String,
}

/// The most frames a second a `Camera` sends on.
///
/// The device runs at thirty; the shell's decoder runs at its own pace
/// and a frame it gets late is worth nothing, so the rest are dropped in
/// the callback rather than copied and queued.
pub const MAX_FPS: u8 = 10;

#[cfg(target_os = "windows")]
pub use windows::{Camera, devices};

#[cfg(target_os = "windows")]
mod windows {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::SyncSender;
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use crate::{Device, Frame, MAX_FPS, Message, failure_of_status};

    /// The C ABI of `src/camera.c`, which declares the same signatures in
    /// its header comment.
    ///
    /// ```c
    /// typedef void (*osk_mf_device_fn)(void *ctx,
    ///                                  const uint16_t *name, uint32_t name_len,
    ///                                  const uint16_t *link, uint32_t link_len);
    /// ```
    type DeviceFn = unsafe extern "C" fn(*mut c_void, *const u16, u32, *const u16, u32);

    /// ```c
    /// typedef int32_t (*osk_mf_frame_fn)(void *ctx, const uint8_t *y,
    ///                                    uint32_t width, uint32_t height,
    ///                                    uint32_t stride, const uint8_t *uv,
    ///                                    uint32_t uv_stride);
    /// ```
    type FrameFn =
        unsafe extern "C" fn(*mut c_void, *const u8, u32, u32, u32, *const u8, u32) -> i32;

    unsafe extern "C" {
        /// ```c
        /// int32_t osk_mf_list(void *ctx, osk_mf_device_fn on_device);
        /// ```
        fn osk_mf_list(ctx: *mut c_void, on_device: DeviceFn) -> i32;
        /// ```c
        /// int32_t osk_mf_capture(void *ctx, const uint16_t *link,
        ///                        osk_mf_frame_fn on_frame);
        /// ```
        fn osk_mf_capture(ctx: *mut c_void, link: *const u16, on_frame: FrameFn) -> i32;
    }

    /// `OSK_MF_OK`: the list is complete, or `on_frame` asked to stop.
    /// Every other status is a [`Message::Unavailable`], with the
    /// reason `failure_of_status` finds in it.
    const OSK_MF_OK: i32 = 0;

    /// The cameras this machine has, in Media Foundation's order (the
    /// first is the one `Camera::start(None, …)` opens). Empty when there
    /// is none, or no Media Foundation on this Windows.
    #[must_use]
    pub fn devices() -> Vec<Device> {
        let mut out: Vec<Device> = Vec::new();
        // SAFETY: `ctx` points at `out`, which lives across the call, and
        // `osk_mf_list` makes every `on_device` call on this thread
        // before it returns. `on_device` has the signature
        // `src/camera.c` declares.
        unsafe { osk_mf_list((&raw mut out).cast::<c_void>(), on_device) };
        out
    }

    /// Called by `osk_mf_list` once for each device.
    ///
    /// # Safety
    ///
    /// `ctx` is the `Vec<Device>` given to `osk_mf_list`, and `name` and
    /// `link` point at `name_len` and `link_len` UTF-16 units, readable
    /// for the duration of the call. `src/camera.c` guarantees all three.
    unsafe extern "C" fn on_device(
        ctx: *mut c_void,
        name: *const u16,
        name_len: u32,
        link: *const u16,
        link_len: u32,
    ) {
        if ctx.is_null() || name.is_null() || link.is_null() {
            return;
        }
        // SAFETY: `ctx` is `devices`' `out`, borrowed by nothing else
        // while `osk_mf_list` runs, and this is called on that thread.
        let out = unsafe { &mut *ctx.cast::<Vec<Device>>() };
        // SAFETY: `src/camera.c` hands over the device's own strings with
        // their lengths in UTF-16 units; they are freed only after this
        // returns, and nothing writes to them meanwhile.
        let (name, link) = unsafe {
            (
                std::slice::from_raw_parts(name, name_len as usize),
                std::slice::from_raw_parts(link, link_len as usize),
            )
        };
        out.push(Device {
            name: String::from_utf16_lossy(name),
            link: String::from_utf16_lossy(link),
        });
    }

    /// What `on_frame` is handed as `ctx`. It lives on the capture
    /// thread's stack for the whole of `osk_mf_capture`, which makes
    /// every callback on that thread, so a `Cell` is enough.
    struct Shared {
        /// Where frames go. A full channel drops the frame.
        tx: SyncSender<Message>,
        /// The earliest time the next frame may be sent: the [`MAX_FPS`]
        /// cap.
        next: Cell<Instant>,
        /// Set by `Camera`'s drop; the next callback answers "stop".
        stop: Arc<AtomicBool>,
    }

    /// The gap [`MAX_FPS`] asks for.
    const INTERVAL: Duration = Duration::from_millis(1000 / MAX_FPS as u64);

    /// Called on the capture thread after every read: with a frame, or
    /// with `y` null when the read gave none. Returns 1 to stop.
    ///
    /// # Safety
    ///
    /// `ctx` is the `Shared` given to `osk_mf_capture`. When `y` is not
    /// null, `width` and `height` are even, `y` points at
    /// `stride * (height - 1) + width` readable bytes, and `uv` is null
    /// or points at `uv_stride * (height / 2 - 1) + width` readable
    /// bytes, all for the duration of the call. `src/camera.c`
    /// guarantees all of it.
    unsafe extern "C" fn on_frame(
        ctx: *mut c_void,
        y: *const u8,
        width: u32,
        height: u32,
        stride: u32,
        uv: *const u8,
        uv_stride: u32,
    ) -> i32 {
        if ctx.is_null() {
            return 1;
        }
        // SAFETY: `ctx` is the `Shared` on `capture`'s stack, which
        // outlives `osk_mf_capture` and so every callback; the callbacks
        // run on that same thread, so nothing else touches it meanwhile.
        let shared = unsafe { &*ctx.cast::<Shared>() };
        let stop = i32::from(shared.stop.load(Ordering::Relaxed));
        if stop != 0 || y.is_null() {
            return stop;
        }
        let (width, height, stride) = (width as usize, height as usize, stride as usize);
        if width == 0
            || height < 2
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
            || stride < width
        {
            return 0;
        }
        let now = Instant::now();
        if now < shared.next.get() {
            return 0;
        }
        // SAFETY: the locked buffer holds plane Y as `height` rows of
        // `stride` bytes, the last only `width` long at the least, so
        // `len` bytes from `y` are readable and no Rust reference aliases
        // them.
        let plane = unsafe { std::slice::from_raw_parts(y, stride * (height - 1) + width) };
        let uv_stride = uv_stride as usize;
        let chroma = (!uv.is_null() && uv_stride >= width).then(|| {
            // SAFETY: plane UV of the same locked buffer holds
            // `height / 2` rows of `uv_stride` bytes, the last `width`
            // long at the least, so `len` bytes from `uv` are readable
            // for this call and nothing aliases them.
            let len = uv_stride * (height / 2 - 1) + width;
            (unsafe { std::slice::from_raw_parts(uv, len) }, uv_stride)
        });
        let Some(frame) = Frame::from_plane(plane, width, height, stride, chroma) else {
            return 0;
        };
        // A full channel means the shell is still on the last frame;
        // this one is stale already.
        if shared.tx.try_send(Message::Frame(frame)).is_ok() {
            let mut next = shared.next.get() + INTERVAL;
            // More than an interval behind (a stalled device, a slow
            // decode): start again from now rather than bursting.
            if next < now {
                next = now;
            }
            shared.next.set(next);
        }
        0
    }

    /// The capture thread's body: opens the camera, reads until told to
    /// stop, and says once if it could not.
    fn capture(link: Option<Vec<u16>>, tx: SyncSender<Message>, stop: Arc<AtomicBool>) {
        let shared = Shared {
            tx,
            next: Cell::new(Instant::now()),
            stop,
        };
        let link = link.as_ref().map_or(std::ptr::null(), |l| l.as_ptr());
        // SAFETY: `ctx` points at `shared` and `link` is null or a
        // NUL-terminated UTF-16 string, both live across the call;
        // `osk_mf_capture` makes every callback on this thread before it
        // returns and keeps neither pointer. `on_frame` has the signature
        // `src/camera.c` declares.
        let status = unsafe {
            osk_mf_capture(
                (&raw const shared).cast_mut().cast::<c_void>(),
                link,
                on_frame,
            )
        };
        if status != OSK_MF_OK && !shared.stop.load(Ordering::Relaxed) {
            let _ = shared
                .tx
                .try_send(Message::Unavailable(failure_of_status(status)));
        }
    }

    /// A running capture. Dropping it stops the camera.
    pub struct Camera {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Camera {
        /// Starts the camera whose symbolic link is `link` ([`devices`]),
        /// or the first when that is `None`, on a thread of its own.
        ///
        /// Returns at once: the frames, and any failure, arrive on `tx`,
        /// a failure as one [`Message::Unavailable`]. A channel of two
        /// is right — one frame being decoded and one waiting.
        #[must_use]
        pub fn start(link: Option<&str>, tx: SyncSender<Message>) -> Camera {
            let link: Option<Vec<u16>> =
                link.map(|l| l.encode_utf16().chain(std::iter::once(0)).collect());
            let stop = Arc::new(AtomicBool::new(false));
            let flag = Arc::clone(&stop);
            let failed = tx.clone();
            let thread = std::thread::Builder::new()
                .name(String::from("camera"))
                .spawn(move || capture(link, tx, flag))
                .ok();
            if thread.is_none() {
                let _ = failed.try_send(Message::Unavailable(None));
            }
            Camera { stop, thread }
        }
    }

    impl Drop for Camera {
        /// Waits for the capture thread, which is what releases the
        /// device: it ends at the next read, a frame's time at most.
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The statuses and HRESULTs a person meets: the privacy setting, a
    /// camera another program holds, no camera. Anything else has no
    /// reason.
    #[test]
    fn statuses_give_the_reason() {
        assert_eq!(failure_of_status(1), Some(Failure::NotFound));
        assert_eq!(
            failure_of_status(0x8007_0005_u32 as i32),
            Some(Failure::Refused)
        );
        assert_eq!(
            failure_of_status(0xC00D_3704_u32 as i32),
            Some(Failure::InUse)
        );
        assert_eq!(
            failure_of_status(0x8007_0020_u32 as i32),
            Some(Failure::InUse)
        );
        assert_eq!(
            failure_of_status(0x8007_048F_u32 as i32),
            Some(Failure::NotFound)
        );
        // MF_E_INVALIDMEDIATYPE, and a failure with no HRESULT.
        assert_eq!(failure_of_status(0xC00D_36B4_u32 as i32), None);
        assert_eq!(failure_of_status(3), None);
        assert_eq!(failure_of_status(0), None);
    }

    /// The stride the device lays its rows out at is honoured, not
    /// assumed away: a 4 × 4 frame with two bytes of row padding.
    #[test]
    fn padded_stride_is_dropped() {
        let plane: Vec<u8> = (0..24).collect();
        // Plane UV is 2 x 2 pairs at a stride of 6: two bytes of padding
        // a row there too.
        let uv: Vec<u8> = vec![10, 20, 11, 21, 0, 0, 12, 22, 13, 23, 0, 0];
        let frame =
            Frame::from_plane(&plane, 4, 4, 6, Some((&uv, 6))).expect("the plane is long enough");
        assert_eq!((frame.width, frame.height), (4, 4));
        assert_eq!(
            frame.luma,
            vec![0, 1, 2, 3, 6, 7, 8, 9, 12, 13, 14, 15, 18, 19, 20, 21]
        );
        assert_eq!(
            frame.chroma,
            Some(vec![10, 20, 11, 21, 12, 22, 13, 23]),
            "the pairs, without the row padding"
        );
        // A plane one byte short of its last row is refused rather than
        // guessed at; a short chroma plane leaves the preview grey
        // rather than refusing the frame.
        assert_eq!(Frame::from_plane(&plane[..21], 4, 4, 6, None), None);
        let frame = Frame::from_plane(&plane, 4, 4, 6, Some((&uv[..6], 6))).expect("long enough");
        assert_eq!(frame.chroma, None);
    }

    /// A buffer without padding, as most cameras give at 640 × 480,
    /// comes through as it is.
    #[test]
    fn unpadded_planes_pass_through() {
        let (width, height) = (640usize, 480usize);
        let plane: Vec<u8> = (0..width * height).map(|i| (i % 251) as u8).collect();
        let uv: Vec<u8> = (0..width * height / 2).map(|i| (i % 241) as u8).collect();
        let frame = Frame::from_plane(&plane, width, height, width, Some((&uv, width)))
            .expect("long enough");
        assert_eq!((frame.width, frame.height), (640, 480));
        assert_eq!(frame.luma, plane);
        assert_eq!(frame.chroma, Some(uv));
    }

    /// NV12 has no odd sizes; a frame claiming one is refused rather
    /// than half-read.
    #[test]
    fn odd_sizes_are_refused() {
        let plane = vec![0u8; 64];
        assert_eq!(Frame::from_plane(&plane, 5, 4, 6, None), None);
        assert_eq!(Frame::from_plane(&plane, 4, 5, 6, None), None);
        assert!(Frame::from_plane(&plane, 6, 4, 6, None).is_some());
    }

    /// A 2560 × 1440 plane comes back halved: 2560 needs a factor of two
    /// to reach 1280.
    #[test]
    fn wide_planes_are_halved() {
        let (width, height, stride) = (2560usize, 1440usize, 2688usize);
        let mut plane = vec![0u8; stride * height];
        for y in 0..height {
            let value = (y % 256) as u8;
            plane[y * stride..y * stride + width].fill(value);
        }
        let (cw, ch) = (1280usize, 720usize);
        let mut uv = vec![0u8; stride * ch];
        for y in 0..ch {
            for x in 0..cw {
                let at = y * stride + x * 2;
                uv[at] = (y % 256) as u8;
                uv[at + 1] = (x % 256) as u8;
            }
        }
        let frame =
            Frame::from_plane(&plane, width, height, stride, Some((&uv, stride))).expect("long");
        assert_eq!((frame.width, frame.height), (1280, 720));
        // Output row 0 averages rows 0 and 1, row 3 averages 6 and 7.
        assert_eq!(frame.luma[0], 0);
        assert_eq!(frame.luma[3 * 1280], 6);
        // The chroma is halved with it: each output pair averages a 2 x 2
        // block of pairs.
        let chroma = frame.chroma.expect("the plane was given");
        assert_eq!(chroma.len(), 640 * 360 * 2);
        assert_eq!((chroma[0], chroma[1]), (0, 0));
        assert_eq!((chroma[2], chroma[3]), (0, 2));
    }
}
