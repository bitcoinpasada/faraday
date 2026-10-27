//! The Starter (`docs/STARTER.md`): Faraday's screens for making a stick,
//! starting this computer from it, and moving files to and from the
//! device by QR code.
//!
//! ```text
//! faraday-starter [--image FILE] [--downloads DIR] [--size WxH] [--timings]
//! faraday-starter --version
//! faraday-starter --uninstall
//! faraday-starter --record-disks                  (Windows)
//! faraday-starter --write …                       (Windows, elevated)
//! faraday-starter --bitlocker key|suspend|resume … (Windows, elevated)
//! faraday-starter --record-bitlocker              (Windows, as administrator)
//! ```
//!
//! `--image FILE` writes a developer's own image in place of the
//! payload's stick image (§1.1): an `.img`, an `.img.xz`, or a
//! `-boot.vfat`, which only *keep data* writes. Its SHA-256 is computed
//! at start and shown on Confirm. A Starter built without a payload
//! (`FARADAY_STARTER_PAYLOAD` unset) offers only this.
//!
//! Transfer sends from and saves into `$HOME/Downloads`, or `--downloads
//! DIR`, as the desktop app's does; the camera is the desktop app's too.
//! `--timings` prints what the frames cost once a second. `--version`
//! prints the build string and exits before anything is initialised;
//! install reads the installed copy's version that way.
//!
//! The window, its pacing and its input are the desktop app's
//! (`faraday-window`); everything that differs between operating systems
//! is behind `faraday_starter_shell::platform::Platform`.
//!
//! Faraday Demo runs in this window (F126, `docs/STARTER.md` §5.3): Open
//! Faraday Demo on Home shows it under the top app bar, its sticks the
//! folders in `~/faraday-sticks` and its Downloads the Starter's, served
//! by `faraday_desktop::Demo` as in the Demo's own window; the bar's way
//! back, the window's close button or the Demo's power off returns to
//! Home. The Starter opens in Cobalt, Light or Dark as this computer's
//! own setting says (`faraday_starter_shell::scheme`), and a theme chosen
//! in the Demo is the Starter's until it closes.

#![deny(unsafe_code)]
// A window program on Windows: no console opens beside it.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use faraday_core::strings::EN;
use faraday_core::{Host as AppHost, StorageEvent};
use faraday_desktop::{Asked, Demo, Paths};
use faraday_shell_api::{App, Command, Event};
use faraday_starter::api::{Reason, Source, StarterCommand, StarterEvent, WriteOrder};
use faraday_starter::{Starter, StarterAction};
use faraday_starter_shell::payload::{Image, Payload};
use faraday_starter_shell::platform::{self, Platform};
use faraday_starter_shell::writer::Job;
use faraday_storage::{Boxes, Dirs, Host, serve_with};
use faraday_window::camera::{self, Camera};
use faraday_window::drag::TitleDrag;
use faraday_window::entropy;
use faraday_window::input::Pointer;
use faraday_window::lens::{Lens, LensHost};
use faraday_window::pace::{Pacer, TICK};
use faraday_window::view::{self, View};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// How often the Downloads folder and the cameras are looked at again,
/// on a thread of their own ([`watch_listing`]).
const RESCAN: Duration = Duration::from_millis(700);
/// How often what is attached is read again for what is of concern
/// (docs/STARTER.md §3.1a): polling, the same on every system, since no
/// system tells an unprivileged program about every screen and USB
/// device without a window or a bus of its own; Check again reads at
/// once.
const CONCERNS_EVERY: Duration = Duration::from_secs(3);
const TITLE: &str = EN.starter_window_title;

