//! Mixing: several sources combined by their commitments.

use crate::{Error, RawEntropy, Strength};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Most sources a mix combines.
pub const MAX_MIX: usize = 5;

/// Fewest sources a mix combines: one source is not a mix.
pub const MIN_MIX: usize = 2;

/// Several sources combined: the entropy is SHA-256 over the sources'
/// 32-byte commitments, concatenated in the order they were taken,
/// truncated to the strength.
///
/// A commitment is SHA-256 of that one source's raw input. Writing the
/// commitments down before the words appear is what makes the mix
/// checkable by hand: the same commitments in the same order always
/// give the same key, so nothing can have been changed afterwards.
pub struct Mixed {
    parts: [[u8; 32]; MAX_MIX],
    len: u8,
}

impl Default for Mixed {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for Mixed {
    fn zeroize(&mut self) {
        for p in &mut self.parts {
            p.zeroize();
        }
        self.len.zeroize();
    }
}

impl Drop for Mixed {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Mixed {}

impl Mixed {
    /// No sources yet.
    pub fn new() -> Self {
        Mixed {
            parts: [[0u8; 32]; MAX_MIX],
            len: 0,
        }
    }

    /// Adds one source's commitment. Returns `false` past [`MAX_MIX`].
    pub fn push(&mut self, commitment: [u8; 32]) -> bool {
        if self.len() >= MAX_MIX {
            return false;
        }
        self.parts[self.len()] = commitment;
        self.len += 1;
        true
    }

    /// Sources taken so far.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether no source has been taken.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The commitments taken so far, in order.
    pub fn commitments(&self) -> &[[u8; 32]] {
        &self.parts[..self.len()]
    }

    /// Forgets every source.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// SHA-256 over the commitments, concatenated in order.
    pub fn commitment(&self) -> [u8; 32] {
        let mut buf = [0u8; MAX_MIX * 32];
        for (i, p) in self.commitments().iter().enumerate() {
            buf[i * 32..(i + 1) * 32].copy_from_slice(p);
        }
        let h = sha256(&buf[..self.len() * 32]);
        buf.zeroize();
        h
    }

    /// The combination, once [`MIN_MIX`] sources are in.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if self.len() < MIN_MIX {
            return Err(Error::TooFew {
                have: self.len(),
                need: MIN_MIX,
            });
        }
        let mut hash = self.commitment();
        let e = RawEntropy::new(&hash[..strength.bytes()]);
        hash.zeroize();
        Ok(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::tests::dice;
    use crate::known::{self, bytes, count, strength};
    use crate::{CoinFlips, Error};

    /// The known answers are `tools/vectors/mix/`, whose README writes out
    /// the construction and a few lines of Python that reproduce them.
    #[test]
    fn a_mix_commits_to_its_sources_and_hashes_the_commitments() {
        let v = known::json(known::MIX);
        let case = &v["vectors"][0];
        let [d, c] = [0, 1].map(|i| &case["sources"][i]);
        assert_eq!(d["source"], "dice");
        assert_eq!(c["source"], "coins");
        // Fifty rolls, then 128 flips, flip i heads when i mod 3 is 0.
        let dice = dice(d["rolls"].as_str().unwrap());
        let mut coins = CoinFlips::new();
        for f in c["flips"].as_str().unwrap().bytes() {
            assert!(coins.push(f == b'H'));
        }
        // Each commitment is SHA-256 of that source's raw input.
        assert_eq!(dice.commitment()[..], bytes(&d["commitment"])[..]);
        assert_eq!(coins.commitment()[..], bytes(&c["commitment"])[..]);

        let at = strength(&case["strength"]);
        let mut m = Mixed::new();
        let refused = |n: usize| {
            let r = &v["refused"][n];
            Some(Error::TooFew {
                have: count(&r["have"]),
                need: count(&r["need"]),
            })
        };
        assert_eq!(m.entropy(at).err(), refused(0));
        assert!(m.push(dice.commitment()));
        assert_eq!(m.entropy(at).err(), refused(1), "one source is not a mix");
        assert!(m.push(coins.commitment()));
        assert_eq!(m.commitments(), &[dice.commitment(), coins.commitment()]);
        // SHA-256 over the two commitments, in the order taken.
        assert_eq!(m.commitment()[..], bytes(&case["commitment"])[..]);
        assert_eq!(
            m.entropy(at).unwrap().as_bytes(),
            &bytes(&case["entropy"])[..]
        );
        // The order is part of the definition.
        let mut n = Mixed::new();
        assert!(n.push(coins.commitment()) && n.push(dice.commitment()));
        assert_ne!(n.commitment(), m.commitment());
        m.clear();
        assert!(m.is_empty());
    }
}
