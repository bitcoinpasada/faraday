//! Effects with no answer (02 §4.9).

use crate::devices::InputId;
use crate::storage::StickId;
use crate::values::Orientation;

pub enum Order {
    CameraOff,
    /// A held pointer's targets are up for this device; the arrow shows
    /// (None: no sheet aims).
    AimInput(Option<InputId>),
    /// The person let a held device in, or kept it out.
    DecideInput { id: InputId, believed: bool },
    /// Run the device check and send `DevicesFact::Usb` and `Screen`.
    WatchDevices(bool),
    /// Whether this process has held or typed no secret; the stick
    /// adapter publishes it for the grant helper.
    Clean(bool),
    /// A `Display` follows.
    Turn(Orientation),
    /// A `Display` follows.
    PreviewSize(Option<(u16, u16)>),
    /// Faraday Demo's stick pills: plug a folder stick in or pull it; a
    /// `SticksFact::Listed` follows.
    ToggleStick(StickId),
    Vibrate { ms: u16 },
    Exit(Exit),
}

pub enum Exit {
    /// The process ends for good: the device powers off; a desktop root
    /// closes.
    End,
    /// A lock that ends the process: the device's process ends with the
    /// restart code.
    Lock,
    /// The device restarts the computer.
    RestartComputer,
    /// The lock scope of this generation was dropped in this process: at a
    /// lock that does not end the process, at a failed keep's rebuild and at
    /// the Starter's Close. Each adapter drops what belonged to that
    /// generation and sends its state facts again (section 05, §7.6).
    Reset(u32),
}
