//! Each stick's and card's own IDs (`docs/STICK-IDENTITY.md` §4, F174).
//!
//! The image keeps fixed IDs, so a build is reproducible (§4.1). A whole
//! stick is made for its size (§4.2, [`Capture::stick`]): its table with
//! a new disk GUID and new partition GUIDs, its data partition's FAT32
//! with a new serial; its boot partition is the release's, serial
//! included (F182). A Pi card is the image with a new MBR disk signature
//! and a new serial for each FAT volume, in its boot sector and, on
//! FAT32, in the backup boot sector it names. Keep data writes no
//! partition table and changes nothing of the boot partition (§4.4).
//!
//! What is changed is a list of [`Patch`]es on the source's stream: the
//! writer writes and compares the stream with them in place, and takes
//! them back out of what it read back to check the image's SHA-256
//! (§4.3). [`Capture`] gathers, as the source streams past once, the few
//! sectors they are worked out from.
//!
//! A name the person gave the data partition (§6) is a list of patches
//! too: `faraday_fat`'s own label write, run over the captured sectors of
//! the data partition ([`Capture::name`]), each byte it changed one
//! patch.

use std::io::{self, Write};

use faraday_boot::layout::{BOOT_SECTORS, DATA_SECTORS, DATA_START, SECTOR};
use faraday_crypto::crc32;
use faraday_starter::api::StickFault;

use crate::gpt::{self, le32, le64};

/// The start of an image read before its partition table is known.
pub const HEAD: usize = 64 << 10;
/// What is kept of each partition's start: its boot sector and the
/// backup boot sector, which FAT32 puts in the reserved sectors.
const PART_HEAD: usize = 64 << 10;
/// An MBR's disk signature (§4.2).
const MBR_SIGNATURE: usize = 440;
/// The MBR partition types a FAT volume has.
const FAT_TYPES: [u8; 5] = [0x04, 0x06, 0x0B, 0x0C, 0x0E];
/// A Pi card's partitions' lengths (F80), in order.
const CARD: [u64; 2] = [BOOT_SECTORS * SECTOR, DATA_BYTES];
/// The data partition's start and length, in bytes.
const DATA_BYTE_START: u64 = DATA_START * SECTOR;
const DATA_BYTES: u64 = DATA_SECTORS * SECTOR;

/// Bytes put in place of the source's at `at` in its decompressed
/// stream, and the source's own bytes there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// Where, in the source's stream.
    pub at: u64,
    /// What is written there.
    pub bytes: Vec<u8>,
    /// What the source holds there.
    pub was: Vec<u8>,
}

/// What a source is, as far as its IDs go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A whole image: a stick's (GPT) or a Pi card's (MBR).
    Whole,
    /// A boot partition alone (`-boot.vfat`).
    Boot,
}

/// The sectors of a source the IDs are worked out from, gathered as it
/// streams past ([`Write`]): its first [`HEAD`] bytes, the start of each
/// partition, and a stick's backup GPT.
#[derive(Debug)]
pub struct Capture {
    shape: Shape,
    total: u64,
    pos: u64,
    /// The stretches wanted, in order, each with what has come of it.
    wanted: Vec<(u64, Vec<u8>, usize)>,
    /// The table, once the head is read.
    table: Option<Table>,
    /// Whether the data partition's root directory is asked for: once
    /// its boot sector has come, which says where the root is.
    root_asked: bool,
    fault: Option<StickFault>,
}

/// An image's partition table, as far as its IDs go.
#[derive(Debug, Clone)]
enum Table {
    /// A stick's GPT: its sector size, the primary header's place, its
    /// size and its entries' place and length, the backup's region.
    Gpt {
        sector: u64,
        header: u64,
        header_len: usize,
        entries: u64,
        entries_len: usize,
        backup_entries: u64,
        backup_header: u64,
        parts: Vec<u64>,
    },
    /// A Pi card's MBR: where its partitions start.
    Mbr { parts: Vec<u64> },
    /// A boot partition alone, at 0.
    Boot,
}

impl Capture {
    /// A capture of a source of `total` bytes shaped as `shape`.
    pub fn new(shape: Shape, total: u64) -> Self {
        let first = match shape {
            Shape::Whole => HEAD,
            Shape::Boot => PART_HEAD,
        };
        Capture {
            shape,
            total,
            pos: 0,
            wanted: vec![(0, vec![0; first.min(total as usize)], 0)],
            table: None,
            root_asked: false,
            fault: None,
        }
    }

