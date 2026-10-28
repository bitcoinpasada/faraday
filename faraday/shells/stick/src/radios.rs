//! The boot report's radios block (`--boot-report`): what the kernel and
//! the device tree say of the board's radios and serial ports, so a dev
//! card can show that the Bluetooth and Wi-Fi are off where the release
//! image says they are.
//!
//! Four things, each read from the kernel's own files: whether
//! `/sys/class/bluetooth` and `/sys/class/net` hold anything (no Faraday
//! kernel builds Bluetooth or networking, `common/kernel.forbidden`);
//! the device tree's Bluetooth and Wi-Fi nodes, found by their
//! `compatible` strings rather than by a path guessed for one board,
//! each with its path, its status and its parent's (the serial port the
//! Bluetooth hangs off, the SDIO controller the Wi-Fi does); and every
//! serial port with the driver bound to it, the serial devices bound on
//! top of one, which is how a kernel would attach Bluetooth to its port,
//! and whether it is a console or has a getty.

use std::path::{Path, PathBuf};

/// The most device-tree nodes looked at: a board's tree is a few hundred.
const MAX_NODES: usize = 4096;

/// What a node's `compatible` says it is, if it is a radio: Bluetooth
/// (`brcm,bcm43438-bt`, `infineon,cyw55572-bt`, …) or Wi-Fi
/// (`brcm,bcm4329-fmac`, …).
fn radio(compatible: &str) -> Option<&'static str> {
    let c = compatible.to_ascii_lowercase();
    if c.ends_with("-bt") || c.contains("bluetooth") {
        Some("bluetooth")
    } else if c.ends_with("-fmac") || c.contains("wifi") || c.contains("wlan") {
        Some("wifi")
    } else {
        None
    }
}

/// The block for the board whose files are under `root`. Costs a walk of
/// the device tree and a few reads of `/sys` and `/proc`.
pub fn block(root: &Path) -> String {
    let mut out = String::from("\n== radios ==\n");
    for class in ["bluetooth", "net"] {
        let dir = root.join("sys/class").join(class);
        let text = match names(&dir) {
            None => String::from("absent"),
            Some(n) if n.is_empty() => String::from("empty"),
            Some(n) => n.join(" "),
        };
        out.push_str(&format!("/sys/class/{class:<10} {text}\n"));
    }
    out.push_str(&tree_nodes(root));
    out.push_str(&serial(root));
    out
}

/// The entries of `dir`, sorted; `None` where there is no such folder.
fn names(dir: &Path) -> Option<Vec<String>> {
    let mut n: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    n.sort();
    Some(n)
}

/// A device-tree property as text: its NUL-separated strings joined
/// with commas and spaces.
fn property(node: &Path, name: &str) -> Option<String> {
    let bytes = std::fs::read(node.join(name)).ok()?;
    let parts: Vec<String> = bytes
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect();
    Some(parts.join(", "))
}

/// A node's status, `okay` where it states none (the device tree's own
/// rule).
fn status(node: &Path) -> String {
    property(node, "status").unwrap_or_else(|| String::from("okay (no status)"))
}

/// The device tree's radio nodes, each with its path, compatible and
/// status and its parent's.
fn tree_nodes(root: &Path) -> String {
    let base = [
        root.join("sys/firmware/devicetree/base"),
        root.join("proc/device-tree"),
    ]
    .into_iter()
    .find(|p| p.is_dir());
    let Some(base) = base else {
        return String::from("device tree        not available: no /sys/firmware/devicetree\n");
    };
    let mut found = Vec::new();
    let mut stack = vec![base.clone()];
    let mut seen = 0;
    while let Some(node) = stack.pop() {
        seen += 1;
        if seen > MAX_NODES {
            break;
        }
        if let Some(compatible) = property(&node, "compatible")
            && let Some(kind) = radio(&compatible)
        {
            found.push((kind, node.clone(), compatible));
        }
        let mut children: Vec<PathBuf> = std::fs::read_dir(&node)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .map(|e| e.path())
            .collect();
        children.sort();
        stack.extend(children.into_iter().rev());
    }
    found.sort_by(|a, b| a.0.cmp(b.0).then(a.1.cmp(&b.1)));
    let mut out = String::new();
    for kind in ["bluetooth", "wifi"] {
        if !found.iter().any(|(k, _, _)| *k == kind) {
            out.push_str(&format!(
                "{kind:<18} no device-tree node with a {kind} compatible\n"
            ));
        }
    }
    for (kind, node, compatible) in &found {
        let path = node.strip_prefix(&base).unwrap_or(node);
        let parent = node.parent().unwrap_or(node);
        let parent_compatible = property(parent, "compatible").unwrap_or_else(|| String::from("-"));
        out.push_str(&format!(
            "{kind:<18} /{} ({compatible}) status {}; parent ({parent_compatible}) status {}\n",
            path.display(),
            status(node),
            status(parent),
        ));
    }
    out
}

