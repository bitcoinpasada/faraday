//! Windows' answers about disks and volumes, read as the hub's disks
//! (`docs/HUB.md` §4.2, §4.3), with no Win32 call in it: [`Snapshot`]
//! holds the raw bytes each IOCTL answered and what the volume calls
//! said, as `sys` reads them on Windows or as a test records them
//! ([`Snapshot::parse`], `tests/windows/*.txt`).
//!
//! A disk is `\\.\PhysicalDriveN`: its storage descriptor
//! (`IOCTL_STORAGE_QUERY_PROPERTY`: bus type, vendor, product, serial),
//! its geometry (`IOCTL_DISK_GET_DRIVE_GEOMETRY_EX`: size) and its
//! partitions (`IOCTL_DISK_GET_DRIVE_LAYOUT_EX`: offsets and GPT names).
//! All three answer a handle opened with no access, so no elevation.
//! A volume is where it lies (`IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS`),
//! its label and file system, its drive letters and folders, and its NT
//! device. The disk that holds the Windows folder, the system partition
//! (`HKLM\SYSTEM\Setup\SystemPartition`) or a page file
//! (`ExistingPageFiles`) is the computer's own, and is refused.

use std::fmt::Write as _;

use faraday_hub::api::{Bus, Disk, FaradayStick, InUse, Volume};

use crate::writer::Identity;

/// `STORAGE_BUS_TYPE`: USB.
pub const BUS_USB: u32 = 7;
/// `STORAGE_BUS_TYPE`: SD.
pub const BUS_SD: u32 = 12;
/// `STORAGE_BUS_TYPE`: MMC.
pub const BUS_MMC: u32 = 13;

/// `PARTITION_STYLE`.
const STYLE_MBR: u32 = 0;
const STYLE_GPT: u32 = 1;

/// `DRIVE_LAYOUT_INFORMATION_EX`'s fixed part, and each
/// `PARTITION_INFORMATION_EX` after it (x86_64).
const LAYOUT_HEAD: usize = 48;
const PARTITION_SIZE: usize = 144;
/// `VOLUME_DISK_EXTENTS`'s count (padded) and each `DISK_EXTENT`.
const EXTENTS_HEAD: usize = 8;
const EXTENT_SIZE: usize = 24;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The NUL-terminated ASCII string at `offset` in `b`, trimmed; empty
/// for offset 0 or one past the end.
fn c_string(b: &[u8], offset: u32) -> String {
    let at = offset as usize;
    if at == 0 || at >= b.len() {
        return String::new();
    }
    let rest = &b[at..];
    let end = rest.iter().position(|&c| c == 0).unwrap_or(rest.len());
    rest[..end]
        .iter()
        .map(|&c| {
            if c.is_ascii_graphic() || c == b' ' {
                c as char
            } else {
                '?'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// A `STORAGE_DEVICE_DESCRIPTOR`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Descriptor {
    /// `BusType`.
    pub bus: u32,
    /// `RemovableMedia`.
    pub removable: bool,
    /// The vendor string.
    pub vendor: String,
    /// The product string.
    pub product: String,
    /// The serial number string.
    pub serial: String,
}

/// Reads a `STORAGE_DEVICE_DESCRIPTOR`: `Version`, `Size`, `DeviceType`,
/// `DeviceTypeModifier`, `RemovableMedia`, `CommandQueueing`, then the
/// vendor, product, revision and serial offsets and `BusType`.
pub fn descriptor(b: &[u8]) -> Option<Descriptor> {
    let size = u32_at(b, 4)? as usize;
    let b = b.get(..size.min(b.len()))?;
    Some(Descriptor {
        removable: *b.get(10)? != 0,
        vendor: c_string(b, u32_at(b, 12)?),
        product: c_string(b, u32_at(b, 16)?),
        serial: c_string(b, u32_at(b, 24)?),
        bus: u32_at(b, 28)?,
    })
}

/// A `DISK_GEOMETRY_EX`, as far as the hub reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// `DiskSize`.
    pub size: u64,
    /// `Geometry.BytesPerSector`.
    pub sector: u32,
}

/// Reads a `DISK_GEOMETRY_EX`: the `DISK_GEOMETRY` (24 bytes, its
/// `BytesPerSector` at 20), then `DiskSize`.
pub fn geometry(b: &[u8]) -> Option<Geometry> {
    Some(Geometry {
        sector: u32_at(b, 20)?,
        size: u64_at(b, 24)?,
    })
}

/// One `DISK_EXTENT`: where part of a volume lies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Extent {
    /// The disk's number (`PhysicalDriveN`).
    pub disk: u32,
    /// Its first byte on the disk.
    pub offset: u64,
    /// Its length.
    pub length: u64,
}

/// Reads a `VOLUME_DISK_EXTENTS`.
pub fn extents(b: &[u8]) -> Option<Vec<Extent>> {
    let n = u32_at(b, 0)? as usize;
    if b.len() < EXTENTS_HEAD + n.checked_mul(EXTENT_SIZE)? {
        return None;
    }
    (0..n)
        .map(|i| {
            let at = EXTENTS_HEAD + i * EXTENT_SIZE;
            Some(Extent {
                disk: u32_at(b, at)?,
                offset: u64_at(b, at + 8)?,
                length: u64_at(b, at + 16)?,
            })
        })
        .collect()
}

/// A disk's partition table style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// MBR.
    Mbr,
    /// GPT.
    Gpt,
    /// No partition table.
    Raw,
}

