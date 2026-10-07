//! evdev: the codes on the wire, the capability bitmaps in sysfs, and the
//! devices this shell reads.
//!
//! A laptop booted from the stick has whatever the person plugged in: a
//! built-in keyboard, a touchpad, a USB mouse, sometimes a touchscreen.
//! The Pi has one panel controller and nothing else. Both are the same
//! problem — open every `/dev/input/event*`, ask each what it can report,
//! and read the ones that say something this shell understands.
//!
//! **What a device is** is read from
//! `/sys/class/input/eventN/device/capabilities/{ev,key,rel,abs}` and
//! `…/device/properties`, which the kernel prints as hexadecimal bitmaps.
//! That is text: no `EVIOCGBIT` ioctl, no `libc` and no `unsafe`, which
//! is the rule this crate is built under. The four rules are:
//!
//! - a **keyboard** reports `EV_KEY` with the letter keys;
//! - a **mouse** reports `EV_REL` with `REL_X` and `REL_Y`;
//! - a **touchpad** reports `EV_ABS` with `ABS_X` and `ABS_Y` and either
//!   `BTN_TOOL_FINGER` or `INPUT_PROP_POINTER`, and is not
//!   `INPUT_PROP_DIRECT`;
//! - a **touchscreen** reports `ABS_MT_POSITION_X` or is
//!   `INPUT_PROP_DIRECT`, and is not a touchpad — a pad reports
//!   multitouch too, and the finger on it is a pointer and not a place on
//!   the screen.
//!
//! One device may be several: a wireless receiver is one node that is
//! both a keyboard and a mouse, and both are read.
//!
//! `--touch-name` still wins for touch: a device whose name holds it is
//! the touchscreen and no other device is, which is how the Pi's panel is
//! found and how a laptop with a touchscreen the classification misreads
//! can be told what it has.
//!
//! **Devices plugged in after start are found.** [`Watch`] lists
//! `/dev/input` again every couple of seconds and classifies whatever is
//! new, so a keyboard plugged into a laptop after the app is up starts
//! typing. A device that is unplugged ends its reader thread, and its node
//! going from the listing is what drops it. That is a listing and a
//! `read_dir`: no netlink socket, no hotplug helper, no ioctl.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;
use std::thread;

use crate::Wake;
use crate::keyboard::{Keyboard, Stroke};
use crate::pointer::Pointer;
use crate::touch;

// linux/input-event-codes.h
/// Synchronisation events; `SYN_REPORT` ends a packet.
pub const EV_SYN: u16 = 0x00;
/// Keys and buttons.
pub const EV_KEY: u16 = 0x01;
/// Relative axes: a mouse's motion and its wheel.
pub const EV_REL: u16 = 0x02;
/// Absolute axes: a touch controller's or a touchpad's positions.
pub const EV_ABS: u16 = 0x03;
/// The end of a packet.
pub const SYN_REPORT: u16 = 0x00;

/// `KEY_A`, one of the two letters a keyboard is recognised by.
pub const KEY_A: u16 = 30;
/// `KEY_Z`, the other.
pub const KEY_Z: u16 = 44;
/// The left mouse button, and a touchpad's own button.
pub const BTN_LEFT: u16 = 0x110;
/// A touchpad saying one finger is on it.
pub const BTN_TOOL_FINGER: u16 = 0x145;
/// Contact: a finger is down.
pub const BTN_TOUCH: u16 = 0x14a;
/// A touchpad saying two fingers are on it, which is the scroll gesture.
pub const BTN_TOOL_DOUBLETAP: u16 = 0x14d;

/// Relative x: mouse motion across.
pub const REL_X: u16 = 0x00;
/// Relative y: mouse motion down.
pub const REL_Y: u16 = 0x01;
/// The wheel, in notches.
pub const REL_WHEEL: u16 = 0x08;
/// The wheel, in 120ths of a notch.
pub const REL_WHEEL_HI_RES: u16 = 0x0b;

