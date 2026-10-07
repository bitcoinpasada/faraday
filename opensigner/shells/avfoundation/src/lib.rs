//! AVFoundation capture for the macOS shell: start the camera, and hand
//! back 8-bit luma frames with the colour beside them.
//!
//! # Why this crate may use `unsafe`
//!
//! The workspace sets `unsafe_code = "forbid"`. This crate and
//! `opensigner-v4l2` are the two exceptions, and their `Cargo.toml`s say
//! so rather than inheriting the workspace lints. The reason is
//! `docs/PLANNING.md` §16.33:
//!
//! > **macOS uses AVFoundation from one small Objective-C file** (about
//! > a hundred lines: a capture session, the default video device, a
//! > bi-planar 4:2:0 output, a delegate that hands the Y plane to a C
//! > callback), compiled by the `cc` crate the workspace already builds
//! > libsecp256k1 with, behind a three-function C ABI. No Objective-C
//! > binding crates. The camera permission needs an `.app` bundle with
//! > an `NSCameraUsageDescription`, ad-hoc signed; a `just mac-app`
//! > recipe makes it. Built and tested on the owner's Mac only.
//!
//! The alternative was a camera library, and the owner's rule is that
//! one is acceptable only if it is minimal, vendored and small enough to
//! review in full. No Objective-C binding crate is; `src/camera.m` is,
//! at about two hundred lines including its comments. Calling it needs
//! three `extern "C"` declarations and two callbacks, which have no safe
//! form in `std`. Every `unsafe` block below carries a `// SAFETY:`
//! comment naming the invariant it relies on, and the public API —
//! `Camera`, [`Frame`], [`Message`] — is entirely safe: no raw pointer
//! and no pixel buffer crosses it.
//!
//! # Platforms
//!
//! Everything that touches AVFoundation is behind
//! `#[cfg(target_os = "macos")]`, including the `cc` build dependency.
//! On Linux and Windows this crate is [`Frame`], [`Message`] and their
//! tests, and `cargo build` runs no compiler but `rustc`.
//!
//! # Shape
//!
//! ```no_run
//! # #[cfg(target_os = "macos")] {
//! use std::sync::mpsc::sync_channel;
//! use opensigner_avfoundation::{Camera, Message};
//!
//! let (tx, rx) = sync_channel(2);
//! let camera = Camera::start(tx);
//! // ... rx.try_recv() for frames, until:
//! drop(camera);
//! # }
//! ```
//!
//! The camera permission is answered by the user, so `Camera::start`
//! cannot fail synchronously: the first message is
//! [`Message::Unavailable`] if there is no camera or no permission, and
//! a [`Message::Frame`] otherwise.

mod convert;

/// One captured frame: 8-bit luma, row-major, `width × height` bytes,
/// and the NV12 chroma plane for it, which is exactly the payload of
/// `osk_shell_api::Event::CameraFrame`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Frame width in pixels.
    pub width: u16,
    /// Frame height in pixels.
    pub height: u16,
    /// The pixels, black 0 to white 255.
    pub luma: Vec<u8>,
    /// `⌈width / 2⌉ × ⌈height / 2⌉` pairs of interleaved Cb and Cr, or
    /// `None` when the device gave no second plane.
    pub chroma: Option<Vec<u8>>,
}

impl Frame {
    /// Copies a buffer's two planes into a frame: the `stride` the
    /// device lays each one's rows out at is honoured, and a frame wider
    /// than 800 pixels is shrunk by an integer factor.
    ///
    /// `chroma` is plane 1 and its own bytes per row; the bi-planar
    /// video-range format the session asks for lays it out as the
    /// interleaved pairs the core takes. `None`, or a plane shorter than
    /// the rows it claims, leaves the preview grey.
    ///
    /// `None` when the luma plane is shorter than the rows it claims, or
    /// when the result would not fit a `u16`.
    #[must_use]
    pub fn from_plane(
        plane: &[u8],
        width: usize,
        height: usize,
        stride: usize,
        chroma: Option<(&[u8], usize)>,
    ) -> Option<Frame> {
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
    /// No frames will come: no camera, or the user refused permission.
    /// Sent once.
    Unavailable,
}

/// The most frames a second a `Camera` sends on.
///
/// The device runs at thirty; the shell's decoder runs at its own pace
/// and a frame it gets late is worth nothing, so the rest are dropped in the
/// callback rather than copied and queued.
pub const MAX_FPS: u8 = 10;

#[cfg(target_os = "macos")]
pub use macos::Camera;

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::SyncSender;
    use std::time::{Duration, Instant};

