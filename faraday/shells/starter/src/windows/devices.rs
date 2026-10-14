//! What is attached that is of concern on Windows (`docs/STARTER.md`
//! §3.1a), from Windows' answers, with no Win32 call in it: `sys` reads
//! them (`QueryDisplayConfig` and `DisplayConfigGetDeviceInfo` for the
//! screens, the monitor's EDID from the registry, `SM_REMOTESESSION`,
//! SetupAPI for the USB devices, `CPUID` for a hypervisor) and these
//! read them, so the tests feed recorded answers.

use faraday_starter::api::{Concern, Kind};
use faraday_starter::concern::{self, Usb};

/// `DISPLAYCONFIG_OUTPUT_TECHNOLOGY_MIRACAST`.
pub const MIRACAST: u32 = 15;
/// `DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL`.
pub const INTERNAL: u32 = 0x8000_0000;
/// `DISPLAYCONFIG_OUTPUT_TECHNOLOGY_HDMI`.
pub const HDMI: u32 = 5;
/// The analogue outputs only a TV takes: S-Video (1), composite (2),
/// component (3), D-Jpn (8) and an SDTV dongle (14).
pub const TV_OUTPUTS: [u32; 5] = [1, 2, 3, 8, 14];

/// What an output technology says by itself: a wireless display for
/// Miracast, a TV for an analogue TV output; nothing for the rest,
/// whose EDID says the rest.
pub fn output_kind(technology: u32) -> Option<Kind> {
    if technology == MIRACAST {
        Some(Kind::WirelessDisplay)
    } else if TV_OUTPUTS.contains(&technology) {
        Some(Kind::Tv)
    } else {
        None
    }
}

/// The registry key holding a monitor's EDID (its `EDID` value), from
/// the monitor's device path as `DisplayConfigGetDeviceInfo` gives it:
/// `\\?\DISPLAY#SAM0F9C#5&1a2b&0&UID4352#{e6f07b5f-…}` is
/// `SYSTEM\CurrentControlSet\Enum\DISPLAY\SAM0F9C\5&1a2b&0&UID4352\Device
/// Parameters` under `HKEY_LOCAL_MACHINE`.
pub fn edid_key(device_path: &str) -> Option<String> {
    let rest = device_path.strip_prefix(r"\\?\")?;
    let parts: Vec<&str> = rest.split('#').collect();
    let [bus, model, instance, ..] = parts.as_slice() else {
        return None;
    };
    let plain = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"&_-.".contains(&b))
    };
    if !bus.eq_ignore_ascii_case("DISPLAY") || !plain(model) || !plain(instance) {
        return None;
    }
    Some(format!(
        r"SYSTEM\CurrentControlSet\Enum\DISPLAY\{model}\{instance}\Device Parameters"
    ))
}

/// One active display output as Windows reports it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Output {
    /// Its output technology.
    pub technology: u32,
    /// The monitor's friendly name (from its EDID).
    pub name: String,
    /// The monitor's device path.
    pub path: String,
    /// Its EDID from the registry, when it was read.
    pub edid: Option<Vec<u8>>,
}

/// What an output is of concern as, and its name: the output technology
/// first (Miracast, an analogue TV output), then its EDID's rule. An
/// internal panel is never one.
pub fn output(o: &Output) -> Option<Concern> {
    if o.technology == INTERNAL {
        return None;
    }
    let kind = output_kind(o.technology).or_else(|| concern::screen(o.edid.as_deref()?))?;
    let name = o
        .edid
        .as_deref()
        .and_then(concern::edid_name)
        .unwrap_or_else(|| o.name.trim().to_string());
    Some(Concern {
        kind,
        name,
        id: format!("display:{}", o.path),
    })
}

/// One device SetupAPI lists under the `USB` enumerator: a device, or
/// one interface of a composite device (`&MI_xx`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawUsb {
    /// `SPDRP_HARDWAREID`: `USB\VID_046D&PID_C52B&REV_1211`, …
    pub hardware_ids: Vec<String>,
    /// `SPDRP_COMPATIBLEIDS`: `USB\Class_03&SubClass_01&Prot_01`, …
    pub compatible_ids: Vec<String>,
    /// `DEVPKEY_Device_BusReportedDeviceDesc`: the device's own product
    /// string.
    pub bus_description: String,
    /// `SPDRP_DEVICEDESC`: Windows' name for it.
    pub description: String,
    /// `SPDRP_MFG`: the maker its driver names.
    pub maker: String,
}

