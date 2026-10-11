//! The camera channel, which exists on Linux and macOS.
//!
//! On Linux `opensigner-v4l2` finds a `/dev/video*`, streams it on its
//! own thread and posts frames to a bounded channel; the event loop
//! drains that channel on every pass, so a frame reaches the core within
//! a tick of capture and a slow decode drops frames instead of backing
//! up. On macOS `opensigner-avfoundation` does the same through an
//! `AVCaptureSession`, which posts to the channel from AVFoundation's
//! own queue. On Windows there is no capture yet (`docs/PLANNING.md`
//! §16.33 has Media Foundation deferred), so every `CameraOn` is refused
//! and the app offers its file fallback.
//!
//! macOS answers the permission prompt when it pleases rather than at
//! `CameraOn`, so a refusal arrives as a message on the frame channel
//! and not as an error from `on`. That is why `on` returning `Ok` is not
//! a promise of frames, on any platform.
//!
//! Every platform presents the same three calls, so the command match in
//! `main.rs` has no `#[cfg]` in it.

use std::path::PathBuf;

use osk_shell_api::Event;

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
    /// no-op, as the contract requires. The error is a reason to print,
    /// and the caller answers `CameraUnavailable` once.
    pub fn on(&mut self) -> Result<(), String> {
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
    /// sheet; a camera already running is restarted on it. Linux only:
    /// elsewhere the system's own camera is used.
    pub fn choose(&mut self, path: PathBuf) {
        self.inner.choose(path);
    }
}

/// The cameras this machine has, as `(path, name)`, for the scan sheet's
/// choice: every `/dev/videoN` whose sysfs node is its device's first
/// (`index` 0), which tells a webcam's capture node from its metadata
/// node without opening either; the name is the one the device gives.
/// The stick shell lists them the same way. Empty off Linux.
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
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};
    use std::thread::JoinHandle;

    use opensigner_v4l2::{Camera as Device, Frame, MAX_FPS, Size};
    use osk_shell_api::Event;

    /// Frames the channel holds. Two is one being decoded and one
    /// waiting; a third would already be out of date by the time the
    /// core reached it.
    const QUEUE: usize = 2;

    /// A running capture: the thread, its stop flag, and the frames.
    struct Session {
        frames: Receiver<Frame>,
        stop: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    }

    pub struct Inner {
        device: Option<PathBuf>,
        disabled: bool,
        session: Option<Session>,
    }

    impl Inner {
        pub fn new(device: Option<PathBuf>, disabled: bool) -> Inner {
            Inner {
                device,
                disabled,
                session: None,
            }
        }

        pub fn on(&mut self) -> Result<(), String> {
            if self.disabled {
                return Err(String::from("--no-camera"));
            }
            if self.session.is_some() {
                return Ok(());
            }
            let path = match &self.device {
                Some(p) => p.clone(),
                None => opensigner_v4l2::find()
                    .ok_or_else(|| String::from("no /dev/video* is a streaming capture device"))?,
            };
            let device = Device::open(&path, Size::VGA)
                .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
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
            let thread = opensigner_v4l2::spawn(device, tx, Arc::clone(&stop));
            self.session = Some(Session {
                frames,
                stop,
                thread,
            });
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
            let session = self.session.as_ref()?;
            match session.frames.try_recv() {
                Ok(f) => Some(Event::CameraFrame {
                    width: f.width,
                    height: f.height,
                    luma: f.luma,
                    chroma: f.chroma,
                }),
                Err(TryRecvError::Empty) => None,
                // The capture thread ended on its own: the device is
                // gone. Tidy up so a later CameraOn tries again.
                Err(TryRecvError::Disconnected) => {
                    eprintln!("camera: the device stopped delivering frames");
                    self.off();
                    None
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
use macos::Inner;

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;
    use std::sync::mpsc::{Receiver, TryRecvError, sync_channel};

    use opensigner_avfoundation::{Camera as Session, MAX_FPS, Message};
    use osk_shell_api::Event;

    /// Frames the channel holds, as on Linux: one being decoded and one
    /// waiting. A third would be out of date by the time the core
    /// reached it, and the capture callback drops it instead.
    const QUEUE: usize = 2;

    pub struct Inner {
        disabled: bool,
        session: Option<(Session, Receiver<Message>)>,
    }

    impl Inner {
        pub fn new(device: Option<PathBuf>, disabled: bool) -> Inner {
            if device.is_some() {
                eprintln!("camera: --camera names a V4L2 device and does nothing on macOS");
            }
            Inner {
                disabled,
                session: None,
            }
        }

        pub fn on(&mut self) -> Result<(), String> {
            if self.disabled {
                return Err(String::from("--no-camera"));
            }
            if self.session.is_some() {
                return Ok(());
            }
            let (tx, frames) = sync_channel(QUEUE);
            // Starting is asynchronous: the permission prompt, the
            // device and the session all answer on the channel. A
            // failure arrives as `Unavailable`, never as an error here.
            let session = Session::start(tx);
            eprintln!("camera: the default AVFoundation device, up to {MAX_FPS} frames a second");
            self.session = Some((session, frames));
            Ok(())
        }

        pub fn choose(&mut self, _path: PathBuf) {}

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
                Ok(Message::Unavailable) => {
                    eprintln!("camera: no device, or macOS refused access");
                    self.off();
                    Some(Event::CameraUnavailable)
                }
                Err(TryRecvError::Empty) => None,
                // The sender lives as long as the session, so this is
                // unreachable while one is on; treat it as no frame.
                Err(TryRecvError::Disconnected) => None,
            }
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use other::Inner;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod other {
    use std::path::PathBuf;

    use osk_shell_api::Event;

    pub struct Inner;

    impl Inner {
        pub fn new(_device: Option<PathBuf>, _disabled: bool) -> Inner {
            Inner
        }

        pub fn on(&mut self) -> Result<(), String> {
            Err(String::from("this shell captures on Linux and macOS only"))
        }

        pub fn off(&mut self) {}

        pub fn choose(&mut self, _path: PathBuf) {}

        pub fn poll(&mut self) -> Option<Event> {
            None
        }
    }
}