/// Absolute x.
pub const ABS_X: u16 = 0x00;
/// Absolute y.
pub const ABS_Y: u16 = 0x01;
/// The contact the following `ABS_MT_*` events belong to.
pub const ABS_MT_SLOT: u16 = 0x2f;
/// Multitouch x for the current contact.
pub const ABS_MT_POSITION_X: u16 = 0x35;
/// Multitouch y for the current contact.
pub const ABS_MT_POSITION_Y: u16 = 0x36;
/// The contact's tracking id; −1 closes it.
pub const ABS_MT_TRACKING_ID: u16 = 0x39;

/// `INPUT_PROP_POINTER`: the device moves a pointer rather than naming a
/// place on the screen.
pub const INPUT_PROP_POINTER: u16 = 0x00;
/// `INPUT_PROP_DIRECT`: the device is on the screen it reports positions
/// in.
pub const INPUT_PROP_DIRECT: u16 = 0x01;

/// `struct input_event` on a 32-bit target: two 32-bit `timeval` fields,
/// then `u16` type, `u16` code, `i32` value.
pub const EVENT_SIZE_32: usize = 16;
/// The same struct on a 64-bit target, where `timeval` is two 64-bit
/// fields.
pub const EVENT_SIZE_64: usize = 24;

/// The size of one `struct input_event` on this build. The Pi runs a
/// 32-bit ARM kernel and userland, so on it this is 16; on a laptop and
/// on a build box it is 24, which is why the parser is written against a
/// size rather than a layout.
pub const EVENT_SIZE: usize = if cfg!(target_pointer_width = "64") {
    EVENT_SIZE_64
} else {
    EVENT_SIZE_32
};

/// The four fields of a `struct input_event` this shell cares about. The
/// timestamp is the kernel's, and only differences between two of them
/// are ever used, so which clock it came from does not matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawEvent {
    /// When the kernel stamped the event, in milliseconds.
    pub time_ms: u64,
    /// `EV_ABS`, `EV_KEY`, `EV_SYN`, …
    pub kind: u16,
    /// The axis or button within that type.
    pub code: u16,
    /// The value: a coordinate, a tracking id, or a button state.
    pub value: i32,
}

/// Decodes one `struct input_event` of `size` bytes.
///
/// The type, code and value are the last eight bytes whatever the size of
/// the timestamp in front of them, which is what makes one decoder serve
/// both the 16-byte and the 24-byte layout. The timestamp is the two
/// `timeval` fields in front of them: two `u32` on the 16-byte layout,
/// two `u64` on the 24-byte one. Little-endian: every target this runs on
/// (armv7 and x86-64) is.
pub fn decode(bytes: &[u8], size: usize) -> Option<RawEvent> {
    if size < 8 || bytes.len() < size {
        return None;
    }
    let head = &bytes[..size - 8];
    let (secs, micros) = match head.len() {
        8 => (
            u64::from(u32::from_le_bytes([head[0], head[1], head[2], head[3]])),
            u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]])),
        ),
        16 => (
            u64::from_le_bytes(head[..8].try_into().ok()?),
            u64::from_le_bytes(head[8..16].try_into().ok()?),
        ),
        _ => (0, 0),
    };
    let tail = &bytes[size - 8..size];
    Some(RawEvent {
        time_ms: secs.wrapping_mul(1000).wrapping_add(micros / 1000),
        kind: u16::from_le_bytes([tail[0], tail[1]]),
        code: u16::from_le_bytes([tail[2], tail[3]]),
        value: i32::from_le_bytes([tail[4], tail[5], tail[6], tail[7]]),
    })
}

