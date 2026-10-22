//! Reading an APK (`docs/APK-SIGNING.md` §5). The file is untrusted
//! input, read in parts through a [`Source`]: the end of the file, the
//! central directory, each entry's local header as the entries pass, and
//! the manifest. Nothing is allocated past the central directory's bound
//! and the manifest's.
//!
//! One reading only. What Faraday shows must be what Android installs, so
//! a ZIP that two readers could take two ways is [`Refusal::NotApk`]:
//! entries that do not lie end to end from offset 0 to the central
//! directory, a central directory out of the order of its entries, a
//! local header that disagrees with its central entry, an encrypted entry,
//! a data descriptor, a ZIP64 field, a name twice.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::axml;

/// The largest central directory read.
pub const MAX_CENTRAL_DIRECTORY: u64 = 16 << 20;
/// The largest manifest inflated.
pub const MAX_MANIFEST: usize = 1 << 20;
/// The magic that ends an APK Signing Block.
pub const BLOCK_MAGIC: &[u8; 16] = b"APK Sig Block 42";

const EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
const CD_SIG: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
const LOCAL_SIG: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];
const ZIP64_LOCATOR_SIG: [u8; 4] = [0x50, 0x4b, 0x06, 0x07];

/// Why an APK is not signed, in the words of `docs/APK-SIGNING.md` §4.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Not a ZIP, or one read two ways, or no manifest read one way.
    NotApk,
    /// Already carries an APK Signing Block or a v1 signature.
    AlreadySigned,
    /// A stored entry's data is not where `zipalign -P 16 4` puts it.
    NotAligned,
    /// `minSdkVersion` is below 24, which would need a v1 signature.
    OldSdk,
    /// The source could not be read.
    Unreadable,
}

/// Where the bytes of a file come from: a whole file in memory, or a
/// stick read in parts.
pub trait Source {
    /// The file's length.
    fn len(&self) -> u64;
    /// Whether the file is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Fills `buf` from offset `at`; false when it cannot.
    fn read_at(&mut self, at: u64, buf: &mut [u8]) -> bool;
}

/// A whole file in memory.
pub struct Slice<'a>(pub &'a [u8]);

impl Source for Slice<'_> {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }

    fn read_at(&mut self, at: u64, buf: &mut [u8]) -> bool {
        let Ok(at) = usize::try_from(at) else {
            return false;
        };
        match at
            .checked_add(buf.len())
            .and_then(|end| self.0.get(at..end))
        {
            Some(src) => {
                buf.copy_from_slice(src);
                true
            }
            None => false,
        }
    }
}

/// What an APK states of itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    /// The manifest's `package`.
    pub package: String,
    /// `versionCode`.
    pub version_code: u32,
    /// `versionCodeMajor`, 0 unless the manifest states one.
    pub version_code_major: u32,
    /// `versionName`, when it has one.
    pub version_name: Option<String>,
    /// `minSdkVersion`, 1 when absent.
    pub min_sdk: u32,
}

/// An APK read whole in one way, ready to sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Apk {
    /// What it states of itself.
    pub info: Info,
    /// The file's length.
    pub size: u64,
    /// Where the central directory starts, which is also where the
    /// entries end.
    pub cd_offset: u64,
    /// The central directory's length.
    pub cd_size: u64,
    /// The end of central directory record with its comment.
    pub eocd: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// One entry of the central directory.
struct Entry {
    name_at: usize,
    name_len: usize,
    flags: u16,
    method: u16,
    crc: u32,
    csize: u32,
    usize: u32,
    lho: u32,
    /// Where its data starts, from the local header read once.
    data_at: u64,
}