    /// The stretch at `at` of `len` bytes, as it came.
    fn got(&self, at: u64, len: usize) -> Result<&[u8], StickFault> {
        self.wanted
            .iter()
            .find(|(from, buf, n)| {
                *from <= at && at + len as u64 <= *from + *n as u64 && *n == buf.len()
            })
            .map(|(from, buf, _)| &buf[(at - from) as usize..(at - from) as usize + len])
            .ok_or(StickFault::BootPastEnd)
    }

    /// The source's first sector, as it came.
    pub fn first_sector(&self) -> Result<&[u8], StickFault> {
        self.got(0, 512)
    }

    /// Reads the table from the head, and asks for the rest.
    fn plan(&mut self) -> Result<(), StickFault> {
        let head = self.wanted[0].1.clone();
        let table = match self.shape {
            Shape::Boot => Table::Boot,
            Shape::Whole if head.len() >= 1024 && &head[512..520] == b"EFI PART" => {
                gpt_table(&head, self.total)?
            }
            Shape::Whole => mbr_table(&head)?,
        };
        let mut more: Vec<(u64, usize)> = Vec::new();
        match &table {
            Table::Gpt {
                parts,
                backup_entries,
                backup_header,
                sector,
                ..
            } => {
                more.extend(parts.iter().map(|&p| (p, PART_HEAD)));
                more.push((
                    *backup_entries,
                    (backup_header + sector - backup_entries) as usize,
                ));
            }
            Table::Mbr { parts } => more.extend(parts.iter().map(|&p| (p, PART_HEAD))),
            Table::Boot => {}
        }
        for (at, len) in more {
            if at + len as u64 > self.total || at < self.pos {
                return Err(StickFault::BootPastEnd);
            }
            self.wanted.push((at, vec![0; len], 0));
        }
        self.wanted.sort_by_key(|w| w.0);
        self.table = Some(table);
        Ok(())
    }

    /// The fresh IDs of a whole Pi card (§4.2), drawn from `random`: a
    /// new MBR disk signature and a new serial for each FAT volume.
    pub fn fresh(&self, random: &mut dyn FnMut(&mut [u8])) -> Result<Vec<Patch>, StickFault> {
        let mut out = Vec::new();
        let table = self.done()?;
        match table {
            // A stick is made, not patched ([`Capture::stick`]).
            Table::Gpt { .. } => return Err(StickFault::CardLayout),
            Table::Mbr { .. } => {
                let was = self.got(MBR_SIGNATURE as u64, 4)?.to_vec();
                out.push(Patch {
                    at: MBR_SIGNATURE as u64,
                    bytes: nonzero(random).to_vec(),
                    was,
                });
            }
            Table::Boot => {}
        }
        for &p in parts(table) {
            out.extend(self.serial(p, nonzero(random))?);
        }
        Ok(out)
    }

    /// Whether the source is a stick image (GPT), which the Starter
    /// makes a stick from ([`Capture::stick`]), and not a Pi card's.
    pub fn is_stick(&self) -> bool {
        matches!(self.table, Some(Table::Gpt { .. }))
    }