struct Shell {
    starter: Starter,
    /// The operating system's side (`Platform`).
    system: Arc<dyn Platform>,
    payload: Option<Payload>,
    image: Option<Image>,
    /// What the system's threads report: the disks, a write.
    events: Receiver<StarterEvent>,
    tx: Sender<StarterEvent>,
    /// A disk written, to have what is on it read again.
    forget: Sender<String>,
    /// Check again: what is attached read at once.
    check: Sender<()>,
    /// The disk being written.
    writing: Option<String>,
    /// The folders of the sheets this run printed with the BitLocker
    /// recovery key on them (F49), removed when the Starter exits.
    key_sheets: Vec<PathBuf>,
    /// The Downloads folder Transfer uses.
    host: Host,
    /// What [`watch_listing`] reads: the Downloads folder and the
    /// cameras, sent when either changed.
    listing: Receiver<Listing>,
    listed: Option<StorageEvent>,
    cameras: Option<Vec<(String, String)>>,
    boxes: Boxes,
    /// The sticks' folders, their pictures read on a thread of their own.
    dirs: Dirs,
    size: (u32, u32),
    view: Option<View>,
    shown: (u32, u32),
    scale: f64,
    pace: Pacer,
    pointer: Pointer,
    /// A press on the top bar's empty room that may move the window.
    title: TitleDrag,
    lens: Lens,
    /// Faraday Demo's side of the window while it is open (F126): its
    /// sticks, storage, clipboard and preview size.
    demo: Option<Demo>,
}

impl Shell {
    fn window(&self) -> Option<&Window> {
        self.view.as_ref().map(|v| &*v.window)
    }

    fn redraw(&self) {
        if let Some(w) = self.window() {
            w.request_redraw();
        }
    }

