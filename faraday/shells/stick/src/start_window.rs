//! The start window (F202): a device whose input node appears while the
//! shell is starting is trusted, as one on the machine's own bus is, since
//! the person chose what is plugged in at power-on. When the window closes
//! the external USB devices that were present are listed to the person in
//! one notice, so they can see whether anything is there that should not be.
//!
//! The window is a decision on time and on the USB device list only:
//! [`Window`] takes the time since the shell started and the list, and a
//! test drives it with a fake clock. It opens once per power-on: when it
//! closes it writes the devices it trusted to a file, and a process started
//! after a lock reads that file instead of opening it again. [`names`] works out what the notice
//! lists, from `/sys` alone.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use faraday_core::devices::UsbDevice;

/// The window closes once the USB device list has been unchanged this
/// long, if the shell has been up [`MIN_UP`].
pub const QUIET: Duration = Duration::from_millis(1500);

/// The shell has been up at least this long before the quiet time closes
/// the window.
pub const MIN_UP: Duration = Duration::from_secs(3);

/// The window closes at this time whatever happens, so a device that keeps
/// appearing cannot hold it open.
pub const CAP: Duration = Duration::from_secs(8);

/// Whether the start window is open, and what it has seen.
#[derive(Debug, Default)]
pub struct Window {
    last: Option<Vec<UsbDevice>>,
    changed_at: Duration,
    closed: bool,
    /// The `sys` paths of the devices the window trusts: those it let in
    /// while open, or the list a closed window was loaded with.
    trusted: Vec<PathBuf>,
}

/// The file in the boxes directory that carries the window across a lock.
/// It sits beside the Inbox and Outbox folders, not in them.
pub const FILE: &str = "start-window";

/// The first line of [`FILE`].
const HEADER: &str = "faraday start window 1";

impl Window {
    /// A window that opens now: the shell has just started.
    pub fn new() -> Window {
        Window::default()
    }

    /// The window as a process starting now finds it: no file in `dir`
    /// is a power-on, so it opens; a file closes it, with the devices it
    /// lists trusted; a file that cannot be read or is not a list closes
    /// it with nothing trusted.
    pub fn load(dir: &Path) -> Window {
        let closed = |trusted| Window {
            closed: true,
            trusted,
            ..Window::default()
        };
        let text = match fs::read_to_string(dir.join(FILE)) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Window::new(),
            Err(_) => return closed(Vec::new()),
        };
        let mut lines = text.lines();
        if lines.next() != Some(HEADER) || !text.ends_with('\n') {
            return closed(Vec::new());
        }
        let paths: Vec<PathBuf> = lines.map(PathBuf::from).collect();
        if paths.iter().any(|p| !p.is_absolute()) {
            return closed(Vec::new());
        }
        closed(paths)
    }

    /// Writes the trusted devices to `dir`, for a process started after a
    /// lock. If it cannot be written the next process opens the window
    /// again, which is the failure the owner's file is there to prevent,
    /// so the shell says so in its log.
    pub fn save(&self, dir: &Path) {
        let mut text = format!("{HEADER}\n");
        for p in &self.trusted {
            text.push_str(&p.to_string_lossy());
            text.push('\n');
        }
        let tmp = dir.join(format!(".{FILE}.tmp"));
        if let Err(e) = fs::write(&tmp, text).and_then(|()| fs::rename(&tmp, dir.join(FILE))) {
            eprintln!("start window: not saved: {e}");
        }
    }

    /// Whether the device at `sys` is trusted: while the window is open
    /// every device is, and is remembered; after, only one on the list.
    pub fn admit(&mut self, sys: &Path) -> bool {
        if self.closed {
            return self.trusted.iter().any(|p| p == sys);
        }
        self.trusted.push(sys.to_path_buf());
        true
    }

    /// Whether a device found now is trusted.
    pub fn is_open(&self) -> bool {
        !self.closed
    }

    /// Notes the USB device list `usb` at `up` since the shell started.
    pub fn observe(&mut self, up: Duration, usb: Vec<UsbDevice>) {
        if self.last.as_ref() != Some(&usb) {
            self.last = Some(usb);
            self.changed_at = up;
        }
    }

    /// Whether the window is to close at `up`: the cap, or the list quiet
    /// for [`QUIET`] with the shell up [`MIN_UP`].
    pub fn ready(&self, up: Duration) -> bool {
        !self.closed && (up >= CAP || (up >= MIN_UP && up.saturating_sub(self.changed_at) >= QUIET))
    }

    /// Closes the window; a device found from now on is held.
    pub fn close(&mut self) {
        self.closed = true;
    }
}

