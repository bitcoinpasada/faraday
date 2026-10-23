//! Coins: each flip is one bit, packed most significant bit first.

use crate::{COIN_LONG_RUN, Error, MAX_FLIPS, RawEntropy, Strength, Warnings};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Counts and runs of a flip sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinStats {
    /// Heads.
    pub heads: u32,
    /// Tails.
    pub tails: u32,
    /// Longest run of one side.
    pub longest_run: u32,
    /// Whether that run was heads.
    pub run_heads: bool,
}

/// Accumulates coin flips: heads is a 1 bit, tails a 0 bit.
pub struct CoinFlips {
    flips: [u8; MAX_FLIPS],
    len: u16,
}

impl Default for CoinFlips {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for CoinFlips {
    fn zeroize(&mut self) {
        self.flips.zeroize();
        self.len.zeroize();
    }
}

impl Drop for CoinFlips {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CoinFlips {}

impl CoinFlips {
    /// No flips yet.
    pub fn new() -> Self {
        CoinFlips {
            flips: [0u8; MAX_FLIPS],
            len: 0,
        }
    }

    /// Adds a flip. Returns `false` when full.
    pub fn push(&mut self, heads: bool) -> bool {
        if usize::from(self.len) >= MAX_FLIPS {
            return false;
        }
        self.flips[usize::from(self.len)] = u8::from(heads);
        self.len += 1;
        true
    }

    /// Removes the last flip.
    pub fn pop(&mut self) -> Option<bool> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let f = self.flips[usize::from(self.len)];
        self.flips[usize::from(self.len)] = 0;
        Some(f == 1)
    }

    /// Forgets every flip.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Number of flips.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether there are no flips.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The flips so far, `true` for heads.
    pub fn flips(&self) -> impl Iterator<Item = bool> + '_ {
        self.flips[..self.len()].iter().map(|&f| f == 1)
    }

    /// The last flip, if any.
    pub fn last(&self) -> Option<bool> {
        self.flips().last()
    }

    /// Bits gathered so far: one per flip.
    pub fn bits_collected(&self) -> f32 {
        self.len() as f32
    }

    /// Flips needed for `strength`.
    pub fn needed(strength: Strength) -> usize {
        strength.flips()
    }

    /// Whether enough flips are in for `strength`.
    pub fn is_enough(&self, strength: Strength) -> bool {
        self.len() >= Self::needed(strength)
    }

    /// SHA-256 of the flips written as the ASCII letters `H` and `T`.
    /// The packed bits are the entropy; this is the statement of what
    /// was flipped, which is what a mix commits to.
    pub fn commitment(&self) -> [u8; 32] {
        let mut ascii = [0u8; MAX_FLIPS];
        for (a, heads) in ascii.iter_mut().zip(self.flips()) {
            *a = if heads { b'H' } else { b'T' };
        }
        let h = sha256(&ascii[..self.len()]);
        ascii.zeroize();
        h
    }

    /// The first `bits` flips packed MSB-first: flip 0 is bit 7 of byte 0.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if !self.is_enough(strength) {
            return Err(Error::TooFew {
                have: self.len(),
                need: Self::needed(strength),
            });
        }
        let mut bytes = [0u8; 32];
        for (i, heads) in self.flips().take(strength.bits()).enumerate() {
            if heads {
                bytes[i / 8] |= 0x80 >> (i % 8);
            }
        }
        let e = RawEntropy::new(&bytes[..strength.bytes()]);
        bytes.zeroize();
        Ok(e)
    }

    /// Counts and the longest run.
    pub fn stats(&self) -> CoinStats {
        let mut heads = 0u32;
        let (mut run, mut longest_run, mut run_heads) = (0u32, 0u32, false);
        let mut prev: Option<bool> = None;
        for f in self.flips() {
            if f {
                heads += 1;
            }
            run = if prev == Some(f) { run + 1 } else { 1 };
            if run > longest_run {
                longest_run = run;
                run_heads = f;
            }
            prev = Some(f);
        }
        CoinStats {
            heads,
            tails: self.len() as u32 - heads,
            longest_run,
            run_heads,
        }
    }

    /// The sanity flags for `strength`. Skew means the heads count is more
    /// than 3.5 standard deviations (σ = √n / 2) from n / 2, checked
    /// without a square root as `(2h − n)² · 4 > 49 n`.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        let s = self.stats();
        let n = self.len() as i64;
        let d = 2 * i64::from(s.heads) - n;
        Warnings {
            too_few: !self.is_enough(strength),
            long_run: s.longest_run >= COIN_LONG_RUN,
            skewed: n > 0 && d * d * 4 > 49 * n,
            sequential: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, MAX_FLIPS, Strength, Warnings};

    #[test]
    fn coins_pack_msb_first_without_hashing() {
        let mut c = CoinFlips::new();
        // 1000 0000 0000 0001 then 112 tails: 0x80, 0x01, 0…
        c.push(true);
        for _ in 0..14 {
            c.push(false);
        }
        c.push(true);
        for _ in 0..112 {
            c.push(false);
        }
        assert_eq!(c.len(), 128);
        assert!(c.is_enough(Strength::Bits128));
        let e = c.entropy(Strength::Bits128).unwrap();
        assert_eq!(e.as_bytes()[0], 0x80);
        assert_eq!(e.as_bytes()[1], 0x01);
        assert!(e.as_bytes()[2..].iter().all(|&b| b == 0));
        assert_eq!(
            c.entropy(Strength::Bits256).err(),
            Some(Error::TooFew {
                have: 128,
                need: 256
            })
        );
        assert_eq!(c.bits_collected(), 128.0);
        assert_eq!(c.pop(), Some(false));
        assert!(!c.is_enough(Strength::Bits128));
        for _ in 0..(MAX_FLIPS - 127) {
            assert!(c.push(true));
        }
        assert!(!c.push(true), "full");
        // Extra flips beyond the strength are ignored.
        let e = c.entropy(Strength::Bits128).unwrap();
        assert_eq!(e.as_bytes()[0], 0x80);
    }

    #[test]
    fn coin_warnings() {
        let mut fair = CoinFlips::new();
        for i in 0..128 {
            // A short repeating pattern: 5 heads in a row at most, 65 heads.
            fair.push(matches!(i % 10, 0..=4));
        }
        assert_eq!(fair.stats().heads, 65);
        assert_eq!(fair.stats().longest_run, 5);
        assert_eq!(fair.warnings(Strength::Bits128), Warnings::default());

        let mut run = CoinFlips::new();
        for i in 0..128 {
            run.push(i < 12 || i % 2 == 0);
        }
        let w = run.warnings(Strength::Bits128);
        assert!(w.long_run && !w.skewed);
        assert!(run.stats().run_heads);

        // 100 heads of 128: 3.5 σ is 19.8 from 64.
        let mut skew = CoinFlips::new();
        for i in 0..128 {
            skew.push(i % 9 != 0 || i < 28);
        }
        let s = skew.stats();
        assert!(s.heads >= 84, "{}", s.heads);
        assert!(skew.warnings(Strength::Bits128).skewed);
        let mut edge = CoinFlips::new();
        for i in 0..128 {
            edge.push(i % 2 == 0 || i % 7 == 1);
        }
        let s = edge.stats();
        assert!(s.heads < 84 && s.heads > 44, "{}", s.heads);
        assert!(!edge.warnings(Strength::Bits128).skewed);
        assert!(CoinFlips::new().warnings(Strength::Bits128).too_few);
    }
}
