//! A codex32 set made from fresh entropy, as the booklet makes one
//! (`docs/CODEX32.md` §6): the person's characters are the data of the
//! `k` initial shares (`a`, `c`, `d`, … in BIP 93's order), each is
//! checksummed, and the secret `s` and every derived share are
//! interpolated from them. With no threshold (`k` = 0) the characters
//! are the secret's own data and there is nothing to interpolate.
//!
//! BIP 93, "For a fresh master seed": the initial shares are the random
//! ones and the secret is derived, so the seed is whatever the
//! interpolation gives, and its padding bits are not zero. Decoding
//! ignores them, as the BIP says.
//!
//! The data of every string depends only on the initial shares' data
//! and the share indices, never on the header: the threshold and the
//! identifier are the same in every string of the set and interpolate
//! to themselves, so the strings take their headers once the identifier
//! is set ([`Scheme`]).

use alloc::vec::Vec;

use faraday_crypto::work::Compute;

use super::{
    Codex32, Error, HEADER_LEN, INDEX_ORDER, MAX_DATA_LEN, MAX_SHARES, MAX_THRESHOLD, SECRET_INDEX,
    append_checksum, charval, interpolate, note,
};

/// The lengths a fresh set is made at (`docs/CODEX32.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Length {
    /// 48 characters: a 16-byte seed, 26 data characters.
    Chars48,
    /// 74 characters: a 32-byte seed, 52 data characters.
    Chars74,
    /// 127 characters: a 64-byte seed, 103 data characters, the long
    /// checksum.
    Chars127,
}

impl Length {
    /// Characters of each string, `ms1` included.
    pub fn chars(self) -> usize {
        match self {
            Length::Chars48 => 48,
            Length::Chars74 => 74,
            Length::Chars127 => 127,
        }
    }

    /// Data characters of each string, between the header and the
    /// checksum.
    pub fn data_len(self) -> usize {
        match self {
            Length::Chars48 => 26,
            Length::Chars74 => 52,
            Length::Chars127 => 103,
        }
    }

    /// Bytes of the master seed the set carries.
    pub fn seed_len(self) -> usize {
        match self {
            Length::Chars48 => 16,
            Length::Chars74 => 32,
            Length::Chars127 => 64,
        }
    }

    /// The length of strings of `chars` characters, where a fresh set
    /// is made at it.
    pub fn of_chars(chars: usize) -> Option<Length> {
        [Length::Chars48, Length::Chars74, Length::Chars127]
            .into_iter()
            .find(|l| l.chars() == chars)
    }
}

/// What a fresh set is: its threshold, how many strings, the identifier
/// and the length. Public: it carries nothing of the seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scheme {
    threshold: u8,
    shares: u8,
    /// The identifier, lowercase.
    identifier: [u8; 4],
    length: Length,
}

impl Scheme {
    /// A scheme of `threshold` (0, or 2 to 9) and `shares` strings (1
    /// where the threshold is 0, else the threshold to 31), with a
    /// four-character identifier in either case. Cheap.
    pub fn new(
        threshold: u8,
        shares: u8,
        identifier: [u8; 4],
        length: Length,
    ) -> Result<Self, Error> {
        let fits = match threshold {
            0 => shares == 1,
            2..=MAX_THRESHOLD => (threshold..=MAX_SHARES).contains(&shares),
            _ => false,
        };
        if !fits {
            return Err(Error::BadSplit);
        }
        let mut lower = [0u8; 4];
        for (slot, &c) in lower.iter_mut().zip(&identifier) {
            charval(c).ok_or(Error::BadIdentifier)?;
            *slot = c.to_ascii_lowercase();
        }
        Ok(Scheme {
            threshold,
            shares,
            identifier: lower,
            length,
        })
    }

    /// The threshold `k`: 0 for the secret alone.
    pub fn threshold(&self) -> u8 {
        self.threshold
    }

    /// How many strings the set has to write: 1 for the secret alone.
    pub fn shares(&self) -> u8 {
        self.shares
    }

    /// The identifier, lowercase.
    pub fn identifier(&self) -> [u8; 4] {
        self.identifier
    }

