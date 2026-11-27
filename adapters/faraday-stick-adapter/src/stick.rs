//! The stick adapter: the panel, the input devices, the clock, the entropy and the orders, behind
//! the port's `carry` and `poll` (02 §1, §3, §4), and the root's two seams, `present` and `ended`.

use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use faraday_boot::medium::Record;
use faraday_crypto::Zeroizing;
use faraday_port::bytes::{Entropy, EntropyBytes, EntropySource};
use faraday_port::camera;
use faraday_port::event::{Abilities, DevicesFact, Event, Input, Start, Time, TouchPhase};
use faraday_port::host::HostAnswer;
use faraday_port::order::{Exit, Order};
use faraday_port::system::{Reason, SystemAnswer, SystemAsk};
use faraday_port::values::{Display, Orientation, Pace, Point};
use faraday_port::{Adapters, Answer, Ask, AskId, Effect, Failure, Frame, Waker};
use faraday_storage::unsupported;

use crate::cursor::{Cursor, Pointed};
use crate::evdev::{self, Direct};
use crate::fb::Depth;
use crate::files::Files;
use crate::machine;
use crate::orient::{self, Turn};
use crate::panel::{Panel, lock};
use crate::present::{Changed, Known, Moved, Presenter};
use crate::refresh;
use crate::start_window;
use crate::timings::Timings;
use crate::touch;
use crate::usb;
use crate::wakechan::{self, WakeRx, WakeTx};
use crate::{Arrow, StickOptions, Wake};

/// How long the loop waits between ticks at rest.
const TICK: Duration = Duration::from_millis(50);
/// The tick while the last frame said `Pace::Moving`.
const FRAME: Duration = Duration::from_millis(16);
/// Wakes the ring holds. A pass takes every one of them, so the depth only has to cover the
/// touches a controller can report between two passes.
const WAKES: usize = 64;

/// The exit statuses init reads (`faraday/image/common/rootfs-overlay/etc/inittab`): a lock
/// starts a fresh process, a restart restarts the computer, anything else powers off.
const RESTART_CODE: u8 = 75;
const RESTART_COMPUTER_CODE: u8 = 76;

/// Every device adapter but storage.
pub struct Stick {
    rx: WakeRx,
    /// Kept for the readers of devices plugged in later, and for the tests.
    tx: WakeTx,
    waker: Arc<dyn Waker>,
    geometry: crate::fb::Geometry,
    turn: orient::Shared,
    setup: orient::Setup,
    dpi: u16,
    external: bool,
    panel: Arc<Mutex<Panel>>,
    presenter: Presenter,
    cursor: Option<Arc<Mutex<Cursor>>>,
    pointer_speed: f32,
    touch_panel: evdev::TouchPanel,
    watch: Option<evdev::Watch>,
    window: start_window::Window,
    scanned: Instant,
    trust_told: Option<(bool, bool)>,
    epoch: Instant,
    last_tick: Instant,
    /// The last frame said `Pace::Moving`, or a finger is down.
    moving: Arc<AtomicBool>,
    contact: bool,
    sent_start: bool,
    queue: VecDeque<Event>,
    answers: Receiver<Event>,
    answer_tx: Sender<Event>,
    panic_timer: crate::keyboard::PanicTimer,
    stop: Arc<AtomicBool>,
    ended: Option<u8>,
    off_device: bool,
    verbose: bool,
    timings: Option<Timings>,
    files: Files,
    boot_params: Vec<u8>,
    drm_edid: Vec<u8>,
}

