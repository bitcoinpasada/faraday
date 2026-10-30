//! What the grant helper decides (`docs/history/PLAN.md` §4.3, §5.1): which partition
//! nodes go to the disk process, which stay root's, and when they come
//! back.
//!
//! Once a tick it looks at `/sys/class/block`:
//!
//! 1. A partition is considered only when it is a partition, never a whole
//!    disk, and its disk is on the USB bus or is an SD card.
//! 2. Every boot partition is kept back, so the running system never gets
//!    to rewrite its own kernel nor any other stick's: the first partition
//!    of the medium the machine started from, any partition whose GPT name
//!    is a boot partition's (`EFI system partition`, or `FARADAYBOOT` on
//!    a stick made before F174), and the first partition of the Pi's own
//!    card (`docs/STICK-IDENTITY.md` §5.2). All are known from `/sys` and
//!    the medium's record (`faraday_boot::medium`); nothing is read from
//!    a stick for it (`docs/history/PLAN.md` §12, decision 8). A boot
//!    partition is never handed to any process (F128): Faraday writes no
//!    boot partition.
//! 3. Any other partition is handed to the disk process, mode `0600`, but
//!    only while the app's clean marker is there, and only once the
//!    medium the machine started from has been looked for: until its
//!    record says what was found, nothing is handed out.
//! 4. When the clean marker goes, every partition handed out is taken
//!    back, and is decided again when the marker returns.
//!
//! It also applies the USB device policy (`docs/history/PLAN.md` §4.6, layer 2). The
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
//!
//! [`Finder`] is the other half, run once at start as `faraday-grant
//! --find-boot` (`docs/STICK-IDENTITY.md` §5.2): it finds the stick the PC
//! started from among the USB disks with a Faraday stick's layout, by the
//! build its boot partition holds, and writes the record the rest reads.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use faraday_boot::layout::{self, Span};
use faraday_boot::medium::Record;

