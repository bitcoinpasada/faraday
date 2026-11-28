//! What else can capture what Faraday shows or what is typed: the pure rules over what the
//! machine says of itself (`main`'s `devices.rs`, 30-283).

use faraday_kernel::Public;
use faraday_port::devices::UsbDevice;
use faraday_strings::{EN, fill};
use faraday_text::Text;

/// What an item is.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A KVM switch or an IP-KVM: it can see the screen and type.
    Kvm,
    /// A wireless keyboard or mouse receiver: keys travel over the air. A reminder; it holds
    /// nothing.
    Receiver,
    /// Logitech's Bolt receiver, whose link is encrypted.
    BoltReceiver,
    /// Faraday is running inside a virtual machine.
    VirtualMachine,
    /// The screen is a TV: its EDID says so.
    Tv,
}

impl Kind {
    /// A wireless receiver of either kind.
    pub(crate) fn receiver(self) -> bool {
        matches!(self, Kind::Receiver | Kind::BoltReceiver)
    }

    /// What it can do, in the words the sheet uses after its name.
    pub(crate) fn what(self) -> &'static str {
        match self {
            Kind::Kvm => EN.devices_kvm,
            Kind::Receiver => EN.devices_receiver_notice,
            Kind::BoltReceiver => EN.devices_bolt_notice,
            Kind::VirtualMachine => EN.devices_virtual_machine,
            Kind::Tv => EN.devices_tv,
        }
    }
}

/// One thing named on the sheet.
#[derive(Public, PartialEq, Eq)]
pub struct Concern {
    /// What tells it apart from others: its port and identity, or the machine or the screen.
    pub(crate) key: Text,
    /// Its name.
    pub(crate) name: Text,
    /// What it is.
    pub(crate) kind: Kind,
}

/// A reminder: what may capture a secret but was not found.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Reminder {
    /// The screen may be a TV, and its EDID did not settle it.
    Tv,
    /// A wireless receiver is attached.
    Receiver,
    /// Only Logitech's Bolt receiver is attached.
    Bolt,
}

impl Reminder {
    /// The sheet's line.
    pub(crate) fn line(self) -> &'static str {
        match self {
            Reminder::Tv => EN.devices_tv_line,
            Reminder::Receiver => Kind::Receiver.what(),
            Reminder::Bolt => Kind::BoltReceiver.what(),
        }
    }
}

/// The Linux Foundation's USB vendor ID, which the kernel's USB gadget framework uses by
/// default: PiKVM, TinyPilot and other IP-KVMs built on it present a keyboard and a disk under
/// it.
const LINUX_FOUNDATION: u16 = 0x1d6b;

const HID: u8 = 0x03;
const STORAGE: u8 = 0x08;

/// Logitech's Bolt receiver (`USB_DEVICE_ID_LOGITECH_BOLT_RECEIVER`, Linux
/// `drivers/hid/hid-ids.h`); its link is encrypted.
pub(crate) const BOLT_RECEIVER: (u16, u16) = (0x046d, 0xc548);

/// The common wireless keyboard and mouse receivers, by vendor and product ID: Logitech's are
/// the kernel's own (`hid-ids.h`), Microsoft's and Rapoo's are `usb.ids`.
pub(crate) const RECEIVERS: &[(u16, u16)] = &[
    (0x046d, 0xc101),
    (0x046d, 0xc50c),
    (0x046d, 0xc513),
    (0x046d, 0xc517),
    (0x046d, 0xc51b),
    (0x046d, 0xc52b),
    (0x046d, 0xc52f),
    (0x046d, 0xc531),
    (0x046d, 0xc532),
    (0x046d, 0xc534),
    (0x046d, 0xc539),
    (0x046d, 0xc53a),
    (0x046d, 0xc53f),
    (0x046d, 0xc543),
    (0x046d, 0xc545),
    (0x046d, 0xc547),
    BOLT_RECEIVER,
    (0x046d, 0xc54d),
    (0x045e, 0x00f9),
    (0x045e, 0x071f),
    (0x045e, 0x0745),
    (0x045e, 0x07a5),
    (0x045e, 0x07b2),
    (0x045e, 0x07fd),
    (0x24ae, 0x2001),
    (0x24ae, 0x2003),
    (0x24ae, 0x2015),
];