/// An extra field is a run of (id, length, data), and ends in zero
/// bytes where `zipalign` padded it without framing them. ZIP64's id is
/// refused; anything else that does not frame is not an extra field.
fn extra_ok(extra: &[u8]) -> bool {
    let mut at = 0;
    while at + 4 <= extra.len() {
        let id = u16_at(extra, at);
        let len = u16_at(extra, at + 2) as usize;
        if id == 0x0001 {
            return false;
        }
        if at + 4 + len > extra.len() {
            break;
        }
        at += 4 + len;
    }
    extra[at..].iter().all(|&b| b == 0)
}

fn read_vec(src: &mut dyn Source, at: u64, n: usize) -> Result<Vec<u8>, Refusal> {
    let mut v = vec![0u8; n];
    if at.checked_add(n as u64).is_none_or(|end| end > src.len()) {
        return Err(Refusal::NotApk);
    }
    if src.read_at(at, &mut v) {
        Ok(v)
    } else {
        Err(Refusal::Unreadable)
    }
}

impl Apk {
    /// Reads and checks an APK. Every rule of `docs/APK-SIGNING.md` §5 is
    /// here; the refusals of §4.4 that need a key (another app's package,
    /// the output name, FAT32's size) are the caller's.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn read(src: &mut dyn Source) -> Result<Apk, Refusal> {
        let size = src.len();
        // The end record: the only one that ends the file with its comment.
        if size < 22 {
            return Err(Refusal::NotApk);
        }
        let tail_len = size.min(22 + 65_535) as usize;
        let tail_at = size - tail_len as u64;
        let tail = read_vec(src, tail_at, tail_len)?;
        let mut found = None;
        for i in (0..=tail_len - 22).rev() {
            if tail[i..i + 4] == EOCD_SIG && i + 22 + u16_at(&tail, i + 20) as usize == tail_len {
                if found.is_some() {
                    return Err(Refusal::NotApk);
                }
                found = Some(i);
            }
        }
        let i = found.ok_or(Refusal::NotApk)?;
        let eocd = tail[i..].to_vec();
        let eocd_off = tail_at + i as u64;
        if u16_at(&eocd, 4) != 0 || u16_at(&eocd, 6) != 0 || u16_at(&eocd, 8) != u16_at(&eocd, 10) {
            return Err(Refusal::NotApk);
        }
        let count = u16_at(&eocd, 10) as usize;
        let cd_size = u64::from(u32_at(&eocd, 12));
        let cd_offset = u64::from(u32_at(&eocd, 16));
        // ZIP64 states itself by the largest values, or by a locator.
        if count == 0xFFFF
            || cd_size == 0xFFFF_FFFF
            || cd_offset == 0xFFFF_FFFF
            || count == 0
            || (eocd_off >= 20 && {
                let loc = read_vec(src, eocd_off - 20, 4)?;
                loc == ZIP64_LOCATOR_SIG
            })
        {
            return Err(Refusal::NotApk);
        }
        // The central directory is directly before the end record.
        if cd_size > MAX_CENTRAL_DIRECTORY || cd_offset.checked_add(cd_size) != Some(eocd_off) {
            return Err(Refusal::NotApk);
        }
        let cd = read_vec(src, cd_offset, cd_size as usize)?;
        let mut entries: Vec<Entry> = Vec::with_capacity(count);
        let mut at = 0usize;
        for _ in 0..count {
            if at + 46 > cd.len() || cd[at..at + 4] != CD_SIG {
                return Err(Refusal::NotApk);
            }
            let name_len = u16_at(&cd, at + 28) as usize;
            let extra_len = u16_at(&cd, at + 30) as usize;
            let comment_len = u16_at(&cd, at + 32) as usize;
            let next = at + 46 + name_len + extra_len + comment_len;
            if next > cd.len()
                || name_len == 0
                || u16_at(&cd, at + 34) != 0
                || !extra_ok(&cd[at + 46 + name_len..at + 46 + name_len + extra_len])
            {
                return Err(Refusal::NotApk);
            }
            let e = Entry {
                name_at: at + 46,
                name_len,
                flags: u16_at(&cd, at + 8),
                method: u16_at(&cd, at + 10),
                crc: u32_at(&cd, at + 16),
                csize: u32_at(&cd, at + 20),
                usize: u32_at(&cd, at + 24),
                lho: u32_at(&cd, at + 42),
                data_at: 0,
            };
            // Plain entries only: stored or deflated, not encrypted, no
            // data descriptor, no ZIP64 sizes.
            if e.flags & 0x0009 != 0
                || !matches!(e.method, 0 | 8)
                || (e.method == 0 && e.csize != e.usize)
                || e.csize == 0xFFFF_FFFF
                || e.usize == 0xFFFF_FFFF
                || e.lho == 0xFFFF_FFFF
            {
                return Err(Refusal::NotApk);
            }
            entries.push(e);
            at = next;
        }
        if at != cd.len() {
            return Err(Refusal::NotApk);
        }
        // Each name once; the entries in the order of their offsets.
        let name = |e: &Entry| &cd[e.name_at..e.name_at + e.name_len];
        let mut names: Vec<&[u8]> = entries.iter().map(name).collect();
        names.sort_unstable();
        if names.windows(2).any(|w| w[0] == w[1]) {
            return Err(Refusal::NotApk);
        }
        // The entries lie end to end from offset 0.
        let mut expect = 0u64;
        for e in entries.iter_mut() {
            if u64::from(e.lho) != expect {
                return Err(Refusal::NotApk);
            }
            let h = read_vec(src, expect, 30)?;
            let n = u16_at(&h, 26) as usize;
            let x = u16_at(&h, 28) as usize;
            if h[..4] != LOCAL_SIG
                || u16_at(&h, 6) != e.flags
                || u16_at(&h, 8) != e.method
                || u32_at(&h, 14) != e.crc
                || u32_at(&h, 18) != e.csize
                || u32_at(&h, 22) != e.usize
                || n != e.name_len
            {
                return Err(Refusal::NotApk);
            }
            let rest = read_vec(src, expect + 30, n + x)?;
            if rest[..n] != *name(e) || !extra_ok(&rest[n..]) {
                return Err(Refusal::NotApk);
            }
            e.data_at = expect + 30 + (n + x) as u64;
            expect += 30 + n as u64 + x as u64 + u64::from(e.csize);
            if expect > cd_offset {
                return Err(Refusal::NotApk);
            }
        }
        // The last entry ends at the central directory, or a signing block
        // sits before it, a page-aligned run of zero bytes after the
        // entries (`apksigner` writes it so).
        let signed = if expect == cd_offset {
            false
        } else {
            if cd_offset < 24 {
                return Err(Refusal::NotApk);
            }
            let end = read_vec(src, cd_offset - 24, 24)?;
            let block = u64::from_le_bytes(end[..8].try_into().unwrap_or_default());
            if end[8..] != BLOCK_MAGIC[..]
                || block < 24
                || !block
                    .checked_add(8)
                    .and_then(|t| cd_offset.checked_sub(t))
                    .is_some_and(|start| start >= expect && start - expect < 4096)
            {
                return Err(Refusal::NotApk);
            }
            let first = read_vec(src, cd_offset - block - 8, 8)?;
            if u64::from_le_bytes(first.try_into().unwrap_or_default()) != block {
                return Err(Refusal::NotApk);
            }
            true
        };
        // The manifest, exactly once, at the root.
        let manifest = entries
            .iter()
            .find(|e| name(e) == b"AndroidManifest.xml")
            .ok_or(Refusal::NotApk)?;
        let xml = inflate(src, manifest)?;
        let m = axml::parse(&xml).ok_or(Refusal::NotApk)?;
        if signed || entries.iter().any(|e| v1_signature(name(e))) {
            return Err(Refusal::AlreadySigned);
        }
        // Stored data at a multiple of 4 bytes, a stored library at one
        // of 16 KiB: `zipalign -c -P 16 4`.
        for e in &entries {
            let n = name(e);
            if e.method != 0 || n.ends_with(b"/") {
                continue;
            }
            let data = e.data_at;
            let want = if n.len() >= 3 && n[n.len() - 3..].eq_ignore_ascii_case(b".so") {
                16_384
            } else {
                4
            };
            if data % want != 0 {
                return Err(Refusal::NotAligned);
            }
        }
        if m.min_sdk < 24 {
            return Err(Refusal::OldSdk);
        }
        Ok(Apk {
            info: Info {
                package: m.package,
                version_code: m.version_code,
                version_code_major: m.version_code_major,
                version_name: m.version_name,
                min_sdk: m.min_sdk,
            },
            size,
            cd_offset,
            cd_size,
            eocd,
        })
    }
}