impl Stick {
    /// Opens the panel and the input devices and starts the clock's thread. The first `poll` is
    /// the `Start`.
    pub fn open(o: StickOptions, waker: Arc<dyn Waker>) -> Result<Self, Failure> {
        let fb_path = PathBuf::from(o.fb.as_str());
        let off_device = !fb_path.starts_with("/dev");
        let depth = (o.depth != 0)
            .then(|| Depth::from_bits(u32::from(o.depth)))
            .flatten();
        let probed = machine::probe(&fb_path, off_device)?;
        let screen = machine::geometry(o.size, depth, probed)?;
        let cmdline = std::fs::read_to_string(machine::CMDLINE).unwrap_or_default();
        // Only a framebuffer's own stride can hold a window, so never with a size given.
        let inset = (!off_device && o.size.is_none())
            .then(|| screen.window(machine::window_asked(&cmdline)?))
            .flatten();
        let geometry = inset.map_or(screen, |(inside, _)| inside);
        let boot_params = if off_device {
            Vec::new()
        } else {
            std::fs::read(machine::BOOT_PARAMS).unwrap_or_default()
        };
        let drm_edid = if off_device {
            Vec::new()
        } else {
            orient::drm_edid(Path::new(machine::DRM_CLASS))
        };
        let external = !drm_edid.is_empty();
        let edid_size = faraday_paper::edid::from_boot_params(&boot_params)
            .or_else(|| (!drm_edid.is_empty()).then_some(drm_edid.as_slice()))
            .and_then(faraday_paper::edid::size)
            // The EDID gives the size of the whole screen, never of a window in it.
            .filter(|_| inset.is_none());
        let fb_size = (geometry.width, geometry.height);
        let dpi = machine::dpi((o.dpi != 0).then_some(o.dpi), &cmdline, edid_size, fb_size);
        let mount = match o.mount.as_str() {
            "90" => Some(Turn::R90),
            "270" => Some(Turn::R270),
            _ => None,
        };
        let setup = if o.native {
            orient::Setup::stated(true, mount)
        } else if off_device {
            orient::Setup::stated(fb_size.1 > fb_size.0, mount)
        } else {
            let read = |p: &str| std::fs::read_to_string(p).unwrap_or_default();
            orient::Setup::detected(
                fb_size,
                edid_size,
                &read(machine::SYS_VENDOR),
                &read(machine::PRODUCT_NAME),
            )
        };
        // A dev card (`timings`) also reads the framebuffer back.
        let fb = OpenOptions::new()
            .read(o.timings)
            .write(true)
            .create(off_device)
            .truncate(false)
            .open(&fb_path)
            .map_err(|e| {
                e.raw_os_error().map_or(Failure::Unavailable, |c| {
                    Failure::Os(faraday_port::ask::OsError(c))
                })
            })?;

        let (tx, rx) = wakechan::channel(WAKES, Some(waker.clone()));
        let mut window = start_window::Window::load(Path::new(machine::BOXES));
        window.set_laptop(machine::chassis_laptop(
            &std::fs::read_to_string(machine::CHASSIS_TYPE).unwrap_or_default(),
        ));
        let mut watch = (!off_device).then(|| {
            evdev::Watch::new(
                Path::new("/sys/class/input"),
                Path::new("/dev/input"),
                touch::DEFAULT_TOUCH_NAME,
            )
        });
        let found: Vec<(u32, evdev::Device)> = match watch.as_mut() {
            Some(watch) => watch.rescan(&mut |sys| window.admit(sys)),
            None => Vec::new(),
        };
        let pointing = found.iter().any(|(_, d)| d.mouse || d.touchpad);
        let turn = orient::Shared::new(setup.at_start());
        let frame = turn.get().frame_size(fb_size);
        let vblank = fb
            .try_clone()
            .ok()
            .filter(|_| !off_device)
            .map(|f| Box::new(refresh::FbVblank(f)) as Box<dyn refresh::Vblank>);
        let mut panel = Panel::new(fb, geometry);
        if let Some((_, at)) = inset {
            panel.window_in(screen, at);
        }
        panel.set_turn(turn.get());
        let panel = Arc::new(Mutex::new(panel));
        let mut presenter = Presenter::default();
        presenter.write_on_thread(&panel, vblank);
        let pointer_speed = f32::from(if o.pointer_speed == 0 {
            10
        } else {
            o.pointer_speed
        }) / 10.0;
        let cursor =
            pointing.then(|| Arc::new(Mutex::new(Cursor::new(frame.0, frame.1, pointer_speed))));
        let touch_panel = evdev::TouchPanel {
            width: geometry.width,
            height: geometry.height,
            grid: touch::DEFAULT_TOUCH_GRID,
            calibration: touch::Calibration::default(),
            turn: turn.clone(),
            window: inset.map(|(_, at)| evdev::Window {
                screen: (screen.width, screen.height),
                at,
            }),
        };
        let arrow = cursor.clone().map(|cursor| Arrow {
            cursor,
            panel: panel.clone(),
        });
        for (id, device) in found {
            // A device that is not part of the machine is never believed: its input is held back
            // and dropped (fails closed).
            let flag = Arc::new(AtomicBool::new(device.trusted));
            evdev::spawn(
                id,
                device,
                &touch_panel,
                tx.clone(),
                Direct {
                    arrow: arrow.clone(),
                    believed: flag,
                },
            );
        }
        let (answer_tx, answers) = mpsc::channel();
        let moving = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        pacer(waker.clone(), moving.clone(), stop.clone());
        let now = Instant::now();
        Ok(Stick {
            rx,
            tx,
            waker,
            geometry,
            turn,
            setup,
            dpi,
            external,
            panel,
            presenter,
            cursor,
            pointer_speed,
            touch_panel,
            watch,
            window,
            scanned: now,
            trust_told: None,
            epoch: now,
            last_tick: now,
            moving,
            contact: false,
            sent_start: false,
            queue: VecDeque::new(),
            answers,
            answer_tx,
            panic_timer: crate::keyboard::PanicTimer::default(),
            stop,
            ended: None,
            off_device,
            verbose: o.verbose,
            timings: o
                .timings
                .then(|| Timings::new(env!("CARGO_PKG_VERSION"), frame.0, frame.1)),
            files: Files::new(PathBuf::from(o.files.as_str()), !off_device),
            boot_params,
            drm_edid,
        })
    }

