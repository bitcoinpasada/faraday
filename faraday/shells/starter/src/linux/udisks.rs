//! udisks2's objects read as the Starter's disks (`docs/STARTER.md` §2, §4.2),
//! with no D-Bus in it: [`Objects`] is the answer to
//! `org.freedesktop.DBus.ObjectManager.GetManagedObjects` on
//! `/org/freedesktop/UDisks2`, as the transport ([`super::bus`]) reads it
//! or as a test records it ([`Objects::parse`]).
//!
//! A disk is a `Drive` with a whole-disk `Block`; its partitions are the
//! `Block`s whose `Partition.Table` is that block; what is on them comes
//! from their `Block.Id*`, `Filesystem.MountPoints` and
//! `Swapspace.Active`, and from any unlocked `Block` whose
//! `CryptoBackingDevice` is one of them.
//!
//! What the kernel says is built on a block (its `holders` in sysfs) is
//! not udisks2's to tell: the transport reads it and records it on the
//! block as a `Holders` interface with a `Names` list, empty when
//! nothing holds it. A whole disk or partition with no `Holders` was not
//! checked, and the disk is in use ([`InUse::Unchecked`]).

use std::collections::BTreeMap;
use std::path::PathBuf;

use faraday_starter::api::{Bus, Disk, FaradayStick, InUse, Volume};

use crate::gpt;
use crate::writer::Identity;

/// The interfaces' common prefix.
const IFACE: &str = "org.freedesktop.UDisks2.";

/// A property's value, as far as the Starter reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `s`.
    Str(String),
    /// `b`.
    Bool(bool),
    /// `t`, `u` or any other unsigned number.
    Num(u64),
    /// `o`.
    Path(String),
    /// `ay`: a byte string, NUL-terminated as udisks2 sends paths.
    Bytes(Vec<u8>),
    /// `aay`: a list of byte strings (mount points).
    ByteLists(Vec<Vec<u8>>),
    /// Anything else, which the Starter does not read.
    Other,
}

/// One interface's properties.
pub type Props = BTreeMap<String, Value>;

/// Every object udisks2 manages: its path, then each interface (without
/// the `org.freedesktop.UDisks2.` prefix) with its properties.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Objects(pub BTreeMap<String, BTreeMap<String, Props>>);

/// A Faraday stick's boot partition, to read its version from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootPart {
    /// The partition's object (its `Filesystem` to mount).
    pub object: String,
    /// Where the desktop mounted it, when it did.
    pub mounted: Option<PathBuf>,
}

/// A disk as read, and what reading its version needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The disk, its `faraday` version not yet read.
    pub disk: Disk,
    /// A Faraday stick's boot partition.
    pub boot: Option<BootPart>,
}

/// Where the desktop mounts removable disks: a partition mounted
/// anywhere else (the `fstab` places under `/mnt` included) is one the
/// computer uses (§4.2).
const DESKTOP_MOUNTS: [&str; 2] = ["/run/media/", "/media/"];

/// The interface the transport records a block's holders on.
pub const HOLDERS: &str = "Holders";
/// Its property: the kernel names of the devices built on the block.
pub const HOLDERS_NAMES: &str = "Names";

/// A block's kernel name (`sdb1`, from `Block.Device` = `/dev/sdb1`),
/// which names its directory in `/sys/class/block`.
pub fn kernel_name(device: &str) -> Option<&str> {
    device
        .rsplit('/')
        .next()
        .filter(|n| !n.is_empty() && *n != "." && *n != "..")
}

fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

impl Objects {
    fn props(&self, path: &str, iface: &str) -> Option<&Props> {
        self.0.get(path)?.get(iface)
    }

    fn str(&self, path: &str, iface: &str, prop: &str) -> String {
        match self.props(path, iface).and_then(|p| p.get(prop)) {
            Some(Value::Str(s)) => s.trim().to_string(),
            _ => String::new(),
        }
    }

    fn bool(&self, path: &str, iface: &str, prop: &str) -> bool {
        matches!(
            self.props(path, iface).and_then(|p| p.get(prop)),
            Some(Value::Bool(true))
        )
    }