/// Whether `bit` is set in a kernel capability bitmap.
///
/// The kernel prints a bitmap as space-separated hexadecimal words, most
/// significant first, each one an `unsigned long`. Its width is the
/// kernel's, and this binary runs in the same word size as the kernel it
/// is built for, so [`usize::BITS`] is the right divisor.
pub fn has_bit(text: &str, bit: u16) -> bool {
    let words: Vec<u64> = text
        .split_whitespace()
        .filter_map(|w| u64::from_str_radix(w, 16).ok())
        .collect();
    let bits = usize::BITS as usize;
    let (word, shift) = (usize::from(bit) / bits, usize::from(bit) % bits);
    match words.len().checked_sub(word + 1) {
        Some(i) => words[i] >> shift & 1 == 1,
        None => false,
    }
}

/// What one device can report: the five sysfs bitmaps, as the kernel
/// printed them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Caps {
    /// `capabilities/ev`: which event types the device sends at all.
    pub ev: String,
    /// `capabilities/key`: keys and buttons.
    pub key: String,
    /// `capabilities/rel`: relative axes.
    pub rel: String,
    /// `capabilities/abs`: absolute axes.
    pub abs: String,
    /// `properties`: `INPUT_PROP_*`.
    pub props: String,
}

impl Caps {
    /// Reads the five files under a `/sys/class/input/eventN` directory.
    /// A file that is not there is an empty bitmap, which sets no bit.
    pub fn read(dir: &Path) -> Caps {
        let device = dir.join("device");
        let caps = device.join("capabilities");
        let read = |path: PathBuf| fs::read_to_string(path).unwrap_or_default();
        Caps {
            ev: read(caps.join("ev")),
            key: read(caps.join("key")),
            rel: read(caps.join("rel")),
            abs: read(caps.join("abs")),
            props: read(device.join("properties")),
        }
    }

    /// Whether the device is a keyboard: keys, and the letters among them.
    pub fn keyboard(&self) -> bool {
        has_bit(&self.ev, EV_KEY) && has_bit(&self.key, KEY_A) && has_bit(&self.key, KEY_Z)
    }

    /// Whether the device is a mouse: relative motion on both axes.
    pub fn mouse(&self) -> bool {
        has_bit(&self.ev, EV_REL) && has_bit(&self.rel, REL_X) && has_bit(&self.rel, REL_Y)
    }

    /// Whether the device is a touchpad: absolute positions that move a
    /// pointer rather than name a place on the screen.
    pub fn touchpad(&self) -> bool {
        has_bit(&self.ev, EV_ABS)
            && has_bit(&self.abs, ABS_X)
            && has_bit(&self.abs, ABS_Y)
            && (has_bit(&self.key, BTN_TOOL_FINGER) || has_bit(&self.props, INPUT_PROP_POINTER))
            && !has_bit(&self.props, INPUT_PROP_DIRECT)
    }

    /// Whether the device is a touchscreen: multitouch, or a device on
    /// the screen it reports in, and not a pad.
    pub fn touchscreen(&self) -> bool {
        !self.touchpad()
            && (has_bit(&self.abs, ABS_MT_POSITION_X) || has_bit(&self.props, INPUT_PROP_DIRECT))
    }

    /// Whether the wheel reports 120ths of a notch as well as notches.
    /// When it does, only the fine axis is counted, so that a turn is not
    /// counted twice.
    pub fn hi_res_wheel(&self) -> bool {
        has_bit(&self.rel, REL_WHEEL_HI_RES)
    }

    /// Whether the device has a real button. A touchpad without one
    /// clicks by tapping.
    pub fn left_button(&self) -> bool {
        has_bit(&self.key, BTN_LEFT)
    }

    /// Whether the device reports `ABS_X`, the kernel's single-touch
    /// emulation. A pad that has it is followed by `ABS_X`/`ABS_Y`,
    /// which track the oldest contact; a pad that has not is followed by
    /// the multitouch axes of slot 0.
    pub fn abs_single(&self) -> bool {
        has_bit(&self.abs, ABS_X)
    }
}

