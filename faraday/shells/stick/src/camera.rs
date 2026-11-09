//! The camera channel: the Pi camera through V4L2.
//!
//! The capture itself is `faraday-v4l2`, which owns the device on its
//! own thread and posts 8-bit luma frames to a bounded channel. This
//! module is the shell's side of that channel: start, stop, and one
//! frame at a time for the main loop to fold in between touches.
//!
//! The device is asked for 1920 × 1080 (`Size::FULL_HD`): a dense code,
//! version 23 at error correction H, has too few pixels a module at
//! 640 × 480 for the decoder to find. A camera without that mode gives
//! the nearest it has. If no usable frame comes within 2.5 s the device
//! is closed and asked for 1280 × 720, then 640 × 480, so that the
//! scanner is not left dead; those are a fallback, not the goal. The last
//! of them does not give up: it waits for a first frame until the shell
//! says stop, so a slow camera still gets a picture, and a camera that
//! gives nothing ends in the core's "The camera sends no picture". Each
//! try prints one line (size asked, size and format given, buffers
//! dequeued, flagged and short, milliseconds to the first frame), each
//! in one write, and about 5 s after the first frame one more line says
//! the same counts for the run, which is the sustained rate. At this size
//! the planar formats are asked for first (`YU12`, `NV12`, then `YUYV`):
//! a Pi's camera gives 1080p frames in `YU12` and none in `YUYV`.
//!
//! Each frame is reduced here, on the capture thread, before it is
//! queued: the core only wants the frame for the preview, so it is
//! handed the luma and chroma it would otherwise reduce itself (a short
//! side of at most `PREVIEW_MAX`, so 640 × 360 from a 1080p frame), and
//! the loop moves and wipes that rather than the full frame. The full
//! luma rides beside it, because that is what the scanner decodes.
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
//! and the kernel's `bcm2835` driver) the
//! OV5647 and the IMX219 appear as a plain `/dev/video0`, which is what
//! the search finds.