    /// The status the process ends with, once an `Order::Exit` has been carried or the panic chord
    /// has been held: 0 to power off, 75 for a lock, 76 to restart the computer. `None` while the
    /// app runs.
    #[must_use]
    pub fn ended(&self) -> Option<u8> {
        self.ended
    }

    /// Puts a frame on the panel: the rows that changed are converted and written
    /// (`docs/SPEED.md` §11.1). An empty `changed` is unknown, and the frame is compared whole.
    pub fn present(&mut self, frame: &Frame<'_>) {
        if self.ended.is_some() {
            return;
        }
        let g = self.geometry;
        let turn = self.turn.get();
        let (fw, fh) = turn.frame_size((g.width, g.height));
        let pixels = usize::from(fw) * usize::from(fh);
        if (frame.width, frame.height) != (fw, fh) || frame.rgba.len() < pixels * 4 {
            self.presenter.skipped();
            self.say("the frame does not match the panel; not drawing");
            return;
        }
        let changed: Vec<Changed> = frame
            .changed
            .iter()
            .filter_map(|r| Changed::of(*r, (fw, fh)))
            .collect();
        // A rectangle outside the frame says nothing about it: unknown, taken whole.
        let known = (!frame.changed.is_empty() && changed.len() == frame.changed.len())
            .then_some(changed.as_slice());
        let known = Known {
            moved: frame.moved.and_then(|m| Moved::of(m, (fw, fh))),
            changed: known,
        };
        let result = self.presenter.present_parts(
            &frame.rgba[..pixels * 4],
            known,
            g,
            turn,
            &self.panel,
            self.timings.as_mut(),
        );
        if let Err(e) = result {
            self.say(&format!("framebuffer write failed: {e}"));
        }
        self.moving.store(
            frame.pace == Pace::Moving || self.contact,
            Ordering::Relaxed,
        );
    }

    fn say(&self, message: &str) {
        if self.verbose {
            eprintln!("{message}");
        }
    }

    fn frame_size(&self) -> (u16, u16) {
        self.turn
            .get()
            .frame_size((self.geometry.width, self.geometry.height))
    }

    fn display(&self) -> Display {
        let (width, height) = self.frame_size();
        Display {
            width,
            height,
            dpi: self.dpi,
            orientation: if width >= height {
                Orientation::Landscape
            } else {
                Orientation::Portrait
            },
            rotation_offered: self.setup.offered.then(|| self.setup.native()),
            external: self.external,
            preview: None,
        }
    }

