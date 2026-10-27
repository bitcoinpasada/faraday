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
//! winit reports the drop: on X11, not on native Wayland; while one is
//! dragged over the window the page is edged as the place it drops. Nothing
//! received goes into the app's Inbox.
//!
//! Every session here opens in Practice, on testnet, in every build; the
//! pill and Settings choose Testnet, Signet or Regtest, and a lock
//! keeps the mode (F81). No warning comes first: every step that takes
//! a seed carries a line saying never to type one that holds real
//! bitcoin.
//! The stick's app starts on mainnet.
//!
//! Locking ends the app's process on the device; here it replaces the app
//! with a fresh one in the same window, with the Inbox and Outbox carried
//! over, which is what the device's app loop does.
//!
//! The app is held by Faraday Demo's host (`faraday_core::Host`,
//! `docs/DESKTOP-HOST.md`): a strip at the window's foot holds the stick
//! pills, Samples, the preview size and the screen's demo aids, outside
//! the app, and a preview size frames the app as that screen lays it
//! out. A lock keeps the size; a new launch opens at the window's own.
//! With `--panel`, whose window is already a device's screen, the app is
//! held bare: no strip and no preview.
//!
//! This window is the developer's (`cargo run -p faraday-desktop`): people
//! open Faraday Demo inside Faraday Starter's window (F126,
//! `docs/STARTER.md` §5.3). What the Demo does apart from the window is
//! `faraday_desktop::Demo`, which both hold.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use faraday_core::strings::{EN, fill};
use faraday_core::{Action, Faraday, Host as AppHost, Screen};
use faraday_desktop::{Asked, Demo, Paths};
use faraday_shell_api::{App, Command, Event};
use faraday_window::camera::Camera;
use faraday_window::drag::TitleDrag;
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

/// A frame or an event slower than this is reported in a test build.
const SLOW: Duration = Duration::from_millis(50);
const TITLE: &str = EN.desktop_title;

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
    /// A device's screen to stand for, when given.
    panel: Option<Panel>,
    /// The Demo's side of the window: sticks, storage, Downloads, the
    /// clipboard and the preview size.
    demo: Demo,
    /// Open on Transfer once the window is up.
    open_transfer: bool,
    size: (u32, u32),
    /// The app, held by Faraday Demo's host (`docs/DESKTOP-HOST.md`).
    app: Option<AppHost>,
    /// The window and its surface (`faraday-window`).
    view: Option<View>,
    shown: (u32, u32),
    /// The window's scale factor.
    scale: f64,
    /// The ticks and the frames' pace (`faraday-window`).
    pace: Pacer,
    /// When a slow frame or event was last printed.
    last_slow: Option<Instant>,
    /// The mouse, a finger and the keyboard (`faraday-window`).
    pointer: Pointer,
    /// A press on the top strip's empty room that may move the window
    /// (`faraday-window`).
    title: TitleDrag,
    /// The webcam, when the app asks for it, and the scanner that reads
    /// its frames on its own thread (`faraday-window`).
    lens: Lens,
}

impl Shell {
    /// The window's own display: the host lays the app out in it.
    fn display(&self) -> Event {
        view::display(self.shown, self.scale, self.panel.map(|p| p.ppi))
    }

    /// What the app is told of the preview: the size shown, or nothing
    /// with `--panel`, where none is offered.
    fn offered(&self) -> Option<Event> {
        self.panel.is_none().then(|| self.demo.offered())
    }

    /// `app` held as this window shows it: by the host with its strip, or
    /// bare with `--panel`, whose window is a device's screen.
    fn held(&self, app: Faraday) -> AppHost {
        if self.panel.is_some() {
            AppHost::bare(app)
        } else {
            AppHost::new(app)
        }
    }

    fn window(&self) -> Option<&Window> {
        self.view.as_ref().map(|v| &*v.window)
    }

    fn redraw(&self) {
        if let Some(w) = self.window() {
            w.request_redraw();
        }
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
        let Some(app) = self.app.as_ref().map(|h| &h.app) else {
            return;
        };
        self.last_slow = Some(now);
        eprintln!(
            "faraday: slow {what} {} ms · {}",
            took.as_millis(),
            app.screen_name()
        );
    }

