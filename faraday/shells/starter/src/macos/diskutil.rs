//! `diskutil`'s answers read as the Starter's disks (`docs/STARTER.md` §4.2,
//! `docs/MAC.md` §6), with no macOS call in it: [`Answers`] holds what
//! `diskutil list -plist` and `diskutil info -plist <disk>` printed, as
//! the macOS code reads them or as a test writes them
//! (`tests/macos/*.plist`).
//!
//! The layouts read, with where each was taken from (no answer here was
//! recorded on a Mac for this repository yet; batch A3c records them):
//!
//! - `list`: `AllDisksAndPartitions`, one dictionary per whole disk or
//!   APFS container, with `DeviceIdentifier`, `Partitions` (each with
//!   `DeviceIdentifier`, `VolumeName`, `MountPoint`), `APFSVolumes`
//!   (each with `MountPoint` and `MountedSnapshots`, whose
//!   `SnapshotMountPoint` is where the sealed system snapshot is
//!   mounted, `/`), `APFSPhysicalStores` (each with `DeviceIdentifier`)
//!   and, for a file system on the whole disk, `MountPoint`;
//!   `WholeDisks`. From the output recorded in Asahi Linux's installer,
//!   issue 141 (an Apple Silicon Mac's internal disk), and in
//!   hst-imager's tests (`TestData/diskutil/diskutil-all-usb-stick.plist`,
//!   an Intel Mac with a USB stick).
//! - `info`: `WholeDisk`, `DeviceNode`, `BusProtocol` (`USB`, `PCI-Express`,
//!   `Apple Fabric`, `Disk Image`; `Secure Digital` is IOKit's
//!   `kIOPropertyPhysicalInterconnectTypeSecureDigital`),
//!   `VirtualOrPhysical` (`Physical`, `Virtual`, and `Unknown` on a
//!   physical disk), `MediaName`, `IORegistryEntryName` (on a
//!   partition, its GPT name: Apple's `iBootSystemContainer`,
//!   `Container`, `RecoveryOSContainer` in Asahi's recording),
//!   `TotalSize` and `Size` (an integer, or a real), `DeviceBlockSize`,
//!   `FilesystemType`, `MountPoint`. From the same two sources, and
//!   hst-imager's `diskutil-info-disk2-usb-stick.plist`,
//!   `diskutil-info-disk0.plist`; Home Assistant's installer's fixtures
//!   README for `Unknown`.
//!
//! A disk is listed only when every key the rule needs is there and
//! reads as expected, and `diskutil info` answered for each of its
//! partitions: a whole, physical disk (never a disk image, never an
//! APFS container) on USB or in the computer's own SD reader. It is the computer's own
//! ([`InUse::System`]) when any volume on it, or in an APFS container
//! whose physical store is on it, is mounted at `/` or under
//! `/System/Volumes/`; in use ([`InUse::Mounted`]) when one is mounted
//! anywhere else than under `/Volumes/`, where macOS mounts what is
//! plugged in. The serial is empty: `diskutil` has none (A3c reads it
//! from the I/O Registry on a Mac).

use std::collections::BTreeMap;
use std::path::PathBuf;

use faraday_starter::api::{Bus, Disk, FaradayStick, InUse, Volume};

use super::plist::{self, Value};
use crate::writer::Identity;

/// Where macOS mounts what is plugged in: a volume mounted anywhere else
/// is one the computer uses (§4.2).
const DESKTOP_MOUNTS: &str = "/Volumes/";

/// Where the running system's volumes are mounted, besides `/`.
const SYSTEM_MOUNTS: &str = "/System/Volumes/";

/// What `diskutil` answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Answers {
    /// `diskutil list -plist`.
    pub list: Value,
    /// `diskutil info -plist <id>`, by device identifier (`disk4`,
    /// `disk4s1`).
    pub info: BTreeMap<String, Value>,
}

/// A Faraday stick's boot partition, to read its version from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootPart {
    /// Its device identifier (`disk4s1`), to mount read-only.
    pub id: String,
    /// Where macOS mounted it, when it did.
    pub mounted: Option<PathBuf>,
}

/// A disk as read, and what reading its version and writing it need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The disk, its `faraday` version not yet read.
    pub disk: Disk,
    /// A Faraday stick's boot partition.
    pub boot: Option<BootPart>,
    /// The size of its blocks (`DeviceBlockSize`).
    pub block: u64,
}

/// The whole disk an identifier is on: `disk4` for `disk4s1` and
/// `disk4s1s1`; `None` for one that is not `disk<N>…`.
pub fn whole_of(id: &str) -> Option<&str> {
    let digits = id.strip_prefix("disk")?;
    let n = digits.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
        return None;
    }
    let rest = &digits[n..];
    (rest.is_empty() || rest.starts_with('s')).then(|| &id[..4 + n])
}

