//! What can capture what Faraday shows or what is typed (`docs/HUB.md`
//! §3.1a), as rules over what each system reads, with no I/O: the
//! shells read the bytes and the strings, these say what they are. Each
//! rule is a pure function, so the tests feed it made-up inputs.
//!
//! - [`screen`]: a screen's EDID, as a TV or a KVM's capture side.
//! - [`usb`]: a USB device's identity, as a KVM or IP-KVM, or a
//!   wireless keyboard or mouse receiver.
//! - [`virtual_machine`]: the firmware's maker strings and the CPU's
//!   hypervisor flag, as a virtual machine.
//! - [`remote_display`]: an X11 display reached over the network.
//!
//! Not warned about (§3.1a): Wi-Fi, Bluetooth and USB network adapters,
//! and USB video capture cards. The stick has no driver for any of them.

use crate::api::Kind;
use faraday_core::strings::EN;

/// The EDID's fixed header.
const HEADER: [u8; 8] = [0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00];

/// PNP IDs of makers whose screens include TVs, as the UEFI Forum's PNP
/// ID registry lists them: Samsung (`SAM`), LG (`GSM`, Goldstar), Sony
/// (`SNY`), Sharp (`SHP`), Panasonic (`MEI`, Matsushita), Toshiba
/// (`TOS`), Vizio (`VIZ`), Philips (`PHL`), Hisense (`HEC`), TCL
/// (`TCL`). Most of them make monitors too, so a maker alone is not a
/// TV: [`screen`] also asks for the HDMI block and a broadcast timing.
const TV_MAKERS: [&str; 10] = [
    "SAM", "GSM", "SNY", "SHP", "MEI", "TOS", "VIZ", "PHL", "HEC", "TCL",
];

/// CEA-861 video identification codes of interlaced broadcast formats:
/// 1080i at 60 and 50 Hz (5, 20), 480i (6, 7) and 576i (21, 22). A
/// computer monitor has no use for them; a TV lists them for broadcast.
const BROADCAST_VICS: [u8; 6] = [5, 6, 7, 20, 21, 22];

/// The HDMI Licensing IEEE OUI, `00-0C-03`, as an HDMI vendor-specific
/// data block carries it (least significant byte first).
const HDMI_OUI: [u8; 3] = [0x03, 0x0c, 0x00];

/// The monitor name an EDID carries (its `0xFC` descriptor), trimmed;
/// `None` when it has none or the bytes are not an EDID.
pub fn edid_name(edid: &[u8]) -> Option<String> {
    if edid.get(..8)? != HEADER {
        return None;
    }
    (0..4).find_map(|i| {
        let d = edid.get(54 + 18 * i..72 + 18 * i)?;
        if d[..3] != [0, 0, 0] || d[3] != 0xfc {
            return None;
        }
        let text: String = d[5..]
            .iter()
            .take_while(|&&b| b != 0x0a && b != 0)
            .map(|&b| if b.is_ascii_graphic() { b as char } else { ' ' })
            .collect();
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    })
}

/// The maker's three-letter PNP ID an EDID carries (bytes 8 and 9, five
/// bits a letter).
pub fn edid_maker(edid: &[u8]) -> Option<String> {
    if edid.get(..8)? != HEADER {
        return None;
    }
    let v = u16::from_be_bytes([*edid.get(8)?, *edid.get(9)?]);
    let letter = |n: u16| {
        let c = (n & 0x1f) as u8;
        (1..=26).contains(&c).then(|| (b'A' + c - 1) as char)
    };
    Some(
        [letter(v >> 10)?, letter(v >> 5)?, letter(v)?]
            .iter()
            .collect(),
    )
}

/// What the CEA-861 extensions of an EDID hold: an HDMI vendor-specific
/// data block, and the video codes listed.
fn cea(edid: &[u8]) -> (bool, Vec<u8>) {
    let mut hdmi = false;
    let mut vics = Vec::new();
    let count = edid.get(126).copied().unwrap_or(0) as usize;
    for n in 1..=count {
        let Some(block) = edid.get(128 * n..128 * (n + 1)) else {
            break;
        };
        if block[0] != 0x02 {
            continue;
        }
        let end = (block[2] as usize).clamp(4, 127);
        let mut at = 4;
        while at < end {
            let tag = block[at] >> 5;
            let len = (block[at] & 0x1f) as usize;
            let Some(body) = block.get(at + 1..(at + 1 + len).min(end)) else {
                break;
            };
            match tag {
                // Video data block: one code a byte; codes 1 to 64 may
                // carry the native flag in their top bit.
                2 => vics.extend(body.iter().map(|&b| {
                    if (129..=192).contains(&b) {
                        b & 0x7f
                    } else {
                        b
                    }
                })),
                3 if body.get(..3) == Some(&HDMI_OUI[..]) => hdmi = true,
                _ => {}
            }
            at += 1 + len;
        }
    }
    (hdmi, vics)
}

/// The words of a name, upper-cased, split at anything not a letter or
/// a digit.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_uppercase)
        .collect()
}

/// A name that names a TV: the word `TV`, or a short word ending in it
/// (`LGTV`, `SMARTTV`, `UHDTV`).
fn names_tv(name: &str) -> bool {
    words(name)
        .iter()
        .any(|w| w == "TV" || (w.ends_with("TV") && w.len() <= 7))
}

/// A name that names a KVM switch or an IP-KVM: `KVM` in it (PiKVM,
/// JetKVM, BliKVM), or TinyPilot.
fn names_kvm(name: &str) -> bool {
    let up = name.to_ascii_uppercase();
    up.contains("KVM") || up.contains("TINYPILOT")
}

/// What a screen is, from its EDID: a TV when its monitor name names a
/// TV, or when a TV maker's PNP ID comes with the HDMI vendor-specific
/// block and an interlaced broadcast timing; a KVM when its name names
/// one (the EDID a KVM's capture side reports). `None` for anything
/// else, an internal panel or an ordinary monitor, and for bytes that
/// are not an EDID.
pub fn screen(edid: &[u8]) -> Option<Kind> {
    if edid.get(..8)? != HEADER {
        return None;
    }
    let name = edid_name(edid).unwrap_or_default();
    if names_kvm(&name) {
        return Some(Kind::Kvm);
    }
    if names_tv(&name) {
        return Some(Kind::Tv);
    }
    let maker = edid_maker(edid).unwrap_or_default();
    let (hdmi, vics) = cea(edid);
    let broadcast = vics.iter().any(|v| BROADCAST_VICS.contains(v));
    (TV_MAKERS.contains(&maker.as_str()) && hdmi && broadcast).then_some(Kind::Tv)
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

/// Words in a HID keyboard's or mouse's strings that name a receiver.
const RECEIVER_WORDS: [&str; 4] = ["RECEIVER", "DONGLE", "WIRELESS", "TRANSCEIVER"];

/// What a USB device is, of what can capture the screen or the keys: a
/// KVM when it is the Linux gadget vendor with a keyboard and a disk, or
/// its strings name a KVM; a wireless receiver when its IDs are a known
/// receiver's, or it is a HID keyboard or mouse whose strings name a
/// receiver, a dongle, wireless or 2.4G. `None` for anything else,
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

/// Whether an X11 `DISPLAY` is reached over the network: a host before
/// its last `:` (`localhost:10.0`, as SSH's forwarding sets it), other
/// than `unix`.
pub fn remote_display(display: Option<&str>) -> bool {
    display
        .and_then(|d| d.rsplit_once(':'))
        .is_some_and(|(host, _)| !host.is_empty() && host != "unix")
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
