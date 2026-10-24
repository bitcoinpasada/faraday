//! What a vault keeps of a codex32 set (`docs/VAULT.md` §7,
//! `docs/CODEX32.md` §9.4, §9.5): a key record's codex32 set, its plain
//! facts and never a string, and the codex32 share record, one string of
//! a set kept without its seed.
//!
//! **The set** (a key record's field 5) is these bytes:
//!
//! | At | Bytes | What |
//! |---|---|---|
//! | 0 | 1 | the threshold k: 0 (the secret alone), or 2 to 9 |
//! | 1 | 4 | the identifier, four bech32 characters, lower case |
//! | 5 | 1 | each string's length in characters: 48, 54, 61, 67, 74 or 127 |
//! | 6 | 1 | how many strings exist whose index is not known |
//! | 7 | 2 each | per string whose index is known: the index (a bech32 character, lower case), then its state ([`State`]) |
//!
//! At most 31 strings in all, known or not, each index once; at k = 0
//! only the secret, `s`, and nothing else. Anything else is not this
//! layout and [`set_of`] gives `None`.
//!
//! **The share record** (type 15) holds one string of a set whose
//! threshold is 2 or more, never the secret (that is the seed, and goes in
//! as a key record), with the fingerprint of the key the set makes.

use alloc::vec::Vec;

use faraday_bip::codex32::Codex32;

/// The bech32 characters, the codex32 alphabet.
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// The lengths a codex32 string can have (BIP 93).
pub const LENGTHS: [u8; 6] = [48, 54, 61, 67, 74, 127];

/// The most strings a set has.
pub const MAX_STRINGS: usize = 31;

/// What the plan last noted of a string of the set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Made, or typed in, and not yet placed.
    Made,
    /// Placed by a backup plan.
    Placed,
    /// Lost.
    Lost,
    /// Exposed: a copy may have been taken.
    Exposed,
    /// Destroyed.
    Destroyed,
}

impl State {
    /// The state as its byte.
    pub fn code(self) -> u8 {
        match self {
            State::Made => 0,
            State::Placed => 1,
            State::Lost => 2,
            State::Exposed => 3,
            State::Destroyed => 4,
        }
    }

    /// [`State::code`] read back.
    pub fn of(b: u8) -> Option<State> {
        Some(match b {
            0 => State::Made,
            1 => State::Placed,
            2 => State::Lost,
            3 => State::Exposed,
            4 => State::Destroyed,
            _ => return None,
        })
    }
}

/// A codex32 set's plain facts: what a key record keeps of the set its
/// seed was split into, or the set it was loaded from. No string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFacts {
    /// The threshold: 0 for the secret alone, else 2 to 9.
    pub threshold: u8,
    /// The identifier, lower case.
    pub identifier: [u8; 4],
    /// Each string's length in characters.
    pub length: u8,
    /// How many strings exist whose index is not known.
    pub unknown: u8,
    /// The strings whose index is known: each index (lower case) and what
    /// was last noted of it.
    pub strings: Vec<(u8, State)>,
}

impl SetFacts {
    /// Whether the facts are a set this layout can hold.
    pub fn is_sound(&self) -> bool {
        let bech = |c: u8| CHARSET.contains(&c);
        let mut seen = [false; 32];
        let distinct = self.strings.iter().all(|&(c, _)| {
            let Some(k) = CHARSET.iter().position(|&x| x == c) else {
                return false;
            };
            !core::mem::replace(&mut seen[k], true)
        });
        let total = self.strings.len() + usize::from(self.unknown);
        let threshold = match self.threshold {
            0 => self.unknown == 0 && self.strings.iter().all(|&(c, _)| c == b's'),
            2..=9 => true,
            _ => false,
        };
        threshold
            && distinct
            && total <= MAX_STRINGS
            && self.identifier.iter().all(|&c| bech(c))
            && LENGTHS.contains(&self.length)
    }

    /// The indices known, in order.
    pub fn indices(&self) -> Vec<u8> {
        self.strings.iter().map(|&(c, _)| c).collect()
    }

    /// What was last noted of the string at `index`.
    pub fn state(&self, index: u8) -> Option<State> {
        self.strings
            .iter()
            .find(|&&(c, _)| c == index)
            .map(|&(_, s)| s)
    }
}

/// A key record's field 5: the set's bytes, or `None` where the facts are
/// not a set this layout holds.
pub fn set_payload(facts: &SetFacts) -> Option<Vec<u8>> {
    if !facts.is_sound() {
        return None;
    }
    let mut out = Vec::with_capacity(7 + 2 * facts.strings.len());
    out.push(facts.threshold);
    out.extend_from_slice(&facts.identifier);
    out.push(facts.length);
    out.push(facts.unknown);
    for &(c, s) in &facts.strings {
        out.push(c);
        out.push(s.code());
    }
    Some(out)
}

/// [`set_payload`] read back.
pub fn set_of(bytes: &[u8]) -> Option<SetFacts> {
    if bytes.len() < 7 || !(bytes.len() - 7).is_multiple_of(2) {
        return None;
    }
    let mut identifier = [0u8; 4];
    identifier.copy_from_slice(&bytes[1..5]);
    let strings = bytes[7..]
        .chunks_exact(2)
        .map(|p| Some((p[0], State::of(p[1])?)))
        .collect::<Option<Vec<_>>>()?;
    let facts = SetFacts {
        threshold: bytes[0],
        identifier,
        length: bytes[5],
        unknown: bytes[6],
        strings,
    };
    facts.is_sound().then_some(facts)
}

/// Whether `bytes` is a share record's string: a codex32 string, lower
/// case, of a set split with a threshold of 2 or more, and not its secret.
pub fn is_share(bytes: &[u8]) -> bool {
    let Ok(text) = core::str::from_utf8(bytes) else {
        return false;
    };
    if text.bytes().any(|c| c.is_ascii_uppercase()) {
        return false;
    }
    Codex32::parse(text).is_ok_and(|s| s.threshold() >= 2 && !s.is_secret())
}
