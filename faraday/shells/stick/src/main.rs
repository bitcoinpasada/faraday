//! Faraday's stick shell: a copy of `opensigner/shells/pi` at upstream
//! c418768 that hosts `faraday-core` instead of OpenSigner. What
//! differs from the original: the app (`Faraday`), sticks reported to
//! it by the disk process over its two FIFOs (`PLAN.md` §4.3), or from
//! `/proc/mounts` where there is no disk process, with their files read
//! and written through `faraday-storage`; the clean marker
//! `faraday-grant` checks, published while the app is clean (§5.1); the Inbox and Outbox kept in `/run/faraday`
//! across a lock, exit code 75 when the app locks (the inittab's loop
//! starts a fresh process), and a doubled density guess for a panel
//! 2,800 pixels wide or more. The rest is upstream's, as it describes
//! below.
//!
//! The framebuffer shell (`docs/PLANNING.md` §4.1, §14 phase 4, §16.94):
//! a single static binary that owns a framebuffer and whatever input
//! devices the machine has, and nothing else.
//!
//! ```text
//! opensigner-pi [--fb PATH] [--input [KIND:]PATH]... [--camera PATH] [--no-camera]
//!               [--files DIR] [--size WxH] [--depth 16|24|32] [--dpi N]
//!               [--pointer-speed F] [--touch-name NAME] [--touch-grid WxH]
//!               [--boot-report PATH] [--frames N] [--verbose] [--timings]
//! ```
//!
//! Two machines run it. On the Raspberry Pi it is given the flags that
//! describe the soldered-on panel and nothing else: the image writes them
//! into `/etc/opensigner/args` from the panel directory's `panel.conf` and
//! the inittab passes them on, so one binary serves every panel. On a
//! laptop booted from the OpenSigner stick it is given no flags at all:
//! the firmware's framebuffer says how large it is, the kernel command
//! line says how dense it is, and the keyboard, the touchpad and the mouse
//! are found under `/dev/input`.
//!
//! It opens `/dev/fb0`, reads size, depth and row stride from
//! `/sys/class/graphics/fb0`, reads every input device it can use, and
//! runs until the core says it is done, at which point it draws the last
//! frame, waits two seconds so the user can read it, and exits. Powering
//! the machine off is init's next line: the app runs as an unprivileged
//! user and could not do it. The remaining flags exist so that all of that
//! can be exercised on a build box against ordinary files.
//!
//! Input (§16.94): `evdev.rs` finds the devices and says what each one
//! is, `touch.rs` turns a touchscreen's packets into contacts, `pointer.rs`
//! turns a mouse's or a touchpad's into cursor motion, `keyboard.rs` maps
//! key codes to `Event::Key`, and `cursor.rs` owns where the pointer is
//! and draws the arrow. A mouse is a touch at the cursor, so the core
//! needs no event it did not already have and no screen is designed
//! twice.
//!
//! What it is: the standard library, `osk-shell-api`, `opensigner-core`,
//! `opensigner-scanner` for the decode and, for the camera alone,
//! `opensigner-v4l2`. No `mmap` of its own
//! (the standard library has none, and `write_all_at` on a framebuffer
//! is one `pwrite` per frame), no `libc`, no `unsafe`, no logging. The
//! device has no console and no SSH, so nothing is printed unless
//! `--verbose` asks for it.
//!
//! Camera (§16.33): a `CameraOn` opens the first `/dev/video*` that
//! streams video (or `--camera PATH`), asks it for 640 × 480 in `GREY`,
//! `YUYV`, `NV12` or `YU12`, and streams luma frames from a capture
//! thread, which is also the thread the device is opened on, so that
//! the tap that opens the scanner does not wait for it. The capture
//! thread posts a note on the loop's wake channel after each frame it
//! queues, so a frame ends the wait as a touch does and the preview
//! runs at the rate the preview costs rather than at the tick. That
//! thread reduces each frame to preview size as well, so the frame goes
//! two ways without a copy: the reduced one to the core, which draws
//! it, and the captured one to `opensigner-scanner`, whose worker reads
//! the codes in it and hands them back for the loop to send on as
//! `Event::Scanned`. The
//! `ioctl`, `mmap` and `poll` that needs live in `opensigner-v4l2`, the
//! one crate in the workspace allowed to use `unsafe`, and the reason
//! for the exception is in its crate documentation; this shell still
//! has no `unsafe` of its own. `--no-camera` refuses, which is how the
//! file fallback is exercised on a build box that has a webcam.
//!
//! Files (§16.16): the boot medium's `OSKDATA` partition is mounted at
//! `/mnt/microsd` by init, every other FAT partition on a USB disk under
//! `/mnt/usb`, and `--files DIR` points the channel at one directory on
//! a build box. A `RequestFile` is answered with every file in every
//! mounted place that could be what was asked for, newest first, as one
//! `FileList`; the list is built then and there, so a stick plugged in
//! after the app is up is on it. The core draws the list and asks for
//! one of them by name with `ReadFile`, which is answered with its
//! bytes. A `WriteFile` saves to `/mnt/microsd`, or to the first
//! plugged-in partition when there is no boot medium mounted, under the
//! core's name hint, never over a file already there, and is answered
//! `FileWritten`, or `FileNotWritten` when there is nowhere to save to.
//! The shell reads no format and draws nothing: the core draws every
//! screen, and this shell only says what is on the media.
//!
//! Settings (§6): the six settings the app keeps live in
//! `opensigner-settings.txt` on the same partition, read once at start
//! and rewritten whenever the user changes one. The write goes to a
//! temporary file, is flushed, and is renamed over the settings, so a
//! card pulled mid-write keeps the old ones. No card mounted is no
//! settings kept, and one line for `--verbose` to say so.
//!
//! Timings (§16.35): `--timings` is the dev card's one way of saying how
//! long anything takes. The loop times every call it already makes into
//! five buckets and appends a block of them to `opensigner-timings.txt`
//! on the card every five seconds and once at exit. The image adds the
//! flag only when `DEV=1`, so a release card never records anything.
//!
//! What it does not do yet: physical buttons, hotplug (a device plugged
//! in after start is not seen), and the console is never put into
//! graphics mode; all three are named in `README.md`.

mod camera;
mod cursor;
mod evdev;
mod fb;
mod files;
mod keyboard;
mod pointer;
mod timings;
mod touch;

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::time::{Duration, Instant};

use camera::{Camera, Capture};
use cursor::{Cursor, Pointed};
use faraday_core::{Faraday, StickInfo, StorageEvent};
use faraday_scanner::Scanner;
use faraday_storage::{Boxes, DiskProcess, serve, serve_with, stick_info};
use fb::{Depth, Geometry};
use files::Files;
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
use timings::{Bucket, Timings};

/// How long the main loop waits to be woken before sending a tick.
const TICK: Duration = Duration::from_millis(50);

/// How long it waits while a finger is on the panel. A drag is drawn as
/// often as the panel can be painted rather than five times a second,
/// and the pass costs the same as any other.
const DRAG_TICK: Duration = Duration::from_millis(16);