/// One input device this shell will read, and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// The node to open, `/dev/input/eventN`.
    pub node: PathBuf,
    /// What the device calls itself, for `--verbose`.
    pub name: String,
    /// It sends [`Event::Key`](osk_shell_api::Event::Key).
    pub keyboard: bool,
    /// It moves the cursor with relative motion.
    pub mouse: bool,
    /// It moves the cursor with a finger.
    pub touchpad: bool,
    /// It names places on the screen.
    pub touchscreen: bool,
    /// Its wheel counts in 120ths of a notch.
    pub hi_res_wheel: bool,
    /// It has a button of its own.
    pub left_button: bool,
    /// Its position comes from `ABS_X`/`ABS_Y` rather than from the
    /// multitouch axes.
    pub abs_single: bool,
}

impl Device {
    /// A device this shell has no reader for is not opened at all.
    pub fn wanted(&self) -> bool {
        self.keyboard || self.mouse || self.touchpad || self.touchscreen
    }

    /// What the device is, in the order the verbose line prints it.
    pub fn kinds(&self) -> String {
        let mut kinds = Vec::new();
        for (yes, word) in [
            (self.keyboard, "keyboard"),
            (self.mouse, "mouse"),
            (self.touchpad, "touchpad"),
            (self.touchscreen, "touchscreen"),
        ] {
            if yes {
                kinds.push(word);
            }
        }
        kinds.join("+")
    }
}

/// How often [`Watch::rescan`] is worth doing. A person plugging a
/// keyboard in waits this long at worst, and the listing is one
/// `read_dir` of a directory with a handful of entries in it.
pub const RESCAN: std::time::Duration = std::time::Duration::from_secs(2);

/// Every device under `/sys/class/input` this shell can read.
///
/// `wanted` is `--touch-name`: a device whose name holds it is the
/// touchscreen and no other device is. That is how the Pi's Goodix panel
/// is found, and it is why a Pi's frames and events are what they were
/// before a laptop was ever considered.
pub fn find_devices(sys_class_input: &Path, dev_dir: &Path, wanted: &str) -> Vec<Device> {
    let mut names: Vec<String> = match fs::read_dir(sys_class_input) {
        Ok(dir) => dir
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n.starts_with("event"))
            .collect(),
        Err(_) => return Vec::new(),
    };
    // event2 must not sort before event10.
    names.sort_by_key(|n| (n[5..].parse::<u32>().unwrap_or(u32::MAX), n.clone()));

    let mut devices = Vec::new();
    for entry in names {
        let dir = sys_class_input.join(&entry);
        let node = dev_dir.join(&entry);
        if !node.exists() {
            continue;
        }
        let caps = Caps::read(&dir);
        let name = fs::read_to_string(dir.join("device").join("name"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let device = Device {
            node,
            name,
            keyboard: caps.keyboard(),
            mouse: caps.mouse(),
            touchpad: caps.touchpad(),
            touchscreen: caps.touchscreen(),
            hi_res_wheel: caps.hi_res_wheel(),
            left_button: caps.left_button(),
            abs_single: caps.abs_single(),
        };
        if device.wanted() || !device.name.is_empty() {
            devices.push(device);
        }
    }

    // The named panel is the touch device, whatever the bitmaps said, and
    // it is the only one.
    if !wanted.is_empty()
        && let Some(i) = devices.iter().position(|d| d.name.contains(wanted))
    {
        for (j, d) in devices.iter_mut().enumerate() {
            d.touchscreen = j == i;
        }
    }
    devices.retain(Device::wanted);
    devices
}

/// The parsers one device needs, and the thread body that feeds them.
///
/// A device that is two things gets two parsers and both are fed every
/// event: a receiver that is a keyboard and a mouse types and points.
#[derive(Debug)]
pub struct Readers {
    touch: Option<touch::Parser>,
    pointer: Option<Pointer>,
    keyboard: Option<Keyboard>,
}

impl Readers {
    /// The readers for one device on a `width × height` panel whose touch
    /// controller reports in `grid`.
    pub fn new(device: &Device, width: u16, height: u16, grid: (u16, u16)) -> Readers {
        Readers {
            touch: device
                .touchscreen
                .then(|| touch::Parser::new(width, height, grid)),
            pointer: (device.mouse || device.touchpad).then(|| {
                Pointer::new(
                    device.hi_res_wheel,
                    device.touchpad && !device.left_button,
                    device.abs_single,
                )
            }),
            keyboard: device.keyboard.then(Keyboard::default),
        }
    }

    /// Feeds one raw event to every parser this device has, appending
    /// whatever they produced.
    pub fn feed(&mut self, ev: RawEvent, out: &mut Vec<Wake>) {
        if let Some(parser) = self.touch.as_mut()
            && let Some(t) = parser.feed(ev)
        {
            out.push(Wake::Touch(t));
        }
        if let Some(pointer) = self.pointer.as_mut() {
            pointer.feed(ev, out);
        }
        if let Some(keyboard) = self.keyboard.as_mut() {
            match keyboard.feed(ev) {
                Some(Stroke::Down(key)) => out.push(Wake::Key(key)),
                Some(Stroke::Up(key)) => out.push(Wake::KeyUp(key)),
                None => {}
            }
        }
    }
}

/// Reads one device forever, waking the main loop with everything it
/// decodes.
///
/// Returns once the device gives an error or ends (which a regular file
/// used with `--input` does); the caller then sends [`Wake::InputEnded`]
/// and the loop keeps ticking without that source.
pub fn read_loop(
    device: &Device,
    readers: &mut Readers,
    tx: &SyncSender<Wake>,
) -> std::io::Result<()> {
    let mut file = fs::File::open(&device.node)?;
    // Room for a whole packet's worth of events and then some.
    let mut buf = [0u8; EVENT_SIZE * 64];
    let mut held = Vec::new();
    let mut out = Vec::new();
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        held.extend_from_slice(&buf[..n]);
        let whole = held.len() / EVENT_SIZE * EVENT_SIZE;
        for chunk in held[..whole].chunks_exact(EVENT_SIZE) {
            if let Some(raw) = decode(chunk, EVENT_SIZE) {
                readers.feed(raw, &mut out);
            }
        }
        held.drain(..whole);
        for wake in out.drain(..) {
            if tx.send(wake).is_err() {
                return Ok(());
            }
        }
    }
}

