//! What can capture what Faraday shows or what is typed (`docs/STARTER.md`
//! §3.1a), as rules over what each system reads, with no I/O: the
//! shells read the bytes and the strings, these say what they are. Each
//! rule is a pure function, so the tests feed it made-up inputs.
//!
//! - [`screen`]: a screen's EDID, as a TV or a KVM's capture side.
//! - [`usb`]: a USB device's identity, as a KVM or IP-KVM, or a
//!   wireless keyboard or mouse receiver.
//! - [`virtual_machine`]: the firmware's maker strings and the CPU's
//!   hypervisor flag, as a virtual machine.
//!
//! Not warned about (§3.1a): Wi-Fi, Bluetooth and USB network adapters,
//! and USB video capture cards. The stick has no driver for any of them.

use crate::api::Kind;
use faraday_core::edid::{self, Display, names_kvm, words};
pub use faraday_core::edid::{edid_maker, edid_name};
use faraday_strings::EN;

/// What a screen is, from its EDID: [`edid::display`]'s rules, as the
/// Starter's kinds.
pub fn screen(edid: &[u8]) -> Option<Kind> {
    edid::display(edid).map(|d| match d {
        Display::Tv => Kind::Tv,
        Display::Kvm => Kind::Kvm,
    })
}

/// One USB device as the shells read it: its IDs, its strings and its
/// interfaces' class, subclass and protocol.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usb {
    /// `idVendor`.
    pub vendor: u16,
    /// `idProduct`.
    pub product: u16,
    /// The maker's string.
    pub manufacturer: String,
    /// The product's string.
    pub name: String,
    /// Each interface's class, subclass and protocol.
    pub interfaces: Vec<(u8, u8, u8)>,
}

/// The Linux Foundation's vendor ID, which Linux's USB gadget drivers
/// present by default: what PiKVM, TinyPilot and their like show the
/// computer as their keyboard and disk.
pub const GADGET_VENDOR: u16 = 0x1d6b;

/// Wireless receivers by ID, from Linux's `drivers/hid/hid-ids.h` (the
/// `USB_DEVICE_ID_LOGITECH_*` receivers `hid-logitech-dj.c` drives):
/// Logitech's Unifying receiver (`c52b`, `c532`), Nano receivers
/// (`c52f`, `c534`), Lightspeed receiver (`c539`) and Bolt receiver
/// (`c548`), all under Logitech's vendor ID `046d`.
pub const RECEIVERS: [(u16, u16); 6] = [
    (0x046d, 0xc52b),
    (0x046d, 0xc532),
    (0x046d, 0xc52f),
    (0x046d, 0xc534),
    (0x046d, 0xc539),
    (0x046d, 0xc548),
];

/// Logitech's Bolt receiver: `USB_DEVICE_ID_LOGITECH_BOLT_RECEIVER` in
/// Linux's `drivers/hid/hid-ids.h` (read 2026-10-12). Its link to a Bolt
/// keyboard is encrypted (Logitech's Logi Bolt white paper).
pub const BOLT_RECEIVER: (u16, u16) = (0x046d, 0xc548);

/// Words in a HID keyboard's or mouse's strings that name a receiver.
const RECEIVER_WORDS: [&str; 4] = ["RECEIVER", "DONGLE", "WIRELESS", "TRANSCEIVER"];

/// What a USB device is, of what can capture the screen or the keys: a
/// KVM when it is the Linux gadget vendor with a keyboard and a disk, or
/// its strings name a KVM; a wireless receiver when its IDs are a known
/// receiver's, or it is a HID keyboard or mouse whose strings name a
/// receiver, a dongle, wireless or 2.4G (Logitech's Bolt receiver, by
/// its ID, apart). `None` for anything else,
/// root hubs and webcams among them.
pub fn usb(d: &Usb) -> Option<Kind> {
    let strings = format!("{} {}", d.manufacturer, d.name);
    let keyboard = d.interfaces.iter().any(|&(c, _, p)| c == 0x03 && p == 1);
    let hid_input = d
        .interfaces
        .iter()
        .any(|&(c, _, p)| c == 0x03 && (p == 1 || p == 2));
    let disk = d.interfaces.iter().any(|&(c, ..)| c == 0x08);
    if (d.vendor == GADGET_VENDOR && keyboard && disk) || names_kvm(&strings) {
        return Some(Kind::Kvm);
    }
    if (d.vendor, d.product) == BOLT_RECEIVER {
        return Some(Kind::BoltReceiver);
    }
    if RECEIVERS.contains(&(d.vendor, d.product)) {
        return Some(Kind::Receiver);
    }
    let w = words(&strings);
    // "2.4G" splits into "2" and "4G".
    let two_four = w.windows(2).any(|p| p[0] == "2" && p[1] == "4G");
    let named = two_four || w.iter().any(|x| RECEIVER_WORDS.contains(&x.as_str()));
    (hid_input && named).then_some(Kind::Receiver)
}

/// Hypervisors by a word of the firmware's maker or model strings, and
/// the name the line gives them.
const HYPERVISORS: [(&str, &str); 8] = [
    ("QEMU", EN.vm_qemu),
    ("KVM", EN.vm_kvm),
    ("VMWARE", EN.vm_vmware),
    ("VIRTUALBOX", EN.vm_virtualbox),
    ("INNOTEK", EN.vm_virtualbox),
    ("XEN", EN.vm_xen),
    ("PARALLELS", EN.vm_parallels),
    ("BOCHS", EN.vm_bochs),
];

/// Whether this computer is a virtual machine, and its name for the
/// line: the hypervisor a word of the firmware's maker or model names
/// (QEMU, KVM, VMware, VirtualBox, Xen, Parallels, Bochs; Hyper-V as
/// Microsoft's "Virtual Machine"), or the maker and model when only the
/// CPU says so. `guest` is the CPU's hypervisor flag for a guest: on
/// Windows the shell clears it for the host's own root partition, which
/// Windows' virtualization-based security runs in.
pub fn virtual_machine(maker: &str, model: &str, guest: bool) -> Option<String> {
    let both = format!("{maker} {model}");
    let w = words(&both);
    if let Some((_, name)) = HYPERVISORS
        .iter()
        .find(|(word, _)| w.iter().any(|x| x == word))
    {
        return Some((*name).to_string());
    }
    if words(maker).first().is_some_and(|m| m == "MICROSOFT")
        && model.to_ascii_lowercase().contains("virtual machine")
    {
        return Some(EN.vm_hyper_v.to_string());
    }
    guest.then(|| both.trim().to_string())
}

/// A USB device's name for its line: the maker's and the product's
/// strings, the maker once when the product already begins with it.
pub fn usb_name(d: &Usb) -> String {
    let (m, p) = (d.manufacturer.trim(), d.name.trim());
    if m.is_empty() || p.to_ascii_lowercase().starts_with(&m.to_ascii_lowercase()) {
        p.to_string()
    } else {
        format!("{m} {p}").trim().to_string()
    }
}