/// Wakes the channel holds. A pass takes every one of them, so the
/// depth only has to cover the touches a controller can report between
/// two passes; past that the touch reader waits, which is the right
/// answer for a loop that is already behind.
const WAKES: usize = 64;

/// What woke the main loop.
///
/// One channel carries all of it, so the loop waits once and is woken by
/// whichever source is ready. A frame carries no payload: the frame
/// itself stays on the camera's own bounded channel, where a full
/// channel drops the stale frame, and the note only says that a pass is
/// worth making now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wake {
    /// One contact from a touchscreen, already in panel pixels.
    Touch(touch::Touch),
    /// One motion from a mouse or a touchpad, in device units: the
    /// cursor, which the main loop owns, turns it into a touch.
    Pointer(pointer::Motion),
    /// One key from a keyboard, going down.
    Key(Key),
    /// One key from a keyboard, coming up: a hold is held with Enter the
    /// way it is held with a finger (`docs/DESIGN.md` §4.15).
    KeyUp(Key),
    /// An input reader has ended: the device is gone, or `--input` named
    /// a file that has been read to the end. The number is the device's,
    /// so the watch knows which one stopped.
    InputEnded(u32),
    /// The capture thread has queued a frame.
    Frame,
    /// What a device not yet believed sent, held back (`PLAN.md` §4.6).
    Held(u32, HeldInput),
}

/// What a held-back device sends: the input [`Wake`]s, by themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldInput {
    Touch(touch::Touch),
    Pointer(pointer::Motion),
    Key(Key),
    KeyUp(Key),
}

impl HeldInput {
    /// The input a reader produced, held; anything else is not input.
    fn of(wake: Wake) -> Option<HeldInput> {
        match wake {
            Wake::Touch(t) => Some(HeldInput::Touch(t)),
            Wake::Pointer(m) => Some(HeldInput::Pointer(m)),
            Wake::Key(k) => Some(HeldInput::Key(k)),
            Wake::KeyUp(k) => Some(HeldInput::KeyUp(k)),
            _ => None,
        }
    }

    /// The input let through.
    fn wake(self) -> Wake {
        match self {
            HeldInput::Touch(t) => Wake::Touch(t),
            HeldInput::Pointer(m) => Wake::Pointer(m),
            HeldInput::Key(k) => Wake::Key(k),
            HeldInput::KeyUp(k) => Wake::KeyUp(k),
        }
    }
}

/// Where a held-back device stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hold {
    /// Shown to the person; its keys go to the app as a code.
    Pending,
    /// Believed: its input goes through.
    Believed,
    /// Kept out until unplugged.
    Ignored,
}

/// How long the final screen stays up before the machine powers off.
const GOODBYE: Duration = Duration::from_secs(2);

/// The panel this shell is built for: 480×640 at 286 dpi.
const DEFAULT_WIDTH: u16 = 480;
/// The panel's height.
const DEFAULT_HEIGHT: u16 = 640;
/// The pixel density to report when nothing says otherwise: a laptop
/// panel's, near enough, and the density the design system's dp are 1:1
/// at. The Pi's `panel.conf` gives the real figure for its panel, and the
/// stick's kernel command line gives it for a laptop.
const DEFAULT_DPI: u16 = 160;

/// The kernel command line key the stick sets to say what the screen's
/// density is.
const CMDLINE_DPI: &str = "opensigner.dpi=";

/// The file the kernel command line is read from.
const CMDLINE: &str = "/proc/cmdline";

/// Where the boot medium's exchange partition is mounted on the device.
const DEFAULT_FILES: &str = "/mnt/microsd";

/// Where the Inbox and Outbox wait between processes. The inittab makes
/// it, owned by the app's user, in the RAM root filesystem.
const BOXES: &str = "/run/faraday";

/// Where init mounts every other FAT partition of a USB disk.
const USB_MOUNTS: &str = "/mnt/usb";

/// How often the mounts are looked at again.
const STICK_SCAN: Duration = Duration::from_millis(700);

/// The disk process's FIFOs, which `rcS` makes (`PLAN.md` §4.3). Where
/// they are missing, sticks are read from the mounts.
const DISK_REQUESTS: &str = "/run/faraday-disk/requests";
const DISK_RESPONSES: &str = "/run/faraday-disk/responses";

/// The clean marker (`PLAN.md` §5.1): present exactly while the app is in
/// the clean state. `faraday-grant` hands a partition out only while it
/// is there, and takes every one back when it goes.
const CLEAN_MARKER: &str = "/run/faraday-clean/clean";

/// Everything the command line can change.
struct Args {
    /// The framebuffer to write; a regular file off-device.
    fb: PathBuf,
    /// Files of recorded packets to read instead of searching for
    /// devices, each with the kind of device it stands in for.
    input: Vec<evdev::Device>,
    /// The capture device to open, instead of searching `/dev/video*`.
    camera: Option<PathBuf>,
    /// Refuse every `CameraOn`, so the file fallback can be exercised.
    no_camera: bool,
    /// The directory files are read from and written to.
    files: PathBuf,
    /// True when `--files` was given, which also drops the mount check:
    /// off the device the directory is an ordinary one.
    files_given: bool,
    /// An explicit panel size, overriding sysfs.
    size: Option<(u16, u16)>,
    /// An explicit depth, overriding sysfs.
    depth: Option<Depth>,
    /// Pixel density to report to the core, when a flag gave one.
    dpi: Option<u16>,
    /// Device pixels the cursor moves per unit of pointer motion.
    pointer_speed: f32,
    /// The input device name to look for.
    touch_name: String,
    /// The grid the touch controller reports in.
    touch_grid: (u16, u16),
    /// A file to append the boot report to, which the dev image points
    /// at the exchange partition.
    boot_report: Option<PathBuf>,
    /// Exit after this many frames.
    frames: Option<u64>,
    /// Whether anything may be written to stderr.
    verbose: bool,
    /// Whether the loop records what its calls cost, onto the card.
    timings: bool,
    /// True when `--fb` was given, which also turns off the sysfs probe.
    off_device: bool,
}

