//! A GPT partition table read, and a Faraday stick known by its layout
//! (`docs/STICK-IDENTITY.md` §5.1, F174), as
//! `faraday/image/stick-boot.sh` checks it before keep data writes the
//! boot partition: exactly two partitions, the first an EFI system
//! partition of exactly 64 MiB at 1 MiB, the second a basic data
//! partition right after it of at least 128 MiB, ending at or before the
//! disk's last usable sector (F80, F182). Names, IDs and labels play no
//! part: each stick has its own (§4), and a stick made before keeps its
//! old ones (§5.4).

use std::io;

use faraday_boot::layout::{BOOT_SECTORS, BOOT_START, DATA_SECTORS, DATA_START, SECTOR};
use faraday_crypto::crc32;
use faraday_starter::api::StickFault;

/// The EFI system partition's type.
pub const ESP_TYPE: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
/// The basic data partition's type.
pub const DATA_TYPE: &str = "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7";
/// Where a Faraday stick's boot partition starts, in bytes (F80).
const BOOT_AT: u64 = BOOT_START * SECTOR;
/// Its length.
const BOOT_LEN: u64 = BOOT_SECTORS * SECTOR;
/// Where its data partition starts.
const DATA_AT: u64 = DATA_START * SECTOR;
/// Its least length (F80, F182): a stick written with `dd` has this, one
/// made for its size runs to the disk's last usable sector.
const DATA_LEN: u64 = DATA_SECTORS * SECTOR;
/// What a GPT takes at a disk's end after the last usable sector: the
/// backup entries and header, 33 sectors of 512 bytes.
pub const TAIL: u64 = 33 * 512;

/// The byte after a disk's last usable sector, where the table itself is
/// not read and only the disk's length in bytes is known: with 512-byte
/// sectors, the last usable sector is 34 from the end (its byte after
/// is 33). A disk of 4096-byte sectors keeps more at its end, so this is
/// only for listing a stick; keep data reads the table's own last usable
/// sector ([`faraday_boot`]).
pub fn usable_end(disk_len: u64) -> u64 {
    disk_len.saturating_sub(TAIL)
}

/// One partition as a system lists it, in bytes: what §5.1's layout is
/// read from on each system (udisks2, `diskutil`,
/// `IOCTL_DISK_GET_DRIVE_LAYOUT_EX`, or the table itself).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed<'a> {
    /// Its type, as a GUID string in either case.
    pub type_guid: &'a str,
    /// Its first byte.
    pub offset: u64,
    /// Its length in bytes.
    pub len: u64,
}

/// Whether `parts`, in their order on the disk, are a Faraday stick's
/// (§5.1); otherwise why not. `usable_end` is the byte after the disk's
/// last usable sector (the table header's, or [`usable_end`]). A boot
/// partition at 1 MiB shorter than 64 MiB (a stick made before F80) is
/// [`StickFault::BootSmall`], with both sizes.
pub fn layout(parts: &[Placed<'_>], usable_end: u64) -> Result<(), StickFault> {
    let [boot, data] = parts else {
        return Err(StickFault::Partitions(parts.len()));
    };
    if !boot.type_guid.eq_ignore_ascii_case(ESP_TYPE) {
        return Err(StickFault::BootType);
    }
    if !data.type_guid.eq_ignore_ascii_case(DATA_TYPE) {
        return Err(StickFault::DataType);
    }
    if boot.offset == BOOT_AT && boot.len < BOOT_LEN {
        return Err(StickFault::BootSmall {
            boot: boot.len,
            image: BOOT_LEN,
        });
    }
    if boot.offset != BOOT_AT || boot.len != BOOT_LEN {
        return Err(StickFault::BootPlace);
    }
    if data.offset != DATA_AT
        || data.len < DATA_LEN
        || data
            .offset
            .checked_add(data.len)
            .is_none_or(|e| e > usable_end)
    {
        return Err(StickFault::DataPlace);
    }
    Ok(())
}

/// One partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// Its type, as a GUID string.
    pub type_guid: String,
    /// Its own GUID.
    pub guid: String,
    /// Its first sector.
    pub first: u64,
    /// Its last sector, inclusive.
    pub last: u64,
    /// Its GPT name.
    pub name: String,
}

/// A GPT, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The sector size it was found with.
    pub sector: u64,
    /// The disk's GUID.
    pub disk_guid: String,
    /// The header's last usable sector.
    pub last_usable: u64,
    /// The header's size in bytes.
    pub header_len: usize,
    /// The first sector of the entries.
    pub entries_lba: u64,
    /// How many entries the table has, used or not.
    pub entry_count: usize,
    /// The size of one entry in bytes.
    pub entry_size: usize,
    /// The sector of the backup header.
    pub alternate: u64,
    /// The partitions in use, in the table's order.
    pub parts: Vec<Part>,
    /// The byte ranges the table occupies: the protective MBR, the
    /// header and its entries, and the backup entries and header.
    pub regions: Vec<(u64, u64)>,
}

/// The boot partition of a Faraday stick: where it is and how long, in
/// bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boot {
    /// Its first byte.
    pub offset: u64,
    /// Its length.
    pub len: u64,
}

pub(crate) fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub(crate) fn le64(b: &[u8], at: usize) -> u64 {
    u64::from(le32(b, at)) | (u64::from(le32(b, at + 4)) << 32)
}

