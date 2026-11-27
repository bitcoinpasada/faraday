//! The FAT32 formatter: what the Starter writes to a stick's data
//! partition before any file is put there (`docs/STICK-IDENTITY.md`
//! §4.2). It makes the volume `mkfs.fat -F 32` makes for the same size,
//! label and serial (dosfstools 4.2: the same cluster size, reserved
//! sectors, FAT length, geometry fields, FSInfo and backup boot sector),
//! with the two values Windows leaves at shutdown
//! (`docs/STICK-IDENTITY.md` §4.1): FAT[1] is `0xFFFFFFFF` and FSInfo's
//! next-free hint is cluster 3, the first free one on an empty volume.
//!
//! Nothing here touches a disk or allocates for the volume's size:
//! [`format32`] works the layout out from three numbers, and
//! [`Format32::regions`] lists the few places that hold anything but
//! zeros (the boot sector and its backup, FSInfo and its copy, the start
//! of each FAT, the root directory's cluster), with the runs of zeros
//! between them as lengths. A caller writes each to its partition.

use alloc::vec;
use alloc::vec::Vec;

use crate::{Disk, Error};

/// The bytes in a sector: the only size the formatter makes.
const SECTOR: usize = 512;

/// The reserved sectors of a FAT32 volume, as `mkfs.fat` makes them
/// (before alignment to the cluster size).
const RESERVED: u32 = 32;

/// The FSInfo sector's place, and the backup boot sector's, as `mkfs.fat`
/// places them; FSInfo's copy follows the backup.
const INFO_SECTOR: u32 = 1;
const BACKUP_SECTOR: u32 = 6;

/// The fewest clusters a FAT32 volume has: the specification's line
/// between FAT16 and FAT32, below which `mkfs.fat` warns.
pub const MIN_CLUSTERS: u32 = 65525;

/// The label of a volume that has none.
const NO_NAME: [u8; 11] = *b"NO NAME    ";

/// The date `mkfs.fat` gives a label entry when it has no time: 1 January
/// 1980, which a formatter that must give the same bytes every time uses.
const DATE_1980: u16 = 1 + (1 << 5);

/// The boot sector's code: halt. The volume is not a boot volume.
const HALT: [u8; 4] = [0xFA, 0xF4, 0xEB, 0xFD];

/// Why a volume cannot be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatError {
    /// Fewer than [`MIN_CLUSTERS`] clusters fit: the partition is too
    /// small for FAT32.
    TooSmall,
    /// More clusters than FAT32 numbers (not reached below 2 TiB).
    TooLarge,
}

/// A place in the volume to write: its byte offset from the volume's
/// start, and what goes there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Region {
    /// These bytes.
    Bytes {
        /// The offset in the volume.
        at: u64,
        /// What to write.
        data: Vec<u8>,
    },
    /// `len` zero bytes, given as a length so that a FAT of a hundred
    /// megabytes is not held.
    Zeros {
        /// The offset in the volume.
        at: u64,
        /// How many bytes.
        len: u64,
    },
}

/// A FAT32 volume's layout, worked out: no files, the root directory
/// empty but for a label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format32 {
    serial: u32,
    hidden: u32,
    label: [u8; 11],
    /// Sectors the volume takes (a multiple of the geometry's sectors
    /// per track, as `mkfs.fat` makes it, and at most `0xFFFFFFFF`).
    total: u32,
    cluster_sectors: u32,
    reserved: u32,
    fat_sectors: u32,
    clusters: u32,
    heads: u16,
    track: u16,
}