/// Starts one reader thread. The thread owns the device; the main loop
/// only ever sees [`Wake`] values, and one last word naming the device
/// when it is gone.
pub fn spawn(
    id: u32,
    device: Device,
    width: u16,
    height: u16,
    grid: (u16, u16),
    tx: SyncSender<Wake>,
) {
    let mut readers = Readers::new(&device, width, height, grid);
    thread::Builder::new()
        .name(String::from("input"))
        .spawn(move || {
            let _ = read_loop(&device, &mut readers, &tx);
            let _ = tx.send(Wake::InputEnded(id));
        })
        .ok();
}

/// One device this shell has a reader for, and whether that reader is
/// still running.
#[derive(Debug)]
struct Known {
    /// What [`Wake::InputEnded`] names this device by.
    id: u32,
    /// The node, which is also the identity: the same path is the same
    /// device until it goes away.
    node: PathBuf,
    /// It moves the cursor.
    pointer: bool,
    /// Its reader thread has not ended.
    live: bool,
}

/// The devices being read, and what has changed since the last look.
///
/// The shell starts with whatever is plugged in and keeps looking, so a
/// keyboard, a mouse or a touchscreen put into a laptop after the app is
/// up is read from the moment it appears. Two things drive it:
///
/// - [`rescan`](Watch::rescan) lists the directory again and returns the
///   devices that were not there before. Everything else about a new
///   device — what it is, whether it is wanted at all — is the same
///   classification [`find_devices`] does.
/// - [`ended`](Watch::ended) is the reader thread saying its device gave
///   an error or end of file, which is what an unplugged device does. The
///   entry stays until the node itself is gone, so a device whose reader
///   stopped for another reason is not opened again and again.
#[derive(Debug)]
pub struct Watch {
    sys_class_input: PathBuf,
    dev_dir: PathBuf,
    wanted: String,
    known: Vec<Known>,
    next_id: u32,
}

