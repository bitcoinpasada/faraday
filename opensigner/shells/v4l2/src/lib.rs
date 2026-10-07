//! Video4Linux2 capture for the Linux shells: find a camera, stream it,
//! and hand back 8-bit luma frames, with the colour beside them where
//! the device has colour to give.
//!
//! The order the formats are asked for is colour first — `YUYV`, then
//! `NV12`, `YU12`, and `GREY` last — because the core draws the
//! viewfinder's preview in colour when a frame carries chroma, and
//! decodes from the luma either way (§16.33 wrote the order the other
//! way round, when the preview was grey).
//!
//! # Why this crate may use `unsafe`
//!
//! The workspace sets `unsafe_code = "forbid"`. This crate is the one
//! exception, and its `Cargo.toml` says so rather than inheriting the
//! workspace lints. The reason is `docs/PLANNING.md` §16.33:
//!
//! > **Linux desktop and the Pi share one V4L2 module.** Raw `ioctl`s
//! > through `libc` (already in the dependency graph): query
//! > capabilities, set a 640 × 480 format, four memory-mapped buffers,
//! > stream, `poll`, dequeue, hand the luma to the core, requeue.
//! > Formats asked for in order: `GREY`, then `YUYV` (every second
//! > byte), then `NV12`/`YU12` (the first width × height bytes). The
//! > V4L2 structs and request numbers are declared in the module, about
//! > two hundred lines; no `v4l` crate. The first `/dev/video*` that
//! > streams a video format is the camera; no device chooser yet.
//!
//! The alternative was a camera library, and the owner's rule is that
//! one is acceptable only if it is minimal, vendored and small enough to
//! review in full. No `v4l` crate is; this is. `ioctl`, `mmap` and
//! `poll` have no safe equivalent in `std`, so the choice was a
//! dependency we cannot read or a page of `unsafe` we can. Every
//! `unsafe` block below carries a `// SAFETY:` comment naming the
//! invariant it relies on, and the public API — [`find`], [`Camera`],
//! [`Frame`], [`spawn`] — is entirely safe: no raw pointer, no file
//! descriptor and no mapped buffer crosses it.
//!
//! # Shape
//!
//! ```no_run
//! # use std::sync::atomic::AtomicBool;
//! # use std::sync::mpsc::sync_channel;
//! # use std::sync::Arc;
//! # use opensigner_v4l2::{Camera, Size};
//! let path = opensigner_v4l2::find().expect("no camera");
//! let camera = Camera::open(&path, Size::VGA)?;
//! let (tx, rx) = sync_channel(2);
//! let stop = Arc::new(AtomicBool::new(false));
//! let thread = opensigner_v4l2::spawn(camera, tx, Arc::clone(&stop));
//! // ... rx.try_recv() for frames, then:
//! stop.store(true, std::sync::atomic::Ordering::Relaxed);
//! let _ = thread.join();
//! # Ok::<(), std::io::Error>(())
//! ```

mod abi;
mod convert;

use std::ffi::{c_int, c_void};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How many `/dev/videoN` nodes [`find`] looks at.
const MAX_NODES: u8 = 32;

/// Buffers in the driver's queue. Four is the usual minimum that lets a
/// driver keep filling while one is dequeued.
const BUFFERS: u32 = 4;

/// The longest a frame is allowed to be, as a guard against a driver
/// that reports an absurd `sizeimage`: 4096 × 4096 of luma.
const MAX_FRAME_BYTES: usize = 4096 * 4096;

/// The frame size to ask a driver for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
}

impl Size {
    /// 640 × 480: what every shell asks for (§16.33). A driver may give
    /// something else, and the shell takes what it gets.
    pub const VGA: Size = Size {
        width: 640,
        height: 480,
    };
}

/// One captured frame: 8-bit luma, row-major, `width × height` bytes,
/// and the NV12 chroma plane for it where the device gave colour, which
/// is exactly the payload of `osk_shell_api::Event::CameraFrame`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// Frame width in pixels.
    pub width: u16,
    /// Frame height in pixels.
    pub height: u16,
    /// The pixels, black 0 to white 255.
    pub luma: Vec<u8>,
    /// `⌈width / 2⌉ × ⌈height / 2⌉` pairs of interleaved U and V, or
    /// `None` from a `GREY` device.
    pub chroma: Option<Vec<u8>>,
}