impl Default for Args {
    fn default() -> Args {
        Args {
            fb: PathBuf::from("/dev/fb0"),
            input: Vec::new(),
            camera: None,
            no_camera: false,
            files: PathBuf::from(DEFAULT_FILES),
            files_given: false,
            size: None,
            depth: None,
            dpi: None,
            pointer_speed: 1.0,
            touch_name: String::from(touch::DEFAULT_TOUCH_NAME),
            touch_grid: touch::DEFAULT_TOUCH_GRID,
            boot_report: None,
            frames: None,
            verbose: false,
            timings: false,
            off_device: false,
        }
    }
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--fb" => {
                args.fb = PathBuf::from(value()?);
                args.off_device = true;
            }
            "--input" => args.input.push(input_device(&value()?)?),
            "--camera" => args.camera = Some(PathBuf::from(value()?)),
            "--no-camera" => args.no_camera = true,
            "--files" => {
                args.files = PathBuf::from(value()?);
                args.files_given = true;
            }
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size wants WxH")?;
                let w = w.parse().map_err(|_| "bad width")?;
                let h = h.parse().map_err(|_| "bad height")?;
                args.size = Some((w, h));
            }
            "--depth" => {
                let v: u32 = value()?.parse().map_err(|_| "--depth wants 16, 24 or 32")?;
                args.depth = Some(Depth::from_bits(v).ok_or("--depth wants 16, 24 or 32")?);
            }
            "--dpi" => args.dpi = Some(value()?.parse().map_err(|_| "bad dpi")?),
            "--pointer-speed" => {
                let speed: f32 = value()?.parse().map_err(|_| "bad pointer speed")?;
                if !(speed.is_finite() && speed > 0.0) {
                    return Err(String::from("--pointer-speed wants a positive number"));
                }
                args.pointer_speed = speed;
            }
            "--touch-name" => args.touch_name = value()?,
            "--touch-grid" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--touch-grid wants WxH")?;
                let w: u16 = w.parse().map_err(|_| "bad touch grid width")?;
                let h: u16 = h.parse().map_err(|_| "bad touch grid height")?;
                if w == 0 || h == 0 {
                    return Err(String::from("--touch-grid has no pixels"));
                }
                args.touch_grid = (w, h);
            }
            "--boot-report" => args.boot_report = Some(PathBuf::from(value()?)),
            "--frames" => args.frames = Some(value()?.parse().map_err(|_| "bad frame count")?),
            "--verbose" => args.verbose = true,
            "--timings" => args.timings = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

/// What sysfs says about the framebuffer named by `--fb`, or by default
/// `/dev/fb0`.
///
/// `Ok(None)` is "nothing was asked": off the device, or on it with no
/// such framebuffer directory. An `Err` is a framebuffer that is there
/// and cannot be described, which is not something to guess past.
fn probe(args: &Args) -> Result<Option<Geometry>, String> {
    if args.off_device {
        return Ok(None);
    }
    match sysfs_dir(&args.fb) {
        Some(dir) if dir.is_dir() => Geometry::from_sysfs(&dir).map(Some),
        _ => Ok(None),
    }
}

/// The geometry to draw with: sysfs on the device, the flags off it, and
/// the flags win wherever both have an answer.
///
/// `probed` is [`probe`]'s answer. A framebuffer the device has and
/// cannot describe stops the shell here with the reason, rather than
/// falling back to the 480×640 panel this shell was first written for and
/// drawing it into the corner of somebody's laptop screen. That default
/// is for the file mode and nothing else.
fn geometry(args: &Args, probed: Result<Option<Geometry>, String>) -> Result<Geometry, String> {
    let probed = probed?;
    let (width, height) = args
        .size
        .or(probed.map(|g| (g.width, g.height)))
        .unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));
    let depth = args
        .depth
        .or(probed.map(|g| g.depth))
        .unwrap_or(Depth::Rgb565);
    if width == 0 || height == 0 {
        return Err(String::from("the panel has no pixels"));
    }
    // A probed stride is only meaningful for the probed size and depth.
    let packed = usize::from(width) * depth.bytes_per_pixel();
    let stride = match probed {
        Some(g) if (g.width, g.height, g.depth) == (width, height, depth) => g.stride,
        _ => packed,
    };
    Ok(Geometry {
        width,
        height,
        depth,
        stride,
    })
}

/// One `--input` value: `PATH`, or `KIND:PATH` where the kind is
/// `touch`, `mouse`, `pad` or `keys`.
///
/// A file of recorded packets stands in for a device a build box has not
/// got, which is how every input path is exercised here. A bare path is a
/// touchscreen, which is what `--input` has always meant.
fn input_device(value: &str) -> Result<evdev::Device, String> {
    let (kind, path) = match value.split_once(':') {
        Some((kind, path)) if ["touch", "mouse", "pad", "keys"].contains(&kind) => (kind, path),
        _ => ("touch", value),
    };
    Ok(evdev::Device {
        node: PathBuf::from(path),
        name: format!("--input {kind}"),
        keyboard: kind == "keys",
        mouse: kind == "mouse",
        touchpad: kind == "pad",
        touchscreen: kind == "touch",
        hi_res_wheel: false,
        left_button: kind != "pad",
        abs_single: kind == "pad",
        // A file standing in for a device on a build box.
        trusted: true,
    })
}

/// The density to report to the core: `--dpi` first, then whatever the
/// kernel command line says, then 160.
///
/// The stick H2 builds puts `opensigner.dpi=N` on the command line, since
/// a firmware framebuffer says how many pixels it has and never how large
/// they are. Nothing on a laptop can be asked, so a person who finds the
/// text small edits one boot entry.
fn dpi(args: &Args, cmdline: &str) -> u16 {
    if let Some(dpi) = args.dpi {
        return dpi;
    }
    cmdline
        .split_whitespace()
        .find_map(|word| word.strip_prefix(CMDLINE_DPI))
        .and_then(|n| n.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_DPI)
}

/// `/dev/fb0` → `/sys/class/graphics/fb0`.
fn sysfs_dir(fb: &Path) -> Option<PathBuf> {
    let name = fb.file_name()?.to_str()?;
    name.starts_with("fb")
        .then(|| PathBuf::from("/sys/class/graphics").join(name))
}

/// 32 bytes from `/dev/random` as an [`Event::Entropy`].
///
/// The device has no `getrandom` crate; `/dev/random` is the kernel
/// CSPRNG, and since Linux 5.6 it blocks only until that CSPRNG is
/// initialised and never afterwards. The board seeds it from the SoC's
/// hardware RNG (`CONFIG_HW_RANDOM_BCM2835`), so the wait is over before
/// the first frame. A read that fails or comes up short is answered with
/// nothing at all, which the contract allows: the core then runs a weak
/// session and says so.
fn entropy() -> Option<Event> {
    let mut bytes = [0u8; 32];
    let mut file = File::open("/dev/random").ok()?;
    file.read_exact(&mut bytes).ok()?;
    Some(Event::Entropy(EntropyBytes::new(bytes)))
}

