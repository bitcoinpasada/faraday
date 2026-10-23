//! The payload layouts a vault's records hold (`docs/VAULT.md` §7): a
//! key's words, a master seed, a wallet's recovery sheet and the text of
//! a note. A record's field carries one of these byte strings, and this
//! module reads and writes them.
//!
//! - Words ([`KIND_WORDS`]): 34 bytes. The word count, the word list's
//!   index in [`Language::ALL`], then the entropy zero-padded to 32
//!   bytes.
//! - Seed ([`KIND_SEED`]): 65 bytes. The seed's length (16 to 64), then
//!   the seed zero-padded to 64 bytes.
//! - Recovery sheet ([`KIND_SHEET`]): the descriptor, the name and the
//!   note, each a little-endian `u16` length and that many bytes of
//!   UTF-8, in that order.
//! - Note: its UTF-8 bytes.
//!
//! Every pad byte must be zero and every length must fit, or the bytes
//! are not that layout and [`payload_of`] gives `None`.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.

use alloc::vec::Vec;

use faraday_bip::bip39::{Language, Mnemonic};
use zeroize::Zeroizing;

/// A key's words.
pub const KIND_WORDS: u8 = 1;
/// A master seed of 16 to 64 bytes, which is what a SLIP-39 or a
/// codex32 key is.
pub const KIND_SEED: u8 = 2;
/// A wallet's recovery sheet: its descriptor, its name and a note.
pub const KIND_SHEET: u8 = 4;

/// The payload's two leading bytes: word count and language.
const PREFIX_LEN: usize = 2;
/// The shortest entropy a mnemonic carries.
const MIN_ENTROPY: usize = 16;
/// The longest, and the room the words payload always makes.
const MAX_ENTROPY: usize = 32;

/// The words payload, the same length for every key.
const WORDS_LEN: usize = PREFIX_LEN + MAX_ENTROPY;

/// The shortest master seed a [`KIND_SEED`] payload carries.
pub const MIN_SEED: usize = 16;
/// The longest, and the room that payload always makes.
pub const MAX_SEED: usize = 64;

/// The seed payload, the same length for every seed: its length, then
/// 64 bytes zero-padded on the right.
const SEED_LEN_PAYLOAD: usize = 1 + MAX_SEED;

/// The most a sheet payload may hold.
pub const MAX_PAYLOAD: usize = 16 * 1024;

/// What a payload held.
pub enum Opened {
    /// A key's words.
    Words(Mnemonic),
    /// A master seed, 16 to 64 bytes.
    Seed(Zeroizing<Vec<u8>>),
    /// A wallet's recovery sheet.
    Sheet(Sheet),
}

/// A recovery sheet's three parts.
pub struct Sheet {
    /// The wallet's descriptor.
    pub descriptor: Zeroizing<Vec<u8>>,
    /// What the person calls the wallet, empty where it has no name.
    pub name: Zeroizing<Vec<u8>>,
    /// The sheet's note, empty until one is written.
    pub note: Zeroizing<Vec<u8>>,
}

/// A [`KIND_SHEET`] payload: the descriptor, the name and the note,
/// each a little-endian `u16` length and that many bytes, in that
/// order. `None` where it would pass [`MAX_PAYLOAD`] or a part is
/// longer than a `u16` says.
pub fn sheet_payload(descriptor: &[u8], name: &[u8], note: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let total = 6 + descriptor.len() + name.len() + note.len();
    if total > MAX_PAYLOAD {
        return None;
    }
    let mut payload: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    payload.try_reserve_exact(total).ok()?;
    for part in [descriptor, name, note] {
        let n = u16::try_from(part.len()).ok()?;
        payload.extend_from_slice(&n.to_le_bytes());
        payload.extend_from_slice(part);
    }
    Some(payload)
}