    fn time(&self) -> Time {
        Time {
            now_ms: u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX),
            unix_secs: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
        }
    }

    fn start(&self) -> Start {
        let medium = machine::medium();
        let cores = std::thread::available_parallelism()
            .ok()
            .and_then(|n| u16::try_from(n.get()).ok())
            .unwrap_or(1);
        Start {
            display: self.display(),
            medium,
            abilities: Abilities {
                clipboard: false,
                print: false,
                host_files: false,
                opens: false,
                camera: false,
                names_sticks: true,
                holds_back_input: true,
                restart_computer: true,
                moves_window: false,
                system: false,
                lock_ends_process: true,
            },
            machine: machine::machine(&self.boot_params, &self.drm_edid, cores),
            time: self.time(),
            carried: None,
            settings: None,
            scheme: None,
            root_read_only: if self.off_device {
                None
            } else {
                machine::root_read_only()
            },
        }
    }

    /// The arrow's thread-side handle, once there is a cursor to move.
    fn arrow(&self) -> Option<Arrow> {
        self.cursor.clone().map(|cursor| Arrow {
            cursor,
            panel: self.panel.clone(),
        })
    }

    /// Puts the arrow on the panel where the cursor is, or takes it away when the cursor is
    /// hidden: its rows only, now.
    fn move_arrow(&self) {
        let at = self.cursor.as_ref().and_then(|c| {
            let c = lock(c);
            c.visible().then(|| c.at())
        });
        if let Err(e) = lock(&self.panel).move_arrow(at) {
            self.say(&format!("framebuffer write failed: {e}"));
        }
    }

    /// What the cursor made of a motion, as the port's input, in order.
    fn deliver(&mut self, pointed: Option<Pointed>) {
        for input in pointed.into_iter().flat_map(Pointed::events) {
            if let Input::Touch { phase, .. } = input {
                self.contact = phase != TouchPhase::Up;
            }
            self.queue.push_back(Event::Input(input));
        }
    }

    /// Takes in what the readers sent since the last look, turned into the port's events.
    fn take_wakes(&mut self) {
        let mut batch: Vec<Wake> = Vec::new();
        while let Ok(wake) = self.rx.try_recv() {
            match wake {
                Wake::Frame | Wake::Work => {}
                Wake::InputEnded(id) => self.input_ended(id),
                // Held back: a device the person has not let in never presses anything.
                Wake::Held(..) => {}
                other => batch.push(other),
            }
            if batch.len() >= WAKES {
                break;
            }
        }
        collapse_moves(&mut batch);
        for wake in batch {
            match wake {
                Wake::Touch(t) => {
                    // A finger on the screen is where the person is looking: the arrow gets out
                    // of the way, and with it whatever it was over.
                    if let Some(c) = self.cursor.clone() {
                        let mut c = lock(&c);
                        if c.visible() {
                            c.hide();
                            drop(c);
                            self.move_arrow();
                            self.queue.push_back(Event::Input(Input::Hover(None)));
                        }
                    }
                    self.contact = t.phase != TouchPhase::Up;
                    self.queue.push_back(Event::Input(Input::Touch {
                        at: Point {
                            x: i32::from(t.x),
                            y: i32::from(t.y),
                        },
                        phase: t.phase,
                    }));
                }
                Wake::Pointed(pointed) => self.deliver(pointed),
                Wake::Pointer(motion) => {
                    let Some(c) = self.cursor.clone() else {
                        continue;
                    };
                    let pointed = lock(&c).apply(motion);
                    self.move_arrow();
                    self.deliver(pointed);
                }
                Wake::Key(key) => self.queue.push_back(Event::Input(Input::Key(key))),
                Wake::KeyUp(key) => self.queue.push_back(Event::Input(Input::KeyUp(key))),
                Wake::Panic(on) => self.panic_timer.chord(on, Instant::now()),
                Wake::Frame | Wake::Work | Wake::InputEnded(_) | Wake::Held(..) => {}
            }
        }
        // A tap's click whose wait for a drag is over comes up now (`docs/MOTION.md` §3.9).
        if let Some(c) = self.cursor.clone() {
            let released = lock(&c).due(Instant::now());
            self.deliver(released);
        }
    }

    fn input_ended(&mut self, id: u32) {
        self.say("an input device ended");
        if let Some(watch) = self.watch.as_mut() {
            watch.ended(id);
            if !watch.pointing()
                && let Some(c) = self.cursor.clone()
            {
                let mut c = lock(&c);
                if c.visible() {
                    c.hide();
                    drop(c);
                    self.move_arrow();
                }
            }
        }
    }

    /// What input the machine itself holds that can be trusted: a pointer, a keyboard.
    fn trust(&self) -> (bool, bool) {
        match self.watch.as_ref() {
            Some(w) => (w.trusted_points(), w.trusted_types()),
            None => (true, false),
        }
    }

    /// Looks at what was plugged in, closes the start window, and tells the app what input it can
    /// trust.
    fn watch_devices(&mut self) {
        if let Some(watch) = self.watch.as_mut() {
            let window = &mut self.window;
            let found = watch.take(&mut |sys| window.admit(sys));
            self.start_readers(found);
        }
        let every = if self.window.is_open() {
            evdev::RESCAN_AT_START
        } else {
            evdev::RESCAN
        };
        if self.scanned.elapsed() >= every {
            self.scanned = Instant::now();
            // The look is done on a thread of its own: the loop never waits on the kernel for it.
            if let Some(watch) = self.watch.as_mut() {
                watch.ask();
            }
        }
        self.close_window();
        let trust = self.trust();
        if self.trust_told != Some(trust) && self.watch.is_some() {
            self.trust_told = Some(trust);
            self.queue.push_back(Event::Devices(DevicesFact::Trust {
                pointer: trust.0,
                keyboard: trust.1,
            }));
        }
    }

    fn start_readers(&mut self, found: Vec<(u32, evdev::Device)>) {
        for (id, device) in found {
            if (device.mouse || device.touchpad) && self.cursor.is_none() {
                let (w, h) = self.frame_size();
                self.cursor = Some(Arc::new(Mutex::new(Cursor::new(w, h, self.pointer_speed))));
            }
            let believed = Arc::new(AtomicBool::new(device.trusted));
            evdev::spawn(
                id,
                device,
                &self.touch_panel,
                self.tx.clone(),
                Direct {
                    arrow: self.arrow(),
                    believed,
                },
            );
        }
    }

    /// Closes the start window (F202) once the USB device list has been quiet, or at the cap: the
    /// devices found are trusted, and the app is told which external USB devices were plugged in.
    fn close_window(&mut self) {
        if !self.window.is_open() {
            return;
        }
        if self.watch.is_none() {
            self.window.close();
            return;
        }
        let up = self.epoch.elapsed();
        self.window
            .observe(up, usb::devices(Path::new(machine::USB_DEVICES)));
        if !self.window.ready(up) {
            return;
        }
        // One more look while the window is still open, so a device whose node came just now is
        // found as it is.
        if let Some(watch) = self.watch.as_mut() {
            let window = &mut self.window;
            let found = watch.rescan(&mut |sys| window.admit(sys));
            self.start_readers(found);
        }
        self.window.close();
        self.window.save(Path::new(machine::BOXES));
        let usb = usb::devices(Path::new(machine::USB_DEVICES));
        let boot =
            match Record::parse(&std::fs::read_to_string(machine::EXCHANGE).unwrap_or_default()) {
                Record::Found { partition, .. } => {
                    start_window::medium_port(Path::new(machine::SYS_BLOCK), &partition)
                }
                _ => None,
            };
        let names = start_window::names(Path::new(machine::USB_DEVICES), &usb, boot.as_deref());
        if !names.is_empty() {
            let names = names.iter().map(|n| faraday_text::Text::of(n)).collect();
            self.queue
                .push_back(Event::Devices(DevicesFact::BootUsb(names)));
        }
    }

    fn tick_due(&mut self) {
        let step = if self.moving.load(Ordering::Relaxed) {
            FRAME
        } else {
            TICK
        };
        if self.last_tick.elapsed() >= step {
            self.last_tick = Instant::now();
            self.queue.push_back(Event::Time(self.time()));
            self.write_timings(false);
        }
    }

    /// Appends a block to the dev card when one is due (`timings`).
    fn write_timings(&mut self, last: bool) {
        let elapsed = self.epoch.elapsed();
        let Some(t) = self.timings.as_mut() else {
            return;
        };
        let block = if last {
            Some(t.last(elapsed, None))
        } else {
            t.due(elapsed, || None)
        };
        if let Some(block) = block
            && let Err(e) = self.files.append_timings(&block)
        {
            self.say(&e);
        }
    }

    /// The process ends: what the panel shows is gone before the power is, and every copy of the
    /// picture is wiped (02 P13).
    fn end(&mut self, status: u8) {
        self.write_timings(true);
        let _ = self.presenter.settle(None);
        lock(&self.panel).blank();
        self.wipe_copies();
        self.stop.store(true, Ordering::Relaxed);
        self.ended = Some(status);
    }

    /// Wipes the kept frame, its converted copy and the panel's kept copy.
    fn wipe_copies(&mut self) {
        let _ = self.presenter.settle(None);
        self.presenter.wipe();
        lock(&self.panel).wipe();
    }

    fn order(&mut self, order: Order) {
        match order {
            Order::CameraOff
            | Order::AimInput(_)
            | Order::DecideInput { .. }
            | Order::WatchDevices(_)
            | Order::ToggleStick(_)
            | Order::Vibrate { .. } => {}
            Order::Clean(on) => {
                if !self.off_device {
                    let put = machine::put_marker(Path::new(machine::CLEAN_MARKER), on, b"clean\n");
                    if let Err(e) = put {
                        self.say(&format!("the clean marker: {e}"));
                    }
                }
            }
            Order::Turn(chosen) => {
                let turn = self.setup.turn_for(chosen);
                self.turn.set(turn);
                lock(&self.panel).set_turn(turn);
                let (w, h) = self.frame_size();
                if let Some(c) = self.cursor.as_ref() {
                    lock(c).resize(w, h);
                }
                self.presenter.skipped();
                self.queue.push_back(Event::Display(self.display()));
            }
            Order::PreviewSize(_) => self.queue.push_back(Event::Display(self.display())),
            Order::Exit(Exit::End) => self.end(0),
            Order::Exit(Exit::Lock) => self.end(RESTART_CODE),
            Order::Exit(Exit::RestartComputer) => self.end(RESTART_COMPUTER_CODE),
            Order::Exit(Exit::Reset(_)) => {
                // The scope dropped: no copy of the picture outlives it, and the display is sent
                // again (§7.6).
                self.wipe_copies();
                self.queue.push_back(Event::Display(self.display()));
            }
        }
    }

    fn answer(&self, id: AskId, answer: Answer) {
        self.queue_answer(Event::Answer(id, answer));
    }

    fn queue_answer(&self, event: Event) {
        let _ = self.answer_tx.send(event);
        self.waker.wake();
    }

    /// Randomness from `/dev/random`, read on a thread of its own: at an early boot the read may
    /// wait.
    fn entropy(&self, id: AskId) {
        let tx = self.answer_tx.clone();
        let waker = self.waker.clone();
        let verbose = self.verbose;
        let spawned = std::thread::Builder::new()
            .name(String::from("entropy"))
            .spawn(move || {
                let answer = read_entropy(machine::ENTROPY_SOURCE);
                if verbose && answer.is_err() {
                    eprintln!("entropy: could not read {}", machine::ENTROPY_SOURCE);
                }
                let _ = tx.send(Event::Answer(id, Answer::Entropy(answer)));
                waker.wake();
            });
        if spawned.is_err() {
            self.answer(id, Answer::Entropy(Err(Failure::Unavailable)));
        }
    }
}