/// Appends what the shell found to the boot report `rcS` started.
///
/// A laptop has no serial port, so a dev image that will not come up has
/// nowhere to say why. `rcS` writes the kernel log to this file on the
/// exchange partition and the shell adds what it made of the machine: the
/// panel it chose, every input device with what it was classified as, and
/// every capture device. The person plugs the stick into a computer and
/// reads it. The release image passes no `--boot-report` and writes
/// nothing.
fn boot_report(
    path: &Path,
    geometry: Geometry,
    dpi: u16,
    devices: &[evdev::Device],
) -> Result<(), String> {
    let mut out = String::from("\n== opensigner-pi ==\n");
    out.push_str(&format!(
        "panel {}x{} {}bpp stride {} dpi {}\n",
        geometry.width,
        geometry.height,
        geometry.depth.bits(),
        geometry.stride,
        dpi,
    ));
    if devices.is_empty() {
        out.push_str("input: none\n");
    }
    for device in devices {
        out.push_str(&format!(
            "input {} {:?}: {}\n",
            device.node.display(),
            device.name,
            device.kinds()
        ));
    }
    let mut cameras: Vec<String> = std::fs::read_dir("/dev")
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("video"))
        .collect();
    cameras.sort();
    if cameras.is_empty() {
        out.push_str("camera: none\n");
    }
    for camera in cameras {
        out.push_str(&format!("camera /dev/{camera}\n"));
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    file.write_all(out.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// The shell: the app, the panel, the clock, and the two counters that
/// decide when to stop.
struct Shell {
    app: Option<Faraday>,
    /// Where the Inbox and Outbox are kept across a lock.
    boxes: Boxes,
    /// The sticks last reported to the app.
    sticks: Vec<StickInfo>,
    /// When the mounts are looked at next.
    next_scan: Instant,
    /// The disk process, when the image runs one.
    disk: Option<DiskProcess>,
    /// What the clean marker last said: `None` before it was written.
    clean: Option<bool>,
    /// Devices whose input is held back, and where each stands.
    held: std::collections::BTreeMap<u32, Hold>,
    /// The last frame went to the scanner inverted.
    inverted: bool,
    /// Held-back devices found before the app was there to be told.
    announce: Vec<(u32, String, bool, bool)>,
    /// The app locked: exit with the restart code.
    restart: bool,
    fb: File,
    geometry: Geometry,
    /// The converted frame, kept between draws so a redraw allocates
    /// nothing. The cursor is drawn into this copy, never into the
    /// core's frame.
    scratch: Vec<u8>,
    /// Where a mouse or a touchpad is pointing, on a machine that has
    /// one. A machine with only a touchscreen has no cursor and draws no
    /// arrow.
    cursor: Option<Cursor>,
    epoch: Instant,
    /// Frames drawn so far, for `--frames`.
    drawn: u64,
    /// `--frames N`, or `None` to run until the core exits.
    limit: Option<u64>,
    /// A `Draw` arrived since the last `flush`.
    dirty: bool,
    /// The core asked to exit.
    done: bool,
    verbose: bool,
    /// The camera channel; off until the core asks for it.
    camera: Camera,
    /// Reads the codes in the camera's frames on its own thread, so the
    /// loop keeps drawing while a frame is decoded. Alive exactly while
    /// the camera is.
    scanner: Option<Scanner>,
    /// The file channel: every mounted exchange directory.
    files: Files,
    /// What each call costs, when `--timings` asked for it.
    timings: Option<Timings>,
    /// The devices being read and the ones that turn up later. `None`
    /// off the device, where `--input` names the files that stand in for
    /// them and nothing is ever plugged in.
    watch: Option<evdev::Watch>,
    /// The sender a newly found device's reader thread is given. Held for
    /// as long as the watch is.
    tx: Option<SyncSender<Wake>>,
    /// The grid a touchscreen reports in, which a new one needs.
    grid: (u16, u16),
    /// `--pointer-speed`, which a mouse plugged in later needs.
    pointer_speed: f32,
    /// When the directory was last listed.
    scanned: Instant,
}

impl Shell {
    /// Lists `/dev/input` again and starts a reader for anything new.
    ///
    /// A device that appears is read from here on, which is what makes a
    /// keyboard plugged into a laptop after the app is up work. A mouse
    /// plugged into a machine that had none also brings the arrow with
    /// it: the cursor is made here rather than only at start-up.
    fn rescan_inputs(&mut self) {
        if self.scanned.elapsed() < evdev::RESCAN {
            return;
        }
        self.scanned = Instant::now();
        let (Some(watch), Some(tx)) = (self.watch.as_mut(), self.tx.as_ref()) else {
            return;
        };
        for (id, device) in watch.rescan() {
            if self.verbose {
                eprintln!(
                    "input {} {:?}: {} (plugged in{})",
                    device.node.display(),
                    device.name,
                    device.kinds(),
                    if device.trusted { "" } else { ", held back" }
                );
            }
            if !device.trusted {
                self.held.insert(id, Hold::Pending);
                self.announce.push((
                    id,
                    device.name.clone(),
                    device.keyboard,
                    device.mouse || device.touchpad,
                ));
            }
            if (device.mouse || device.touchpad) && self.cursor.is_none() {
                self.cursor = Some(Cursor::new(
                    self.geometry.width,
                    self.geometry.height,
                    self.pointer_speed,
                ));
                self.dirty = true;
            }
            evdev::spawn(
                id,
                device,
                self.geometry.width,
                self.geometry.height,
                self.grid,
                tx.clone(),
            );
        }
    }

    /// Records that one reader has stopped, and hides the arrow when the
    /// last thing that could move it has been unplugged.
    fn input_ended(&mut self, id: u32) {
        self.say("an input device ended");
        if self.held.remove(&id).is_some()
            && let Some(app) = self.app.as_mut()
        {
            app.storage(StorageEvent::InputGone { id });
        }
        if let Some(watch) = self.watch.as_mut() {
            watch.ended(id);
            if !watch.pointing()
                && let Some(c) = self.cursor.as_mut()
                && c.visible()
            {
                c.hide();
                self.dirty = true;
            }
        }
    }
}

impl Shell {
    fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    fn say(&self, message: &str) {
        if self.verbose {
            eprintln!("{message}");
        }
    }

    /// Converts the core's frame and writes it to the framebuffer.
    ///
    /// One `pwrite` when the rows are packed, one per row when the kernel
    /// pads them. `write_all_at` needs no seek and no `mmap`, so the whole
    /// path stays inside safe `std`.
    /// Paints the panel if a `Draw` is pending. Called once per loop
    /// pass, after every event of the pass has gone in.
    fn flush(&mut self) {
        if self.dirty {
            self.dirty = false;
            self.draw();
        }
    }

    fn draw(&mut self) {
        let g = self.geometry;
        let pixels = usize::from(g.width) * usize::from(g.height);
        let bytes = pixels * g.depth.bytes_per_pixel();
        let mut scratch = std::mem::take(&mut self.scratch);
        if scratch.len() != bytes {
            scratch = vec![0u8; bytes];
        }

        // The frame is painted when it is read, so the app is borrowed
        // mutably here and anything to say waits until it is given back.
        let mut complaint = None;
        let converted = match self.app.as_mut().map(App::frame) {
            Some(frame) if (frame.width, frame.height) != (g.width, g.height) => {
                complaint = Some("frame does not match the panel; not drawing");
                false
            }
            Some(frame) if frame.rgba.len() < pixels * 4 => {
                complaint = Some("short frame; not drawing");
                false
            }
            Some(frame) => {
                let start = self.timings.is_some().then(Instant::now);
                fb::convert_into(&frame.rgba[..pixels * 4], g.depth, &mut scratch);
                if let (Some(start), Some(t)) = (start, self.timings.as_mut()) {
                    t.add(Bucket::Convert, start.elapsed());
                }
                true
            }
            None => false,
        };
        if let Some(message) = complaint {
            self.say(message);
        }
        if !converted {
            self.scratch = scratch;
            return;
        }

        if let Some(c) = self.cursor.as_ref()
            && c.visible()
        {
            cursor::draw(&mut scratch, g, c.at());
        }

        let row = g.row_bytes();
        let start = self.timings.is_some().then(Instant::now);
        let result = if g.stride == row {
            self.fb.write_all_at(&scratch, 0)
        } else {
            (0..usize::from(g.height)).try_for_each(|y| {
                self.fb
                    .write_all_at(&scratch[y * row..(y + 1) * row], (y * g.stride) as u64)
            })
        };
        if let (Some(start), Some(t)) = (start, self.timings.as_mut()) {
            t.add(Bucket::Write, start.elapsed());
        }
        match result {
            Ok(()) => self.drawn += 1,
            Err(e) => self.say(&format!("framebuffer write failed: {e}")),
        }
        self.scratch = scratch;
    }

    /// True once enough frames have been drawn for `--frames`.
    fn enough(&self) -> bool {
        self.limit.is_some_and(|n| self.drawn >= n)
    }

    /// Delivers one contact, from a finger or from the pointer, and
    /// counts it.
    fn touch(&mut self, t: touch::Touch) {
        self.send_timed(
            Bucket::Touch,
            Event::Touch {
                x: t.x,
                y: t.y,
                phase: t.phase,
            },
        );
        if let Some(timings) = self.timings.as_mut() {
            timings.touch();
        }
    }

    /// Delivers one event and records what it cost, when `--timings`
    /// asked. The clock is read only then, so a release card pays for
    /// none of this.
    fn send_timed(&mut self, bucket: Bucket, event: Event) {
        let start = self.timings.is_some().then(Instant::now);
        self.send(event);
        if let (Some(start), Some(t)) = (start, self.timings.as_mut()) {
            t.add(bucket, start.elapsed());
        }
    }

    /// Appends a block to the card if one is due, or the last one at
    /// exit. A card that cannot be written is one line for `--verbose`:
    /// the timings are a diagnostic and never worth stopping for.
    fn report(&mut self, last: bool) {
        let elapsed = self.epoch.elapsed();
        let Some(t) = self.timings.as_mut() else {
            return;
        };
        let block = if last {
            Some(t.last(elapsed))
        } else {
            t.due(elapsed)
        };
        if let Some(block) = block
            && let Err(reason) = self.files.append_timings(&block)
        {
            // With no exchange partition mounted (Faraday mounts none)
            // the timings have nowhere to go; that is said once.
            if self.timings.take().is_some() {
                self.say(&format!("timings off: {reason}"));
            }
        }
    }

    /// Delivers one event and acts on every command it produces, in order.
    ///
    /// A request the shell answers becomes another event at the back of
    /// the queue, so its own commands are handled too — the same shape as
    /// the desktop and snapshot shells.
    fn send(&mut self, event: Event) {
        let mut pending = std::collections::VecDeque::from([event]);
        while let Some(e) = pending.pop_front() {
            let Some(app) = self.app.as_mut() else {
                return;
            };
            app.event(e);
            while let Some(c) = self.app.as_mut().and_then(|a| a.poll_command()) {
                match c {
                    // Drawn once per loop pass by `flush`, however many
                    // events the pass delivered: a flick is dozens of
                    // moves, and the panel cannot be painted for each.
                    Command::Draw => self.dirty = true,
                    // No motor on this build.
                    Command::Vibrate { .. } => {}
                    // No clipboard on a panel with no window system:
                    // the rows say so and stay dead.
                    Command::RequestClipboard { kind } => {
                        pending.push_back(Event::ClipboardUnavailable { kind });
                    }
                    Command::WriteClipboard { kind, .. } => {
                        pending.push_back(Event::ClipboardNotWritten { kind });
                    }
                    Command::Exit => self.done = true,
                    // The boot medium's OSKDATA partition at
                    // /mnt/microsd and every FAT partition init mounted
                    // under /mnt/usb, looked at again for each request
                    // so a stick plugged in a moment ago is on the
                    // list: one answer per request, carrying the names
                    // a person finds those partitions by, and the
                    // reason for an empty one only where --verbose can
                    // see it.
                    Command::RequestFile { kind } => {
                        let event = match self.files.list(kind) {
                            Ok(entries) => Event::FileList {
                                kind,
                                entries,
                                place: self.files.place(),
                            },
                            Err(reason) => {
                                self.say(&reason);
                                Event::FileUnavailable { kind }
                            }
                        };
                        pending.push_back(event);
                    }
                    // One of the names the listing carried, which the
                    // person chose on the core's Files screen.
                    Command::ReadFile { kind, name } => {
                        let event = match self.files.read(kind, &name) {
                            Ok(bytes) => Event::File { kind, bytes },
                            Err(reason) => {
                                self.say(&reason);
                                Event::FileUnavailable { kind }
                            }
                        };
                        pending.push_back(event);
                    }
                    Command::WriteFile {
                        kind,
                        name_hint,
                        bytes,
                    } => {
                        let event = match self.files.write(&name_hint, &bytes) {
                            Ok(path) => {
                                self.say(&format!("wrote {}", path.display()));
                                Event::FileWritten { kind }
                            }
                            Err(reason) => {
                                self.say(&reason);
                                Event::FileNotWritten { kind }
                            }
                        };
                        pending.push_back(event);
                    }
                    // The settings, kept beside the user's files under a
                    // name of this shell's choosing.
                    Command::StoreSettings { bytes } => match self.files.write_settings(&bytes) {
                        Ok(path) => self.say(&format!("wrote {}", path.display())),
                        Err(reason) => self.say(&reason),
                    },
                    Command::CameraOn => {
                        // Which cameras there are, for the scan sheet's
                        // choice, as of the moment it opens.
                        if let Some(app) = self.app.as_mut() {
                            app.storage(StorageEvent::Cameras(camera::list()));
                        }
                        if let Err(reason) = self.camera.on() {
                            self.say(&format!("camera unavailable: {reason}"));
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
                        } else {
                            self.say("no entropy from /dev/random; the session is weak");
                        }
                    }
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
            if let Some(app) = self.app.as_mut() {
                let said = match self.disk.as_mut() {
                    Some(disk) => serve_with(app, &mut self.boxes, None, disk),
                    None => serve(app, &mut self.boxes, None),
                };
                for line in said {
                    if self.verbose {
                        eprintln!("{line}");
                    }
                }
                if app.restart_requested() {
                    self.restart = true;
                }
            }
            self.publish_clean();
            self.take_input_decisions();
        }
    }

    /// Tells the app about held-back devices it has not heard of, and takes
    /// what it decided about others.
    fn take_input_decisions(&mut self) {
        let Some(app) = self.app.as_mut() else { return };
        for (id, name, keyboard, pointer) in self.announce.drain(..) {
            if self.verbose {
                eprintln!("input {id} {name:?} held back until a person says it is theirs");
            }
            app.storage(StorageEvent::NewInput {
                id,
                name,
                keyboard,
                pointer,
            });
        }
        while let Some((id, believed)) = app.poll_input_decision() {
            if let Some(h) = self.held.get_mut(&id) {
                *h = if believed {
                    Hold::Believed
                } else {
                    Hold::Ignored
                };
            }
        }
    }

    /// What a held-back device sent: through, once believed; its typed
    /// characters to the app, while it waits; nothing, once ignored.
    fn held_wake(&mut self, id: u32, input: HeldInput, batch: &mut Vec<Wake>) {
        match self.held.get(&id) {
            Some(Hold::Believed) => batch.push(input.wake()),
            Some(Hold::Pending) => {
                if let HeldInput::Key(Key::Char(ch)) = input
                    && let Some(app) = self.app.as_mut()
                {
                    app.storage(StorageEvent::InputTyped { id, ch });
                    self.dirty = true;
                    self.take_input_decisions();
                }
            }
            _ => {}
        }
    }

    /// Writes or removes the clean marker when the app's state changes.
    /// A process that is not clean, or about to restart, never leaves the
    /// marker behind.
    fn publish_clean(&mut self) {
        let Some(app) = self.app.as_mut() else { return };
        let clean = app.clean() && !app.restart_requested();
        if self.clean == Some(clean) {
            return;
        }
        let done = if clean {
            std::fs::write(CLEAN_MARKER, b"clean\n")
        } else {
            match std::fs::remove_file(CLEAN_MARKER) {
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                r => r,
            }
        };
        match done {
            Ok(()) => self.clean = Some(clean),
            // Off the device there is no marker directory; nothing waits
            // on it there.
            Err(e) => {
                if self.verbose && self.clean.is_none() {
                    eprintln!("clean marker {CLEAN_MARKER}: {e}");
                }
                self.clean = Some(clean);
            }
        }
    }

    /// The sticks: from the disk process when there is one, else the boot
    /// medium's data partition and every partition init mounted under
    /// `/mnt/usb`, as `/proc/mounts` lists them now.
    fn mounted_sticks(&mut self) -> Vec<StickInfo> {
        if let Some(disk) = self.disk.as_mut() {
            return match disk.list() {
                Ok(s) => s,
                Err(e) => {
                    if self.verbose {
                        eprintln!("disk process: {e}");
                    }
                    self.sticks.clone()
                }
            };
        }
        let mounts = std::fs::read_to_string("/proc/mounts").unwrap_or_default();
        let mut sticks = Vec::new();
        for line in mounts.lines() {
            let point = line.split_whitespace().nth(1).unwrap_or("");
            if point == DEFAULT_FILES {
                sticks.push(stick_info(Path::new(point), "Boot stick", true));
            } else if let Some(name) = point
                .strip_prefix(USB_MOUNTS)
                .and_then(|p| p.strip_prefix('/'))
            {
                // init names the mount after the volume label, or after the
                // device when the stick has none.
                let label = if name.starts_with("sd")
                    && name[2..].chars().skip(1).all(|c| c.is_ascii_digit())
                {
                    format!("USB stick ({name})")
                } else {
                    name.to_string()
                };
                sticks.push(stick_info(Path::new(point), &label, false));
            }
        }
        sticks
    }

    /// Tells the app when the sticks or their files change.
    fn scan_sticks(&mut self, force: bool) {
        if !force && Instant::now() < self.next_scan {
            return;
        }
        self.next_scan = Instant::now() + STICK_SCAN;
        let now = self.mounted_sticks();
        if !force && now == self.sticks {
            return;
        }
        if self.verbose {
            let said: Vec<String> = now
                .iter()
                .map(|s| format!("{} ({}, {} files)", s.id, s.label, s.files.len()))
                .collect();
            eprintln!("sticks: [{}]", said.join(", "));
        }
        self.sticks = now.clone();
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
        }
        // What the app asked for in answer, and the redraw.
        self.send(Event::Tick {
            now_ms: self.epoch.elapsed().as_millis() as u64,
        });
    }
}

fn run(args: Args) -> Result<bool, String> {
    let geometry = geometry(&args, probe(&args))?;
    let mut dpi = dpi(&args, &std::fs::read_to_string(CMDLINE).unwrap_or_default());
    // The kernel command line gives one density for every laptop; a panel
    // this wide is a high-density one, and Settings can still change it.
    if args.dpi.is_none() && geometry.width >= 2800 {
        dpi = dpi.saturating_mul(2);
    }
    let fb = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&args.fb)
        .map_err(|e| format!("cannot open {}: {e}", args.fb.display()))?;

    // One channel the loop waits on. The touch reader owns the input
    // device on its own thread and sends the contacts it decodes; the
    // capture thread sends a note for each frame it queues.
    let (tx, rx) = sync_channel(WAKES);
    // `--input` is a review flag: each one names a file of recorded
    // packets standing in for a device, so the shell can be driven on a
    // build box that has none. It also says there is no directory to
    // watch: the files are the devices, and nothing is plugged in later.
    let mut watch = args.input.is_empty().then(|| {
        evdev::Watch::new(
            Path::new("/sys/class/input"),
            Path::new("/dev/input"),
            &args.touch_name,
        )
    });
    let found: Vec<(u32, evdev::Device)> = match watch.as_mut() {
        Some(watch) => watch.rescan(),
        None => args
            .input
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, d)| (i as u32, d))
            .collect(),
    };
    let devices: Vec<evdev::Device> = found.iter().map(|(_, d)| d.clone()).collect();
    let pointing = devices.iter().any(|d| d.mouse || d.touchpad);
    // The camera's own sender, taken before the readers' are given away:
    // with no input device the loop still has a source and still wakes on
    // frames.
    let camera = Camera::new(args.camera.clone(), args.no_camera, tx.clone());
    // Devices already plugged in that are not part of the machine: held
    // back from the start, and announced once the app is up.
    let initial: Vec<(u32, String, bool, bool)> = found
        .iter()
        .filter(|(_, d)| !d.trusted)
        .map(|(id, d)| (*id, d.name.clone(), d.keyboard, d.mouse || d.touchpad))
        .collect();
    for (id, device) in found {
        evdev::spawn(
            id,
            device,
            geometry.width,
            geometry.height,
            args.touch_grid,
            tx.clone(),
        );
    }
    // On the device the sender is kept, because a device plugged in later
    // needs one. Off it nothing more can appear, so the last sender goes
    // and a loop with no source left stops waiting on it.
    let tx = watch.is_some().then_some(tx);

    let mut shell = Shell {
        app: Some(Faraday::new()),
        boxes: Boxes::Dir(PathBuf::from(BOXES)),
        sticks: Vec::new(),
        next_scan: Instant::now(),
        disk: if Path::new(DISK_REQUESTS).exists() {
            DiskProcess::open(Path::new(DISK_REQUESTS), Path::new(DISK_RESPONSES)).ok()
        } else {
            None
        },
        clean: None,
        held: initial
            .iter()
            .map(|(id, ..)| (*id, Hold::Pending))
            .collect(),
        announce: initial,
        inverted: false,
        restart: false,
        fb,
        geometry,
        scratch: Vec::new(),
        cursor: pointing.then(|| Cursor::new(geometry.width, geometry.height, args.pointer_speed)),
        epoch: Instant::now(),
        drawn: 0,
        limit: args.frames,
        done: false,
        dirty: false,
        verbose: args.verbose,
        camera,
        scanner: None,
        files: Files::new(args.files.clone(), !args.files_given),
        timings: args
            .timings
            .then(|| Timings::new(env!("CARGO_PKG_VERSION"), geometry.width, geometry.height)),
        watch,
        tx,
        grid: args.touch_grid,
        pointer_speed: args.pointer_speed,
        scanned: Instant::now(),
    };
    if args.verbose {
        eprintln!(
            "panel {}x{} {}bpp stride {} dpi {}; touch name {:?} grid {}x{}",
            geometry.width,
            geometry.height,
            geometry.depth.bits(),
            geometry.stride,
            dpi,
            args.touch_name,
            args.touch_grid.0,
            args.touch_grid.1,
        );
        if devices.is_empty() {
            eprintln!("input: none");
        }
        for device in &devices {
            eprintln!(
                "input {} {:?}: {}",
                device.node.display(),
                device.name,
                device.kinds()
            );
        }
    }

    if let Some(path) = args.boot_report.as_ref()
        && let Err(reason) = boot_report(path, geometry, dpi, &devices)
    {
        shell.say(&format!("no boot report: {reason}"));
    }

    shell.send(Event::Display(DisplayInfo {
        width: geometry.width,
        height: geometry.height,
        dpi,
        // The panel is the whole display: no gesture bar, no cutout, and
        // the frame the core draws is every pixel a person can see.
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        // The camera is a module in a case someone built, mounted
        // whichever way the case allows.
        camera_fixed: false,
        secure: SecureHardware::None,
        // The board has no attestation to say how it came up.
        boot: BootState::Unknown,
        memory_mib: total_memory_mib(),
    }));
    // What the process before a lock left in the Inbox and Outbox, then
    // the sticks present now.
    let restored = shell.boxes.restore();
    if let Some(app) = shell.app.as_mut() {
        app.storage(restored);
    }
    shell.scan_sticks(true);
    shell.flush();

    main_loop(&mut shell, &rx);
    // The last screen the core asked for, if the loop ended on an event.
    shell.flush();
    // Everything the last interval measured, before the machine goes.
    shell.report(true);

    // The capture thread first: joining it closes the device.
    shell.camera.off();
    // Drop the app before the process ends so its zeroizing types run.
    let exiting = shell.done;
    let restart = shell.restart;
    shell.app = None;
    if exiting && !restart {
        // The last screen stays up long enough to be read, and then the
        // process ends. Powering the board off is init's: `poweroff -f`
        // is the inittab line after this one, and the app, which is not
        // root, could not do it anyway.
        std::thread::sleep(GOODBYE);
    }
    Ok(restart)
}