/// A [`KIND_WORDS`] payload: the word count, the word list's place in
/// [`Language::ALL`] and the entropy zero-padded to 32 bytes.
pub fn words_payload(m: &Mnemonic) -> Zeroizing<Vec<u8>> {
    let entropy = m.entropy();
    let bytes = entropy.expose().as_bytes();
    let lang = Language::ALL
        .iter()
        .position(|l| *l == m.language())
        .unwrap_or(0) as u8;
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; WORDS_LEN]);
    out[0] = m.word_count() as u8;
    out[1] = lang;
    out[PREFIX_LEN..PREFIX_LEN + bytes.len()].copy_from_slice(bytes);
    out
}

/// A [`KIND_SEED`] payload: the seed's length, then the seed zero-padded
/// to 64 bytes. `None` where the seed is not 16 to 64 bytes.
pub fn seed_payload(seed: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if !(MIN_SEED..=MAX_SEED).contains(&seed.len()) {
        return None;
    }
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; SEED_LEN_PAYLOAD]);
    out[0] = seed.len() as u8;
    out[1..1 + seed.len()].copy_from_slice(seed);
    Some(out)
}

/// What a payload of `kind` holds. `None` where the bytes are not that
/// kind's.
pub fn payload_of(kind: u8, payload: &[u8]) -> Option<Opened> {
    match kind {
        KIND_WORDS => words_of(payload),
        KIND_SEED => seed_of(payload),
        KIND_SHEET => sheet_of(payload),
        _ => None,
    }
}

/// The words a [`KIND_WORDS`] payload holds. The count says how many of
/// the 32 bytes are the key's; the rest are the pad, and a pad that is
/// not zero is not a payload.
fn words_of(payload: &[u8]) -> Option<Opened> {
    if payload.len() != WORDS_LEN {
        return None;
    }
    let words = usize::from(payload[0]);
    let lang = Language::ALL.get(usize::from(payload[1])).copied()?;
    let entropy = words * 4 / 3;
    if words * 4 != entropy * 3
        || !(MIN_ENTROPY..=MAX_ENTROPY).contains(&entropy)
        || payload[PREFIX_LEN + entropy..].iter().any(|&b| b != 0)
    {
        return None;
    }
    Mnemonic::from_entropy(lang, &payload[PREFIX_LEN..PREFIX_LEN + entropy])
        .ok()
        .map(Opened::Words)
}

/// The master seed a [`KIND_SEED`] payload holds.
fn seed_of(payload: &[u8]) -> Option<Opened> {
    if payload.len() != SEED_LEN_PAYLOAD {
        return None;
    }
    let n = usize::from(payload[0]);
    if !(MIN_SEED..=MAX_SEED).contains(&n) || payload[1 + n..].iter().any(|&b| b != 0) {
        return None;
    }
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    out.try_reserve_exact(n).ok()?;
    out.extend_from_slice(&payload[1..1 + n]);
    Some(Opened::Seed(out))
}

/// One length-prefixed field's bytes, kept in a buffer of its own.
fn text_of(bytes: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    core::str::from_utf8(bytes).ok()?;
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    out.try_reserve_exact(bytes.len()).ok()?;
    out.extend_from_slice(bytes);
    Some(out)
}

/// The three parts a [`KIND_SHEET`] payload holds.
fn sheet_of(payload: &[u8]) -> Option<Opened> {
    let mut at = 0usize;
    let mut parts: [Option<Zeroizing<Vec<u8>>>; 3] = [None, None, None];
    for part in &mut parts {
        if at + 2 > payload.len() {
            return None;
        }
        let n = usize::from(u16::from_le_bytes([payload[at], payload[at + 1]]));
        at += 2;
        if at + n > payload.len() {
            return None;
        }
        *part = Some(text_of(&payload[at..at + n])?);
        at += n;
    }
    if at != payload.len() {
        return None;
    }
    let [descriptor, name, note] = parts;
    Some(Opened::Sheet(Sheet {
        descriptor: descriptor?,
        name: name?,
        note: note?,
    }))
}
