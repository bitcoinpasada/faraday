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
//! `--image FILE` writes a developer's own image in place of a found
//! stick image (§1.1): an `.img`, an `.img.xz`, or a
//! `-boot.vfat`, which only *keep data* writes. Its SHA-256 is computed
//! at start and shown on Confirm.
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
//! by `faraday_demo::Demo` as in the Demo's own window; the bar's way
//! back, the window's close button or the Demo's power off returns to
//! Home. The Starter opens in Cobalt, Light or Dark as this computer's
//! own setting says (`faraday_starter_shell::scheme`), and a theme chosen
//! in the Demo is the Starter's until it closes. An installed Starter
//! opens at the text size its settings file keeps
//! (`faraday_starter_shell::settings`).

#![deny(unsafe_code)]
// A window program on Windows: no console opens beside it.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use faraday_core::StorageEvent;
use faraday_core::strings::EN;
use faraday_demo::{Asked, Demo, Paths};
use faraday_host::Host as AppHost;
use faraday_shell_api::{App, Command, Event};
use faraday_starter::api::{Reason, Source, StarterCommand, StarterEvent, WriteOrder};
use faraday_starter::{Starter, StarterAction};
use faraday_starter_shell::finder::{Finder, built_in_list, found_job, image_names};
use faraday_starter_shell::image::Image;
use faraday_starter_shell::platform::{self, Platform};
use faraday_starter_shell::signed;
use faraday_starter_shell::writer::Job;
use faraday_storage::{Boxes, Dirs, Host, serve_with};
use faraday_window::camera::{self, Camera};
use faraday_window::entropy;
use faraday_window::lens::Lens;
use faraday_window::run::{Core, Program, run};
use faraday_window::view;
use winit::dpi::LogicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowAttributes};

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
    /// What the held Demo's worker calls to wake the loop.
    proxy: EventLoopProxy<()>,
    /// The operating system's side (`Platform`).
    system: Arc<dyn Platform>,
    /// The release's image list built in and the sums worked out against it.
    finder: Arc<Finder>,
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
    /// The window was asked to close during the write.
    close_held: bool,
    /// The folders of the sheets this run printed with the BitLocker
    /// recovery key on them (F49), removed when the Starter exits.
    key_sheets: Vec<PathBuf>,
    /// The Downloads folder Transfer uses.
    host: Host,
    /// What [`watch_listing`] reads: the Downloads folder and the
    /// cameras, sent when either changed.
    listing: Receiver<Listing>,
    listed: Option<faraday_storage::Downloads>,
    cameras: Option<Vec<(String, String)>>,
    boxes: Boxes,
    /// The sticks' folders, their pictures read on a thread of their own.
    dirs: Dirs,
    /// Faraday Demo's side of the window while it is open (F126): its
    /// sticks, storage, clipboard and preview size.
    demo: Option<Demo>,
}

impl Shell {
    /// Delivers one event, then answers what the Starter and the app it
    /// holds asked for.
    fn send_event(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, event: Event) {
        let ticked = matches!(event, Event::Tick { .. });
        core.pace.before(&event);
        let mut events = vec![event];
        let mut draw = false;
        while let Some(e) = events.pop() {
            self.starter.event(e);
            draw |= self.answer(core, event_loop, &mut events);
        }
        if ticked {
            // A job on the held Demo's worker keeps the frames coming.
            let working = self.starter.demo().is_some_and(|h| h.app.busy().is_some());
            core.pace.animating = draw || working;
        }
        if draw {
            core.redraw();
        }
    }

    /// Answers the Starter's commands and the held app's storage, and takes
    /// what the Starter asks of the system. Whether a frame was asked for.
    fn answer(
        &mut self,
        core: &mut Core,
        event_loop: &ActiveEventLoop,
        events: &mut Vec<Event>,
    ) -> bool {
        let mut draw = false;
        while let Some(c) = self.starter.poll_command() {
            match c {
                Command::Draw => draw = true,
                Command::RequestEntropy => events.extend(entropy()),
                Command::CameraOn => events.extend(core.lens.on(self.starter.held())),
                Command::CameraOff => core.lens.off(),
                Command::Exit if !self.starter.held().restart_requested() => event_loop.exit(),
                _ => {}
            }
        }
        if !self.starter.showing_demo() {
            core.lens.chosen(self.starter.held());
        }
        for line in serve_with(
            self.starter.held(),
            &mut self.boxes,
            Some(&self.host),
            &mut self.dirs,
        ) {
            eprintln!("{}", &*line);
        }
        draw |= self.answer_demo(core, events);
        while let Some(c) = self.starter.next_command() {
            self.command(core, event_loop, c);
        }
        draw
    }