/// One partition, from `PARTITION_INFORMATION_EX`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    /// `PartitionNumber`.
    pub number: u32,
    /// `StartingOffset`.
    pub offset: u64,
    /// `PartitionLength`.
    pub length: u64,
    /// The GPT name; empty on MBR.
    pub name: String,
}

/// A disk's partitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The table's style.
    pub style: Style,
    /// Its partitions in their order on the disk; MBR's unused and
    /// extended entries left out.
    pub partitions: Vec<Partition>,
}

/// Reads a `DRIVE_LAYOUT_INFORMATION_EX`: `PartitionStyle`,
/// `PartitionCount`, the MBR or GPT header (padded to 40 bytes), then
/// each `PARTITION_INFORMATION_EX`: style, `StartingOffset`,
/// `PartitionLength`, `PartitionNumber`, two flags, then at 32 the MBR
/// part (`PartitionType` first) or the GPT part (type and id GUIDs,
/// attributes, and at 72 the name, 36 UTF-16 units).
pub fn layout(b: &[u8]) -> Option<Layout> {
    let style = match u32_at(b, 0)? {
        STYLE_MBR => Style::Mbr,
        STYLE_GPT => Style::Gpt,
        _ => Style::Raw,
    };
    let n = u32_at(b, 4)? as usize;
    if b.len() < LAYOUT_HEAD + n.checked_mul(PARTITION_SIZE)? {
        return None;
    }
    let mut partitions = Vec::new();
    for i in 0..n {
        let p = &b[LAYOUT_HEAD + i * PARTITION_SIZE..LAYOUT_HEAD + (i + 1) * PARTITION_SIZE];
        let offset = u64_at(p, 8)?;
        let length = u64_at(p, 16)?;
        let number = u32_at(p, 24)?;
        let name = match style {
            Style::Gpt => {
                let units: Vec<u16> = p[72..144]
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .take_while(|&u| u != 0)
                    .collect();
                String::from_utf16_lossy(&units)
            }
            _ => {
                // 0: unused; 0x05 and 0x0F: an extended partition's
                // container, whose logical partitions are listed too.
                if matches!(p[32], 0x00 | 0x05 | 0x0f) || length == 0 {
                    continue;
                }
                String::new()
            }
        };
        partitions.push(Partition {
            number,
            offset,
            length,
            name,
        });
    }
    partitions.sort_by_key(|p| p.offset);
    Some(Layout { style, partitions })
}

