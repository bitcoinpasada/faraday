//! The camera channel: the Pi camera through V4L2.
//!
//! The capture itself is `opensigner-v4l2`, which owns the device on its
//! own thread and posts 8-bit luma frames to a bounded channel. This
//! module is the shell's side of that channel: start, stop, and one
//! frame at a time for the main loop to fold in between touches.
//!
//! Each frame is reduced here, on the capture thread, before it is
//! queued: the core only wants the frame for the preview, so it is
//! handed the 320 × 240 luma and chroma it would otherwise reduce
//! itself, and the loop moves and wipes a quarter of the bytes. The
//! full 640 × 480 luma rides beside it, because that is what the
//! scanner decodes.
//!
//! The capture thread also posts a note on the shell's wake channel for
//! each frame it queues, so the loop's wait ends on a frame as it ends
//! on a touch. The note carries nothing and is dropped when that channel
//! is full: the frame is what matters and it is already queued here.
//!
//! Opening the device is part of what the thread does, not part of
//! [`Camera::on`]. Finding a `/dev/video*`, negotiating a format and
//! mapping the buffers takes half a second on this board, and the loop
//! that calls `on` is the loop that draws: doing it here froze the
//! screen on the tap that opened the scanner. So `on` starts a thread
//! and returns, the thread opens the device, and a device that will not
//! open arrives on the frame channel as the `CameraUnavailable` the
//! contract already has for it.
//!
//! On the legacy camera stack (`start_x=1`, the firmware's `start_x.elf`
//! and the kernel's `bcm2835` driver, `docs/PLANNING.md` §16.33) the
//! OV5647 and the IMX219 appear as a plain `/dev/video0`, which is what
//! the search finds.

use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError, sync_channel};
use std::thread::JoinHandle;
use std::time::Duration;

use opensigner_core::scan::{reduce, reduce_chroma, reduce_factor};
use opensigner_v4l2::{Camera as Device, Frame, Size};
use zeroize::Zeroizing;

use crate::Wake;

/// Frames the channel holds. Two is one being decoded and one waiting; a
/// third would be out of date before the core reached it, and this
/// device decodes a frame slowly enough for that to matter.
const QUEUE: usize = 2;

/// How long the forwarding thread waits on the capture channel before
/// looking at the stop flag again.
const POLL: Duration = Duration::from_millis(100);

/// Frames a second the capture thread asks the device for. A frame
/// costs the loop a reduced frame's worth of copying now rather than a
/// full one's, so the preview on the panel is drawn oftener than the
/// crate's default ten.
const FPS: u8 = 15;

/// What the capture thread sends back.
enum Message {
    /// One captured frame, waiting to be taken.
    Frame(Queued),
    /// The device could not be opened, with the reason for `--verbose`.
    Unavailable(String),
}

/// A captured frame on the channel: reduced for the core's preview, and
/// full-size for the scanner's decode. Both wait in `Zeroizing`, since a
/// frame of a seed's QR code is a secret until it is wiped.
struct Queued {
    width: u16,
    height: u16,
    luma: Zeroizing<Vec<u8>>,
    chroma: Option<Zeroizing<Vec<u8>>>,
    full_width: u16,
    full_height: u16,
    full_luma: Zeroizing<Vec<u8>>,
}

/// One frame as the main loop takes it.
pub struct Shot {
    /// The reduced frame's width in pixels.
    pub width: u16,
    /// The reduced frame's height in pixels.
    pub height: u16,
    /// The reduced luma, which the core draws as the preview.
    pub luma: Vec<u8>,
    /// The reduced NV12 chroma, where the device gave colour.
    pub chroma: Option<Vec<u8>>,
    /// The captured frame's width in pixels.
    pub full_width: u16,
    /// The captured frame's height in pixels.
    pub full_height: u16,
    /// The captured luma, which the scanner decodes.
    pub full_luma: Vec<u8>,
}

/// What [`Camera::poll`] has for the loop.
pub enum Capture {
    /// A frame to draw and to decode.
    Frame(Shot),
    /// No device, or one that would not stream: the loop sends the
    /// contract's `Event::CameraUnavailable`.
    Unavailable,
}

/// A running capture: the thread, its stop flag, and the frames.
struct Session {
    messages: Receiver<Message>,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

/// The shell's side of the camera channel.
pub struct Camera {
    device: Option<PathBuf>,
    disabled: bool,
    /// The loop's wake channel, for the capture thread to say a frame is
    /// ready.
    wake: SyncSender<Wake>,
    session: Option<Session>,
    /// Why the last attempt gave no frames, for the caller to print.
    reason: Option<String>,
}

impl Camera {
    /// A camera that opens `device` when asked, or searches
    /// `/dev/video*` when that is `None`. `disabled` is `--no-camera`,
    /// and `wake` is the channel the main loop waits on.
    pub fn new(device: Option<PathBuf>, disabled: bool, wake: SyncSender<Wake>) -> Camera {
        Camera {
            device,
            disabled,
            wake,
            session: None,
            reason: None,
        }
    }