    /// Answers Faraday Demo's commands and serves its sticks and storage
    /// while it is open (F126); a lock in it starts a fresh one, and its
    /// power off leaves it. Whether a frame was asked for.
    fn answer_demo(&mut self, core: &mut Core, events: &mut Vec<Event>) -> bool {
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
                    Command::CameraOn => events.extend(core.lens.on(&mut host.app)),
                    Command::CameraOff => core.lens.off(),
                    other => match demo.command(&mut host.app, other) {
                        Asked::Nothing => {}
                        Asked::Event(e) => events.push(e),
                        a @ (Asked::Restart | Asked::Exit) => end = a,
                    },
                }
            }
            core.lens.chosen(&mut host.app);
            demo.took(&mut host.app);
            let looked = demo.settle(&mut host.app);
            match end {
                Asked::Restart => {
                    self.restart_demo();
                    draw = true;
                }
                Asked::Exit => {
                    core.lens.off();
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
    fn open_demo(&mut self, core: &Core) {
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
        let mut app = faraday_demo::desktop_app_in(faraday_core::Mode::Demo);
        faraday_window::wake_on_work(&mut app, &self.proxy);
        self.starter.show_demo(AppHost::new(app));
        if let Some(host) = self.starter.demo() {
            host.event(demo.offered());
            demo.rescan(&mut host.app, true);
            demo.relist(&mut host.app);
        }
        self.demo = Some(demo);
        let _ = self.tx.send(StarterEvent::DesktopStarted(Ok(())));
        core.redraw();
    }

    /// A lock in Faraday Demo: a fresh app in its place, in the mode the
    /// old one chose, given what the old one kept.
    fn restart_demo(&mut self) {
        let (Some(demo), Some(old)) = (self.demo.as_mut(), self.starter.demo()) else {
            return;
        };
        let mut fresh = demo.restart(&old.app);
        faraday_window::wake_on_work(&mut fresh, &self.proxy);
        self.starter.show_demo(AppHost::new(fresh));
        if let (Some(demo), Some(host)) = (self.demo.as_mut(), self.starter.demo()) {
            host.event(demo.offered());
            demo.restarted(&mut host.app);
        }
    }

    /// A file dragged over the window (`over`) while Faraday Demo is open,
    /// or the drag gone.
    fn drag_over(&mut self, core: &Core, over: bool) {
        if let Some(host) = self.starter.demo() {
            host.app.storage(StorageEvent::Hovered(over));
            core.redraw();
        }
    }

    /// A file dropped on the window: while Faraday Demo is open, for its
    /// Transfer to send; otherwise for What to write's boot file (§4.5).
    fn drop_file(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, path: &std::path::Path) {
        let (Some(demo), Some(host)) = (self.demo.as_mut(), self.starter.demo()) else {
            if let Ok(full) = std::path::absolute(path) {
                self.starter.file_dropped(full.display().to_string());
                let mut events = Vec::new();
                self.answer(core, event_loop, &mut events);
                core.redraw();
            }
            return;
        };
        demo.dropped(&mut host.app, path);
        let mut events = Vec::new();
        self.answer(core, event_loop, &mut events);
        for e in events {
            self.send_event(core, event_loop, e);
        }
        core.redraw();
    }

    /// The window's close, and the Starter's own close button's (F266):
    /// a write under way is not cut short, the window closes when it
    /// ends (`reported`); otherwise the Starter closes.
    fn close_window(&mut self, core: &mut Core, event_loop: &ActiveEventLoop) {
        if self.writing.is_some() {
            self.starter.hold_close();
            self.close_held = true;
            core.redraw();
        } else {
            event_loop.exit();
        }
    }

    /// What the Starter asks of the system.
    fn command(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, c: StarterCommand) {
        match c {
            StarterCommand::Close => self.close_window(core, event_loop),
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
                let (system, tx) = self.threaded();
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
                let (system, tx) = self.threaded();
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
            StarterCommand::OpenDemo => self.open_demo(core),
            // Asked only while a Starter is installed; one that cannot
            // be written leaves the size for this run.
            StarterCommand::KeepTextSize(text) => {
                if let Some(file) = self.system.settings_file()
                    && let Err(e) = faraday_starter_shell::settings::write(&file, text)
                {
                    eprintln!("faraday-starter: {}: {e}", file.display());
                }
            }
            StarterCommand::DemoClosed => {
                core.lens.off();
                self.demo = None;
            }
            StarterCommand::CheckAgain => {
                let _ = self.check.send(());
            }
            StarterCommand::RemoveBootEntry => {
                let (system, tx) = self.threaded();
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
                        detail: String::from(&*detail),
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
                let (system, tx) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(StarterEvent::BitLocker(system.bitlocker()));
                });
            }
            StarterCommand::BitLocker(step) => {
                let (system, tx) = self.threaded();
                std::thread::spawn(move || {
                    let (result, read) = system.bitlocker_step(step);
                    let _ = tx.send(StarterEvent::BitLockerDone { step, result });
                    // As the elevated copy read it at the end; read again
                    // without elevation when it did not.
                    let now = read.or_else(|| system.bitlocker());
                    let _ = tx.send(StarterEvent::BitLocker(now));
                });
            }
            StarterCommand::ReadVersion {
                disk,
                serial,
                size,
                source,
            } => {
                let (system, tx) = self.threaded();
                // The release to judge the partition by, with the size of a
                // developer's image the elevated copy cannot work out.
                let judge = source.and_then(|source| {
                    let size = match self.job(&source)? {
                        Job::Image(i) => Some(i.sum.size),
                        Job::Found { .. } => None,
                    };
                    Some((source, size))
                });
                std::thread::spawn(move || {
                    let (result, signed) = match system.read_version(&disk, &serial, size, judge) {
                        Ok((release, sum, signed)) => (Ok((release, sum)), signed),
                        Err(why) => (Err(why), None),
                    };
                    let _ = tx.send(StarterEvent::VersionRead {
                        disk,
                        serial,
                        size,
                        result,
                        signed,
                    });
                });
            }
            StarterCommand::ReadRecoveryKey => {
                let (system, tx) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(StarterEvent::RecoveryKey(system.recovery_key()));
                });
            }
            StarterCommand::ListBootFiles => {
                let (tx, folder) = (self.tx.clone(), self.host.downloads.clone());
                std::thread::spawn(move || {
                    let files = signed::list(&folder);
                    let folder = folder.display().to_string();
                    let _ = tx.send(StarterEvent::BootFiles { folder, files });
                });
            }
            StarterCommand::FindImages { dropped } => {
                let (tx, finder, folder) = (
                    self.tx.clone(),
                    Arc::clone(&self.finder),
                    self.host.downloads.clone(),
                );
                std::thread::spawn(move || {
                    let dropped: Vec<PathBuf> = dropped.into_iter().map(PathBuf::from).collect();
                    let _ = tx.send(StarterEvent::ImagesFound(finder.answer(&folder, &dropped)));
                });
            }
            StarterCommand::JudgeBoot {
                disk,
                serial,
                size,
                sha256,
                source,
            } => {
                let (system, tx) = self.threaded();
                let job = self.job(&source);
                std::thread::spawn(move || {
                    // A partition not read, or changed since it was
                    // listed, is answered as not signed: the listed one
                    // already differs from the release's.
                    let read = system.read_boot_partition(&disk, &serial, size);
                    let signed = match (read, job) {
                        (Some(bytes), Some(job)) => {
                            signed::sha256(&bytes) == sha256
                                && signed::is_release_signed(&job, &bytes)
                        }
                        _ => false,
                    };
                    let _ = tx.send(StarterEvent::BootJudged {
                        disk,
                        serial,
                        size,
                        sha256,
                        signed,
                    });
                });
            }
            StarterCommand::CheckBootFile { path, source } => {
                let (tx, job) = (self.tx.clone(), self.job(&source));
                std::thread::spawn(move || {
                    let result = job
                        .ok_or(Reason::NotCarried)
                        .and_then(|job| signed::choose(std::path::Path::new(&path), &job));
                    let _ = tx.send(StarterEvent::BootFileChecked { path, result });
                });
            }
        }
    }

    /// What a thread of its own needs to answer the system's slow
    /// commands.
    fn threaded(&self) -> (Arc<dyn Platform>, Sender<StarterEvent>) {
        (Arc::clone(&self.system), self.tx.clone())
    }

    /// What the writer takes for `source`: the release image found, with
    /// its line in the list beside it, or the developer's image the
    /// Starter named. A lookup.
    fn job(&self, source: &Source) -> Option<Job> {
        match source {
            Source::Found {
                path,
                xz_sha256,
                img_sha256,
            } => found_job(path, xz_sha256, img_sha256),
            Source::Image { path, sha256 } => self
                .image
                .as_ref()
                .filter(|i| {
                    i.path.display().to_string() == *path && i.sum.sha256.to_string() == *sha256
                })
                .map(|i| Job::Image(i.clone())),
        }
    }

    /// Starts a confirmed write, from the image found or the developer's
    /// image the Starter named.
    fn write(&mut self, order: WriteOrder) {
        let Some(job) = self.job(&order.source) else {
            let _ = self.tx.send(StarterEvent::Written(Err(Reason::NotCarried)));
            return;
        };
        self.writing = Some(order.disk.clone());
        self.system.write(order, job, self.tx.clone());
    }

    /// Passes on what the system's threads reported.
    fn reported(&mut self, core: &mut Core, event_loop: &ActiveEventLoop) {
        let mut any = false;
        while let Ok(e) = self.events.try_recv() {
            if matches!(e, StarterEvent::Written(_))
                && let Some(id) = self.writing.take()
            {
                let _ = self.forget.send(id);
            }
            let ended = matches!(e, StarterEvent::Written(_));
            self.starter.starter_event(e);
            any = true;
            if ended && self.close_held {
                event_loop.exit();
                return;
            }
        }
        if any {
            let mut events = Vec::new();
            let mut draw = self.answer(core, event_loop, &mut events);
            for e in events {
                self.send_event(core, event_loop, e);
                draw = true;
            }
            if draw {
                core.redraw();
            }
        }
    }

    /// Tells the held app what Downloads holds and which cameras there
    /// are, when that changed: the newest of what [`watch_listing`] read.
    /// A lookup; the reading is done on that thread.
    fn relist(&mut self, core: &Core) {
        let Some(now) = self.listing.try_iter().last() else {
            return;
        };
        if self.listed.as_ref() != Some(&now.files) {
            self.starter.held().storage(now.files.event());
            self.listed = Some(now.files);
            core.redraw();
        }
        if self.cameras.as_ref() != Some(&now.cameras) {
            let cameras = now
                .cameras
                .iter()
                .map(|(id, name)| (faraday_text::Text::of(id), faraday_text::Text::of(name)))
                .collect();
            self.starter.held().storage(StorageEvent::Cameras(cameras));
            self.cameras = Some(now.cameras);
            core.redraw();
        }
    }

    /// Faraday Demo's Downloads listing and cameras, to its app, and its
    /// sticks looked at again when the timer says (F126). A lookup but
    /// for the look at the sticks, which is the Demo's own.
    fn demo_due(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, now: Instant) {
        let (changed, looked) = match (self.demo.as_mut(), self.starter.demo()) {
            (Some(demo), Some(host)) => (
                demo.relist(&mut host.app),
                demo.due(now) && demo.rescan(&mut host.app, false),
            ),
            _ => return,
        };
        if looked {
            let mut events = Vec::new();
            self.answer(core, event_loop, &mut events);
            for e in events {
                self.send_event(core, event_loop, e);
            }
        }
        if changed || looked {
            core.redraw();
        }
    }
}

