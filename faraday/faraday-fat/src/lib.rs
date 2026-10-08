//! FAT16 and FAT32, read and written in Rust by the unprivileged disk
//! process (`PLAN.md` §4.3, §12.5). The kernel has no FAT driver; this is
//! the only code that parses a stick's filesystem.
//!
//! What it does is the subset Faraday needs and nothing more: the files
//! in a partition's root directory are listed, read, created, renamed and
//! deleted. Subdirectories are listed and never entered. Nothing formats
//! a partition, and nothing edits a file in place: a write is a new file,
//! read back, then renamed over the old one by the caller.
//!
//! Every number on the disk is untrusted. The boot sector is checked
//! before anything else is read, every cluster number is checked against
//! the volume before it is followed, every chain is bounded by the
//! volume's cluster count, and every size is bounded by the caller's
//! limit before memory is allocated for it.
//!
//! Writes go in the order a crash can least hurt: a new file's data,
//! then its clusters in every copy of the FAT, then its directory
//! entries, the short entry last. A crash leaves lost clusters, which a
//! check reclaims, rather than an entry pointing at nothing.
//!
//! FAT12 is refused: no stick is that small.
//!
//! `no_std` + `alloc`.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// What the filesystem is read from and written to: a partition, as
/// bytes at offsets.
pub trait Disk {
    /// The partition's length in bytes.
    fn size(&self) -> u64;
    /// Fills `buf` from `offset`.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error>;
    /// Writes `buf` at `offset`.
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error>;
    /// Puts every write so far on the medium.
    fn flush(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

/// A partition held in memory: tests and the fuzzer.
impl Disk for Vec<u8> {
    fn size(&self) -> u64 {
        self.as_slice().len() as u64
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        let start = usize::try_from(offset).map_err(|_| Error::Io)?;
        let end = start.checked_add(buf.len()).ok_or(Error::Io)?;
        buf.copy_from_slice(self.get(start..end).ok_or(Error::Io)?);
        Ok(())
    }

    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        let start = usize::try_from(offset).map_err(|_| Error::Io)?;
        let end = start.checked_add(buf.len()).ok_or(Error::Io)?;
        self.get_mut(start..end)
            .ok_or(Error::Io)?
            .copy_from_slice(buf);
        Ok(())
    }
}

/// Why an operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The disk could not be read or written there.
    Io,
    /// Not a FAT boot sector, or one whose numbers do not fit together.
    NotFat(&'static str),
    /// A FAT12 volume.
    Fat12,
    /// The filesystem contradicts itself: a cluster out of range, a
    /// chain too short or looping.
    Corrupt(&'static str),
    /// No file by that name.
    NotFound,
    /// A file by that name is already there.
    Exists,
    /// Not enough free clusters.
    Full,
    /// The root directory has no room for the entries (FAT16's is fixed).
    DirFull,
    /// A name FAT cannot hold.
    Name(&'static str),
    /// The file is larger than the caller allows.
    TooLarge,
}

impl Error {
    /// The sentence a screen shows.
    pub fn reason(self) -> &'static str {
        match self {
            Error::Io => "the stick could not be read or written",
            Error::NotFat(why) => why,
            Error::Fat12 => "a FAT12 volume, which Faraday does not read",
            Error::Corrupt(why) => why,
            Error::NotFound => "no such file",
            Error::Exists => "a file by that name is already there",
            Error::Full => "the stick is full",
            Error::DirFull => "the stick's top folder is full",
            Error::Name(why) => why,
            Error::TooLarge => "the file is too large",
        }
    }
}

/// Which FAT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// 16-bit cluster numbers, a fixed root directory.
    Fat16,
    /// 28-bit cluster numbers, the root directory a cluster chain.
    Fat32,
}

/// One entry of the root directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The long name, or the short one when there is none.
    pub name: String,
    /// The size in bytes; 0 for a directory.
    pub size: u32,
    /// A directory, listed and never entered.
    pub dir: bool,
    cluster: u32,
    /// The directory slots it takes, long-name entries first.
    slots: (usize, usize),
    short: [u8; 11],
}