    /// Delivers `event`; returns the time spent on storage.
    fn deliver(&mut self, event_loop: &ActiveEventLoop, event: Event) -> Duration {
        let mut storage = Duration::ZERO;
        let ticked = matches!(event, Event::Tick { .. });
        self.pace.before(&event);
        let mut draw = false;
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
                    Command::CameraOn => events.extend(self.lens.on(&mut app.app)),
                    Command::CameraOff => self.lens.off(),
                    other => match self.demo.command(&mut app.app, other) {
                        Asked::Nothing => {}
                        Asked::Event(e) => events.push(e),
                        Asked::Restart => {
                            self.restart(event_loop);
                            return Duration::ZERO;
                        }
                        Asked::Exit => {
                            event_loop.exit();
                            return Duration::ZERO;
                        }
                    },
                }
            }
            // A camera the person chose on the scan sheet.
            self.lens.chosen(&mut app.app);
            storage += self.demo.took(&mut app.app);
        }
        if ticked {
            self.pace.animating = draw;
        }
        // A stick toggled or a storage command served changes a stick's
        // files: look again now. Otherwise the timer looks.
        let at = Instant::now();
        if let Some(app) = self.app.as_mut()
            && self.demo.settle(&mut app.app)
        {
            self.after_rescan(event_loop);
        }
        storage += at.elapsed();
        self.pace.storage_took(storage);
        if draw {
            self.redraw();
        }
        storage
    }

    fn restart(&mut self, event_loop: &ActiveEventLoop) {
        let Some(old) = self.app.as_ref() else {
            return;
        };
        let fresh = self.demo.restart(&old.app);
        self.app = Some(self.held(fresh));
        let display = self.display();
        let offered = self.offered();
        if let Some(app) = self.app.as_mut() {
            app.event(display);
            if let Some(o) = offered {
                app.event(o);
            }
            self.demo.restarted(&mut app.app);
        }
        let tick = self.pace.tick();
        self.send(event_loop, tick);
        self.after_rescan(event_loop);
    }

    /// After the sticks were looked at: a stick pulled lets what waited
    /// under the Pull sheet carry on, and a Scan turns the camera on here.
    fn after_rescan(&mut self, event_loop: &ActiveEventLoop) {
        let Some(app) = self.app.as_mut().map(|h| &mut h.app) else {
            return;
        };
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
        self.redraw();
    }

    /// The newest Downloads listing and cameras, to the app.
    fn relist(&mut self) {
        if let Some(app) = self.app.as_mut().map(|h| &mut h.app)
            && self.demo.relist(app)
        {
            self.redraw();
        }
    }

    /// A file dragged over the window (`over`), or the drag gone.
    fn hovered(&mut self, over: bool) {
        let Some(app) = self.app.as_mut().map(|h| &mut h.app) else {
            return;
        };
        app.storage(faraday_core::StorageEvent::Hovered(over));
        self.redraw();
    }

    /// A file dropped on the window, for Transfer to send.
    fn dropped(&mut self, event_loop: &ActiveEventLoop, path: &Path) {
        let Some(app) = self.app.as_mut().map(|h| &mut h.app) else {
            return;
        };
        self.demo.dropped(app, path);
        while let Some(c) = app.poll_command() {
            if c == Command::Exit && !app.restart_requested() {
                event_loop.exit();
            }
        }
        self.redraw();
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
            app.app.press(action);
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
        self.shell.app.as_mut().map(|h| &mut h.app)
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
        if let Some(o) = self.offered() {
            self.send(event_loop, o);
        }
        if let Some(app) = self.app.as_mut() {
            self.demo.rescan(&mut app.app, true);
        }
        self.after_rescan(event_loop);
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
            // A file dragged over the window edges the page as the place
            // it drops, until it is dropped or the drag leaves.
            WindowEvent::HoveredFile(_) => self.hovered(true),
            WindowEvent::HoveredFileCancelled => self.hovered(false),
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
                    // The host lays the app out again, or moves a
                    // preview's frame.
                    let display = self.display();
                    self.send(event_loop, display);
                }
                self.redraw();
            }
            // The mouse, a finger, the wheel and the keyboard, as the
            // shell API's events (`faraday-window`).
            // In a preview, carried into its frame.
            other => {
                // A press on the top strip's empty room and a drag move
                // the window, a double-click maximises it; not in a
                // preview, whose window is a frame for another size; nor
                // the press the system makes of a finger's.
                if self.demo.preview().is_none()
                    && !self.pointer.is_echo(&other)
                    && let (Some(view), Some(app)) = (self.view.as_ref(), self.app.as_ref())
                    && self
                        .title
                        .takes(&other, self.pointer.cursor, &view.window, |x, y| {
                            app.app.is_strip_room(x, y)
                        })
                {
                    return;
                }
                // The host carries the pointer into the app's frame.
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
        if self.demo.due(now)
            && let Some(app) = self.app.as_mut()
            && self.demo.rescan(&mut app.app, false)
        {
            self.after_rescan(event_loop);
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.lens.off();
        self.view = None;
        self.app = None;
    }
}