impl Watch {
    /// A watch over the two directories, knowing about nothing yet. The
    /// first [`rescan`](Watch::rescan) returns everything already plugged
    /// in.
    pub fn new(sys_class_input: &Path, dev_dir: &Path, wanted: &str) -> Watch {
        Watch {
            sys_class_input: sys_class_input.to_path_buf(),
            dev_dir: dev_dir.to_path_buf(),
            wanted: wanted.to_string(),
            known: Vec::new(),
            next_id: 0,
        }
    }

    /// The devices that have appeared since the last call, each with the
    /// id its reader will report when it ends.
    ///
    /// A device whose reader has ended and whose node is gone is forgotten
    /// here, so plugging the same port again is a new device and is read.
    pub fn rescan(&mut self) -> Vec<(u32, Device)> {
        self.known.retain(|k| k.live || k.node.exists());
        let mut fresh = Vec::new();
        for device in find_devices(&self.sys_class_input, &self.dev_dir, &self.wanted) {
            if self.known.iter().any(|k| k.node == device.node) {
                continue;
            }
            let id = self.next_id;
            self.next_id += 1;
            self.known.push(Known {
                id,
                node: device.node.clone(),
                pointer: device.mouse || device.touchpad,
                live: true,
            });
            fresh.push((id, device));
        }
        fresh
    }

    /// Records that the reader for `id` has stopped.
    pub fn ended(&mut self, id: u32) {
        if let Some(k) = self.known.iter_mut().find(|k| k.id == id) {
            k.live = false;
        }
    }

