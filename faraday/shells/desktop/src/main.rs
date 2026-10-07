//! Faraday in a window.
//!
//! ```text
//! faraday [--sticks DIR] [--size WxH] [--full-kit]
//! ```
//!
//! The window draws at the display's own resolution and can be resized;
//! the app lays itself out for whatever size it is given.
//!
//! Sticks are folders. `--sticks DIR` (default `$HOME/faraday-sticks`)
//! holds `TESTSTICK`, which gets every test kit file it lacks at each
//! start and never has a file overwritten, and `BLANK`, which starts empty. Neither is
//! attached at start. The test stick is the backup test stick
//! (`faraday_core::testkit::backup_files`: a 2-of-3 Taproot multisig's
//! public backup files and a vault, passphrase `a`, holding its three
//! seeds); `--full-kit` makes it the full test kit instead. A folder made
//! for the other kit is moved aside, not mixed.
//!
//! - F2 plugs the test stick in, or pulls it out;
//! - F3 does the same for the blank stick;
//! - F4 pulls every stick out.
//!
//! Any other folder in `DIR` can be added by hand and plugged with F5,
//! which attaches every folder there. What the app writes to a stick is a
//! file in that folder.
//!
//! This is the online Faraday: a sheet file in the Inbox or the Outbox has
//! Make PDF, which saves the PDF in `$HOME/faraday-print`.
//!
//! Every session here starts on testnet, the test stick's network; the
//! stick's app starts on mainnet.
//!
//! Locking ends the app's process on the device; here it replaces the app
//! with a fresh one in the same window, with the Inbox and Outbox carried
//! over, which is what the device's app loop does.

mod camera;

use std::collections::BTreeSet;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::time::{Duration, Instant};

use camera::Camera;
use faraday_core::{Faraday, StickInfo, StorageEvent};
use faraday_scanner::Scanner;
use faraday_storage::{Boxes, serve, stick_info};
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

const TICK: Duration = Duration::from_millis(50);
/// How often the attached folders are looked at again.
const RESCAN: Duration = Duration::from_millis(700);
const TITLE: &str = "Faraday · F2 test stick · F3 blank stick · F4 pull all";

struct Shell {
    sticks_dir: PathBuf,
    /// Where PDFs are saved.
    print_dir: PathBuf,
    size: (u32, u32),
    app: Option<Faraday>,
    boxes: Boxes,
    attached: BTreeSet<String>,
    seen: Vec<StickInfo>,
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    shown: (u32, u32),
    /// The window's scale factor.
    scale: f64,
    epoch: Instant,
    next_tick: Instant,
    next_scan: Instant,
    cursor: (u16, u16),
    /// The webcam, when the app asks for it.
    camera: Camera,
    /// Reads the codes in the camera's frames on its own thread.
    scanner: Option<Scanner>,
    pressed: bool,
    shift: bool,
    wheel: f32,
}

fn entropy() -> Option<Event> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .ok()
        .map(|()| Event::Entropy(EntropyBytes::new(bytes)))
}

impl Shell {
    fn display(&self) -> Event {
        let (w, h) = self.shown;
        Event::Display(DisplayInfo {
            width: w.clamp(320, u32::from(u16::MAX)) as u16,
            height: h.clamp(240, u32::from(u16::MAX)) as u16,
            // The compositor's scale for this window, as a density: the
            // app grows no larger than a quarter beyond it.
            dpi: (160.0 * self.scale).round().clamp(80.0, 640.0) as u16,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: true,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        })
    }

    fn sticks(&self) -> Vec<StickInfo> {
        self.attached
            .iter()
            .map(|name| stick_info(&self.sticks_dir.join(name), name, false))
            .collect()
    }

