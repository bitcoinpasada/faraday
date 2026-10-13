//! The received files' store: the Companion's own byte format, written
//! whole after each change and handed to the shell to keep
//! (`docs/COMPANION.md` §7).
//!
//! ```text
//! "FCRS"  version (1 byte, 1)  count (u32 LE)
//! then for each file, newest last:
//!   kind (1 byte)  kept at (u64 LE, seconds since the epoch, 0 unknown)
//!   SHA-256 of the bytes (32)  name length (u16 LE)  name (UTF-8)
//!   size (u32 LE)  bytes
//! ```
//!
//! A store is read whole or not at all: a wrong magic, a version this
//! build does not know, a kind it does not know, a name that is not
//! UTF-8, a SHA-256 that does not match its bytes or anything left over
//! is a store it cannot read.

use osk_bip::bitcoin::hashes::{Hash, sha256};

/// The first four bytes of a store.
pub const MAGIC: &[u8; 4] = b"FCRS";

/// The version this build writes and reads.
pub const VERSION: u8 = 1;

/// What a kept file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A PSBT, kept binary.
    Psbt,
    /// A finished transaction, kept as hex text.
    Transaction,
    /// A wallet description: a descriptor, a multisig config, an account
    /// key.
    Description,
    /// A file from a Faraday file envelope, or anything else.
    File,
    /// Text the person kept.
    Text,
}

impl Kind {
    fn code(self) -> u8 {
        match self {
            Kind::Psbt => 1,
            Kind::Transaction => 2,
            Kind::Description => 3,
            Kind::File => 4,
            Kind::Text => 5,
        }
    }

    fn of(code: u8) -> Option<Kind> {
        Some(match code {
            1 => Kind::Psbt,
            2 => Kind::Transaction,
            3 => Kind::Description,
            4 => Kind::File,
            5 => Kind::Text,
            _ => return None,
        })
    }
}

/// One kept file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// Its name.
    pub name: String,
    /// What it is.
    pub kind: Kind,
    /// When it was kept, seconds since the epoch; 0 when the phone's
    /// clock had not been given.
    pub time: u64,
    /// The SHA-256 of its bytes.
    pub sha256: [u8; 32],
    /// The file.
    pub bytes: Vec<u8>,
}

impl Kept {
    /// A file kept now, its SHA-256 worked out.
    pub fn new(name: String, kind: Kind, time: u64, bytes: Vec<u8>) -> Kept {
        Kept {
            sha256: digest(&bytes),
            name,
            kind,
            time,
            bytes,
        }
    }
}

/// The SHA-256 of `bytes`.
pub fn digest(bytes: &[u8]) -> [u8; 32] {
    sha256::Hash::hash(bytes).to_byte_array()
}

/// The store holding `files`, oldest first.
pub fn write(files: &[Kept]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for f in files {
        out.push(f.kind.code());
        out.extend_from_slice(&f.time.to_le_bytes());
        out.extend_from_slice(&f.sha256);
        let name = f.name.as_bytes();
        let name = &name[..name.len().min(usize::from(u16::MAX))];
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&(f.bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&f.bytes);
    }
    out
}

/// Bytes read from the front of a store.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
}

/// The files a store holds, oldest first, or `None` for a store this
/// build cannot read.
pub fn read(bytes: &[u8]) -> Option<Vec<Kept>> {
    let mut r = Reader(bytes);
    if r.take(4)? != MAGIC || r.u8()? != VERSION {
        return None;
    }
    let count = r.u32()?;
    let mut files = Vec::new();
    for _ in 0..count {
        let kind = Kind::of(r.u8()?)?;
        let time = r.u64()?;
        let sha: [u8; 32] = r.take(32)?.try_into().ok()?;
        let len = usize::from(r.u16()?);
        let name = std::str::from_utf8(r.take(len)?).ok()?.to_string();
        let size = usize::try_from(r.u32()?).ok()?;
        let data = r.take(size)?.to_vec();
        if digest(&data) != sha {
            return None;
        }
        files.push(Kept {
            name,
            kind,
            time,
            sha256: sha,
            bytes: data,
        });
    }
    r.0.is_empty().then_some(files)
}
