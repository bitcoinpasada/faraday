//! The item side of the blob's notes record (`docs/PLANNING.md` §6,
//! §16.112 pass E3).
//!
//! A device that keeps keys keeps the notes and the recovery sheets the
//! person said to keep, in a record of their own after the wallets
//! record, so that Tools › Notes is not empty after a restart and a
//! wallet's Recovery sheet comes back with the wallet.
//!
//! The split from the crate root is the one [`crate::wallets`]
//! makes: the root seals and opens [`BODY_LEN`] bytes and never looks
//! inside them; this module turns those bytes into items and back, and
//! never touches a key, a PIN or the cipher. Unlike the wallets, a note
//! is what a person wrote and is treated as a secret: every buffer here
//! zeroizes on drop, and this file is listed in `tools/lint-secrets.sh`
//! and holds no heap text.
//!
//! The body is a count and [`MAX_NOTES`] slots of [`SLOT_LEN`] bytes.
//! One slot is the container's own plaintext without its padding — the
//! kind byte, a little-endian `u16` length and that many bytes — so an
//! item in the blob and the same item in an `osk-backup` file are the
//! same bytes, read by the same decoder (`osk_backup::oskb`'s kinds 3
//! and 4). A slot with no item in it is zero, and every slot is written
//! whether it holds an item or not, so the stored bytes say neither how
//! many items there are nor how long each one is.
//!
//! [`SLOT_LEN`] is 4,352 bytes: a note is at most 4,096 characters
//! (OpenSigner's `MAX_NOTE`), which is 4,096 bytes of the Latin text this device's
//! keyboard types, and the rest is the slot's own three bytes and room
//! for a sheet's descriptor and name beside its note.
//! [`MAX_NOTES`] is 8 because an item is kept one at a time, by hand,
//! and eight slots are 34 KiB of a blob a phone rewrites on every
//! change.

use alloc::vec::Vec;

use osk_crypto::Zeroize;
use zeroize::Zeroizing;

/// Items the record has room for.
pub const MAX_NOTES: usize = 8;
/// Bytes one item is padded to.
pub const SLOT_LEN: usize = 4352;
/// The record's body: the count, then every slot.
pub const BODY_LEN: usize = 1 + MAX_NOTES * SLOT_LEN;

/// A slot's own bytes before the payload: the kind and the length.
const HEAD_LEN: usize = 3;
/// The longest payload a slot holds.
pub const MAX_ITEM: usize = SLOT_LEN - HEAD_LEN;

/// One kept item: the container's kind byte and the payload bytes that
/// kind is read from.
pub struct KeptNote {
    kind: u8,
    payload: Zeroizing<Vec<u8>>,
}

impl KeptNote {
    /// The item of `kind` over `payload`; `None` where the payload is
    /// longer than a slot holds.
    pub fn new(kind: u8, payload: &[u8]) -> Option<KeptNote> {
        if payload.len() > MAX_ITEM {
            return None;
        }
        let mut bytes: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
        bytes.try_reserve_exact(payload.len()).ok()?;
        bytes.extend_from_slice(payload);
        Some(KeptNote {
            kind,
            payload: bytes,
        })
    }

    /// Which kind of item it is.
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// The payload, as the container encodes that kind.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl Zeroize for KeptNote {
    fn zeroize(&mut self) {
        self.kind = 0;
        self.payload.zeroize();
    }
}

impl Drop for KeptNote {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// Every item the device keeps, in the order they come back. Room for
/// [`MAX_NOTES`]; [`KeptNotes::push`] refuses the rest.
pub struct KeptNotes {
    items: Vec<KeptNote>,
}

impl KeptNotes {
    /// No items.
    pub fn new() -> Self {
        KeptNotes { items: Vec::new() }
    }

    /// Adds one; `false`, and nothing added, when every slot is taken
    /// or the item is longer than a slot.
    pub fn push(&mut self, item: KeptNote) -> bool {
        if self.items.len() >= MAX_NOTES || item.payload.len() > MAX_ITEM {
            return false;
        }
        self.items.push(item);
        true
    }

    /// How many items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The items, in order.
    pub fn iter(&self) -> impl Iterator<Item = &KeptNote> {
        self.items.iter()
    }

    /// Writes the body over `out`, which is [`BODY_LEN`] bytes. A
    /// shorter buffer is left alone, because the record's size is
    /// fixed.
    pub fn write_body(&self, out: &mut [u8]) {
        if out.len() != BODY_LEN {
            return;
        }
        out.zeroize();
        out[0] = self.items.len() as u8;
        for (i, item) in self.items.iter().enumerate() {
            let at = 1 + i * SLOT_LEN;
            let n = item.payload.len();
            out[at] = item.kind;
            out[at + 1..at + 3].copy_from_slice(&(n as u16).to_le_bytes());
            out[at + HEAD_LEN..at + HEAD_LEN + n].copy_from_slice(&item.payload);
        }
    }
}

impl Default for KeptNotes {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for KeptNotes {
    fn zeroize(&mut self) {
        for item in &mut self.items {
            item.zeroize();
        }
        self.items.clear();
    }
}

impl Drop for KeptNotes {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// The items a body holds, in the order they were kept. A slot whose
/// length runs past the slot is dropped rather than failing the unlock:
/// the keys matter more than the list of notes.
pub fn items(body: &[u8]) -> KeptNotes {
    let mut out = KeptNotes::new();
    if body.len() != BODY_LEN {
        return out;
    }
    let count = usize::from(body[0]).min(MAX_NOTES);
    for i in 0..count {
        let at = 1 + i * SLOT_LEN;
        let slot = &body[at..at + SLOT_LEN];
        let n = usize::from(u16::from_le_bytes([slot[1], slot[2]]));
        if n > MAX_ITEM {
            continue;
        }
        let Some(item) = KeptNote::new(slot[0], &slot[HEAD_LEN..HEAD_LEN + n]) else {
            continue;
        };
        out.push(item);
    }
    out
}