    /// Delivers one event, then answers what the app asked for. A lock
    /// starts a fresh app, as the device's app loop does.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let mut draw = false;
        let mut events = vec![event];
        while let Some(e) = events.pop() {
            let Some(app) = self.app.as_mut() else { return };
            app.event(e);
            while let Some(c) = app.poll_command() {
                match c {
                    Command::Draw => draw = true,
                    Command::RequestEntropy => events.extend(entropy()),
                    Command::CameraOn => {
                        // Which cameras there are, for the scan sheet's
                        // choice, as of the moment it opens.
                        app.storage(faraday_core::StorageEvent::Cameras(camera::list()));
                        if let Err(reason) = self.camera.on() {
                            eprintln!("faraday: camera unavailable: {reason}");
                            events.push(Event::CameraUnavailable);
                        } else if self.scanner.is_none() {
                            self.scanner = Some(Scanner::new());
                        }
                    }
                    Command::CameraOff => {
                        self.camera.off();
                        self.scanner = None;
                    }
                    Command::Exit => {
                        if app.restart_requested() {
                            for line in serve(app, &mut self.boxes, Some(&self.print_dir)) {
                                eprintln!("{line}");
                            }
                            self.restart(event_loop);
                            return;
                        }
                        event_loop.exit();
                        return;
                    }
                    _ => {}
                }
            }
            // A camera the person chose on the scan sheet.
            if let Some(id) = app.take_camera() {
                self.camera.choose(PathBuf::from(id));
            }
            for line in serve(app, &mut self.boxes, Some(&self.print_dir)) {
                eprintln!("{line}");
            }
        }
        // A write changes a stick's files: look again now.
        self.rescan(event_loop, false);
        if draw && let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn restart(&mut self, event_loop: &ActiveEventLoop) {
        self.app = Some(desktop_app());
        self.seen.clear();
        let display = self.display();
        let restored = self.boxes.restore();
        if let Some(app) = self.app.as_mut() {
            app.event(display);
            app.storage(restored);
        }
        self.send(
            event_loop,
            Event::Tick {
                now_ms: self.epoch.elapsed().as_millis() as u64,
            },
        );
        self.rescan(event_loop, true);
    }

    fn rescan(&mut self, event_loop: &ActiveEventLoop, force: bool) {
        let now = self.sticks();
        if !force && now == self.seen {
            return;
        }
        self.seen = now.clone();
        if let Some(app) = self.app.as_mut() {
            app.storage(StorageEvent::Sticks(now));
            if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                app.storage(StorageEvent::Clock {
                    unix_secs: t.as_secs(),
                });
            }
            if let Some(available_mib) = faraday_storage::memory_available_mib() {
                app.storage(StorageEvent::Memory { available_mib });
            }
            for line in serve(app, &mut self.boxes, Some(&self.print_dir)) {
                eprintln!("{line}");
            }
            while let Some(c) = app.poll_command() {
                if c == Command::Exit && !app.restart_requested() {
                    event_loop.exit();
                }
            }
        }
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    fn toggle(&mut self, event_loop: &ActiveEventLoop, name: &str) {
        if !self.attached.remove(name) {
            self.attached.insert(name.to_string());
        }
        self.rescan(event_loop, false);
    }

    fn draw(&mut self) {
        let (Some(window), Some(surface), Some(app)) =
            (&self.window, self.surface.as_mut(), self.app.as_mut())
        else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else {
            return;
        };
        let frame = app.frame();
        let (fw, fh) = (usize::from(frame.width), usize::from(frame.height));
        let (sw, sh) = (size.width as usize, size.height as usize);
        for y in 0..sh {
            for x in 0..sw {
                let px = if x < fw && y < fh {
                    let i = (y * fw + x) * 4;
                    let p = &frame.rgba[i..i + 4];
                    (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2])
                } else {
                    0x0011_161c
                };
                buffer[y * sw + x] = px;
            }
        }
        let _ = buffer.present();
    }

    fn touch(&mut self, event_loop: &ActiveEventLoop, phase: TouchPhase) {
        let (x, y) = self.cursor;
        self.send(event_loop, Event::Touch { x, y, phase });
    }
}

