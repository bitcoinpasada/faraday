//! What the grant helper decides (`PLAN.md` §4.3, §5.1): which partition
//! nodes go to the disk process, which stay root's, and when they come
//! back.
//!
//! Once a tick it looks at `/sys/class/block`:
//!
//! 1. A partition is considered only when it is a partition, never a whole
//!    disk, and its disk is on the USB bus or is an SD card.
//! 2. The boot partition is kept back, so the running system never gets
//!    to rewrite its own kernel: a partition whose GPT name is `OSKBOOT`,
//!    or the first partition of the Pi's own card. Both are known from
//!    `/sys`; nothing is read from a stick (`PLAN.md` §12, decision 8).
//!    The one exception is upgrading a stick (`PLAN.md` §5.5): while the
//!    app publishes its upgrade marker beside the clean marker, a
//!    partition named `OSKBOOT` is handed to the boot copier, mode
//!    `0600`, and never to the disk process. The Pi's own card is not,
//!    yet.
//! 3. Any other partition is handed to the disk process, mode `0600`, but
//!    only while the app's clean marker is there.
//! 4. When the clean marker goes, every partition handed out is taken
//!    back, and is decided again when the marker returns; when either
//!    marker goes, every boot partition handed to the copier is.
//!
//! It also applies the USB device policy (`PLAN.md` §4.6, layer 2). The
//! kernel authorises only devices on hard-wired ports by itself
//! (`usbcore.authorized_default=2`), and `rcS` sets every root hub to
//! authorise no interface by default. Here each other device is
//! authorised, and then its interfaces one by one: mass storage, HID,
//! video and hub, and no others. A device with a storage interface has
//! only that kind authorised; its HID interfaces never are, so a stick
//! that also says it is a keyboard types nothing.
//!
//! The work that needs privilege is behind [`Owner`], so the decisions
//! are tested against a stand-in `/sys` and `/dev` by a user who has none.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// The boot partition's GPT name and FAT label.
pub const BOOT: &str = "OSKBOOT";

/// The changes of ownership the helper makes.
pub trait Owner {
    /// Makes the node the helper's own, mode `0600`, so nobody else can
    /// open it on the way to the disk process.
    fn take(&mut self, node: &Path) -> std::io::Result<()>;
    /// Gives the node to the disk process.
    fn give(&mut self, node: &Path) -> std::io::Result<()>;
    /// Gives a boot partition's node to the boot copier.
    fn give_boot(&mut self, node: &Path) -> std::io::Result<()>;
    /// Gives the node back to root.
    fn back(&mut self, node: &Path) -> std::io::Result<()>;
    /// Writes `1` to a USB `authorized` attribute, first making it the
    /// helper's own.
    fn authorize(&mut self, attr: &Path) -> std::io::Result<()>;
    /// Gives a node to `uid:gid` with `mode`, as `/etc/mdev.conf` would
    /// have at boot: for an input or video device that appeared later.
    fn assign(&mut self, node: &Path, uid: u32, gid: u32, mode: u32) -> std::io::Result<()>;
    /// The node's owner and group.
    fn owner_of(&mut self, node: &Path) -> std::io::Result<(u32, u32)>;
    /// Asks the USB bus to bind a driver to an interface just authorised,
    /// by writing its name to `drivers_probe`: authorising alone binds
    /// nothing.
    fn probe(&mut self, drivers_probe: &Path, interface: &str) -> std::io::Result<()>;
}

/// The USB interface classes that are authorised: mass storage, HID,
/// video and hub. Nothing else has a driver either (`PLAN.md` §4.6).
const STORAGE: u8 = 0x08;
const ALLOWED: [u8; 4] = [STORAGE, 0x03, 0x0e, 0x09];

/// What became of a USB interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usb {
    /// Authorised.
    Authorized,
    /// Left unauthorised: a class with no place here, or HID beside
    /// storage.
    Refused,
}

/// The Pi's own card's first partition: the boot partition of an MBR
/// card, which has no partition names but a fixed layout.
const PI_BOOT: &str = "mmcblk0p1";

/// Whether `/sys` says this is the boot partition.
fn is_boot(name: &str, dir: &Path) -> bool {
    name == PI_BOOT || partname(dir).as_deref() == Some(BOOT)
}

