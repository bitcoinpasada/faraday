//! The hub app (`docs/HUB.md`): Faraday's screens for making a stick,
//! starting this computer from it, and moving files to and from the
//! device by QR code.
//!
//! ```text
//! faraday-starter [--image FILE] [--downloads DIR] [--size WxH] [--timings]
//! faraday-starter --version
//! faraday-starter --uninstall
//! faraday-starter --remind                        (Windows, at logon)
//! faraday-starter --record-disks                  (Windows)
//! faraday-starter --record-bitlocker              (Windows, as administrator)
//! faraday-starter --write …                       (Windows, elevated)
//! faraday-starter --bitlocker <verb> …            (Windows, elevated)
//! ```
//!
//! `--remind` is the logon reminder's command while the hub app's
//! BitLocker suspension stands (`docs/HUB.md` §6.3): it opens the window
//! on the BitLocker page only when Resume BitLocker can be taken, and
//! otherwise exits with no window, removing itself once there is nothing
//! to resume.
//!
//! `--image FILE` writes a developer's own image in place of the
//! payload's stick image (§1.1): an `.img`, an `.img.xz`, or a
//! `-boot.vfat`, which only *keep data* writes. Its SHA-256 is computed
//! at start and shown on Confirm. A hub app built without a payload
//! (`FARADAY_HUB_PAYLOAD` unset) offers only this.
//!
//! Transfer sends from and saves into `$HOME/Downloads`, or `--downloads
//! DIR`, as the desktop app's does; the camera is the desktop app's too.
//! `--timings` prints what the frames cost once a second. `--version`
//! prints the build string and exits before anything is initialised;
//! install reads the installed copy's version that way.
//!
//! The window, its pacing and its input are the desktop app's
//! (`faraday-window`); everything that differs between operating systems
//! is behind `faraday_hub_shell::platform::Platform`.

#![deny(unsafe_code)]
// A window program on Windows: no console opens beside it.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use faraday_core::StorageEvent;
use faraday_core::strings::EN;
use faraday_hub::api::{
    BitLockerStep, HubCommand, HubEvent, Reason, Remind, Source, WriteOrder, remind,
};
use faraday_hub::{Hub, HubAction};
use faraday_hub_shell::payload::{Image, Payload};
use faraday_hub_shell::platform::{self, Platform};
use faraday_hub_shell::writer::Job;
use faraday_storage::{Boxes, Host, serve};
use faraday_window::camera::{self, Camera};
use faraday_window::entropy;
use faraday_window::input::{Input, Pointer};
use faraday_window::lens::{Lens, LensHost};
use faraday_window::pace::{Pacer, TICK};
use faraday_window::view::{self, View};
use osk_shell_api::{App, Command, Event};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// How often the Downloads folder and the cameras are looked at again.
const RESCAN: Duration = Duration::from_millis(700);
/// How often what is attached is read again for what is of concern
/// (docs/HUB.md §3.1a): polling, the same on every system, since no
/// system tells an unprivileged program about every screen and USB
/// device without a window or a bus of its own; Check again reads at
/// once.
const CONCERNS_EVERY: Duration = Duration::from_secs(3);
const TITLE: &str = EN.hub_window_title;

struct Shell {
    hub: Hub,
    /// The operating system's side (`Platform`).
    system: Arc<dyn Platform>,
    payload: Option<Payload>,
    image: Option<Image>,
    /// What the system's threads report: the disks, a write.
    events: Receiver<HubEvent>,
    tx: Sender<HubEvent>,
    /// A disk written, to have what is on it read again.
    forget: Sender<String>,
    /// Check again: what is attached read at once.
    check: Sender<()>,
    /// The disk being written.
    writing: Option<String>,
    /// The Downloads folder Transfer uses.
    host: Host,
    listed: Option<StorageEvent>,
    cameras: Option<Vec<(String, String)>>,
    boxes: Boxes,
    size: (u32, u32),
    view: Option<View>,
    shown: (u32, u32),
    scale: f64,
    pace: Pacer,
    next_scan: Instant,
    pointer: Pointer,
    lens: Lens,
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

    /// Delivers one event, then answers what the hub and the app it
    /// holds asked for.
    fn send(&mut self, event_loop: &ActiveEventLoop, event: Event) {
        let ticked = matches!(event, Event::Tick { .. });
        self.pace.before(&event);
        let mut events = vec![event];
        let mut draw = false;
        while let Some(e) = events.pop() {
            self.hub.event(e);
            draw |= self.answer(event_loop, &mut events);
        }
        if ticked {
            self.pace.animating = draw;
        }
        if draw {
            self.redraw();
        }
    }