/// The first `/dev/videoN` that is a video capture node, can stream, and
/// accepts one of the four pixel formats we read.
///
/// The Pi exposes ISP, codec and metadata nodes alongside the camera;
/// they fail one of those three tests, which is how they are skipped. No
/// device is chosen by name and there is no chooser: the first one that
/// passes is the camera. Nothing is set on a node that is only probed —
/// the format test is `VIDIOC_TRY_FMT`.
pub fn find() -> Option<PathBuf> {
    (0..MAX_NODES).find_map(|n| {
        let path = PathBuf::from(format!("/dev/video{n}"));
        can_capture(&path).then_some(path)
    })
}

/// Whether one node is a streaming video capture device that takes a
/// format we can convert.
fn can_capture(path: &Path) -> bool {
    let Ok(file) = open_rw(path) else {
        return false;
    };
    let fd = file.as_raw_fd();
    let Ok(cap) = query_cap(fd) else {
        return false;
    };
    let caps = cap.node_caps();
    if caps & abi::CAP_VIDEO_CAPTURE == 0 || caps & abi::CAP_STREAMING == 0 {
        return false;
    }
    abi::FORMATS.iter().any(|&want| {
        try_format(fd, abi::VIDIOC_TRY_FMT, want, Size::VGA)
            .is_ok_and(|f| convert::luma_step(f.pix.pixelformat).is_some())
    })
}

/// A streaming camera: the device, its negotiated format, and the four
/// mapped buffers.
///
/// Dropping it stops the stream, unmaps every buffer and closes the
/// device, in that order.
pub struct Camera {
    file: File,
    /// The negotiated FourCC, one of the four we accept.
    format: u32,
    /// Bytes between neighbouring luma samples.
    step: usize,
    width: usize,
    height: usize,
    stride: usize,
    /// How much a frame is shrunk before it leaves this crate.
    factor: usize,
    /// The most frames a second [`spawn`] sends on, [`MAX_FPS`] until a
    /// caller asks for another rate.
    max_fps: u8,
    buffers: Vec<Mapped>,
    streaming: bool,
}

/// One mmap'd capture buffer.
struct Mapped {
    ptr: *mut c_void,
    len: usize,
}

impl Camera {
    /// Opens `path`, negotiates a format at `want`, maps four buffers
    /// and starts the stream.
    ///
    /// The formats are asked for colour first - `YUYV`, `NV12`, `YU12`,
    /// then `GREY` - and the first the driver grants is used. A driver
    /// is free to adjust the size, and whatever it returns is what the
    /// frames will be; a mode much wider than 800 pixels is shrunk by an
    /// integer factor before the frame leaves this crate.
    pub fn open(path: &Path, want: Size) -> io::Result<Camera> {
        let file = open_rw(path)?;
        let fd = file.as_raw_fd();

        let cap = query_cap(fd)?;
        let caps = cap.node_caps();
        if caps & abi::CAP_VIDEO_CAPTURE == 0 || caps & abi::CAP_STREAMING == 0 {
            return Err(io::Error::other(format!(
                "{} is not a streaming video capture device",
                path.display()
            )));
        }

        let pix = negotiate(fd, want)?;
        let step = convert::luma_step(pix.pixelformat).ok_or_else(|| {
            io::Error::other(format!(
                "{} offers no pixel format this shell can read",
                path.display()
            ))
        })?;
        let width = pix.width as usize;
        let height = pix.height as usize;
        let packed = convert::packed_stride(pix.pixelformat, width).unwrap_or(width * step);
        let stride = if pix.bytesperline as usize >= packed {
            pix.bytesperline as usize
        } else {
            packed
        };
        if width == 0 || height == 0 || stride.saturating_mul(height) > MAX_FRAME_BYTES {
            return Err(io::Error::other(format!(
                "{} reports an unusable frame size {width}x{height}",
                path.display()
            )));
        }

        let mut camera = Camera {
            file,
            format: pix.pixelformat,
            step,
            width,
            height,
            stride,
            factor: convert::scale_factor(width),
            max_fps: MAX_FPS,
            buffers: Vec::new(),
            streaming: false,
        };
        camera.map_buffers()?;
        camera.stream_on()?;
        Ok(camera)
    }

    /// The negotiated frame size, before any downscale.
    pub fn size(&self) -> Size {
        Size {
            width: self.width.min(u16::MAX as usize) as u16,
            height: self.height.min(u16::MAX as usize) as u16,
        }
    }