    use crate::{Frame, MAX_FPS, Message};

    /// The C ABI of `src/camera.m`, which declares the same two
    /// signatures in its header comment.
    ///
    /// ```c
    /// typedef void (*osk_avf_frame_fn)(void *ctx, const uint8_t *y,
    ///                                  uint32_t width, uint32_t height,
    ///                                  uint32_t stride, const uint8_t *uv,
    ///                                  uint32_t uv_stride);
    /// ```
    type FrameFn = unsafe extern "C" fn(*mut c_void, *const u8, u32, u32, u32, *const u8, u32);

    /// ```c
    /// typedef void (*osk_avf_state_fn)(void *ctx, int32_t ok);
    /// ```
    type StateFn = unsafe extern "C" fn(*mut c_void, i32);

    unsafe extern "C" {
        /// ```c
        /// void *osk_avf_start(void *ctx, osk_avf_frame_fn on_frame,
        ///                     osk_avf_state_fn on_state);
        /// ```
        fn osk_avf_start(ctx: *mut c_void, on_frame: FrameFn, on_state: StateFn) -> *mut c_void;
        /// ```c
        /// void osk_avf_stop(void *handle);
        /// ```
        fn osk_avf_stop(handle: *mut c_void);
    }

    /// What the callbacks are handed as `ctx`. It lives in a `Box` the
    /// [`Camera`] owns and frees after `osk_avf_stop` has returned,
    /// which is the point after which no callback can run.
    struct Shared {
        /// Where frames go. A full channel drops the frame.
        tx: SyncSender<Message>,
        /// The earliest time the next frame may be sent: the [`MAX_FPS`]
        /// cap. The callbacks run on AVFoundation's queues, so it is
        /// behind a lock rather than a plain field.
        next: Mutex<Instant>,
        /// Set before `osk_avf_stop`, so that a callback already inside
        /// the Objective-C lock does nothing.
        stopped: AtomicBool,
    }

    /// The callbacks run on AVFoundation's queues, so `Shared` must be
    /// shareable between threads. The `unsafe` deref in the callbacks
    /// asserts that rather than proving it; this checks it.
    const _: () = {
        const fn assert_sync<T: Sync>() {}
        assert_sync::<Shared>();
    };

    /// The gap [`MAX_FPS`] asks for.
    const INTERVAL: Duration = Duration::from_millis(1000 / MAX_FPS as u64);

