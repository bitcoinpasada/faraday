//! What is attached that is of concern on Linux (`docs/STARTER.md` §3.1a),
//! read from files every user can read, with no device opened:
//!
//! - screens: `/sys/class/drm/card*-*/status` and `edid`, each connector
//!   the graphics driver lists;
//! - USB devices: `/sys/bus/usb/devices/*` (`idVendor`, `idProduct`,
//!   `manufacturer`, `product`, and each interface's
//!   `bInterfaceClass`, `bInterfaceSubClass` and
//!   `bInterfaceProtocol`);
//! - a virtual machine: `/sys/class/dmi/id/sys_vendor` and
//!   `product_name`, and the `hypervisor` flag in `/proc/cpuinfo`;
//!
//! Everything is read under a root given to it, so the tests read a
//! made-up tree in a temporary directory. The rules are
//! `faraday_starter::concern`'s.

use std::fs;
use std::path::Path;

use faraday_starter::api::{Concern, Kind};
use faraday_starter::concern::{self, Usb};

fn text(p: &Path) -> String {
    fs::read_to_string(p)
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn hex16(p: &Path) -> Option<u16> {
    u16::from_str_radix(&text(p), 16).ok()
}

fn hex8(p: &Path) -> Option<u8> {
    u8::from_str_radix(&text(p), 16).ok()
}

/// The entries of a directory, by name, in order.
fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|r| {
            r.filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// The screens connected now that are of concern: each `card*-*`
/// connector whose `status` is `connected`, by its EDID.
pub fn screens(root: &Path) -> Vec<Concern> {
    let drm = root.join("sys/class/drm");
    names(&drm)
        .into_iter()
        .filter(|n| n.starts_with("card") && n.contains('-'))
        .filter_map(|n| {
            let dir = drm.join(&n);
            if text(&dir.join("status")) != "connected" {
                return None;
            }
            let edid = fs::read(dir.join("edid")).ok()?;
            let kind = concern::screen(&edid)?;
            Some(Concern {
                kind,
                name: concern::edid_name(&edid).unwrap_or_default(),
                id: format!("drm:{n}"),
            })
        })
        .collect()
}

/// Every USB device attached, by its sysfs name (its port, `1-2.3`),
/// with its interfaces.
pub fn usb_devices(root: &Path) -> Vec<(String, Usb)> {
    let dir = root.join("sys/bus/usb/devices");
    let all = names(&dir);
    all.iter()
        .filter(|n| !n.contains(':'))
        .filter_map(|n| {
            let d = dir.join(n);
            let vendor = hex16(&d.join("idVendor"))?;
            let product = hex16(&d.join("idProduct"))?;
            let prefix = format!("{n}:");
            let interfaces = all
                .iter()
                .filter(|i| i.starts_with(&prefix))
                .filter_map(|i| {
                    let f = dir.join(i);
                    Some((
                        hex8(&f.join("bInterfaceClass"))?,
                        hex8(&f.join("bInterfaceSubClass")).unwrap_or(0),
                        hex8(&f.join("bInterfaceProtocol")).unwrap_or(0),
                    ))
                })
                .collect();
            Some((
                n.clone(),
                Usb {
                    vendor,
                    product,
                    manufacturer: text(&d.join("manufacturer")),
                    name: text(&d.join("product")),
                    interfaces,
                },
            ))
        })
        .collect()
}

/// The USB devices of concern.
pub fn usb(root: &Path) -> Vec<Concern> {
    usb_devices(root)
        .into_iter()
        .filter_map(|(port, d)| {
            Some(Concern {
                kind: concern::usb(&d)?,
                name: concern::usb_name(&d),
                id: format!("usb:{port}:{:04x}:{:04x}", d.vendor, d.product),
            })
        })
        .collect()
}

/// This computer as a virtual machine, when it is one.
pub fn virtual_machine(root: &Path) -> Option<Concern> {
    let maker = text(&root.join("sys/class/dmi/id/sys_vendor"));
    let model = text(&root.join("sys/class/dmi/id/product_name"));
    let flag = fs::read_to_string(root.join("proc/cpuinfo"))
        .unwrap_or_default()
        .lines()
        .filter(|l| l.starts_with("flags"))
        .any(|l| l.split_whitespace().any(|w| w == "hypervisor"));
    Some(Concern {
        kind: Kind::VirtualMachine,
        name: concern::virtual_machine(&maker, &model, flag)?,
        id: String::from("vm"),
    })
}

/// Everything of concern, with `/` at `root`: the virtual machine, the
/// screens, the USB devices.
pub fn read(root: &Path) -> Vec<Concern> {
    let mut out: Vec<Concern> = virtual_machine(root).into_iter().collect();
    out.extend(screens(root));
    out.extend(usb(root));
    out
}