    /// Asks [`spawn`] for at most `fps` frames a second instead of
    /// [`MAX_FPS`], for a shell whose loop can afford them. Zero means
    /// one: a camera that sends nothing is a camera that is off.
    pub fn set_max_fps(&mut self, fps: u8) {
        self.max_fps = fps.max(1);
    }

    /// The most frames a second [`spawn`] will send on for this camera.
    pub fn max_fps(&self) -> u8 {
        self.max_fps
    }

    /// The negotiated FourCC as its four characters (`GREY`, `YUYV`).
    pub fn format(&self) -> String {
        String::from_utf8_lossy(&self.format.to_le_bytes()).into_owned()
    }

    /// Waits up to `timeout` for a frame, converts it to luma and
    /// requeues the buffer. `None` when the timeout passed with nothing
    /// to dequeue.
    pub fn frame(&mut self, timeout: Duration) -> io::Result<Option<Frame>> {
        if !self.wait(timeout)? {
            return Ok(None);
        }
        let mut buf = abi::Buffer {
            type_: abi::BUF_TYPE_VIDEO_CAPTURE,
            memory: abi::MEMORY_MMAP,
            ..abi::Buffer::default()
        };
        match ioctl(self.file.as_raw_fd(), abi::VIDIOC_DQBUF, &raw mut buf) {
            Ok(()) => {}
            // Nothing ready after all: `poll` can wake for an event.
            Err(e) if e.raw_os_error() == Some(libc::EAGAIN) => return Ok(None),
            Err(e) => return Err(e),
        }

        let index = buf.index as usize;
        let frame = self
            .buffers
            .get(index)
            .and_then(|mapped| {
                let used = (buf.bytesused as usize).min(mapped.len);
                self.convert(mapped.as_slice(used))
            })
            .ok_or_else(|| io::Error::other("the driver returned a frame this shell cannot read"));

        // The buffer goes back whether or not the conversion worked; a
        // queue that leaks buffers stops after four frames.
        let requeue = ioctl(self.file.as_raw_fd(), abi::VIDIOC_QBUF, &raw mut buf);
        let frame = frame?;
        requeue?;
        Ok(Some(frame))
    }

    /// Luma and, where the format has it, chroma out of one dequeued
    /// buffer, both shrunk to at most 800 pixels across.
    fn convert(&self, bytes: &[u8]) -> Option<Frame> {
        let luma = convert::luma(bytes, self.width, self.height, self.stride, self.step)?;
        let chroma = convert::chroma(self.format, bytes, self.width, self.height, self.stride)
            .map(|uv| convert::downscale_chroma(uv, self.width, self.height, self.factor));
        let (w, h, luma) = convert::downscale(luma, self.width, self.height, self.factor);
        Some(Frame {
            width: u16::try_from(w).ok()?,
            height: u16::try_from(h).ok()?,
            luma,
            chroma,
        })
    }