/// Whether a boot partition may go to the boot copier while the app is
/// upgrading a stick: one named `OSKBOOT`. The Pi's own card's first
/// partition is not, until the Pi's upgrade is built (`PLAN.md` §5.5).
fn is_upgradable(name: &str) -> bool {
    name != PI_BOOT
}

/// What became of a partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The disk process has it.
    Handed,
    /// The boot partition: kept back, root's.
    Boot,
    /// A boot partition the boot copier has, while the app is upgrading
    /// a stick.
    Upgrade,
    /// Root's, waiting for the clean marker.
    Waiting,
    /// Could not be read or changed: tried again next tick.
    Failed,
}

/// The helper's state over its roots.
pub struct Grant<O: Owner> {
    sys: PathBuf,
    dev: PathBuf,
    marker: PathBuf,
    upgrade: PathBuf,
    /// Each partition by node and disk sequence, and what was decided.
    pub decided: BTreeMap<String, (PathBuf, Decision)>,
    /// Each USB interface by name and its device's number, and what was
    /// decided.
    pub usb: BTreeMap<String, Usb>,
    owner: O,
}

/// A symlink's target as a path, its `..` taken off: `/sys` links are
/// relative, and following them component by component needs no more
/// than `readlink`.
fn resolve(link: &Path) -> Option<PathBuf> {
    let target = fs::read_link(link).ok()?;
    let joined = if target.is_absolute() {
        target
    } else {
        link.parent()?.join(target)
    };
    let mut out = PathBuf::new();
    for c in joined.components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    Some(out)
}

fn partname(dir: &Path) -> Option<String> {
    fs::read_to_string(dir.join("uevent"))
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("PARTNAME="))
        .map(str::to_string)
}

impl<O: Owner> Grant<O> {
    /// The helper over these roots: `/sys`, `/dev`, the clean marker and
    /// the upgrade marker on the device.
    pub fn new(sys: &Path, dev: &Path, marker: &Path, upgrade: &Path, owner: O) -> Grant<O> {
        Grant {
            sys: sys.to_path_buf(),
            dev: dev.to_path_buf(),
            marker: marker.to_path_buf(),
            upgrade: upgrade.to_path_buf(),
            decided: BTreeMap::new(),
            usb: BTreeMap::new(),
            owner,
        }
    }

    /// The partitions on a USB or SD disk now, by identity, with their
    /// node and their `/sys` directory.
    fn removable(&self) -> Vec<(String, String, PathBuf, PathBuf)> {
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(self.sys.join("class/block")) else {
            return out;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(dir) = resolve(&e.path()) else {
                continue;
            };
            if !dir.join("partition").exists() {
                continue;
            }
            let Some(disk) = dir.parent() else { continue };
            let on_usb = dir
                .components()
                .any(|c| c.as_os_str().to_str().is_some_and(|s| s.starts_with("usb")));
            if !on_usb && !name.starts_with("mmcblk") {
                continue;
            }
            let seq = fs::read_to_string(disk.join("diskseq")).unwrap_or_default();
            out.push((
                format!("{name}#{}", seq.trim()),
                name.clone(),
                self.dev.join(&name),
                dir,
            ));
        }
        out
    }

    /// Hands a boot partition to the boot copier.
    fn decide_boot(&mut self, node: &Path) -> Decision {
        if self.owner.take(node).is_err() {
            return Decision::Failed;
        }
        match self.owner.give_boot(node) {
            Ok(()) => Decision::Upgrade,
            Err(_) => {
                let _ = self.owner.back(node);
                Decision::Failed
            }
        }
    }

    fn decide(&mut self, node: &Path) -> Decision {
        if self.owner.take(node).is_err() {
            return Decision::Failed;
        }
        match self.owner.give(node) {
            Ok(()) => Decision::Handed,
            Err(_) => {
                let _ = self.owner.back(node);
                Decision::Failed
            }
        }
    }

