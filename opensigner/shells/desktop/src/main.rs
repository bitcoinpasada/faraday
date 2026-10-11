//! The desktop shell (`docs/PLANNING.md` §4.1, §13.1): a fixed-size window
//! that forwards mouse, wheel, touch and keyboard input to [`OpenSigner`] as
//! shell events and blits its framebuffer.
//!
//! ```text
//! opensigner-desktop [--size WxH] [--dpi N] [--tier A|B|C|D] [--scale N] [--psbt FILE] [--out FILE]
//!                    [--settings FILE] [--no-settings]
//! ```
//!
//! `--size` and `--dpi` describe the display the core lays out for; a
//! 2.8" panel is `--size 480x640 --dpi 286`. `--scale` zooms the window by
//! an integer factor so that small panels are readable on a large monitor.
//! The window is not resizable. No platform code lives here: winit and
//! softbuffer own the platform.
//!
//! Sizing: the window asks for `W×scale` by `H×scale` *logical* points, so
//! it is the size the user asked for on screen whatever the display's scale
//! factor. Its physical surface is that times the scale factor (and
//! whatever the window manager actually granted), so every draw stretches
//! the `W×H` frame over the physical surface with nearest-neighbour
//! sampling, and pointer positions, which arrive in physical pixels, are
//! mapped back the same way. A fractional ratio (a 1.5× display, or a
//! window manager that ignores the size hints) is stretched, not
//! letterboxed: the frame always fills the surface.
//!
//! Drawing happens only in `RedrawRequested`; a [`Command::Draw`] asks for
//! one. A [`Event::Tick`] goes out every 50 ms whether or not there is
//! input: the event loop waits until the next tick, never for an event,
//! so the core's auto-lock and auto-wipe timers fire on an idle window.
//!
//! Entropy: a [`Command::RequestEntropy`] is answered at once with 32
//! bytes from the OS RNG (`getrandom`, `docs/deps/getrandom.md`). If the
//! OS refuses, the request goes unanswered and the core runs with a weak
//! session, which its Settings screen states.
//!
//! File channel: there is no file dialog and no GUI toolkit taken for
//! one (§16.16), so the core's Files screen is the picker. `--psbt`
//! names the file that answers the first `RequestFile` for a PSBT;
//! every other request is answered with a listing of `--files DIR`
//! (`$HOME/Downloads` when it exists, otherwise the current directory),
//! newest first, and the name the person taps comes back as `ReadFile`
//! and is read. A request this shell cannot answer with bytes is one
//! `Event::FileUnavailable`, which is what it means: there is no picker
//! here for a person to cancel, so this shell never sends
//! `Event::FileCancelled`. `WriteFile` goes to `--out`, or to the core's name hint
//! in the current directory, and the path is printed to stderr; the
//! write is answered `FileWritten` or, when it fails, `FileNotWritten`.
//!
//! Clipboard: the platform's own tool, not a crate (`docs/deps/clipboard.md`).
//! A `RequestClipboard` runs `wl-paste`, `xclip -o -selection clipboard`
//! or `pbpaste`, in that order, and a `WriteClipboard` runs `wl-copy`,
//! `xclip -selection clipboard` or `pbcopy`; the first that runs
//! answers, and a session with none of them is one
//! `Event::ClipboardUnavailable` or `Event::ClipboardNotWritten`, which
//! dims the row. Nothing is read from the clipboard the core did not
//! ask for.
//!
//! Settings (§6): the six settings the app keeps — network, unit,
//! auto-lock, auto-wipe, the PIN-pad shuffle and the camera rotation,
//! whose row this shell does not offer because a webcam is upright —
//! live in one small text file, read once at start and rewritten
//! whenever the user changes one. The default place is the platform's
//! own: `$XDG_CONFIG_HOME/opensigner/settings` (or
//! `$HOME/.config/opensigner/settings`) on Linux, and
//! `$HOME/Library/Application Support/OpenSigner/settings` on macOS.
//! `--settings FILE` names another file; `--no-settings` keeps nothing,
//! which is how a fresh device is reviewed. A write goes to a temporary
//! file beside the target and is renamed over it, so an interrupted
//! write leaves the old settings rather than half of the new ones.
//!
//! Camera (§16.33): on Linux a `CameraOn` opens the first `/dev/video*`
//! that streams video (or `--camera PATH`), asks it for 640 × 480 in
//! `GREY`, `YUYV`, `NV12` or `YU12`, and streams luma frames from a
//! capture thread; the event loop drains them on every pass, so a frame
//! reaches the core within a tick of capture and a slow decode drops
//! frames rather than queueing them. On macOS a `CameraOn` starts an
//! `AVCaptureSession` on the default video device, asking for the camera
//! permission the first time; the answer comes back asynchronously, so a
//! refusal is a `CameraUnavailable` from the frame channel rather than
//! an error at the request. The permission needs a bundle with an
//! `NSCameraUsageDescription`: `just mac-app` builds one. `--no-camera`
//! refuses everywhere, which is how the file fallback is reviewed on a
//! machine that has a webcam. No device, or an open that fails, is one
//! `CameraUnavailable` with the reason on stderr. On Windows there is no
//! capture yet.
//!
//! The capture itself is `opensigner-v4l2` on Linux and
//! `opensigner-avfoundation` on macOS, the two crates in the workspace
//! allowed to use `unsafe`; this shell has none of its own.