/// The changes of ownership the helper makes.
pub trait Owner {
    /// Makes the node the helper's own, mode `0600`, so nobody else can
    /// open it on the way to the disk process.
    fn take(&mut self, node: &Path) -> std::io::Result<()>;
    /// Gives the node to the disk process.
    fn give(&mut self, node: &Path) -> std::io::Result<()>;
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
/// video and hub. Nothing else has a driver either (`docs/history/PLAN.md` §4.6).
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

/// Whether `/sys` and the medium's record say this is a boot partition:
/// the Pi's own card's first, one named as a boot partition is, or the
/// first partition of the medium the machine started from.
fn is_boot(name: &str, dir: &Path, record: &Record) -> bool {
    name == PI_BOOT
        || partname(dir).is_some_and(|n| layout::is_boot_name(&n))
        || (number(dir) == Some(1) && dir.parent().is_some_and(|d| boot_disk(d, record)))
}

/// The partition's number on its disk, as `/sys` states it.
fn number(dir: &Path) -> Option<u32> {
    read_u64(&dir.join("partition")).and_then(|n| u32::try_from(n).ok())
}

fn read_u64(p: &Path) -> Option<u64> {
    fs::read_to_string(p).ok()?.trim().parse().ok()
}

/// The partitions of a disk in `/sys`: each one's name, directory and
/// number, in the order of their numbers.
fn partitions(disk: &Path) -> Vec<(String, PathBuf, u32)> {
    let mut out: Vec<(String, PathBuf, u32)> = fs::read_dir(disk)
        .map(|d| {
            d.flatten()
                .filter_map(|e| {
                    let dir = e.path();
                    let n = number(&dir)?;
                    Some((e.file_name().to_string_lossy().into_owned(), dir, n))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|p| p.2);
    out
}

/// Whether the disk at `disk` in `/sys` is the medium the machine started
/// from, as the record names it.
fn boot_disk(disk: &Path, record: &Record) -> bool {
    let seq = fs::read_to_string(disk.join("diskseq")).unwrap_or_default();
    let children = partitions(disk);
    record.is_boot_disk(children.iter().map(|c| c.0.as_str()), &seq)
}

/// What became of a partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The disk process has it.
    Handed,
    /// The boot partition: kept back, root's.
    Boot,
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
    /// The medium's record (`/etc/faraday/exchange`), read each tick.
    record: PathBuf,
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
    /// the medium's record on the device.
    pub fn new(sys: &Path, dev: &Path, marker: &Path, record: &Path, owner: O) -> Grant<O> {
        Grant {
            sys: sys.to_path_buf(),
            dev: dev.to_path_buf(),
            marker: marker.to_path_buf(),
            record: record.to_path_buf(),
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
        // Devices with an interface authorised on this tick.
        let mut touched = Vec::new();
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
                touched.push(device.to_string());
                Usb::Authorized
            } else {
                Usb::Refused
            };
            self.usb.insert(id, d);
        }
        // Drivers are asked for only once every interface of the device
        // that may be is authorised. A driver claims its device's other
        // interfaces as it binds, and the kernel refuses a claim on one
        // not yet authorised: uvcvideo, bound to a webcam's control
        // interface before its streaming interface was authorised, makes
        // no /dev/video at all. An interface authorised on an earlier
        // tick that has no driver yet is asked again with its siblings.
        for (name, dir) in &interfaces {
            let device = name.split(':').next().unwrap_or_default();
            if !touched.iter().any(|t| t == device)
                || read(&dir.join("authorized")).as_deref() != Some("1")
                || dir.join("driver").exists()
            {
                continue;
            }
            let _ = self
                .owner
                .probe(&self.sys.join("bus/usb/drivers_probe"), name);
        }
        // An interface gone with its device is forgotten.
        self.usb.retain(|id, _| seen.contains(id));
    }

    /// Input and video nodes that appeared after boot get the owners
    /// `/etc/mdev.conf` gave the ones present at boot: input to the app's
    /// user (`faraday:faraday 0640`), video to its group
    /// (`root:faraday 0660`). No hotplug helper runs, so nothing else
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
        let record = Record::parse(&fs::read_to_string(&self.record).unwrap_or_default());
        // Nothing is handed out before the medium has been looked for: its
        // boot partition could not yet be told by its place.
        let clean = self.marker.exists() && record != Record::Waiting;
        let now = self.removable();
        // A partition gone from /sys is forgotten: its node went with it.
        self.decided
            .retain(|id, _| now.iter().any(|(n, _, _, _)| n == id));
        for (node, d) in self.decided.values_mut() {
            let back = match *d {
                Decision::Handed if !clean => Decision::Waiting,
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
            let d = if is_boot(&name, &dir, &record) {
                match before {
                    // Never given: root's.
                    None | Some(Decision::Boot) => Decision::Boot,
                    // A take-back that failed: back to root, tried again
                    // each tick until it is.
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

/// How long the set of USB disks must stay the same, once a stick of this
/// build is among them, before the finder decides: a second stick put in
/// with the first appears a moment later, and with two in neither is
/// taken (`docs/STICK-IDENTITY.md` §11).
pub const SETTLE: Duration = Duration::from_secs(3);

/// How long the finder looks before it says no stick of this build is in.
pub const GIVE_UP: Duration = Duration::from_secs(30);

/// How much of a boot partition is read at a time.
const CHUNK: usize = 1 << 20;

/// A USB disk with a Faraday stick's layout.
struct Candidate {
    /// Its name and sequence number this boot: `sdb#7`.
    id: String,
    seq: String,
    /// Its first partition's node.
    boot: PathBuf,
    /// Its second partition's name.
    data: String,
}

/// The finder (`docs/STICK-IDENTITY.md` §5.2): which USB disk is the
/// stick the machine started from, by its layout and its boot partition's
/// release string, looked at again until it can say.
pub struct Finder {
    sys: PathBuf,
    dev: PathBuf,
    release: String,
    /// Each candidate read, and whether it holds this build.
    held: BTreeMap<String, bool>,
    /// The USB disks last seen, and when that set last changed.
    seen: Vec<String>,
    changed: Duration,
}

impl Finder {
    /// The finder over `/sys` and `/dev`, for the running kernel's release
    /// string.
    pub fn new(sys: &Path, dev: &Path, release: &str) -> Finder {
        Finder {
            sys: sys.to_path_buf(),
            dev: dev.to_path_buf(),
            release: release.trim().to_string(),
            held: BTreeMap::new(),
            seen: Vec::new(),
            changed: Duration::ZERO,
        }
    }

    /// The whole disks on the USB bus, and those among them with a
    /// Faraday stick's layout as `/sys` states it.
    fn disks(&self) -> (Vec<String>, Vec<Candidate>) {
        let mut all = Vec::new();
        let mut out = Vec::new();
        let Ok(entries) = fs::read_dir(self.sys.join("class/block")) else {
            return (all, out);
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(dir) = resolve(&e.path()) else {
                continue;
            };
            let on_usb = dir
                .components()
                .any(|c| c.as_os_str().to_str().is_some_and(|s| s.starts_with("usb")));
            if dir.join("partition").exists() || !on_usb {
                continue;
            }
            let seq = fs::read_to_string(dir.join("diskseq"))
                .unwrap_or_default()
                .trim()
                .to_string();
            let id = format!("{name}#{seq}");
            all.push(id.clone());
            let parts = partitions(&dir);
            let spans: Vec<Span> = parts
                .iter()
                .filter_map(|(_, d, _)| {
                    Some(Span {
                        start: read_u64(&d.join("start"))?,
                        size: read_u64(&d.join("size"))?,
                    })
                })
                .collect();
            if spans.len() != parts.len() || !layout::is_stick(&spans) {
                continue;
            }
            out.push(Candidate {
                id,
                seq,
                boot: self.dev.join(&parts[0].0),
                data: parts[1].0.clone(),
            });
        }
        all.sort();
        (all, out)
    }

    /// Costs a read of up to 64 MiB: whether the boot partition at `node`
    /// holds the running build's release string
    /// (`faraday_boot::holds_release`), read a piece at a time and stopped
    /// where it is found.
    fn holds(&self, node: &Path) -> std::io::Result<bool> {
        let mut f = fs::File::open(node)?;
        let keep = faraday_boot::MAX_RELEASE + 1;
        let mut buf: Vec<u8> = Vec::with_capacity(CHUNK + keep);
        let mut chunk = vec![0u8; CHUNK];
        let mut read: u64 = 0;
        while read < faraday_boot::MAX_PARTITION {
            let n = f.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            read += n as u64;
            buf.extend_from_slice(&chunk[..n]);
            if faraday_boot::holds_release(&buf, &self.release) {
                return Ok(true);
            }
            // The tail, in case the string runs across two pieces.
            let cut = buf.len().saturating_sub(keep);
            buf.drain(..cut);
        }
        Ok(false)
    }

    /// Costs what [`Finder::holds`] costs for each candidate not read
    /// before. One look, `now` after the finder started: the record once
    /// it can say what was found, `None` while it cannot yet. One stick of
    /// this build is found once the USB disks have stayed the same for
    /// [`SETTLE`]; none, or two, once [`GIVE_UP`] has passed.
    pub fn look(&mut self, now: Duration) -> Option<Record> {
        let (all, candidates) = self.disks();
        if all != self.seen {
            self.seen = all;
            self.changed = now;
        }
        // A disk gone is forgotten; one whose read failed is read again.
        self.held
            .retain(|id, _| candidates.iter().any(|c| &c.id == id));
        let mut unread = false;
        for c in &candidates {
            if self.held.contains_key(&c.id) {
                continue;
            }
            match self.holds(&c.boot) {
                Ok(h) => {
                    self.held.insert(c.id.clone(), h);
                }
                Err(_) => unread = true,
            }
        }
        let found: Vec<&Candidate> = candidates
            .iter()
            .filter(|c| self.held.get(&c.id) == Some(&true))
            .collect();
        let settled = !found.is_empty() && !unread && now.saturating_sub(self.changed) >= SETTLE;
        if !settled && now < GIVE_UP {
            return None;
        }
        Some(match found.as_slice() {
            [] => Record::NotFound,
            [one] => Record::Found {
                partition: one.data.clone(),
                diskseq: Some(one.seq.clone()),
            },
            _ => Record::Two,
        })
    }
}
