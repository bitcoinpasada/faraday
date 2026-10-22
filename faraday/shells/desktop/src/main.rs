//! Faraday in a window.
//!
//! ```text
//! faraday [--sticks DIR] [--size WxH] [--transfer]
//!         [--downloads DIR] [--panel INCHES [--aspect W:H] [--ppi N]]
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
//! Sticks are folders. `--sticks DIR` (default `faraday-sticks` in the home folder: `$HOME`, or
//! `%USERPROFILE%` on Windows)
//! holds `BOOTSTICK`, the boot stick, which starts empty and to which
//! the desktop never adds a file (owner, 2026-10-19: a person puts there
//! the files to boot with), and one folder for each backup stick:
//! `STICK-2`, `STICK-3` and on (the boot stick is Stick 1), made when a
//! stick is first put in and empty. The app never moves, renames or
//! removes anything in these folders, so they stay as a person left them
//! across installs and new versions (owner, 2026-10-20). The boot stick
//! is made once, the first time; renamed or removed later it is not made
//! again, and the app starts with no stick in. The boot
//! stick is plugged in when Faraday opens, as the device starts with the
//! stick it booted from: its `faraday-settings.txt` is read and every
//! other file on it is copied into memory for the boot import, and
//! pulling it starts the import. Plugged in again later, it is
//! visited like any stick.
//!
//! The sticks are the pills across the top of the window
//! (`faraday_core::stick::pills`): Boot stick, Stick 2, and one for each
//! stick a backup flow needs. A click on a pill plugs its stick in, or
//! pulls it out; several may be in at once. Only the boot stick is in
//! at start. What the app writes to a stick is a file in its folder, so a
//! stick pulled and put back holds it still.
//!
//! This is the online Faraday: a sheet file in the Inbox or the Outbox has
//! Make PDF, which saves the PDF in `$HOME/faraday-print`.
//!
//! It is also the device's QR link: **Transfer** (in the sidebar here
//! only; `--transfer` opens on it) sends any file of `$HOME/Downloads`
//! (`--downloads DIR` for another folder), listed newest first, as
//! animated codes for the device's camera, and receives the device's
//! codes through the webcam, saving each whole file into that folder
//! under its own name or `received-N`, never over a file already there
//! (`name (2).ext`). A file dropped on the window is sent too, where
//! winit reports the drop: on X11, not on native Wayland. Nothing
//! received goes into the app's Inbox.
//!
//! Every session here opens in Demo, on testnet, in every build; the
//! pill and Settings choose Testnet, Signet or Regtest, and a lock
//! keeps the mode (F81). No warning comes first: every step that takes
//! a seed carries a line saying never to type one that holds real
//! bitcoin.
//! The stick's app starts on mainnet.
//!
//! Locking ends the app's process on the device; here it replaces the app
//! with a fresh one in the same window, with the Inbox and Outbox carried
//! over, which is what the device's app loop does.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use faraday_core::strings::{EN, fill};
use faraday_core::{Action, BOOT_FOLDER, Faraday, Screen, StickInfo, StorageEvent};
use faraday_shell_api::{App, Command, Event};
use faraday_storage::{Boxes, Dirs, Host, serve_with, stick_info};
use faraday_window::camera::{self, Camera};
use faraday_window::clipboard::Clipboard;
use faraday_window::entropy;
use faraday_window::input::Pointer;
use faraday_window::lens::{Lens, LensHost};
use faraday_window::pace::{Pacer, TICK};
use faraday_window::view::{self, View};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// How often the Downloads folder and the cameras are looked at again,
/// on a thread of their own ([`watch_listing`]); and the sticks.
const RESCAN: Duration = Duration::from_millis(700);
/// A frame or an event slower than this is reported in a test build.
const SLOW: Duration = Duration::from_millis(50);
const TITLE: &str = EN.desktop_title;
/// The boot stick's folder (Stick 1), and the first backup stick's,
/// Stick 2.
const BOOT: &str = BOOT_FOLDER;
const FIRST: &str = "STICK-2";

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
    /// Where PDFs are saved, and the Downloads folder Transfer uses.
    host: Host,
    /// What [`watch_listing`] reads: the Downloads folder and the
    /// cameras, sent when either changed.
    listing: Receiver<Listing>,
    /// The newest of what [`watch_listing`] sent; a fresh app is given it
    /// again.
    latest: Option<Listing>,
    /// The Downloads folder as last listed to the app.
    listed: Option<StorageEvent>,
    /// The cameras as last listed to the app.
    cameras: Option<Vec<(String, String)>>,
    /// Open on Transfer once the window is up.
    open_transfer: bool,
    size: (u32, u32),
    app: Option<Faraday>,
    boxes: Boxes,
    /// The sticks' folders, their pictures read on a thread of their own.
    dirs: Dirs,
    attached: BTreeSet<String>,
    seen: Vec<StickInfo>,
    /// The window and its surface (`faraday-window`).
    view: Option<View>,
    shown: (u32, u32),
    /// The window's scale factor.
    scale: f64,
    /// The ticks and the frames' pace (`faraday-window`).
    pace: Pacer,
    /// When the sticks, the clock and the memory are next looked at.
    next_scan: Instant,
    /// When a slow frame or event was last printed.
    last_slow: Option<Instant>,
    /// The mouse, a finger and the keyboard (`faraday-window`).
    pointer: Pointer,
    /// The webcam, when the app asks for it, and the scanner that reads
    /// its frames on its own thread (`faraday-window`).
    lens: Lens,
    /// The system clipboard, read for the scanner's Paste.
    clipboard: Clipboard,
}