#![forbid(unsafe_code)]

mod camera;
mod clipboard;
mod files;

use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{Duration, Instant};

use camera::Camera;
use opensigner_core::{AssuranceTier, BuildInfo, OpenSigner};
use opensigner_scanner::Scanner;
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, FileKind, Frame, Key,
    SecureHardware, TouchPhase,
};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{
    ElementState, MouseButton, MouseScrollDelta, TouchPhase as WinitTouchPhase, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

/// Tick period. Holds and the passphrase mask animate on ticks.
const TICK: Duration = Duration::from_millis(50);
/// Frame pixels of scroll per wheel line.
const WHEEL_LINE_PX: f32 = 48.0;
/// Smallest framebuffer the core lays out for.
/// The smallest supported panel, in either orientation: 240 across the
/// short side and 320 along the long one (`docs/PLANNING.md` §4.3).
const MIN_SIZE: (u16, u16) = (240, 320);
/// Largest `--scale`.
const MAX_SCALE: u32 = 8;

struct Args {
    width: u16,
    height: u16,
    dpi: u16,
    tier: AssuranceTier,
    scale: u32,
    psbt: Option<PathBuf>,
    out: Option<PathBuf>,
    /// The directory the file channel lists.
    files: PathBuf,
    /// The camera to open, instead of searching `/dev/video*`.
    camera: Option<PathBuf>,
    /// Refuse every `CameraOn`, so the file fallback can be reviewed.
    no_camera: bool,
    /// The settings file, instead of the platform's own place.
    settings: Option<PathBuf>,
    /// Keep no settings at all: nothing is read and nothing is written.
    no_settings: bool,
}

const USAGE: &str = "usage: opensigner-desktop [--size WxH] [--dpi N] [--tier A|B|C|D] [--scale N] [--psbt FILE] [--out FILE]
                          [--files DIR] [--camera PATH] [--no-camera] [--settings FILE] [--no-settings]

  --size WxH    framebuffer size in pixels (default 960x640, minimum 240x320)
  --dpi N       pixel density the layout is computed for (default 160)
  --tier T      assurance tier shown in the app, A-D (default C)
  --scale N     integer window zoom, 1 to 8 (default 1)
  --psbt FILE   the file offered when the app asks for a PSBT (binary or base64)
  --out FILE    where the app's output file goes (default: its name hint, in the current directory)
  --files DIR   the directory the app lists when it asks for a file (default: $HOME/Downloads
                when it exists, otherwise the current directory)
  --camera PATH the capture device to use, Linux only (default: the first /dev/video*
                that streams video)
  --no-camera   answer every camera request \"unavailable\", so the scanner offers a file
  --settings FILE  where the settings the app keeps are read and written
                (default: $XDG_CONFIG_HOME/opensigner/settings on Linux,
                $HOME/Library/Application Support/OpenSigner/settings on macOS)
  --no-settings keep no settings: start with the defaults and write nothing
  --help        this text

The window is W*scale by H*scale points, so it has the same size on screen on every
display. On a high-DPI display the system maps points to more device pixels and the
frame is stretched over them (nearest neighbour), so one frame pixel covers several
device pixels. The alternative, a window of W*scale by H*scale device pixels, would be
pixel-exact but half the size on a 2x display; this shell does not offer it.

The camera works on Linux (V4L2) and macOS (AVFoundation); --camera names a device on
Linux only. On Windows, and wherever no camera is found, the scanner offers \"Load from
file instead\", and --no-camera makes that the case everywhere. On macOS the camera
permission needs an application bundle: `just mac-app` builds one.

The desktop window has one size: 960x640 at 160 dpi, the wide size class, which fits
any laptop. --size and --dpi are review flags for the other screens the app is built
for. Small panels are portrait: the 2.8\" reference is 480x640 at 286 dpi and the
smallest supported panel is 240x320 at 143 dpi. A phone: --size 1080x2340 --dpi 420.
A 240x320 panel on a large monitor: --size 240x320 --dpi 143 --scale 3.";

/// Parses the command line (without the program name). `Ok(None)` is
/// `--help`.
fn parse_args<I>(argv: I) -> Result<Option<Args>, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = Args {
        width: 960,
        height: 640,
        dpi: 160,
        tier: AssuranceTier::C,
        scale: 1,
        psbt: None,
        out: None,
        files: files::default_dir(),
        camera: None,
        no_camera: false,
        settings: None,
        no_settings: false,
    };
    let mut it = argv.into_iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--help" | "-h" => return Ok(None),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size wants WxH")?;
                args.width = w.parse().map_err(|_| "bad width")?;
                args.height = h.parse().map_err(|_| "bad height")?;
            }
            "--dpi" => args.dpi = value()?.parse().map_err(|_| "bad dpi")?,
            "--tier" => {
                args.tier = AssuranceTier::parse(&value()?).ok_or("--tier wants A, B, C or D")?;
            }
            "--scale" => args.scale = value()?.parse().map_err(|_| "bad scale")?,
            "--psbt" => args.psbt = Some(PathBuf::from(value()?)),
            "--out" => args.out = Some(PathBuf::from(value()?)),
            "--files" => args.files = PathBuf::from(value()?),
            "--camera" => args.camera = Some(PathBuf::from(value()?)),
            "--no-camera" => args.no_camera = true,
            "--settings" => args.settings = Some(PathBuf::from(value()?)),
            "--no-settings" => args.no_settings = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if args.width.min(args.height) < MIN_SIZE.0 || args.width.max(args.height) < MIN_SIZE.1 {
        return Err(format!(
            "minimum supported size is {}x{} in either orientation",
            MIN_SIZE.0, MIN_SIZE.1
        ));
    }
    if args.scale == 0 || args.scale > MAX_SCALE {
        return Err(format!("--scale must be 1 to {MAX_SCALE}"));
    }
    if args.dpi == 0 {
        return Err(String::from("--dpi must be positive"));
    }
    Ok(Some(args))
}