/// Drops every move but the last of a run of them.
///
/// A controller reports a moving finger a hundred times a second and a
/// mouse reports motion as fast as it is polled; only the newest position
/// of a run matters, so a flick is a handful of events and not a queue
/// that replays for seconds after the finger lifts. A pointer's moves are
/// deltas, so the dropped ones are added into the one that is kept.
fn collapse_moves(batch: &mut Vec<Wake>) {
    let mut i = 0;
    while i + 1 < batch.len() {
        match (batch[i], batch[i + 1]) {
            (Wake::Touch(a), Wake::Touch(b))
                if a.phase == TouchPhase::Move && b.phase == TouchPhase::Move =>
            {
                batch.remove(i);
            }
            (
                Wake::Pointer(pointer::Motion::Move { dx, dy }),
                Wake::Pointer(pointer::Motion::Move { dx: bx, dy: by }),
            ) => {
                batch[i + 1] = Wake::Pointer(pointer::Motion::Move {
                    dx: dx + bx,
                    dy: dy + by,
                });
                batch.remove(i);
            }
            (
                Wake::Pointer(pointer::Motion::Pan { dy }),
                Wake::Pointer(pointer::Motion::Pan { dy: by }),
            ) => {
                batch[i + 1] = Wake::Pointer(pointer::Motion::Pan { dy: dy + by });
                batch.remove(i);
            }
            _ => i += 1,
        }
    }
}