    /// What a whole stick of `disk_len` bytes is made of
    /// (`docs/STICK-IDENTITY.md` §4.2), as places on the disk in order
    /// of offset, with their offsets from the disk's start: the
    /// protective MBR covering the disk, the primary header at sector 1
    /// (first usable sector 34, last usable 33 before the backup at the
    /// disk's last sector), the entries at sector 2 (the image's, with
    /// the stick's own GUIDs and the data partition running to the last
    /// usable sector), the backup entries and header at the disk's end,
    /// every CRC worked out; and the data partition's FAT32 volume from
    /// `faraday_fat::format32`, hidden sectors 133120, with the stick's
    /// own serial and `label`. Sectors are 512 bytes whatever the
    /// disk's own. A lookup and some arithmetic; the FATs come as
    /// lengths of zeros.
    pub fn stick(
        &self,
        disk_len: u64,
        label: Option<[u8; 11]>,
        random: &mut dyn FnMut(&mut [u8]),
    ) -> Result<Vec<faraday_fat::Region>, StickFault> {
        use faraday_fat::Region;
        const S: u64 = 512;
        let table = self.done()?;
        let Table::Gpt {
            sector,
            header,
            header_len,
            entries,
            entries_len,
            ..
        } = *table
        else {
            return Err(StickFault::NotGpt);
        };
        let entry_sectors = (entries_len as u64).div_ceil(S);
        if sector != S || entry_sectors != 32 || !(92..=512).contains(&header_len) {
            return Err(StickFault::EntriesDamaged);
        }
        let sectors = disk_len / S;
        let (data_first, boot_last) = (DATA_START, DATA_START - 1);
        // First usable 34, entries at 2: the last usable sector is 33
        // before the backup header at the last sector.
        if sectors < data_first + DATA_SECTORS + 33 + 1 {
            return Err(StickFault::DataPlace);
        }
        let (last, last_usable) = (sectors - 1, sectors - 34);
        let mbr_in = self.got(0, 512)?;
        if mbr_in[510..512] != [0x55, 0xAA] || mbr_in[446 + 4] != 0xEE {
            return Err(StickFault::NotGpt);
        }
        let mut mbr = mbr_in.to_vec();
        let covered = u32::try_from(last).unwrap_or(u32::MAX);
        mbr[446 + 12..446 + 16].copy_from_slice(&covered.to_le_bytes());
        let primary = self.got(header, header_len)?.to_vec();
        let list = self.got(entries, entries_len)?;
        let entry_size = le32(&primary, 84) as usize;
        let disk = guid(random);
        let mut new_list = list.to_vec();
        let mut data = false;
        for e in new_list.chunks_exact_mut(entry_size) {
            if e[..16].iter().all(|&b| b == 0) {
                continue;
            }
            e[16..32].copy_from_slice(&guid(random));
            if le64(e, 32) == data_first {
                e[40..48].copy_from_slice(&last_usable.to_le_bytes());
                data = true;
            } else if le64(e, 40) != boot_last {
                return Err(StickFault::BootPlace);
            }
        }
        if !data {
            return Err(StickFault::DataPlace);
        }
        let list_crc = crc32(&new_list).to_le_bytes();
        let header_at = |current: u64, other: u64, entries_lba: u64| {
            let mut h = vec![0u8; 512];
            h[..header_len].copy_from_slice(&primary);
            h[24..32].copy_from_slice(&current.to_le_bytes());
            h[32..40].copy_from_slice(&other.to_le_bytes());
            h[40..48].copy_from_slice(&34u64.to_le_bytes());
            h[48..56].copy_from_slice(&last_usable.to_le_bytes());
            h[56..72].copy_from_slice(&disk);
            h[72..80].copy_from_slice(&entries_lba.to_le_bytes());
            h[88..92].copy_from_slice(&list_crc);
            h[16..20].fill(0);
            let crc = crc32(&h[..header_len]).to_le_bytes();
            h[16..20].copy_from_slice(&crc);
            h
        };
        let backup_entries = last - entry_sectors;
        let mut out = vec![
            Region::Bytes { at: 0, data: mbr },
            Region::Bytes {
                at: S,
                data: header_at(1, last, 2),
            },
            Region::Bytes {
                at: 2 * S,
                data: new_list.clone(),
            },
        ];
        out.extend(data_volume(last_usable + 1 - data_first, label, random)?);
        out.push(Region::Bytes {
            at: backup_entries * S,
            data: new_list,
        });
        out.push(Region::Bytes {
            at: last * S,
            data: header_at(last, 1, backup_entries),
        });
        Ok(out)
    }

    fn done(&self) -> Result<&Table, StickFault> {
        if let Some(f) = &self.fault {
            return Err(f.clone());
        }
        match &self.table {
            Some(t) if self.wanted.iter().all(|(_, b, n)| *n == b.len()) => Ok(t),
            _ => Err(StickFault::BootPastEnd),
        }
    }