impl Program for Shell {
    const NAME: &'static str = "faraday-starter";

    fn attributes(&self, core: &Core, _event_loop: &ActiveEventLoop) -> WindowAttributes {
        let attributes = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(
                f64::from(core.size.0),
                f64::from(core.size.1),
            ))
            .with_min_inner_size(LogicalSize::new(640.0, 400.0));
        view::identify(
            attributes,
            faraday_core::names::MENU_ID,
            include_bytes!("../assets/icon.png"),
        )
    }

    fn started(&mut self, core: &mut Core, _event_loop: &ActiveEventLoop) {
        self.relist(core);
    }

    fn send(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, event: Event) {
        self.send_event(core, event_loop, event);
    }

    fn faraday(&mut self) -> Option<&mut faraday_core::Faraday> {
        // Faraday Demo's app while it is open, the held app's otherwise.
        if self.starter.showing_demo() {
            return self.starter.demo().map(|h| &mut h.app);
        }
        Some(self.starter.held())
    }

    fn close_requested(&mut self, core: &mut Core, event_loop: &ActiveEventLoop) {
        if self.starter.showing_demo() {
            // Closed while Faraday Demo is open, the window goes back to
            // Home (F126).
            self.starter.close_demo();
            let mut events = Vec::new();
            self.answer(core, event_loop, &mut events);
            core.redraw();
        } else {
            self.close_window(core, event_loop);
        }
    }

    fn dropped(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, path: &Path) {
        self.drop_file(core, event_loop, path);
    }

    fn hovered(&mut self, core: &mut Core, over: bool) -> bool {
        let taken = self.starter.showing_demo();
        if taken {
            self.drag_over(core, over);
        }
        taken
    }

    fn draw(&mut self, core: &mut Core) {
        if let Some(view) = core.view.as_mut() {
            view.draw(&mut self.starter, &mut core.pace);
        }
    }

    fn strip_room(&self, x: i32, y: i32) -> bool {
        self.starter.is_strip_room(x, y)
    }

    fn polled(&mut self, core: &mut Core, event_loop: &ActiveEventLoop) {
        self.reported(core, event_loop);
    }

    fn ticked(&mut self, core: &mut Core, event_loop: &ActiveEventLoop, now: Instant) {
        self.relist(core);
        self.demo_due(core, event_loop, now);
    }

    fn exiting(&mut self, core: &mut Core) {
        self.demo = None;
        core.view = None;
        faraday_starter_shell::pdf::remove_key_sheets(&self.key_sheets);
        self.key_sheets.clear();
        self.system.exiting();
    }
}

