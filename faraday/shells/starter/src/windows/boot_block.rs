//! The boot file's bytes, from the window to the elevated writer over
//! the writer's pipe (`docs/STARTER.md` §4.5, F133): the elevated copy
//! never opens a path for the boot file a person signed. One block,
//! sent once after the elevated copy connects and before it unmounts or
//! opens the disk:
//!
//! ```text
//! 8 bytes   the length, little-endian
//! length    the boot file's bytes
//! ```
//!
//! The length is at most [`MAX_BLOCK`]. Plain `Read` and `Write`, so the
//! tests run on any system.

use std::io::{self, Read, Write};

use faraday_starter::api::{BootFile, Reason};

/// The most bytes a block carries: the most a boot file may have.
pub const MAX_BLOCK: u64 = crate::signed::MAX_BOOT_FILE;

/// Why a block was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// Its length is over [`MAX_BLOCK`].
    Large,
    /// The pipe ended before the length or the bytes it promised.
    Short,
    /// The bytes' SHA-256 is not the order's.
    Sum,
}

impl Refused {
    /// The reason shown. The elevated copy has no path to name: the
    /// window puts the file's name in.
    pub fn reason(self) -> Reason {
        match self {
            Refused::Large => Reason::BootFileLarge(String::new()),
            Refused::Short => Reason::BootFileRead(String::new()),
            Refused::Sum => Reason::BootFileChanged(String::new()),
        }
    }
}

/// `reason` with the chosen boot file's path put back when it is a
/// boot file's refusal the elevated copy made without one.
pub fn named(reason: Reason, file: Option<&BootFile>) -> Reason {
    match (reason, file) {
        (Reason::BootFileRead(p), Some(f)) if p.is_empty() => Reason::BootFileRead(f.path.clone()),
        (Reason::BootFileLarge(p), Some(f)) if p.is_empty() => {
            Reason::BootFileLarge(f.path.clone())
        }
        (Reason::BootFileChanged(p), Some(f)) if p.is_empty() => {
            Reason::BootFileChanged(f.path.clone())
        }
        (r, _) => r,
    }
}

/// Sends `bytes` as one block. Refused, without writing, past
/// [`MAX_BLOCK`].
pub fn send<W: Write>(w: &mut W, bytes: &[u8]) -> io::Result<()> {
    let len = bytes.len() as u64;
    if len > MAX_BLOCK {
        return Err(io::Error::other("a boot file past the block's size"));
    }
    w.write_all(&len.to_le_bytes())?;
    w.write_all(bytes)?;
    w.flush()
}

/// Costs a SHA-256 of what is read: the bytes of one block, taken only
/// when their SHA-256 is `sha256` (hex), the order's. Nothing is
/// allocated for a length over [`MAX_BLOCK`].
pub fn receive<R: Read>(r: &mut R, sha256: &str) -> Result<Vec<u8>, Refused> {
    let mut head = [0u8; 8];
    r.read_exact(&mut head).map_err(|_| Refused::Short)?;
    let len = u64::from_le_bytes(head);
    if len > MAX_BLOCK {
        return Err(Refused::Large);
    }
    let mut bytes = Vec::new();
    r.take(len)
        .read_to_end(&mut bytes)
        .map_err(|_| Refused::Short)?;
    if bytes.len() as u64 != len {
        return Err(Refused::Short);
    }
    if crate::signed::sha256(&bytes) != sha256 {
        return Err(Refused::Sum);
    }
    Ok(bytes)
}