/// Whether a file is a ZIP whose central directory names
/// `AndroidManifest.xml`: what Files takes for an APK, whatever its name.
/// Nothing else is checked; [`Apk::read`] reads it for real.
pub fn names_manifest(file: &[u8]) -> bool {
    let n = file.len();
    if n < 22 {
        return false;
    }
    let tail = &file[n.saturating_sub(22 + 65_535)..];
    let Some(i) = (0..=tail.len() - 22).rev().find(|&i| {
        tail[i..i + 4] == EOCD_SIG && i + 22 + u16_at(tail, i + 20) as usize == tail.len()
    }) else {
        return false;
    };
    let eocd = &tail[i..];
    let count = u16_at(eocd, 10) as usize;
    let size = u32_at(eocd, 12) as usize;
    let at = u32_at(eocd, 16) as usize;
    let Some(cd) = at.checked_add(size).and_then(|end| file.get(at..end)) else {
        return false;
    };
    let mut p = 0;
    for _ in 0..count {
        if p + 46 > cd.len() || cd[p..p + 4] != CD_SIG {
            return false;
        }
        let name = u16_at(cd, p + 28) as usize;
        let rest = u16_at(cd, p + 30) as usize + u16_at(cd, p + 32) as usize;
        let Some(nm) = cd.get(p + 46..p + 46 + name) else {
            return false;
        };
        if nm == b"AndroidManifest.xml" {
            return true;
        }
        p += 46 + name + rest;
    }
    false
}