/// The date every entry written here carries: 1980-01-01, FAT's first
/// day. The device keeps no clock it would vouch for.
const DOS_DATE: u16 = (1 << 5) | 1;

const ATTR_RO: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_LABEL: u8 = 0x08;
const ATTR_DIR: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LFN: u8 = ATTR_RO | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_LABEL;

/// The volume label a boot sector states, from its first 512 bytes,
/// without trusting anything else in it: the label of a FAT16 or FAT32
/// extended boot record, by the signature byte that says one is there.
/// What `faraday-grant` reads to keep `OSKBOOT` back (`PLAN.md` §4.3).
pub fn boot_sector_label(sector: &[u8]) -> Option<[u8; 11]> {
    if sector.len() < 512 {
        return None;
    }
    let fat16_size = u16::from_le_bytes([sector[22], sector[23]]);
    let (sig, at) = if fat16_size == 0 { (66, 71) } else { (38, 43) };
    if sector[sig] != 0x29 {
        return None;
    }
    let mut label = [0u8; 11];
    label.copy_from_slice(&sector[at..at + 11]);
    Some(label)
}

/// An open FAT16 or FAT32 volume.
pub struct Volume<D: Disk> {
    disk: D,
    kind: Kind,
    cluster_bytes: u32,
    fat_start: u64,
    fat_bytes: u64,
    fats: u32,
    root_start: u64,
    root_entries: u32,
    data_start: u64,
    clusters: u32,
    root_cluster: u32,
    fsinfo: Option<u64>,
    label: Option<[u8; 11]>,
}