/// 32 bytes from `path`, wiped when it is dropped.
fn read_entropy(path: &str) -> Result<Entropy, Failure> {
    use std::io::Read;
    let mut bytes = Zeroizing::new([0u8; 32]);
    let mut file = std::fs::File::open(path).map_err(|_| Failure::Unavailable)?;
    file.read_exact(&mut bytes[..])
        .map_err(|_| Failure::Unavailable)?;
    Ok(Entropy {
        bytes: EntropyBytes::new(*bytes),
        source: EntropySource::DevRandom,
    })
}

/// Drops every move but the last of a run of them.
///
/// A controller reports a moving finger a hundred times a second and a mouse reports motion as
/// fast as it is polled; only the newest position of a run matters. A pointer's moves are deltas,
/// so the dropped ones are added into the one that is kept.
fn collapse_moves(batch: &mut Vec<Wake>) {
    use crate::pointer::Motion;
    let mut i = 0;
    while i + 1 < batch.len() {
        match (batch[i], batch[i + 1]) {
            (Wake::Touch(a), Wake::Touch(b))
                if a.phase == TouchPhase::Move && b.phase == TouchPhase::Move =>
            {
                batch.remove(i);
            }
            (
                Wake::Pointer(Motion::Move { dx, dy }),
                Wake::Pointer(Motion::Move { dx: bx, dy: by }),
            ) => {
                batch[i + 1] = Wake::Pointer(Motion::Move {
                    dx: dx + bx,
                    dy: dy + by,
                });
                batch.remove(i);
            }
            (
                Wake::Pointer(Motion::Pan { dx, dy }),
                Wake::Pointer(Motion::Pan { dx: bx, dy: by }),
            ) => {
                batch[i + 1] = Wake::Pointer(Motion::Pan {
                    dx: dx + bx,
                    dy: dy + by,
                });
                batch.remove(i);
            }
            // Moves already applied to the arrow: the app wants where it is now.
            (
                Wake::Pointed(Some(Pointed::Hover { .. })),
                Wake::Pointed(Some(Pointed::Hover { .. })),
            ) => {
                batch.remove(i);
            }
            (Wake::Pointed(Some(Pointed::Touch(a))), Wake::Pointed(Some(Pointed::Touch(b))))
                if a.phase == TouchPhase::Move && b.phase == TouchPhase::Move =>
            {
                batch.remove(i);
            }
            _ => i += 1,
        }
    }
}