/// What Transfer lists of this computer: the Downloads folder and the
/// cameras.
#[derive(Clone, PartialEq)]
struct Listing {
    files: faraday_storage::Downloads,
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
                files: host.downloads(),
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
  --image: write FILE (.img, .img.xz, or -boot.vfat for keep data) in place of a found stick image
  --downloads: the folder Transfer sends from and saves into (default ~/Downloads)
  --timings: once a second, what the frames cost on stderr
  --version: print this build's version and exit
  --uninstall: open on Installed, with Uninstall asked once (Windows' uninstall entry runs this)";

fn main() -> ExitCode {
    // First of all, before any library is loaded: libraries come from
    // System32 only (faraday/shells/window/src/system32.rs).
    if let Err(code) = faraday_window::system32::system32_only() {
        eprintln!("cannot restrict library loading (error {code})");
        return ExitCode::FAILURE;
    }
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
    let finder = Arc::new(Finder::new(built_in_list()));
    let mut starter = Starter::new();
    starter.find_images();
    // Cobalt, Light or Dark as this computer's own setting says (F126).
    starter.set_theme(faraday_starter_shell::scheme::at_start(system.dark()));
    // The text size an installed Starter keeps (docs/TEXT-SIZE.md §6).
    if let Some(text) = system
        .settings_file()
        .and_then(|f| faraday_starter_shell::settings::read(&f))
    {
        starter.starter_event(StarterEvent::TextKept(text));
    }
    starter.starter_event(StarterEvent::Computer(computer));
    starter.starter_event(StarterEvent::OtherEncryption(system.other_encryption()));
    starter.starter_event(StarterEvent::ImagesWanted(image_names(
        built_in_list().as_ref(),
    )));
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
    let opens = faraday_storage::can_open_folders();
    let (listed, listing) = channel();
    watch_listing(downloads.clone(), opens, listed);
    let shell = Shell {
        proxy: event_loop.create_proxy(),
        starter,
        system,
        finder,
        image,
        events,
        tx,
        forget,
        check,
        writing: None,
        close_held: false,
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
        demo: None,
    };
    let core = Core::new(size, timings, Lens::new(Camera::new(None, false)));
    let (ran, shell) = run(event_loop, core, shell);
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
