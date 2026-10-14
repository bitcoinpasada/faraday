//! The kept store: wallet descriptions and BitLocker recovery keys
//! (`docs/COMPANION.md` §4.5, §4.6, §7), the Companion's own byte
//! format, separate from Received. The shell holds it encrypted under a
//! key that needs the phone's unlock, hands it to the core when the
//! person has unlocked ([`crate::CompanionEvent::Kept`]) and keeps what
//! the core writes back whole after each change
//! ([`crate::CompanionCommand::KeepKept`]).
//!
//! ```text
//! "FCKS"  version (1 byte, 1)
//! descriptions: count (u32 LE), then for each, oldest first:
//!   kept at (u64 LE, seconds since the epoch, 0 unknown)
//!   name length (u16 LE)  name (UTF-8)
//!   text length (u32 LE)  text (UTF-8, as it arrived)
//! keys: count (u32 LE), then for each, oldest first:
//!   kept at (u64 LE)
//!   computer length (u16 LE)  computer (UTF-8)
//!   drive length (u16 LE)  drive (UTF-8)
//!   key (55 bytes: eight groups of six digits, `-` between)
//! ```
//!
//! An empty store is no bytes at all: what the shell hands over the
//! first time. A store is read whole or not at all: a wrong magic, a
//! version this build does not know, text that is not UTF-8, a key that
//! is not one, or anything left over is a store it cannot read. Every
//! copy of the bytes is wiped when dropped.

use faraday_qr::bitlocker::{KeyText, RecoveryKey};
use zeroize::{Zeroize, Zeroizing};

/// The first four bytes of a store.
pub const MAGIC: &[u8; 4] = b"FCKS";

/// The version this build writes and reads.
pub const VERSION: u8 = 1;

/// A kept wallet description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    /// The name the person gave it.
    pub name: String,
    /// The text, as it arrived.
    pub text: String,
    /// When it was kept, seconds since the epoch; 0 when not known.
    pub time: u64,
}

/// A kept BitLocker recovery key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    /// The computer, the drive and the key, as the Starter wrote them.
    pub text: KeyText,
    /// When it was kept, seconds since the epoch; 0 when not known.
    pub time: u64,
}

/// What the kept store holds. Wiped when dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    /// The wallet descriptions, oldest first.
    pub descriptions: Vec<Description>,
    /// The BitLocker keys, oldest first.
    pub keys: Vec<Key>,
}

impl Store {
    /// How many descriptions and how many keys it holds: what Home shows
    /// while the store is locked.
    pub fn counts(&self) -> (u32, u32) {
        (
            u32::try_from(self.descriptions.len()).unwrap_or(u32::MAX),
            u32::try_from(self.keys.len()).unwrap_or(u32::MAX),
        )
    }

    /// Whether it holds nothing.
    pub fn is_empty(&self) -> bool {
        self.descriptions.is_empty() && self.keys.is_empty()
    }
}

impl Zeroize for Store {
    fn zeroize(&mut self) {
        for d in &mut self.descriptions {
            d.name.zeroize();
            d.text.zeroize();
        }
        for k in &mut self.keys {
            k.text.zeroize();
            k.text.computer.zeroize();
            k.text.drive.zeroize();
        }
        self.descriptions.clear();
        self.keys.clear();
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Room for a field's bytes: at most `u16::MAX` of them, cut at a
/// character's edge.
fn short(s: &str) -> &[u8] {
    let mut n = s.len().min(usize::from(u16::MAX));
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    &s.as_bytes()[..n]
}

/// The store's bytes, wiped when dropped.
pub fn write(store: &Store) -> Zeroizing<Vec<u8>> {
    let size = 13
        + store
            .descriptions
            .iter()
            .map(|d| 14 + d.name.len() + d.text.len())
            .sum::<usize>()
        + store
            .keys
            .iter()
            .map(|k| 12 + k.text.computer.len() + k.text.drive.len() + 55)
            .sum::<usize>();
    let mut out = Zeroizing::new(Vec::with_capacity(size));
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&(store.descriptions.len() as u32).to_le_bytes());
    for d in &store.descriptions {
        out.extend_from_slice(&d.time.to_le_bytes());
        let name = short(&d.name);
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&(d.text.len() as u32).to_le_bytes());
        out.extend_from_slice(d.text.as_bytes());
    }
    out.extend_from_slice(&(store.keys.len() as u32).to_le_bytes());
    for k in &store.keys {
        out.extend_from_slice(&k.time.to_le_bytes());
        for field in [&k.text.computer, &k.text.drive] {
            let f = short(field);
            out.extend_from_slice(&(f.len() as u16).to_le_bytes());
            out.extend_from_slice(f);
        }
        out.extend_from_slice(k.text.key.as_str().as_bytes());
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

    fn text(&mut self, len: usize) -> Option<String> {
        std::str::from_utf8(self.take(len)?)
            .ok()
            .map(str::to_string)
    }
}

/// What a store holds, or `None` for a store this build cannot read. No
/// bytes at all is the empty store.
pub fn read(bytes: &[u8]) -> Option<Store> {
    let mut store = Store::default();
    if bytes.is_empty() {
        return Some(store);
    }
    let mut r = Reader(bytes);
    if r.take(4)? != MAGIC || r.u8()? != VERSION {
        return None;
    }
    for _ in 0..r.u32()? {
        let time = r.u64()?;
        let len = usize::from(r.u16()?);
        let name = r.text(len)?;
        let len = usize::try_from(r.u32()?).ok()?;
        let text = r.text(len)?;
        store.descriptions.push(Description { name, text, time });
    }
    for _ in 0..r.u32()? {
        let time = r.u64()?;
        let len = usize::from(r.u16()?);
        let computer = r.text(len)?;
        let len = usize::from(r.u16()?);
        let drive = r.text(len)?;
        let key = RecoveryKey::parse(std::str::from_utf8(r.take(55)?).ok()?)?;
        store.keys.push(Key {
            text: KeyText {
                computer,
                drive,
                key,
            },
            time,
        });
    }
    r.0.is_empty().then_some(store)
}