/// The thread that wakes the root's loop every tick, so a tick is sent when nothing else woke it
/// (every 50 ms at rest, about every frame while moving); it ends when `stop` is set.
fn pacer(waker: Arc<dyn Waker>, moving: Arc<AtomicBool>, stop: Arc<AtomicBool>) {
    let spawned = std::thread::Builder::new()
        .name(String::from("pacer"))
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(if moving.load(Ordering::Relaxed) {
                    FRAME
                } else {
                    TICK
                });
                waker.wake();
            }
        });
    drop(spawned);
}

impl Adapters for Stick {
    fn carry(&mut self, effect: Effect) {
        match effect {
            Effect::Order(order) => self.order(order),
            Effect::Ask(id, ask) => match ask {
                Ask::Entropy => self.entropy(id),
                Ask::Clipboard => self.answer(id, Answer::Clipboard(Err(Failure::Unsupported))),
                Ask::Print(_) => self.answer(id, Answer::Printed(Err(Failure::Unsupported))),
                Ask::Host(ask) => {
                    let no = Failure::Unsupported;
                    let answer = match ask {
                        faraday_port::host::HostAsk::Read { .. } => HostAnswer::Read(Err(no)),
                        faraday_port::host::HostAsk::Save { .. } => HostAnswer::Saved(Err(no)),
                        faraday_port::host::HostAsk::Open(_) => HostAnswer::Opened(Err(no)),
                    };
                    self.answer(id, Answer::Host(answer));
                }
                Ask::Settings(_) => self.answer(id, Answer::Settings(Err(Failure::Unsupported))),
                Ask::Camera(_) => {
                    self.answer(id, Answer::Camera(Err(camera::Failure::Unsupported)));
                }
                Ask::System(ask) => {
                    if let Some(answer) = system_unsupported(&ask) {
                        self.answer(id, Answer::System(answer));
                    }
                }
                // The storage adapter's: the root routes these there. An ask that reaches here is
                // answered as it would be by a product with no sticks (P9).
                Ask::Storage(ask) => self.answer(id, Answer::Storage(unsupported(&ask))),
                Ask::Keep(carry) => {
                    self.answer(id, Answer::Kept(Err((Failure::Unsupported, carry))));
                }
                Ask::Forget => self.answer(id, Answer::Forgotten(Err(Failure::Unsupported))),
                Ask::Memory => self.answer(id, Answer::Memory(None)),
            },
        }
    }