/// One `\\.\PhysicalDriveN` as it answered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawDisk {
    /// N.
    pub number: u32,
    /// `IOCTL_STORAGE_QUERY_PROPERTY`'s answer.
    pub descriptor: Vec<u8>,
    /// `IOCTL_DISK_GET_DRIVE_GEOMETRY_EX`'s answer; empty when it
    /// failed (a card reader with no card).
    pub geometry: Vec<u8>,
    /// `IOCTL_DISK_GET_DRIVE_LAYOUT_EX`'s answer; empty when it failed.
    pub layout: Vec<u8>,
}

/// One volume (`\\?\Volume{…}\`) as it answered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawVolume {
    /// Its name, `\\?\Volume{…}\`.
    pub name: String,
    /// Its NT device (`\Device\HarddiskVolume3`), from `QueryDosDeviceW`.
    pub device: String,
    /// `IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS`'s answer.
    pub extents: Vec<u8>,
    /// Its label (`GetVolumeInformationW`); empty for none.
    pub label: String,
    /// Its file system's name; empty when Windows mounted none.
    pub filesystem: String,
    /// Its drive letters and folders (`E:\`), from
    /// `GetVolumePathNamesForVolumeNameW`.
    pub paths: Vec<String>,
}

/// Everything read about the disks at one moment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// The Windows folder (`GetSystemWindowsDirectoryW`, `C:\Windows`).
    pub windows: String,
    /// The system partition's NT device, from the registry.
    pub system_partition: String,
    /// The page files, as the registry lists them (`\??\C:\pagefile.sys`).
    pub page_files: Vec<String>,
    /// The disks.
    pub disks: Vec<RawDisk>,
    /// The volumes.
    pub volumes: Vec<RawVolume>,
}

/// A disk as read, and the volume its version is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The disk, its `faraday` version not yet read.
    pub disk: Disk,
    /// A Faraday stick's boot volume (`\\?\Volume{…}\`), when Windows
    /// mounted its file system.
    pub boot: Option<String>,
}

/// The id the hub keeps for disk `n`, and the path the writer opens.
pub fn disk_id(n: u32) -> String {
    format!(r"\\.\PhysicalDrive{n}")
}

/// What a person reads as disk `n`: Disk Management's name for it.
pub fn disk_device(n: u32) -> String {
    format!("Disk {n}")
}