    /// `poll` for readability; `true` when a buffer is ready.
    fn wait(&self, timeout: Duration) -> io::Result<bool> {
        let ms = c_int::try_from(timeout.as_millis()).unwrap_or(c_int::MAX);
        let mut fds = libc::pollfd {
            fd: self.file.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: `fds` is one initialised `pollfd` owned by this stack
        // frame and the count passed is 1, so `poll` writes only inside
        // it. The fd is owned by `self.file` and outlives the call.
        let n = unsafe { libc::poll(&raw mut fds, 1, ms) };
        if n < 0 {
            let e = io::Error::last_os_error();
            // A signal is not a failure; the caller polls again.
            if e.raw_os_error() == Some(libc::EINTR) {
                return Ok(false);
            }
            return Err(e);
        }
        Ok(n > 0 && fds.revents & libc::POLLIN != 0)
    }

    /// `REQBUFS`, then `QUERYBUF` + `mmap` + `QBUF` for each buffer.
    fn map_buffers(&mut self) -> io::Result<()> {
        let fd = self.file.as_raw_fd();
        let mut req = abi::RequestBuffers {
            count: BUFFERS,
            type_: abi::BUF_TYPE_VIDEO_CAPTURE,
            memory: abi::MEMORY_MMAP,
            ..abi::RequestBuffers::default()
        };
        ioctl(fd, abi::VIDIOC_REQBUFS, &raw mut req)?;
        if req.count < 2 {
            return Err(io::Error::other(format!(
                "the driver granted {} capture buffers, which is too few",
                req.count
            )));
        }
        for index in 0..req.count {
            let mut buf = abi::Buffer {
                index,
                type_: abi::BUF_TYPE_VIDEO_CAPTURE,
                memory: abi::MEMORY_MMAP,
                ..abi::Buffer::default()
            };
            ioctl(fd, abi::VIDIOC_QUERYBUF, &raw mut buf)?;
            let len = buf.length as usize;
            if len == 0 || len > MAX_FRAME_BYTES {
                return Err(io::Error::other(format!(
                    "the driver reports a {len}-byte capture buffer"
                )));
            }
            // SAFETY: a null hint lets the kernel choose the address;
            // `len` and the offset are the ones the driver just reported
            // for this buffer, and `fd` is the open device. The mapping
            // is owned by `self.buffers` from here and unmapped exactly
            // once, in `Drop`.
            let ptr = unsafe {
                libc::mmap(
                    ptr::null_mut(),
                    len,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    fd,
                    buf.m as libc::off_t,
                )
            };
            if ptr == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            self.buffers.push(Mapped { ptr, len });
            ioctl(fd, abi::VIDIOC_QBUF, &raw mut buf)?;
        }
        Ok(())
    }

    fn stream_on(&mut self) -> io::Result<()> {
        let mut type_ = abi::BUF_TYPE_VIDEO_CAPTURE as c_int;
        ioctl(self.file.as_raw_fd(), abi::VIDIOC_STREAMON, &raw mut type_)?;
        self.streaming = true;
        Ok(())
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        if self.streaming {
            let mut type_ = abi::BUF_TYPE_VIDEO_CAPTURE as c_int;
            let _ = ioctl(self.file.as_raw_fd(), abi::VIDIOC_STREAMOFF, &raw mut type_);
        }
        for b in self.buffers.drain(..) {
            // SAFETY: `b.ptr` and `b.len` are the address and length one
            // `mmap` in `map_buffers` returned, this is the only place
            // they are unmapped, and `drain` means no other copy of them
            // survives the loop.
            unsafe {
                libc::munmap(b.ptr, b.len);
            }
        }
        // `self.file` closes the device as it drops, after the unmaps.
    }
}

impl Mapped {
    /// The first `len` bytes of the mapping.
    fn as_slice(&self, len: usize) -> &[u8] {
        // SAFETY: `self.ptr` is a live `MAP_SHARED` mapping of
        // `self.len` bytes that outlives the returned slice (it is
        // unmapped only in `Camera::drop`), `len` is clamped to
        // `self.len` by the caller, and the driver has handed the
        // buffer back to us with `DQBUF`, so nothing else writes it
        // while the slice lives.
        unsafe { std::slice::from_raw_parts(self.ptr.cast::<u8>(), len.min(self.len)) }
    }
}

// The mapping is only ever read through `&Camera`, and the fd it belongs
// to moves with it, so a `Camera` can be sent to the capture thread.
// SAFETY: `Mapped` is a plain address and length; nothing in this crate
// shares one between threads, and the mapping is created and destroyed
// on whichever single thread owns the `Camera`.
unsafe impl Send for Camera {}

/// Runs `camera` on a thread, sending frames to `tx` until `stop` is set.
///
/// The channel is expected to be bounded: a full channel drops the frame
/// rather than blocking, because the shell's decoder runs at its own pace
/// and a frame it gets late is worth nothing. Sends are capped for the
/// same reason — the shell's loop has to fold each frame in, and a Pi
/// 3B+ has better things to do than take thirty frames a second. The cap
/// is [`MAX_FPS`] unless the caller asked for another with
/// [`Camera::set_max_fps`]: a shell that reduces the frame before it
/// reaches its loop pays less for one and can afford more of them.
///
/// The camera is dropped on the thread, so joining the returned handle
/// is what guarantees the device is closed.
pub fn spawn(mut camera: Camera, tx: SyncSender<Frame>, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let interval = Duration::from_millis(1000 / u64::from(camera.max_fps()));
        // A deadline that advances by the interval, not by the time of
        // the last send. Comparing against the last send beats against a
        // driver whose own period is a shade under the interval — one
        // frame in two is dropped and the rate halves — while a deadline
        // averages out to the cap whatever the driver's rate.
        let mut next = Instant::now();
        while !stop.load(Ordering::Relaxed) {
            match camera.frame(POLL) {
                Ok(Some(frame)) => {
                    let now = Instant::now();
                    // A full channel means the core is still on the last
                    // frame; this one is stale already.
                    if now >= next && tx.try_send(frame).is_ok() {
                        next += interval;
                        // More than an interval behind (a stalled
                        // device, a slow decode): start again from now
                        // rather than bursting to catch up.
                        if next < now {
                            next = now;
                        }
                    }
                }
                Ok(None) => {}
                // A device that has gone (unplugged, or the driver
                // unloaded) ends the thread; the shell sees the channel
                // close.
                Err(_) => break,
            }
        }
    })
}

