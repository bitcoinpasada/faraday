//! Seed XOR: Coldcard's split of a seed into parts whose bitwise XOR is the seed.

use crate::{Error, RawEntropy, Strength};
use faraday_crypto::{Zeroize, ZeroizeOnDrop};

/// Most parts a Seed XOR split or combine takes.
pub const MAX_XOR_PARTS: usize = 4;

/// Fewest parts a Seed XOR has: one part alone is the seed itself.
pub const MIN_XOR_PARTS: usize = 2;

/// Coldcard's Seed XOR: a seed's entropy split into parts of the same
/// length whose bitwise XOR is the seed again. Every part is itself a
/// valid BIP-39 mnemonic of the same word count, and every part is
/// needed: XOR is not a threshold scheme.
///
/// The accumulator is the same for both directions. Combining is
/// pushing every part and reading the entropy; splitting is pushing the
/// random parts and the seed, and reading the last part out.
pub struct SeedXor {
    acc: [u8; 32],
    len: u8,
    parts: u8,
}

impl Default for SeedXor {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for SeedXor {
    fn zeroize(&mut self) {
        self.acc.zeroize();
        self.len.zeroize();
        self.parts.zeroize();
    }
}

impl Drop for SeedXor {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SeedXor {}

impl SeedXor {
    /// No parts yet.
    pub fn new() -> Self {
        SeedXor {
            acc: [0u8; 32],
            len: 0,
            parts: 0,
        }
    }

    /// Adds one part: a BIP-39 length in bytes (16, 20, 24, 28 or 32),
    /// the same length as the parts already added. A part of another
    /// length is refused, and so is one past [`MAX_XOR_PARTS`].
    pub fn push(&mut self, part: &[u8]) -> Result<(), Error> {
        if Strength::for_bytes(part.len()).is_none() {
            return Err(Error::BadLength);
        }
        if self.parts > 0 && usize::from(self.len) != part.len() {
            return Err(Error::BadLength);
        }
        if usize::from(self.parts) >= MAX_XOR_PARTS {
            return Err(Error::BadLength);
        }
        for (a, b) in self.acc.iter_mut().zip(part) {
            *a ^= b;
        }
        self.len = part.len() as u8;
        self.parts += 1;
        Ok(())
    }

    /// Parts added so far.
    pub fn parts(&self) -> usize {
        usize::from(self.parts)
    }

    /// Whether no part has been added.
    pub fn is_empty(&self) -> bool {
        self.parts == 0
    }

    /// The strength every part has, once one is added.
    pub fn strength(&self) -> Option<Strength> {
        Strength::for_bytes(usize::from(self.len))
    }

    /// The XOR of the parts. Fewer than [`MIN_XOR_PARTS`] is refused:
    /// one part is the seed itself and nothing was split.
    pub fn entropy(&self) -> Result<RawEntropy, Error> {
        if usize::from(self.parts) < MIN_XOR_PARTS {
            return Err(Error::TooFew {
                have: usize::from(self.parts),
                need: MIN_XOR_PARTS,
            });
        }
        Ok(RawEntropy::new(&self.acc[..usize::from(self.len)]))
    }
}
