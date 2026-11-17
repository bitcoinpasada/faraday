//! What a vault keeps of a Seed XOR set (`docs/VAULT.md` §7): a key
//! record's field 7, how many parts the key's seed was split into and each
//! part's own master fingerprint. Public facts; never a part, which is a
//! seed itself.
//!
//! **The record** (a key record's field 7) is these bytes:
//!
//! | At | Bytes | What |
//! |---|---|---|
//! | 0 | 1 | how many parts, n: 2 to 4 |
//! | 1 | 4 each | per part, in order: its master fingerprint |
//!
//! Anything else is not this layout and [`xor_of`] gives `None`.

use alloc::vec::Vec;

/// The fewest parts a set has.
pub const MIN_PARTS: usize = 2;

/// The most parts a set has.
pub const MAX_PARTS: usize = faraday_entropy::MAX_XOR_PARTS;

/// A Seed XOR set's plain facts: the master fingerprint of each of its
/// parts, every one needed to rebuild the seed. No part.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
pub struct XorFacts {
    /// Each part's master fingerprint, in the order of the parts.
    pub parts: Vec<[u8; 4]>,
}

impl XorFacts {
    /// Whether the layout holds it: 2 to 4 parts.
    pub fn is_sound(&self) -> bool {
        (MIN_PARTS..=MAX_PARTS).contains(&self.parts.len())
    }
}

/// A key record's field 7: the facts' bytes, or `None` where they are not
/// a set this layout holds.
pub fn payload(f: &XorFacts) -> Option<Vec<u8>> {
    if !f.is_sound() {
        return None;
    }
    let mut out = Vec::with_capacity(1 + 4 * f.parts.len());
    out.push(u8::try_from(f.parts.len()).ok()?);
    for p in &f.parts {
        out.extend_from_slice(p);
    }
    Some(out)
}

/// [`payload`] read back.
pub fn xor_of(bytes: &[u8]) -> Option<XorFacts> {
    let (&n, rest) = bytes.split_first()?;
    if rest.len() != 4 * usize::from(n) {
        return None;
    }
    let parts = rest
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect();
    let f = XorFacts { parts };
    f.is_sound().then_some(f)
}