/// Disk `n` from its id, when the id is `\\.\PhysicalDriveN`.
pub fn disk_number(id: &str) -> Option<u32> {
    let digits = id.strip_prefix(r"\\.\PhysicalDrive")?;
    let ok = !digits.is_empty()
        && digits.len() <= 3
        && digits.bytes().all(|b| b.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'));
    ok.then(|| digits.parse().ok()).flatten()
}

/// The page file's path from a registry entry: `\??\C:\pagefile.sys`
/// is `C:\pagefile.sys`.
pub fn page_file_path(entry: &str) -> &str {
    entry.strip_prefix(r"\??\").unwrap_or(entry)
}

/// Whether `path` is at or under the folder `mount` (`C:\`), ignoring
/// case as Windows does.
fn under(path: &str, mount: &str) -> bool {
    let (p, m) = (path.to_ascii_lowercase(), mount.to_ascii_lowercase());
    let m = m.trim_end_matches('\\');
    !m.is_empty() && (p == m || p.starts_with(&format!("{m}\\")))
}

impl RawVolume {
    fn extents(&self) -> Vec<Extent> {
        extents(&self.extents).unwrap_or_default()
    }
}

impl Snapshot {
    /// The volume `path` lies on: the one with the longest mount point
    /// it is under.
    fn volume_of(&self, path: &str) -> Option<&RawVolume> {
        self.volumes
            .iter()
            .flat_map(|v| v.paths.iter().map(move |m| (m, v)))
            .filter(|(m, _)| under(path, m))
            .max_by_key(|(m, _)| m.len())
            .map(|(_, v)| v)
    }

    /// The disks the computer runs from: those holding the Windows
    /// folder's volume, the system partition or a page file. `None`
    /// when the Windows folder's volume cannot be found, so that no disk
    /// can be told apart from the system's.
    pub fn system_disks(&self) -> Option<Vec<u32>> {
        let windows = self.volume_of(&self.windows)?;
        let mut used: Vec<&RawVolume> = vec![windows];
        used.extend(self.volumes.iter().filter(|v| {
            !self.system_partition.is_empty()
                && v.device.eq_ignore_ascii_case(&self.system_partition)
        }));
        used.extend(
            self.page_files
                .iter()
                .filter_map(|p| self.volume_of(page_file_path(p))),
        );
        let mut disks: Vec<u32> = used
            .iter()
            .flat_map(|v| v.extents())
            .map(|e| e.disk)
            .collect();
        disks.sort_unstable();
        disks.dedup();
        Some(disks)
    }

    /// The volumes with any part on disk `n`.
    pub fn volumes_on(&self, n: u32) -> Vec<&RawVolume> {
        self.volumes
            .iter()
            .filter(|v| v.extents().iter().any(|e| e.disk == n))
            .collect()
    }

    /// Every disk attached, as the hub lists them: each with a storage
    /// descriptor and a size (a card reader with no card has none).
    pub fn disks(&self) -> Vec<Found> {
        let system = self.system_disks();
        let mut out = Vec::new();
        for raw in &self.disks {
            let Some(desc) = descriptor(&raw.descriptor) else {
                continue;
            };
            let Some(geo) = geometry(&raw.geometry).filter(|g| g.size > 0) else {
                continue;
            };
            let n = raw.number;
            let bus = match desc.bus {
                BUS_USB => Bus::Usb,
                BUS_SD | BUS_MMC => Bus::SdReader,
                _ => Bus::Other,
            };
            let on_disk = self.volumes_on(n);
            let at = |offset: u64| {
                on_disk.iter().copied().find(|v| {
                    v.extents()
                        .iter()
                        .any(|e| e.disk == n && e.offset == offset)
                })
            };
            let volume = |v: Option<&RawVolume>| Volume {
                label: v.map(|v| v.label.clone()).filter(|l| !l.is_empty()),
                files: v.is_some_and(|v| !v.filesystem.is_empty()),
            };
            let table = layout(&raw.layout).filter(|l| l.style != Style::Raw);
            let parts = table
                .as_ref()
                .map(|l| l.partitions.clone())
                .unwrap_or_default();
            let volumes: Vec<Volume> = if parts.is_empty() {
                // A file system on the whole disk, with no partition table.
                on_disk.iter().map(|v| volume(Some(v))).collect()
            } else {
                parts.iter().map(|p| volume(at(p.offset))).collect()
            };
            let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
            let faraday = table.as_ref().is_some_and(|l| l.style == Style::Gpt)
                && names == ["OSKBOOT", "OSKDATA"];
            let boot = faraday
                .then(|| at(parts[0].offset))
                .flatten()
                .filter(|v| !v.filesystem.is_empty())
                .map(|v| v.name.clone());
            let in_use = match &system {
                Some(disks) if !disks.contains(&n) => None,
                _ => Some(InUse::System),
            };
            out.push(Found {
                disk: Disk {
                    id: disk_id(n),
                    device: disk_device(n),
                    vendor: desc.vendor,
                    model: desc.product,
                    serial: desc.serial,
                    size: geo.size,
                    bus,
                    volumes,
                    in_use,
                    faraday: faraday.then_some(FaradayStick { release: None }),
                },
                boot,
            });
        }
        out
    }

    /// Disk `n` as it is now, for the writer's check that it is still
    /// the disk confirmed, with how it is attached.
    pub fn identity(&self, n: u32) -> Option<(Identity, Bus)> {
        let found = self.disks().into_iter().find(|f| f.disk.id == disk_id(n))?;
        let d = found.disk;
        Some((
            Identity {
                device: d.device,
                serial: d.serial,
                size: d.size,
                in_use: d.in_use,
            },
            d.bus,
        ))
    }

    /// Reads a recording: what `faraday-hub --record-disks` prints on
    /// Windows, as `tests/windows/*.txt` hold it:
    ///
    /// ```text
    /// windows C:\Windows
    /// system-partition \Device\HarddiskVolume1
    /// page-file \??\C:\pagefile.sys
    /// disk 1
    ///   descriptor 0100000000040000…
    ///   geometry …
    ///   layout …
    /// volume \\?\Volume{…}\
    ///   device \Device\HarddiskVolume7
    ///   extents …
    ///   label OSKBOOT
    ///   filesystem FAT32
    ///   path E:\
    /// ```
    ///
    /// Bytes are hex. Blank lines and lines starting with `#` are
    /// skipped.
    pub fn parse(text: &str) -> Result<Snapshot, String> {
        enum At {
            Top,
            Disk,
            Volume,
        }
        let mut s = Snapshot::default();
        let mut at = At::Top;
        for (n, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let bad = || format!("line {}: {line}", n + 1);
            let (key, value) = trimmed.split_once(' ').unwrap_or((trimmed, ""));
            let indented = line.starts_with(' ');
            match (indented, &at, key) {
                (false, _, "windows") => s.windows = value.to_string(),
                (false, _, "system-partition") => s.system_partition = value.to_string(),
                (false, _, "page-file") => s.page_files.push(value.to_string()),
                (false, _, "disk") => {
                    s.disks.push(RawDisk {
                        number: value.parse().map_err(|_| bad())?,
                        ..RawDisk::default()
                    });
                    at = At::Disk;
                }
                (false, _, "volume") => {
                    s.volumes.push(RawVolume {
                        name: value.to_string(),
                        ..RawVolume::default()
                    });
                    at = At::Volume;
                }
                (true, At::Disk, _) => {
                    let d = s.disks.last_mut().ok_or_else(bad)?;
                    let bytes = unhex(value).ok_or_else(bad)?;
                    match key {
                        "descriptor" => d.descriptor = bytes,
                        "geometry" => d.geometry = bytes,
                        "layout" => d.layout = bytes,
                        _ => return Err(bad()),
                    }
                }
                (true, At::Volume, _) => {
                    let v = s.volumes.last_mut().ok_or_else(bad)?;
                    match key {
                        "device" => v.device = value.to_string(),
                        "extents" => v.extents = unhex(value).ok_or_else(bad)?,
                        "label" => v.label = value.to_string(),
                        "filesystem" => v.filesystem = value.to_string(),
                        "path" => v.paths.push(value.to_string()),
                        _ => return Err(bad()),
                    }
                }
                _ => return Err(bad()),
            }
        }
        Ok(s)
    }

    /// Writes the recording [`Snapshot::parse`] reads.
    pub fn record(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "windows {}", self.windows);
        let _ = writeln!(out, "system-partition {}", self.system_partition);
        for p in &self.page_files {
            let _ = writeln!(out, "page-file {p}");
        }
        for d in &self.disks {
            let _ = writeln!(out, "disk {}", d.number);
            for (k, b) in [
                ("descriptor", &d.descriptor),
                ("geometry", &d.geometry),
                ("layout", &d.layout),
            ] {
                if !b.is_empty() {
                    let _ = writeln!(out, "  {k} {}", hex(b));
                }
            }
        }
        for v in &self.volumes {
            let _ = writeln!(out, "volume {}", v.name);
            let _ = writeln!(out, "  device {}", v.device);
            if !v.extents.is_empty() {
                let _ = writeln!(out, "  extents {}", hex(&v.extents));
            }
            if !v.label.is_empty() {
                let _ = writeln!(out, "  label {}", v.label);
            }
            if !v.filesystem.is_empty() {
                let _ = writeln!(out, "  filesystem {}", v.filesystem);
            }
            for p in &v.paths {
                let _ = writeln!(out, "  path {p}");
            }
        }
        out
    }
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .fold(String::with_capacity(b.len() * 2), |mut s, x| {
            let _ = write!(s, "{x:02x}");
            s
        })
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}