impl Shell {
    fn display(&self) -> Event {
        view::display(self.shown, self.scale, self.panel.map(|p| p.ppi))
    }

    fn window(&self) -> Option<&Window> {
        self.view.as_ref().map(|v| &*v.window)
    }

    fn sticks(&self) -> Vec<StickInfo> {
        self.attached
            .iter()
            // A folder renamed or removed is a stick pulled.
            .filter(|name| self.sticks_dir.join(name).is_dir())
            // The boot stick's first plugging in a session, at start,
            // reads its settings file and brings up the boot import.
            .map(|name| stick_info(&self.sticks_dir.join(name), name, name == BOOT))
            .collect()
    }

    /// Delivers one event, then answers what the app asked for. A lock
    /// starts a fresh app, as the device's app loop does.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let begun = Instant::now();
        let storage = self.deliver(event_loop, event);
        let took = begun.elapsed();
        self.pace.event_took(took.saturating_sub(storage));
        self.slow("event", took);
    }

    /// Prints a slow frame or event on stderr in a build that is not a
    /// release: durations and screen names only, one line a second.
    fn slow(&mut self, what: &str, took: Duration) {
        if faraday_core::is_release() || took <= SLOW {
            return;
        }
        let now = Instant::now();
        if self
            .last_slow
            .is_some_and(|t| now - t < Duration::from_secs(1))
        {
            return;
        }
        let Some(app) = self.app.as_ref() else { return };
        self.last_slow = Some(now);
        eprintln!(
            "faraday: slow {what} {} ms · {}",
            took.as_millis(),
            app.screen_name()
        );
    }

    /// Delivers `event`; returns the time spent on storage.
    fn deliver(&mut self, event_loop: &ActiveEventLoop, event: Event) -> Duration {
        let mut served = false;
        let mut storage = Duration::ZERO;
        let ticked = matches!(event, Event::Tick { .. });
        self.pace.before(&event);
        let mut draw = false;
        let mut toggles = Vec::new();
        let mut events = vec![event];
        while let Some(e) = events.pop() {
            let Some(app) = self.app.as_mut() else {
                return Duration::ZERO;
            };
            app.event(e);
            while let Some(c) = app.poll_command() {
                match c {
                    Command::Draw => draw = true,
                    Command::RequestEntropy => events.extend(entropy()),
                    Command::CameraOn => events.extend(self.lens.on(app)),
                    Command::CameraOff => self.lens.off(),
                    Command::RequestClipboard { kind } => {
                        events.push(self.clipboard.answer(kind));
                    }
                    Command::Exit => {
                        if app.restart_requested() {
                            for line in
                                serve_with(app, &mut self.boxes, Some(&self.host), &mut self.dirs)
                            {
                                eprintln!("{line}");
                            }
                            self.restart(event_loop);
                            return Duration::ZERO;
                        }
                        event_loop.exit();
                        return Duration::ZERO;
                    }
                    _ => {}
                }
            }
            // The sticks the person clicked on, plugged in or pulled.
            toggles.extend(app.take_stick_toggles());
            // A camera the person chose on the scan sheet.
            self.lens.chosen(app);
            served |= app.storage_pending();
            let at = Instant::now();
            for line in serve_with(app, &mut self.boxes, Some(&self.host), &mut self.dirs) {
                eprintln!("{line}");
            }
            storage += at.elapsed();
        }
        if ticked {
            self.pace.animating = draw;
        }
        // A stick toggled or a storage command served changes a stick's
        // files: look again now. Otherwise the timer looks.
        let looks = served || !toggles.is_empty();
        for name in toggles {
            self.toggle(&name);
        }
        if looks {
            let at = Instant::now();
            self.rescan(event_loop, false);
            storage += at.elapsed();
        }
        self.pace.storage_took(storage);
        if draw && let Some(w) = self.window() {
            w.request_redraw();
        }
        storage
    }

    fn restart(&mut self, event_loop: &ActiveEventLoop) {
        let mode = self
            .app
            .as_ref()
            .map_or(faraday_core::Mode::Demo, Faraday::mode_after_restart);
        self.app = Some(desktop_app_in(mode));
        // Pictures the last app asked for are not this one's.
        self.dirs = Dirs::default();
        self.seen.clear();
        let display = self.display();
        let restored = self.boxes.restore();
        if let Some(app) = self.app.as_mut() {
            app.event(display);
            app.storage(restored);
        }
        let tick = self.pace.tick();
        self.send(event_loop, tick);
        self.rescan(event_loop, true);
        // The new app has been told nothing yet: give it the newest
        // listing again, without waiting for the next change.
        self.listed = None;
        self.cameras = None;
        self.relist();
    }

    /// Tells the app what Downloads holds and which cameras there are,
    /// when that changed: the newest of what [`watch_listing`] read, or
    /// the last one again for a fresh app. A lookup; the reading is done
    /// on that thread.
    fn relist(&mut self) {
        if let Some(now) = self.listing.try_iter().last() {
            self.latest = Some(now);
        }
        let Some(now) = self.latest.clone() else {
            return;
        };
        let Some(app) = self.app.as_mut() else { return };
        let mut changed = false;
        if self.listed.as_ref() != Some(&now.files) {
            app.storage(now.files.clone());
            self.listed = Some(now.files);
            changed = true;
        }
        if self.cameras.as_ref() != Some(&now.cameras) {
            app.storage(StorageEvent::Cameras(now.cameras.clone()));
            self.cameras = Some(now.cameras);
            changed = true;
        }
        if changed && let Some(w) = self.window() {
            w.request_redraw();
        }
    }

    /// A file dropped on the window, for Transfer to send.
    fn dropped(&mut self, event_loop: &ActiveEventLoop, path: &Path) {
        let Some(app) = self.app.as_mut() else { return };
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        app.storage(StorageEvent::Dropped {
            path: path.display().to_string(),
            size,
        });
        for line in serve_with(app, &mut self.boxes, Some(&self.host), &mut self.dirs) {
            eprintln!("{line}");
        }
        while let Some(c) = app.poll_command() {
            if c == Command::Exit && !app.restart_requested() {
                event_loop.exit();
            }
        }
        if let Some(w) = self.window() {
            w.request_redraw();
        }
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
            for line in serve_with(app, &mut self.boxes, Some(&self.host), &mut self.dirs) {
                eprintln!("{line}");
            }
            // A stick pulled lets what waited under the Pull sheet carry
            // on, and a Scan turns the camera on here.
            let mut events = Vec::new();
            while let Some(c) = app.poll_command() {
                match c {
                    Command::CameraOn => events.extend(self.lens.on(app)),
                    Command::CameraOff => self.lens.off(),
                    Command::Exit if !app.restart_requested() => event_loop.exit(),
                    _ => {}
                }
            }
            self.lens.chosen(app);
            for e in events {
                self.send(event_loop, e);
            }
        }
        if let Some(w) = self.window() {
            w.request_redraw();
        }
    }

    /// Plugs the stick of folder `name` in, or pulls it out. A backup
    /// stick's folder is made the first time it is plugged in.
    fn toggle(&mut self, name: &str) {
        if self.attached.remove(name) {
            return;
        }
        if name != BOOT {
            let _ = std::fs::create_dir_all(self.sticks_dir.join(name));
        }
        self.attached.insert(name.to_string());
    }

    fn draw(&mut self) {
        let (Some(view), Some(app)) = (self.view.as_mut(), self.app.as_mut()) else {
            return;
        };
        let begun = Instant::now();
        view.draw(app, &mut self.pace);
        self.slow("frame", begun.elapsed());
    }

    /// Presses `action` as the person would, and answers what it asks.
    fn send_press(&mut self, event_loop: &ActiveEventLoop, action: Action) {
        if let Some(app) = self.app.as_mut() {
            app.press(action);
        }
        // Any event carries what the press asked for to the shell.
        let tick = self.pace.tick();
        self.send(event_loop, tick);
    }
}

