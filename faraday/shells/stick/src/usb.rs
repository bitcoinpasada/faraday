//! The USB devices attached, as `/sys/bus/usb/devices` describes them:
//! the same attributes `faraday-grant` reads before it authorises a
//! device, read here for the app's device check (`docs/STARTER.md` §3.1a).
//! Every one of them is readable by any user; nothing is bound or opened.

use std::fs;
use std::path::Path;

use faraday_core::devices::{Interface, UsbDevice};

fn read(dir: &Path, name: &str) -> Option<String> {
    fs::read_to_string(dir.join(name))
        .ok()
        .map(|s| s.trim().to_string())
}

fn hex<T: TryFrom<u32>>(dir: &Path, name: &str) -> Option<T> {
    u32::from_str_radix(&read(dir, name)?, 16)
        .ok()
        .and_then(|v| T::try_from(v).ok())
}

/// Every device under `root` (`/sys/bus/usb/devices`), root hubs and
/// all, sorted by port, each with its interfaces.
pub fn devices(root: &Path) -> Vec<UsbDevice> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut out = Vec::new();
    for port in names.iter().filter(|n| !n.contains(':')) {
        let dir = root.join(port);
        let (Some(vendor), Some(product)) = (hex(&dir, "idVendor"), hex(&dir, "idProduct")) else {
            continue;
        };
        let prefix = format!("{port}:");
        let interfaces = names
            .iter()
            .filter(|n| n.starts_with(&prefix))
            .map(|n| {
                let i = root.join(n);
                Interface {
                    class: hex(&i, "bInterfaceClass").unwrap_or(0),
                    subclass: hex(&i, "bInterfaceSubClass").unwrap_or(0),
                    protocol: hex(&i, "bInterfaceProtocol").unwrap_or(0),
                }
            })
            .collect();
        out.push(UsbDevice {
            port: port.clone(),
            vendor,
            product,
            manufacturer: read(&dir, "manufacturer").unwrap_or_default(),
            name: read(&dir, "product").unwrap_or_default(),
            interfaces,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A receiver as sysfs lays it out: the device's attributes, and an
    /// interface beside it; a root hub's interface-less entry too.
    #[test]
    fn a_receiver_reads_as_its_identity_and_interfaces() {
        let root = std::env::temp_dir().join(format!("faraday-usb-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let put = |dir: &str, name: &str, value: &str| {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(root.join(dir).join(name), format!("{value}\n")).unwrap();
        };
        put("usb1", "idVendor", "1d6b");
        put("usb1", "idProduct", "0002");
        put("1-2", "idVendor", "046d");
        put("1-2", "idProduct", "c52b");
        put("1-2", "manufacturer", "Logitech");
        put("1-2", "product", "USB Receiver");
        put("1-2:1.0", "bInterfaceClass", "03");
        put("1-2:1.0", "bInterfaceSubClass", "01");
        put("1-2:1.0", "bInterfaceProtocol", "01");
        let found = devices(&root);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(found.len(), 2);
        let r = &found[0];
        assert_eq!(
            (r.port.as_str(), r.vendor, r.product),
            ("1-2", 0x046d, 0xc52b)
        );
        assert_eq!(r.name, "USB Receiver");
        assert_eq!(
            r.interfaces,
            vec![Interface {
                class: 3,
                subclass: 1,
                protocol: 1
            }]
        );
        assert!(found[1].interfaces.is_empty());
    }
}