    /// Called on the capture queue for every frame the device delivers.
    ///
    /// # Safety
    ///
    /// `ctx` is the pointer given to `osk_avf_start`, and `y` points at
    /// `stride * (height - 1) + width` readable bytes for the duration
    /// of the call. `uv` is null, or points at
    /// `uv_stride * (⌈height / 2⌉ - 1) + ⌈width / 2⌉ * 2` readable bytes
    /// for the same duration. `src/camera.m` guarantees all three.
    unsafe extern "C" fn on_frame(
        ctx: *mut c_void,
        y: *const u8,
        width: u32,
        height: u32,
        stride: u32,
        uv: *const u8,
        uv_stride: u32,
    ) {
        if ctx.is_null() || y.is_null() {
            return;
        }
        // SAFETY: `ctx` is the `Shared` this crate leaked in
        // `Camera::start`. It is freed only after `osk_avf_stop` has
        // returned, and `src/camera.m` runs no callback after that, so
        // the reference is live for this call. `Shared` is `Sync` by its
        // fields, which is what lets an AVFoundation queue hold it.
        let shared = unsafe { &*ctx.cast::<Shared>() };
        if shared.stopped.load(Ordering::Relaxed) {
            return;
        }
        let (width, height, stride) = (width as usize, height as usize, stride as usize);
        if width == 0 || height == 0 || stride < width {
            return;
        }
        // The lock is uncontended in practice: frames arrive on one
        // serial queue. It is held across the copy so that a stop cannot
        // begin freeing while a frame is being read.
        let mut next = shared.next.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        if now < *next {
            return;
        }
        let len = stride * (height - 1) + width;
        // SAFETY: the pixel buffer is locked read-only around this call
        // and holds `height` rows of `stride` bytes, so `len` bytes from
        // `y` are readable and no Rust reference aliases them.
        let plane = unsafe { std::slice::from_raw_parts(y, len) };
        let uv_stride = uv_stride as usize;
        let (cw, ch) = crate::convert::chroma_size(width, height);
        let chroma = (!uv.is_null() && uv_stride >= cw * 2).then(|| {
            let len = uv_stride * (ch - 1) + cw * 2;
            // SAFETY: plane 1 of the same locked buffer holds `ch` rows
            // of `uv_stride` bytes, so `len` bytes from `uv` are
            // readable for this call and nothing aliases them.
            (unsafe { std::slice::from_raw_parts(uv, len) }, uv_stride)
        });
        let Some(frame) = Frame::from_plane(plane, width, height, stride, chroma) else {
            return;
        };
        // A full channel means the shell is still on the last frame;
        // this one is stale already.
        if shared.tx.try_send(Message::Frame(frame)).is_ok() {
            *next += INTERVAL;
            // More than an interval behind (a stalled device, a slow
            // decode): start again from now rather than bursting.
            if *next < now {
                *next = now;
            }
        }
    }

    /// Called once when the session either started or cannot start.
    ///
    /// # Safety
    ///
    /// `ctx` is the pointer given to `osk_avf_start`.
    unsafe extern "C" fn on_state(ctx: *mut c_void, ok: i32) {
        if ctx.is_null() {
            return;
        }
        // SAFETY: as in `on_frame` — the box outlives every callback.
        let shared = unsafe { &*ctx.cast::<Shared>() };
        if ok == 0 && !shared.stopped.load(Ordering::Relaxed) {
            let _ = shared.tx.try_send(Message::Unavailable);
        }
    }

    /// A running capture session. Dropping it stops the camera.
    ///
    /// Not `Send`: the handle belongs to the thread that started it,
    /// which for this shell is the event loop's.
    pub struct Camera {
        /// The Objective-C object, owned.
        handle: *mut c_void,
        /// The `Shared` the callbacks are handed, owned.
        ctx: *mut Shared,
    }

    impl Camera {
        /// Starts the camera, asking for permission the first time.
        ///
        /// Returns as soon as the request is made: the answer, and any
        /// failure, arrives on `tx` as [`Message::Unavailable`]. A
        /// channel of two is right — one frame being decoded and one
        /// waiting.
        #[must_use]
        pub fn start(tx: SyncSender<Message>) -> Camera {
            let ctx = Box::into_raw(Box::new(Shared {
                tx,
                next: Mutex::new(Instant::now()),
                stopped: AtomicBool::new(false),
            }));
            // SAFETY: `ctx` points at a live `Shared` that outlives the
            // session, since `Drop` stops the session before freeing it.
            // The two callbacks are `extern "C"` with the signatures
            // `src/camera.m` declares.
            let handle = unsafe { osk_avf_start(ctx.cast::<c_void>(), on_frame, on_state) };
            Camera { handle, ctx }
        }
    }