fn u16_at(b: &[u8], at: usize) -> u32 {
    u32::from(u16::from_le_bytes([b[at], b[at + 1]]))
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

impl<D: Disk> Volume<D> {
    /// Reads and checks the boot sector.
    pub fn open(mut disk: D) -> Result<Self, Error> {
        let mut bs = [0u8; 512];
        disk.read_at(0, &mut bs)?;
        if bs[510] != 0x55 || bs[511] != 0xAA {
            return Err(Error::NotFat("no boot sector signature"));
        }
        let bps = u16_at(&bs, 11);
        if !matches!(bps, 512 | 1024 | 2048 | 4096) {
            return Err(Error::NotFat("a sector size FAT does not have"));
        }
        let spc = u32::from(bs[13]);
        if spc == 0 || !spc.is_power_of_two() || spc * bps > 64 * 1024 {
            return Err(Error::NotFat("a cluster size FAT does not have"));
        }
        let reserved = u16_at(&bs, 14);
        let fats = u32::from(bs[16]);
        if reserved == 0 || !(1..=2).contains(&fats) {
            return Err(Error::NotFat("no reserved sectors or no FAT"));
        }
        let root_entries = u16_at(&bs, 17);
        let total = match u16_at(&bs, 19) {
            0 => u32_at(&bs, 32),
            n => n,
        };
        let fat16_size = u16_at(&bs, 22);
        let fat_size = match fat16_size {
            0 => u32_at(&bs, 36),
            n => n,
        };
        if fat_size == 0 {
            return Err(Error::NotFat("a FAT of no size"));
        }
        let root_sectors = (root_entries * 32).div_ceil(bps);
        let meta =
            u64::from(reserved) + u64::from(fats) * u64::from(fat_size) + u64::from(root_sectors);
        let data_sectors = u64::from(total)
            .checked_sub(meta)
            .ok_or(Error::NotFat("the FATs are larger than the volume"))?;
        let clusters = u32::try_from(data_sectors / u64::from(spc))
            .map_err(|_| Error::NotFat("too many clusters"))?;
        // The boot sector says which FAT it is, as Linux reads it: a FAT32
        // boot sector has no 16-bit FAT size and no fixed root. The
        // cluster count decides only between FAT12 and FAT16; tools such
        // as mtools make FAT32 volumes smaller than the specification's
        // 65525 clusters, and every system reads them.
        let kind = if fat16_size == 0 {
            if root_entries != 0 {
                return Err(Error::NotFat("a FAT32 volume with a FAT16 root directory"));
            }
            Kind::Fat32
        } else if clusters < 4085 {
            return Err(Error::Fat12);
        } else {
            if root_entries == 0 {
                return Err(Error::NotFat("a FAT16 volume with no root directory"));
            }
            Kind::Fat16
        };
        if u64::from(total) * u64::from(bps) > disk.size() {
            return Err(Error::NotFat("a volume larger than its partition"));
        }
        let entry = if kind == Kind::Fat16 { 2 } else { 4 };
        let fat_bytes = u64::from(fat_size) * u64::from(bps);
        if fat_bytes < (u64::from(clusters) + 2) * entry {
            return Err(Error::NotFat("a FAT too small for its clusters"));
        }
        let bps = u64::from(bps);
        let fat_start = u64::from(reserved) * bps;
        let root_start = fat_start + u64::from(fats) * fat_bytes;
        let data_start = root_start + u64::from(root_sectors) * bps;
        let (root_cluster, fsinfo) = match kind {
            Kind::Fat16 => (0, None),
            Kind::Fat32 => {
                let at = u16_at(&bs, 48);
                let fsinfo =
                    (at != 0 && at != 0xFFFF && at < reserved).then(|| u64::from(at) * bps);
                (u32_at(&bs, 44) & 0x0FFF_FFFF, fsinfo)
            }
        };
        let mut v = Volume {
            disk,
            kind,
            cluster_bytes: spc * bps as u32,
            fat_start,
            fat_bytes,
            fats,
            root_start,
            root_entries,
            data_start,
            clusters,
            root_cluster,
            fsinfo,
            label: boot_sector_label(&bs),
        };
        if kind == Kind::Fat32 && !v.valid(root_cluster) {
            return Err(Error::NotFat("a root directory outside the volume"));
        }
        // An FSInfo sector without its signatures is not one.
        if let Some(at) = v.fsinfo {
            let mut s = [0u8; 512];
            v.disk.read_at(at, &mut s)?;
            if u32_at(&s, 0) != 0x4161_5252 || u32_at(&s, 484) != 0x6141_7272 {
                v.fsinfo = None;
            }
        }
        Ok(v)
    }

    /// Which FAT this is.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The label in the boot sector, trimmed, when there is one.
    pub fn label(&self) -> Option<String> {
        let l = self.label?;
        let s: String = l.iter().map(|&b| char::from(b)).collect();
        let s = s.trim_end();
        (!s.is_empty() && s != "NO NAME").then(|| String::from(s))
    }

    /// Gives the disk back.
    pub fn into_disk(self) -> D {
        self.disk
    }

    /// Puts every write so far on the medium.
    pub fn flush(&mut self) -> Result<(), Error> {
        self.disk.flush()
    }

    fn valid(&self, c: u32) -> bool {
        c >= 2 && c < self.clusters + 2
    }

    fn eoc(&self) -> u32 {
        match self.kind {
            Kind::Fat16 => 0xFFFF,
            Kind::Fat32 => 0x0FFF_FFFF,
        }
    }

    fn is_end(&self, v: u32) -> bool {
        match self.kind {
            Kind::Fat16 => v >= 0xFFF8,
            Kind::Fat32 => v >= 0x0FFF_FFF8,
        }
    }

    fn fat_entry_at(&self, c: u32) -> u64 {
        let size = if self.kind == Kind::Fat16 { 2 } else { 4 };
        self.fat_start + u64::from(c) * size
    }

    fn fat_get(&mut self, c: u32) -> Result<u32, Error> {
        let at = self.fat_entry_at(c);
        Ok(match self.kind {
            Kind::Fat16 => {
                let mut b = [0u8; 2];
                self.disk.read_at(at, &mut b)?;
                u32::from(u16::from_le_bytes(b))
            }
            Kind::Fat32 => {
                let mut b = [0u8; 4];
                self.disk.read_at(at, &mut b)?;
                u32::from_le_bytes(b) & 0x0FFF_FFFF
            }
        })
    }

    /// Sets a cluster's entry in every copy of the FAT.
    fn fat_set(&mut self, c: u32, v: u32) -> Result<(), Error> {
        for k in 0..u64::from(self.fats) {
            let at = self.fat_entry_at(c) + k * self.fat_bytes;
            match self.kind {
                Kind::Fat16 => self.disk.write_at(at, &(v as u16).to_le_bytes())?,
                Kind::Fat32 => {
                    // The top four bits are reserved and kept.
                    let mut b = [0u8; 4];
                    self.disk.read_at(at, &mut b)?;
                    let top = u32::from_le_bytes(b) & 0xF000_0000;
                    self.disk
                        .write_at(at, &(top | (v & 0x0FFF_FFFF)).to_le_bytes())?;
                }
            }
        }
        Ok(())
    }

    /// The clusters of a chain, in order, bounded by the volume.
    fn chain(&mut self, first: u32) -> Result<Vec<u32>, Error> {
        let mut out = Vec::new();
        let mut c = first;
        loop {
            if !self.valid(c) {
                return Err(Error::Corrupt("a cluster outside the volume"));
            }
            if out.len() as u32 >= self.clusters {
                return Err(Error::Corrupt("a cluster chain that loops"));
            }
            out.push(c);
            let next = self.fat_get(c)?;
            if self.is_end(next) {
                return Ok(out);
            }
            c = next;
        }
    }

    fn cluster_at(&self, c: u32) -> u64 {
        self.data_start + u64::from(c - 2) * u64::from(self.cluster_bytes)
    }

    /// The root directory's clusters (FAT32), or none (FAT16).
    fn root_chain(&mut self) -> Result<Vec<u32>, Error> {
        match self.kind {
            Kind::Fat16 => Ok(Vec::new()),
            Kind::Fat32 => self.chain(self.root_cluster),
        }
    }

    fn slots(&self, chain: &[u32]) -> usize {
        match self.kind {
            Kind::Fat16 => self.root_entries as usize,
            Kind::Fat32 => chain.len() * (self.cluster_bytes as usize / 32),
        }
    }

    fn slot_at(&self, chain: &[u32], i: usize) -> u64 {
        match self.kind {
            Kind::Fat16 => self.root_start + i as u64 * 32,
            Kind::Fat32 => {
                let per = self.cluster_bytes as usize / 32;
                self.cluster_at(chain[i / per]) + (i % per) as u64 * 32
            }
        }
    }

    /// The root directory's bytes, every slot.
    fn root_bytes(&mut self, chain: &[u32]) -> Result<Vec<u8>, Error> {
        match self.kind {
            Kind::Fat16 => {
                let mut b = vec![0u8; self.root_entries as usize * 32];
                self.disk.read_at(self.root_start, &mut b)?;
                Ok(b)
            }
            Kind::Fat32 => {
                let mut b = vec![0u8; chain.len() * self.cluster_bytes as usize];
                for (k, &c) in chain.iter().enumerate() {
                    let at = self.cluster_at(c);
                    let n = self.cluster_bytes as usize;
                    self.disk.read_at(at, &mut b[k * n..(k + 1) * n])?;
                }
                Ok(b)
            }
        }
    }

    /// The root directory's files and directories.
    pub fn list(&mut self) -> Result<Vec<Entry>, Error> {
        let chain = self.root_chain()?;
        let bytes = self.root_bytes(&chain)?;
        Ok(parse_dir(&bytes))
    }

    fn find(&mut self, name: &str) -> Result<Option<Entry>, Error> {
        Ok(self.list()?.into_iter().find(|e| same_name(&e.name, name)))
    }

    /// A file's contents, refused when larger than `max` bytes.
    pub fn read(&mut self, name: &str, max: u32) -> Result<Vec<u8>, Error> {
        let e = self.find(name)?.filter(|e| !e.dir).ok_or(Error::NotFound)?;
        if e.size > max {
            return Err(Error::TooLarge);
        }
        if e.size == 0 {
            return Ok(Vec::new());
        }
        let chain = self.chain(e.cluster)?;
        let size = e.size as usize;
        let cb = self.cluster_bytes as usize;
        if chain.len() < size.div_ceil(cb) {
            return Err(Error::Corrupt("a file longer than its clusters"));
        }
        let mut out = vec![0u8; size];
        for (k, part) in out.chunks_mut(cb).enumerate() {
            let at = self.cluster_at(chain[k]);
            self.disk.read_at(at, part)?;
        }
        Ok(out)
    }

    /// Free clusters, enough for `n`, lowest first.
    fn free_clusters(&mut self, n: usize) -> Result<Vec<u32>, Error> {
        let mut out = Vec::with_capacity(n);
        if n == 0 {
            return Ok(out);
        }
        // The FAT is read whole: one read rather than one per cluster.
        let size = if self.kind == Kind::Fat16 { 2 } else { 4 };
        let len = (u64::from(self.clusters) + 2) * size;
        let mut fat = vec![0u8; len as usize];
        self.disk.read_at(self.fat_start, &mut fat)?;
        for c in 2..self.clusters + 2 {
            let at = c as usize * size as usize;
            let v = if size == 2 {
                u32_at(&[fat[at], fat[at + 1], 0, 0], 0)
            } else {
                u32_at(&fat, at) & 0x0FFF_FFFF
            };
            if v == 0 {
                out.push(c);
                if out.len() == n {
                    return Ok(out);
                }
            }
        }
        Err(Error::Full)
    }

    /// FSInfo's free count and next free cluster, marked unknown: a
    /// count this code kept would be one more number to get wrong.
    fn fsinfo_unknown(&mut self) -> Result<(), Error> {
        if let Some(at) = self.fsinfo {
            self.disk.write_at(at + 488, &[0xFF; 8])?;
        }
        Ok(())
    }

    /// Writes `data` into newly taken clusters, chained. Returns the first
    /// cluster, 0 for an empty file.
    fn store(&mut self, data: &[u8]) -> Result<u32, Error> {
        let cb = self.cluster_bytes as usize;
        let n = data.len().div_ceil(cb);
        let taken = self.free_clusters(n)?;
        // One cluster's buffer for the whole file, written over with zeros
        // once done: what is written may be a secret, and the disk process
        // that writes it outlives the app.
        let mut block = vec![0u8; cb];
        let mut wrote = Ok(());
        for (k, &c) in taken.iter().enumerate() {
            let part = &data[k * cb..data.len().min((k + 1) * cb)];
            block.fill(0);
            block[..part.len()].copy_from_slice(part);
            let at = self.cluster_at(c);
            wrote = self.disk.write_at(at, &block);
            if wrote.is_err() {
                break;
            }
        }
        block.fill(0);
        core::hint::black_box(&block);
        wrote?;
        for (k, &c) in taken.iter().enumerate() {
            let next = taken.get(k + 1).copied().unwrap_or(self.eoc());
            self.fat_set(c, next)?;
        }
        self.fsinfo_unknown()?;
        Ok(taken.first().copied().unwrap_or(0))
    }

    fn free_chain(&mut self, first: u32) -> Result<(), Error> {
        if first == 0 {
            return Ok(());
        }
        for c in self.chain(first)? {
            self.fat_set(c, 0)?;
        }
        self.fsinfo_unknown()
    }

    /// `count` consecutive free slots in the root directory, growing a
    /// FAT32 root by a cluster when there are none.
    fn free_run(&mut self, count: usize) -> Result<(Vec<u32>, usize), Error> {
        for _ in 0..2 {
            let chain = self.root_chain()?;
            let bytes = self.root_bytes(&chain)?;
            let mut run = 0;
            for i in 0..self.slots(&chain) {
                let first = bytes[i * 32];
                if first == 0 || first == 0xE5 {
                    run += 1;
                    if run == count {
                        return Ok((chain, i + 1 - count));
                    }
                } else {
                    run = 0;
                }
            }
            if self.kind == Kind::Fat16 {
                return Err(Error::DirFull);
            }
            // A new, empty cluster at the end of the root's chain.
            let last = *chain.last().ok_or(Error::Corrupt("an empty root"))?;
            let c = *self.free_clusters(1)?.first().ok_or(Error::Full)?;
            let zero = vec![0u8; self.cluster_bytes as usize];
            let at = self.cluster_at(c);
            self.disk.write_at(at, &zero)?;
            self.fat_set(c, self.eoc())?;
            self.fat_set(last, c)?;
            self.fsinfo_unknown()?;
        }
        Err(Error::DirFull)
    }

    /// Writes a name's entries for a file at `cluster` of `size` bytes.
    fn write_entries(&mut self, name: &str, cluster: u32, size: u32) -> Result<(), Error> {
        let taken: Vec<[u8; 11]> = self.list()?.iter().map(|e| e.short).collect();
        let (short, long) = short_name(name, &taken)?;
        let mut entries: Vec<[u8; 32]> = Vec::new();
        if let Some(units) = &long {
            let sum = checksum(&short);
            let parts = units.len().div_ceil(13);
            for p in (0..parts).rev() {
                let mut e = [0u8; 32];
                e[0] = (p as u8 + 1) | if p + 1 == parts { 0x40 } else { 0 };
                e[11] = ATTR_LFN;
                e[13] = sum;
                for (k, at) in LFN_AT.iter().enumerate() {
                    let i = p * 13 + k;
                    let u: u16 = match i.cmp(&units.len()) {
                        core::cmp::Ordering::Less => units[i],
                        core::cmp::Ordering::Equal => 0,
                        core::cmp::Ordering::Greater => 0xFFFF,
                    };
                    e[*at..*at + 2].copy_from_slice(&u.to_le_bytes());
                }
                entries.push(e);
            }
        }
        let mut e = [0u8; 32];
        e[..11].copy_from_slice(&short);
        e[11] = ATTR_ARCHIVE;
        e[16..18].copy_from_slice(&DOS_DATE.to_le_bytes());
        e[18..20].copy_from_slice(&DOS_DATE.to_le_bytes());
        e[20..22].copy_from_slice(&((cluster >> 16) as u16).to_le_bytes());
        e[24..26].copy_from_slice(&DOS_DATE.to_le_bytes());
        e[26..28].copy_from_slice(&(cluster as u16).to_le_bytes());
        e[28..32].copy_from_slice(&size.to_le_bytes());
        entries.push(e);
        let (chain, start) = self.free_run(entries.len())?;
        // The short entry is written last: until it is there, the long
        // name belongs to nothing.
        for (k, e) in entries.iter().enumerate() {
            let at = self.slot_at(&chain, start + k);
            self.disk.write_at(at, e)?;
        }
        Ok(())
    }

    fn erase_entries(&mut self, e: &Entry) -> Result<(), Error> {
        let chain = self.root_chain()?;
        for i in e.slots.0..=e.slots.1 {
            let at = self.slot_at(&chain, i);
            self.disk.write_at(at, &[0xE5])?;
        }
        Ok(())
    }

    /// A new file. Refused when the name is taken.
    pub fn create(&mut self, name: &str, data: &[u8]) -> Result<(), Error> {
        check_name(name)?;
        let size = u32::try_from(data.len()).map_err(|_| Error::TooLarge)?;
        if self.find(name)?.is_some() {
            return Err(Error::Exists);
        }
        let first = self.store(data)?;
        if let Err(e) = self.write_entries(name, first, size) {
            // The clusters go back: no entry points at them.
            self.free_chain(first)?;
            return Err(e);
        }
        Ok(())
    }

    /// Gives a file another name. Refused when the new name is taken.
    pub fn rename(&mut self, from: &str, to: &str) -> Result<(), Error> {
        check_name(to)?;
        let e = self.find(from)?.filter(|e| !e.dir).ok_or(Error::NotFound)?;
        if !same_name(from, to) && self.find(to)?.is_some() {
            return Err(Error::Exists);
        }
        // The old entries go first, so two names never share one chain.
        self.erase_entries(&e)?;
        self.write_entries(to, e.cluster, e.size)
    }

    /// Removes a file and frees its clusters.
    pub fn delete(&mut self, name: &str) -> Result<(), Error> {
        let e = self.find(name)?.filter(|e| !e.dir).ok_or(Error::NotFound)?;
        self.erase_entries(&e)?;
        self.free_chain(e.cluster)
    }
}

/// Where a long-name entry keeps its thirteen UTF-16 units.
const LFN_AT: [usize; 13] = [1, 3, 5, 7, 9, 14, 16, 18, 20, 22, 24, 28, 30];

fn checksum(short: &[u8; 11]) -> u8 {
    short.iter().fold(0u8, |s, &b| {
        ((s & 1) << 7).wrapping_add(s >> 1).wrapping_add(b)
    })
}

/// FAT names compare without regard to ASCII case.
fn same_name(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// A long name being gathered: its checksum, its parts, its first slot.
type Gathering = (u8, Vec<Option<[u16; 13]>>, usize);

/// The root directory's entries from its bytes.
fn parse_dir(bytes: &[u8]) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut long: Option<Gathering> = None;
    for (i, e) in bytes.chunks_exact(32).enumerate() {
        match e[0] {
            0 => break,
            0xE5 => {
                long = None;
                continue;
            }
            _ => {}
        }
        if e[11] & 0x3F == ATTR_LFN {
            let ord = usize::from(e[0] & 0x1F);
            if ord == 0 {
                long = None;
                continue;
            }
            if e[0] & 0x40 != 0 {
                long = Some((e[13], vec![None; ord], i));
            }
            if let Some((sum, parts, _)) = long.as_mut()
                && *sum == e[13]
                && ord <= parts.len()
            {
                let mut units = [0u16; 13];
                for (k, at) in LFN_AT.iter().enumerate() {
                    units[k] = u16::from_le_bytes([e[*at], e[*at + 1]]);
                }
                parts[ord - 1] = Some(units);
            } else {
                long = None;
            }
            continue;
        }
        let gathered = long.take();
        if e[11] & ATTR_LABEL != 0 {
            continue;
        }
        let mut short = [0u8; 11];
        short.copy_from_slice(&e[..11]);
        if short[0] == 0x05 {
            short[0] = 0xE5;
        }
        if &short == b".          " || &short == b"..         " {
            continue;
        }
        let from_long = gathered.and_then(|(sum, parts, start)| {
            if sum
                != checksum(&{
                    let mut s = short;
                    if s[0] == 0xE5 {
                        s[0] = 0x05;
                    }
                    s
                })
            {
                return None;
            }
            let mut units = Vec::new();
            for p in parts {
                units.extend_from_slice(&p?);
            }
            let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
            let name: String = char::decode_utf16(units[..end].iter().copied())
                .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect();
            (!name.is_empty()).then_some((name, start))
        });
        let (name, start) = match from_long {
            Some(x) => x,
            None => (short_text(&short, e[12]), i),
        };
        let cluster = (u32::from(u16::from_le_bytes([e[20], e[21]])) << 16)
            | u32::from(u16::from_le_bytes([e[26], e[27]]));
        let dir = e[11] & ATTR_DIR != 0;
        out.push(Entry {
            name,
            size: if dir { 0 } else { u32_at(e, 28) },
            dir,
            cluster,
            slots: (start, i),
            short: {
                let mut s = [0u8; 11];
                s.copy_from_slice(&e[..11]);
                s
            },
        });
    }
    out
}

/// A short name as text, with the lower-case flags Windows sets in
/// byte 12 applied.
fn short_text(short: &[u8; 11], flags: u8) -> String {
    let part = |b: &[u8], lower: bool| -> String {
        b.iter()
            .map(|&c| {
                let c = char::from(c);
                if lower { c.to_ascii_lowercase() } else { c }
            })
            .collect::<String>()
            .trim_end()
            .into()
    };
    let base = part(&short[..8], flags & 0x08 != 0);
    let ext = part(&short[8..], flags & 0x10 != 0);
    if ext.is_empty() {
        base
    } else {
        alloc::format!("{base}.{ext}")
    }
}

/// Whether FAT can hold the name as a long name.
fn check_name(name: &str) -> Result<(), Error> {
    if name.is_empty() || name == "." || name == ".." {
        return Err(Error::Name("an empty name"));
    }
    if name.encode_utf16().count() > 255 {
        return Err(Error::Name("a name longer than 255 characters"));
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Err(Error::Name("a name ending in a space or a dot"));
    }
    if name
        .chars()
        .any(|c| (c as u32) < 0x20 || "\"*/:<>?\\|".contains(c))
    {
        return Err(Error::Name("a name with a character FAT does not allow"));
    }
    Ok(())
}

fn short_char(c: char) -> Option<u8> {
    let c = c.to_ascii_uppercase();
    (c.is_ascii_alphanumeric() || "!#$%&'()-@^_`{}~".contains(c)).then_some(c as u8)
}

/// The short name for `name`, unique among `taken`, and the long name's
/// UTF-16 units when one is needed. A name that is already an upper-case
/// 8.3 name is written as it is, with no long name.
fn short_name(name: &str, taken: &[[u8; 11]]) -> Result<([u8; 11], Option<Vec<u16>>), Error> {
    let (base, ext) = match name.rfind('.') {
        Some(0) | None => (name, ""),
        Some(i) => (&name[..i], &name[i + 1..]),
    };
    let exact = (1..=8).contains(&base.len())
        && ext.len() <= 3
        && base
            .chars()
            .chain(ext.chars())
            .all(|c| short_char(c).is_some_and(|b| char::from(b) == c));
    let pad = |s: &[u8], n: usize| {
        let mut v = s.to_vec();
        v.resize(n, b' ');
        v
    };
    if exact {
        let mut s = [b' '; 11];
        s[..8].copy_from_slice(&pad(base.as_bytes(), 8));
        s[8..].copy_from_slice(&pad(ext.as_bytes(), 3));
        if taken.contains(&s) {
            return Err(Error::Exists);
        }
        return Ok((s, None));
    }
    let clean = |s: &str| -> Vec<u8> {
        s.chars()
            .filter(|c| *c != ' ' && *c != '.')
            .map(|c| short_char(c).unwrap_or(b'_'))
            .collect()
    };
    let mut b = clean(base.trim_start_matches('.'));
    if b.is_empty() {
        b = b"FILE".to_vec();
    }
    let mut e = clean(ext);
    e.truncate(3);
    let e = pad(&e, 3);
    let long = Some(name.encode_utf16().collect());
    // The upper-case form itself, when it fits and is free.
    if b.len() <= 8 {
        let mut s = [b' '; 11];
        s[..8].copy_from_slice(&pad(&b, 8));
        s[8..].copy_from_slice(&e);
        if !taken.contains(&s) {
            return Ok((s, long));
        }
    }
    for n in 1u32..1_000_000 {
        let tail = alloc::format!("~{n}");
        let keep = 8 - tail.len();
        let mut s = [b' '; 11];
        let mut head = b[..b.len().min(keep)].to_vec();
        head.extend_from_slice(tail.as_bytes());
        s[..8].copy_from_slice(&pad(&head, 8));
        s[8..].copy_from_slice(&e);
        if !taken.contains(&s) {
            return Ok((s, long));
        }
    }
    Err(Error::DirFull)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_are_unique_and_upper_case() {
        let (s, l) = short_name("vault.ofv", &[]).unwrap();
        assert_eq!(&s, b"VAULT   OFV");
        assert!(l.is_some());
        let (s, l) = short_name("README.TXT", &[]).unwrap();
        assert_eq!(&s, b"README  TXT");
        assert!(l.is_none());
        let (s, _) = short_name("savings-unsigned.psbt", &[]).unwrap();
        assert_eq!(&s, b"SAVING~1PSB");
        let taken = [*b"SAVING~1PSB"];
        let (s, _) = short_name("savings-unsigned.psbt", &taken).unwrap();
        assert_eq!(&s, b"SAVING~2PSB");
    }
}