/// The vendor and product IDs in a hardware ID (`VID_046D&PID_C52B`).
pub fn usb_ids(hardware_id: &str) -> Option<(u16, u16)> {
    let up = hardware_id.to_ascii_uppercase();
    let hex = |key: &str| {
        let at = up.find(key)? + key.len();
        u16::from_str_radix(up.get(at..at + 4)?, 16).ok()
    };
    Some((hex("VID_")?, hex("PID_")?))
}

/// The class, subclass and protocol in a compatible ID
/// (`USB\Class_03&SubClass_01&Prot_01`); only the full form counts.
pub fn usb_interface(compatible_id: &str) -> Option<(u8, u8, u8)> {
    let up = compatible_id.to_ascii_uppercase();
    let rest = up.strip_prefix(r"USB\CLASS_")?;
    let mut parts = rest.split('&');
    let class = u8::from_str_radix(parts.next()?, 16).ok()?;
    let sub = u8::from_str_radix(parts.next()?.strip_prefix("SUBCLASS_")?, 16).ok()?;
    let prot = u8::from_str_radix(parts.next()?.strip_prefix("PROT_")?, 16).ok()?;
    Some((class, sub, prot))
}

/// The USB devices SetupAPI listed, each with every interface its
/// entries carry, by vendor and product: a composite device's interfaces
/// are entries of their own. A maker in brackets (`(Standard USB Host
/// Controller)`) is Windows' own, not the device's, and is left out.
pub fn usb_devices(raw: &[RawUsb]) -> Vec<(String, Usb)> {
    let mut out: Vec<(String, Usb)> = Vec::new();
    for r in raw {
        let Some((vendor, product)) = r.hardware_ids.iter().find_map(|h| usb_ids(h)) else {
            continue;
        };
        let interfaces: Vec<(u8, u8, u8)> = r
            .compatible_ids
            .iter()
            .filter_map(|c| usb_interface(c))
            .collect();
        let name = if r.bus_description.trim().is_empty() {
            r.description.trim()
        } else {
            r.bus_description.trim()
        };
        let maker = r.maker.trim();
        let maker = if maker.starts_with('(') { "" } else { maker };
        let id = format!("usb:{vendor:04x}:{product:04x}");
        match out.iter_mut().find(|(i, _)| *i == id) {
            Some((_, d)) => {
                d.interfaces.extend(interfaces);
                if d.name.is_empty() {
                    d.name = name.to_string();
                }
                if d.manufacturer.is_empty() {
                    d.manufacturer = maker.to_string();
                }
            }
            None => out.push((
                id,
                Usb {
                    vendor,
                    product,
                    manufacturer: maker.to_string(),
                    name: name.to_string(),
                    interfaces,
                },
            )),
        }
    }
    out
}

/// Whether Windows runs as a guest of a hypervisor, from `CPUID`: leaf
/// 1's `ECX` bit 31 says a hypervisor is there, but Windows with
/// virtualization-based security runs in Hyper-V's root partition and
/// sees it too. Hyper-V's root partition has `CreatePartitions`, bit 0
/// of leaf `0x40000003`'s `EBX` (Hyper-V's Top-Level Functional
/// Specification), behind the vendor `Microsoft Hv` of leaf
/// `0x40000000`.
pub fn guest(leaf1_ecx: u32, vendor: &[u8; 12], features_ebx: u32) -> bool {
    let present = leaf1_ecx & (1 << 31) != 0;
    let root = vendor == b"Microsoft Hv" && features_ebx & 1 != 0;
    present && !root
}

/// Everything of concern, in order: the virtual machine, the remote
/// session, the screens, the USB devices.
pub fn concerns(
    vm: Option<String>,
    remote: bool,
    outputs: &[Output],
    usb: &[RawUsb],
) -> Vec<Concern> {
    let mut out = Vec::new();
    if let Some(name) = vm {
        out.push(Concern {
            kind: Kind::VirtualMachine,
            name,
            id: String::from("vm"),
        });
    }
    if remote {
        out.push(Concern {
            kind: Kind::RemoteSession,
            name: String::new(),
            id: String::from("session"),
        });
    }
    out.extend(outputs.iter().filter_map(output));
    for (id, d) in usb_devices(usb) {
        if let Some(kind) = concern::usb(&d) {
            out.push(Concern {
                kind,
                name: concern::usb_name(&d),
                id,
            });
        }
    }
    out
}