// Pure geometry, shared by the blit and the pointer mapping and unit-tested
// below. Both directions use the same rule: surface pixel `d` of `D` shows
// frame pixel `floor(d × F / D)`, which for an integer ratio is a plain
// pixel-replicating zoom and for any other ratio a nearest-neighbour
// stretch that always fills the surface.

/// Frame index shown at surface index `dst` of `dst_len`, for a frame axis
/// of `src_len`. Never out of range, even for `dst >= dst_len`.
fn source_index(dst: u32, dst_len: u32, src_len: u32) -> u32 {
    if dst_len == 0 || src_len == 0 {
        return 0;
    }
    // Fits: both factors are below 2^32, so the product is below 2^64.
    let i = u64::from(dst) * u64::from(src_len) / u64::from(dst_len);
    u32::try_from(i).unwrap_or(u32::MAX).min(src_len - 1)
}

/// Surface position (physical pixels, may be outside the window during a
/// drag) → frame pixel along one axis, clamped to the frame.
fn to_frame(pos: f64, surface_len: u32, frame_len: u16) -> u16 {
    if surface_len == 0 || frame_len == 0 {
        return 0;
    }
    let f = (pos * f64::from(frame_len) / f64::from(surface_len)).floor();
    // NaN clamps to NaN and casts to 0; infinities clamp to the edges.
    f.clamp(0.0, f64::from(frame_len - 1)) as u16
}

/// softbuffer's pixel format: `0000_0000 RRRR_RRRR GGGG_GGGG BBBB_BBBB`.
fn pack(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// Stretches `frame` over a `width × height` surface, nearest neighbour.
/// `out` is the surface's buffer in row-major order; rows beyond
/// `width × height` are left alone.
///
/// The frame is premultiplied RGBA and the surface has no alpha, so the
/// colour bytes are copied as they are: premultiplied colour is the pixel
/// composited over black, which for the core's opaque frames is the colour
/// itself.
fn blit(frame: &Frame<'_>, width: u32, height: u32, out: &mut [u32]) {
    let (fw, fh) = (u32::from(frame.width), u32::from(frame.height));
    let (w, h) = (width as usize, height as usize);
    let stride = fw as usize * 4;
    let rows = out.chunks_exact_mut(w.max(1)).take(h);
    if fw == 0 || fh == 0 || frame.rgba.len() < stride * fh as usize {
        // Nothing to show yet (before `Display`) or a frame that breaks
        // the contract: paint black rather than index out of range.
        for row in rows {
            row.fill(0);
        }
        return;
    }
    // Per-column source byte offsets, computed once per blit.
    let cols: Vec<usize> = (0..width)
        .map(|x| source_index(x, width, fw) as usize * 4)
        .collect();
    for (y, row) in rows.enumerate() {
        let sy = source_index(y as u32, height, fh) as usize;
        let src = &frame.rgba[sy * stride..(sy + 1) * stride];
        for (px, &sx) in row.iter_mut().zip(&cols) {
            let p = &src[sx..sx + 4];
            *px = pack(p[0], p[1], p[2]);
        }
    }
}

/// Where the settings live when `--settings` did not say: the place the
/// platform keeps application data. `None` when the environment names
/// no home directory, and then this shell keeps nothing.
fn default_settings_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = std::env::var_os("HOME")?;
        let mut path = PathBuf::from(home);
        path.push("Library/Application Support/OpenSigner/settings");
        return Some(path);
    }
    let dir = match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(dir.join("opensigner/settings"))
}