    impl Drop for Camera {
        fn drop(&mut self) {
            // SAFETY: `self.ctx` is the box from `start`, not yet freed.
            unsafe { (*self.ctx).stopped.store(true, Ordering::Relaxed) };
            // SAFETY: `self.handle` came from `osk_avf_start` and is
            // stopped exactly once, here. The call returns only when no
            // callback is running and none can start.
            unsafe { osk_avf_stop(self.handle) };
            // SAFETY: no callback can dereference `ctx` after the stop
            // above, so this is the last owner.
            drop(unsafe { Box::from_raw(self.ctx) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The stride the device lays its rows out at is honoured, not
    /// assumed away: a 4 × 3 plane with two bytes of row padding.
    #[test]
    fn padded_stride_is_dropped() {
        let plane: Vec<u8> = (0..18).collect();
        // Plane 1 is 2 x 2 pairs at a stride of 6: two bytes of padding
        // a row there too.
        let uv: Vec<u8> = vec![10, 20, 11, 21, 0, 0, 12, 22, 13, 23, 0, 0];
        let frame =
            Frame::from_plane(&plane, 4, 3, 6, Some((&uv, 6))).expect("the plane is long enough");
        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 3);
        assert_eq!(frame.luma, vec![0, 1, 2, 3, 6, 7, 8, 9, 12, 13, 14, 15]);
        assert_eq!(
            frame.chroma,
            Some(vec![10, 20, 11, 21, 12, 22, 13, 23]),
            "the pairs, without the row padding"
        );
        // A plane one byte short of its last row is refused rather than
        // guessed at; a short chroma plane leaves the preview grey
        // rather than refusing the frame.
        assert_eq!(Frame::from_plane(&plane[..15], 4, 3, 6, None), None);
        let frame = Frame::from_plane(&plane, 4, 3, 6, Some((&uv[..6], 6))).expect("long enough");
        assert_eq!(frame.chroma, None);
    }

    /// A 1280 × 720 plane, which is what a Mac's built-in camera offers,
    /// comes back halved: 1280 needs a factor of two to reach 800.
    #[test]
    fn wide_planes_are_halved() {
        // Rows of a constant value, so the block average of any 2 × 2 is
        // the average of two neighbouring row values.
        let (width, height, stride) = (1280usize, 720usize, 1408usize);
        let mut plane = vec![0u8; stride * height];
        for y in 0..height {
            let value = (y % 256) as u8;
            plane[y * stride..y * stride + width].fill(value);
        }
        let frame = Frame::from_plane(&plane, width, height, stride, None).expect("long enough");
        assert_eq!(frame.width, 640);
        assert_eq!(frame.height, 360);
        assert_eq!(frame.luma.len(), 640 * 360);
        // Output row 0 averages rows 0 and 1, row 3 averages 6 and 7.
        assert_eq!(frame.luma[0], 0);
        assert_eq!(frame.luma[3 * 640], 6);
        // The chroma is halved with it: 640 x 360 pairs become
        // 320 x 180, each the average of a 2 x 2 block of pairs.
        let (cw, ch, uv_stride) = (640usize, 360usize, 1408usize);
        let mut uv = vec![0u8; uv_stride * ch];
        for y in 0..ch {
            for x in 0..cw {
                let at = y * uv_stride + x * 2;
                uv[at] = (y % 256) as u8;
                uv[at + 1] = (x % 256) as u8;
            }
        }
        let frame =
            Frame::from_plane(&plane, width, height, stride, Some((&uv, uv_stride))).expect("long");
        let chroma = frame.chroma.expect("the plane was given");
        assert_eq!(chroma.len(), 320 * 180 * 2);
        // Output pair 0 averages rows 0 and 1 and columns 0 and 1.
        assert_eq!((chroma[0], chroma[1]), (0, 0));
        assert_eq!((chroma[2], chroma[3]), (0, 2));
        // A 640 × 480 frame is untouched.
        let small = vec![7u8; 640 * 480];
        let frame = Frame::from_plane(&small, 640, 480, 640, None).expect("long enough");
        assert_eq!((frame.width, frame.height), (640, 480));
    }
}