    /// Starts capture, or does nothing when it is already running, as
    /// the contract requires. Returns as soon as the thread is running:
    /// a device that will not open is reported later, as an
    /// `Event::CameraUnavailable` from [`Camera::poll`]. The error here
    /// is only for a camera that was refused before anything was tried.
    pub fn on(&mut self) -> Result<(), String> {
        if self.disabled {
            return Err(String::from("--no-camera"));
        }
        if self.session.is_some() {
            return Ok(());
        }
        let (tx, messages) = sync_channel(QUEUE);
        let stop = Arc::new(AtomicBool::new(false));
        let device = self.device.clone();
        let wake = self.wake.clone();
        let thread = {
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || capture(device, &tx, &wake, &stop))
        };
        self.session = Some(Session {
            messages,
            stop,
            thread,
        });
        Ok(())
    }

    /// Stops capture and waits for the thread, which is what closes the
    /// device. Harmless when the camera is off.
    pub fn off(&mut self) {
        if let Some(session) = self.session.take() {
            session.stop.store(true, Ordering::Relaxed);
            let _ = session.thread.join();
        }
    }

    /// The next captured frame, if one is waiting. Never blocks, so
    /// folding it into the main loop cannot delay a touch.
    pub fn poll(&mut self) -> Option<Capture> {
        let session = self.session.as_ref()?;
        match session.messages.try_recv() {
            Ok(Message::Frame(mut f)) => Some(Capture::Frame(Shot {
                width: f.width,
                height: f.height,
                // Taken out of the wrapper rather than copied: what the
                // wrapper leaves behind is an empty vector, and the
                // bytes go straight on to the core and the scanner,
                // which wipe them in their turn.
                luma: mem::take(&mut *f.luma),
                chroma: f.chroma.as_mut().map(|uv| mem::take(&mut **uv)),
                full_width: f.full_width,
                full_height: f.full_height,
                full_luma: mem::take(&mut *f.full_luma),
            })),
            // No device, or one that would not stream. Turn the camera
            // off so that a later `CameraOn` tries again.
            Ok(Message::Unavailable(reason)) => {
                self.reason = Some(reason);
                self.off();
                Some(Capture::Unavailable)
            }
            Err(TryRecvError::Empty) => None,
            // The capture thread ended on its own: the device is gone.
            // Tidy up so a later CameraOn tries again.
            Err(TryRecvError::Disconnected) => {
                self.off();
                None
            }
        }
    }

    /// Why the last `CameraUnavailable` came, once.
    pub fn take_reason(&mut self) -> Option<String> {
        self.reason.take()
    }
}

/// The thread `on` starts: open the device, then pass its frames on
/// until the shell says stop.
///
/// The capture is `opensigner-v4l2`'s own thread, because that is where
/// the device is owned and closed; this one waits on it, reduces it for
/// the preview and forwards both sizes. A full channel drops the frame:
/// the shell only ever wants the newest. Every message it does send is
/// followed by a wake, dropped rather than waited on, so this thread
/// never blocks on the main loop.
fn capture(
    device: Option<PathBuf>,
    tx: &SyncSender<Message>,
    wake: &SyncSender<Wake>,
    stop: &AtomicBool,
) {
    let opened = match device {
        Some(p) => {
            Device::open(&p, Size::VGA).map_err(|e| format!("cannot open {}: {e}", p.display()))
        }
        None => match opensigner_v4l2::find() {
            Some(p) => {
                Device::open(&p, Size::VGA).map_err(|e| format!("cannot open {}: {e}", p.display()))
            }
            None => Err(String::from("no /dev/video* is a streaming capture device")),
        },
    };
    let opened = match opened {
        Ok(mut device) => {
            device.set_max_fps(FPS);
            device
        }
        Err(reason) => {
            let _ = tx.send(Message::Unavailable(reason));
            let _ = wake.try_send(Wake::Frame);
            return;
        }
    };
    let (frames_tx, frames) = sync_channel(QUEUE);
    let inner_stop = Arc::new(AtomicBool::new(false));
    let inner = opensigner_v4l2::spawn(opened, frames_tx, Arc::clone(&inner_stop));
    while !stop.load(Ordering::Relaxed) {
        match frames.recv_timeout(POLL) {
            Ok(frame) => {
                if tx.try_send(Message::Frame(prepare(frame))).is_ok() {
                    let _ = wake.try_send(Wake::Frame);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            // The device is gone; ending here closes the shell's channel.
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    inner_stop.store(true, Ordering::Relaxed);
    let _ = inner.join();
}

/// One captured frame reduced for the preview, with the captured luma
/// kept beside it for the scanner.
///
/// The reduced frame is what `Event::CameraFrame` carries, so the core
/// reduces nothing: `reduce_factor` of a 320 × 240 frame is 1. A device
/// whose chroma plane is short of what its size says reduces to nothing,
/// and that frame's preview is grey rather than wrong.
fn prepare(frame: Frame) -> Queued {
    let (w, h) = (usize::from(frame.width), usize::from(frame.height));
    let (width, height, luma) = reduce(w, h, &frame.luma);
    let factor = reduce_factor(w, h);
    let chroma = frame
        .chroma
        .map(|uv| reduce_chroma(w, h, &uv, factor))
        .filter(|uv| !uv.is_empty())
        .map(Zeroizing::new);
    Queued {
        width,
        height,
        luma: Zeroizing::new(luma),
        chroma,
        full_width: frame.width,
        full_height: frame.height,
        full_luma: Zeroizing::new(frame.luma),
    }
}