/// The shell and its event loop, for the lens to deliver to.
struct Pass<'a> {
    shell: &'a mut Shell,
    event_loop: &'a ActiveEventLoop,
}

impl LensHost for Pass<'_> {
    fn lens(&mut self) -> &mut Lens {
        &mut self.shell.lens
    }

    fn send(&mut self, event: Event) {
        self.shell.send(self.event_loop, event);
    }

    fn faraday(&mut self) -> Option<&mut Faraday> {
        self.shell.app.as_mut()
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
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
                    .with_title(fill(EN.desktop_title_panel, &[&p.inches.to_string()]))
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
        let view = match View::open(event_loop, attributes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("faraday: {e}");
                event_loop.exit();
                return;
            }
        };
        let size = view.window.inner_size();
        self.shown = (size.width, size.height);
        self.scale = view.window.scale_factor();
        self.pace.paced = view.paced();
        self.view = Some(view);
        let display = self.display();
        self.send(event_loop, display);
        self.rescan(event_loop, true);
        self.relist();
        if std::mem::take(&mut self.open_transfer) {
            self.send_press(event_loop, Action::Nav(Screen::Transfer));
        }
        self.pace.next_tick = Instant::now() + TICK;
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            // Only X11 reports a drop in winit 0.30; the list is the way
            // on native Wayland.
            WindowEvent::DroppedFile(path) => self.dropped(event_loop, &path),
            WindowEvent::RedrawRequested => {
                // With the compositor pacing the frames, this is the
                // moment before the screen's next refresh: the tick moves
                // whatever moves to now, and the frame is drawn for the
                // moment it will be seen. A tick that asks for a frame
                // asks for the next redraw, which comes with the refresh
                // after; the timer stays as the idle tick, pushed back so
                // it does not tick between two refreshes.
                if self.pace.paced {
                    let tick = self.pace.tick();
                    self.send(event_loop, tick);
                    self.pace.next_tick = Instant::now() + TICK;
                }
                self.draw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                // A device's screen stays the same size in this screen's
                // pixels at any scale.
                if let (Some(p), Some(w)) = (self.panel, self.window()) {
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
                if let Some(w) = self.window() {
                    w.request_redraw();
                }
            }
            // The mouse, a finger, the wheel and the keyboard, as the
            // shell API's events (`faraday-window`).
            other => {
                for e in self.pointer.translate(&other) {
                    self.send(event_loop, e);
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_none() {
            return;
        }
        // The newest camera frame, to the app to draw and to the scanner
        // to read; and whatever the scanner has read since.
        Lens::pump(&mut Pass {
            shell: self,
            event_loop,
        });
        let now = Instant::now();
        if now >= self.pace.next_tick {
            let tick = self.pace.tick();
            self.send(event_loop, tick);
            let refresh = view::refresh(self.window());
            self.pace.ticked_at(now, refresh);
        }
        self.relist();
        if now >= self.next_scan {
            self.rescan(event_loop, false);
            self.next_scan = now + RESCAN;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.lens.off();
        self.view = None;
        self.app = None;
    }
}

/// What Transfer lists of this computer: the Downloads folder and the
/// cameras.
#[derive(Clone, PartialEq)]
struct Listing {
    files: StorageEvent,
    cameras: Vec<(String, String)>,
}

/// Whether `now` is to be sent: it differs from the last one sent (the
/// first always is). A lookup.
fn is_new(last: &mut Option<Listing>, now: &Listing) -> bool {
    if last.as_ref() == Some(now) {
        return false;
    }
    *last = Some(now.clone());
    true
}

/// Costs a directory listing with each file's metadata and a camera
/// enumeration (Media Foundation's on Windows) every [`RESCAN`], on a
/// thread of its own so the window's thread never waits on either; sends
/// the listing when it changed, the first at once.
fn watch_listing(downloads: PathBuf, opens: bool, out: Sender<Listing>) {
    std::thread::spawn(move || {
        let host = Host {
            print: PathBuf::new(),
            downloads,
            opens,
        };
        let mut last = None;
        loop {
            let now = Listing {
                files: host.files(),
                cameras: camera::list(),
            };
            if is_new(&mut last, &now) && out.send(now).is_err() {
                return;
            }
            std::thread::sleep(RESCAN);
        }
    });
}

/// A fresh app as the desktop runs it: the online Faraday, which turns
/// sheets into PDFs, in `mode`. Every build starts in Demo (the key
/// store's public set, on testnet); a lock starts the fresh session in
/// the mode the old one chose or was in. No warning comes first: the
/// steps that take a seed carry a line instead (F81).
fn desktop_app_in(mode: faraday_core::Mode) -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    // This shell answers `RequestClipboard` (faraday_window::clipboard).
    app.clipboard = true;
    app.shell_sticks = true;
    app.session = faraday_core::wallet::Session::on(mode.network());
    if mode == faraday_core::Mode::Demo {
        app.session.set_demo(&faraday_samples::SAMPLES);
    }
    app.set_modes(faraday_core::DESKTOP_MODES);
    app
}

/// Makes the stick folders the first time: `faraday-sticks`, an empty
/// boot stick and an empty Stick 2. The boot stick starts empty and the
/// desktop never adds to it: a person puts there the files to boot with
/// (owner, 2026-10-19). Nothing in a folder is ever moved, renamed or
/// removed here, so a person's sticks stay as they left them across
/// installs and new versions (owner, 2026-10-20). A boot stick folder
/// renamed or removed is not made again: the app then starts with no
/// stick in. Stick 2 is a stick the app starts with, made if it is not
/// there.
fn prepare(dir: &Path) -> Result<(), String> {
    let boot = dir.join(BOOT);
    let first_stick = dir.join(FIRST);
    // Made once: the mark says the folders have been made here.
    let made = dir.join(".faraday-sticks");
    if !made.exists() {
        std::fs::create_dir_all(&boot).map_err(|e| format!("{}: {e}", boot.display()))?;
        std::fs::write(&made, "BOOTSTICK and STICK-2 made\n")
            .map_err(|e| format!("{}: {e}", made.display()))?;
    }
    std::fs::create_dir_all(&first_stick).map_err(|e| format!("{}: {e}", first_stick.display()))
}

/// The person's home folder: `HOME`, and on Windows `USERPROFILE` when
/// `HOME` is not set. Downloads is `Downloads` in it; a Downloads folder
/// that Windows has moved elsewhere is not followed.
fn home() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty());
    #[cfg(windows)]
    let home = home.or_else(|| std::env::var_os("USERPROFILE").filter(|h| !h.is_empty()));
    home.map(PathBuf::from)
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn main() -> ExitCode {
    // `--version` answers before anything else is initialised: the
    // Starter asks an installed copy this (docs/STARTER.md §5.2).
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!(
            "{}",
            faraday_core::version_line(
                faraday_core::Program::DesktopApp,
                faraday_core::BUILD.unwrap_or(faraday_core::VERSION)
            )
        );
        return ExitCode::SUCCESS;
    }
    let mut sticks_dir = home()
        .map(|h| h.join("faraday-sticks"))
        .unwrap_or_else(|| PathBuf::from("faraday-sticks"));
    let mut size = (1280, 800);
    let mut panel: Option<f64> = None;
    let mut aspect: Option<(f64, f64)> = None;
    let mut ppi: Option<f64> = None;
    let mut open_transfer = false;
    let mut timings = false;
    let mut downloads = home()
        .map(|h| h.join("Downloads"))
        .unwrap_or_else(|| PathBuf::from("Downloads"));
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
            "--transfer" => open_transfer = true,
            "--timings" => timings = true,
            "--downloads" => match args.next() {
                Some(d) => downloads = PathBuf::from(d),
                None => {
                    eprintln!("faraday: --downloads needs a directory");
                    return ExitCode::from(2);
                }
            },
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
                    "faraday [--sticks DIR] [--size WxH] [--transfer] [--downloads DIR] [--panel INCHES [--aspect W:H] [--ppi N]] [--timings]\n  the sticks are the pills at the top of the window\n  --transfer: open on Transfer · --downloads: the folder Transfer sends from and saves into (default ~/Downloads)\n  --panel: a device's screen at its true size: 2.8 is the Pi's panel (3:4), 5 a phone (9:16)\n  --aspect: the panel's width to height · --ppi: this screen's pixels per inch, if its EDID is wrong\n  --timings: once a second, what the frames cost (render, copy to the window, frame to frame) on stderr"
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
    if let Err(e) = prepare(&sticks_dir) {
        eprintln!("faraday: {e}");
        return ExitCode::FAILURE;
    }
    eprintln!("faraday: sticks are folders in {}", sticks_dir.display());
    // The boot stick is in at start, as the stick the device booted from
    // is; none when the folder has been renamed or removed. Every other
    // stick is out until its pill is clicked.
    let mut attached = BTreeSet::new();
    if sticks_dir.join(BOOT).is_dir() {
        attached.insert(BOOT.to_string());
    }
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("faraday: no display to open a window on: {e}");
            return ExitCode::FAILURE;
        }
    };
    let print_dir = home()
        .map(|h| h.join("faraday-print"))
        .unwrap_or_else(|| PathBuf::from("faraday-print"));
    let app = desktop_app_in(faraday_core::Mode::Demo);
    let pace = Pacer::new(timings);
    let next_scan = pace.epoch;
    let opens = faraday_storage::can_open_folders();
    let (sender, listing) = channel();
    watch_listing(downloads.clone(), opens, sender);
    let mut shell = Shell {
        panel,
        host: Host {
            print: print_dir,
            downloads,
            opens,
        },
        listing,
        latest: None,
        listed: None,
        cameras: None,
        open_transfer,
        sticks_dir,
        size,
        app: Some(app),
        boxes: Boxes::Memory {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: Vec::new(),
        },
        dirs: Dirs::default(),
        attached,
        seen: Vec::new(),
        view: None,
        shown: size,
        scale: 1.0,
        pace,
        next_scan,
        last_slow: None,
        pointer: Pointer::new(),
        lens: Lens::new(Camera::new(None, false)),
        clipboard: Clipboard::system(),
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

    /// The boot stick starts empty and the desktop adds nothing to it;
    /// what a person put in the stick folders, hidden files included, is
    /// left byte for byte by every later start (owner, 2026-10-20).
    #[test]
    fn a_persons_stick_folders_stay_as_they_left_them() {
        let dir = std::env::temp_dir().join(format!("faraday-prepare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        prepare(&dir).unwrap();
        let boot = dir.join(BOOT);
        let two = dir.join(FIRST);
        assert_eq!(std::fs::read_dir(&boot).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&two).unwrap().count(), 0);
        let files = [
            (boot.join("vault.ofv"), b"vault bytes".to_vec()),
            (boot.join(".faraday-testkit"), b"an old mark".to_vec()),
            (boot.join("spend.psbt"), b"psbt".to_vec()),
            (two.join("SHA256SUMS"), b"sums".to_vec()),
            (dir.join("STICK-3").join("notes.txt"), b"notes".to_vec()),
        ];
        for (path, bytes) in &files {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        let listing = |d: &Path| {
            let mut names: Vec<_> = std::fs::read_dir(d)
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            names.sort();
            names
        };
        let before: Vec<_> = [&dir, &boot, &two].iter().map(|d| listing(d)).collect();
        prepare(&dir).unwrap();
        prepare(&dir).unwrap();
        let after: Vec<_> = [&dir, &boot, &two].iter().map(|d| listing(d)).collect();
        assert_eq!(before, after);
        for (path, bytes) in &files {
            assert_eq!(&std::fs::read(path).unwrap(), bytes, "{}", path.display());
        }
        // A boot stick removed is not made again.
        std::fs::remove_dir_all(&boot).unwrap();
        prepare(&dir).unwrap();
        assert!(!boot.exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The desktop opens on testnet, which takes any seed, with no
    /// warning to acknowledge; mainnet, chosen from Network, is still the
    /// demo that takes only public seeds (F20).
    /// The listing thread tells the window only of a change.
    #[test]
    fn a_listing_is_sent_only_when_it_changed() {
        let mut last = None;
        let mut now = Listing {
            files: StorageEvent::Cameras(Vec::new()),
            cameras: Vec::new(),
        };
        assert!(is_new(&mut last, &now));
        assert!(!is_new(&mut last, &now));
        now.cameras.push(("0".into(), "Webcam".into()));
        assert!(is_new(&mut last, &now));
        assert!(!is_new(&mut last, &now));
    }

    #[test]
    fn the_desktop_opens_in_demo_on_testnet_and_a_mode_change_locks() {
        let c = faraday_core::testkit::compute();
        let mut app = desktop_app_in(faraday_core::Mode::Demo);
        assert_eq!(app.mode(), faraday_core::Mode::Demo);
        assert_eq!(app.session.network(), faraday_core::testkit::NET);
        let other = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
        assert!(app.session.keys.add_words(&c, other, "", None).is_err());
        let public = faraday_core::testkit::test_words("zebra");
        assert!(app.session.keys.add_words(&c, &public, "", None).is_ok());
        app.press(faraday_core::Action::Mode(faraday_core::Mode::Signet));
        assert_eq!(app.mode_after_restart(), faraday_core::Mode::Signet);
        assert!(app.restart_requested());
    }

    /// Every build opens in Demo with no sheet; a lock keeps the mode,
    /// and a mode chosen is where the fresh session starts.
    #[test]
    fn every_build_opens_in_demo_with_no_sheet() {
        let app = desktop_app_in(faraday_core::Mode::Demo);
        assert!(app.sheet.is_none());
        assert_eq!(app.mode(), faraday_core::Mode::Demo);
        assert_eq!(app.session.network(), faraday_core::testkit::NET);
    }

    #[test]
    fn leaving_demo_for_testnet_starts_a_fresh_testnet_session() {
        let mut app = desktop_app_in(faraday_core::Mode::Demo);
        app.press(faraday_core::Action::Mode(faraday_core::Mode::Testnet));
        let fresh = desktop_app_in(app.mode_after_restart());
        assert!(fresh.sheet.is_none());
        assert_eq!(fresh.mode(), faraday_core::Mode::Testnet);
        assert!(!fresh.session.keys.demo());
        let locked = desktop_app_in(fresh.mode_after_restart());
        assert_eq!(locked.mode(), faraday_core::Mode::Testnet);
    }
}