/// Every serial port with its driver, the serial devices bound on one,
/// and which are consoles or have a getty.
fn serial(root: &Path) -> String {
    let consoles: Vec<String> = std::fs::read_to_string(root.join("proc/consoles"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_owned))
        .collect();
    let inittab = std::fs::read_to_string(root.join("etc/inittab")).unwrap_or_default();
    let ports: Vec<String> = names(&root.join("sys/class/tty"))
        .unwrap_or_default()
        .into_iter()
        .filter(|n| root.join("sys/class/tty").join(n).join("device").exists())
        .collect();
    let mut out = String::new();
    if ports.is_empty() {
        out.push_str("serial             none\n");
    }
    for port in &ports {
        let dir = root.join("sys/class/tty").join(port);
        let mut notes = vec![format!("driver {}", driver(&dir.join("device")))];
        if consoles.iter().any(|c| c == port) {
            notes.push(String::from("console"));
        }
        let getty = inittab.lines().any(|l| {
            let l = l.trim_start();
            !l.starts_with('#')
                && l.contains("getty")
                && l.split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|w| w == port)
        });
        if getty {
            notes.push(String::from("getty in /etc/inittab"));
        }
        out.push_str(&format!("serial {port:<11} {}\n", notes.join(", ")));
    }
    // A device bound on top of a port (serdev): the port then has no tty.
    for dev in names(&root.join("sys/bus/serial/devices")).unwrap_or_default() {
        let dir = root.join("sys/bus/serial/devices").join(&dev);
        out.push_str(&format!(
            "serial device      {dev}, driver {}\n",
            driver(&dir)
        ));
    }
    out
}

/// The driver bound to the device at `dir`, by its `driver` link's name.
fn driver(dir: &Path) -> String {
    std::fs::read_link(dir.join("driver"))
        .ok()
        .and_then(|l| l.file_name()?.to_str().map(str::to_owned))
        .unwrap_or_else(|| String::from("none"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn tree(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("faraday-radios-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn put(root: &Path, path: &str, bytes: &[u8]) {
        let p = root.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, bytes).unwrap();
    }

    fn dir(root: &Path, path: &str) {
        std::fs::create_dir_all(root.join(path)).unwrap();
    }

    #[test]
    fn a_pi_with_its_radios_disabled_says_where_each_node_is_and_its_status() {
        let root = tree("pi");
        let dt = "sys/firmware/devicetree/base";
        put(
            &root,
            &format!("{dt}/compatible"),
            b"raspberrypi,3-model-b-plus\0brcm,bcm2837\0",
        );
        let uart = format!("{dt}/soc/serial@7e201000");
        put(
            &root,
            &format!("{uart}/compatible"),
            b"arm,pl011\0arm,primecell\0",
        );
        put(&root, &format!("{uart}/status"), b"okay\0");
        put(
            &root,
            &format!("{uart}/bluetooth/compatible"),
            b"brcm,bcm43438-bt\0",
        );
        put(&root, &format!("{uart}/bluetooth/status"), b"disabled\0");
        let mmc = format!("{dt}/soc/mmcnr@7e300000");
        put(
            &root,
            &format!("{mmc}/compatible"),
            b"brcm,bcm2835-mmc\0brcm,bcm2835-sdhci\0",
        );
        put(&root, &format!("{mmc}/status"), b"disabled\0");
        put(
            &root,
            &format!("{mmc}/wifi@1/compatible"),
            b"brcm,bcm4329-fmac\0",
        );
        // A port with a driver, its console and its getty.
        dir(&root, "devices/platform/soc/3f201000.serial/tty/ttyAMA0");
        dir(&root, "bus/amba/drivers/uart-pl011");
        dir(&root, "sys/class/tty");
        symlink(
            root.join("devices/platform/soc/3f201000.serial"),
            root.join("devices/platform/soc/3f201000.serial/tty/ttyAMA0/device"),
        )
        .unwrap();
        symlink(
            root.join("bus/amba/drivers/uart-pl011"),
            root.join("devices/platform/soc/3f201000.serial/driver"),
        )
        .unwrap();
        symlink(
            root.join("devices/platform/soc/3f201000.serial/tty/ttyAMA0"),
            root.join("sys/class/tty/ttyAMA0"),
        )
        .unwrap();
        dir(&root, "sys/class/tty/tty1");
        put(
            &root,
            "proc/consoles",
            b"ttyAMA0              -W- (EC p a)    204:64\ntty1 -WU (E p ) 4:1\n",
        );
        put(
            &root,
            "etc/inittab",
            b"# ttyAMA0 off\nttyAMA0::respawn:/sbin/getty -L ttyAMA0 115200 vt100\n",
        );
        let text = block(&root);
        assert_eq!(
            text,
            "\n== radios ==\n\
             /sys/class/bluetooth  absent\n\
             /sys/class/net        absent\n\
             bluetooth          /soc/serial@7e201000/bluetooth (brcm,bcm43438-bt) status disabled; \
             parent (arm,pl011, arm,primecell) status okay\n\
             wifi               /soc/mmcnr@7e300000/wifi@1 (brcm,bcm4329-fmac) status okay (no status); \
             parent (brcm,bcm2835-mmc, brcm,bcm2835-sdhci) status disabled\n\
             serial ttyAMA0     driver uart-pl011, console, getty in /etc/inittab\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_board_with_radios_and_a_device_on_its_port_shows_them() {
        let root = tree("radios");
        dir(&root, "sys/class/bluetooth/hci0");
        dir(&root, "sys/class/net/wlan0");
        dir(&root, "sys/class/net/lo");
        dir(&root, "sys/firmware/devicetree/base");
        dir(&root, "sys/bus/serial/devices/serial0-0");
        dir(&root, "bus/serial/drivers/hci_uart_bcm");
        symlink(
            root.join("bus/serial/drivers/hci_uart_bcm"),
            root.join("sys/bus/serial/devices/serial0-0/driver"),
        )
        .unwrap();
        let text = block(&root);
        assert!(text.contains("/sys/class/bluetooth  hci0\n"), "{text}");
        assert!(text.contains("/sys/class/net        lo wlan0\n"), "{text}");
        assert!(
            text.contains("bluetooth          no device-tree node with a bluetooth compatible\n")
        );
        assert!(text.contains("serial             none\n"));
        assert!(
            text.contains("serial device      serial0-0, driver hci_uart_bcm\n"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