/// Works out a FAT32 volume for a partition of `sectors` sectors of 512
/// bytes whose first sector on the disk is `hidden` (the BPB's hidden
/// sectors, as `mkfs.fat` writes for a partition), with this volume serial and label (`None` is no label, as
/// `label_bytes` gives it).
///
/// The cluster size is Microsoft's table as `mkfs.fat -F 32` follows it:
/// 512 bytes up to 260 MiB, 4 KiB to 8 GiB, 8 KiB to 16 GiB, 16 KiB to
/// 32 GiB, 32 KiB above. The volume takes the partition rounded down to
/// whole tracks and to 2^32 - 1 sectors at most, as `mkfs.fat` does; the
/// rest is left alone. Fewer than [`MIN_CLUSTERS`] clusters is refused
/// (a partition of about 32.5 MiB at least). A lookup and a little
/// arithmetic.
pub fn format32(
    sectors: u64,
    hidden: u32,
    serial: u32,
    label: Option<[u8; 11]>,
) -> Result<Format32, FormatError> {
    // Geometry and cluster size are read off the partition's whole size.
    let (heads, track): (u16, u16) = if sectors <= 524_288 {
        (
            match sectors {
                0..=32_768 => 2,
                32_769..=65_536 => 4,
                65_537..=262_144 => 8,
                _ => 16,
            },
            if sectors <= 4096 { 16 } else { 32 },
        )
    } else {
        let k = 63 * 1024;
        (
            match sectors {
                s if s <= 16 * k => 16,
                s if s <= 32 * k => 32,
                s if s <= 64 * k => 64,
                s if s <= 128 * k => 128,
                _ => 255,
            },
            63,
        )
    };
    let cluster_sectors: u32 = match sectors {
        s if s > 32 * 1024 * 1024 * 2 => 64,
        s if s > 16 * 1024 * 1024 * 2 => 32,
        s if s > 8 * 1024 * 1024 * 2 => 16,
        s if s > 260 * 1024 * 2 => 8,
        _ => 1,
    };
    let total = u32::try_from(sectors).unwrap_or(u32::MAX) / u32::from(track) * u32::from(track);
    let align = |n: u32| n.next_multiple_of(cluster_sectors);
    let reserved = align(RESERVED);
    let data = total.checked_sub(reserved).ok_or(FormatError::TooSmall)?;
    let guess = (u64::from(data) * 512 + 16) / (u64::from(cluster_sectors) * 512 + 8);
    let fat_sectors = u32::try_from(align_u64(((guess + 2) * 4).div_ceil(512), cluster_sectors))
        .map_err(|_| FormatError::TooLarge)?;
    let clusters = data
        .checked_sub(2 * fat_sectors)
        .ok_or(FormatError::TooSmall)?
        / cluster_sectors;
    if clusters < MIN_CLUSTERS {
        return Err(FormatError::TooSmall);
    }
    if clusters > 0x0FFF_FFF5 || u64::from(clusters) + 2 > u64::from(fat_sectors) * 128 {
        return Err(FormatError::TooLarge);
    }
    Ok(Format32 {
        serial,
        hidden,
        label: label.unwrap_or(NO_NAME),
        total,
        cluster_sectors,
        reserved,
        fat_sectors,
        clusters,
        heads,
        track,
    })
}

fn align_u64(n: u64, to: u32) -> u64 {
    n.next_multiple_of(u64::from(to))
}

impl Format32 {
    /// The volume's length in bytes: the partition less its unused tail.
    pub fn volume_bytes(&self) -> u64 {
        u64::from(self.total) * SECTOR as u64
    }

    /// How many clusters the volume has.
    pub fn clusters(&self) -> u32 {
        self.clusters
    }

    /// Sectors in a cluster.
    pub fn cluster_sectors(&self) -> u32 {
        self.cluster_sectors
    }

    fn label_set(&self) -> bool {
        self.label != NO_NAME
    }

    /// The boot sector, which the backup repeats.
    fn boot_sector(&self) -> [u8; SECTOR] {
        let mut b = [0u8; SECTOR];
        b[0..3].copy_from_slice(&[0xEB, 0x58, 0x90]);
        b[3..11].copy_from_slice(b"mkfs.fat");
        b[11..13].copy_from_slice(&(SECTOR as u16).to_le_bytes());
        b[13] = self.cluster_sectors as u8;
        b[14..16].copy_from_slice(&(self.reserved as u16).to_le_bytes());
        b[16] = 2;
        // Root entries (17) and the 16-bit sector count (19) are zero.
        b[21] = 0xF8;
        // The 16-bit FAT size (22) is zero.
        b[24..26].copy_from_slice(&self.track.to_le_bytes());
        b[26..28].copy_from_slice(&self.heads.to_le_bytes());
        b[28..32].copy_from_slice(&self.hidden.to_le_bytes());
        b[32..36].copy_from_slice(&self.total.to_le_bytes());
        b[36..40].copy_from_slice(&self.fat_sectors.to_le_bytes());
        // Flags (40) and version (42) are zero.
        b[44..48].copy_from_slice(&2u32.to_le_bytes());
        b[48..50].copy_from_slice(&(INFO_SECTOR as u16).to_le_bytes());
        b[50..52].copy_from_slice(&(BACKUP_SECTOR as u16).to_le_bytes());
        b[64] = 0x80;
        b[66] = 0x29;
        b[67..71].copy_from_slice(&self.serial.to_le_bytes());
        b[71..82].copy_from_slice(&self.label);
        b[82..90].copy_from_slice(b"FAT32   ");
        b[90..90 + HALT.len()].copy_from_slice(&HALT);
        b[510] = 0x55;
        b[511] = 0xAA;
        b
    }