    /// The patches that name the data partition `label` (§6), as
    /// [`faraday_fat::label_bytes`] gives it: `faraday_fat`'s label write
    /// (the boot sector, the FAT32 backup boot sector and the root
    /// directory's volume-label entry), run over the sectors captured,
    /// each stretch of bytes it changed a patch. A data partition whose
    /// label cannot be written from them is [`StickFault::Name`].
    pub fn name(&self, label: [u8; 11]) -> Result<Vec<Patch>, StickFault> {
        self.done()?;
        let base = self.data_part().ok_or(StickFault::Name)?;
        let disk = Sparse {
            capture: self,
            base,
            writes: Vec::new(),
        };
        let mut volume = faraday_fat::Volume::open(disk).map_err(|_| StickFault::Name)?;
        volume
            .set_label(Some(label))
            .map_err(|_| StickFault::Name)?;
        let mut disk = volume.into_disk();
        let mut spans: Vec<(u64, u64)> = disk
            .writes
            .iter()
            .map(|(at, b)| (*at, *at + b.len() as u64))
            .collect();
        spans.sort_unstable();
        let mut out = Vec::new();
        let mut done_to = 0u64;
        for (from, to) in spans {
            let from = from.max(done_to);
            if from >= to {
                continue;
            }
            done_to = to;
            let mut now = vec![0u8; (to - from) as usize];
            faraday_fat::Disk::read_at(&mut disk, from, &mut now).map_err(|_| StickFault::Name)?;
            let was = self.got(base + from, now.len())?;
            out.extend(changed(base + from, was, &now));
        }
        Ok(out)
    }

    /// The patches that set the serial of the FAT volume at `part` to
    /// `serial`: in its boot sector and, on FAT32, in its backup boot
    /// sector (Microsoft's FAT specification, `BS_VolID`, `BPB_BkBootSec`).
    fn serial(&self, part: u64, serial: [u8; 4]) -> Result<Vec<Patch>, StickFault> {
        let sector = self.got(part, 512)?;
        let bs = faraday_fat::BootSector::parse(sector).ok_or(StickFault::NoSerial)?;
        let old = bs.serial.ok_or(StickFault::NoSerial)?;
        if sector[510..512] != [0x55, 0xAA] {
            return Err(StickFault::NoSerial);
        }
        let at = bs.serial_at as u64;
        let mut places = vec![part + at];
        if bs.fat32 && bs.backup_boot != 0 {
            let b = part + u64::from(bs.backup_boot) * u64::from(bs.bytes_per_sector);
            let copy = self.got(b, 512).map_err(|_| StickFault::NoSerial)?;
            if faraday_fat::boot_sector_serial(copy) != Some(old) {
                return Err(StickFault::NoSerial);
            }
            places.push(b + at);
        }
        places
            .into_iter()
            .map(|p| {
                Ok(Patch {
                    at: p,
                    bytes: serial.to_vec(),
                    was: self.got(p, 4)?.to_vec(),
                })
            })
            .collect()
    }
}

impl Capture {
    /// The data partition's start: a whole image's second partition.
    fn data_part(&self) -> Option<u64> {
        match &self.table {
            Some(Table::Gpt { parts, .. } | Table::Mbr { parts }) => parts.get(1).copied(),
            _ => None,
        }
    }

    /// The next place in the stream where what is wanted next is
    /// decided: the head's end, then the data partition's boot sectors'.
    fn next_stop(&self) -> Option<u64> {
        if self.fault.is_some() {
            return None;
        }
        match &self.table {
            None => Some(self.wanted[0].1.len() as u64),
            Some(_) if !self.root_asked => self.data_part().map(|d| d + PART_HEAD as u64),
            Some(_) => None,
        }
    }

    /// Decides what is wanted next, once the stream is at the place
    /// [`Capture::next_stop`] named.
    fn decide(&mut self) {
        if self.fault.is_some() {
            return;
        }
        if self.table.is_none() {
            if self.pos >= self.wanted[0].1.len() as u64
                && let Err(f) = self.plan()
            {
                self.fault = Some(f);
            }
            return;
        }
        let Some(data) = self.data_part() else {
            return;
        };
        if self.root_asked || self.pos < data + PART_HEAD as u64 {
            return;
        }
        self.root_asked = true;
        // A boot sector that is not FAT's asks for nothing: the name is
        // then refused where it is asked for.
        let Some((at, len)) = self.got(data, 512).ok().and_then(root_region) else {
            return;
        };
        let at = data + at;
        if at >= self.pos && at + len <= data + DATA_BYTES && at + len <= self.total {
            self.wanted.push((at, vec![0; len as usize], 0));
            self.wanted.sort_by_key(|w| w.0);
        }
    }

    /// Keeps what of `data`, the next bytes, falls in a stretch wanted.
    fn feed(&mut self, data: &[u8]) {
        let (start, end) = (self.pos, self.pos + data.len() as u64);
        self.pos = end;
        for (from, buf, n) in &mut self.wanted {
            let to = *from + buf.len() as u64;
            let (a, b) = (start.max(*from), end.min(to));
            if a < b {
                buf[(a - *from) as usize..(b - *from) as usize]
                    .copy_from_slice(&data[(a - start) as usize..(b - start) as usize]);
                *n += (b - a) as usize;
            }
        }
    }
}