/// The file the settings are written to and read back from: `path.tmp`
/// is written first and renamed over it.
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".tmp");
    PathBuf::from(name)
}

/// 32 bytes from the OS RNG as an [`Event::Entropy`], or `None` if the
/// OS refused (the core is told nothing and stays weak).
fn entropy() -> Option<Event> {
    let mut bytes = [0u8; 32];
    match getrandom::fill(&mut bytes) {
        Ok(()) => Some(Event::Entropy(EntropyBytes::new(bytes))),
        Err(e) => {
            eprintln!("error: the OS gave no entropy: {e}");
            None
        }
    }
}

/// One line on stderr the first time the kernel refuses to pin a page
/// that holds a secret, and nothing after that. It is a machine
/// configuration a developer can act on (`RLIMIT_MEMLOCK`, `ulimit -l`)
/// and the person using the device cannot, so it goes here and onto no
/// screen. The app runs either way; the pages are simply not pinned.
fn report_unpinned_pages() {
    if osk_crypto::pin_failure_to_report() {
        eprintln!(
            "warning: the kernel would not lock secret pages into RAM \
             (RLIMIT_MEMLOCK); a seed can reach swap"
        );
    }
}

/// Whether a key held down repeats: Backspace deletes character after
/// character, and Tab and the arrows walk a list while they are held.
/// A letter held down does not type itself fifty times into a seed word.
fn repeats(key: Key) -> bool {
    matches!(
        key,
        Key::Backspace | Key::Tab | Key::BackTab | Key::Up | Key::Down | Key::Left | Key::Right
    )
}