    /// The USB policy, once over every device and interface now present.
    fn usb_tick(&mut self) {
        let root = self.sys.join("bus/usb/devices");
        let Ok(entries) = fs::read_dir(&root) else {
            return;
        };
        let read = |p: &Path| fs::read_to_string(p).ok().map(|t| t.trim().to_string());
        let mut devices = Vec::new();
        let mut interfaces = Vec::new();
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(dir) = resolve(&e.path()) else {
                continue;
            };
            if name.contains(':') {
                interfaces.push((name, dir));
            } else {
                devices.push((name, dir));
            }
        }
        devices.sort();
        interfaces.sort();
        // A device the kernel did not authorise by itself is authorised
        // here; what it may do is decided by its interfaces.
        for (_, dir) in &devices {
            if read(&dir.join("authorized")).as_deref() == Some("0") {
                let _ = self.owner.authorize(&dir.join("authorized"));
            }
        }
        let mut seen = Vec::new();
        for (name, dir) in &interfaces {
            let Some(device) = name.split(':').next() else {
                continue;
            };
            let devnum = devices
                .iter()
                .find(|(n, _)| n == device)
                .and_then(|(_, d)| read(&d.join("devnum")))
                .unwrap_or_default();
            let id = format!("{name}#{devnum}");
            seen.push(id.clone());
            if self.usb.contains_key(&id) {
                continue;
            }
            if read(&dir.join("authorized")).as_deref() != Some("0") {
                continue;
            }
            let class = |d: &Path| {
                read(&d.join("bInterfaceClass")).and_then(|c| u8::from_str_radix(&c, 16).ok())
            };
            let storage_beside = interfaces
                .iter()
                .filter(|(n, _)| n.split(':').next() == Some(device))
                .any(|(_, d)| class(d) == Some(STORAGE));
            let ok = match class(dir) {
                Some(c) if storage_beside => c == STORAGE,
                Some(c) => ALLOWED.contains(&c),
                None => false,
            };
            let d = if ok && self.owner.authorize(&dir.join("authorized")).is_ok() {
                let _ = self
                    .owner
                    .probe(&self.sys.join("bus/usb/drivers_probe"), name);
                Usb::Authorized
            } else {
                Usb::Refused
            };
            self.usb.insert(id, d);
        }
        // An interface gone with its device is forgotten.
        self.usb.retain(|id, _| seen.contains(id));
    }

    /// Input and video nodes that appeared after boot get the owners
    /// `/etc/mdev.conf` gave the ones present at boot: input to the app's
    /// user (`opensigner:opensigner 0640`), video to its group
    /// (`root:opensigner 0660`). No hotplug helper runs, so nothing else
    /// would, and a keyboard plugged in later could not be read.
    fn devnodes_tick(&mut self) {
        const APP: u32 = 200;
        let lists = [
            (self.dev.join("input"), "event", APP, APP, 0o640),
            (self.dev.clone(), "video", 0, APP, 0o660),
        ];
        for (dir, prefix, uid, gid, mode) in lists {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let numbered = name
                    .strip_prefix(prefix)
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
                if !numbered {
                    continue;
                }
                let node = e.path();
                if self.owner.owner_of(&node).ok() != Some((uid, gid)) {
                    let _ = self.owner.assign(&node, uid, gid, mode);
                }
            }
        }
    }

    /// One look at the machine.
    pub fn tick(&mut self) {
        self.usb_tick();
        self.devnodes_tick();
        let clean = self.marker.exists();
        // Upgrading a stick: only ever beside the clean marker.
        let upgrade = clean && self.upgrade.exists();
        let now = self.removable();
        // A partition gone from /sys is forgotten: its node went with it.
        self.decided
            .retain(|id, _| now.iter().any(|(n, _, _, _)| n == id));
        for (node, d) in self.decided.values_mut() {
            let back = match *d {
                Decision::Handed if !clean => Decision::Waiting,
                Decision::Upgrade if !upgrade => Decision::Boot,
                _ => continue,
            };
            *d = if self.owner.back(node).is_ok() {
                back
            } else {
                Decision::Failed
            };
        }
        for (id, name, node, dir) in now {
            let before = self.decided.get(&id).map(|(_, d)| *d);
            let d = if is_boot(&name, &dir) {
                match before {
                    Some(Decision::Upgrade) => continue,
                    _ if upgrade && is_upgradable(&name) => self.decide_boot(&node),
                    // Taken back from the copier, or never given: root's.
                    None | Some(Decision::Boot) => Decision::Boot,
                    // A give or a take-back that failed: back to root,
                    // tried again each tick until it is.
                    _ => match self.owner.back(&node) {
                        Ok(()) => Decision::Boot,
                        Err(_) => Decision::Failed,
                    },
                }
            } else {
                match before {
                    Some(Decision::Handed) => continue,
                    _ if !clean => Decision::Waiting,
                    _ => self.decide(&node),
                }
            };
            self.decided.insert(id, (node, d));
        }
    }
}
