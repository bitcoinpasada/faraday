//! The devices family's value types (02 §3.6).

/// One interface of a USB device: `bInterfaceClass`, `bInterfaceSubClass` and
/// `bInterfaceProtocol`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interface {
    /// The class: 3 HID, 8 mass storage.
    pub class: u8,
    /// The subclass: 1 for a HID boot device.
    pub subclass: u8,
    pub protocol: u8,
}

/// A USB device as `/sys/bus/usb/devices` holds it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsbDevice {
    pub port: faraday_text::Text,
    pub vendor: u16,
    pub product: u16,
    pub manufacturer: faraday_text::Text,
    pub name: faraday_text::Text,
    pub interfaces: alloc::vec::Vec<Interface>,
}

/// The adapter's number for an input device it holds back until the person lets it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputId(pub u32);

/// A screen's EDID, the first 128 bytes as read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edid(pub [u8; 128]);