/// A winit key → a shell key. `None` for keys the core has no use for.
fn map_key(key: &WinitKey) -> Option<Key> {
    match key {
        WinitKey::Named(NamedKey::Backspace) => Some(Key::Backspace),
        WinitKey::Named(NamedKey::Enter) => Some(Key::Enter),
        WinitKey::Named(NamedKey::Escape) => Some(Key::Escape),
        WinitKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
        WinitKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
        WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
        WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
        WinitKey::Named(NamedKey::Tab) => Some(Key::Tab),
        WinitKey::Named(NamedKey::Space) => Some(Key::Char(' ')),
        // The logical key already has the layout and shift applied, so a
        // shifted symbol arrives as itself. Dead keys and multi-character
        // strings are not single keys and are dropped.
        WinitKey::Character(s) => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if !c.is_control() => Some(Key::Char(c)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Turns wheel motion in frame pixels into whole-pixel [`Event::Scroll`]
/// amounts, carrying the fraction over so that slow touchpad scrolling is
/// not rounded away.
#[derive(Default)]
struct Wheel {
    remainder: f32,
}

impl Wheel {
    /// Adds `dy` frame pixels (positive = content moves up) and returns
    /// the whole pixels to send, if any.
    fn step(&mut self, dy: f32) -> Option<i16> {
        if !dy.is_finite() {
            return None;
        }
        let total = self.remainder + dy;
        let whole = total.trunc();
        self.remainder = total - whole;
        let whole = whole.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
        (whole != 0).then_some(whole)
    }
}

struct Shell {
    args: Args,
    /// `None` once the loop is exiting: the app is dropped there so that
    /// its zeroizing types run before the process ends.
    app: Option<OpenSigner>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    /// What the surface was last resized to.
    surface_size: Option<(NonZeroU32, NonZeroU32)>,
    epoch: Instant,
    next_tick: Instant,
    /// Last pointer position, in frame pixels.
    cursor: (u16, u16),
    /// A contact (left button or touch) is down.
    pressed: bool,
    /// Either Shift is held, which turns Tab into Back-Tab.
    shift: bool,
    wheel: Wheel,
    camera: Camera,
    /// Reads the codes in the camera's frames on its own thread, so a
    /// decode never stands between a frame and the next paint. Alive
    /// exactly while the camera is.
    scanner: Option<Scanner>,
}

impl Shell {
    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    fn surface_size(&self) -> PhysicalSize<u32> {
        self.window
            .as_ref()
            .map(|w| w.inner_size())
            .unwrap_or_default()
    }

    /// Physical window position → frame pixel.
    fn frame_pos(&self, x: f64, y: f64) -> (u16, u16) {
        let size = self.surface_size();
        (
            to_frame(x, size.width, self.args.width),
            to_frame(y, size.height, self.args.height),
        )
    }

    /// Delivers one event and acts on the commands it produced. A file
    /// request is answered with a further event, whose commands are
    /// handled in turn. A `Draw` becomes a redraw request; the blit itself
    /// waits for `RedrawRequested`.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let mut pending = VecDeque::from([event]);
        let mut draw = false;
        while let Some(e) = pending.pop_front() {
            let Some(app) = self.app.as_mut() else {
                return;
            };
            app.event(e);
            while let Some(c) = self.app.as_mut().and_then(|a| a.poll_command()) {
                match c {
                    Command::Draw => draw = true,
                    Command::Vibrate { .. } => {}
                    Command::Exit => event_loop.exit(),
                    Command::RequestFile { kind } => pending.push_back(self.list_files(kind)),
                    Command::ReadFile { kind, name } => {
                        pending.push_back(self.read_file(kind, &name));
                    }
                    Command::CameraOn => {
                        if let Err(reason) = self.camera.on() {
                            eprintln!("camera unavailable: {reason}");
                            pending.push_back(Event::CameraUnavailable);
                        } else if self.scanner.is_none() {
                            self.scanner = Some(Scanner::new());
                        }
                    }
                    Command::CameraOff => {
                        self.camera.off();
                        // Dropping it ends the worker and wipes the
                        // frame and any payload it still held.
                        self.scanner = None;
                    }
                    Command::RequestEntropy => {
                        if let Some(e) = entropy() {
                            pending.push_back(e);
                        }
                    }
                    Command::WriteFile {
                        kind,
                        name_hint,
                        bytes,
                    } => pending.push_back(self.write_file(kind, &name_hint, &bytes)),
                    Command::RequestClipboard { kind } => {
                        pending.push_back(match clipboard::read() {
                            Some(text) => Event::Clipboard { kind, text },
                            None => Event::ClipboardUnavailable { kind },
                        });
                    }
                    Command::WriteClipboard { kind, text } => {
                        pending.push_back(if clipboard::write(&text) {
                            Event::ClipboardWritten { kind }
                        } else {
                            Event::ClipboardNotWritten { kind }
                        });
                    }
                    Command::StoreSettings { bytes } => self.write_settings(&bytes),
                    // No secure element and nowhere to keep a blob:
                    // the answer is "unavailable" to everything on the
                    // kept-secret channel, so the core offers no
                    // stored key.
                    Command::SecureMac { .. } => pending.push_back(Event::SecureUnavailable),
                    Command::StoreSecret { .. } => pending.push_back(Event::SecretNotStored),
                    Command::LoadSecret => pending.push_back(Event::SecretUnavailable),
                    Command::ForgetSecret => pending.push_back(Event::SecretForgotten),
                }
            }
        }
        if draw && let Some(w) = &self.window {
            w.request_redraw();
        }
        report_unpinned_pages();
    }

    /// The `--psbt` file answers the first request for a PSBT, as a
    /// shell with a picker of its own answers one; every other request
    /// is answered with what `--files` holds, and the core shows it.
    fn list_files(&mut self, kind: FileKind) -> Event {
        if kind == FileKind::Psbt
            && let Some(path) = self.args.psbt.take()
        {
            return match std::fs::read(&path) {
                Ok(bytes) => Event::File { kind, bytes },
                Err(e) => {
                    eprintln!("error: cannot read {}: {e}", path.display());
                    Event::FileUnavailable { kind }
                }
            };
        }
        match files::list(&self.args.files, kind) {
            Ok(entries) => Event::FileList {
                kind,
                entries,
                // The directory's own name, which is what a person
                // looking for it in a file manager reads.
                place: self
                    .args
                    .files
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned()),
            },
            Err(reason) => {
                eprintln!("error: {reason}");
                Event::FileUnavailable { kind }
            }
        }
    }

    /// One of the names the listing carried, which the person chose on
    /// the core's Files screen.
    fn read_file(&mut self, kind: FileKind, name: &str) -> Event {
        match files::read(&self.args.files, kind, name) {
            Ok(bytes) => Event::File { kind, bytes },
            Err(reason) => {
                eprintln!("error: {reason}");
                Event::FileUnavailable { kind }
            }
        }
    }

    fn write_file(&self, kind: FileKind, name_hint: &str, bytes: &[u8]) -> Event {
        let mut path = self
            .args
            .out
            .clone()
            .unwrap_or_else(|| PathBuf::from(name_hint));
        // A picture is saved under a name that says it is one, whatever
        // `--out` was given as.
        if kind == FileKind::Png
            && !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
        {
            path.set_extension("png");
        }
        match std::fs::write(&path, bytes) {
            Ok(()) => {
                eprintln!("wrote {}", path.display());
                Event::FileWritten { kind }
            }
            Err(e) => {
                eprintln!("error: cannot write {}: {e}", path.display());
                Event::FileNotWritten { kind }
            }
        }
    }

    /// The settings file, or `None` for `--no-settings` and for an
    /// environment with no home directory to keep one in.
    fn settings_path(&self) -> Option<PathBuf> {
        if self.args.no_settings {
            return None;
        }
        self.args.settings.clone().or_else(default_settings_path)
    }

    /// What was kept last time, as one [`Event::Settings`]. Nothing kept
    /// is no event at all, which is a device with its defaults.
    fn read_settings(&self) -> Option<Event> {
        let path = self.settings_path()?;
        match std::fs::read(&path) {
            Ok(bytes) => Some(Event::Settings { bytes }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", path.display());
                None
            }
        }
    }

    /// Keeps `bytes` for the next run: the directory is made, the
    /// temporary file is written, and the rename puts it in place in one
    /// step.
    fn write_settings(&self, bytes: &[u8]) {
        let Some(path) = self.settings_path() else {
            return;
        };
        let temp = temp_path(&path);
        let made = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => std::fs::create_dir_all(dir),
            _ => Ok(()),
        };
        let written = made
            .and_then(|()| std::fs::write(&temp, bytes))
            .and_then(|()| std::fs::rename(&temp, &path));
        if let Err(e) = written {
            eprintln!("error: cannot write {}: {e}", path.display());
        }
    }

    fn touch(&mut self, event_loop: &ActiveEventLoop, phase: TouchPhase) {
        let (x, y) = self.cursor;
        self.send(event_loop, Event::Touch { x, y, phase });
    }

    /// Moves the pointer to a physical position; a `Move` goes out while a
    /// contact is down and the frame pixel changed.
    fn pointer_moved(&mut self, event_loop: &ActiveEventLoop, x: f64, y: f64) {
        let p = self.frame_pos(x, y);
        let moved = p != self.cursor;
        self.cursor = p;
        if self.pressed && moved {
            self.touch(event_loop, TouchPhase::Move);
        }
    }

    fn pointer_down(&mut self, event_loop: &ActiveEventLoop) {
        self.pressed = true;
        self.touch(event_loop, TouchPhase::Down);
    }

    fn pointer_up(&mut self, event_loop: &ActiveEventLoop) {
        if self.pressed {
            self.pressed = false;
            self.touch(event_loop, TouchPhase::Up);
        }
    }

    /// Paints the current frame over the whole window. Called only from
    /// `RedrawRequested`, and always paints: the window system may need
    /// the surface refilled even when the frame has not changed.
    fn draw(&mut self) {
        let (Some(window), Some(surface), Some(app)) =
            (&self.window, self.surface.as_mut(), self.app.as_mut())
        else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            // Minimised or not yet mapped: there is nothing to paint on.
            return;
        };
        if self.surface_size != Some((w, h)) {
            if let Err(e) = surface.resize(w, h) {
                eprintln!("error: cannot size the drawing surface to {w}x{h}: {e}");
                return;
            }
            self.surface_size = Some((w, h));
        }
        let mut buffer = match surface.buffer_mut() {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error: cannot get the drawing buffer: {e}");
                return;
            }
        };
        blit(&app.frame(), size.width, size.height, &mut buffer);
        if let Err(e) = buffer.present() {
            eprintln!("error: cannot present the frame: {e}");
        }
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() || self.app.is_none() {
            return;
        }
        let size = LogicalSize::new(
            f64::from(self.args.width) * f64::from(self.args.scale),
            f64::from(self.args.height) * f64::from(self.args.scale),
        );
        let attributes = Window::default_attributes()
            .with_title("OpenSigner")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(size)
            .with_resizable(false);
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("error: cannot create a window: {e}");
                event_loop.exit();
                return;
            }
        };
        let context = match Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: cannot create a drawing context: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot create a drawing surface: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window);
        self.surface = Some(surface);
        self.send(
            event_loop,
            Event::Display(DisplayInfo {
                width: self.args.width,
                height: self.args.height,
                dpi: self.args.dpi,
                // A window has no system edges of its own: the whole
                // client area is usable.
                inset_bottom: 0,
                inset_top: 0,
                buttons: 0,
                // A laptop's or a desk's webcam sits upright.
                camera_fixed: true,
                secure: SecureHardware::None,
                // A desktop reports nothing about how the machine booted.
                boot: BootState::Unknown,
                memory_mib: total_memory_mib(),
            }),
        );
        // What was kept last time, before any input can change it.
        if let Some(event) = self.read_settings() {
            self.send(event_loop, event);
        }
        // The first frame must appear whether or not `Display` drew.
        if let Some(w) = &self.window {
            w.request_redraw();
        }
        self.next_tick = Instant::now() + TICK;
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.draw(),
            // The surface is sized from the window at the next draw; the
            // scale factor only matters through the physical size.
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_moved(event_loop, position.x, position.y);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => self.pointer_down(event_loop),
                ElementState::Released => self.pointer_up(event_loop),
            },
            WindowEvent::Touch(t) => {
                self.pointer_moved(event_loop, t.location.x, t.location.y);
                match t.phase {
                    WinitTouchPhase::Started => self.pointer_down(event_loop),
                    WinitTouchPhase::Moved => {}
                    WinitTouchPhase::Ended | WinitTouchPhase::Cancelled => {
                        self.pointer_up(event_loop);
                    }
                }
            }
            // A release that happens while another window has focus never
            // arrives; end the contact so that a hold cannot stick.
            WindowEvent::Focused(false) => self.pointer_up(event_loop),
            WindowEvent::MouseWheel { delta, .. } => {
                // winit: positive = content moves down. The core: positive
                // = content moves up. Pixel deltas are physical pixels.
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => -lines * WHEEL_LINE_PX,
                    MouseScrollDelta::PixelDelta(p) => {
                        let size = self.surface_size();
                        let per_px = if size.height == 0 {
                            1.0
                        } else {
                            f64::from(self.args.height) / f64::from(size.height)
                        };
                        -(p.y * per_px) as f32
                    }
                };
                if let Some(dy) = self.wheel.step(dy) {
                    let (x, y) = self.cursor;
                    self.send(event_loop, Event::Scroll { x, y, dy });
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.shift = mods.state().shift_key();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let Some(key) = map_key(&event.logical_key) else {
                    return;
                };
                let key = match key {
                    Key::Tab if self.shift => Key::BackTab,
                    other => other,
                };
                if event.state != ElementState::Pressed {
                    self.send(event_loop, Event::KeyUp(key));
                    return;
                }
                // Auto-repeat deletes and walks a list; it does not type.
                if event.repeat && !repeats(key) {
                    return;
                }
                self.send(event_loop, Event::Key(key));
            }
            // IME composition, other buttons, focus gain, and the rest.
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() || self.app.is_none() {
            return;
        }
        // Camera frames are taken here rather than posted as user
        // events: the loop already wakes every tick, so a frame waits at
        // most one tick. One frame per pass, the newest: a decode can
        // take longer than the gap between frames, and a loop that
        // drained the channel until it was empty would never get back
        // to the ticks and the input while the camera was on.
        let mut newest = None;
        let mut answers = Vec::new();
        while let Some(event) = self.camera.poll() {
            match event {
                Event::CameraFrame { .. } => newest = Some(event),
                // A camera that was refused, which is not a frame and
                // is not dropped for a newer one.
                other => answers.push(other),
            }
        }
        for answer in answers {
            self.scanner = None;
            self.send(event_loop, answer);
        }
        if let Some(Event::CameraFrame {
            width,
            height,
            luma,
            chroma,
        }) = newest
        {
            // The frame goes twice: to the core, which draws it, and to
            // the scanner, which reads it. One copy, since the core
            // takes the original.
            if let Some(scanner) = self.scanner.as_ref() {
                scanner.offer(width, height, luma.clone());
            }
            self.send(
                event_loop,
                Event::CameraFrame {
                    width,
                    height,
                    luma,
                    chroma,
                },
            );
        }
        // Whatever the worker read since the last pass.
        while let Some(bytes) = self.scanner.as_ref().and_then(Scanner::poll) {
            self.send(event_loop, Event::Scanned { bytes });
        }
        let now = Instant::now();
        if now >= self.next_tick {
            let now_ms = self.now_ms();
            self.send(event_loop, Event::Tick { now_ms });
            self.next_tick = now + TICK;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // The capture thread first: joining it closes the device.
        self.camera.off();
        self.scanner = None;
        // Surface before window, app last: the app's zeroizing types run
        // here, before the event loop tears down.
        self.surface = None;
        self.surface_size = None;
        self.window = None;
        self.app = None;
    }
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(Some(a)) => a,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let app = OpenSigner::new(
        args.tier,
        BuildInfo {
            version: env!("CARGO_PKG_VERSION"),
            core_hash: None,
        },
    );
    let event_loop = match EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error: cannot start the event loop: {e}");
            return ExitCode::FAILURE;
        }
    };
    let epoch = Instant::now();
    let camera = Camera::new(args.camera.clone(), args.no_camera);
    let mut shell = Shell {
        args,
        app: Some(app),
        window: None,
        surface: None,
        surface_size: None,
        epoch,
        next_tick: epoch,
        cursor: (0, 0),
        pressed: false,
        shift: false,
        wheel: Wheel::default(),
        camera,
        scanner: None,
    };
    match event_loop.run_app(&mut shell) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// This machine's total memory in MiB, read from `/proc/meminfo`, or