impl Write for Capture {
    fn write(&mut self, mut data: &[u8]) -> io::Result<usize> {
        let len = data.len();
        // The head first, and the table read from it, before any byte
        // past it; then the data partition's boot sectors, before any
        // byte past them: what else is wanted depends on each.
        while !data.is_empty() {
            let take = match self.next_stop() {
                Some(stop) if stop > self.pos => ((stop - self.pos) as usize).min(data.len()),
                _ => data.len(),
            };
            self.feed(&data[..take]);
            data = &data[take..];
            self.decide();
        }
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The data partition's FAT32 volume for a partition of `sectors` sectors
/// at the stick's data start: its regions, with offsets from the disk's
/// start, the stick's own serial and `label`.
fn data_volume(
    sectors: u64,
    label: Option<[u8; 11]>,
    random: &mut dyn FnMut(&mut [u8]),
) -> Result<Vec<faraday_fat::Region>, StickFault> {
    use faraday_fat::Region;
    let hidden = u32::try_from(DATA_START).map_err(|_| StickFault::DataPlace)?;
    let serial = u32::from_le_bytes(nonzero(random));
    let format =
        faraday_fat::format32(sectors, hidden, serial, label).map_err(|_| StickFault::DataPlace)?;
    Ok(format
        .regions()
        .into_iter()
        .map(|r| match r {
            Region::Bytes { at, data } => Region::Bytes {
                at: DATA_BYTE_START + at,
                data,
            },
            Region::Zeros { at, len } => Region::Zeros {
                at: DATA_BYTE_START + at,
                len,
            },
        })
        .collect())
}

/// The 11 bytes of the data partition's name `name` (§6), when it is one
/// as an order carries it: as FAT writes it ([`faraday_fat::label_bytes`]:
/// in capitals, at most 11 of FAT's characters, no spaces at its ends),
/// not empty, and not a boot partition's (`faraday_boot::layout`). A
/// lookup.
pub fn label_of(name: &str) -> Option<[u8; 11]> {
    let bytes = faraday_fat::label_bytes(name).ok()??;
    let written = bytes.trim_ascii_end();
    (written == name.as_bytes() && !faraday_boot::layout::is_boot_label(&bytes)).then_some(bytes)
}

/// The data partition as far as it was captured, seen by `faraday_fat`
/// as a disk: reads come from the stretches captured, with the writes so
/// far over them; a read of a byte not captured fails. Nothing reaches
/// the stick from here: the writes become patches ([`Capture::name`]).
struct Sparse<'c> {
    capture: &'c Capture,
    base: u64,
    writes: Vec<(u64, Vec<u8>)>,
}

impl faraday_fat::Disk for Sparse<'_> {
    fn size(&self) -> u64 {
        DATA_BYTES
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), faraday_fat::Error> {
        let got = self
            .capture
            .got(self.base + offset, buf.len())
            .map_err(|_| faraday_fat::Error::Io)?;
        buf.copy_from_slice(got);
        let end = offset + buf.len() as u64;
        for (at, bytes) in &self.writes {
            let (a, b) = (offset.max(*at), end.min(*at + bytes.len() as u64));
            if a < b {
                buf[(a - offset) as usize..(b - offset) as usize]
                    .copy_from_slice(&bytes[(a - at) as usize..(b - at) as usize]);
            }
        }
        Ok(())
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), faraday_fat::Error> {
        if offset + buf.len() as u64 > DATA_BYTES {
            return Err(faraday_fat::Error::Io);
        }
        self.writes.push((offset, buf.to_vec()));
        Ok(())
    }
}

/// A patch for each run of bytes where `now` differs from `was`, both
/// starting at stream byte `at`.
fn changed(at: u64, was: &[u8], now: &[u8]) -> Vec<Patch> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < now.len() {
        if now[i] == was[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < now.len() && now[i] != was[i] {
            i += 1;
        }
        out.push(Patch {
            at: at + start as u64,
            bytes: now[start..i].to_vec(),
            was: was[start..i].to_vec(),
        });
    }
    out
}

