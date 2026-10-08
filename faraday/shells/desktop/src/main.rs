//! Faraday in a window.
//!
//! ```text
//! faraday [--sticks DIR] [--size WxH] [--full-kit]
//!         [--panel INCHES [--aspect W:H] [--ppi N]]
//! ```
//!
//! The window draws at the display's own resolution and can be resized;
//! the app lays itself out for whatever size it is given.
//!
//! `--panel` stands for a device's own screen instead: a window that
//! measures INCHES corner to corner on this screen, 3:4 upright below 4
//! inches (the Pi's 2.8-inch panel) and 9:16 at 4 and over (a phone),
//! or `--aspect` given, with the app laid out at that screen's true size
//! as the device lays it out. The window keeps its size. This screen's
//! pixels per inch come from its EDID; `--ppi` overrides them. A finger
//! on a touchscreen presses as the device's panel does.
//!
//! Sticks are folders. `--sticks DIR` (default `$HOME/faraday-sticks`)
//! holds `TESTSTICK`, which gets every test kit file it lacks at each
//! start and never has a file overwritten, and `BLANK`, which starts empty. Neither is
//! attached at start. The test stick is the backup test stick
//! (`faraday_core::testkit::backup_files`: a 2-of-3 Taproot multisig's
//! public backup files and a vault, passphrase `a`, holding its three
//! seeds); `--full-kit` makes it the full test kit instead. A folder made
//! for the other kit is moved aside, not mixed. The test stick stands for
//! the boot stick: the first time it is plugged in a session, its
//! `faraday-settings.txt` is read and every other file on it is copied
//! into memory for the boot import, as the device does with the stick it
//! booted from; later, it is visited like any stick.
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
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, NamedKey};
use winit::window::{Window, WindowId};

const TICK: Duration = Duration::from_millis(50);

/// The tick while something moves: a frame at 60 Hz (`docs/MOTION.md`
/// §3.2).
const FRAME: Duration = Duration::from_micros(16_667);
/// How often the attached folders are looked at again.
const RESCAN: Duration = Duration::from_millis(700);
const TITLE: &str = "Faraday · F2 test stick · F3 blank stick · F4 pull all";
const TITLE_KEYS: &str = "F2 test stick · F3 blank · F4 pull all";

/// A device's screen, shown at its true size on this one.
#[derive(Clone, Copy)]
struct Panel {
    /// Its diagonal, inches.
    inches: f64,
    /// Its width to its height.
    aspect: (f64, f64),
    /// This screen's pixels per inch.
    ppi: f64,
}

impl Panel {
    /// Its size in this screen's pixels.
    fn pixels(&self) -> (u32, u32) {
        let (aw, ah) = self.aspect;
        let d = (aw * aw + ah * ah).sqrt();
        let w = self.inches * aw / d * self.ppi;
        let h = self.inches * ah / d * self.ppi;
        (w.round() as u32, h.round() as u32)
    }
}

/// This screen's pixels per inch, from the first connected panel's EDID:
/// its first detailed timing's active pixels over its size in millimetres.
fn screen_ppi() -> Option<f64> {
    let dir = std::fs::read_dir("/sys/class/drm").ok()?;
    let mut paths: Vec<PathBuf> = dir.flatten().map(|e| e.path()).collect();
    // A laptop's own panel first.
    paths.sort_by_key(|p| !p.to_string_lossy().contains("eDP"));
    for p in paths {
        let Ok(e) = std::fs::read(p.join("edid")) else {
            continue;
        };
        if e.len() < 72 {
            continue;
        }
        let px_w = f64::from(u16::from(e[56]) | (u16::from(e[58] & 0xF0) << 4));
        let px_h = f64::from(u16::from(e[59]) | (u16::from(e[61] & 0xF0) << 4));
        let mm_w = f64::from(u16::from(e[66]) | (u16::from(e[68] & 0xF0) << 4));
        let mm_h = f64::from(u16::from(e[67]) | (u16::from(e[68] & 0x0F) << 8));
        if px_w > 0.0 && mm_w > 0.0 && mm_h > 0.0 {
            let px = (px_w * px_w + px_h * px_h).sqrt();
            let inches = (mm_w * mm_w + mm_h * mm_h).sqrt() / 25.4;
            return Some(px / inches);
        }
    }
    None
}

