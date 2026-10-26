//! Pointer and keys (`docs/OWN-RANDOMNESS.md` §2.1): pointer moves and
//! keys a person makes on the source's area, each written as a fixed
//! record with the time and a running count, and committed in batches
//! of at most [`POINTER_KEYS_BATCH`].
//!
//! A record is 17 bytes, big-endian:
//!
//! - a pointer move: `P`, the running count (u32), the time in ms (u64),
//!   x (u16) and y (u16) in the frame's pixels;
//! - a key: `K`, the running count (u32), the time in ms (u64) and the
//!   key's code (u32).
//!
//! A batch's commitment is SHA-256("faraday pointer keys" ‖ its records,
//! in order). The running count goes on across batches, so two batches
//! of the same moves at the same times are never the same.

use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Events a batch holds before it is committed.
pub const POINTER_KEYS_BATCH: usize = 64;

/// The bytes of one event's record.
pub const POINTER_KEYS_RECORD: usize = 17;

/// The commitment's domain, before the records.
const DOMAIN: &[u8; 20] = b"faraday pointer keys";

/// Pointer moves and keys not yet committed, with the running count.
pub struct PointerKeys {
    /// The domain, then the records of the batch.
    buf: [u8; DOMAIN.len() + POINTER_KEYS_BATCH * POINTER_KEYS_RECORD],
    /// Events in the batch.
    len: u8,
    /// Events taken since this accumulator was made, every batch's.
    count: u32,
    /// Pointer moves taken, every batch's.
    moves: u32,
    /// Keys taken, every batch's.
    keys: u32,
}

impl Default for PointerKeys {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for PointerKeys {
    fn zeroize(&mut self) {
        self.buf.zeroize();
        self.len.zeroize();
        self.count.zeroize();
        self.moves.zeroize();
        self.keys.zeroize();
    }
}

impl Drop for PointerKeys {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for PointerKeys {}

impl PointerKeys {
    /// Nothing taken yet.
    pub fn new() -> Self {
        PointerKeys {
            buf: [0u8; DOMAIN.len() + POINTER_KEYS_BATCH * POINTER_KEYS_RECORD],
            len: 0,
            count: 0,
            moves: 0,
            keys: 0,
        }
    }

    /// One record: its tag, the running count, the time, then `tail`.
    /// Returns `true` when the batch is full and wants committing.
    fn push(&mut self, tag: u8, ms: u64, tail: [u8; 4]) -> bool {
        if usize::from(self.len) >= POINTER_KEYS_BATCH {
            return true;
        }
        let at = DOMAIN.len() + usize::from(self.len) * POINTER_KEYS_RECORD;
        let r = &mut self.buf[at..at + POINTER_KEYS_RECORD];
        r[0] = tag;
        r[1..5].copy_from_slice(&self.count.to_be_bytes());
        r[5..13].copy_from_slice(&ms.to_be_bytes());
        r[13..17].copy_from_slice(&tail);
        self.len += 1;
        self.count = self.count.saturating_add(1);
        usize::from(self.len) >= POINTER_KEYS_BATCH
    }

    /// A pointer move to (x, y) at `ms`. Returns `true` when the batch
    /// is full.
    pub fn pointer(&mut self, x: u16, y: u16, ms: u64) -> bool {
        let [x0, x1] = x.to_be_bytes();
        let [y0, y1] = y.to_be_bytes();
        self.moves = self.moves.saturating_add(1);
        self.push(b'P', ms, [x0, x1, y0, y1])
    }

    /// A key whose code is `code` at `ms`. Returns `true` when the batch
    /// is full.
    pub fn key(&mut self, code: u32, ms: u64) -> bool {
        self.keys = self.keys.saturating_add(1);
        self.push(b'K', ms, code.to_be_bytes())
    }

    /// Events in the batch not yet committed.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Pointer moves taken, every batch's.
    pub fn moves(&self) -> u32 {
        self.moves
    }

    /// Keys taken, every batch's.
    pub fn keys(&self) -> u32 {
        self.keys
    }

    /// The batch's commitment and how many events it holds, the batch
    /// then wiped; the running count goes on. `None` for an empty batch.
    pub fn commit(&mut self) -> Option<([u8; 32], u32)> {
        if self.len == 0 {
            return None;
        }
        let n = usize::from(self.len);
        self.buf[..DOMAIN.len()].copy_from_slice(DOMAIN);
        let h = sha256(&self.buf[..DOMAIN.len() + n * POINTER_KEYS_RECORD]);
        self.buf.zeroize();
        self.len = 0;
        Some((h, n as u32))
    }

    /// Forgets the batch and every count.
    pub fn clear(&mut self) {
        self.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_batch_asks_to_be_committed_and_the_count_runs_on() {
        let mut pk = PointerKeys::new();
        for i in 0..63u16 {
            assert!(!pk.pointer(i, i, u64::from(i)));
        }
        assert!(pk.key(u32::from('a'), 63));
        let (first, n) = pk.commit().unwrap();
        assert_eq!(n, 64);
        assert!(pk.is_empty());
        for i in 0..63u16 {
            pk.pointer(i, i, u64::from(i));
        }
        pk.key(u32::from('a'), 63);
        let (second, _) = pk.commit().unwrap();
        assert_ne!(first, second, "the running count makes the batches differ");
        assert_eq!((pk.moves(), pk.keys()), (126, 2));
        assert!(pk.commit().is_none());
    }
}