fn str_of<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str()
}

fn items<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).unwrap_or(&[])
}

impl Answers {
    /// Reads `diskutil list -plist`; the answers of `diskutil info` are
    /// added with [`Answers::add_info`].
    pub fn from_list(text: &str) -> Result<Answers, String> {
        let list = plist::parse(text)?;
        if list
            .get("AllDisksAndPartitions")
            .and_then(Value::as_array)
            .is_none()
            || list.get("WholeDisks").and_then(Value::as_array).is_none()
        {
            return Err(String::from(
                "diskutil list: no AllDisksAndPartitions or WholeDisks",
            ));
        }
        Ok(Answers {
            list,
            info: BTreeMap::new(),
        })
    }

    /// Adds what `diskutil info -plist <id>` answered. An answer about
    /// another identifier than the one asked is refused.
    pub fn add_info(&mut self, id: &str, text: &str) -> Result<(), String> {
        let info = plist::parse(text)?;
        if str_of(&info, "DeviceIdentifier") != Some(id) {
            return Err(format!("diskutil info {id}: an answer about another disk"));
        }
        self.info.insert(id.to_string(), info);
        Ok(())
    }

    /// The entry `diskutil list` gave the whole disk `id`, to tell
    /// whether it changed since its `info` was read.
    pub fn listed(&self, id: &str) -> Option<&Value> {
        self.entry(id)
    }

    /// The whole disks, in `diskutil`'s order.
    pub fn whole_disks(&self) -> Vec<&str> {
        items(&self.list, "WholeDisks")
            .iter()
            .filter_map(Value::as_str)
            .collect()
    }

    /// The entry of `AllDisksAndPartitions` for `id`.
    fn entry(&self, id: &str) -> Option<&Value> {
        items(&self.list, "AllDisksAndPartitions")
            .iter()
            .find(|e| str_of(e, "DeviceIdentifier") == Some(id))
    }

