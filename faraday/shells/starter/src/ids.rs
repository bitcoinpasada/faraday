//! Each stick's and card's own IDs (`docs/STICK-IDENTITY.md` §4, F174).
//!
//! The image keeps fixed IDs, so a build is reproducible (§4.1). When the
//! Starter writes a whole stick it gives it its own: a new disk GUID and
//! new partition GUIDs in the primary and the backup GPT, with both
//! headers' and both entry arrays' CRCs worked out again; on a Pi card a
//! new MBR disk signature; and a new serial for each FAT volume, in its
//! boot sector and, on FAT32, in the backup boot sector it names (§4.2).
//! Keep data writes no partition table, and gives the new boot partition
//! the serial the stick's had (§4.4).
//!
//! What is changed is a list of [`Patch`]es on the source's stream: the
//! writer writes and compares the stream with them in place, and takes
//! them back out of what it read back to check the image's SHA-256
//! (§4.3). [`Capture`] gathers, as the source streams past once, the few
//! sectors they are worked out from.

use std::io::{self, Write};

use faraday_crypto::crc32;
use faraday_starter::api::StickFault;

use crate::gpt;

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
const CARD: [u64; 2] = [gpt::BOOT_LEN, gpt::DATA_LEN];

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

    /// The fresh IDs of a whole stick or card (§4.2), drawn from
    /// `random`: a new disk GUID and partition GUIDs, or a new MBR disk
    /// signature, and a new serial for each FAT volume.
    pub fn fresh(&self, random: &mut dyn FnMut(&mut [u8])) -> Result<Vec<Patch>, StickFault> {
        let mut out = Vec::new();
        let table = self.done()?;
        match table {
            Table::Gpt { .. } => self.fresh_gpt(table, random, &mut out)?,
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

    /// Keep data's one change (§4.4): the boot partition's serial put
    /// back to `serial`, the one the stick's boot partition had.
    pub fn boot_serial(&self, serial: [u8; 4]) -> Result<Vec<Patch>, StickFault> {
        let table = self.done()?;
        let first = *parts(table).first().ok_or(StickFault::Partitions(0))?;
        self.serial(first, serial)
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

    fn fresh_gpt(
        &self,
        table: &Table,
        random: &mut dyn FnMut(&mut [u8]),
        out: &mut Vec<Patch>,
    ) -> Result<(), StickFault> {
        let Table::Gpt {
            header,
            header_len,
            entries,
            entries_len,
            backup_entries,
            backup_header,
            ..
        } = *table
        else {
            return Ok(());
        };
        let primary = self.got(header, header_len)?.to_vec();
        let list = self.got(entries, entries_len)?.to_vec();
        let backup = self.got(backup_header, header_len)?.to_vec();
        let backup_list = self.got(backup_entries, entries_len)?.to_vec();
        // The backup must be the primary's: the same entries, the same
        // disk GUID, a header whose CRC holds and that names its own
        // entries where they are.
        let crc_holds = |h: &[u8]| {
            let mut c = h.to_vec();
            c[16..20].fill(0);
            crc32(&c) == le32(h, 16)
        };
        if backup_list != list
            || &backup[..8] != b"EFI PART"
            || backup[56..72] != primary[56..72]
            || !crc_holds(&backup)
            || le64(&backup, 72).checked_mul(self.sector()?) != Some(backup_entries)
            || le32(&backup, 88) != le32(&primary, 88)
        {
            return Err(StickFault::BackupTable);
        }
        let disk = guid(random);
        let entry_size = le32(&primary, 84) as usize;
        let mut new_list = list.clone();
        for e in new_list.chunks_exact_mut(entry_size) {
            if e[..16].iter().any(|&b| b != 0) {
                e[16..32].copy_from_slice(&guid(random));
            }
        }
        let list_crc = crc32(&new_list).to_le_bytes();
        let rewrite = |h: &[u8]| {
            let mut h = h.to_vec();
            h[56..72].copy_from_slice(&disk);
            h[88..92].copy_from_slice(&list_crc);
            h[16..20].fill(0);
            let crc = crc32(&h).to_le_bytes();
            h[16..20].copy_from_slice(&crc);
            h
        };
        out.push(Patch {
            at: header,
            bytes: rewrite(&primary),
            was: primary.clone(),
        });
        out.push(Patch {
            at: entries,
            bytes: new_list.clone(),
            was: list,
        });
        out.push(Patch {
            at: backup_entries,
            bytes: new_list,
            was: backup_list,
        });
        out.push(Patch {
            at: backup_header,
            bytes: rewrite(&backup),
            was: backup,
        });
        Ok(())
    }

    fn sector(&self) -> Result<u64, StickFault> {
        match &self.table {
            Some(Table::Gpt { sector, .. }) => Ok(*sector),
            _ => Ok(512),
        }
    }

    /// The patches that set the serial of the FAT volume at `part` to
    /// `serial`: in its boot sector and, on FAT32, in its backup boot
    /// sector (Microsoft's FAT specification, `BS_VolID`, `BPB_BkBootSec`).
    fn serial(&self, part: u64, serial: [u8; 4]) -> Result<Vec<Patch>, StickFault> {
        let sector = self.got(part, 512)?;
        let old = faraday_fat::boot_sector_serial(sector).ok_or(StickFault::NoSerial)?;
        if sector[510..512] != [0x55, 0xAA] {
            return Err(StickFault::NoSerial);
        }
        let fat32 = u16::from_le_bytes([sector[22], sector[23]]) == 0;
        let at = if fat32 { 67 } else { 39 };
        let mut places = vec![part + at];
        if fat32 {
            let backup = u64::from(u16::from_le_bytes([sector[50], sector[51]]));
            let bytes = u64::from(u16::from_le_bytes([sector[11], sector[12]]));
            if backup != 0 {
                let b = part + backup * bytes;
                let copy = self.got(b, 512).map_err(|_| StickFault::NoSerial)?;
                if faraday_fat::boot_sector_serial(copy) != Some(old) {
                    return Err(StickFault::NoSerial);
                }
                places.push(b + at);
            }
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
        if self.table.is_none() && self.fault.is_none() {
            // The head first, and the table read from it, before any
            // byte past it: what else is wanted depends on the table.
            let head_end = self.wanted[0].1.len() as u64;
            let take = (head_end.saturating_sub(self.pos) as usize).min(data.len());
            self.feed(&data[..take]);
            data = &data[take..];
            if self.pos >= head_end
                && let Err(f) = self.plan()
            {
                self.fault = Some(f);
            }
        }
        self.feed(data);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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
    let header = &head[t.sector as usize..];
    let header_len = le32(header, 12) as usize;
    let entries = le64(header, 72) * t.sector;
    let entries_len = le32(header, 80) as usize * le32(header, 84) as usize;
    let alternate = le64(header, 32);
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

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn le64(b: &[u8], at: usize) -> u64 {
    u64::from(le32(b, at)) | (u64::from(le32(b, at + 4)) << 32)
}