fn names_any(d: &UsbDevice, words: &[&str]) -> bool {
    let s = format!("{} {}", d.manufacturer.as_str(), d.name.as_str()).to_ascii_lowercase();
    words.iter().any(|w| s.contains(w))
}

/// What a USB device is, when it is one the stick warns about: a KVM (its strings name one, or
/// the Linux gadget vendor with a keyboard and a disk), or a wireless receiver (a known
/// receiver's ID, or a HID device whose strings name a receiver, a dongle or wireless),
/// Logitech's Bolt receiver told apart by its ID.
#[must_use]
pub fn usb_kind(d: &UsbDevice) -> Option<Kind> {
    let has = |class: u8| d.interfaces.iter().any(|i| i.class == class);
    let keyboard = d
        .interfaces
        .iter()
        .any(|i| i.class == HID && i.protocol == 1);
    if names_any(d, &["kvm", "tinypilot"])
        || (d.vendor == LINUX_FOUNDATION && keyboard && has(STORAGE))
    {
        return Some(Kind::Kvm);
    }
    if (d.vendor, d.product) == BOLT_RECEIVER {
        return Some(Kind::BoltReceiver);
    }
    if RECEIVERS.contains(&(d.vendor, d.product))
        || (has(HID) && names_any(d, &["receiver", "dongle", "wireless"]))
    {
        return Some(Kind::Receiver);
    }
    None
}

/// A USB device's name: its maker and product strings, the maker left out when the product
/// already starts with it; its IDs when it gives neither.
#[must_use]
pub fn usb_name(d: &UsbDevice) -> Text {
    let (m, p) = (d.manufacturer.as_str().trim(), d.name.as_str().trim());
    match (m.is_empty(), p.is_empty()) {
        (true, true) => Text::of(&fill(
            EN.devices_unnamed,
            &[&format!("{:04x}:{:04x}", d.vendor, d.product)],
        )),
        (true, false) => Text::of(p),
        (false, true) => Text::of(m),
        (false, false) if p.to_ascii_lowercase().starts_with(&m.to_ascii_lowercase()) => {
            Text::of(p)
        }
        (false, false) => {
            // "Logitech, Inc." reads as "Logitech".
            let m = m.split(',').next().unwrap_or(m).trim();
            Text::of(&format!("{m} {p}"))
        }
    }
}

/// Whether Faraday runs inside a virtual machine, and its name: from the CPU's `hypervisor`
/// flag and the firmware's maker and product. A hypervisor whose maker is not one of these is
/// still one, named "This computer".
#[must_use]
pub fn virtual_machine(
    hypervisor: bool,
    sys_vendor: Option<&str>,
    product_name: Option<&str>,
) -> Option<&'static str> {
    let (v, p) = (
        sys_vendor.unwrap_or_default().trim(),
        product_name.unwrap_or_default().trim(),
    );
    let named = if v.contains("QEMU") || p.contains("QEMU") {
        Some(EN.vm_qemu)
    } else if v.contains("VMware") || p.contains("VMware") {
        Some(EN.vm_vmware)
    } else if v.contains("innotek") || p.contains("VirtualBox") {
        Some(EN.vm_virtualbox)
    } else if v.starts_with("Microsoft") && p == "Virtual Machine" {
        Some(EN.vm_hyper_v)
    } else if v.contains("Xen") || p.contains("HVM domU") {
        Some(EN.vm_xen)
    } else if v.contains("Parallels") || p.contains("Parallels") {
        Some(EN.vm_parallels)
    } else {
        None
    };
    match (named, hypervisor) {
        (Some(n), _) => Some(n),
        (None, true) => Some(EN.devices_virtual_machine_unknown),
        (None, false) => None,
    }
}

/// Whether the computer's screen is a separate one, from its DMI chassis type (the SMBIOS
/// System Enclosure type): a desktop (3), low-profile desktop (4), pizza box (5), mini tower
/// (6), tower (7), space-saving (15), lunch box (16), sealed-case PC (24), mini PC (35) or
/// stick PC (36). Laptops, all-in-ones, tablets and anything unknown have a panel of their own,
/// or are not asked.
#[must_use]
pub fn screen_external(chassis: Option<u8>) -> bool {
    matches!(chassis, Some(3..=7 | 15 | 16 | 24 | 35 | 36))
}