/// How long [`spawn`] waits on each `poll` before checking `stop`.
const POLL: Duration = Duration::from_millis(100);

/// The most frames a second [`spawn`] sends on unless the caller raises
/// it with [`Camera::set_max_fps`].
pub const MAX_FPS: u8 = 10;

// --- the ioctl plumbing --------------------------------------------------

/// Opens a device read-write, which is what the streaming ioctls need.
fn open_rw(path: &Path) -> io::Result<File> {
    OpenOptions::new().read(true).write(true).open(path)
}

/// One `ioctl` with a pointer argument.
///
/// `arg` must point at an initialised value of exactly the type the
/// request number encodes the size of; that is what every caller here
/// does, and it is why the request numbers are derived from `size_of`
/// rather than pasted in.
fn ioctl<T>(fd: RawFd, request: u32, arg: *mut T) -> io::Result<()> {
    // SAFETY: `fd` is a file descriptor owned by a live `File` in every
    // caller, `request` is a `_IOC` number built from `size_of::<T>()`
    // for the same `T` the pointer has, and `arg` points at an
    // initialised, uniquely borrowed `T` that outlives the call. The
    // kernel writes at most `size_of::<T>()` bytes there.
    let r = unsafe { libc::ioctl(fd, request as _, arg.cast::<c_void>()) };
    if r < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// `VIDIOC_QUERYCAP`.
fn query_cap(fd: RawFd) -> io::Result<abi::Capability> {
    let mut cap = abi::Capability::default();
    ioctl(fd, abi::VIDIOC_QUERYCAP, &raw mut cap)?;
    Ok(cap)
}

/// `VIDIOC_S_FMT` or `VIDIOC_TRY_FMT` for one FourCC at one size; the
/// answer is what the driver would give, which may differ in both.
fn try_format(fd: RawFd, request: u32, format: u32, want: Size) -> io::Result<abi::Format> {
    let mut fmt = abi::Format::capture(abi::PixFormat {
        width: u32::from(want.width),
        height: u32::from(want.height),
        pixelformat: format,
        field: abi::FIELD_ANY,
        ..abi::PixFormat::default()
    });
    ioctl(fd, request, &raw mut fmt)?;
    Ok(fmt)
}

/// Sets the first format of the four the driver grants.
///
/// A driver that will not give the format asked for substitutes one of
/// its own and reports success, so the answer is checked rather than the
/// return value. When none of the four is granted outright but the
/// driver's own substitute is one of them, that one is set instead.
fn negotiate(fd: RawFd, want: Size) -> io::Result<abi::PixFormat> {
    let mut substitute = None;
    for &format in &abi::FORMATS {
        let fmt = try_format(fd, abi::VIDIOC_S_FMT, format, want)?;
        if fmt.pix.pixelformat == format {
            return Ok(fmt.pix);
        }
        if substitute.is_none() && convert::luma_step(fmt.pix.pixelformat).is_some() {
            substitute = Some(fmt.pix.pixelformat);
        }
    }
    let Some(format) = substitute else {
        return Err(io::Error::other(
            "the camera offers none of YUYV, NV12, YU12 or GREY",
        ));
    };
    let fmt = try_format(fd, abi::VIDIOC_S_FMT, format, want)?;
    if fmt.pix.pixelformat == format {
        Ok(fmt.pix)
    } else {
        Err(io::Error::other(
            "the camera would not settle on a pixel format",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `find` must not panic or hang on a box with no camera at all;
    /// whether it returns one depends on the box.
    #[test]
    fn find_answers_without_a_camera() {
        let _ = find();
    }
}
