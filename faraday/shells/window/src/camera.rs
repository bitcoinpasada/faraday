//! The camera channel, which exists on Linux, macOS and Windows.
//!
//! On Linux `faraday-v4l2` finds a `/dev/video*`, streams it on its
//! own thread and posts frames to a bounded channel; the event loop
//! drains that channel on every pass, so a frame reaches the core within
//! a tick of capture and a slow decode drops frames instead of backing
//! up. On macOS `faraday-avfoundation` does the same through an
//! `AVCaptureSession`, which posts to the channel from AVFoundation's
//! own queue. On Windows `faraday-mediafoundation` reads a Media
//! Foundation source reader on its own thread and posts to the channel
//! from there; the scan sheet's camera choice lists Media Foundation's
//! devices by name. Anywhere else every `CameraOn` is refused and the
//! app offers its file fallback.
//!
//! macOS answers the permission prompt when it pleases rather than at
//! `CameraOn`, and Windows' privacy setting is only known once the
//! device is opened on the capture thread, so on both a refusal arrives
//! as a message on the frame channel and not as an error from `on`. That
//! is why `on` returning `Ok` is not a promise of frames, on any
//! platform.
//!
//! On Linux a webcam's first stream after it wakes may end, or stay
//! silent, where a second one opened straight after streams. So a
//! capture that ends or sends nothing for three seconds before its
//! first frame is started again, once for each `CameraOn`; a capture
//! that ends after that tells the app.
//!
//! Every platform presents the same three calls, so the command match in
//! the shells (the desktop app's and the Starter's) has no `#[cfg]` in it.

use std::path::PathBuf;

use faraday_shell_api::{CameraFailure, Event};

/// A camera that would not start: the reason to print, and the reason
/// to tell the app when it is clearly one of the three the app has
/// lines for.
#[derive(Debug)]
pub struct Unavailable {
    /// For `--verbose` and the terminal.
    pub reason: String,
    /// Refused, in use or not found; `None` when it is none of these.
    pub why: Option<CameraFailure>,
}

impl Unavailable {
    /// A failure that is none of the three.
    fn plain(reason: impl Into<String>) -> Unavailable {
        Unavailable {
            reason: reason.into(),
            why: None,
        }
    }

    /// The event that tells the app: `CameraFailed` with the reason, or
    /// `CameraUnavailable` when the reason is not known.
    pub fn event(&self) -> Event {
        event_for(self.why)
    }
}

impl std::fmt::Display for Unavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

/// The event for a camera that gave no frames: `CameraFailed` when the
/// reason is known, else `CameraUnavailable`.
fn event_for(why: Option<CameraFailure>) -> Event {
    match why {
        Some(why) => Event::CameraFailed(why),
        None => Event::CameraUnavailable,
    }
}

/// The shell's side of the camera channel.
pub struct Camera {
    inner: Inner,
}

impl Camera {
    /// A camera that will open `device` when asked, or search for one
    /// when that is `None`. `disabled` is `--no-camera`, which makes
    /// every request unavailable so that the file fallback can be
    /// reviewed on a machine that has a webcam.
    pub fn new(device: Option<PathBuf>, disabled: bool) -> Camera {
        Camera {
            inner: Inner::new(device, disabled),
        }
    }

    /// Starts capture. Doing this while the camera is already on is a
    /// no-op, as the contract requires. The error is a reason to print
    /// and, when it is known, the reason to tell the app
    /// ([`Unavailable::event`]); the caller answers once.
    pub fn on(&mut self) -> Result<(), Unavailable> {
        self.inner.on()
    }

    /// Stops capture and waits for the capture to be torn down, which
    /// is what closes the device. Harmless when the camera is off.
    pub fn off(&mut self) {
        self.inner.off();
    }

    /// The next captured frame, if one is waiting. Called until it
    /// answers `None`.
    pub fn poll(&mut self) -> Option<Event> {
        self.inner.poll()
    }