use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError, sync_channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use faraday_scanner::{reduce, reduce_chroma, reduce_factor};
use faraday_shell_api::CameraFailure;
use faraday_v4l2::{Camera as Device, Failure, Frame, Size};
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
    /// The device could not be opened, with the reason for `--verbose`
    /// and the one the app has a line for, when there is one.
    Unavailable(String, Option<CameraFailure>),
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
    /// contract's `Event::CameraFailed` with the reason when there is
    /// one, else `Event::CameraUnavailable`.
    Unavailable(Option<CameraFailure>),
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
            Ok(Message::Unavailable(reason, why)) => {
                self.reason = Some(reason);
                self.off();
                Some(Capture::Unavailable(why))
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

    /// Opens `path` from now on, the camera the person chose; a camera
    /// already running is restarted on it.
    pub fn choose(&mut self, path: PathBuf) {
        let running = self.session.is_some();
        self.device = Some(path);
        if running {
            self.off();
            let _ = self.on();
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
/// The capture is `faraday-v4l2`'s own thread, because that is where
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
    let path = match device {
        Some(p) => p,
        None => match faraday_v4l2::find_or_failure() {
            Ok(p) => p,
            Err(why) => {
                let _ = tx.send(Message::Unavailable(
                    String::from("no /dev/video* is a streaming capture device"),
                    Some(app_reason(why)),
                ));
                let _ = wake.try_send(Wake::Frame);
                return;
            }
        },
    };
    let (opened, first) = match climb(&LADDER, |size| attempt(&path, size, stop)) {
        Climb::Got(device, first) => (device, first),
        Climb::Failed(reason, why) => {
            let _ = tx.send(Message::Unavailable(reason, why));
            let _ = wake.try_send(Wake::Frame);
            return;
        }
        // A camera that gave nothing at any size: say nothing more, as a
        // camera that streams nothing does; the core's wait ends in "The
        // camera sends no picture".
        Climb::Nothing => {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(POLL);
            }
            return;
        }
        Climb::Stopped => return,
    };
    if tx.try_send(Message::Frame(prepare(first))).is_ok() {
        let _ = wake.try_send(Wake::Frame);
    }
    let (frames_tx, frames) = sync_channel(QUEUE);
    let inner_stop = Arc::new(AtomicBool::new(false));
    let inner = faraday_v4l2::spawn_reporting(
        opened,
        frames_tx,
        Arc::clone(&inner_stop),
        SUSTAINED,
        |size, format, stats| {
            say(format!(
                "camera after {} s: {}x{} {format}, dequeued {}, flagged {}, short {}",
                SUSTAINED.as_secs(),
                size.width,
                size.height,
                stats.dequeued,
                stats.flagged,
                stats.short,
            ));
        },
    );
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

/// The sizes asked of the device, in order: 1920 × 1080 is the goal, and
/// the smaller two are only so that the scanner is never left with no
/// picture at all.
const LADDER: [Size; 3] = [Size::FULL_HD, Size::HD, Size::VGA];

/// How long a try waits for its first usable frame before the device is
/// closed and the next size is tried. The last size waits without limit.
const FIRST_FRAME: Duration = Duration::from_millis(2500);

// The two tries that give up, each waiting out its time, end before the
// scan sheet says there is no picture, with a margin for opening the
// device; a first frame takes 50 to 92 ms. (The stick links
// `faraday-draw` in its tests only, so this holds there.)
#[cfg(test)]
const _: () = assert!(
    (FIRST_FRAME.as_millis() as u64) * (LADDER.len() as u64 - 1) + 1000
        <= faraday_draw::tokens::NO_PICTURE_MS
);

/// Polls `next` until it gives a frame or fails, up to `limit` (none: until
/// the shell says stop). `Err(None)` is a stop, `Ok(None)` is time up.
fn wait_first<E>(
    limit: Option<Duration>,
    stop: &AtomicBool,
    mut next: impl FnMut() -> Result<Option<Frame>, E>,
) -> Result<Option<Frame>, Option<E>> {
    let started = Instant::now();
    while limit.is_none_or(|l| started.elapsed() < l) {
        if stop.load(Ordering::Relaxed) {
            return Err(None);
        }
        if let Some(frame) = next().map_err(Some)? {
            return Ok(Some(frame));
        }
    }
    Ok(None)
}

/// What one try at one size came to.
enum Try<D> {
    /// The device streams: it, and the first usable frame.
    Frame(D, Frame),
    /// The device opened and gave no usable frame in time.
    Silent,
    /// The shell said stop during the try.
    Stopped,
    /// The device would not open or broke, with the reason for
    /// `--verbose` and the one the app has a line for, when there is one.
    Failed(String, Option<CameraFailure>),
}

/// What the whole ladder came to.
enum Climb<D> {
    /// A size that streams.
    Got(D, Frame),
    /// Every size that opened gave no frame.
    Nothing,
    /// The shell said stop.
    Stopped,
    /// No size opened.
    Failed(String, Option<CameraFailure>),
}

/// Tries `sizes` in order with `attempt`, which opens the device at the
/// size and closes it again unless it streams, and stops at the first
/// that streams. A device the kernel refuses, holds elsewhere or lacks
/// ends the climb at once, as another size would meet the same.
fn climb<D>(sizes: &[Size], mut attempt: impl FnMut(Size) -> Try<D>) -> Climb<D> {
    let mut silent = false;
    let mut failed = None;
    for &size in sizes {
        match attempt(size) {
            Try::Frame(device, first) => return Climb::Got(device, first),
            Try::Stopped => return Climb::Stopped,
            Try::Silent => silent = true,
            Try::Failed(reason, Some(why)) => return Climb::Failed(reason, Some(why)),
            Try::Failed(reason, None) => failed = Some(reason),
        }
    }
    match failed {
        Some(reason) if !silent => Climb::Failed(reason, None),
        _ => Climb::Nothing,
    }
}

/// One try: opens `path` at `size`, waits up to [`FIRST_FRAME`] (for ever on
/// the last size of the ladder) for a usable frame and prints one line saying what happened.
fn attempt(path: &std::path::Path, size: Size, stop: &AtomicBool) -> Try<Device> {
    let limit = (size != LADDER[LADDER.len() - 1]).then_some(FIRST_FRAME);
    let asked = format!("{}x{}", size.width, size.height);
    let mut device = match Device::open(path, size) {
        Ok(device) => device,
        Err(e) => {
            say(format!(
                "camera asked {asked}: cannot open {}: {e}",
                path.display()
            ));
            let why = Failure::of_io(&e).map(app_reason);
            return Try::Failed(format!("cannot open {}: {e}", path.display()), why);
        }
    };
    device.set_max_fps(FPS);
    let started = Instant::now();
    let (first, broke) = match wait_first(limit, stop, || device.frame(POLL)) {
        Ok(first) => (first, None),
        Err(None) => return Try::Stopped,
        Err(Some(e)) => (None, Some(e)),
    };
    let got = device.size();
    let stats = device.stats();
    let when = match &first {
        Some(_) => format!("first frame {} ms", started.elapsed().as_millis()),
        None => format!("no frame in {} ms", started.elapsed().as_millis()),
    };
    say(format!(
        "camera asked {asked}: gave {}x{} {}, dequeued {}, flagged {}, short {}, {when}",
        got.width,
        got.height,
        device.format(),
        stats.dequeued,
        stats.flagged,
        stats.short,
    ));
    if let Some(e) = broke {
        say(format!("camera asked {asked}: the device failed: {e}"));
        return Try::Failed(format!("the device failed: {e}"), None);
    }
    match first {
        Some(frame) => Try::Frame(device, frame),
        None => Try::Silent,
    }
}

/// Prints one camera line in a single write, so that it reaches the
/// kernel log whole and not in fragments.
fn say(mut line: String) {
    use std::io::Write;
    line.push('\n');
    let _ = std::io::stderr().write_all(line.as_bytes());
}

/// How long after the first frame the sustained rate is printed.
const SUSTAINED: Duration = Duration::from_secs(5);

/// The app's reason for a V4L2 one.
fn app_reason(why: Failure) -> CameraFailure {
    match why {
        Failure::Refused => CameraFailure::Refused,
        Failure::InUse => CameraFailure::InUse,
        Failure::NotFound => CameraFailure::NotFound,
    }
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

/// The cameras this machine has, as `(path, name)`: every
/// `/dev/videoN` whose sysfs node is its device's first (`index` 0), which
/// is how a UVC camera's metadata node and a Pi's ISP nodes are told from
/// the capture node without opening any of them. The name is the one the
/// device gives.
pub fn list() -> Vec<(String, String)> {
    faraday_camera::list()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(size: Size) -> Frame {
        Frame {
            width: size.width,
            height: size.height,
            luma: vec![0; usize::from(size.width) * usize::from(size.height)],
            chroma: None,
        }
    }

    /// A camera that streams only at `works`, noting each size asked.
    fn camera(works: Option<Size>, asked: &mut Vec<Size>) -> impl FnMut(Size) -> Try<Size> + '_ {
        move |size| {
            asked.push(size);
            if Some(size) == works {
                Try::Frame(size, frame(size))
            } else {
                Try::Silent
            }
        }
    }

    #[test]
    fn a_camera_that_streams_only_smaller_reaches_the_scanner() {
        let mut asked = Vec::new();
        let got = climb(&LADDER, camera(Some(Size::VGA), &mut asked));
        assert!(matches!(got, Climb::Got(Size::VGA, f) if f.width == 640 && f.height == 480));
        assert_eq!(asked, [Size::FULL_HD, Size::HD, Size::VGA]);
    }

    #[test]
    fn a_camera_that_streams_at_full_hd_is_not_asked_for_less() {
        let mut asked = Vec::new();
        let got = climb(&LADDER, camera(Some(Size::FULL_HD), &mut asked));
        assert!(matches!(got, Climb::Got(Size::FULL_HD, _)));
        assert_eq!(asked, [Size::FULL_HD]);
    }

    #[test]
    fn a_camera_that_streams_nowhere_says_so() {
        let mut asked = Vec::new();
        let got = climb(&LADDER, camera(None, &mut asked));
        assert!(matches!(got, Climb::Nothing));
        assert_eq!(asked.len(), 3);
    }

    fn never() -> Result<Option<Frame>, ()> {
        std::thread::sleep(Duration::from_millis(5));
        Ok(None)
    }

    #[test]
    fn a_camera_slow_only_at_full_hd_by_under_the_wait_stays_there() {
        let stop = AtomicBool::new(false);
        let mut polls = 0;
        let got = wait_first(Some(FIRST_FRAME), &stop, || {
            polls += 1;
            Ok::<_, ()>((polls == 3).then(|| frame(Size::FULL_HD)))
        });
        assert!(matches!(got, Ok(Some(f)) if f.width == 1920));
    }

    #[test]
    fn the_last_size_waits_until_a_stop() {
        let stop = AtomicBool::new(false);
        let mut polls = 0;
        let got = wait_first(None, &stop, || {
            polls += 1;
            if polls == 20 {
                stop.store(true, Ordering::Relaxed);
            }
            never()
        });
        assert!(matches!(got, Err(None)));
        assert_eq!(polls, 20);
    }

    #[test]
    fn a_size_with_a_limit_gives_up_when_it_passes() {
        let stop = AtomicBool::new(false);
        let got = wait_first(Some(Duration::from_millis(30)), &stop, never);
        assert!(matches!(got, Ok(None)));
    }

    #[test]
    fn a_stop_ends_the_climb_at_once() {
        let mut asked = 0;
        let got = climb(&LADDER, |_| {
            asked += 1;
            Try::<Size>::Stopped
        });
        assert!(matches!(got, Climb::Stopped));
        assert_eq!(asked, 1);
    }

    #[test]
    fn a_camera_held_elsewhere_ends_the_climb_with_its_reason() {
        let mut asked = 0;
        let got = climb(&LADDER, |_| {
            asked += 1;
            Try::<Size>::Failed(String::from("busy"), Some(CameraFailure::InUse))
        });
        assert!(matches!(got, Climb::Failed(_, Some(CameraFailure::InUse))));
        assert_eq!(asked, 1);
    }

    #[test]
    fn a_size_that_will_not_open_is_followed_by_one_that_does() {
        let got = climb(&LADDER, |size| {
            if size == Size::FULL_HD {
                Try::Failed(String::from("no such mode"), None)
            } else {
                Try::Frame(size, frame(size))
            }
        });
        assert!(matches!(got, Climb::Got(Size::HD, _)));
    }
}