    fn num(&self, path: &str, iface: &str, prop: &str) -> u64 {
        match self.props(path, iface).and_then(|p| p.get(prop)) {
            Some(Value::Num(n)) => *n,
            _ => 0,
        }
    }

    fn path(&self, path: &str, iface: &str, prop: &str) -> Option<&str> {
        match self.props(path, iface).and_then(|p| p.get(prop)) {
            Some(Value::Path(p)) if p != "/" => Some(p),
            _ => None,
        }
    }

    fn device(&self, block: &str) -> String {
        let read = |prop: &str| match self.props(block, "Block").and_then(|p| p.get(prop)) {
            Some(Value::Bytes(b)) => text(b),
            _ => String::new(),
        };
        let d = read("PreferredDevice");
        if d.is_empty() { read("Device") } else { d }
    }

    /// The mount points of a block's filesystem.
    pub fn mount_points(&self, block: &str) -> Vec<String> {
        match self
            .props(block, "Filesystem")
            .and_then(|p| p.get("MountPoints"))
        {
            Some(Value::ByteLists(l)) => l
                .iter()
                .map(|b| text(b))
                .filter(|s| !s.is_empty())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The whole-disk block of `drive`.
    fn whole(&self, drive: &str) -> Option<&str> {
        self.0
            .iter()
            .filter(|(p, ifaces)| {
                ifaces.contains_key("Block")
                    && !ifaces.contains_key("Partition")
                    && self.path(p, "Block", "Drive") == Some(drive)
                    && self.path(p, "Block", "CryptoBackingDevice").is_none()
            })
            .map(|(p, _)| p.as_str())
            .next()
    }

    /// The partitions of a whole-disk block, in their numbers' order.
    pub fn partitions(&self, whole: &str) -> Vec<&str> {
        let mut parts: Vec<(u64, &str)> = self
            .0
            .keys()
            .filter(|p| self.path(p, "Partition", "Table") == Some(whole))
            .map(|p| (self.num(p, "Partition", "Number"), p.as_str()))
            .collect();
        parts.sort();
        parts.into_iter().map(|(_, p)| p).collect()
    }

    /// The blocks unlocked from `backing` (a LUKS partition's cleartext
    /// device), and those unlocked from them.
    fn unlocked(&self, backing: &str) -> Vec<&str> {
        let mut out = Vec::new();
        for p in self.0.keys() {
            if self.path(p, "Block", "CryptoBackingDevice") == Some(backing) {
                out.push(p.as_str());
                out.extend(self.unlocked(p));
            }
        }
        out
    }

    /// Every block whose use counts for the disk `whole`: itself, its
    /// partitions, and what is unlocked from either.
    fn blocks_of<'a>(&'a self, whole: &'a str) -> Vec<&'a str> {
        let mut blocks = vec![whole];
        blocks.extend(self.partitions(whole));
        let unlocked: Vec<&str> = blocks.iter().flat_map(|b| self.unlocked(b)).collect();
        blocks.extend(unlocked);
        blocks
    }

    /// Whether a block has a holder: `Some(true)` when the kernel lists
    /// one, `Some(false)` when it lists none, `None` when the holders
    /// were not read.
    fn held(&self, block: &str) -> Option<bool> {
        match self.props(block, HOLDERS)?.get(HOLDERS_NAMES) {
            Some(Value::ByteLists(l)) => Some(l.iter().any(|b| !text(b).is_empty())),
            _ => None,
        }
    }

    /// Why the computer uses the disk `whole`, when it does: a mount
    /// point outside the desktop's places, active swap, a device the
    /// kernel lists as built on the disk or one of its partitions, or,
    /// when that could not be read, that it was not checked.
    pub fn in_use(&self, whole: &str) -> Option<InUse> {
        let blocks = self.blocks_of(whole);
        for b in &blocks {
            for m in self.mount_points(b) {
                let desktop = DESKTOP_MOUNTS.iter().any(|d| m.starts_with(d));
                if !desktop {
                    return Some(InUse::Mounted(m));
                }
            }
        }
        if blocks.iter().any(|b| self.bool(b, "Swapspace", "Active")) {
            return Some(InUse::Swap);
        }
        // The disk and its partitions: what is unlocked from them is
        // held by them already.
        let held: Vec<Option<bool>> = std::iter::once(whole)
            .chain(self.partitions(whole))
            .map(|b| self.held(b))
            .collect();
        if held.contains(&Some(true)) {
            Some(InUse::Held)
        } else if held.contains(&None) {
            Some(InUse::Unchecked)
        } else {
            None
        }
    }

    /// The filesystems the desktop mounted from the disk `whole`, to
    /// unmount before a write.
    pub fn desktop_mounted(&self, whole: &str) -> Vec<String> {
        self.blocks_of(whole)
            .into_iter()
            .filter(|b| !self.mount_points(b).is_empty())
            .map(str::to_string)
            .collect()
    }

    /// The drive of the whole-disk block `whole`.
    fn drive_of(&self, whole: &str) -> Option<&str> {
        self.path(whole, "Block", "Drive")
    }

    /// The disk whose whole-disk block is `whole` as it is now, for the
    /// writer's check that it is still the disk confirmed.
    pub fn identity(&self, whole: &str) -> Option<Identity> {
        let drive = self.drive_of(whole)?;
        Some(Identity {
            device: self.device(whole),
            serial: self.str(drive, "Drive", "Serial"),
            size: self.size(drive, whole),
            in_use: self.in_use(whole),
        })
    }

    fn size(&self, drive: &str, whole: &str) -> u64 {
        match self.num(drive, "Drive", "Size") {
            0 => self.num(whole, "Block", "Size"),
            n => n,
        }
    }

    /// Every disk attached, as the Starter lists them: each `Drive` with a
    /// whole-disk block and a size (a card reader without a card has
    /// none).
    pub fn disks(&self) -> Vec<Found> {
        let mut out = Vec::new();
        for (drive, ifaces) in &self.0 {
            if !ifaces.contains_key("Drive") {
                continue;
            }
            let Some(whole) = self.whole(drive) else {
                continue;
            };
            let size = self.size(drive, whole);
            if size == 0 {
                continue;
            }
            let device = self.device(whole);
            let usb = self.str(drive, "Drive", "ConnectionBus") == "usb";
            let hotplug = self.bool(drive, "Drive", "Removable")
                || self.bool(drive, "Drive", "Ejectable")
                || self.bool(drive, "Drive", "MediaRemovable");
            let bus = if usb && hotplug {
                Bus::Usb
            } else if device.starts_with("/dev/mmcblk") {
                Bus::SdReader
            } else {
                Bus::Other
            };
            let parts = self.partitions(whole);
            let volume = |b: &str| {
                let label = self.str(b, "Block", "IdLabel");
                Volume {
                    label: (!label.is_empty()).then_some(label),
                    files: !self.str(b, "Block", "IdUsage").is_empty(),
                }
            };
            let volumes: Vec<Volume> = if parts.is_empty() {
                // A filesystem on the whole disk, with no partition table.
                if self.str(whole, "Block", "IdUsage").is_empty() {
                    Vec::new()
                } else {
                    vec![volume(whole)]
                }
            } else {
                parts.iter().map(|p| volume(p)).collect()
            };
            // A Faraday stick by its layout alone
            // (`docs/STICK-IDENTITY.md` §5.1): udisks2's `Partition.Type`
            // is the GPT type GUID, `Offset` and `Size` in bytes.
            let types: Vec<String> = parts
                .iter()
                .map(|p| self.str(p, "Partition", "Type"))
                .collect();
            let placed: Vec<gpt::Placed<'_>> = parts
                .iter()
                .zip(&types)
                .map(|(p, t)| gpt::Placed {
                    type_guid: t,
                    offset: self.num(p, "Partition", "Offset"),
                    len: self.num(p, "Partition", "Size"),
                })
                .collect();
            let stick =
                self.str(whole, "PartitionTable", "Type") == "gpt" && gpt::layout(&placed).is_ok();
            let boot = stick.then(|| BootPart {
                object: parts[0].to_string(),
                mounted: self.mount_points(parts[0]).first().map(PathBuf::from),
            });
            let vendor = self.str(drive, "Drive", "Vendor");
            out.push(Found {
                disk: Disk {
                    id: whole.to_string(),
                    device,
                    vendor,
                    model: self.str(drive, "Drive", "Model"),
                    serial: self.str(drive, "Drive", "Serial"),
                    size,
                    bus,
                    volumes,
                    in_use: self.in_use(whole),
                    faraday: boot.as_ref().map(|_| FaradayStick {
                        release: None,
                        read: true,
                    }),
                },
                boot,
            });
        }
        out
    }

    /// Reads a recorded answer: one line per property, indented under
    /// its object and interface, as `tests/udisks/*.txt` hold them:
    ///
    /// ```text
    /// /org/freedesktop/UDisks2/drives/Maker_Stick_1234
    ///   Drive
    ///     Vendor s Maker
    ///     Size t 16008609792
    ///     Removable b true
    /// /org/freedesktop/UDisks2/block_devices/sdb
    ///   Block
    ///     Device ay /dev/sdb
    ///     Drive o /org/freedesktop/UDisks2/drives/Maker_Stick_1234
    ///   Filesystem
    ///     MountPoints aay /run/media/me/STICK,/mnt/other
    /// ```
    ///
    /// A block's holders, as the transport reads them from sysfs, are a
    /// `Holders` interface with `Names aay`, the holders' kernel names
    /// separated by commas, empty for none. A block meant to be free
    /// states them, since one without is not checked:
    ///
    /// ```text
    /// /org/freedesktop/UDisks2/block_devices/sdb1
    ///   Holders
    ///     Names aay dm-3,md127
    /// ```
    ///
    /// Types: `s`, `b`, `t`/`u`, `o`, `ay` (text), `aay` (text, comma
    /// separated). Blank lines and lines starting with `#` are skipped.
    pub fn parse(text: &str) -> Result<Objects, String> {
        let mut objects = Objects::default();
        let (mut object, mut iface) = (None::<String>, None::<String>);
        for (n, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            let bad = || format!("line {}: {line}", n + 1);
            match indent {
                0 => {
                    objects.0.entry(trimmed.to_string()).or_default();
                    object = Some(trimmed.to_string());
                    iface = None;
                }
                2 => {
                    let o = object.as_ref().ok_or_else(bad)?;
                    let name = trimmed.strip_prefix(IFACE).unwrap_or(trimmed).to_string();
                    objects
                        .0
                        .entry(o.clone())
                        .or_default()
                        .entry(name.clone())
                        .or_default();
                    iface = Some(name);
                }
                _ => {
                    let (o, i) = (
                        object.as_ref().ok_or_else(bad)?,
                        iface.as_ref().ok_or_else(bad)?,
                    );
                    let mut fields = trimmed.splitn(3, ' ');
                    let (Some(prop), Some(ty)) = (fields.next(), fields.next()) else {
                        return Err(bad());
                    };
                    let raw = fields.next().unwrap_or("");
                    let value = match ty {
                        "s" => Value::Str(raw.to_string()),
                        "b" => Value::Bool(raw == "true"),
                        "t" | "u" => Value::Num(raw.parse().map_err(|_| bad())?),
                        "o" => Value::Path(raw.to_string()),
                        "ay" => Value::Bytes(nul(raw)),
                        "aay" => Value::ByteLists(
                            raw.split(',').filter(|s| !s.is_empty()).map(nul).collect(),
                        ),
                        _ => return Err(bad()),
                    };
                    objects
                        .0
                        .entry(o.clone())
                        .or_default()
                        .entry(i.clone())
                        .or_default()
                        .insert(prop.to_string(), value);
                }
            }
        }
        Ok(objects)
    }
}

/// A path as udisks2 sends it: its bytes and a NUL.
fn nul(s: &str) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b.push(0);
    b
}

/// Strips the interfaces' common prefix: what [`Objects`] keys
/// interfaces by.
pub fn short(iface: &str) -> &str {
    iface.strip_prefix(IFACE).unwrap_or(iface)
}
