//! What a vault keeps of a BIP-85 child seed (`docs/VAULT.md` §7,
//! `docs/decisions/F166.md`): a key record's field 6, the key it was
//! derived from and where, written only when the person chose to record
//! it. The vault is the only place Faraday keeps this.
//!
//! **The record** (a key record's field 6) is these 14 bytes:
//!
//! | At | Bytes | What |
//! |---|---|---|
//! | 0 | 4 | the parent key's fingerprint |
//! | 4 | 4 | the BIP-85 application, `u32` little-endian: 39, BIP-39 words |
//! | 8 | 1 | the BIP-85 language number: 0 to 8 |
//! | 9 | 1 | how many words: 12, 18 or 24 |
//! | 10 | 4 | the index, `u32` little-endian, at most 2³¹ − 1 |
//!
//! Anything else is not this layout and [`child_of`] gives `None`.

use alloc::vec::Vec;

/// How long the record is.
pub const LEN: usize = 14;

/// BIP-85's application for BIP-39 words.
pub const WORDS_APP: u32 = 39;

/// The word counts BIP-85's application 39 derives.
pub const WORD_COUNTS: [u8; 3] = [12, 18, 24];

/// The highest BIP-85 language number (8, Czech).
pub const LAST_LANGUAGE: u8 = 8;

/// The largest index: a hardened step's.
pub const MAX_INDEX: u32 = 0x7fff_ffff;

/// A BIP-85 child seed: the key it is derived from, and where. Public: a
/// fingerprint and three numbers, no secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChildOf {
    /// The parent key's fingerprint.
    pub parent: [u8; 4],
    /// The BIP-85 language number (0 English).
    pub language: u8,
    /// How many words.
    pub words: u8,
    /// The index.
    pub index: u32,
}

impl ChildOf {
    /// Whether the layout holds it: a word count application 39 derives,
    /// a BIP-85 language, an index BIP-85 takes.
    pub fn is_sound(&self) -> bool {
        WORD_COUNTS.contains(&self.words)
            && self.language <= LAST_LANGUAGE
            && self.index <= MAX_INDEX
    }
}

/// A key record's field 6: the record's bytes, or `None` where `c` is not
/// one this layout holds.
pub fn payload(c: &ChildOf) -> Option<Vec<u8>> {
    if !c.is_sound() {
        return None;
    }
    let mut out = Vec::with_capacity(LEN);
    out.extend_from_slice(&c.parent);
    out.extend_from_slice(&WORDS_APP.to_le_bytes());
    out.push(c.language);
    out.push(c.words);
    out.extend_from_slice(&c.index.to_le_bytes());
    Some(out)
}

/// [`payload`] read back.
pub fn child_of(bytes: &[u8]) -> Option<ChildOf> {
    if bytes.len() != LEN {
        return None;
    }
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    if u32_at(4) != WORDS_APP {
        return None;
    }
    let c = ChildOf {
        parent: [bytes[0], bytes[1], bytes[2], bytes[3]],
        language: bytes[8],
        words: bytes[9],
        index: u32_at(10),
    };
    c.is_sound().then_some(c)
}
