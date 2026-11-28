use faraday_kernel::session::NoRandomness;
use faraday_kernel::{FrameSheet, Gate, Message};
use faraday_port::bytes::Bytes;
use faraday_port::devices::{Edid, UsbDevice};
use faraday_port::event::DevicesFact;
use faraday_present::Pressed;
use faraday_text::Text;

use crate::inputs::Hit;

#[derive(Message)]
pub enum Msg {
    Press(Press),
    /// A feature's press would show a secret: answered with `Out::Cleared` at once, or after
    /// the Devices sheet (06 §7).
    Reveal(Gate),
    /// The USB devices attached now.
    Usb(Vec<UsbDevice>),
    /// The screen's EDID as read now.
    Screen(Option<Edid>),
    /// What was plugged in when the start window closed.
    BootUsb(Vec<Text>),
    /// A device the adapter holds back.
    NewInput {
        id: u32,
        name: Text,
        keyboard: bool,
        pointer: bool,
        touch: bool,
    },
    /// A key a held-back keyboard typed.
    #[withheld]
    Typed {
        id: u32,
        ch: char,
    },
    /// A held-back device pressed or touched. Where is the root's: it resolves the place
    /// against the frame's targets and sends `Hit` for a press that counts.
    Tapped {
        id: u32,
    },
    /// What a held-back device's press hit, once the root resolved it.
    Hit {
        id: u32,
        hit: Hit,
    },
    /// A held-back device acted.
    Used(u32),
    /// A held-back device was unplugged.
    Gone(u32),
    /// Whether trusted input can point, and whether it can type.
    Trust {
        pointer: bool,
        keyboard: bool,
    },
    /// The session's randomness for device `id`'s code and targets.
    #[withheld]
    Drawn {
        id: u32,
        bytes: Result<Bytes, NoRandomness>,
    },
}

impl From<Press> for Msg {
    fn from(press: Press) -> Msg {
        Msg::Press(press)
    }
}

impl From<DevicesFact> for Msg {
    fn from(fact: DevicesFact) -> Msg {
        match fact {
            DevicesFact::Usb(d) => Msg::Usb(d),
            DevicesFact::Screen(e) => Msg::Screen(e),
            DevicesFact::BootUsb(n) => Msg::BootUsb(n),
            DevicesFact::NewInput {
                id,
                name,
                keyboard,
                pointer,
                touch,
            } => Msg::NewInput {
                id: id.0,
                name,
                keyboard,
                pointer,
                touch,
            },
            DevicesFact::Typed { id, ch } => Msg::Typed { id: id.0, ch },
            DevicesFact::Tapped { id, .. } => Msg::Tapped { id: id.0 },
            DevicesFact::Used { id } => Msg::Used(id.0),
            DevicesFact::Gone { id } => Msg::Gone(id.0),
            DevicesFact::Trust { pointer, keyboard } => Msg::Trust { pointer, keyboard },
        }
    }
}

/// What the feature tells its parent.
#[derive(Message)]
pub enum Out {
    /// A frame sheet.
    Front(Option<FrameSheet>),
    /// The press that would show a secret may go on (06 §7).
    Cleared(Gate),
    /// Something that can capture the screen arrived: every secret on show goes off it, as the
    /// idle warning's does.
    Hide,
    /// Power off (a virtual machine's sheet).
    PowerOff,
}

/// What a press on this feature's pages sends. No press is a target: a held device's press
/// arrives as a fact (`Msg::Hit`), never from a trusted device's hit.
#[derive(Clone, PartialEq, Pressed)]
pub enum Press {
    /// Check again.
    Check,
    /// Continue anyway: the sheet asks.
    Continue,
    /// Hold to continue: lets every item through for this session.
    LetThrough,
    /// I understand.
    Understood,
    /// Cancel.
    Cancel,
    /// Power off.
    PowerOff,
    /// Ignore this device.
    Ignore(u32),
    /// Use this pointer.
    Use(u32),
}

/// What the feature tells the person in a toast.
#[derive(faraday_kernel::Public)]
pub enum Said {
    /// A pointer or touchscreen held until it is used.
    NewDeviceHeld(Text),
    KeyboardIn(Text),
    PointerIn(Text),
    KeyboardPointerIn(Text),
    TouchIn(Text),
    /// A keyboard kept out for the wrong code.
    WrongCode(Text),
    /// A pointer kept out for presses beside the target.
    WrongPresses(Text),
    /// A touchscreen kept out for taps beside the target.
    WrongTaps(Text),
    /// The door is closed: new devices of the kinds the session no longer asks about are
    /// ignored. `alone`: no input is trusted, so pointers are among them.
    Ignoring {
        alone: bool,
    },
    /// What was plugged in when the start window closed.
    Plugged(Vec<Text>),
    /// Check again found them still attached.
    Still,
}
