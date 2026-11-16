//! The camera channel, which exists on Linux and Windows.
//!
//! On Linux `faraday-v4l2` finds a `/dev/video*`, streams it on its
//! own thread and posts frames to a bounded channel; the event loop
//! drains that channel on every pass, so a frame reaches the core within
//! a tick of capture and a slow decode drops frames instead of backing
//! up. On Windows `faraday-mediafoundation` reads a Media
//! Foundation source reader on its own thread and posts to the channel
//! from there; the scan sheet's camera choice lists Media Foundation's
//! devices by name. Anywhere else every `CameraOn` is refused and the
//! app offers its file fallback.
//!
//! Windows' privacy setting is only known once the device is opened on
//! the capture thread, so a refusal there arrives as a message on the
//! frame channel and not as an error from `on`. That is why `on`
//! returning `Ok` is not a promise of frames, on any platform.
//!
//! Every platform presents the same three calls, so the command match in
//! the shells (the desktop app's and the Starter's) has no `#[cfg]` in it.

use std::path::PathBuf;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use std::path::Path;

#[cfg(any(target_os = "linux", target_os = "windows"))]
use faraday_camera::Frame;
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
    /// a `/dev/videoN`, on Windows a device's symbolic link.
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
/// name Windows shows (`windows::list`).
#[cfg(not(target_os = "windows"))]
pub fn list() -> Vec<(String, String)> {
    faraday_camera::list()
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
/// What differs between the platforms: how a capture starts and stops
/// and how it is read. Everything else about the camera (the
/// `--no-camera` refusal, starting twice, choosing a device, turning the
/// camera off once it has ended) is [`Shared`].
trait Backend {
    /// A running capture and the channel its frames arrive on.
    type Session;

    /// Starts capturing on `device`, or on the first camera when that is
    /// `None`.
    fn start(device: Option<&Path>) -> Result<Self::Session, Unavailable>;

    /// Ends the capture and waits until the device is closed.
    fn stop(session: Self::Session);

    /// The next thing the capture has to say.
    fn poll(session: &mut Self::Session) -> Polled;
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
/// What a capture had for one [`Backend::poll`].
enum Polled {
    /// Nothing is waiting.
    Nothing,
    /// An event for the app; the capture goes on.
    Event(Event),
    /// The capture is over: turn the camera off, so that a later
    /// `CameraOn` tries again, and tell the app.
    Ended(Event),
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
/// The camera, written once over a platform's [`Backend`].
struct Shared<B: Backend> {
    device: Option<PathBuf>,
    disabled: bool,
    session: Option<B::Session>,
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
impl<B: Backend> Shared<B> {
    fn new(device: Option<PathBuf>, disabled: bool) -> Self {
        Shared {
            device,
            disabled,
            session: None,
        }
    }

    fn on(&mut self) -> Result<(), Unavailable> {
        if self.disabled {
            return Err(Unavailable::plain("--no-camera"));
        }
        if self.session.is_some() {
            return Ok(());
        }
        self.session = Some(B::start(self.device.as_deref())?);
        Ok(())
    }

    fn choose(&mut self, path: PathBuf) {
        let running = self.session.is_some();
        self.device = Some(path);
        if running {
            self.off();
            if let Err(reason) = self.on() {
                eprintln!("faraday: camera unavailable: {reason}");
            }
        }
    }

    fn off(&mut self) {
        if let Some(session) = self.session.take() {
            B::stop(session);
        }
    }

    fn poll(&mut self) -> Option<Event> {
        match B::poll(self.session.as_mut()?) {
            Polled::Nothing => None,
            Polled::Event(event) => Some(event),
            Polled::Ended(event) => {
                self.off();
                Some(event)
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
/// A frame as the event that carries it to the core.
fn frame_event(mut f: Frame) -> Event {
    Event::CameraFrame {
        width: f.width,
        height: f.height,
        // Taken out of the wiping wrapper, not copied: the bytes go on
        // to the core, which wipes them.
        luma: std::mem::take(&mut *f.luma),
        chroma: f.chroma.as_mut().map(|uv| std::mem::take(&mut **uv)),
    }
}

/// Frames the channel holds. Two is one being decoded and one waiting; a
/// third would already be out of date by the time the core reached it.
#[cfg(any(target_os = "linux", target_os = "windows"))]
const QUEUE: usize = 2;

#[cfg(target_os = "linux")]
type Inner = Shared<linux::Platform>;

#[cfg(target_os = "linux")]
mod linux {
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
    use std::thread::JoinHandle;

    use faraday_shell_api::{CameraFailure, Event};
    use faraday_v4l2::{Camera as Device, Failure, Frame, MAX_FPS, Size};

    use super::{Backend, Polled, QUEUE, Unavailable, frame_event};

    /// The app's reason for a V4L2 one.
    fn reason(why: Failure) -> CameraFailure {
        match why {
            Failure::Refused => CameraFailure::Refused,
            Failure::InUse => CameraFailure::InUse,
            Failure::NotFound => CameraFailure::NotFound,
        }
    }

    /// A running capture: the thread, its stop flag, and the frames.
    pub struct Session {
        frames: Receiver<Frame>,
        stop: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    }

    pub struct Platform;

    impl Backend for Platform {
        type Session = Session;

        fn start(device: Option<&Path>) -> Result<Session, Unavailable> {
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
            Ok(Session {
                frames,
                stop,
                thread,
            })
        }

        fn stop(session: Session) {
            session.stop.store(true, Ordering::Relaxed);
            // The camera is dropped on that thread, so the join is what
            // closes the device.
            let _ = session.thread.join();
        }

        fn poll(session: &mut Session) -> Polled {
            match session.frames.try_recv() {
                Ok(f) => Polled::Event(frame_event(f)),
                Err(TryRecvError::Empty) => Polled::Nothing,
                // The capture thread ended on its own: the device failed
                // or is gone.
                Err(TryRecvError::Disconnected) => {
                    eprintln!("camera: the device stopped delivering frames");
                    Polled::Ended(Event::CameraUnavailable)
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
type Inner = Shared<windows::Platform>;
#[cfg(target_os = "windows")]
pub use windows::list;

#[cfg(target_os = "windows")]
mod windows {
    use std::path::Path;
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};

    use faraday_mediafoundation::{Camera as Capture, MAX_FPS, Message};
    use faraday_shell_api::{CameraFailure, Event};

    use super::{Backend, Polled, QUEUE, Unavailable, event_for, frame_event};

    /// The app's reason for the backend's.
    fn reason(why: faraday_camera::Failure) -> CameraFailure {
        match why {
            faraday_camera::Failure::Refused => CameraFailure::Refused,
            faraday_camera::Failure::InUse => CameraFailure::InUse,
            faraday_camera::Failure::NotFound => CameraFailure::NotFound,
        }
    }

    /// The cameras this machine has, as `(symbolic link, name)`, for the
    /// scan sheet's choice, in Media Foundation's order: the first is
    /// the one opened when none was chosen.
    pub fn list() -> Vec<(String, String)> {
        faraday_mediafoundation::devices()
            .into_iter()
            .map(|d| (d.link, d.name))
            .collect()
    }

    pub struct Platform;

    impl Backend for Platform {
        type Session = (Capture, Receiver<Message>);

        /// `device` is the symbolic link of the camera to open.
        fn start(device: Option<&Path>) -> Result<Self::Session, Unavailable> {
            let link = device.map(|p| p.to_string_lossy().into_owned());
            let (tx, frames) = sync_channel(QUEUE);
            // Starting is asynchronous: the device, the privacy setting
            // and the format all answer on the channel. A failure
            // arrives as `Unavailable`, never as an error here.
            let capture = Capture::start(link.as_deref(), tx);
            eprintln!(
                "camera: Media Foundation, {}, up to {MAX_FPS} frames a second",
                link.as_deref().unwrap_or("the first device")
            );
            Ok((capture, frames))
        }

        fn stop(session: Self::Session) {
            // Dropping the capture stops it: the capture thread has
            // released the device before the drop returns.
            drop(session);
        }

        fn poll((_, frames): &mut Self::Session) -> Polled {
            match frames.try_recv() {
                Ok(Message::Frame(f)) => Polled::Event(frame_event(f)),
                // No camera, the chosen one gone, the privacy setting,
                // or a device that failed; or the capture thread ended
                // with the channel too full to say so.
                Ok(Message::Unavailable(why)) => {
                    eprintln!("camera: no device, Windows refused access, or the device failed");
                    Polled::Ended(event_for(why.map(reason)))
                }
                Err(TryRecvError::Disconnected) => {
                    eprintln!("camera: the capture thread ended without a word");
                    Polled::Ended(Event::CameraUnavailable)
                }
                Err(TryRecvError::Empty) => Polled::Nothing,
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use other::Inner;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
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
                "this shell captures on Linux and Windows only",
            ))
        }

        pub fn off(&mut self) {}

        pub fn choose(&mut self, _path: PathBuf) {}

        pub fn poll(&mut self) -> Option<Event> {
            None
        }
    }
}