    /// Delivers one event, then answers what the Starter and the app it
    /// holds asked for.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let ticked = matches!(event, Event::Tick { .. });
        self.pace.before(&event);
        let mut events = vec![event];
        let mut draw = false;
        while let Some(e) = events.pop() {
            self.starter.event(e);
            draw |= self.answer(event_loop, &mut events);
        }
        if ticked {
            self.pace.animating = draw;
        }
        if draw {
            self.redraw();
        }
    }

    /// Answers the Starter's commands and the held app's storage, and takes
    /// what the Starter asks of the system. Whether a frame was asked for.
    fn answer(&mut self, event_loop: &ActiveEventLoop, events: &mut Vec<Event>) -> bool {
        let mut draw = false;
        while let Some(c) = self.starter.poll_command() {
            match c {
                Command::Draw => draw = true,
                Command::RequestEntropy => events.extend(entropy()),
                Command::CameraOn => events.extend(self.lens.on(self.starter.held())),
                Command::CameraOff => self.lens.off(),
                Command::Exit if !self.starter.held().restart_requested() => event_loop.exit(),
                _ => {}
            }
        }
        if !self.starter.showing_demo() {
            self.lens.chosen(self.starter.held());
        }
        for line in serve_with(
            self.starter.held(),
            &mut self.boxes,
            Some(&self.host),
            &mut self.dirs,
        ) {
            eprintln!("{line}");
        }
        draw |= self.answer_demo(events);
        while let Some(c) = self.starter.next_command() {
            self.command(c);
        }
        draw
    }

    /// Answers Faraday Demo's commands and serves its sticks and storage
    /// while it is open (F126); a lock in it starts a fresh one, and its
    /// power off leaves it. Whether a frame was asked for.
    fn answer_demo(&mut self, events: &mut Vec<Event>) -> bool {
        let mut draw = false;
        // A look at the sticks can ask for more: a few rounds at most.
        for _ in 0..4 {
            let commands: Vec<Command> =
                std::iter::from_fn(|| self.starter.poll_demo_command()).collect();
            let (Some(demo), Some(host)) = (self.demo.as_mut(), self.starter.demo()) else {
                return draw;
            };
            let mut end = Asked::Nothing;
            for c in commands {
                match c {
                    Command::Draw => draw = true,
                    Command::RequestEntropy => events.extend(entropy()),
                    Command::CameraOn => events.extend(self.lens.on(&mut host.app)),
                    Command::CameraOff => self.lens.off(),
                    other => match demo.command(&mut host.app, other) {
                        Asked::Nothing => {}
                        Asked::Event(e) => events.push(e),
                        a @ (Asked::Restart | Asked::Exit) => end = a,
                    },
                }
            }
            self.lens.chosen(&mut host.app);
            demo.took(&mut host.app);
            let looked = demo.settle(&mut host.app);
            match end {
                Asked::Restart => {
                    self.restart_demo();
                    draw = true;
                }
                Asked::Exit => {
                    self.lens.off();
                    self.starter.close_demo();
                    return true;
                }
                Asked::Nothing | Asked::Event(_) if !looked => return draw,
                Asked::Nothing | Asked::Event(_) => draw = true,
            }
        }
        draw
    }

    /// Opens Faraday Demo in the window: its stick folders made the first
    /// time, its app in Practice, held by its host under the top app bar.
    fn open_demo(&mut self) {
        let paths = Paths {
            sticks: Paths::in_home().sticks,
            downloads: self.host.downloads.clone(),
            print: self.host.print.clone(),
        };
        let sticks = paths.sticks.display().to_string();
        let mut demo = match Demo::open(paths) {
            Ok(d) => d,
            Err(detail) => {
                let failed = Reason::File {
                    path: sticks,
                    detail,
                };
                let _ = self.tx.send(StarterEvent::DesktopStarted(Err(failed)));
                return;
            }
        };
        let app = faraday_desktop::desktop_app_in(faraday_core::Mode::Demo);
        self.starter.show_demo(AppHost::new(app));
        if let Some(host) = self.starter.demo() {
            host.event(demo.offered());
            demo.rescan(&mut host.app, true);
            demo.relist(&mut host.app);
        }
        self.demo = Some(demo);
        let _ = self.tx.send(StarterEvent::DesktopStarted(Ok(())));
        self.redraw();
    }

    /// A lock in Faraday Demo: a fresh app in its place, in the mode the
    /// old one chose, given what the old one kept.
    fn restart_demo(&mut self) {
        let (Some(demo), Some(old)) = (self.demo.as_mut(), self.starter.demo()) else {
            return;
        };
        let fresh = demo.restart(&old.app);
        self.starter.show_demo(AppHost::new(fresh));
        if let (Some(demo), Some(host)) = (self.demo.as_mut(), self.starter.demo()) {
            host.event(demo.offered());
            demo.restarted(&mut host.app);
        }
    }

    /// A file dragged over the window (`over`) while Faraday Demo is open,
    /// or the drag gone.
    fn hovered(&mut self, over: bool) {
        if let Some(host) = self.starter.demo() {
            host.app.storage(StorageEvent::Hovered(over));
            self.redraw();
        }
    }

    /// A file dropped on the window while Faraday Demo is open, for its
    /// Transfer to send.
    fn dropped(&mut self, event_loop: &ActiveEventLoop, path: &std::path::Path) {
        let (Some(demo), Some(host)) = (self.demo.as_mut(), self.starter.demo()) else {
            return;
        };
        demo.dropped(&mut host.app, path);
        let mut events = Vec::new();
        self.answer(event_loop, &mut events);
        for e in events {
            self.send(event_loop, e);
        }
        self.redraw();
    }

    /// What the Starter asks of the system.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn command(&mut self, c: StarterCommand) {
        match c {
            StarterCommand::Write(order) => self.write(order),
            StarterCommand::RestartIntoFirmware => {
                let system = Arc::clone(&self.system);
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    if let Err(why) = system.restart_into_firmware() {
                        let _ = tx.send(StarterEvent::RestartFailed(why));
                    }
                });
            }
            StarterCommand::RestartFromStick { device } => {
                let system = Arc::clone(&self.system);
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    if let Err(why) = system.restart_from_stick(&device) {
                        let _ = tx.send(StarterEvent::StickRestartFailed(why));
                    }
                });
            }
            StarterCommand::Install => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let done = system.install();
                    let ok = done.is_ok();
                    let _ = tx.send(StarterEvent::InstallDone(done));
                    if ok {
                        let _ = tx.send(StarterEvent::Installed(system.installed()));
                    }
                });
            }
            StarterCommand::Uninstall => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let done = system.uninstall();
                    // The firmware's entry goes too; declined, the
                    // uninstall still finishes and says it remains.
                    if done.is_ok()
                        && let Err(why) = system.remove_stick_entry()
                    {
                        let _ = tx.send(StarterEvent::BootEntryKept(why));
                    }
                    let _ = tx.send(StarterEvent::UninstallDone(done));
                    let _ = tx.send(StarterEvent::Installed(system.installed()));
                    let _ = tx.send(StarterEvent::BootEntry(system.boot_entry()));
                });
            }
            StarterCommand::OpenDemo => self.open_demo(),
            StarterCommand::DemoClosed => {
                self.lens.off();
                self.demo = None;
            }
            StarterCommand::CheckAgain => {
                let _ = self.check.send(());
            }
            StarterCommand::RemoveBootEntry => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(StarterEvent::BootEntryRemoved(system.remove_stick_entry()));
                    let _ = tx.send(StarterEvent::BootEntry(system.boot_entry()));
                });
            }
            StarterCommand::SaveTemplate { name, pdf } => {
                let saved = faraday_storage::save_download(&self.host.downloads, &name, &pdf)
                    .map(|p| p.display().to_string())
                    .map_err(|detail| Reason::File {
                        path: self.host.downloads.display().to_string(),
                        detail,
                    });
                let _ = self.tx.send(StarterEvent::TemplateSaved(saved));
            }
            StarterCommand::PrintTemplate { name, pdf } => {
                let opened = faraday_starter_shell::pdf::write_temp(&name, &pdf)
                    .and_then(|path| faraday_starter_shell::pdf::open(&path));
                let _ = self.tx.send(StarterEvent::TemplateOpened(opened));
            }
            StarterCommand::OpenTemplate { path } => {
                let opened = faraday_starter_shell::pdf::open(std::path::Path::new(&path));
                let _ = self.tx.send(StarterEvent::TemplateOpened(opened));
            }
            StarterCommand::PrintNotes {
                name,
                mut pdf,
                with_key,
            } => {
                use zeroize::Zeroize;
                let written = if with_key {
                    let w = faraday_starter_shell::pdf::write_key_sheet(&name, &pdf);
                    // The Starter's copy of the key goes once it is in the
                    // file the viewer opens.
                    pdf.zeroize();
                    if let Some(dir) = w.as_ref().ok().and_then(|p| p.parent()) {
                        self.key_sheets.push(dir.to_path_buf());
                    }
                    w
                } else {
                    faraday_starter_shell::pdf::write_temp(&name, &pdf)
                };
                let opened = written.and_then(|path| faraday_starter_shell::pdf::open(&path));
                let _ = self.tx.send(StarterEvent::NotesOpened(opened));
            }
            StarterCommand::ReadBitLocker => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(StarterEvent::BitLocker(system.bitlocker()));
                });
            }
            StarterCommand::BitLocker(step) => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let (result, read) = system.bitlocker_step(step);
                    let _ = tx.send(StarterEvent::BitLockerDone { step, result });
                    // As the elevated copy read it at the end; read again
                    // without elevation when it did not.
                    let now = read.or_else(|| system.bitlocker());
                    let _ = tx.send(StarterEvent::BitLocker(now));
                });
            }
            StarterCommand::ReadVersion { disk, serial, size } => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let result = system.read_version(&disk, &serial, size);
                    let _ = tx.send(StarterEvent::VersionRead {
                        disk,
                        serial,
                        size,
                        result,
                    });
                });
            }
            StarterCommand::ReadRecoveryKey => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(StarterEvent::RecoveryKey(system.recovery_key()));
                });
            }
        }
    }

    /// What a thread of its own needs to answer the system's slow
    /// commands.
    fn threaded(&self) -> (Arc<dyn Platform>, Sender<StarterEvent>, Option<Payload>) {
        (
            Arc::clone(&self.system),
            self.tx.clone(),
            self.payload.clone(),
        )
    }

    /// Starts a confirmed write, from the payload or the developer's
    /// image the Starter named.
    fn write(&mut self, order: WriteOrder) {
        let job = match &order.source {
            Source::Payload { name, inner_sha256 } => self
                .payload
                .as_ref()
                .and_then(|p| p.job(name, inner_sha256)),
            Source::Image { path, sha256 } => self
                .image
                .as_ref()
                .filter(|i| {
                    i.path.display().to_string() == *path && i.sum.sha256.to_string() == *sha256
                })
                .map(|i| Job::Image(i.clone())),
        };
        let Some(job) = job else {
            let _ = self.tx.send(StarterEvent::Written(Err(Reason::NotCarried)));
            return;
        };
        self.writing = Some(order.disk.clone());
        self.system.write(order, job, self.tx.clone());
    }

    /// Passes on what the system's threads reported.
    fn reported(&mut self, event_loop: &ActiveEventLoop) {
        let mut any = false;
        while let Ok(e) = self.events.try_recv() {
            if matches!(e, StarterEvent::Written(_))
                && let Some(id) = self.writing.take()
            {
                let _ = self.forget.send(id);
            }
            self.starter.starter_event(e);
            any = true;
        }
        if any {
            let mut events = Vec::new();
            let mut draw = self.answer(event_loop, &mut events);
            for e in events {
                self.send(event_loop, e);
                draw = true;
            }
            if draw {
                self.redraw();
            }
        }
    }

    /// Tells the held app what Downloads holds and which cameras there
    /// are, when that changed: the newest of what [`watch_listing`] read.
    /// A lookup; the reading is done on that thread.
    fn relist(&mut self) {
        let Some(now) = self.listing.try_iter().last() else {
            return;
        };
        if self.listed.as_ref() != Some(&now.files) {
            self.starter.held().storage(now.files.clone());
            self.listed = Some(now.files);
            self.redraw();
        }
        if self.cameras.as_ref() != Some(&now.cameras) {
            self.starter
                .held()
                .storage(StorageEvent::Cameras(now.cameras.clone()));
            self.cameras = Some(now.cameras);
            self.redraw();
        }
    }

    /// Faraday Demo's Downloads listing and cameras, to its app, and its
    /// sticks looked at again when the timer says (F126). A lookup but
    /// for the look at the sticks, which is the Demo's own.
    fn demo_due(&mut self, event_loop: &ActiveEventLoop, now: Instant) {
        let (changed, looked) = match (self.demo.as_mut(), self.starter.demo()) {
            (Some(demo), Some(host)) => (
                demo.relist(&mut host.app),
                demo.due(now) && demo.rescan(&mut host.app, false),
            ),
            _ => return,
        };
        if looked {
            let mut events = Vec::new();
            self.answer(event_loop, &mut events);
            for e in events {
                self.send(event_loop, e);
            }
        }
        if changed || looked {
            self.redraw();
        }
    }

    fn display(&self) -> Event {
        view::display(self.shown, self.scale, None)
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

    fn faraday(&mut self) -> Option<&mut faraday_core::Faraday> {
        // Faraday Demo's app while it is open, the held app's otherwise.
        if self.shell.starter.showing_demo() {
            return self.shell.starter.demo().map(|h| &mut h.app);
        }
        Some(self.shell.starter.held())
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(
                f64::from(self.size.0),
                f64::from(self.size.1),
            ))
            .with_min_inner_size(LogicalSize::new(640.0, 400.0));
        let view = match View::open(event_loop, attributes) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("faraday-starter: {e}");
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
        self.relist();
        self.pace.next_tick = Instant::now() + TICK;
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            // Closed while Faraday Demo is open, the window goes back to
            // Home (F126).
            WindowEvent::CloseRequested if self.starter.showing_demo() => {
                self.starter.close_demo();
                let mut events = Vec::new();
                self.answer(event_loop, &mut events);
                self.redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::DroppedFile(path) if self.starter.showing_demo() => {
                self.dropped(event_loop, &path);
            }
            WindowEvent::HoveredFile(_) if self.starter.showing_demo() => self.hovered(true),
            WindowEvent::HoveredFileCancelled if self.starter.showing_demo() => {
                self.hovered(false);
            }
            WindowEvent::RedrawRequested => {
                if self.pace.paced {
                    let tick = self.pace.tick();
                    self.send(event_loop, tick);
                    self.pace.next_tick = Instant::now() + TICK;
                }
                if let Some(view) = self.view.as_mut() {
                    view.draw(&mut self.starter, &mut self.pace);
                }
            }
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
                self.redraw();
            }
            other => {
                // A press on the top bar's empty room and a drag move
                // the window, a double-click maximises it; not the press
                // the system makes of a finger's.
                if let Some(view) = self.view.as_ref()
                    && !self.pointer.is_echo(&other)
                    && self
                        .title
                        .takes(&other, self.pointer.cursor, &view.window, |x, y| {
                            self.starter.is_strip_room(x, y)
                        })
                {
                    return;
                }
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
        self.reported(event_loop);
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
        self.demo_due(event_loop, now);
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.lens.off();
        self.demo = None;
        self.view = None;
        faraday_starter_shell::pdf::remove_key_sheets(&self.key_sheets);
        self.key_sheets.clear();
        self.system.exiting();
    }
}