fn map_key(key: &WinitKey) -> Option<Key> {
    match key {
        // Delete as well: the fields have no cursor to delete after, and
        // with a field selected it empties it.
        WinitKey::Named(NamedKey::Backspace | NamedKey::Delete) => Some(Key::Backspace),
        WinitKey::Named(NamedKey::Enter) => Some(Key::Enter),
        WinitKey::Named(NamedKey::Escape) => Some(Key::Escape),
        WinitKey::Named(NamedKey::ArrowUp) => Some(Key::Up),
        WinitKey::Named(NamedKey::ArrowDown) => Some(Key::Down),
        WinitKey::Named(NamedKey::ArrowLeft) => Some(Key::Left),
        WinitKey::Named(NamedKey::ArrowRight) => Some(Key::Right),
        WinitKey::Named(NamedKey::Tab) => Some(Key::Tab),
        WinitKey::Named(NamedKey::Space) => Some(Key::Char(' ')),
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

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(
                f64::from(self.size.0),
                f64::from(self.size.1),
            ))
            .with_min_inner_size(LogicalSize::new(640.0, 400.0));
        let window = match event_loop.create_window(attributes) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                eprintln!("faraday: cannot open a window: {e}");
                event_loop.exit();
                return;
            }
        };
        let Ok(context) = Context::new(window.clone()) else {
            eprintln!("faraday: cannot create a drawing context");
            event_loop.exit();
            return;
        };
        let Ok(surface) = Surface::new(&context, window.clone()) else {
            eprintln!("faraday: cannot create a drawing surface");
            event_loop.exit();
            return;
        };
        let size = window.inner_size();
        self.shown = (size.width, size.height);
        self.scale = window.scale_factor();
        self.window = Some(window);
        self.surface = Some(surface);
        let display = self.display();
        self.send(event_loop, display);
        self.rescan(event_loop, true);
        self.next_tick = Instant::now() + TICK;
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                let display = self.display();
                self.send(event_loop, display);
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 && (size.width, size.height) != self.shown {
                    self.shown = (size.width, size.height);
                    let display = self.display();
                    self.send(event_loop, display);
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = (
                    position.x.clamp(0.0, 65535.0) as u16,
                    position.y.clamp(0.0, 65535.0) as u16,
                );
                let moved = p != self.cursor;
                self.cursor = p;
                if self.pressed && moved {
                    self.touch(event_loop, TouchPhase::Move);
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => {
                    self.pressed = true;
                    self.touch(event_loop, TouchPhase::Down);
                }
                ElementState::Released => {
                    if self.pressed {
                        self.pressed = false;
                        self.touch(event_loop, TouchPhase::Up);
                    }
                }
            },
            WindowEvent::Focused(false) => {
                if self.pressed {
                    self.pressed = false;
                    self.touch(event_loop, TouchPhase::Up);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // The shell API's scroll is pixels the content moves up;
                // winit's positive is a wheel turned away or two fingers
                // moved up, which move the content down. A wheel notch
                // is 48 pixels, as on the stick.
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => -lines * 48.0,
                    MouseScrollDelta::PixelDelta(p) => -(p.y as f32),
                };
                self.wheel += dy;
                let whole = self.wheel.trunc();
                self.wheel -= whole;
                if whole != 0.0 {
                    let (x, y) = self.cursor;
                    self.send(
                        event_loop,
                        Event::Scroll {
                            x,
                            y,
                            dy: whole.clamp(-32768.0, 32767.0) as i16,
                        },
                    );
                }
            }
            WindowEvent::ModifiersChanged(m) => self.shift = m.state().shift_key(),
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                match &event.logical_key {
                    WinitKey::Named(NamedKey::F2) => return self.toggle(event_loop, "TESTSTICK"),
                    WinitKey::Named(NamedKey::F3) => return self.toggle(event_loop, "BLANK"),
                    WinitKey::Named(NamedKey::F4) => {
                        self.attached.clear();
                        return self.rescan(event_loop, false);
                    }
                    WinitKey::Named(NamedKey::F5) => {
                        if let Ok(d) = std::fs::read_dir(&self.sticks_dir) {
                            for e in d.flatten() {
                                if e.path().is_dir() {
                                    self.attached
                                        .insert(e.file_name().to_string_lossy().into_owned());
                                }
                            }
                        }
                        return self.rescan(event_loop, false);
                    }
                    _ => {}
                }
                let Some(key) = map_key(&event.logical_key) else {
                    return;
                };
                let key = if key == Key::Tab && self.shift {
                    Key::BackTab
                } else {
                    key
                };
                self.send(event_loop, Event::Key(key));
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }
        // The newest camera frame, to the app to draw and to the scanner
        // to read; and whatever the scanner has read since.
        let mut newest = None;
        let mut answers = Vec::new();
        while let Some(event) = self.camera.poll() {
            match event {
                Event::CameraFrame { .. } => newest = Some(event),
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
        while let Some(bytes) = self.scanner.as_ref().and_then(Scanner::poll) {
            self.send(event_loop, Event::Scanned { bytes });
        }
        // What the scanner saw of a code, for the outline and for "too
        // fine for this camera".
        if let Some(seen) = self.scanner.as_ref().and_then(Scanner::poll_seen)
            && let Some(app) = self.app.as_mut()
        {
            app.storage(faraday_core::StorageEvent::QrSeen {
                width: seen.width,
                height: seen.height,
                corners: seen.corners,
                module_tenths: (seen.module_px * 10.0).clamp(0.0, 65535.0) as u16,
                read: seen.read,
            });
        }
        let now = Instant::now();
        if now >= self.next_tick {
            let now_ms = self.epoch.elapsed().as_millis() as u64;
            self.send(event_loop, Event::Tick { now_ms });
            self.next_tick = now + TICK;
        }
        if now >= self.next_scan {
            self.rescan(event_loop, false);
            self.next_scan = now + RESCAN;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.camera.off();
        self.scanner = None;
        self.surface = None;
        self.window = None;
        self.app = None;
    }
}

/// A fresh app as the desktop runs it: the online Faraday, which turns
/// sheets into PDFs, and a test app, which starts every session on
/// testnet as its test stick is. The stick's app starts on mainnet.
fn desktop_app() -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    app.session = faraday_core::wallet::Session::on(faraday_core::testkit::NET);
    app
}