    /// The partitions of the whole disk `id`, in `diskutil`'s order.
    pub fn partitions(&self, id: &str) -> Vec<&str> {
        self.entry(id)
            .map(|e| {
                items(e, "Partitions")
                    .iter()
                    .filter_map(|p| str_of(p, "DeviceIdentifier"))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every identifier whose `diskutil info` the disks need: each whole
    /// disk and its partitions.
    pub fn wanted(&self) -> Vec<String> {
        let mut out = Vec::new();
        for d in self.whole_disks() {
            out.push(d.to_string());
            out.extend(self.partitions(d).into_iter().map(str::to_string));
        }
        out
    }

    /// Every mount point on the whole disk `id`: its own file system,
    /// its partitions', and every volume and snapshot of an APFS
    /// container whose physical store is on it (or the disk itself).
    fn mounts(&self, id: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut add = |v: &Value| {
            if let Some(m) = str_of(v, "MountPoint").filter(|m| !m.is_empty()) {
                out.push(m.to_string());
            }
            for s in items(v, "MountedSnapshots") {
                if let Some(m) = str_of(s, "SnapshotMountPoint").filter(|m| !m.is_empty()) {
                    out.push(m.to_string());
                }
            }
        };
        if let Some(e) = self.entry(id) {
            add(e);
            items(e, "Partitions").iter().for_each(&mut add);
            items(e, "APFSVolumes").iter().for_each(&mut add);
        }
        for p in std::iter::once(id).chain(self.partitions(id)) {
            if let Some(info) = self.info.get(p) {
                add(info);
            }
        }
        for e in items(&self.list, "AllDisksAndPartitions") {
            let on_it = items(e, "APFSPhysicalStores")
                .iter()
                .filter_map(|s| str_of(s, "DeviceIdentifier"))
                .any(|s| whole_of(s) == Some(id));
            if on_it {
                add(e);
                items(e, "APFSVolumes").iter().for_each(&mut add);
            }
        }
        out
    }

    /// Why the computer uses the whole disk `id`, when it does.
    pub fn in_use(&self, id: &str) -> Option<InUse> {
        let mounts = self.mounts(id);
        if mounts
            .iter()
            .any(|m| m == "/" || m.starts_with(SYSTEM_MOUNTS))
        {
            return Some(InUse::System);
        }
        mounts
            .into_iter()
            .find(|m| !m.starts_with(DESKTOP_MOUNTS))
            .map(InUse::Mounted)
    }

    /// The disk `id` as listed, or `None` when it is not offered at all
    /// (not a whole physical disk on USB or the SD reader, or an answer
    /// the rule needs is missing).
    pub fn found(&self, id: &str) -> Option<Found> {
        let info = self.info.get(id)?;
        if !info.get("WholeDisk")?.as_bool()? {
            return None;
        }
        // An APFS container is a disk macOS makes from a partition: never
        // one to write, whatever it says of itself.
        if info.get("APFSPhysicalStores").is_some() || info.get("APFSContainerReference").is_some()
        {
            return None;
        }
        if str_of(info, "DeviceIdentifier")? != id || whole_of(id) != Some(id) {
            return None;
        }
        let device = str_of(info, "DeviceNode")?;
        if device != format!("/dev/{id}") {
            return None;
        }
        if str_of(info, "VirtualOrPhysical")? == "Virtual" {
            return None;
        }
        let bus = match str_of(info, "BusProtocol")? {
            "USB" => Bus::Usb,
            "Secure Digital" => Bus::SdReader,
            _ => return None,
        };
        let size = info
            .get("TotalSize")
            .or_else(|| info.get("Size"))?
            .as_u64()
            .filter(|s| *s > 0)?;
        let block = info
            .get("DeviceBlockSize")?
            .as_u64()
            .filter(|b| b.is_power_of_two() && *b >= 512)?;
        let entry = self.entry(id)?;
        if entry.get("APFSPhysicalStores").is_some() {
            return None;
        }
        let parts = self.partitions(id);
        // Each partition's answer too: one missing could hide where it
        // is mounted.
        if parts.iter().any(|p| !self.info.contains_key(*p)) {
            return None;
        }
        let stores: Vec<&str> = items(&self.list, "AllDisksAndPartitions")
            .iter()
            .flat_map(|e| items(e, "APFSPhysicalStores"))
            .filter_map(|s| str_of(s, "DeviceIdentifier"))
            .collect();
        let volume = |listed: &Value, id: &str| {
            let label = str_of(listed, "VolumeName").unwrap_or("").trim();
            let fs = self
                .info
                .get(id)
                .and_then(|i| str_of(i, "FilesystemType"))
                .unwrap_or("");
            Volume {
                label: (!label.is_empty()).then(|| label.to_string()),
                files: !fs.is_empty() || stores.contains(&id),
            }
        };
        let volumes: Vec<Volume> = if parts.is_empty() {
            // A file system on the whole disk, with no partition table.
            let v = volume(entry, id);
            let mounted = str_of(entry, "MountPoint").is_some_and(|m| !m.is_empty());
            if v.files || v.label.is_some() || mounted {
                vec![Volume { files: true, ..v }]
            } else {
                Vec::new()
            }
        } else {
            items(entry, "Partitions")
                .iter()
                .filter_map(|p| Some(volume(p, str_of(p, "DeviceIdentifier")?)))
                .collect()
        };
        // A Faraday stick, as Linux finds one: two partitions named
        // OSKBOOT and OSKDATA (their GPT names, which IOKit gives the
        // partitions' media).
        let names: Vec<&str> = parts
            .iter()
            .map(|p| {
                self.info
                    .get(*p)
                    .and_then(|i| str_of(i, "IORegistryEntryName"))
                    .unwrap_or("")
            })
            .collect();
        let boot = (names == ["OSKBOOT", "OSKDATA"]).then(|| BootPart {
            id: parts[0].to_string(),
            mounted: items(entry, "Partitions")
                .first()
                .and_then(|p| str_of(p, "MountPoint"))
                .filter(|m| !m.is_empty())
                .map(PathBuf::from),
        });
        let media = str_of(info, "MediaName").unwrap_or("").trim();
        Some(Found {
            disk: Disk {
                id: id.to_string(),
                device: device.to_string(),
                vendor: vendor(str_of(info, "IORegistryEntryName").unwrap_or(""), media),
                model: media.to_string(),
                serial: String::new(),
                size,
                bus,
                volumes,
                in_use: self.in_use(id),
                faraday: boot.as_ref().map(|_| FaradayStick {
                    release: None,
                    read: true,
                }),
            },
            boot,
            block,
        })
    }

    /// Every disk offered, in `diskutil`'s order.
    pub fn disks(&self) -> Vec<Found> {
        self.whole_disks()
            .into_iter()
            .filter_map(|d| self.found(d))
            .collect()
    }

    /// The disk `id` as it is now, for the writer's check that it is
    /// still the disk confirmed; `None` when it is no longer offered.
    pub fn identity(&self, id: &str) -> Option<(Identity, u64)> {
        let f = self.found(id)?;
        Some((
            Identity {
                device: f.disk.device,
                serial: f.disk.serial,
                size: f.disk.size,
                in_use: f.disk.in_use,
            },
            f.block,
        ))
    }
}

/// The maker, when IOKit's name for the disk's media is the maker, the
/// model and `Media` (`SanDisk' Cruzer Fit Media` for `Cruzer Fit`);
/// empty otherwise.
fn vendor(entry: &str, media: &str) -> String {
    let name = entry.trim().strip_suffix(" Media").unwrap_or(entry).trim();
    match name.strip_suffix(media) {
        Some(maker) if !media.is_empty() => maker.trim().to_string(),
        _ => String::new(),
    }
}
