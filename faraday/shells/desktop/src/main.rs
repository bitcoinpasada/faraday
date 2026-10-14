//! Faraday in a window.
//!
//! ```text
//! faraday [--sticks DIR] [--size WxH] [--full-kit] [--transfer]
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
//! holds `BOOTSTICK`, which gets every test kit file it lacks at each
//! start and never has a file overwritten, and `STICK-2`, a second stick,
//! which starts empty (until 2026-10-11 they were `TESTSTICK` and
//! `BLANK`, renamed at the next start). The boot stick holds the backup
//! test kit (`faraday_core::testkit::backup_files`: a 2-of-3 Taproot
//! multisig's public backup files and a vault, passphrase `a`, holding
//! its three seeds); `--full-kit` makes it the full test kit instead. A
//! folder made for the other kit is moved aside, not mixed. In a
//! release build the boot stick starts empty, as the stick image's does,
//! and nothing in either folder is added to or moved. The two folders
//! are made once, the first time; one renamed or removed later is not
//! made again, and with no `BOOTSTICK` the app starts with no stick in,
//! as it did before 2026-10-11. The boot
//! stick is plugged in when Faraday opens, as the device starts with the
//! stick it booted from: its `faraday-settings.txt` is read and every
//! other file on it is copied into memory for the boot import, and
//! pulling it (F2) starts the import. Plugged in again later, it is
//! visited like any stick.
//!
//! - F2 plugs the boot stick in, or pulls it out;
//! - F3 does the same for stick 2;
//! - F4 pulls every stick out.
//!
//! Any other folder in `DIR` can be added by hand and plugged with F5,
//! which attaches every folder there. What the app writes to a stick is a
//! file in that folder.
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
//! Every session here starts on testnet, the test stick's network; the
//! stick's app starts on mainnet.
//!
//! Locking ends the app's process on the device; here it replaces the app
//! with a fresh one in the same window, with the Inbox and Outbox carried
//! over, which is what the device's app loop does.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use faraday_core::strings::{EN, fill};
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageEvent};
use faraday_storage::{Boxes, Host, serve, stick_info};
use faraday_window::camera::{self, Camera};
use faraday_window::entropy;
use faraday_window::input::{Input, Pointer};
use faraday_window::lens::{Lens, LensHost};
use faraday_window::pace::{Pacer, TICK};
use faraday_window::view::{self, View};
use osk_shell_api::{App, Command, Event};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// How often the attached folders are looked at again.
const RESCAN: Duration = Duration::from_millis(700);
const TITLE: &str = EN.desktop_title;
const TITLE_KEYS: &str = EN.desktop_title_keys;
/// The keys, pinned at the top of the window in small text.
const KEYS_LINE: &str = EN.desktop_keys_line;
/// The two stick folders.
const BOOT: &str = "BOOTSTICK";
const SECOND: &str = "STICK-2";

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
    /// The Downloads folder as last listed to the app.
    listed: Option<StorageEvent>,
    /// Open on Transfer once the window is up.
    open_transfer: bool,
    size: (u32, u32),
    app: Option<Faraday>,
    boxes: Boxes,
    attached: BTreeSet<String>,
    seen: Vec<StickInfo>,
    /// The window and its surface (`faraday-window`).
    view: Option<View>,
    shown: (u32, u32),
    /// The window's scale factor.
    scale: f64,
    /// The ticks and the frames' pace (`faraday-window`).
    pace: Pacer,
    next_scan: Instant,
    /// The mouse, a finger and the keyboard (`faraday-window`).
    pointer: Pointer,
    /// The webcam, when the app asks for it, and the scanner that reads
    /// its frames on its own thread (`faraday-window`).
    lens: Lens,
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
        let ticked = matches!(event, Event::Tick { .. });
        self.pace.before(&event);
        let mut draw = false;
        let mut events = vec![event];
        while let Some(e) = events.pop() {
            let Some(app) = self.app.as_mut() else { return };
            app.event(e);
            while let Some(c) = app.poll_command() {
                match c {
                    Command::Draw => draw = true,
                    Command::RequestEntropy => events.extend(entropy()),
                    Command::CameraOn => events.extend(self.lens.on(app)),
                    Command::CameraOff => self.lens.off(),
                    Command::Exit => {
                        if app.restart_requested() {
                            for line in serve(app, &mut self.boxes, Some(&self.host)) {
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
            self.lens.chosen(app);
            for line in serve(app, &mut self.boxes, Some(&self.host)) {
                eprintln!("{line}");
            }
        }
        if ticked {
            self.pace.animating = draw;
        }
        // A write changes a stick's files: look again now.
        self.rescan(event_loop, false);
        if draw && let Some(w) = self.window() {
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
        let tick = self.pace.tick();
        self.send(event_loop, tick);
        self.rescan(event_loop, true);
        self.listed = None;
        self.relist();
    }

    /// Tells the app what Downloads holds, when that changed.
    fn relist(&mut self) {
        let now = self.host.files();
        if self.listed.as_ref() == Some(&now) {
            return;
        }
        if let Some(app) = self.app.as_mut() {
            app.storage(now.clone());
            self.listed = Some(now);
            if let Some(w) = self.window() {
                w.request_redraw();
            }
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
        for line in serve(app, &mut self.boxes, Some(&self.host)) {
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
            // Which cameras there are, so a screen can offer the camera
            // or do without it.
            app.storage(StorageEvent::Cameras(camera::list()));
            for line in serve(app, &mut self.boxes, Some(&self.host)) {
                eprintln!("{line}");
            }
            while let Some(c) = app.poll_command() {
                if c == Command::Exit && !app.restart_requested() {
                    event_loop.exit();
                }
            }
        }
        if let Some(w) = self.window() {
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
        let (Some(view), Some(app)) = (self.view.as_mut(), self.app.as_mut()) else {
            return;
        };
        view.draw(app, &mut self.pace);
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

    /// A function key: F2 plugs the boot stick in or pulls it out, F3
    /// stick 2, F4 pulls every stick, F5 plugs every folder in.
    fn function(&mut self, event_loop: &ActiveEventLoop, n: u8) {
        match n {
            2 => self.toggle(event_loop, BOOT),
            3 => self.toggle(event_loop, SECOND),
            4 => {
                self.attached.clear();
                self.rescan(event_loop, false);
            }
            5 => {
                if let Ok(d) = std::fs::read_dir(&self.sticks_dir) {
                    for e in d.flatten() {
                        if e.path().is_dir() {
                            self.attached
                                .insert(e.file_name().to_string_lossy().into_owned());
                        }
                    }
                }
                self.rescan(event_loop, false);
            }
            _ => {}
        }
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
                    .with_title(fill(
                        EN.desktop_title_panel,
                        &[&p.inches.to_string(), TITLE_KEYS],
                    ))
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
                for input in self.pointer.translate(&other) {
                    match input {
                        Input::Event(e) => self.send(event_loop, e),
                        Input::Function(n) => self.function(event_loop, n),
                    }
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
        if now >= self.next_scan {
            self.rescan(event_loop, false);
            self.relist();
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

/// A fresh app as the desktop runs it: the online Faraday, which turns
/// sheets into PDFs, on mainnet as the demo (F20): there it takes only
/// the public seeds and shows every address with a broken checksum. A
/// test network takes any seed, after the not-air-gapped warning.
fn desktop_app() -> Faraday {
    let mut app = Faraday::new();
    app.online = true;
    app.shell_keys = Some(KEYS_LINE.to_string());
    app.session = faraday_core::wallet::Session::default();
    app.session.set_demo();
    app
}

/// Which test kit a test stick folder holds. A folder from another kit
/// (the mainnet one before this) is moved aside, not mixed with this one.
const KIT_MARK: &str = "faraday-testkit testnet 1\n";
/// The mark of the backup test stick.
const BACKUP_MARK: &str = "faraday-testkit backup 1\n";

/// Makes the two stick folders the first time, and fills the boot stick
/// with the test kit while there is one. A folder renamed or removed is
/// not made again: with no boot stick, the app starts with no stick in.
/// A release's boot stick starts empty, as the stick image's does; it
/// gets the test kit only when `--full-kit` asks for it.
fn prepare(dir: &Path, full: bool) -> Result<(), String> {
    let test = dir.join(BOOT);
    let blank = dir.join(SECOND);
    // Made once: the mark says the folders have been made here.
    let made = dir.join(".faraday-sticks");
    let first = !made.exists();
    if first {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        // The folders' names before 2026-10-11, taken over as they are.
        for (old, new) in [("TESTSTICK", &test), ("BLANK", &blank)] {
            let old = dir.join(old);
            if old.is_dir() && !new.exists() {
                std::fs::rename(&old, new).map_err(|e| format!("{}: {e}", old.display()))?;
            }
        }
        std::fs::create_dir_all(&blank).map_err(|e| format!("{}: {e}", blank.display()))?;
        std::fs::write(&made, "BOOTSTICK and STICK-2 made\n")
            .map_err(|e| format!("{}: {e}", made.display()))?;
    } else if !test.is_dir() {
        return Ok(());
    }
    if faraday_core::is_release() && !full {
        // What is in it is the person's own: nothing added, nothing moved.
        return std::fs::create_dir_all(&test).map_err(|e| format!("{}: {e}", test.display()));
    }
    let (kit_mark, files) = if full {
        (KIT_MARK, faraday_core::testkit::files()?)
    } else {
        (BACKUP_MARK, faraday_core::testkit::backup_files()?)
    };
    let mark = test.join(".faraday-testkit");
    if test.exists() && std::fs::read_to_string(&mark).ok().as_deref() != Some(kit_mark) {
        let aside = (1..)
            .map(|n| dir.join(format!("{BOOT}-old-{n}")))
            .find(|p| !p.exists())
            .expect("a free name");
        std::fs::rename(&test, &aside).map_err(|e| format!("{}: {e}", test.display()))?;
        eprintln!(
            "faraday: the old boot stick folder is now {}",
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

/// The person's home folder: `HOME`, and on Windows `USERPROFILE` when
/// `HOME` is not set. Downloads is `Downloads` in it; a Downloads folder
/// that Windows has moved elsewhere is not followed.
fn home() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty());
    #[cfg(windows)]
    let home = home.or_else(|| std::env::var_os("USERPROFILE").filter(|h| !h.is_empty()));
    home.map(PathBuf::from)
}

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
    let mut full_kit = false;
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
            "--full-kit" => full_kit = true,
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
                    "faraday [--sticks DIR] [--size WxH] [--full-kit] [--transfer] [--downloads DIR] [--panel INCHES [--aspect W:H] [--ppi N]] [--timings]\n  F2 boot stick, F3 stick 2, F4 pull all, F5 plug every folder in DIR\n  --full-kit: the boot stick carries the full test kit, not the backup test kit\n  --transfer: open on Transfer · --downloads: the folder Transfer sends from and saves into (default ~/Downloads)\n  --panel: a device's screen at its true size: 2.8 is the Pi's panel (3:4), 5 a phone (9:16)\n  --aspect: the panel's width to height · --ppi: this screen's pixels per inch, if its EDID is wrong\n  --timings: once a second, what the frames cost (render, copy to the window, frame to frame) on stderr"
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
    // The boot stick is in at start, as the stick the device booted from
    // is; none when the folder has been renamed or removed.
    let attached = if sticks_dir.join(BOOT).is_dir() {
        BTreeSet::from([BOOT.to_string()])
    } else {
        BTreeSet::new()
    };
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
    let app = desktop_app();
    let pace = Pacer::new(timings);
    let next_scan = pace.epoch;
    let mut shell = Shell {
        panel,
        host: Host {
            print: print_dir,
            downloads,
            opens: faraday_storage::can_open_folders(),
        },
        listed: None,
        open_transfer,
        sticks_dir,
        size,
        app: Some(app),
        boxes: Boxes::Memory {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: Vec::new(),
        },
        attached,
        seen: Vec::new(),
        view: None,
        shown: size,
        scale: 1.0,
        pace,
        next_scan,
        pointer: Pointer::new(),
        lens: Lens::new(Camera::new(None, false)),
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

    /// The desktop starts on mainnet as the demo (F20): a public seed
    /// loads and the words of any other key are refused; on testnet the
    /// same words load.
    #[test]
    fn the_desktop_starts_on_mainnet_taking_only_public_seeds() {
        let mut app = desktop_app();
        assert!(app.session.network().is_mainnet());
        let public = faraday_core::demo::words("zebra");
        assert!(app.session.add_words(&public, "", None).is_ok());
        let other = "abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon about";
        assert!(app.session.add_words(other, "", None).is_err());
        let mut app = desktop_app();
        app.session.set_network(faraday_core::testkit::NET).unwrap();
        assert!(app.session.add_words(other, "", None).is_ok());
    }
}