struct Shell {
    sticks_dir: PathBuf,
    /// A device's screen to stand for, when given.
    panel: Option<Panel>,
    /// The finger pressing, on a touchscreen.
    finger: Option<u64>,
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
    /// The last tick asked for a frame: something is moving, so the next
    /// tick comes a frame later rather than at the idle rate.
    animating: bool,
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
            // app grows no larger than a quarter beyond it. Standing for
            // a device's screen, this screen's own density, so the app
            // is laid out at that screen's true size.
            dpi: match self.panel {
                Some(p) => p.ppi.round().clamp(80.0, 640.0) as u16,
                None => (160.0 * self.scale).round().clamp(80.0, 640.0) as u16,
            },
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
            // The test stick stands for the boot stick: its first plugging
            // in a session reads its settings file and brings up the boot
            // import.
            .map(|name| stick_info(&self.sticks_dir.join(name), name, name == "TESTSTICK"))
            .collect()
    }

    /// Delivers one event, then answers what the app asked for. A lock
    /// starts a fresh app, as the device's app loop does.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let ticked = matches!(event, Event::Tick { .. });
        // What may start something moving is followed by a tick within a
        // frame, so the motion starts at once.
        if matches!(
            event,
            Event::Wheel { .. }
                | Event::Scroll { .. }
                | Event::ScrollEnd { .. }
                | Event::Touch { .. }
        ) {
            self.next_tick = self.next_tick.min(Instant::now() + FRAME);
        }
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
        if ticked {
            self.animating = draw;
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
        let attributes = match self.panel {
            // A device's screen keeps its size: given in the compositor's
            // units, since a window is sized before it knows its scale.
            Some(p) => {
                let (w, h) = p.pixels();
                // The laptop's own panel, whose EDID gave the pixels per
                // inch, is the screen it stands on.
                let monitors: Vec<_> = event_loop.available_monitors().collect();
                let scale = monitors
                    .iter()
                    .find(|m| m.name().is_some_and(|n| n.starts_with("eDP")))
                    .or(monitors.first())
                    .map_or(1.0, |m| m.scale_factor());
                let size = LogicalSize::new(f64::from(w) / scale, f64::from(h) / scale);
                Window::default_attributes()
                    .with_title(format!("Faraday · {} in panel · {TITLE_KEYS}", p.inches))
                    .with_inner_size(size)
                    .with_min_inner_size(size)
                    .with_max_inner_size(size)
                    .with_resizable(false)
            }
            None => Window::default_attributes()
                .with_title(TITLE)
                .with_inner_size(LogicalSize::new(
                    f64::from(self.size.0),
                    f64::from(self.size.1),
                ))
                .with_min_inner_size(LogicalSize::new(640.0, 400.0)),
        };
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
                // A device's screen stays the same size in this screen's
                // pixels at any scale.
                if let (Some(p), Some(w)) = (self.panel, &self.window) {
                    let size = PhysicalSize::new(p.pixels().0, p.pixels().1);
                    w.set_min_inner_size(Some(size));
                    w.set_max_inner_size(Some(size));
                    let _ = w.request_inner_size(size);
                }
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
                } else if moved {
                    self.send(event_loop, Event::Hover { x: p.0, y: p.1 });
                }
            }
            WindowEvent::CursorLeft { .. } => {
                if !self.pressed {
                    self.send(event_loop, Event::HoverEnd);
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
            // A finger on a touchscreen presses as on the device's panel:
            // the first finger down is the press, others are ignored.
            WindowEvent::Touch(t) => {
                use winit::event::TouchPhase as T;
                let p = (
                    t.location.x.clamp(0.0, 65535.0) as u16,
                    t.location.y.clamp(0.0, 65535.0) as u16,
                );
                match t.phase {
                    T::Started if self.finger.is_none() => {
                        self.finger = Some(t.id);
                        self.cursor = p;
                        self.pressed = true;
                        self.touch(event_loop, TouchPhase::Down);
                    }
                    T::Moved if self.finger == Some(t.id) => {
                        self.cursor = p;
                        self.touch(event_loop, TouchPhase::Move);
                    }
                    T::Ended | T::Cancelled if self.finger == Some(t.id) => {
                        self.cursor = p;
                        self.finger = None;
                        self.pressed = false;
                        self.touch(event_loop, TouchPhase::Up);
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseWheel { delta, phase, .. } => {
                // The shell API's scroll is pixels the content moves up;
                // winit's positive is a wheel turned away or two fingers
                // moved up, which move the content down. A wheel notch
                // is 48 pixels, as on the stick, and glides; two fingers
                // move the content with them and coast when they lift.
                let (x, y) = self.cursor;
                let (dy, notched) = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => (-lines * 48.0, true),
                    MouseScrollDelta::PixelDelta(p) => (-(p.y as f32), false),
                };
                self.wheel += dy;
                let whole = self.wheel.round();
                self.wheel -= whole;
                if whole != 0.0 {
                    let dy = whole.clamp(-32768.0, 32767.0) as i16;
                    let event = if notched {
                        Event::Wheel { x, y, dy }
                    } else {
                        Event::Scroll { x, y, dy }
                    };
                    self.send(event_loop, event);
                }
                if !notched && phase == winit::event::TouchPhase::Ended {
                    self.wheel = 0.0;
                    self.send(event_loop, Event::ScrollEnd { x, y });
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
            self.next_tick = now + if self.animating { FRAME } else { TICK };
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
    let mut panel: Option<f64> = None;
    let mut aspect: Option<(f64, f64)> = None;
    let mut ppi: Option<f64> = None;
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
            "--panel" => match args
                .next()
                .and_then(|s| s.trim_end_matches("in").parse().ok())
            {
                Some(i) if (1.0..=13.0).contains(&i) => panel = Some(i),
                _ => {
                    eprintln!("faraday: --panel needs a diagonal in inches, 1 to 13");
                    return ExitCode::from(2);
                }
            },
            "--aspect" => {
                let parsed = args.next().and_then(|s| {
                    let (w, h) = s.split_once(':')?;
                    Some((w.parse::<f64>().ok()?, h.parse::<f64>().ok()?))
                });
                match parsed {
                    Some((w, h)) if w > 0.0 && h > 0.0 => aspect = Some((w, h)),
                    _ => {
                        eprintln!("faraday: --aspect needs W:H, such as 3:4");
                        return ExitCode::from(2);
                    }
                }
            }
            "--ppi" => match args.next().and_then(|s| s.parse().ok()) {
                Some(p) if (50.0..=1000.0).contains(&p) => ppi = Some(p),
                _ => {
                    eprintln!("faraday: --ppi needs this screen's pixels per inch");
                    return ExitCode::from(2);
                }
            },
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
                    "faraday [--sticks DIR] [--size WxH] [--full-kit] [--panel INCHES [--aspect W:H] [--ppi N]]\n  F2 test stick, F3 blank stick, F4 pull all, F5 plug every folder in DIR\n  --full-kit: the test stick carries the full test kit, not the backup test stick\n  --panel: a device's screen at its true size: 2.8 is the Pi's panel (3:4), 5 a phone (9:16)\n  --aspect: the panel's width to height · --ppi: this screen's pixels per inch, if its EDID is wrong"
                );
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("faraday: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    let panel = match panel {
        Some(inches) => {
            let Some(ppi) = ppi.or_else(screen_ppi) else {
                eprintln!("faraday: this screen's pixels per inch are unknown: give --ppi");
                return ExitCode::from(2);
            };
            let aspect = aspect.unwrap_or(if inches < 4.0 {
                (3.0, 4.0)
            } else {
                (9.0, 16.0)
            });
            eprintln!("faraday: a {inches} in panel at {ppi:.0} pixels per inch here");
            Some(Panel {
                inches,
                aspect,
                ppi,
            })
        }
        None => None,
    };
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
        panel,
        finger: None,
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
        animating: false,
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