    /// Opens `path` from now on, the camera the person chose on the scan
    /// sheet; a camera already running is restarted on it. On Linux it is
    /// a `/dev/videoN`, on Windows a device's symbolic link, on macOS a
    /// device's unique ID.
    pub fn choose(&mut self, path: PathBuf) {
        self.inner.choose(path);
    }
}

/// The cameras this machine has, as `(path, name)`, for the scan sheet's
/// choice. On Linux, every `/dev/videoN` whose sysfs node is its
/// device's first (`index` 0), which tells a webcam's capture node from
/// its metadata node without opening either; the name is the one the
/// device gives. The stick shell lists them the same way. On Windows,
/// Media Foundation's video capture devices: the symbolic link and the
/// name Windows shows (`windows::list`). On macOS, AVFoundation's video
/// devices: the unique ID and the name macOS shows (`macos::list`).
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
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

#[cfg(target_os = "linux")]
use linux::Inner;

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    use faraday_shell_api::{CameraFailure, Event};
    use faraday_v4l2::{Camera as Device, Failure, Frame, MAX_FPS, Size};

    use super::Unavailable;

    /// The app's reason for a V4L2 one.
    fn reason(why: Failure) -> CameraFailure {
        match why {
            Failure::Refused => CameraFailure::Refused,
            Failure::InUse => CameraFailure::InUse,
            Failure::NotFound => CameraFailure::NotFound,
        }
    }

    /// Frames the channel holds. Two is one being decoded and one
    /// waiting; a third would already be out of date by the time the
    /// core reached it.
    const QUEUE: usize = 2;

    /// How long a capture may stay silent after it starts before it is
    /// started again, once. A webcam's first stream after it wakes can
    /// end or stay silent where a second one, opened straight after,
    /// streams. This is half the app's own wait before it says the
    /// camera sends no picture (`NO_PICTURE_MS`, `docs/QR.md` §8 item
    /// 6), so the second stream has time to show a frame before then.
    const FIRST_FRAME: Duration = Duration::from_secs(3);

    /// A running capture: the thread, its stop flag, the frames, when it
    /// started and whether a frame has come from it yet.
    struct Session {
        frames: Receiver<Frame>,
        stop: Arc<AtomicBool>,
        thread: JoinHandle<()>,
        started: Instant,
        seen: bool,
    }

    impl Session {
        /// A capture just started, on `thread`, posting to `frames`.
        fn new(frames: Receiver<Frame>, stop: Arc<AtomicBool>, thread: JoinHandle<()>) -> Session {
            Session {
                frames,
                stop,
                thread,
                started: Instant::now(),
                seen: false,
            }
        }
    }

    /// What starts a capture: of the device given, or of the camera
    /// found when that is `None`.
    type Start = Box<dyn FnMut(Option<&Path>) -> Result<Session, Unavailable> + Send>;

    pub struct Inner {
        device: Option<PathBuf>,
        disabled: bool,
        session: Option<Session>,
        /// Whether the capture for this `CameraOn` was started again.
        retried: bool,
        start: Start,
    }

    /// Costs opening a device (a format negotiated, buffers mapped, a
    /// stream started) and a thread. V4L2 capture of `device`, or of the
    /// camera `faraday_v4l2::find_or_failure` finds when that is `None`.
    fn v4l2(device: Option<&Path>) -> Result<Session, Unavailable> {
        let path = match device {
            Some(p) => p.to_path_buf(),
            None => faraday_v4l2::find_or_failure().map_err(|why| Unavailable {
                reason: String::from("no /dev/video* is a streaming capture device"),
                why: Some(reason(why)),
            })?,
        };
        let device = Device::open(&path, Size::HD).map_err(|e| Unavailable {
            reason: format!("cannot open {}: {e}", path.display()),
            why: Failure::of_io(&e).map(reason),
        })?;
        let size = device.size();
        eprintln!(
            "camera {}: {} {}x{}, up to {MAX_FPS} frames a second",
            path.display(),
            device.format(),
            size.width,
            size.height
        );
        let (tx, frames) = sync_channel(QUEUE);
        let stop = Arc::new(AtomicBool::new(false));
        let thread = faraday_v4l2::spawn(device, tx, Arc::clone(&stop));
        Ok(Session::new(frames, stop, thread))
    }

    impl Inner {
        pub fn new(device: Option<PathBuf>, disabled: bool) -> Inner {
            Inner::with_start(device, disabled, Box::new(v4l2))
        }

        /// An `Inner` whose captures `start` starts.
        fn with_start(device: Option<PathBuf>, disabled: bool, start: Start) -> Inner {
            Inner {
                device,
                disabled,
                session: None,
                retried: false,
                start,
            }
        }

        pub fn on(&mut self) -> Result<(), Unavailable> {
            if self.disabled {
                return Err(Unavailable::plain("--no-camera"));
            }
            if self.session.is_some() {
                return Ok(());
            }
            self.retried = false;
            self.session = Some((self.start)(self.device.as_deref())?);
            Ok(())
        }

        pub fn choose(&mut self, path: PathBuf) {
            let running = self.session.is_some();
            self.device = Some(path);
            if running {
                self.off();
                if let Err(reason) = self.on() {
                    eprintln!("faraday: camera unavailable: {reason}");
                }
            }
        }

        pub fn off(&mut self) {
            if let Some(session) = self.session.take() {
                session.stop.store(true, Ordering::Relaxed);
                // The camera is dropped on that thread, so the join is
                // what closes the device.
                let _ = session.thread.join();
            }
        }

        pub fn poll(&mut self) -> Option<Event> {
            self.poll_at(Instant::now())
        }

        /// [`Inner::poll`] at `now`. A capture that ends, or stays
        /// silent for [`FIRST_FRAME`], before its first frame is started
        /// again, once for each `CameraOn`; one that ends after that
        /// tells the app, and turns off so a later `CameraOn` tries
        /// again.
        fn poll_at(&mut self, now: Instant) -> Option<Event> {
            let session = self.session.as_mut()?;
            let first = !session.seen && !self.retried;
            match session.frames.try_recv() {
                Ok(f) => {
                    session.seen = true;
                    Some(Event::CameraFrame {
                        width: f.width,
                        height: f.height,
                        luma: f.luma,
                        chroma: f.chroma,
                    })
                }
                Err(TryRecvError::Empty) => {
                    if first && now.saturating_duration_since(session.started) >= FIRST_FRAME {
                        eprintln!("camera: no frame yet; starting the capture again");
                        return self.again();
                    }
                    None
                }
                // The capture thread ended on its own: the device failed
                // or is gone.
                Err(TryRecvError::Disconnected) => {
                    if first {
                        eprintln!(
                            "camera: the capture ended before its first frame; starting again"
                        );
                        return self.again();
                    }
                    eprintln!("camera: the device stopped delivering frames");
                    self.off();
                    Some(Event::CameraUnavailable)
                }
            }
        }

        /// Stops the capture and starts it again; the event to send the
        /// app when it will not start.
        fn again(&mut self) -> Option<Event> {
            self.off();
            self.retried = true;
            match (self.start)(self.device.as_deref()) {
                Ok(session) => {
                    self.session = Some(session);
                    None
                }
                Err(reason) => {
                    eprintln!("faraday: camera unavailable: {reason}");
                    Some(reason.event())
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use std::sync::atomic::AtomicUsize;
        use std::sync::mpsc::SyncSender;

        use super::*;

        /// A frame as a camera would send it.
        fn frame() -> Frame {
            Frame {
                width: 64,
                height: 48,
                luma: vec![120; 64 * 48],
                chroma: None,
            }
        }

        /// A fake camera: the `n`th capture started (from 0) runs
        /// `stream` on its own thread until stopped. Counts the starts.
        fn fake(stream: fn(usize, &SyncSender<Frame>, &AtomicBool)) -> (Inner, Arc<AtomicUsize>) {
            let starts = Arc::new(AtomicUsize::new(0));
            let counted = Arc::clone(&starts);
            let start: Start = Box::new(move |_| {
                let n = counted.fetch_add(1, Ordering::SeqCst);
                let (tx, frames) = sync_channel(QUEUE);
                let stop = Arc::new(AtomicBool::new(false));
                let flag = Arc::clone(&stop);
                let thread = std::thread::spawn(move || stream(n, &tx, &flag));
                Ok(Session::new(frames, stop, thread))
            });
            (Inner::with_start(None, false, start), starts)
        }

        /// Sends a frame every few milliseconds until stopped.
        fn streams(tx: &SyncSender<Frame>, stop: &AtomicBool) {
            while !stop.load(Ordering::Relaxed) {
                let _ = tx.try_send(frame());
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        /// Sends nothing until stopped.
        fn silent(stop: &AtomicBool) {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(5));
            }
        }

        /// Polls for up to two seconds, at `at` past now, for the first
        /// event.
        fn first_event(inner: &mut Inner, at: Duration) -> Option<Event> {
            for _ in 0..400 {
                if let Some(e) = inner.poll_at(Instant::now() + at) {
                    return Some(e);
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            None
        }

        #[test]
        fn a_first_stream_that_ends_without_a_frame_is_started_again() {
            let (mut inner, starts) = fake(|n, tx, stop| {
                if n > 0 {
                    streams(tx, stop);
                }
            });
            assert!(inner.on().is_ok());
            let e = first_event(&mut inner, Duration::ZERO);
            assert!(
                matches!(e, Some(Event::CameraFrame { .. })),
                "no picture: {e:?}"
            );
            assert_eq!(starts.load(Ordering::SeqCst), 2);
            inner.off();
        }

        #[test]
        fn a_first_stream_that_stays_silent_is_started_again_once() {
            let (mut inner, starts) = fake(|n, tx, stop| {
                if n == 1 {
                    streams(tx, stop);
                } else {
                    silent(stop);
                }
            });
            assert!(inner.on().is_ok());
            // Silent, but not for long yet: left alone.
            assert!(inner.poll_at(Instant::now()).is_none());
            assert_eq!(starts.load(Ordering::SeqCst), 1);
            let e = first_event(&mut inner, FIRST_FRAME);
            assert!(
                matches!(e, Some(Event::CameraFrame { .. })),
                "no picture: {e:?}"
            );
            assert_eq!(starts.load(Ordering::SeqCst), 2);
            inner.off();

            // A second request starts afresh, and a camera that stays
            // silent then is started again once only: the app says it
            // sends no picture.
            assert!(inner.on().is_ok());
            for _ in 0..5 {
                assert!(inner.poll_at(Instant::now() + FIRST_FRAME * 4).is_none());
            }
            assert_eq!(starts.load(Ordering::SeqCst), 4);
            inner.off();
        }

        #[test]
        fn a_stream_that_ends_after_its_frames_tells_the_app() {
            let (mut inner, starts) = fake(|_, tx, _| {
                let _ = tx.try_send(frame());
            });
            assert!(inner.on().is_ok());
            let e = first_event(&mut inner, Duration::ZERO);
            assert!(matches!(e, Some(Event::CameraFrame { .. })));
            let e = first_event(&mut inner, Duration::ZERO);
            assert!(matches!(e, Some(Event::CameraUnavailable)), "{e:?}");
            assert_eq!(starts.load(Ordering::SeqCst), 1);
        }
    }
}

#[cfg(target_os = "macos")]
use macos::Inner;
#[cfg(target_os = "macos")]
pub use macos::list;

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};

    use faraday_avfoundation::{Camera as Session, MAX_FPS, Message};
    use faraday_shell_api::{CameraFailure, Event};

    use super::{Unavailable, event_for};
    /// The app's reason for the backend's.
    fn failure(why: faraday_avfoundation::Failure) -> CameraFailure {
        match why {
            faraday_avfoundation::Failure::Refused => CameraFailure::Refused,
            faraday_avfoundation::Failure::InUse => CameraFailure::InUse,
            faraday_avfoundation::Failure::NotFound => CameraFailure::NotFound,
        }
    }

    /// Frames the channel holds, as on Linux: one being decoded and one
    /// waiting. A third would be out of date by the time the core
    /// reached it, and the capture callback drops it instead.
    const QUEUE: usize = 2;

    /// The cameras this machine has, as `(unique ID, name)`, for the scan
    /// sheet's choice, in the system's order: the first is not
    /// necessarily the default one, which is opened when none was chosen.
    pub fn list() -> Vec<(String, String)> {
        faraday_avfoundation::devices()
            .into_iter()
            .map(|d| (d.id, d.name))
            .collect()
    }

    pub struct Inner {
        /// The unique ID of the camera to open, or `None` for the
        /// system's default.
        device: Option<String>,
        disabled: bool,
        session: Option<(Session, Receiver<Message>)>,
    }

    impl Inner {
        pub fn new(device: Option<PathBuf>, disabled: bool) -> Inner {
            Inner {
                device: device.map(|p| p.to_string_lossy().into_owned()),
                disabled,
                session: None,
            }
        }

        pub fn on(&mut self) -> Result<(), Unavailable> {
            if self.disabled {
                return Err(Unavailable::plain("--no-camera"));
            }
            if self.session.is_some() {
                return Ok(());
            }
            let (tx, frames) = sync_channel(QUEUE);
            // Starting is asynchronous: the permission prompt, the
            // device and the session all answer on the channel. A
            // failure arrives as `Unavailable`, never as an error here.
            let session = Session::start(self.device.as_deref(), tx);
            eprintln!(
                "camera: AVFoundation, {}, up to {MAX_FPS} frames a second",
                self.device.as_deref().unwrap_or("the default device")
            );
            self.session = Some((session, frames));
            Ok(())
        }

        pub fn choose(&mut self, path: PathBuf) {
            let running = self.session.is_some();
            self.device = Some(path.to_string_lossy().into_owned());
            if running {
                self.off();
                if let Err(reason) = self.on() {
                    eprintln!("faraday: camera unavailable: {reason}");
                }
            }
        }

        pub fn off(&mut self) {
            // Dropping the session stops it: the capture is torn down
            // before the drop returns.
            self.session = None;
        }

        pub fn poll(&mut self) -> Option<Event> {
            let (_, frames) = self.session.as_ref()?;
            match frames.try_recv() {
                Ok(Message::Frame(f)) => Some(Event::CameraFrame {
                    width: f.width,
                    height: f.height,
                    luma: f.luma,
                    chroma: f.chroma,
                }),
                // No camera, or the user refused. Turn the camera off so
                // that a later `CameraOn` starts a session and asks
                // again; macOS answers a second request from its
                // remembered decision, without another prompt.
                Ok(Message::Unavailable(why)) => {
                    eprintln!("camera: no device, or macOS refused access");
                    self.off();
                    Some(event_for(why.map(failure)))
                }
                Err(TryRecvError::Empty) => None,
                // The sender lives as long as the session, so this is
                // unreachable while one is on; treat it as no frame.
                Err(TryRecvError::Disconnected) => None,
            }
        }
    }
}

#[cfg(target_os = "windows")]
use windows::Inner;
#[cfg(target_os = "windows")]
pub use windows::list;

#[cfg(target_os = "windows")]
mod windows {
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};

    use faraday_mediafoundation::{Camera as Session, MAX_FPS, Message};
    use faraday_shell_api::{CameraFailure, Event};

    use super::{Unavailable, event_for};
    /// The app's reason for the backend's.
    fn failure(why: faraday_mediafoundation::Failure) -> CameraFailure {
        match why {
            faraday_mediafoundation::Failure::Refused => CameraFailure::Refused,
            faraday_mediafoundation::Failure::InUse => CameraFailure::InUse,
            faraday_mediafoundation::Failure::NotFound => CameraFailure::NotFound,
        }
    }

    /// Frames the channel holds, as on Linux: one being decoded and one
    /// waiting. A third would be out of date by the time the core
    /// reached it, and the capture callback drops it instead.
    const QUEUE: usize = 2;

    /// The cameras this machine has, as `(symbolic link, name)`, for the
    /// scan sheet's choice, in Media Foundation's order: the first is
    /// the one opened when none was chosen.
    pub fn list() -> Vec<(String, String)> {
        faraday_mediafoundation::devices()
            .into_iter()
            .map(|d| (d.link, d.name))
            .collect()
    }

    pub struct Inner {
        /// The symbolic link of the camera to open, or `None` for the
        /// first.
        device: Option<String>,
        disabled: bool,
        session: Option<(Session, Receiver<Message>)>,
    }

    impl Inner {
        pub fn new(device: Option<PathBuf>, disabled: bool) -> Inner {
            Inner {
                device: device.map(|p| p.to_string_lossy().into_owned()),
                disabled,
                session: None,
            }
        }

        pub fn on(&mut self) -> Result<(), Unavailable> {
            if self.disabled {
                return Err(Unavailable::plain("--no-camera"));
            }
            if self.session.is_some() {
                return Ok(());
            }
            let (tx, frames) = sync_channel(QUEUE);
            // Starting is asynchronous: the device, the privacy setting
            // and the format all answer on the channel. A failure
            // arrives as `Unavailable`, never as an error here.
            let session = Session::start(self.device.as_deref(), tx);
            eprintln!(
                "camera: Media Foundation, {}, up to {MAX_FPS} frames a second",
                self.device.as_deref().unwrap_or("the first device")
            );
            self.session = Some((session, frames));
            Ok(())
        }

        pub fn choose(&mut self, path: PathBuf) {
            let running = self.session.is_some();
            self.device = Some(path.to_string_lossy().into_owned());
            if running {
                self.off();
                if let Err(reason) = self.on() {
                    eprintln!("faraday: camera unavailable: {reason}");
                }
            }
        }

        pub fn off(&mut self) {
            // Dropping the session stops it: the capture thread has
            // released the device before the drop returns.
            self.session = None;
        }

        pub fn poll(&mut self) -> Option<Event> {
            let (_, frames) = self.session.as_ref()?;
            match frames.try_recv() {
                Ok(Message::Frame(f)) => Some(Event::CameraFrame {
                    width: f.width,
                    height: f.height,
                    luma: f.luma,
                    chroma: f.chroma,
                }),
                // No camera, the chosen one gone, the privacy setting,
                // or a device that failed; or the capture thread ended
                // with the channel too full to say so. Turn the camera
                // off so that a later `CameraOn` tries again.
                Ok(Message::Unavailable(why)) => {
                    eprintln!("camera: no device, Windows refused access, or the device failed");
                    self.off();
                    Some(event_for(why.map(failure)))
                }
                Err(TryRecvError::Disconnected) => {
                    eprintln!("camera: the capture thread ended without a word");
                    self.off();
                    Some(Event::CameraUnavailable)
                }
                Err(TryRecvError::Empty) => None,
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use other::Inner;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod other {
    use std::path::PathBuf;

    use faraday_shell_api::Event;

    use super::Unavailable;

    pub struct Inner;

    impl Inner {
        pub fn new(_device: Option<PathBuf>, _disabled: bool) -> Inner {
            Inner
        }

        pub fn on(&mut self) -> Result<(), Unavailable> {
            Err(Unavailable::plain(
                "this shell captures on Linux, macOS and Windows only",
            ))
        }

        pub fn off(&mut self) {}

        pub fn choose(&mut self, _path: PathBuf) {}

        pub fn poll(&mut self) -> Option<Event> {
            None
        }
    }
}