    /// The FSInfo sector: the free clusters (all but the root's) and the
    /// first free one, 3.
    fn info_sector(&self) -> [u8; SECTOR] {
        let mut b = [0u8; SECTOR];
        b[0..4].copy_from_slice(b"RRaA");
        b[484..488].copy_from_slice(b"rrAa");
        b[488..492].copy_from_slice(&(self.clusters - 1).to_le_bytes());
        b[492..496].copy_from_slice(&3u32.to_le_bytes());
        b[510] = 0x55;
        b[511] = 0xAA;
        b
    }

    /// The first sector of a FAT: the media entry, FAT[1] as Windows
    /// leaves it, and the end of the root directory's chain.
    fn fat_start(&self) -> Vec<u8> {
        let mut b = vec![0u8; SECTOR];
        b[0..4].copy_from_slice(&0x0FFF_FFF8u32.to_le_bytes());
        b[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        b[8..12].copy_from_slice(&0x0FFF_FFF8u32.to_le_bytes());
        b
    }

    /// Everything to write, in order of offset: the reserved sectors, both
    /// FATs and the root directory's cluster. Nothing else of the volume
    /// is written: its other clusters are free, and a FAT32 reader does
    /// not read a free cluster. At most twelve regions, whatever the
    /// size.
    pub fn regions(&self) -> Vec<Region> {
        let s = SECTOR as u64;
        let sector = |n: u32| u64::from(n) * s;
        let boot = self.boot_sector().to_vec();
        let info = self.info_sector().to_vec();
        let mut r = vec![
            Region::Bytes {
                at: 0,
                data: boot.clone(),
            },
            Region::Bytes {
                at: sector(INFO_SECTOR),
                data: info.clone(),
            },
            Region::Zeros {
                at: sector(INFO_SECTOR + 1),
                len: sector(BACKUP_SECTOR - INFO_SECTOR - 1),
            },
            Region::Bytes {
                at: sector(BACKUP_SECTOR),
                data: boot,
            },
            Region::Bytes {
                at: sector(BACKUP_SECTOR + INFO_SECTOR),
                data: info,
            },
            Region::Zeros {
                at: sector(BACKUP_SECTOR + INFO_SECTOR + 1),
                len: sector(self.reserved - BACKUP_SECTOR - INFO_SECTOR - 1),
            },
        ];
        for copy in 0..2 {
            let at = sector(self.reserved + copy * self.fat_sectors);
            r.push(Region::Bytes {
                at,
                data: self.fat_start(),
            });
            r.push(Region::Zeros {
                at: at + s,
                len: sector(self.fat_sectors - 1),
            });
        }
        let root = sector(self.reserved + 2 * self.fat_sectors);
        let cluster = sector(self.cluster_sectors);
        if self.label_set() {
            let mut e = [0u8; 32];
            e[..11].copy_from_slice(&self.label);
            if e[0] == 0xE5 {
                e[0] = 0x05;
            }
            e[11] = 0x08;
            for at in [16usize, 18, 24] {
                e[at..at + 2].copy_from_slice(&DATE_1980.to_le_bytes());
            }
            r.push(Region::Bytes {
                at: root,
                data: e.to_vec(),
            });
            r.push(Region::Zeros {
                at: root + 32,
                len: cluster - 32,
            });
        } else {
            r.push(Region::Zeros {
                at: root,
                len: cluster,
            });
        }
        r
    }

    /// Writes [`Format32::regions`] to a disk (a partition from its first
    /// byte), zeros a piece at a time, and flushes. The zeros are the
    /// volume's FATs, some megabytes on a large stick.
    pub fn write<D: Disk>(&self, disk: &mut D) -> Result<(), Error> {
        let zeros = [0u8; 64 << 10];
        for region in self.regions() {
            match region {
                Region::Bytes { at, data } => disk.write_at(at, &data)?,
                Region::Zeros { at, len } => {
                    let mut done = 0;
                    while done < len {
                        let n = (len - done).min(zeros.len() as u64) as usize;
                        disk.write_at(at + done, &zeros[..n])?;
                        done += n as u64;
                    }
                }
            }
        }
        disk.flush()
    }
}
