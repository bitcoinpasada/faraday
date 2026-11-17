//! What a vault keeps of a SLIP-39 set (`docs/VAULT.md` §7): a key
//! record's field 8, the set's identifier, iteration exponent, group
//! threshold, each group's threshold and member count, and which member
//! indices were made. Public facts; never a share.
//!
//! **The record** (a key record's field 8) is these bytes:
//!
//! | At | Bytes | What |
//! |---|---|---|
//! | 0 | 2 | the identifier, 15 bits, `u16` little-endian |
//! | 2 | 1 | the iteration exponent: 0 to 15 |
//! | 3 | 1 | the group threshold: 1 to the number of groups |
//! | 4 | 1 | how many groups: 1 to 16 |
//! | 5 | 4 each | per group: its member threshold (1 to its count), its member count (1 to 16), then the member indices made as a `u16` little-endian mask, bit i for index i, only bits below the count |
//!
//! Anything else is not this layout and [`slip39_of`] gives `None`.

use alloc::vec::Vec;

/// The most groups, and the most members of a group (SLIP-39).
pub const MAX: u8 = faraday_bip::slip39::MAX_COUNT as u8;

/// One group of a set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    /// How many of its members recover the group's share.
    pub threshold: u8,
    /// How many members it has.
    pub count: u8,
    /// The member indices made, bit i for index i.
    pub made: u16,
}

/// A SLIP-39 set's plain facts. No share.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::disallowed_methods, reason = "a derived Clone of public data")]
pub struct Slip39Facts {
    /// The set's identifier, 15 bits.
    pub identifier: u16,
    /// The iteration exponent, 0 to 15.
    pub exponent: u8,
    /// How many groups recover the secret.
    pub threshold: u8,
    /// The groups, in order.
    pub groups: Vec<Group>,
}

impl Slip39Facts {
    /// Whether the layout holds it.
    pub fn is_sound(&self) -> bool {
        let n = self.groups.len();
        self.identifier < 0x8000
            && self.exponent <= faraday_bip::slip39::MAX_ITERATION_EXPONENT
            && (1..=usize::from(MAX)).contains(&n)
            && (1..=n).contains(&usize::from(self.threshold))
            && self.groups.iter().all(|g| {
                (1..=MAX).contains(&g.count)
                    && (1..=g.count).contains(&g.threshold)
                    && (g.threshold > 1 || g.count == 1)
                    && u32::from(g.made) < 1u32 << g.count
            })
    }
}

/// A key record's field 8: the facts' bytes, or `None` where they are not
/// a set this layout holds.
pub fn payload(f: &Slip39Facts) -> Option<Vec<u8>> {
    if !f.is_sound() {
        return None;
    }
    let mut out = Vec::with_capacity(5 + 4 * f.groups.len());
    out.extend_from_slice(&f.identifier.to_le_bytes());
    out.push(f.exponent);
    out.push(f.threshold);
    out.push(u8::try_from(f.groups.len()).ok()?);
    for g in &f.groups {
        out.push(g.threshold);
        out.push(g.count);
        out.extend_from_slice(&g.made.to_le_bytes());
    }
    Some(out)
}

/// [`payload`] read back.
pub fn slip39_of(bytes: &[u8]) -> Option<Slip39Facts> {
    if bytes.len() < 5 || bytes.len() != 5 + 4 * usize::from(bytes[4]) {
        return None;
    }
    let groups = bytes[5..]
        .chunks_exact(4)
        .map(|c| Group {
            threshold: c[0],
            count: c[1],
            made: u16::from_le_bytes([c[2], c[3]]),
        })
        .collect();
    let f = Slip39Facts {
        identifier: u16::from_le_bytes([bytes[0], bytes[1]]),
        exponent: bytes[2],
        threshold: bytes[3],
        groups,
    };
    f.is_sound().then_some(f)
}