    /// The length of every string.
    pub fn length(&self) -> Length {
        self.length
    }

    /// The share indices whose data is the person's entropy, in entry
    /// order: `s` alone at threshold 0, else `a`, `c`, `d`, … (`k` of
    /// them).
    pub fn initial_indices(&self) -> &'static [u8] {
        match self.threshold {
            0 => b"s",
            k => &INDEX_ORDER[..usize::from(k)],
        }
    }

    /// The share indices derived from the initial shares, in BIP 93's
    /// order, up to the number of strings; none at threshold 0.
    pub fn derived_indices(&self) -> &'static [u8] {
        match self.threshold {
            0 => b"",
            k => &INDEX_ORDER[usize::from(k)..usize::from(self.shares)],
        }
    }

    /// The header of the string at share index `index`, as symbols:
    /// threshold, identifier, index. Cheap.
    pub fn header(&self, index: u8) -> Result<[u8; HEADER_LEN], Error> {
        let mut out = [0u8; HEADER_LEN];
        out[0] = charval(b'0' + self.threshold).ok_or(Error::BadThreshold)?;
        for (slot, &c) in out[1..5].iter_mut().zip(&self.identifier) {
            *slot = charval(c).ok_or(Error::BadIdentifier)?;
        }
        out[5] = charval(index).ok_or(Error::BadShareIndex)?;
        Ok(out)
    }
}

/// The strings of a fresh set: those to write, in index order (the
/// initial shares, then the derived ones), and the secret. Each string
/// wipes itself on drop.
pub struct FreshSet {
    strings: Vec<Codex32>,
    secret: Codex32,
}

impl FreshSet {
    /// The strings to write, in index order: the initial shares, then
    /// the derived shares; the secret alone at threshold 0.
    pub fn strings(&self) -> &[Codex32] {
        &self.strings
    }

    /// The secret `s` the set carries; at threshold 0, the one string.
    pub fn secret(&self) -> &Codex32 {
        &self.secret
    }
}

/// Costs an interpolation of `k` strings for each string made, and a
/// checksum of each initial share. Makes the set of `scheme` whose
/// initial shares carry `data` (one slice of symbols, `0..32`, per
/// initial index, each [`Length::data_len`] long): each initial share
/// checksummed, the secret and the derived shares interpolated.
pub fn fresh_set(c: &Compute, scheme: &Scheme, data: &[&[u8]]) -> Result<FreshSet, Error> {
    note(c);
    let initial = scheme.initial_indices();
    if data.len() != initial.len() {
        return Err(Error::WrongShareCount);
    }
    let data_len = scheme.length.data_len();
    let mut made = Vec::with_capacity(initial.len());
    for (&index, payload) in initial.iter().zip(data) {
        if payload.len() != data_len {
            return Err(Error::BadLength);
        }
        let mut symbols = [0u8; MAX_DATA_LEN];
        symbols[..HEADER_LEN].copy_from_slice(&scheme.header(index)?);
        for (n, (slot, &v)) in symbols[HEADER_LEN..HEADER_LEN + data_len]
            .iter_mut()
            .zip(payload.iter())
            .enumerate()
        {
            if v >= 32 {
                return Err(Error::BadCharacter {
                    position: (3 + HEADER_LEN + n) as u8,
                });
            }
            *slot = v;
        }
        let len = append_checksum(&mut symbols, HEADER_LEN + data_len);
        made.push(Codex32 {
            data: symbols,
            len: len as u8,
        });
    }

    if scheme.threshold == 0 {
        let s = made.pop().ok_or(Error::WrongShareCount)?;
        let secret = Codex32 {
            data: s.data,
            len: s.len,
        };
        return Ok(FreshSet {
            strings: alloc::vec![s],
            secret,
        });
    }

    let secret = interpolate(&made, SECRET_INDEX);
    let mut strings = made;
    for &index in scheme.derived_indices() {
        let x = charval(index).ok_or(Error::BadShareIndex)?;
        let derived = interpolate(&strings[..initial.len()], x);
        strings.push(derived);
    }
    Ok(FreshSet { strings, secret })
}
