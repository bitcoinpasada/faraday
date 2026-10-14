//! A GPT partition table read, and a Faraday stick's layout checked, as
//! `faraday/image/stick-boot.sh` checks it before keep data writes the
//! boot partition: GPT with Faraday's disk UUID and exactly two
//! partitions, the first named `OSKBOOT`, an EFI system partition with
//! Faraday's partition UUID and at least as large as what is written,
//! the second named `OSKDATA` with its own UUID, and the two not
//! overlapping (`genimage.cfg` sets the UUIDs).

use std::io;

use faraday_starter::api::StickFault;

/// Faraday's disk UUID (`genimage.cfg`).
pub const DISK_UUID: &str = "4F70656E-5369-676E-6572-4B6974000000";
/// The OSKBOOT partition's UUID.
pub const BOOT_UUID: &str = "4F70656E-5369-676E-6572-4B6974000001";
/// The OSKDATA partition's UUID.
pub const DATA_UUID: &str = "4F70656E-5369-676E-6572-4B6974000002";
/// The EFI system partition's type.
pub const ESP_TYPE: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";

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

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn le64(b: &[u8], at: usize) -> u64 {
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

/// The CRC-32 GPT uses (IEEE 802.3, reflected).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Reads the GPT through `read_at`, which fills a buffer from a byte
/// offset; `len` is the disk's length in bytes. Tries 512-byte sectors,
/// then 4096. The header's and the entries' CRCs must hold.
pub fn read(
    read_at: &mut dyn FnMut(u64, &mut [u8]) -> io::Result<()>,
    len: u64,
) -> Result<Table, StickFault> {
    let mut last = StickFault::NoHeader;
    for sector in [512u64, 4096] {
        match read_with(read_at, len, sector) {
            Ok(t) => return Ok(t),
            Err(e) => last = e,
        }
    }
    Err(last)
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
    if entries_lba.saturating_mul(sector) + entries_len as u64 > len {
        return Err(StickFault::EntriesPastEnd);
    }
    let mut entries = vec![0u8; entries_len];
    read_at(entries_lba * sector, &mut entries).map_err(|e| StickFault::Reading(e.to_string()))?;
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
    let entry_sectors = (entries_len as u64).div_ceil(sector);
    let mut regions = vec![(0, (entries_lba + entry_sectors) * sector)];
    // The backup: its entries just before its header, at the disk's end.
    if alternate > entry_sectors && (alternate + 1) * sector <= len {
        regions.push((
            (alternate - entry_sectors) * sector,
            (alternate + 1) * sector,
        ));
    }
    Ok(Table {
        sector,
        disk_guid: guid(&header[56..72]),
        parts,
        regions,
    })
}

/// Where a Faraday stick's boot partition is, when `table` is a Faraday
/// stick's and its boot partition holds `needs` bytes; otherwise why it
/// is not, in `stick-boot.sh`'s words.
pub fn faraday_boot(table: &Table, needs: u64) -> Result<Boot, StickFault> {
    if !table.disk_guid.eq_ignore_ascii_case(DISK_UUID) {
        return Err(StickFault::DiskUuid);
    }
    if table.parts.len() != 2 {
        return Err(StickFault::Partitions(table.parts.len()));
    }
    let (boot, data) = (&table.parts[0], &table.parts[1]);
    if boot.name != "OSKBOOT" {
        return Err(StickFault::BootName);
    }
    if !boot.guid.eq_ignore_ascii_case(BOOT_UUID) || !boot.type_guid.eq_ignore_ascii_case(ESP_TYPE)
    {
        return Err(StickFault::BootType);
    }
    if data.name != "OSKDATA" || !data.guid.eq_ignore_ascii_case(DATA_UUID) {
        return Err(StickFault::DataType);
    }
    if boot.last < boot.first {
        return Err(StickFault::BootDamaged);
    }
    let len = (boot.last - boot.first + 1) * table.sector;
    if len < needs {
        return Err(StickFault::BootSmall {
            boot: len,
            image: needs,
        });
    }
    if boot.last >= data.first {
        return Err(StickFault::Overlap);
    }
    Ok(Boot {
        offset: boot.first * table.sector,
        len,
    })
}