    /// Whether any device still being read moves the cursor. False is
    /// what hides the arrow: the mouse has been unplugged and nothing is
    /// left to move it.
    pub fn pointing(&self) -> bool {
        self.known.iter().any(|k| k.live && k.pointer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A capability bitmap as the kernel prints one: hexadecimal words of
    /// the kernel's own width, most significant first, with `bits` set.
    /// Written out here rather than pasted from one machine's sysfs so
    /// that the sample says which capabilities it stands for.
    fn bitmap(bits: &[u16]) -> String {
        let width = usize::BITS as usize;
        let top = bits.iter().map(|b| usize::from(*b)).max().unwrap_or(0);
        let mut words = vec![0u64; top / width + 1];
        for bit in bits {
            let bit = usize::from(*bit);
            words[bit / width] |= 1u64 << (bit % width);
        }
        words
            .iter()
            .rev()
            .map(|w| format!("{w:x}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Every letter of the alphabet, by key code: `KEY_Q`..`KEY_P`,
    /// `KEY_A`..`KEY_L`, `KEY_Z`..`KEY_M`.
    fn letters() -> Vec<u16> {
        (16..=25).chain(30..=38).chain(44..=50).collect()
    }

    /// The bitmaps a plain USB keyboard prints: every letter, and no
    /// relative or absolute axis.
    fn keyboard_caps() -> Caps {
        Caps {
            ev: bitmap(&[EV_SYN, EV_KEY]),
            key: bitmap(&letters()),
            rel: String::new(),
            abs: String::new(),
            props: String::new(),
        }
    }

    /// A two-button wheel mouse.
    fn mouse_caps() -> Caps {
        Caps {
            ev: bitmap(&[EV_SYN, EV_KEY, EV_REL]),
            key: bitmap(&[BTN_LEFT]),
            rel: bitmap(&[REL_X, REL_Y, REL_WHEEL, REL_WHEEL_HI_RES]),
            abs: String::new(),
            props: String::new(),
        }
    }

    /// A laptop touchpad: absolute positions, a finger tool,
    /// `INPUT_PROP_POINTER`, and the multitouch axes a pad reports too.
    fn touchpad_caps() -> Caps {
        Caps {
            ev: bitmap(&[EV_SYN, EV_KEY, EV_ABS]),
            key: bitmap(&[BTN_LEFT, BTN_TOOL_FINGER, BTN_TOUCH]),
            rel: String::new(),
            abs: bitmap(&[ABS_X, ABS_Y, ABS_MT_POSITION_X, ABS_MT_POSITION_Y]),
            props: bitmap(&[INPUT_PROP_POINTER]),
        }
    }

    /// A touchscreen: multitouch, and directly on the display.
    fn touchscreen_caps() -> Caps {
        Caps {
            ev: bitmap(&[EV_SYN, EV_KEY, EV_ABS]),
            key: bitmap(&[BTN_TOUCH]),
            rel: String::new(),
            abs: bitmap(&[
                ABS_X,
                ABS_Y,
                ABS_MT_POSITION_X,
                ABS_MT_POSITION_Y,
                ABS_MT_TRACKING_ID,
            ]),
            props: bitmap(&[INPUT_PROP_DIRECT]),
        }
    }

    #[test]
    fn each_device_is_read_as_what_it_is() {
        let k = keyboard_caps();
        assert!(k.keyboard() && !k.mouse() && !k.touchpad() && !k.touchscreen());

        let m = mouse_caps();
        assert!(m.mouse() && !m.keyboard() && !m.touchpad() && !m.touchscreen());
        assert!(m.left_button() && m.hi_res_wheel());

        let p = touchpad_caps();
        assert!(
            p.touchpad() && !p.touchscreen(),
            "a pad reports multitouch, and its finger is still a pointer"
        );
        assert!(p.left_button());

        let s = touchscreen_caps();
        assert!(s.touchscreen() && !s.touchpad() && !s.mouse() && !s.keyboard());
    }

    #[test]
    fn one_device_can_be_both_a_keyboard_and_a_mouse() {
        let mut keys = letters();
        keys.push(BTN_LEFT);
        let caps = Caps {
            ev: bitmap(&[EV_SYN, EV_KEY, EV_REL]),
            key: bitmap(&keys),
            rel: bitmap(&[REL_X, REL_Y, REL_WHEEL]),
            abs: String::new(),
            props: String::new(),
        };
        assert!(caps.keyboard() && caps.mouse());
    }

    #[test]
    fn a_keyboard_and_mouse_on_one_node_types_and_points() {
        let device = Device {
            node: PathBuf::from("/dev/input/event0"),
            name: String::from("Wireless Receiver"),
            keyboard: true,
            mouse: true,
            touchpad: false,
            touchscreen: false,
            hi_res_wheel: false,
            left_button: true,
            abs_single: false,
        };
        let mut readers = Readers::new(&device, 1920, 1080, (0, 0));
        let mut out = Vec::new();
        for ev in [
            RawEvent {
                time_ms: 0,
                kind: EV_REL,
                code: REL_X,
                value: 12,
            },
            RawEvent {
                time_ms: 0,
                kind: EV_SYN,
                code: SYN_REPORT,
                value: 0,
            },
            // KEY_B pressed.
            RawEvent {
                time_ms: 0,
                kind: EV_KEY,
                code: 48,
                value: 1,
            },
            RawEvent {
                time_ms: 0,
                kind: EV_KEY,
                code: BTN_LEFT,
                value: 1,
            },
            RawEvent {
                time_ms: 0,
                kind: EV_SYN,
                code: SYN_REPORT,
                value: 0,
            },
        ] {
            readers.feed(ev, &mut out);
        }
        assert_eq!(
            out,
            vec![
                Wake::Pointer(crate::pointer::Motion::Move { dx: 12, dy: 0 }),
                Wake::Key(osk_shell_api::Key::Char('b')),
                Wake::Pointer(crate::pointer::Motion::Button(true)),
            ]
        );
    }

    #[test]
    fn a_truncated_event_decodes_to_nothing() {
        assert_eq!(decode(&[0u8; 4], EVENT_SIZE_32), None);
    }

    /// A tap is a touch that lifts soon after it lands, so the shell
    /// needs the kernel's timestamp off both layouts of the struct.
    #[test]
    fn an_event_carries_the_time_the_kernel_stamped_it() {
        let mut short = Vec::new();
        short.extend_from_slice(&1_700_000_123u32.to_le_bytes());
        short.extend_from_slice(&456_789u32.to_le_bytes());
        short.extend_from_slice(&EV_KEY.to_le_bytes());
        short.extend_from_slice(&BTN_TOUCH.to_le_bytes());
        short.extend_from_slice(&1i32.to_le_bytes());

        let mut long = Vec::new();
        long.extend_from_slice(&1_700_000_123u64.to_le_bytes());
        long.extend_from_slice(&456_789u64.to_le_bytes());
        long.extend_from_slice(&short[8..]);

        let want = RawEvent {
            time_ms: 1_700_000_123_456,
            kind: EV_KEY,
            code: BTN_TOUCH,
            value: 1,
        };
        assert_eq!(decode(&short, EVENT_SIZE_32), Some(want));
        assert_eq!(decode(&long, EVENT_SIZE_64), Some(want));
    }

    /// Writes a `/sys/class/input/eventN` and its `/dev/input/eventN` as
    /// the kernel would, so a listing has something to classify.
    fn plug(sys: &Path, dev: &Path, n: u32, name: &str, caps: &Caps) {
        let node = format!("event{n}");
        let device = sys.join(&node).join("device");
        fs::create_dir_all(device.join("capabilities")).expect("sysfs");
        fs::write(device.join("name"), format!("{name}\n")).expect("name");
        fs::write(device.join("properties"), &caps.props).expect("properties");
        for (file, text) in [
            ("ev", &caps.ev),
            ("key", &caps.key),
            ("rel", &caps.rel),
            ("abs", &caps.abs),
        ] {
            fs::write(device.join("capabilities").join(file), text).expect("capabilities");
        }
        fs::create_dir_all(dev).expect("dev");
        fs::write(dev.join(&node), []).expect("node");
    }

    #[test]
    fn a_keyboard_plugged_in_after_start_is_read() {
        let root = std::env::temp_dir().join(format!("osk-pi-watch-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (sys, dev) = (root.join("sys"), root.join("dev"));
        plug(&sys, &dev, 0, "Logitech Mouse", &mouse_caps());

        let mut watch = Watch::new(&sys, &dev, "");
        let first = watch.rescan();
        assert_eq!(first.len(), 1, "the mouse already plugged in");
        assert!(first[0].1.mouse && watch.pointing());

        // Nothing has changed, so nothing is opened a second time.
        assert!(watch.rescan().is_empty());

        plug(&sys, &dev, 1, "USB Keyboard", &keyboard_caps());
        let second = watch.rescan();
        assert_eq!(second.len(), 1, "only the keyboard is new");
        assert!(second[0].1.keyboard);
        assert_eq!(second[0].1.node, dev.join("event1"));

        // The mouse is unplugged: its reader ends and its node goes, and
        // with nothing left to move the cursor the arrow has no reason to
        // be on the screen.
        watch.ended(first[0].0);
        assert!(!watch.pointing());
        fs::remove_file(dev.join("event0")).expect("unplug");
        fs::remove_dir_all(sys.join("event0")).expect("unplug");
        assert!(watch.rescan().is_empty());

        // Plugged back into the same port, it is a device again.
        plug(&sys, &dev, 0, "Logitech Mouse", &mouse_caps());
        let third = watch.rescan();
        assert_eq!(third.len(), 1);
        assert!(third[0].1.mouse && watch.pointing());

        fs::remove_dir_all(&root).expect("clean up");
    }
}