/// What Transfer lists of this computer: the Downloads folder and the
/// cameras.
#[derive(Clone, PartialEq)]
struct Listing {
    files: StorageEvent,
    cameras: Vec<(String, String)>,
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
            if last.as_ref() != Some(&now) {
                if out.send(now.clone()).is_err() {
                    return;
                }
                last = Some(now);
            }
            std::thread::sleep(RESCAN);
        }
    });
}

/// Reads what is attached that is of concern now, then every
/// [`CONCERNS_EVERY`] and whenever Check again asks, on a thread of its
/// own; sends it when it changed, and always after Check again.
fn watch_concerns(system: Arc<dyn Platform>, out: Sender<StarterEvent>, checks: Receiver<()>) {
    std::thread::spawn(move || {
        let mut last = None;
        let mut asked = true;
        loop {
            let now = system.concerns();
            if (asked || last.as_ref() != Some(&now))
                && out.send(StarterEvent::Concerns(now.clone())).is_err()
            {
                return;
            }
            last = Some(now);
            asked = match checks.recv_timeout(CONCERNS_EVERY) {
                Ok(()) => true,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => false,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
            };
        }
    });
}

const HELP: &str = "faraday-starter [--image FILE] [--downloads DIR] [--size WxH] [--timings] [--uninstall] | --version
  --image: write FILE (.img, .img.xz, or -boot.vfat for keep data) in place of the payload's stick image
  --downloads: the folder Transfer sends from and saves into (default ~/Downloads)
  --timings: once a second, what the frames cost on stderr
  --version: print this build's version and exit
  --uninstall: open on Installed, with Uninstall asked once (Windows' uninstall entry runs this)";

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn main() -> ExitCode {
    // The elevated writer on Windows, `--write`, runs before anything
    // else is initialised (docs/STARTER.md §2).
    let given: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = platform::early(&given) {
        return code;
    }
    // `--version` answers before anything else is initialised: the
    // installed copy is asked this (docs/STARTER.md §5.2).
    if given.first().map(String::as_str) == Some("--version") {
        println!(
            "{}",
            faraday_core::version_line(
                faraday_core::Program::StarterApp,
                faraday_core::BUILD.unwrap_or(faraday_starter::VERSION)
            )
        );
        return ExitCode::SUCCESS;
    }
    let system = platform::this_computer();
    let mut image: Option<PathBuf> = None;
    let mut downloads = system.downloads();
    let mut size = (1100, 760);
    let mut timings = false;
    let mut uninstall = false;
    let mut args = given.into_iter();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--image" => match args.next() {
                Some(f) => image = Some(PathBuf::from(f)),
                None => {
                    eprintln!("faraday-starter: --image needs a file");
                    return ExitCode::from(2);
                }
            },
            "--downloads" => match args.next() {
                Some(d) => downloads = PathBuf::from(d),
                None => {
                    eprintln!("faraday-starter: --downloads needs a directory");
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
                        eprintln!("faraday-starter: --size needs WxH");
                        return ExitCode::from(2);
                    }
                }
            }
            "--timings" => timings = true,
            // The Windows uninstall entry's command (§5.2): the window
            // opens on Installed with Uninstall asked once.
            "--uninstall" => uninstall = true,
            "--help" | "-h" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("faraday-starter: unknown argument {other}");
                return ExitCode::from(2);
            }
        }
    }
    let image = match image {
        None => None,
        Some(path) => match Image::hash(&path) {
            Ok(i) => {
                eprintln!(
                    "faraday-starter: {} is {} bytes, SHA-256 {}",
                    path.display(),
                    i.sum.size,
                    i.sum.sha256
                );
                Some(i)
            }
            Err(e) => {
                eprintln!("faraday-starter: {e}");
                return ExitCode::from(2);
            }
        },
    };
    // A sheet with the recovery key an earlier run could not remove (a
    // crash, a power cut) goes now (F49).
    faraday_starter_shell::pdf::sweep_key_sheets(&faraday_starter_shell::pdf::key_base());
    let computer = system.computer();
    let payload = Payload::built_in();
    let mut starter = Starter::new();
    // Cobalt, Light or Dark as this computer's own setting says (F126).
    starter.set_theme(faraday_starter_shell::scheme::at_start(system.dark()));
    starter.starter_event(StarterEvent::Computer(computer));
    starter.starter_event(StarterEvent::OtherEncryption(system.other_encryption()));
    starter.starter_event(StarterEvent::Payload(payload.as_ref().map(Payload::event)));
    if let Some(i) = &image {
        starter.starter_event(StarterEvent::DevImage(i.event()));
    }
    let (tx, events) = channel();
    if uninstall {
        starter.starter_event(StarterEvent::Installed(system.installed()));
        starter.press(StarterAction::Installed);
        starter.press(StarterAction::Uninstall);
    }
    // The installed copy is asked its version, which takes a moment;
    // the firmware's entry is listed by a program too.
    {
        let (system, tx) = (Arc::clone(&system), tx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(StarterEvent::Installed(system.installed()));
            let _ = tx.send(StarterEvent::BootEntry(system.boot_entry()));
        });
    }
    let (check, checks) = channel();
    watch_concerns(Arc::clone(&system), tx.clone(), checks);
    let (forget, forgotten) = channel();
    system.watch(tx.clone(), forgotten);
    let event_loop = match EventLoop::new() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("faraday-starter: no display to open a window on: {e}");
            return ExitCode::FAILURE;
        }
    };
    let print = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join("faraday-print"))
        .unwrap_or_else(|| PathBuf::from("faraday-print"));
    let pace = Pacer::new(timings);
    let opens = faraday_storage::can_open_folders();
    let (listed, listing) = channel();
    watch_listing(downloads.clone(), opens, listed);
    let mut shell = Shell {
        starter,
        system,
        payload,
        image,
        events,
        tx,
        forget,
        check,
        writing: None,
        key_sheets: Vec::new(),
        host: Host {
            print,
            downloads,
            opens,
        },
        listing,
        listed: None,
        cameras: None,
        boxes: Boxes::Memory {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: Vec::new(),
        },
        dirs: Dirs::default(),
        size,
        view: None,
        shown: size,
        scale: 1.0,
        pace,
        pointer: Pointer::new(),
        title: TitleDrag::default(),
        lens: Lens::new(Camera::new(None, false)),
        demo: None,
    };
    let ran = event_loop.run_app(&mut shell);
    // However the loop ended, the key's sheets go.
    faraday_starter_shell::pdf::remove_key_sheets(&shell.key_sheets);
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("faraday-starter: {e}");
            ExitCode::FAILURE
        }
    }
}