/// A device the kernel names `1-2` or `3-1.4.2`: a port on a bus, as
/// opposed to a root hub (`usb1`) or an interface (`1-2:1.0`).
fn is_port(name: &str) -> bool {
    let mut parts = name.splitn(2, '-');
    let (bus, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    !bus.is_empty()
        && bus.bytes().all(|b| b.is_ascii_digit())
        && !path.is_empty()
        && path
            .split('.')
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// The USB port of the disk a partition is on, from where `/sys` puts it
/// (`sys_block`, `/sys/class/block`): the last port in the partition's
/// real path. None for a card on the board's own bus.
pub fn medium_port(sys_block: &Path, partition: &str) -> Option<String> {
    let real = fs::canonicalize(sys_block.join(partition)).ok()?;
    real.components()
        .filter_map(|c| c.as_os_str().to_str())
        .rfind(|c| is_port(c))
        .map(str::to_string)
}

/// A hub: it has an interface of the hub class, or is a root hub.
fn is_hub(d: &UsbDevice) -> bool {
    d.port.starts_with("usb") || d.interfaces.iter().any(|i| i.class == 9)
}

/// Whether the device has an interface of a class Faraday can use: HID
/// (3), mass storage (8) or video (14), the classes `faraday-grant`
/// authorises besides hubs. Anything else, such as the Pi 3B+'s own network
/// chip behind a port the firmware does not mark fixed, can do nothing here
/// and would put the notice on every boot with nothing plugged in.
fn has_usable_class(d: &UsbDevice) -> bool {
    d.interfaces
        .iter()
        .any(|i| matches!(i.class, 0x03 | 0x08 | 0x0e))
}

/// What the notice lists: each external USB device in `usb` once, by its
/// product name (empty if it gives none), leaving out hubs, devices on a
/// hard-wired port (the same test as `evdev::trusted_by_bus`) and the
/// device `boot_port` names. `root` is `/sys/bus/usb/devices`.
pub fn names(root: &Path, usb: &[UsbDevice], boot_port: Option<&str>) -> Vec<String> {
    usb.iter()
        .filter(|d| !is_hub(d) && has_usable_class(d) && boot_port != Some(d.port.as_str()))
        .filter(|d| {
            fs::canonicalize(root.join(&d.port)).is_ok_and(|at| !crate::evdev::port_fixed(at))
        })
        .map(|d| d.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_core::devices::Interface;

    fn dev(port: &str, name: &str, class: u8) -> UsbDevice {
        UsbDevice {
            port: port.into(),
            name: name.into(),
            interfaces: vec![Interface {
                class,
                subclass: 0,
                protocol: 0,
            }],
            ..UsbDevice::default()
        }
    }

    fn secs(ms: u64) -> Duration {
        Duration::from_millis(ms)
    }

    /// With the list settled from the first look, the window stays open
    /// until the shell has been up 3 s, then closes.
    #[test]
    fn a_quiet_list_closes_the_window_after_the_minimum_uptime() {
        let mut w = Window::new();
        w.observe(secs(500), vec![dev("1-1", "Keys", 3)]);
        assert!(!w.ready(secs(2000)));
        w.observe(secs(2900), vec![dev("1-1", "Keys", 3)]);
        assert!(!w.ready(secs(2900)));
        assert!(w.ready(secs(3000)));
        w.close();
        assert!(!w.is_open());
    }

    /// A device arriving restarts the quiet time.
    #[test]
    fn a_change_in_the_list_restarts_the_quiet_time() {
        let mut w = Window::new();
        w.observe(secs(500), vec![]);
        w.observe(secs(3500), vec![dev("1-1", "Keys", 3)]);
        assert!(!w.ready(secs(4900)));
        w.observe(secs(4900), vec![dev("1-1", "Keys", 3)]);
        assert!(w.ready(secs(5000)));
    }

    /// Devices that keep arriving do not hold the window past 8 s.
    #[test]
    fn the_window_closes_at_the_cap_whatever_arrives() {
        let mut w = Window::new();
        let mut now = 0;
        while now < 7900 {
            now += 500;
            let list = (0..now / 500)
                .map(|i| dev(&format!("1-{i}"), "X", 3))
                .collect();
            w.observe(secs(now), list);
            assert!(!w.ready(secs(now.min(7999))));
        }
        w.observe(secs(8000), vec![dev("9-9", "Late", 3)]);
        assert!(w.ready(secs(8000)));
    }

    /// A stand-in `/sys/bus/usb/devices`: ports under two root hubs, one
    /// of them hard-wired.
    fn fake_sys(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("faraday-boot-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let put = |dir: &str, name: &str, value: &str| {
            fs::create_dir_all(root.join(dir)).unwrap();
            fs::write(root.join(dir).join(name), value).unwrap();
        };
        put("usb1/1-1", "removable", "removable\n");
        put("usb1/1-2", "removable", "removable\n");
        put("usb1/1-3", "removable", "removable\n");
        put("usb1/1-4", "removable", "fixed\n");
        put("usb1/1-1/1-1.2", "removable", "removable\n");
        root
    }

    fn flat(root: &Path) -> std::path::PathBuf {
        // `/sys/bus/usb/devices` holds links to the real directories.
        let out = root.join("devices");
        fs::create_dir_all(&out).unwrap();
        for (name, to) in [
            ("1-1", "usb1/1-1"),
            ("1-2", "usb1/1-2"),
            ("1-3", "usb1/1-3"),
            ("1-4", "usb1/1-4"),
            ("1-1.2", "usb1/1-1/1-1.2"),
        ] {
            std::os::unix::fs::symlink(root.join(to), out.join(name)).unwrap();
        }
        out
    }

    /// A receiver is one name; hubs, the hard-wired port and the boot
    /// medium are left out; another stick and a device with no name are
    /// listed.
    #[test]
    fn the_notice_lists_external_devices_once() {
        let root = fake_sys("names");
        let devices = flat(&root);
        let usb = vec![
            dev("usb1", "Root Hub", 9),
            dev("1-1", "Hub", 9),
            dev("1-1.2", "Receiver", 3),
            dev("1-2", "Boot stick", 8),
            dev("1-3", "", 8),
            dev("1-4", "Camera", 14),
        ];
        let got = names(&devices, &usb, Some("1-2"));
        let _ = fs::remove_dir_all(&root);
        assert_eq!(got, vec!["Receiver".to_string(), String::new()]);
    }

    fn temp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("faraday-win-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// After a lock a fresh process starts with the window closed: a
    /// device that was trusted is, a new one is held, and a device
    /// unplugged and plugged back in (a new path) is held.
    #[test]
    fn a_lock_does_not_open_the_window_again() {
        let dir = temp("lock");
        let (keys, mouse) = (
            Path::new("/sys/a/input5/event3"),
            Path::new("/sys/a/input6/event4"),
        );
        let mut first = Window::load(&dir);
        assert!(first.is_open());
        assert!(first.admit(keys) && first.admit(mouse));
        first.close();
        first.save(&dir);

        let mut next = Window::load(&dir);
        assert!(!next.is_open(), "no notice: the window is closed");
        assert!(!next.ready(CAP));
        assert!(next.admit(keys) && next.admit(mouse));
        assert!(
            !next.admit(Path::new("/sys/a/input9/event7")),
            "a new device"
        );
        assert!(
            !next.admit(Path::new("/sys/a/input7/event4")),
            "the mouse replugged"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// A list that cannot be read as one trusts nothing and closes the
    /// window.
    #[test]
    fn a_malformed_list_trusts_nothing() {
        let dir = temp("bad");
        for text in [
            "",
            "junk\n",
            "faraday start window 1\nrelative/path\n",
            "faraday start window 1\n/sys/a",
        ] {
            fs::write(dir.join(FILE), text).unwrap();
            let mut w = Window::load(&dir);
            assert!(!w.is_open(), "{text:?}");
            assert!(!w.admit(Path::new("/sys/a")), "{text:?}");
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// A device with no interface Faraday can use is not listed, and with
    /// only that or only the boot medium present nothing is listed.
    #[test]
    fn unusable_devices_and_the_boot_medium_alone_give_no_notice() {
        let root = fake_sys("usable");
        let devices = flat(&root);
        let chip = dev("1-1.2", "", 0xff);
        let got = names(&devices, std::slice::from_ref(&chip), Some("1-2"));
        let boot = names(&devices, &[dev("1-2", "Boot stick", 8)], Some("1-2"));
        let both = names(&devices, &[chip, dev("1-3", "Stick", 8)], Some("1-2"));
        let _ = fs::remove_dir_all(&root);
        assert!(got.is_empty() && boot.is_empty());
        assert_eq!(both, vec!["Stick".to_string()]);
    }

    /// The boot medium's port is found from the partition's real path.
    #[test]
    fn the_boot_medium_is_found_through_sys() {
        let root = std::env::temp_dir().join(format!("faraday-medium-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let part = root.join("devices/pci/usb1/1-2/1-2:1.0/host4/block/sdb/sdb2");
        fs::create_dir_all(&part).unwrap();
        fs::create_dir_all(root.join("class/block")).unwrap();
        std::os::unix::fs::symlink(&part, root.join("class/block/sdb2")).unwrap();
        let got = medium_port(&root.join("class/block"), "sdb2");
        let none = medium_port(&root.join("class/block"), "mmcblk0p2");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(got.as_deref(), Some("1-2"));
        assert_eq!(none, None);
    }
}