    fn poll(&mut self) -> Option<Event> {
        if let Some(event) = self.queue.pop_front() {
            return Some(event);
        }
        if !self.sent_start {
            self.sent_start = true;
            return Some(Event::Start(self.start()));
        }
        if self.ended.is_some() {
            return None;
        }
        if self.panic_timer.due(Instant::now()) {
            // The panic chord held long enough: nothing more is drawn, asked or saved.
            self.end(0);
            return None;
        }
        if let Ok(event) = self.answers.try_recv() {
            return Some(event);
        }
        self.take_wakes();
        self.watch_devices();
        self.tick_due();
        self.queue.pop_front()
    }
}

impl Drop for Stick {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.wipe_copies();
    }
}

/// What a system ask is answered on a product that does not carry the system (the stick's
/// `Abilities::system` is false, so the app never asks): the ask's own refusal where it has one.
fn system_unsupported(ask: &SystemAsk) -> Option<SystemAnswer> {
    let no = Reason::NotCarried;
    Some(match ask {
        SystemAsk::WriteStick(_) => SystemAnswer::WriteStick(Err(no)),
        SystemAsk::Install => SystemAnswer::Install(Err(no)),
        SystemAsk::Uninstall => SystemAnswer::Uninstall(Err(no)),
        SystemAsk::RemoveBootEntry => SystemAnswer::RemoveBootEntry(Err(no)),
        SystemAsk::RestartIntoFirmware => SystemAnswer::RestartIntoFirmware(Err(no)),
        SystemAsk::RestartFromStick { .. } => SystemAnswer::RestartFromStick(Err(no)),
        SystemAsk::ReadBitLocker => SystemAnswer::ReadBitLocker(None),
        SystemAsk::ReadRecoveryKey => SystemAnswer::RecoveryKey(Err(no)),
        SystemAsk::PrepareDemoSticks => SystemAnswer::PrepareDemoSticks(Err(no)),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use faraday_port::ask::Request;
    use faraday_port::event::Key;
    use faraday_text::Text;

    use super::*;
    use crate::touch::Touch;

    struct Woken(AtomicUsize);

    impl Waker for Woken {
        fn wake(&self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A stick adapter drawing into a file of the test's own, off the device: 480 by 640, 16 bits.
    fn open(name: &str) -> (Stick, PathBuf) {
        let path =
            std::env::temp_dir().join(format!("faraday-stick-{}-{name}", std::process::id()));
        let dir = std::env::temp_dir();
        let stick = Stick::open(
            StickOptions {
                fb: Text::of(path.to_str().unwrap_or_default()),
                size: Some((480, 640)),
                depth: 16,
                dpi: 160,
                native: false,
                mount: Text::of(""),
                pointer_speed: 0,
                touch_grid: false,
                files: Text::of(dir.to_str().unwrap_or_default()),
                verbose: false,
                timings: false,
                boot_report: false,
            },
            Arc::new(Woken(AtomicUsize::new(0))),
        )
        .expect("opens");
        (stick, path)
    }

    /// Polls until `found` takes an event, and returns it; the events before it are dropped.
    fn until<T>(stick: &mut Stick, mut found: impl FnMut(&Event) -> Option<T>) -> T {
        for _ in 0..400 {
            while let Some(event) = stick.poll() {
                if let Some(t) = found(&event) {
                    return t;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("the event did not come");
    }

    fn frame(pixel: u8, pace: Pace) -> Vec<u8> {
        let _ = pace;
        vec![pixel; 480 * 640 * 4]
    }

    /// Invariant: a touch and a key from a reader reach the app as the port's `Touch` and `Key`.
    /// Regression: input read but never turned into an event, or a touch delivered at the wrong place.
    /// Owner: adapter
    #[test]
    fn a_touch_and_a_key_become_the_events_the_port_names() {
        let (mut stick, path) = open("input");
        let first = stick.poll().expect("the start");
        let Event::Start(s) = first else {
            panic!("the first event is the start");
        };
        assert_eq!((s.display.width, s.display.height), (480, 640));
        assert!(s.abilities.lock_ends_process && s.abilities.holds_back_input);
        stick
            .tx
            .send(Wake::Touch(Touch {
                x: 10,
                y: 20,
                phase: TouchPhase::Down,
            }))
            .expect("sent");
        stick.tx.send(Wake::Key(Key::Char('a'))).expect("sent");
        let mut seen = Vec::new();
        until(&mut stick, |e| {
            if let Event::Input(i) = e {
                seen.push(*i);
            }
            (seen.len() >= 2).then_some(())
        });
        assert_eq!(
            seen,
            vec![
                Input::Touch {
                    at: Point { x: 10, y: 20 },
                    phase: TouchPhase::Down
                },
                Input::Key(Key::Char('a'))
            ]
        );
        let _ = std::fs::remove_file(path);
    }

    /// Invariant: a frame out is put on the panel, and a frame of another size is not.
    /// Regression: `present` taking a frame and writing nothing, or writing a frame of the wrong size.
    /// Owner: adapter
    #[test]
    fn a_frame_out_is_put_on_the_panel() {
        let (mut stick, path) = open("frame");
        let rgba = frame(0x80, Pace::Rest);
        stick.present(&Frame {
            width: 480,
            height: 640,
            rgba: &rgba,
            changed: &[],
            moved: None,
            regions: &[],
            pace: Pace::Rest,
        });
        let shown = std::fs::read(&path).expect("the panel file");
        assert_eq!(shown.len(), 480 * 640 * 2, "every row written");
        assert!(shown.iter().any(|b| *b != 0));
        let small = vec![0xFFu8; 4 * 4 * 4];
        stick.present(&Frame {
            width: 4,
            height: 4,
            rgba: &small,
            changed: &[],
            moved: None,
            regions: &[],
            pace: Pace::Rest,
        });
        assert_eq!(std::fs::read(&path).expect("the panel file"), shown);
        let _ = std::fs::remove_file(path);
    }

    /// Invariant: the exit orders end the process with the status the root reads: 0 to power off, 75 for a lock, 76 to restart.
    /// Regression: a lock that ends the process with the power-off status, which powers the device off.
    /// Owner: adapter
    #[test]
    fn the_exit_orders_give_the_status_the_root_reads() {
        for (exit, status) in [
            (Exit::End, 0),
            (Exit::Lock, RESTART_CODE),
            (Exit::RestartComputer, RESTART_COMPUTER_CODE),
        ] {
            let (mut stick, path) = open("exit");
            assert_eq!(stick.ended(), None);
            stick.carry(Effect::Order(Order::Exit(exit)));
            assert_eq!(stick.ended(), Some(status));
            let _ = std::fs::remove_file(path);
        }
        let (mut stick, path) = open("reset");
        stick.carry(Effect::Order(Order::Exit(Exit::Reset(1))));
        assert_eq!(stick.ended(), None, "a reset does not end the process");
        let _ = std::fs::remove_file(path);
    }

    /// Invariant: at an exit the panel is blank and the copies of the picture the adapter kept are wiped.
    /// Regression: the last screen, a seed's words among it, left in the adapter's buffers and on the panel.
    /// Owner: adapter
    #[test]
    fn an_exit_blanks_the_panel_and_wipes_every_copy_of_the_picture() {
        let (mut stick, path) = open("wipe");
        let rgba = frame(0xA5, Pace::Rest);
        stick.present(&Frame {
            width: 480,
            height: 640,
            rgba: &rgba,
            changed: &[],
            moved: None,
            regions: &[],
            pace: Pace::Rest,
        });
        assert!(stick.presenter.holds_a_picture());
        assert!(lock(&stick.panel).holds_a_picture());
        stick.carry(Effect::Order(Order::Exit(Exit::Lock)));
        assert!(!stick.presenter.holds_a_picture());
        assert!(!lock(&stick.panel).holds_a_picture());
        let shown = std::fs::read(&path).expect("the panel file");
        assert!(shown.iter().all(|b| *b == 0), "the panel is black");
        let _ = std::fs::remove_file(path);
    }

    /// Invariant: every ask this adapter answers is answered in the variant its own request takes.
    /// Regression: a wrong answer variant, which the runtime reads as the adapter's fault and fails the ask.
    /// Owner: adapter
    #[test]
    fn an_answer_reaches_the_request_that_asked() {
        let (mut stick, path) = open("answers");
        let id = AskId {
            generation: 0,
            n: 7,
        };
        stick.carry(Effect::Ask(id, Ask::Clipboard));
        stick.carry(Effect::Ask(
            AskId {
                generation: 0,
                n: 8,
            },
            Ask::Entropy,
        ));
        let mut clipboard = false;
        let mut entropy = false;
        until(&mut stick, |e| {
            if let Event::Answer(got, answer) = e {
                match got.n {
                    7 => clipboard = true,
                    8 => entropy = true,
                    _ => {}
                }
                let _ = answer;
            }
            (clipboard && entropy).then_some(())
        });
        let answer = Answer::Clipboard(Err(Failure::Unsupported));
        assert!(matches!(
            <faraday_port::ask::Clipboard as Request>::take(answer),
            Ok(Err(Failure::Unsupported))
        ));
        let _ = std::fs::remove_file(path);
    }
}