/// `META-INF/NAME.SF`, `.RSA`, `.EC` or `.DSA`: a v1 signature.
fn v1_signature(name: &[u8]) -> bool {
    const DIR: &[u8] = b"META-INF/";
    if name.len() <= DIR.len()
        || !name[..DIR.len()].eq_ignore_ascii_case(DIR)
        || name[DIR.len()..].contains(&b'/')
    {
        return false;
    }
    [&b".SF"[..], b".RSA", b".EC", b".DSA"].iter().any(|ext| {
        name.len() >= ext.len() && name[name.len() - ext.len()..].eq_ignore_ascii_case(ext)
    })
}

/// The manifest entry's bytes, inflated to exactly its stated size using
/// exactly its compressed bytes, with the CRC-32 its entry states.
fn inflate(src: &mut dyn Source, e: &Entry) -> Result<Vec<u8>, Refusal> {
    let usize_ = e.usize as usize;
    if usize_ > MAX_MANIFEST {
        return Err(Refusal::NotApk);
    }
    let packed = read_vec(src, e.data_at, e.csize as usize)?;
    let out = if e.method == 0 {
        packed
    } else {
        use miniz_oxide::inflate::TINFLStatus;
        use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};
        let mut out = vec![0u8; usize_];
        let mut state = DecompressorOxide::new();
        let (status, used, wrote) = decompress(
            &mut state,
            &packed,
            &mut out,
            0,
            inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
        );
        if status != TINFLStatus::Done || used != packed.len() || wrote != usize_ {
            return Err(Refusal::NotApk);
        }
        out
    };
    if out.len() != usize_ || faraday_crypto::crc32(&out) != e.crc {
        return Err(Refusal::NotApk);
    }
    Ok(out)
}