/// `None` where the file is not there or does not say. The core uses it
/// only to recommend an Argon2id memory cost for an encrypted backup
/// (`docs/PLANNING.md` §16.112).
fn total_memory_mib() -> Option<u32> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    u32::try_from(kib / 1024).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::keyboard::SmolStr;

    fn argv(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn args_defaults() {
        let a = parse_args(argv("")).unwrap().unwrap();
        assert_eq!((a.width, a.height, a.dpi, a.scale), (960, 640, 160, 1));
        assert_eq!(a.tier, AssuranceTier::C);
        assert!(a.psbt.is_none() && a.out.is_none());
        // The camera is searched for, not named, and is on by default.
        assert!(a.camera.is_none() && !a.no_camera);
        // The settings go to the platform's own place, and are kept.
        assert!(a.settings.is_none() && !a.no_settings);
    }

    #[test]
    fn args_all_flags() {
        let a = parse_args(argv(
            "--size 240x320 --dpi 143 --tier A --scale 3 --psbt in.psbt --out signed.psbt \
             --camera /dev/video9 --no-camera --settings prefs --no-settings",
        ))
        .unwrap()
        .unwrap();
        assert_eq!((a.width, a.height, a.dpi, a.scale), (240, 320, 143, 3));
        assert_eq!(a.tier, AssuranceTier::A);
        assert_eq!(a.psbt.as_deref(), Some(std::path::Path::new("in.psbt")));
        assert_eq!(a.out.as_deref(), Some(std::path::Path::new("signed.psbt")));
        assert_eq!(
            a.camera.as_deref(),
            Some(std::path::Path::new("/dev/video9"))
        );
        assert!(a.no_camera);
        assert_eq!(a.settings.as_deref(), Some(std::path::Path::new("prefs")));
        assert!(a.no_settings);
    }

    #[test]
    fn args_help_and_errors() {
        assert!(parse_args(argv("--help")).unwrap().is_none());
        assert!(parse_args(argv("-h")).unwrap().is_none());
        assert!(parse_args(argv("--size")).is_err());
        assert!(parse_args(argv("--size 640")).is_err());
        assert!(parse_args(argv("--size 300x240")).is_err());
        assert!(parse_args(argv("--size 640x200")).is_err());
        assert!(parse_args(argv("--scale 0")).is_err());
        assert!(parse_args(argv("--scale 9")).is_err());
        assert!(parse_args(argv("--dpi 0")).is_err());
        assert!(parse_args(argv("--tier E")).is_err());
        assert!(parse_args(argv("--camera")).is_err());
        assert!(parse_args(argv("--settings")).is_err());
        assert!(parse_args(argv("--bogus")).is_err());
    }

    #[test]
    fn pointer_and_blit_agree() {
        // The pixel under the pointer is the pixel the blit put there.
        for surface in [640u32, 960, 1280, 1000] {
            for x in 0..surface {
                assert_eq!(
                    u32::from(to_frame(f64::from(x), surface, 640)),
                    source_index(x, surface, 640)
                );
            }
        }
    }

    #[test]
    fn blit_before_display_paints_black() {
        let f = Frame {
            width: 0,
            height: 0,
            rgba: &[],
        };
        let mut out = vec![7u32; 4];
        blit(&f, 2, 2, &mut out);
        assert_eq!(out, [0; 4]);
        // A short buffer is a contract breach, not a panic.
        let short = [0u8; 4];
        let f = Frame {
            width: 2,
            height: 2,
            rgba: &short,
        };
        blit(&f, 2, 2, &mut out);
        assert_eq!(out, [0; 4]);
    }

    #[test]
    fn wheel_accumulates_fractions() {
        let mut w = Wheel::default();
        assert_eq!(w.step(0.4), None);
        assert_eq!(w.step(0.4), None);
        assert_eq!(w.step(0.4), Some(1));
        // The 0.2 left over carries into the next step.
        assert_eq!(w.step(-48.0), Some(-47));
        let mut w = Wheel::default();
        assert_eq!(w.step(-48.0), Some(-48));
        assert_eq!(w.step(f32::NAN), None);
        assert_eq!(w.step(1e9), Some(i16::MAX));
        assert_eq!(w.step(-1e9), Some(i16::MIN));
    }

    #[test]
    fn keys_map() {
        assert_eq!(
            map_key(&WinitKey::Named(NamedKey::Backspace)),
            Some(Key::Backspace)
        );
        assert_eq!(map_key(&WinitKey::Named(NamedKey::Enter)), Some(Key::Enter));
        assert_eq!(
            map_key(&WinitKey::Named(NamedKey::Escape)),
            Some(Key::Escape)
        );
        assert_eq!(map_key(&WinitKey::Named(NamedKey::ArrowUp)), Some(Key::Up));
        assert_eq!(
            map_key(&WinitKey::Named(NamedKey::ArrowDown)),
            Some(Key::Down)
        );
        assert_eq!(
            map_key(&WinitKey::Named(NamedKey::ArrowLeft)),
            Some(Key::Left)
        );
        assert_eq!(
            map_key(&WinitKey::Named(NamedKey::ArrowRight)),
            Some(Key::Right)
        );
        assert_eq!(map_key(&WinitKey::Named(NamedKey::Tab)), Some(Key::Tab));
        assert_eq!(
            map_key(&WinitKey::Named(NamedKey::Space)),
            Some(Key::Char(' '))
        );
        assert_eq!(map_key(&WinitKey::Named(NamedKey::Shift)), None);
        assert_eq!(
            map_key(&WinitKey::Character(SmolStr::new("a"))),
            Some(Key::Char('a'))
        );
        assert_eq!(
            map_key(&WinitKey::Character(SmolStr::new("!"))),
            Some(Key::Char('!'))
        );
        assert_eq!(
            map_key(&WinitKey::Character(SmolStr::new("é"))),
            Some(Key::Char('é'))
        );
        assert_eq!(map_key(&WinitKey::Character(SmolStr::new("ab"))), None);
        assert_eq!(map_key(&WinitKey::Character(SmolStr::new(""))), None);
        assert_eq!(map_key(&WinitKey::Character(SmolStr::new("\u{7}"))), None);
    }
}