/// Where a FAT volume's root directory starts, from its boot sector, and
/// the bytes of it a label is written in: a FAT32 root's first cluster,
/// or a FAT16 root's fixed entries (Microsoft's FAT specification, §3.5,
/// §6). `None` for a boot sector that is not FAT's.
fn root_region(bs: &[u8]) -> Option<(u64, u64)> {
    let b = faraday_fat::BootSector::parse(bs)?;
    if bs[510..512] != [0x55, 0xAA] {
        return None;
    }
    let bps = u64::from(b.bytes_per_sector);
    let spc = u64::from(b.sectors_per_cluster);
    let reserved = u64::from(b.reserved);
    let fats = u64::from(b.fats);
    if !matches!(bps, 512 | 1024 | 2048 | 4096) || spc == 0 || reserved == 0 || fats == 0 {
        return None;
    }
    let root_start = (reserved + fats * u64::from(b.fat_size)) * bps;
    if b.root_entries != 0 {
        return Some((root_start, u64::from(b.root_entries) * 32));
    }
    let first = u64::from(b.root_cluster).checked_sub(2)?;
    Some((root_start + first * spc * bps, spc * bps))
}

fn parts(t: &Table) -> &[u64] {
    match t {
        Table::Gpt { parts, .. } | Table::Mbr { parts } => parts,
        Table::Boot => &[0],
    }
}

/// A stick image's GPT, from its first bytes: §5.1's layout, by the
/// same check keep data makes of a stick (`gpt::faraday_boot`).
fn gpt_table(head: &[u8], total: u64) -> Result<Table, StickFault> {
    let mut read_at = |at: u64, buf: &mut [u8]| -> io::Result<()> {
        let from = at as usize;
        let src = head
            .get(from..from + buf.len())
            .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;
        buf.copy_from_slice(src);
        Ok(())
    };
    let t = gpt::read(&mut read_at, total)?;
    gpt::faraday_boot(&t, 0)?;
    let header_len = t.header_len;
    let entries = t.entries_lba * t.sector;
    let entries_len = t.entry_count * t.entry_size;
    let alternate = t.alternate;
    let backup_header = alternate
        .checked_mul(t.sector)
        .filter(|b| b + t.sector <= total)
        .ok_or(StickFault::BackupTable)?;
    let backup_entries = backup_header
        .checked_sub((entries_len as u64).div_ceil(t.sector) * t.sector)
        .ok_or(StickFault::BackupTable)?;
    Ok(Table::Gpt {
        sector: t.sector,
        header: t.sector,
        header_len,
        entries,
        entries_len,
        backup_entries,
        backup_header,
        parts: t.parts.iter().map(|p| p.first * t.sector).collect(),
    })
}

/// A Pi card image's MBR, from its first sector: exactly two FAT
/// partitions, of 64 MiB and 128 MiB (§5.1).
fn mbr_table(head: &[u8]) -> Result<Table, StickFault> {
    if head.len() < 512 || head[510..512] != [0x55, 0xAA] {
        return Err(StickFault::NotGpt);
    }
    let mut parts = Vec::new();
    for i in 0..4 {
        let e = &head[446 + 16 * i..462 + 16 * i];
        if e[4] == 0 {
            continue;
        }
        let first = u64::from(le32(e, 8));
        let count = u64::from(le32(e, 12));
        parts.push((e[4], first * 512, count * 512));
    }
    parts.sort_by_key(|p| p.1);
    let fits = parts.len() == 2
        && parts
            .iter()
            .zip(CARD)
            .all(|(&(ty, at, len), want)| FAT_TYPES.contains(&ty) && at > 0 && len == want)
        && parts[0].1 + parts[0].2 <= parts[1].1;
    if !fits {
        return Err(StickFault::CardLayout);
    }
    Ok(Table::Mbr {
        parts: parts.iter().map(|p| p.1).collect(),
    })
}

/// A random GUID in GPT's byte order, version 4 in form (RFC 9562):
/// the version in the third field's high nibble, the variant `10`.
fn guid(random: &mut dyn FnMut(&mut [u8])) -> [u8; 16] {
    let mut g = [0u8; 16];
    random(&mut g);
    g[7] = (g[7] & 0x0F) | 0x40;
    g[8] = (g[8] & 0x3F) | 0x80;
    g
}

/// Four random bytes, not all zero: a zero MBR signature or volume
/// serial reads as none.
fn nonzero(random: &mut dyn FnMut(&mut [u8])) -> [u8; 4] {
    let mut b = [0u8; 4];
    random(&mut b);
    if b == [0; 4] {
        b[0] = 1;
    }
    b
}