/// A GUID as GPT stores it (the first three fields little-endian), in
/// the upper-case form `sfdisk` prints.
pub fn guid(b: &[u8]) -> String {
    format!(
        "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        le32(b, 0),
        u16::from_le_bytes([b[4], b[5]]),
        u16::from_le_bytes([b[6], b[7]]),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Reads the GPT through `read_at`, which fills a buffer from a byte
/// offset; `len` is the disk's length in bytes. Tries 512-byte sectors,
/// then 4096. The header's and the entries' CRCs must hold. Where a
/// header is found and damaged, its fault is the one given (the
/// 512-byte try's first), not "no GPT" from the other try.
pub fn read(
    read_at: &mut dyn FnMut(u64, &mut [u8]) -> io::Result<()>,
    len: u64,
) -> Result<Table, StickFault> {
    // The first fault of a header that was there: a 4096-byte try that
    // finds no header must not hide the damage the 512-byte one found.
    let mut damaged = None;
    let mut last = StickFault::NoHeader;
    for sector in [512u64, 4096] {
        match read_with(read_at, len, sector) {
            Ok(t) => return Ok(t),
            Err(StickFault::NotGpt) => last = StickFault::NotGpt,
            Err(e @ StickFault::Reading(_)) => return Err(e),
            Err(e) => {
                damaged.get_or_insert(e);
            }
        }
    }
    Err(damaged.unwrap_or(last))
}

fn read_with(
    read_at: &mut dyn FnMut(u64, &mut [u8]) -> io::Result<()>,
    len: u64,
    sector: u64,
) -> Result<Table, StickFault> {
    let ss = sector as usize;
    let mut header = vec![0u8; ss];
    read_at(sector, &mut header).map_err(|e| StickFault::Reading(e.to_string()))?;
    if &header[..8] != b"EFI PART" {
        return Err(StickFault::NotGpt);
    }
    let size = le32(&header, 12) as usize;
    if !(92..=ss).contains(&size) {
        return Err(StickFault::HeaderDamaged);
    }
    let mut check = header[..size].to_vec();
    check[16..20].fill(0);
    if crc32(&check) != le32(&header, 16) {
        return Err(StickFault::HeaderChecksum);
    }
    let alternate = le64(&header, 32);
    let entries_lba = le64(&header, 72);
    let count = le32(&header, 80) as usize;
    let entry_size = le32(&header, 84) as usize;
    if !(128..=4096).contains(&entry_size) || count == 0 || count > 1024 {
        return Err(StickFault::EntriesDamaged);
    }
    let entries_len = count * entry_size;
    let entry_sectors = (entries_len as u64).div_ceil(sector);
    // Every offset below is the header's own numbers times the sector
    // size, so each is worked out checked: a header whose numbers run
    // past what a disk can address is refused, not wrapped.
    let entries_at = entries_lba
        .checked_mul(sector)
        .ok_or(StickFault::EntriesPastEnd)?;
    let primary_end = entries_lba
        .checked_add(entry_sectors)
        .and_then(|s| s.checked_mul(sector))
        .ok_or(StickFault::EntriesPastEnd)?;
    if entries_at
        .checked_add(entries_len as u64)
        .is_none_or(|end| end > len)
    {
        return Err(StickFault::EntriesPastEnd);
    }
    let backup_end = alternate
        .checked_add(1)
        .and_then(|s| s.checked_mul(sector))
        .ok_or(StickFault::HeaderDamaged)?;
    let mut entries = vec![0u8; entries_len];
    read_at(entries_at, &mut entries).map_err(|e| StickFault::Reading(e.to_string()))?;
    if crc32(&entries) != le32(&header, 88) {
        return Err(StickFault::EntriesChecksum);
    }
    let mut parts = Vec::new();
    for e in entries.chunks_exact(entry_size) {
        if e[..16].iter().all(|&b| b == 0) {
            continue;
        }
        let units: Vec<u16> = e[56..128]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        parts.push(Part {
            type_guid: guid(&e[..16]),
            guid: guid(&e[16..32]),
            first: le64(e, 32),
            last: le64(e, 40),
            name: String::from_utf16_lossy(&units),
        });
    }
    let mut regions = vec![(0, primary_end)];
    // The backup: its entries just before its header, at the disk's end.
    if alternate > entry_sectors && backup_end <= len {
        regions.push(((alternate - entry_sectors) * sector, backup_end));
    }
    Ok(Table {
        sector,
        disk_guid: guid(&header[56..72]),
        last_usable: le64(&header, 48),
        header_len: size,
        entries_lba,
        entry_count: count,
        entry_size,
        alternate,
        parts,
        regions,
    })
}

/// Where a Faraday stick's boot partition is, when `table` is a Faraday
/// stick's (§5.1) and its boot partition holds `needs` bytes; otherwise
/// why it is not, in `stick-boot.sh`'s words.
pub fn faraday_boot(table: &Table, needs: u64) -> Result<Boot, StickFault> {
    let mut placed = Vec::with_capacity(table.parts.len());
    for (i, p) in table.parts.iter().enumerate() {
        // The sectors are the table's own numbers: a partition that ends
        // before it starts, or runs past what a disk can address, is
        // damaged, not wrapped.
        let bytes = (p.last >= p.first)
            .then(|| {
                Some((
                    p.first.checked_mul(table.sector)?,
                    (p.last - p.first)
                        .checked_add(1)?
                        .checked_mul(table.sector)?,
                ))
            })
            .flatten();
        let Some((offset, len)) = bytes else {
            return Err(if i == 0 {
                StickFault::BootDamaged
            } else {
                StickFault::DataPlace
            });
        };
        placed.push(Placed {
            type_guid: &p.type_guid,
            offset,
            len,
        });
    }
    let usable_end = table
        .last_usable
        .checked_add(1)
        .and_then(|s| s.checked_mul(table.sector))
        .ok_or(StickFault::HeaderDamaged)?;
    layout(&placed, usable_end)?;
    let boot = placed[0];
    if boot.len < needs {
        return Err(StickFault::BootSmall {
            boot: boot.len,
            image: needs,
        });
    }
    Ok(Boot {
        offset: boot.offset,
        len: boot.len,
    })
}