    /// Answers the hub's commands and the held app's storage, and takes
    /// what the hub asks of the system. Whether a frame was asked for.
    fn answer(&mut self, event_loop: &ActiveEventLoop, events: &mut Vec<Event>) -> bool {
        let mut draw = false;
        while let Some(c) = self.hub.poll_command() {
            match c {
                Command::Draw => draw = true,
                Command::RequestEntropy => events.extend(entropy()),
                Command::CameraOn => events.extend(self.lens.on(self.hub.held())),
                Command::CameraOff => self.lens.off(),
                Command::Exit if !self.hub.held().restart_requested() => event_loop.exit(),
                _ => {}
            }
        }
        self.lens.chosen(self.hub.held());
        for line in serve(self.hub.held(), &mut self.boxes, Some(&self.host)) {
            eprintln!("{line}");
        }
        while let Some(c) = self.hub.poll_hub() {
            self.command(c);
        }
        draw
    }

    /// What the hub asks of the system.
    fn command(&mut self, c: HubCommand) {
        match c {
            HubCommand::Write(order) => self.write(order),
            HubCommand::RestartIntoFirmware => {
                let system = Arc::clone(&self.system);
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    if let Err(why) = system.restart_into_firmware() {
                        let _ = tx.send(HubEvent::RestartFailed(why));
                    }
                });
            }
            HubCommand::RestartFromStick { device } => {
                let system = Arc::clone(&self.system);
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    if let Err(why) = system.restart_from_stick(&device) {
                        let _ = tx.send(HubEvent::StickRestartFailed(why));
                    }
                });
            }
            HubCommand::Install { desktop } => {
                let (system, tx, payload) = self.threaded();
                std::thread::spawn(move || {
                    let done = system.install(desktop, payload.as_ref());
                    let ok = done.is_ok();
                    let _ = tx.send(HubEvent::InstallDone(done));
                    if ok {
                        let _ = tx.send(HubEvent::Installed(system.installed()));
                    }
                });
            }
            HubCommand::Uninstall => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let done = system.uninstall();
                    // The firmware's entry goes too; declined, the
                    // uninstall still finishes and says it remains.
                    if done.is_ok()
                        && let Err(why) = system.remove_stick_entry()
                    {
                        let _ = tx.send(HubEvent::BootEntryKept(why));
                    }
                    // The uninstall took the BitLocker marker with it
                    // (§5.4): the state is read again first, so Home
                    // never shows the suspension as the hub app's.
                    let _ = tx.send(HubEvent::BitLocker(system.bitlocker()));
                    let _ = tx.send(HubEvent::UninstallDone(done));
                    let _ = tx.send(HubEvent::Installed(system.installed()));
                    let _ = tx.send(HubEvent::BootEntry(system.boot_entry()));
                });
            }
            HubCommand::RunDesktop => {
                let (system, tx, payload) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(HubEvent::DesktopStarted(
                        system.run_desktop(payload.as_ref()),
                    ));
                });
            }
            HubCommand::BitLocker(step) => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let (event, read) = match step {
                        BitLockerStep::ShowKey => {
                            let (r, p) = system.show_key();
                            (HubEvent::KeyShown(r), p)
                        }
                        BitLockerStep::Suspend => {
                            let (result, p) = system.suspend();
                            (HubEvent::BitLockerDone { step, result }, p)
                        }
                        BitLockerStep::Resume => {
                            let (result, p) = system.resume();
                            (HubEvent::BitLockerDone { step, result }, p)
                        }
                    };
                    // The state first, as the elevated copy read it, so
                    // the page never shows the step done over the old one.
                    let state = system.bitlocker().map(|mut b| {
                        if let Some(p) = read {
                            b.protection = p;
                        }
                        b
                    });
                    let _ = tx.send(HubEvent::BitLocker(state));
                    let _ = tx.send(event);
                    if step != BitLockerStep::ShowKey {
                        let _ = tx.send(HubEvent::Installed(system.installed()));
                    }
                });
            }
            HubCommand::CheckAgain => {
                let _ = self.check.send(());
            }
            HubCommand::RemoveBootEntry => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(HubEvent::BootEntryRemoved(system.remove_stick_entry()));
                    let _ = tx.send(HubEvent::BootEntry(system.boot_entry()));
                });
            }
            HubCommand::SaveKey { disk, key } => {
                let (system, tx, _) = self.threaded();
                std::thread::spawn(move || {
                    let _ = tx.send(HubEvent::KeySaved(system.save_key(&disk, &key)));
                });
            }
        }
    }

    /// What a thread of its own needs to answer the system's slow
    /// commands.
    fn threaded(&self) -> (Arc<dyn Platform>, Sender<HubEvent>, Option<Payload>) {
        (
            Arc::clone(&self.system),
            self.tx.clone(),
            self.payload.clone(),
        )
    }

    /// Starts a confirmed write, from the payload or the developer's
    /// image the hub named.
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
            let _ = self.tx.send(HubEvent::Written(Err(Reason::NotCarried)));
            return;
        };
        self.writing = Some(order.disk.clone());
        self.system.write(order, job, self.tx.clone());
    }

    /// Passes on what the system's threads reported.
    fn reported(&mut self, event_loop: &ActiveEventLoop) {
        let mut any = false;
        while let Ok(e) = self.events.try_recv() {
            if matches!(e, HubEvent::Written(_))
                && let Some(id) = self.writing.take()
            {
                let _ = self.forget.send(id);
            }
            self.hub.hub_event(e);
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
    /// are, when that changed.
    fn relist(&mut self) {
        let now = self.host.files();
        if self.listed.as_ref() != Some(&now) {
            self.hub.held().storage(now.clone());
            self.listed = Some(now);
            self.redraw();
        }
        let cameras = camera::list();
        if self.cameras.as_ref() != Some(&cameras) {
            self.hub
                .held()
                .storage(StorageEvent::Cameras(cameras.clone()));
            self.cameras = Some(cameras);
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
        Some(self.shell.hub.held())
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
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                if self.pace.paced {
                    let tick = self.pace.tick();
                    self.send(event_loop, tick);
                    self.pace.next_tick = Instant::now() + TICK;
                }
                if let Some(view) = self.view.as_mut() {
                    view.draw(&mut self.hub, &mut self.pace);
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
                for input in self.pointer.translate(&other) {
                    // The function keys are the desktop app's sticks;
                    // the hub has none.
                    if let Input::Event(e) = input {
                        self.send(event_loop, e);
                    }
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
        if now >= self.next_scan {
            self.relist();
            self.next_scan = now + RESCAN;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.pace.next_tick));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.lens.off();
        self.view = None;
        self.system.exiting();
    }
}

/// Reads what is attached that is of concern now, then every
/// [`CONCERNS_EVERY`] and whenever Check again asks, on a thread of its
/// own; sends it when it changed, and always after Check again.
fn watch_concerns(system: Arc<dyn Platform>, out: Sender<HubEvent>, checks: Receiver<()>) {
    std::thread::spawn(move || {
        let mut last = None;
        let mut asked = true;
        loop {
            let now = system.concerns();
            if (asked || last.as_ref() != Some(&now))
                && out.send(HubEvent::Concerns(now.clone())).is_err()
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
  --uninstall: open on Installed, with Uninstall asked once (Windows' uninstall entry runs this)
  --remind: at logon, open on BitLocker when Resume BitLocker can be taken; otherwise exit (Windows' logon reminder runs this)";

fn main() -> ExitCode {
    // The elevated writer on Windows, `--write`, runs before anything
    // else is initialised (docs/HUB.md §2).
    let given: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = platform::early(&given) {
        return code;
    }
    // `--version` answers before anything else is initialised: the
    // installed copy is asked this (docs/HUB.md §5.2).
    if given.first().map(String::as_str) == Some("--version") {
        println!(
            "{}",
            faraday_core::version_line(
                faraday_core::Program::HubApp,
                faraday_core::BUILD.unwrap_or(faraday_hub::VERSION)
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
    let mut reminded = false;
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
            // The logon reminder's command (§6.3).
            "--remind" => reminded = true,
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
    let computer = system.computer();
    let bitlocker = system.bitlocker();
    if reminded {
        let (marker, protection) = match &bitlocker {
            Some(b) => (b.marker.as_ref(), b.protection),
            None => (None, faraday_hub::api::Protection::None),
        };
        match remind(marker, protection, computer.secure_boot) {
            Remind::RemoveAndExit => {
                system.remove_reminder();
                return ExitCode::SUCCESS;
            }
            Remind::Exit => return ExitCode::SUCCESS,
            Remind::Open => {}
        }
    }
    let payload = Payload::built_in();
    let mut hub = Hub::new();
    hub.hub_event(HubEvent::Computer(computer));
    hub.hub_event(HubEvent::BitLocker(bitlocker));
    hub.hub_event(HubEvent::OtherEncryption(system.other_encryption()));
    hub.hub_event(HubEvent::Payload(payload.as_ref().map(Payload::event)));
    if let Some(i) = &image {
        hub.hub_event(HubEvent::DevImage(i.event()));
    }
    let (tx, events) = channel();
    if uninstall {
        hub.hub_event(HubEvent::Installed(system.installed()));
        hub.press(HubAction::Installed);
        hub.press(HubAction::Uninstall);
    } else if reminded {
        hub.press(HubAction::BitLocker);
    }
    // The installed copy is asked its version, which takes a moment;
    // the firmware's entry is listed by a program too.
    {
        let (system, tx) = (Arc::clone(&system), tx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(HubEvent::Installed(system.installed()));
            let _ = tx.send(HubEvent::BootEntry(system.boot_entry()));
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
    let next_scan = pace.epoch;
    let mut shell = Shell {
        hub,
        system,
        payload,
        image,
        events,
        tx,
        forget,
        check,
        writing: None,
        host: Host {
            print,
            downloads,
            opens: faraday_storage::can_open_folders(),
        },
        listed: None,
        cameras: None,
        boxes: Boxes::Memory {
            inbox: Vec::new(),
            outbox: Vec::new(),
            kept: Vec::new(),
        },
        size,
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
            eprintln!("faraday-starter: {e}");
            ExitCode::FAILURE
        }
    }
}
