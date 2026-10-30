//! Faraday sticks and Pi cards at their real size, for the writer's
//! tests: the layout of `docs/STICK-IDENTITY.md` §5.1 (F80), held sparse
//! so a test keeps only the sectors it sets, and written to a sparse
//! file as a disk.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const SECTOR: u64 = 512;
/// The boot partition's first sector, and its sectors (64 MiB).
pub const BOOT_FIRST: u64 = 2048;
pub const BOOT_SECTORS: u64 = 131_072;
/// The data partition's first sector, and its sectors (128 MiB).
pub const DATA_FIRST: u64 = BOOT_FIRST + BOOT_SECTORS;
pub const DATA_SECTORS: u64 = 262_144;
/// A stick image: 193 MiB and the backup GPT's 20 KiB, as genimage
/// makes it.
pub const IMAGE: u64 = 202_395_648;
/// A stick to write it to: 256 MB.
pub const STICK: u64 = 256 << 20;
/// The image's IDs (`docs/STICK-IDENTITY.md` §4.1): its disk GUID and
/// its two partitions'.
pub const IDS: [&str; 3] = [
    "0E6B1D27-5C94-4A3E-8F21-6D0C9B7A4E10",
    "0E6B1D27-5C94-4A3E-8F21-6D0C9B7A4E11",
    "0E6B1D27-5C94-4A3E-8F21-6D0C9B7A4E12",
];
/// An ordinary stick's partition names (§3).
pub const NAMES: [&str; 2] = ["EFI system partition", "Basic data partition"];
pub const ESP: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
pub const BASIC: &str = "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7";
/// The image's FAT serials: the boot partition's and the data's.
pub const SERIALS: [u32; 2] = [0x6E1F_2A01, 0x6E1F_2A02];
/// Where, in the data partition, the person's files stand in: a vault
/// at its start and one near its end.
pub const FILES: [u64; 2] = [64 << 10, (120 << 20) + 7];

/// A disk or image of `len` bytes, zero but for the pieces put in it, a
/// later piece over an earlier one.
#[derive(Debug, Clone)]
pub struct Sparse {
    pub len: u64,
    pub pieces: Vec<(u64, Vec<u8>)>,
}

impl Sparse {
    pub fn new(len: u64) -> Sparse {
        Sparse {
            len,
            pieces: Vec::new(),
        }
    }

    pub fn put(&mut self, at: u64, bytes: &[u8]) {
        assert!(at + bytes.len() as u64 <= self.len, "a piece past the end");
        self.pieces.push((at, bytes.to_vec()));
    }

    /// The bytes at `at`, `buf.len()` of them.
    pub fn read_at(&self, at: u64, buf: &mut [u8]) {
        buf.fill(0);
        let end = at + buf.len() as u64;
        for (from, bytes) in &self.pieces {
            let to = from + bytes.len() as u64;
            let (a, b) = (at.max(*from), end.min(to));
            if a < b {
                buf[(a - at) as usize..(b - at) as usize]
                    .copy_from_slice(&bytes[(a - from) as usize..(b - from) as usize]);
            }
        }
    }

    pub fn bytes(&self, at: u64, len: usize) -> Vec<u8> {
        let mut v = vec![0; len];
        self.read_at(at, &mut v);
        v
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.bytes(0, self.len as usize)
    }

    /// Written to `path` as a sparse file of its length.
    pub fn write(&self, path: &Path) {
        let mut f = File::create(path).expect("a file");
        f.set_len(self.len).expect("its length");
        for (at, bytes) in &self.pieces {
            f.seek(SeekFrom::Start(*at)).expect("seek");
            f.write_all(bytes).expect("written");
        }
    }
}

/// A GUID string as GPT stores it.
pub fn guid_bytes(s: &str) -> [u8; 16] {
    let hex: Vec<u8> = s
        .bytes()
        .filter(|&c| c != b'-')
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).expect("hex"), 16).expect("hex"))
        .collect();
    let mut b = [0u8; 16];
    b.copy_from_slice(&hex);
    b[..4].reverse();
    b[4..6].reverse();
    b[6..8].reverse();
    b
}

/// One partition of a GPT: its type, its GUID, its first and last
/// sectors, its name.
pub type Part<'a> = (&'a str, &'a str, u64, u64, &'a str);