/// Which test kit a test stick folder holds. A folder from another kit
/// (the mainnet one before this) is moved aside, not mixed with this one.
const KIT_MARK: &str = "faraday-testkit testnet 1\n";
/// The mark of the backup test stick.
const BACKUP_MARK: &str = "faraday-testkit backup 1\n";

/// Makes the two stick folders, filling the test stick the first time.
fn prepare(dir: &Path, full: bool) -> Result<(), String> {
    let (kit_mark, files) = if full {
        (KIT_MARK, faraday_core::testkit::files()?)
    } else {
        (BACKUP_MARK, faraday_core::testkit::backup_files()?)
    };
    let test = dir.join("TESTSTICK");
    let blank = dir.join("BLANK");
    std::fs::create_dir_all(&blank).map_err(|e| format!("{}: {e}", blank.display()))?;
    let mark = test.join(".faraday-testkit");
    if test.exists() && std::fs::read_to_string(&mark).ok().as_deref() != Some(kit_mark) {
        let aside = (1..)
            .map(|n| dir.join(format!("TESTSTICK-old-{n}")))
            .find(|p| !p.exists())
            .expect("a free name");
        std::fs::rename(&test, &aside).map_err(|e| format!("{}: {e}", test.display()))?;
        eprintln!(
            "faraday: the old test stick folder is now {}",
            aside.display()
        );
    }
    // Files the test kit has and the folder lacks are added; nothing
    // already there is replaced.
    std::fs::create_dir_all(&test).map_err(|e| format!("{}: {e}", test.display()))?;
    for (name, bytes) in files {
        let path = test.join(&name);
        if !path.exists() {
            std::fs::write(&path, bytes).map_err(|e| format!("{name}: {e}"))?;
        }
    }
    std::fs::write(&mark, kit_mark).map_err(|e| format!("{}: {e}", mark.display()))?;
    Ok(())
}

fn main() -> ExitCode {
    let mut sticks_dir = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("faraday-sticks"))
        .unwrap_or_else(|| PathBuf::from("faraday-sticks"));
    let mut size = (1280, 800);
    let mut full_kit = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--sticks" => match args.next() {
                Some(d) => sticks_dir = PathBuf::from(d),
                None => {
                    eprintln!("faraday: --sticks needs a directory");
                    return ExitCode::from(2);
                }
            },
            "--full-kit" => full_kit = true,
            "--size" => {
                let parsed = args.next().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                });
                match parsed {
                    Some(s) => size = s,
                    None => {
                        eprintln!("faraday: --size needs WxH");
                        return ExitCode::from(2);
                    }
                }
            }
            "--help" | "-h" => {
                println!(
                    "faraday [--sticks DIR] [--size WxH] [--full-kit]\n  F2 test stick, F3 blank stick, F4 pull all, F5 plug every folder in DIR\n  --full-kit: the test stick carries the full test kit, not the backup test stick"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("faraday: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    if let Err(e) = prepare(&sticks_dir, full_kit) {
        eprintln!("faraday: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("faraday: sticks are folders in {}", sticks_dir.display());
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("faraday: no display to open a window on: {e}");
            return ExitCode::FAILURE;
        }
    };
    let now = Instant::now();
    let print_dir = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("faraday-print"))
        .unwrap_or_else(|| PathBuf::from("faraday-print"));
    let app = desktop_app();
    let mut shell = Shell {
        print_dir,
        sticks_dir,
        size,
        app: Some(app),
        boxes: Boxes::Memory {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: Vec::new(),
        },
        attached: BTreeSet::new(),
        seen: Vec::new(),
        window: None,
        surface: None,
        shown: size,
        scale: 1.0,
        epoch: now,
        next_tick: now,
        next_scan: now,
        cursor: (0, 0),
        camera: Camera::new(None, false),
        scanner: None,
        pressed: false,
        shift: false,
        wheel: 0.0,
    };
    match event_loop.run_app(&mut shell) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("faraday: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The desktop app is the test app: its sessions start on the test
    /// stick's network, and the stick's app on mainnet.
    #[test]
    fn the_desktop_starts_on_testnet_and_the_device_on_mainnet() {
        assert_eq!(desktop_app().session.network(), faraday_core::testkit::NET);
        assert!(Faraday::new().session.network().is_mainnet());
    }
}
