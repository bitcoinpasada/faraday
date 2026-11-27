//! Events in: `Event` and every unasked fact (02 §3).

use alloc::vec::Vec;
use core::fmt;

use faraday_text::Text;

use crate::ask::{Answer, AskId};
use crate::bytes::Bytes;
use crate::camera::{self, CameraFrame, CameraInfo, QrSeen};
use crate::devices::{Edid, InputId, UsbDevice};
use crate::storage::{BootMedium, Carry, StickId, StickInfo};
use crate::system::{Computer, Concern, DevImage, Disk, Installed, WriteStage};
use crate::values::{Display, Medium, Point};

/// `Event` and `Answer` are neither `Clone` nor `Public` (02 §6).
#[expect(
    clippy::large_enum_variant,
    reason = "the port's enums carry the domain's real types, which differ in size; a value is moved once through the queue"
)]
pub enum Event {
    /// First, exactly once per process. Nothing else comes before it.
    Start(Start),
    /// The display changed: a resize, a turn, a preview size, a new screen.
    Display(Display),
    /// Monotonic and wall time, at each tick.
    Time(Time),
    Input(Input),
    Sticks(SticksFact),
    Devices(DevicesFact),
    Camera(CameraFact),
    Host(HostFact),
    System(SystemFact),
    /// The person closed the window (desktop roots only).
    CloseAsked,
    Answer(AskId, Answer),
}

/// The variant names of an `Event`, which the runtime logs (P15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventKind {
    Start,
    Display,
    Time,
    Input,
    Sticks,
    Devices,
    Camera,
    Host,
    System,
    CloseAsked,
    Answer,
}

impl Event {
    pub fn kind(&self) -> EventKind {
        todo!()
    }
}

impl fmt::Debug for Event {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

pub struct Start {
    pub display: Display,
    pub medium: Medium,
    pub abilities: Abilities,
    pub machine: Machine,
    pub time: Time,
    /// What the previous process kept for this one; moved in, once.
    pub carried: Option<Carry>,
    /// The Starter's own settings text, stored unread.
    pub settings: Option<Text>,
    /// The computer's light or dark scheme, where the adapter reads one.
    pub scheme: Option<Scheme>,
    /// The stick adapter's read of the kernel's mount list: the device's root is mounted
    /// read-only. `None` off the device.
    pub root_read_only: Option<bool>,
}

/// What this product's adapters carry. The app offers a control only where its effect is
/// carried (a Paste button only with a clipboard).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Abilities {
    pub clipboard: bool,
    pub print: bool,
    /// Downloads, a dropped file, saving a received file.
    pub host_files: bool,
    /// Opening a folder, a file or the one fixed address.
    pub opens: bool,
    pub camera: bool,
    /// False for folders standing for sticks.
    pub names_sticks: bool,
    /// The device: new input devices wait (F153).
    pub holds_back_input: bool,
    pub restart_computer: bool,
    pub moves_window: bool,
    /// The Starter's operating-system asks.
    pub system: bool,
    /// The device: a lock ends the process.
    pub lock_ends_process: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub maker: Option<Text>,
    pub product: Option<Text>,
    pub family: Option<Text>,
    pub chassis: Option<u8>,
    /// The CPU's hypervisor flag.
    pub hypervisor: bool,
    /// The first 128 bytes, as read.
    pub screen: Option<Edid>,
    /// From the firmware's boot parameters.
    pub secure_boot: SecureBootAtStart,
    pub memory_total_mib: Option<u32>,
    pub cores: u16,
}

/// Whether the stick started with Secure Boot on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SecureBootAtStart {
    #[default]
    Unknown,
    Off,
    On,
}

/// The computer's light or dark scheme. `Start::scheme` is `None` where the setting says
/// neither or cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Light,
    Dark,
}

/// `now_ms` is monotonic from the adapter's start; `unix_secs` is the computer's clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time {
    pub now_ms: u64,
    pub unix_secs: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// A finger, or the left button.
    Touch {
        at: Point,
        phase: TouchPhase,
    },
    /// `None`: the pointer left.
    Hover(Option<Point>),
    /// A trackpad, in whole pixels.
    Scroll {
        at: Point,
        dy: i16,
    },
    ScrollEnd {
        at: Point,
    },
    /// 48 px a notch.
    Wheel {
        at: Point,
        dy: i16,
    },
    Key(Key),
    KeyUp(Key),
    Shift {
        held: bool,
    },
    Back,
    Forward,
    Focus(bool),
}

/// Phase of a touch (or mouse) contact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    Down,
    Move,
    Up,
}

/// A physical keyboard key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Delete,
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Tab,
    BackTab,
    Home,
    End,
    SelectAll,
    Paste,
}

pub enum SticksFact {
    /// The kernel shows storage attached, listed or not.
    Attached(bool),
    /// Every stick present now, on start and on every change.
    Listed(Vec<StickInfo>),
    /// A stick the backup flow cannot use: not FAT32 as Faraday reads it, or read-only.
    Problem {
        stick: StickId,
        read_only: bool,
    },
    BootMedium(BootMedium),
}

pub enum DevicesFact {
    /// The device check's view, while the app watches.
    Usb(Vec<UsbDevice>),
    Screen(Option<Edid>),
    /// The USB devices present when the start window closed.
    BootUsb(Vec<Text>),
    NewInput {
        id: InputId,
        name: Text,
        keyboard: bool,
        pointer: bool,
        touch: bool,
    },
    Typed {
        id: InputId,
        ch: char,
    },
    Tapped {
        id: InputId,
        at: Point,
    },
    Used {
        id: InputId,
    },
    Gone {
        id: InputId,
    },
    Trust {
        pointer: bool,
        keyboard: bool,
    },
}

pub enum CameraFact {
    /// The cameras this adapter can open; sent before every answer to `camera::On`, and when
    /// the list changes.
    Listed(Vec<CameraInfo>),
    /// A running stream's frame, for the preview and New key's noise.
    Frame { stream: AskId, frame: CameraFrame },
    /// A code the adapter's scanner read in this stream.
    Code { stream: AskId, bytes: Bytes },
    /// What one scan pass saw.
    Seen { stream: AskId, seen: QrSeen },
    /// The stream ended without being asked to.
    Ended { stream: AskId, why: camera::Failure },
}

/// The desktop computer's facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostFact {
    /// Sent on change; the adapter rescans every 700 ms.
    Downloads {
        dir: Text,
        files: Vec<(Text, u64)>,
    },
    Hovered(bool),
    Dropped {
        path: Text,
        size: u64,
    },
}

/// The Starter's operating system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemFact {
    Computer(Computer),
    Disks(Vec<Disk>),
    NoWriter,
    Installed(Option<Installed>),
    DevImage(DevImage),
    ImagesWanted(Vec<Text>),
    /// Every 3 s.
    Concerns(Vec<Concern>),
    OtherEncryption(Option<Text>),
    BootEntry(bool),
    /// Of a stick being written.
    WriteProgress {
        ask: AskId,
        stage: WriteStage,
        done: u64,
        total: u64,
    },
}