/// A GPT on a disk of `len` bytes, its backup at the end.
pub fn gpt(s: &mut Sparse, disk: &str, parts: &[Part<'_>]) {
    let mut entries = vec![0u8; 128 * 128];
    for (i, (ty, id, first, last, name)) in parts.iter().enumerate() {
        let e = &mut entries[i * 128..(i + 1) * 128];
        e[..16].copy_from_slice(&guid_bytes(ty));
        e[16..32].copy_from_slice(&guid_bytes(id));
        e[32..40].copy_from_slice(&first.to_le_bytes());
        e[40..48].copy_from_slice(&last.to_le_bytes());
        for (k, u) in name.encode_utf16().enumerate() {
            e[56 + 2 * k..58 + 2 * k].copy_from_slice(&u.to_le_bytes());
        }
    }
    let last = s.len / SECTOR - 1;
    let header = |current: u64, backup: u64, entries_lba: u64| {
        let mut h = vec![0u8; 92];
        h[..8].copy_from_slice(b"EFI PART");
        h[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
        h[12..16].copy_from_slice(&92u32.to_le_bytes());
        h[24..32].copy_from_slice(&current.to_le_bytes());
        h[32..40].copy_from_slice(&backup.to_le_bytes());
        h[40..48].copy_from_slice(&34u64.to_le_bytes());
        h[48..56].copy_from_slice(&(last - 33).to_le_bytes());
        h[56..72].copy_from_slice(&guid_bytes(disk));
        h[72..80].copy_from_slice(&entries_lba.to_le_bytes());
        h[80..84].copy_from_slice(&128u32.to_le_bytes());
        h[84..88].copy_from_slice(&128u32.to_le_bytes());
        h[88..92].copy_from_slice(&faraday_crypto::crc32(&entries).to_le_bytes());
        let crc = faraday_crypto::crc32(&h);
        h[16..20].copy_from_slice(&crc.to_le_bytes());
        h
    };
    // The protective MBR.
    let mut mbr = vec![0u8; 512];
    mbr[446 + 4] = 0xEE;
    mbr[446 + 8..446 + 12].copy_from_slice(&1u32.to_le_bytes());
    mbr[446 + 12..446 + 16]
        .copy_from_slice(&(u32::try_from(last).unwrap_or(u32::MAX)).to_le_bytes());
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    s.put(0, &mbr);
    s.put(SECTOR, &header(1, last, 2));
    s.put(2 * SECTOR, &entries);
    s.put((last - 32) * SECTOR, &entries);
    s.put(last * SECTOR, &header(last, 1, last - 32));
}

/// A FAT boot sector as `mkfs.fat` writes one, of a volume of `sectors`
/// sectors: FAT32 with its backup at sector 6, or FAT16; `label` and
/// `serial` its own.
pub fn boot_sector(fat32: bool, sectors: u32, label: &[u8; 11], serial: u32) -> Vec<u8> {
    let mut v = vec![0u8; 512];
    v[0..3].copy_from_slice(&[0xEB, 0x58, 0x90]);
    v[3..11].copy_from_slice(b"mkfs.fat");
    v[11..13].copy_from_slice(&512u16.to_le_bytes());
    v[16] = 2;
    v[21] = 0xF8;
    v[32..36].copy_from_slice(&sectors.to_le_bytes());
    if fat32 {
        v[13] = 1;
        v[14..16].copy_from_slice(&32u16.to_le_bytes());
        v[36..40].copy_from_slice(&1025u32.to_le_bytes());
        v[44..48].copy_from_slice(&2u32.to_le_bytes());
        v[48..50].copy_from_slice(&1u16.to_le_bytes());
        v[50..52].copy_from_slice(&6u16.to_le_bytes());
        v[64] = 0x80;
        v[66] = 0x29;
        v[67..71].copy_from_slice(&serial.to_le_bytes());
        v[71..82].copy_from_slice(label);
        v[82..90].copy_from_slice(b"FAT32   ");
    } else {
        v[13] = 4;
        v[14..16].copy_from_slice(&4u16.to_le_bytes());
        v[17..19].copy_from_slice(&512u16.to_le_bytes());
        v[22..24].copy_from_slice(&128u16.to_le_bytes());
        v[36] = 0x80;
        v[38] = 0x29;
        v[39..43].copy_from_slice(&serial.to_le_bytes());
        v[43..54].copy_from_slice(label);
        v[54..62].copy_from_slice(b"FAT16   ");
    }
    v[510] = 0x55;
    v[511] = 0xAA;
    v
}

/// A FAT volume's start at `at`: its boot sector, and on FAT32 the
/// backup at sector 6.
pub fn fat(s: &mut Sparse, at: u64, fat32: bool, sectors: u32, label: &[u8; 11], serial: u32) {
    let sector = boot_sector(fat32, sectors, label, serial);
    s.put(at, &sector);
    if fat32 {
        s.put(at + 6 * SECTOR, &sector);
    }
}

/// A stick of `len` bytes with Faraday's layout, the given IDs and
/// partition names: a FAT32 boot partition labelled EFI and a FAT32
/// data partition with no label, holding `files` where [`FILES`] says.
pub fn stick_with(len: u64, ids: [&str; 3], names: [&str; 2], files: u8) -> Sparse {
    let mut s = Sparse::new(len);
    gpt(
        &mut s,
        ids[0],
        &[
            (ESP, ids[1], BOOT_FIRST, DATA_FIRST - 1, names[0]),
            (
                BASIC,
                ids[2],
                DATA_FIRST,
                DATA_FIRST + DATA_SECTORS - 1,
                names[1],
            ),
        ],
    );
    let boot = BOOT_FIRST * SECTOR;
    fat(
        &mut s,
        boot,
        true,
        BOOT_SECTORS as u32,
        b"EFI        ",
        SERIALS[0],
    );
    let data = DATA_FIRST * SECTOR;
    fat(
        &mut s,
        data,
        true,
        DATA_SECTORS as u32,
        b"NO NAME    ",
        SERIALS[1],
    );
    if files != 0 {
        for at in FILES {
            s.put(data + at, &[files; 4096]);
        }
    }
    s
}

/// The release's stick image, its data partition empty.
pub fn stick_image() -> Sparse {
    stick_with(IMAGE, IDS, NAMES, 0)
}

/// A stick of [`STICK`] bytes written from an earlier image, holding
/// the person's files.
pub fn stick() -> Sparse {
    stick_with(STICK, IDS, NAMES, 0xDA)
}

/// The boot partition's range, in bytes.
pub fn boot_range() -> (u64, u64) {
    (BOOT_FIRST * SECTOR, DATA_FIRST * SECTOR)
}

/// A Pi card image (`faraday/image/boards/pi3/genimage.cfg`): an MBR
/// with signature `signature`, a FAT16 boot partition of 64 MiB at
/// 4 MiB and a FAT32 data partition of 128 MiB after it.
pub fn card_image(signature: u32) -> Sparse {
    let mut s = Sparse::new(196 << 20);
    let mut mbr = vec![0u8; 512];
    mbr[440..444].copy_from_slice(&signature.to_le_bytes());
    for (i, (first, count)) in [(8192u32, 131_072u32), (139_264, 262_144)]
        .iter()
        .enumerate()
    {
        let e = 446 + 16 * i;
        mbr[e + 4] = 0x0C;
        mbr[e + 8..e + 12].copy_from_slice(&first.to_le_bytes());
        mbr[e + 12..e + 16].copy_from_slice(&count.to_le_bytes());
    }
    mbr[446] = 0x80;
    mbr[510] = 0x55;
    mbr[511] = 0xAA;
    s.put(0, &mbr);
    fat(&mut s, 4 << 20, false, 131_072, b"bootfs     ", SERIALS[0]);
    fat(&mut s, 68 << 20, true, 262_144, b"NO NAME    ", SERIALS[1]);
    s
}

/// Reads `len` bytes of the file at `path` from `at`.
pub fn read(path: &Path, at: u64, len: usize) -> Vec<u8> {
    let mut f = File::open(path).expect("open");
    f.seek(SeekFrom::Start(at)).expect("seek");
    let mut v = vec![0; len];
    f.read_exact(&mut v).expect("read");
    v
}

/// Every byte of the file at `path` outside `[from, to)` is what
/// `before` holds there, and the file is `before`'s length.
pub fn untouched_outside(before: &Sparse, path: &Path, from: u64, to: u64) {
    let mut f = OpenOptions::new().read(true).open(path).expect("open");
    assert_eq!(f.metadata().expect("metadata").len(), before.len);
    let chunk = 4 << 20;
    let mut at = 0u64;
    let mut buf = vec![0u8; chunk];
    let mut want = vec![0u8; chunk];
    while at < before.len {
        let n = (chunk as u64).min(before.len - at) as usize;
        f.seek(SeekFrom::Start(at)).expect("seek");
        f.read_exact(&mut buf[..n]).expect("read");
        before.read_at(at, &mut want[..n]);
        // The part of this chunk inside [from, to) is not compared.
        let end = at + n as u64;
        let (a, b) = (from.clamp(at, end), to.clamp(at, end));
        let (a, b) = if a < b {
            ((a - at) as usize, (b - at) as usize)
        } else {
            (n, n)
        };
        for (x, y) in [(0, a), (b, n)] {
            if buf[x..y] == want[x..y] {
                continue;
            }
            if let Some(i) = (x..y).find(|&i| buf[i] != want[i]) {
                panic!("byte {} changed, outside {from}..{to}", at + i as u64);
            }
        }
        at += n as u64;
    }
}

/// The file at `path` is `before`, every byte.
pub fn unchanged(before: &Sparse, path: &Path) {
    untouched_outside(before, path, 0, 0);
}

/// `data` in an `.xz` whose LZMA2 chunks are stored, not compressed, as
/// the xz format allows: what a release's payload file is, made here
/// without an encoder.
pub fn stored_xz(data: &[u8]) -> Vec<u8> {
    let crc = faraday_crypto::crc32;
    let varint = |out: &mut Vec<u8>, mut n: u64| {
        while n >= 0x80 {
            out.push((n as u8) | 0x80);
            n >>= 7;
        }
        out.push(n as u8);
    };
    let mut x = vec![0xFD, b'7', b'z', b'X', b'Z', 0, 0, 0];
    x.extend_from_slice(&crc(&[0, 0]).to_le_bytes());
    // The block: its header (LZMA2, a 4 MiB dictionary), the chunks.
    let mut header = vec![2u8, 0x00, 0x21, 0x01, 0x14, 0, 0, 0];
    header.extend_from_slice(&crc(&header).to_le_bytes());
    x.extend_from_slice(&header);
    let mut body = 0u64;
    for (k, chunk) in data.chunks(1 << 16).enumerate() {
        x.push(if k == 0 { 0x01 } else { 0x02 });
        x.extend_from_slice(&((chunk.len() - 1) as u16).to_be_bytes());
        x.extend_from_slice(chunk);
        body += 3 + chunk.len() as u64;
    }
    x.push(0x00);
    body += 1;
    while !x.len().is_multiple_of(4) {
        x.push(0);
    }
    let mut index = vec![0u8];
    varint(&mut index, 1);
    varint(&mut index, header.len() as u64 + body);
    varint(&mut index, data.len() as u64);
    while !index.len().is_multiple_of(4) {
        index.push(0);
    }
    let sum = crc(&index);
    index.extend_from_slice(&sum.to_le_bytes());
    x.extend_from_slice(&index);
    let mut footer = ((index.len() / 4 - 1) as u32).to_le_bytes().to_vec();
    footer.extend_from_slice(&[0, 0]);
    x.extend_from_slice(&crc(&footer).to_le_bytes());
    x.extend_from_slice(&footer);
    x.extend_from_slice(b"YZ");
    x
}

/// Where a stick image holds its IDs (`docs/STICK-IDENTITY.md` §4.2): the
/// primary GPT's header and entries, the backup's entries and header, and
/// the serial of each FAT volume in its boot sector and backup.
pub fn stick_id_places(len: u64) -> Vec<(u64, u64)> {
    let last = len / SECTOR - 1;
    let mut v = vec![
        (SECTOR, SECTOR + 92),
        (2 * SECTOR, 34 * SECTOR),
        ((last - 32) * SECTOR, last * SECTOR + 92),
    ];
    for part in [BOOT_FIRST * SECTOR, DATA_FIRST * SECTOR] {
        v.push((part + 67, part + 71));
        v.push((part + 6 * SECTOR + 67, part + 6 * SECTOR + 71));
    }
    v
}

/// The same for a Pi card image: its MBR's signature and its volumes'
/// serials.
pub fn card_id_places() -> Vec<(u64, u64)> {
    let boot = 4u64 << 20;
    let data = 68u64 << 20;
    vec![
        (440, 444),
        (boot + 39, boot + 43),
        (data + 67, data + 71),
        (data + 6 * SECTOR + 67, data + 6 * SECTOR + 71),
    ]
}

/// `read`, the first bytes of a disk written from `image`, with the
/// image's own bytes put back at `places`: what a person does by hand to
/// check a stick against the manifest (§4.3).
pub fn ids_put_back(read: &mut [u8], image: &Sparse, places: &[(u64, u64)]) {
    for &(from, to) in places {
        let (a, b) = (from as usize, to as usize);
        read[a..b].copy_from_slice(&image.bytes(from, b - a));
    }
}

/// The SHA-256 of `bytes`, hex.
pub fn sha(bytes: &[u8]) -> String {
    let s = faraday_payload::hash_image(
        std::io::Cursor::new(bytes),
        faraday_payload::ImageKind::Plain,
    )
    .expect("hashed");
    s.sha256.to_string()
}