/// Waits to be woken, folds in the newest camera frame, ticks when
/// nothing wakes it, and stops when the core exits or `--frames` is
/// satisfied.
///
/// A touch and a frame wake the same wait, so a frame that arrives at
/// the start of a wait is drawn then and not up to a tick later: the
/// preview runs at the rate it costs. The frame itself is still taken
/// from the camera's channel, one per pass and the newest one, so a
/// board that decodes slower than it captures still ticks and still
/// feels a finger.
fn main_loop(shell: &mut Shell, rx: &Receiver<Wake>) {
    // Set once nothing can wake the loop again: there is nothing to wait
    // on, so the pass sleeps out the tick interval instead of spinning on
    // an instantly-failing receive.
    let mut unwoken = false;
    // Set between a contact's Down and its Up: the pass waits for the
    // next move rather than for the tick, so a drag moves with the
    // finger.
    let mut contact = false;
    while !shell.done && !shell.enough() {
        // Everything the controller has reported since the last pass
        // goes in now. It reports a moving finger a hundred times a
        // second; only the newest position of a run of moves matters, so
        // the rest are dropped and a flick is a handful of events, not a
        // queue that replays for seconds after the finger lifts.
        let mut batch: Vec<Wake> = Vec::new();
        // The readers whose devices went away during this wait.
        let mut ended: Vec<u32> = Vec::new();
        if unwoken {
            std::thread::sleep(TICK);
        } else {
            let wait = if contact { DRAG_TICK } else { TICK };
            match rx.recv_timeout(wait) {
                Ok(first) => {
                    let mut wakes = vec![first];
                    while let Ok(w) = rx.try_recv() {
                        wakes.push(w);
                    }
                    for wake in wakes {
                        match wake {
                            // The frame is taken below, from the
                            // camera's channel; the note only ended the
                            // wait.
                            Wake::Frame => {}
                            Wake::InputEnded(id) => ended.push(id),
                            Wake::Held(id, input) => shell.held_wake(id, input, &mut batch),
                            other => batch.push(other),
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => unwoken = true,
            }
        }
        for id in ended {
            shell.input_ended(id);
        }
        // Whatever has been plugged in since the last look, at most once
        // every couple of seconds.
        shell.rescan_inputs();
        collapse_moves(&mut batch);
        for wake in batch {
            match wake {
                Wake::Touch(t) => {
                    // A finger on the screen is where the person is
                    // looking; the arrow gets out of the way.
                    if let Some(c) = shell.cursor.as_mut() {
                        c.hide();
                    }
                    contact = t.phase != TouchPhase::Up;
                    shell.touch(t);
                }
                Wake::Pointer(motion) => {
                    let Some(c) = shell.cursor.as_mut() else {
                        continue;
                    };
                    let pointed = c.apply(motion);
                    // The arrow has moved, so the panel is repainted even
                    // when the core asks for nothing.
                    shell.dirty = true;
                    match pointed {
                        Some(Pointed::Touch(t)) => {
                            contact = t.phase != TouchPhase::Up;
                            shell.touch(t);
                        }
                        // A tap is a contact that arrives whole: down
                        // and up at one place, with nothing between.
                        Some(Pointed::Click { x, y }) => {
                            contact = false;
                            for phase in [TouchPhase::Down, TouchPhase::Up] {
                                shell.touch(touch::Touch { x, y, phase });
                            }
                        }
                        Some(Pointed::Scroll { x, y, dy }) => {
                            shell.send_timed(Bucket::Touch, Event::Scroll { x, y, dy });
                        }
                        None => {}
                    }
                }
                Wake::Key(key) => shell.send_timed(Bucket::Touch, Event::Key(key)),
                Wake::KeyUp(key) => shell.send_timed(Bucket::Touch, Event::KeyUp(key)),
                Wake::Frame | Wake::InputEnded(_) | Wake::Held(..) => {}
            }
            if shell.done {
                break;
            }
        }
        if shell.done {
            break;
        }
        // One frame per pass, and the newest one. Decoding a frame costs
        // the core longer than the tenth of a second between captures on
        // a 3B+, so a loop that sent every waiting frame would never
        // empty the channel and never reach the tick, the touches or the
        // flush: the Scan screen froze while QR codes still decoded. The
        // frames behind the newest are stale by definition.
        let mut newest = None;
        // A camera that would not open, which the capture thread
        // reports once. It is not a frame and is not dropped for a
        // newer one.
        let mut unavailable = false;
        while let Some(capture) = shell.camera.poll() {
            match capture {
                Capture::Frame(shot) => newest = Some(shot),
                Capture::Unavailable => unavailable = true,
            }
        }
        if unavailable {
            if let Some(reason) = shell.camera.take_reason() {
                shell.say(&format!("camera unavailable: {reason}"));
            }
            shell.scanner = None;
            shell.send(Event::CameraUnavailable);
        }
        if let Some(shot) = newest {
            // The capture thread reduced the frame once, so each half
            // goes where it is wanted and nothing is copied: the
            // captured luma to the scanner, which reads the codes in
            // it, and the reduced frame to the core, which draws it.
            if let Some(scanner) = shell.scanner.as_ref() {
                // Every other frame goes to the scanner inverted, so a
                // code printed light on dark reads too (`docs/QR.md` §1).
                shell.inverted = !shell.inverted;
                let mut luma = shot.full_luma;
                if shell.inverted {
                    luma.iter_mut().for_each(|b| *b = 255 - *b);
                }
                scanner.offer(shot.full_width, shot.full_height, luma);
            }
            shell.send_timed(
                Bucket::Camera,
                Event::CameraFrame {
                    width: shot.width,
                    height: shot.height,
                    luma: shot.luma,
                    chroma: shot.chroma,
                },
            );
        }
        // Whatever the worker read since the last pass. The core routes
        // each code as it routes one from a file.
        while let Some(bytes) = shell.scanner.as_ref().and_then(Scanner::poll) {
            shell.send_timed(Bucket::Scanned, Event::Scanned { bytes });
        }
        // What the scanner saw of a code, for the outline and for "too
        // fine for this camera".
        if let Some(seen) = shell.scanner.as_ref().and_then(Scanner::poll_seen)
            && let Some(app) = shell.app.as_mut()
        {
            app.storage(StorageEvent::QrSeen {
                width: seen.width,
                height: seen.height,
                corners: seen.corners,
                module_tenths: (seen.module_px * 10.0).clamp(0.0, 65535.0) as u16,
                read: seen.read,
            });
        }
        // A camera the person chose on the scan sheet.
        if let Some(id) = shell
            .app
            .as_mut()
            .and_then(faraday_core::Faraday::take_camera)
        {
            shell.camera.choose(std::path::PathBuf::from(id));
        }
        if !shell.done {
            let now_ms = shell.now_ms();
            shell.send_timed(Bucket::Tick, Event::Tick { now_ms });
        }
        shell.flush();
        if let Some(timings) = shell.timings.as_mut() {
            timings.pass();
        }
        // The first page the kernel would not pin, said once and only
        // with --verbose: it is a machine configuration a developer can
        // act on and the person using the device cannot (§16.49). The
        // release image has no swap and no console, so on a card this
        // line is neither needed nor seen.
        if osk_crypto::pin_failure_to_report() {
            shell.say("the kernel would not lock secret pages into RAM (RLIMIT_MEMLOCK)");
        }
        shell.report(false);
        shell.scan_sticks(false);
    }
}

fn main() -> ExitCode {
    // Nothing reaches stdout or stderr without --verbose, on the device or
    // off it, so a usage error is an exit code and not a message.
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            if std::env::args().any(|a| a == "--verbose") {
                eprintln!("opensigner-pi: {e}");
            }
            return ExitCode::from(2);
        }
    };
    let verbose = args.verbose;
    match run(args) {
        Ok(true) => ExitCode::from(faraday_core::RESTART_CODE),
        Ok(false) => ExitCode::SUCCESS,
        Err(e) => {
            if verbose {
                eprintln!("opensigner-pi: {e}");
            }
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

    #[test]
    fn a_framebuffer_that_cannot_be_described_is_not_guessed_at() {
        // On the device: sysfs is there and says something this shell
        // cannot write. The shell stops and says why, rather than drawing
        // a 480x640 frame into the corner of a laptop screen.
        let reason = "fb0 is 30 bits per pixel, which this shell cannot write";
        assert_eq!(
            geometry(&Args::default(), Err(String::from(reason))),
            Err(String::from(reason))
        );
        // Even with a size and a depth on the command line: a framebuffer
        // that cannot describe itself is not a framebuffer to write to.
        let told = Args {
            size: Some((1366, 768)),
            depth: Some(Depth::Bgra8888),
            ..Args::default()
        };
        assert_eq!(
            geometry(&told, Err(String::from(reason))),
            Err(String::from(reason))
        );

        // Off the device, where `--fb` names a file, the default panel is
        // still what a run with no size means.
        let off = Args {
            off_device: true,
            ..Args::default()
        };
        assert_eq!(
            geometry(&off, Ok(None)),
            Ok(Geometry {
                width: DEFAULT_WIDTH,
                height: DEFAULT_HEIGHT,
                depth: Depth::Rgb565,
                stride: usize::from(DEFAULT_WIDTH) * 2,
            })
        );
    }
}