/// The command line's options.
struct Options {
    paths: Paths,
    size: (u32, u32),
    panel: Option<Panel>,
    open_transfer: bool,
    timings: bool,
}

const HELP: &str = "faraday [--sticks DIR] [--size WxH] [--transfer] [--downloads DIR] [--panel INCHES [--aspect W:H] [--ppi N]] [--timings]\n  the sticks are the pills at the top of the window\n  --transfer: open on Transfer · --downloads: the folder Transfer sends from and saves into (default ~/Downloads)\n  --panel: a device's screen at its true size: 2.8 is the Pi's panel (3:4), 5 a phone (9:16)\n  --aspect: the panel's width to height · --ppi: this screen's pixels per inch, if its EDID is wrong\n  --timings: once a second, what the frames cost (render, copy to the window, frame to frame) on stderr";

/// Prints `why` and gives the exit code of a command line not understood.
fn refused(why: &str) -> ExitCode {
    eprintln!("faraday: {why}");
    ExitCode::from(2)
}

/// Reads the command line; `Err` is the exit code, the reason printed.
fn options() -> Result<Options, ExitCode> {
    let mut paths = Paths::in_home();
    let mut size = (1280, 800);
    let (mut panel, mut aspect, mut ppi) = (None, None, None);
    let (mut open_transfer, mut timings) = (false, false);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut dir = |what: &str| {
            args.next().map(PathBuf::from).ok_or_else(|| {
                eprintln!("faraday: {what} needs a directory");
                ExitCode::from(2)
            })
        };
        match a.as_str() {
            "--sticks" => paths.sticks = dir("--sticks")?,
            "--downloads" => paths.downloads = dir("--downloads")?,
            "--transfer" => open_transfer = true,
            "--timings" => timings = true,
            "--panel" => match args
                .next()
                .and_then(|s| s.trim_end_matches("in").parse::<f64>().ok())
            {
                Some(i) if (1.0..=13.0).contains(&i) => panel = Some(i),
                _ => return Err(refused("--panel needs a diagonal in inches, 1 to 13")),
            },
            "--aspect" => {
                let parsed = args.next().and_then(|s| {
                    let (w, h) = s.split_once(':')?;
                    Some((w.parse::<f64>().ok()?, h.parse::<f64>().ok()?))
                });
                match parsed {
                    Some((w, h)) if w > 0.0 && h > 0.0 => aspect = Some((w, h)),
                    _ => return Err(refused("--aspect needs W:H, such as 3:4")),
                }
            }
            "--ppi" => match args.next().and_then(|s| s.parse::<f64>().ok()) {
                Some(p) if (50.0..=1000.0).contains(&p) => ppi = Some(p),
                _ => return Err(refused("--ppi needs this screen's pixels per inch")),
            },
            "--size" => {
                let parsed = args.next().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                });
                size = parsed.ok_or_else(|| refused("--size needs WxH"))?;
            }
            "--help" | "-h" => {
                println!("{HELP}");
                return Err(ExitCode::SUCCESS);
            }
            other => {
                eprintln!("faraday: unknown argument {other}");
                return Err(ExitCode::from(2));
            }
        }
    }
    let panel = match panel {
        Some(inches) => {
            let Some(ppi) = ppi.or_else(screen_ppi) else {
                eprintln!("faraday: this screen's pixels per inch are unknown: give --ppi");
                return Err(ExitCode::from(2));
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
    Ok(Options {
        paths,
        size,
        panel,
        open_transfer,
        timings,
    })
}

fn main() -> ExitCode {
    // `--version` answers before anything else is initialised: the
    // Starter asks an earlier install's copy this (docs/STARTER.md §5.2).
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
    let o = match options() {
        Ok(o) => o,
        Err(code) => return code,
    };
    let sticks = o.paths.sticks.clone();
    let demo = match Demo::open(o.paths) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("faraday: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("faraday: sticks are folders in {}", sticks.display());
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("faraday: no display to open a window on: {e}");
            return ExitCode::FAILURE;
        }
    };
    let app = faraday_desktop::desktop_app_in(faraday_core::Mode::Demo);
    let mut shell = Shell {
        panel: o.panel,
        demo,
        open_transfer: o.open_transfer,
        size: o.size,
        app: Some(if o.panel.is_some() {
            AppHost::bare(app)
        } else {
            AppHost::new(app)
        }),
        view: None,
        shown: o.size,
        scale: 1.0,
        pace: Pacer::new(o.timings),
        last_slow: None,
        pointer: Pointer::new(),
        title: TitleDrag::default(),
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
