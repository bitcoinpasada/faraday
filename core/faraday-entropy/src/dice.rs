//! Dice: rolls of a six-sided die, and the published procedures that turn them into a key.

use crate::{
    BITS_PER_ROLL, DICE_CHI_SQUARE_LIMIT, DICE_FOUR_CHI_SQUARE_LIMIT, DICE_LONG_RUN,
    DICE_SEQUENTIAL, DICE_SKEW_MIN, DICE_WORD_FACES, DICE_WORD_ROLLS, Error, MAX_ROLLS, RawEntropy,
    Strength, Warnings,
};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// How rolls of a six-sided die become a key.
/// Each is another signer's published
/// procedure, reproduced so that a key made there is the same key here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiceProcedure {
    /// SHA-256 of the rolls written as the ASCII digits `'1'..='6'`,
    /// truncated to the strength. Coldcard's `rolls.py`, SeedSigner's
    /// `generate_mnemonic_from_dice` and EntropyLab's "Base 10 [0-9] /
    /// Hashed rolls" all hash that one string.
    Hashed,
    /// The same hash over a string in which every `6` is written `0`.
    /// EntropyLab's "Dice [1-6] / Hashed rolls", which it states is
    /// Keystone's method and iancoleman's "Dice" mode.
    SixAsZero,
    /// Direct word selection: [`DICE_WORD_FACES`] rolls of 1–4 name a
    /// word and a sixth roll is a coin, 1–3 heads and 4–6 tails, so one
    /// word is 4⁵ × 2 = 2048 outcomes. A 5 or a 6 in the first five
    /// positions is a reroll and is not kept. EntropyLab's "BitBox
    /// diceware / Direct word selection". The rolls name every word,
    /// the last included, and the checksum then replaces the last
    /// word's low bits, as SeedSigner's `calculate_checksum` completes a
    /// final word the person picked.
    Words,
}

impl DiceProcedure {
    /// Every procedure, in the order the Choice lists them.
    pub const ALL: [DiceProcedure; 3] = [
        DiceProcedure::Hashed,
        DiceProcedure::SixAsZero,
        DiceProcedure::Words,
    ];

    /// Whether the procedure names words directly rather than hashing.
    pub fn direct(self) -> bool {
        self == DiceProcedure::Words
    }

    /// Rolls the procedure needs at `strength`: the hashed procedures
    /// take [`Strength::rolls`], direct selection [`DICE_WORD_ROLLS`]
    /// for every word.
    pub fn needed(self, strength: Strength) -> usize {
        match self {
            DiceProcedure::Hashed | DiceProcedure::SixAsZero => strength.rolls(),
            DiceProcedure::Words => DICE_WORD_ROLLS * strength.words(),
        }
    }

    /// Whether `roll` may be kept as the roll after `have` kept rolls.
    /// Direct selection rerolls a 5 or a 6 in a word's first five
    /// positions; nothing else refuses a face.
    pub fn accepts(self, have: usize, roll: u8) -> bool {
        if !(1..=6).contains(&roll) {
            return false;
        }
        match self {
            DiceProcedure::Words => have % DICE_WORD_ROLLS >= DICE_WORD_FACES || roll <= 4,
            _ => true,
        }
    }
}

/// Per-face counts and run statistics of a roll sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiceStats {
    /// How often each face came up, `counts[0]` for face 1.
    pub counts: [u32; 6],
    /// Longest run of one face.
    pub longest_run: u32,
    /// The face of that run (1–6), or 0 with no rolls.
    pub run_face: u8,
    /// Chi-square statistic against a uniform distribution (5 d.o.f.).
    pub chi_square: f32,
    /// Longest run counting up or down through the faces, wrapping.
    pub longest_sequence: u32,
}

/// Accumulates d6 rolls.
pub struct DiceRolls {
    rolls: [u8; MAX_ROLLS],
    len: u8,
}

impl Default for DiceRolls {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for DiceRolls {
    fn zeroize(&mut self) {
        self.rolls.zeroize();
        self.len.zeroize();
    }
}

impl Drop for DiceRolls {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for DiceRolls {}

impl DiceRolls {
    /// No rolls yet.
    pub fn new() -> Self {
        DiceRolls {
            rolls: [0u8; MAX_ROLLS],
            len: 0,
        }
    }

    /// Adds a roll (1–6). Returns `false` for another value or when full.
    pub fn push(&mut self, roll: u8) -> bool {
        if !(1..=6).contains(&roll) || usize::from(self.len) >= MAX_ROLLS {
            return false;
        }
        self.rolls[usize::from(self.len)] = roll;
        self.len += 1;
        true
    }

    /// Removes the last roll.
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let r = self.rolls[usize::from(self.len)];
        self.rolls[usize::from(self.len)] = 0;
        Some(r)
    }

    /// Forgets every roll.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Number of rolls.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether there are no rolls.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The rolls so far, 1–6 each.
    pub fn rolls(&self) -> &[u8] {
        &self.rolls[..self.len()]
    }

    /// The last roll, if any.
    pub fn last(&self) -> Option<u8> {
        self.rolls().last().copied()
    }

    /// Bits gathered so far: rolls × log2(6).
    pub fn bits_collected(&self) -> f32 {
        self.len() as f32 * BITS_PER_ROLL
    }

    /// Rolls needed for `strength`.
    pub fn needed(strength: Strength) -> usize {
        strength.rolls()
    }

    /// Whether enough rolls are in for `strength`.
    pub fn is_enough(&self, strength: Strength) -> bool {
        self.len() >= Self::needed(strength)
    }

    /// SHA-256 of the rolls as ASCII digits.
    pub fn commitment(&self) -> [u8; 32] {
        let mut ascii = [0u8; MAX_ROLLS];
        for (a, r) in ascii.iter_mut().zip(self.rolls()) {
            *a = b'0' + r;
        }
        let h = sha256(&ascii[..self.len()]);
        ascii.zeroize();
        h
    }

    /// SHA-256 of the rolls as ASCII digits, truncated to the strength.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if !self.is_enough(strength) {
            return Err(Error::TooFew {
                have: self.len(),
                need: Self::needed(strength),
            });
        }
        let mut hash = self.commitment();
        let e = RawEntropy::new(&hash[..strength.bytes()]);
        hash.zeroize();
        Ok(e)
    }

    /// Counts, runs and the chi-square statistic.
    pub fn stats(&self) -> DiceStats {
        let rolls = self.rolls();
        let mut counts = [0u32; 6];
        for &r in rolls {
            counts[usize::from(r - 1)] += 1;
        }
        let (mut run, mut longest_run, mut run_face) = (0u32, 0u32, 0u8);
        let (mut seq, mut longest_sequence) = (0u32, 0u32);
        let mut dir = 0i8;
        for (i, &r) in rolls.iter().enumerate() {
            if i > 0 && rolls[i - 1] == r {
                run += 1;
            } else {
                run = 1;
            }
            if run > longest_run {
                longest_run = run;
                run_face = r;
            }
            // +1 for counting up (6 wraps to 1), -1 for down, 0 otherwise.
            let step = if i == 0 {
                0
            } else {
                let prev = i16::from(rolls[i - 1]);
                let d = (i16::from(r) - prev).rem_euclid(6);
                match d {
                    1 => 1,
                    5 => -1,
                    _ => 0,
                }
            };
            if step != 0 && step == dir {
                seq += 1;
            } else if step != 0 {
                dir = step;
                seq = 2;
            } else {
                dir = 0;
                seq = 1;
            }
            longest_sequence = longest_sequence.max(seq);
        }
        let n = rolls.len() as f32;
        let chi_square = if rolls.is_empty() {
            0.0
        } else {
            let sum_sq: f32 = counts.iter().map(|&c| (c as f32) * (c as f32)).sum();
            6.0 * sum_sq / n - n
        };
        DiceStats {
            counts,
            longest_run,
            run_face,
            chi_square,
            longest_sequence,
        }
    }

    /// The sanity flags for `strength`.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        let s = self.stats();
        Warnings {
            too_few: !self.is_enough(strength),
            long_run: s.longest_run >= DICE_LONG_RUN,
            skewed: self.len() >= DICE_SKEW_MIN && s.chi_square > DICE_CHI_SQUARE_LIMIT,
            sequential: s.longest_sequence >= DICE_SEQUENTIAL,
            in_order: false,
        }
    }

    // ----- the other published procedures

    /// Adds a roll under `procedure`. Returns `false` for a face the
    /// procedure rerolls, for a value that is not 1–6, or when full.
    pub fn push_under(&mut self, roll: u8, procedure: DiceProcedure) -> bool {
        procedure.accepts(self.len(), roll) && self.push(roll)
    }

    /// Whether enough rolls are in for `strength` under `procedure`.
    pub fn is_enough_under(&self, strength: Strength, procedure: DiceProcedure) -> bool {
        self.len() >= procedure.needed(strength)
    }

    /// Words the rolls have named so far under
    /// [`DiceProcedure::Words`]: one per whole group of
    /// [`DICE_WORD_ROLLS`].
    pub fn word_count(&self) -> usize {
        self.len() / DICE_WORD_ROLLS
    }

    /// The BIP-39 index a whole group of rolls names, as EntropyLab's
    /// `hodlBitBoxLookupWord` computes it: the five faces as base-4
    /// digits, most significant first, then the coin bit.
    pub fn word_index(&self, word: usize) -> Option<u16> {
        let group = self
            .rolls()
            .get(word * DICE_WORD_ROLLS..(word + 1) * DICE_WORD_ROLLS)?;
        let mut index = 0u16;
        for &face in &group[..DICE_WORD_FACES] {
            if !(1..=4).contains(&face) {
                return None;
            }
            index = index * 4 + u16::from(face - 1);
        }
        Some(index * 2 + u16::from(group[DICE_WORD_FACES] >= 4))
    }

    /// Every index the rolls have named, in order.
    pub fn word_indices(&self) -> impl Iterator<Item = u16> + '_ {
        (0..self.word_count()).filter_map(|w| self.word_index(w))
    }

    /// The rolls under `procedure`, truncated to the strength.
    ///
    /// Under [`DiceProcedure::Words`] the entropy is the named words'
    /// 11-bit indices written one after another, cut to the strength's
    /// bits: every bit of every word but the last, and the last word's
    /// high `11 − checksum` bits. BIP-39 then writes the checksum into
    /// the bits that were cut, so the key's last word is the rolled
    /// word with its low bits replaced.
    pub fn entropy_under(
        &self,
        strength: Strength,
        procedure: DiceProcedure,
    ) -> Result<RawEntropy, Error> {
        match procedure {
            DiceProcedure::Hashed => self.entropy(strength),
            DiceProcedure::SixAsZero => {
                if !self.is_enough(strength) {
                    return Err(Error::TooFew {
                        have: self.len(),
                        need: Self::needed(strength),
                    });
                }
                let mut ascii = [0u8; MAX_ROLLS];
                for (a, &r) in ascii.iter_mut().zip(self.rolls()) {
                    *a = if r == 6 { b'0' } else { b'0' + r };
                }
                let mut hash = sha256(&ascii[..self.len()]);
                ascii.zeroize();
                let e = RawEntropy::new(&hash[..strength.bytes()]);
                hash.zeroize();
                Ok(e)
            }
            DiceProcedure::Words => {
                if !self.is_enough_under(strength, procedure) {
                    return Err(Error::TooFew {
                        have: self.len(),
                        need: procedure.needed(strength),
                    });
                }
                let mut bytes = [0u8; 32];
                let mut bit = 0usize;
                for word in 0..strength.words() {
                    // A group kept by `push` rather than `push_under` can
                    // hold a 5 or a 6 where only 1–4 name a word.
                    let Some(index) = self.word_index(word) else {
                        bytes.zeroize();
                        return Err(Error::BadLength);
                    };
                    for b in (0..11).rev() {
                        if bit == strength.bits() {
                            break;
                        }
                        if index >> b & 1 == 1 {
                            bytes[bit / 8] |= 0x80 >> (bit % 8);
                        }
                        bit += 1;
                    }
                }
                let e = RawEntropy::new(&bytes[..strength.bytes()]);
                bytes.zeroize();
                Ok(e)
            }
        }
    }

    /// Counts over the two streams direct selection keeps apart, as
    /// EntropyLab's own fairness analysis splits them: the faces 1–4 of
    /// the word positions, and heads and tails of the coin positions.
    pub fn word_stats(&self) -> (([u32; 4], f32), (u32, u32)) {
        let mut faces = [0u32; 4];
        let (mut heads, mut tails) = (0u32, 0u32);
        for (i, &r) in self.rolls().iter().enumerate() {
            if i % DICE_WORD_ROLLS >= DICE_WORD_FACES {
                if r <= 3 {
                    heads += 1;
                } else {
                    tails += 1;
                }
            } else if (1..=4).contains(&r) {
                faces[usize::from(r - 1)] += 1;
            }
        }
        let n: u32 = faces.iter().sum();
        let chi_square = if n == 0 {
            0.0
        } else {
            let sum_sq: f32 = faces.iter().map(|&c| (c as f32) * (c as f32)).sum();
            4.0 * sum_sq / n as f32 - n as f32
        };
        ((faces, chi_square), (heads, tails))
    }

    /// The sanity flags for `strength` under `procedure`. Direct
    /// selection keeps only 1–4 in a word's first five positions, so the
    /// six-face chi-square would flag every honest run; it is checked as
    /// two streams instead, and a sequence running up or down means
    /// nothing across them.
    pub fn warnings_under(&self, strength: Strength, procedure: DiceProcedure) -> Warnings {
        if !procedure.direct() {
            let mut w = self.warnings(strength);
            w.too_few = !self.is_enough_under(strength, procedure);
            return w;
        }
        let s = self.stats();
        let ((faces, chi_square), (heads, tails)) = self.word_stats();
        let n = i64::from(heads + tails);
        let d = 2 * i64::from(heads) - n;
        let face_n: u32 = faces.iter().sum();
        Warnings {
            too_few: !self.is_enough_under(strength, procedure),
            long_run: s.longest_run >= DICE_LONG_RUN,
            skewed: (usize::try_from(face_n).unwrap_or(0) >= DICE_SKEW_MIN
                && chi_square > DICE_FOUR_CHI_SQUARE_LIMIT)
                || (n > 0 && d * d * 4 > 49 * n),
            sequential: false,
            in_order: false,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{DICE_CHI_SQUARE_LIMIT, Error, MAX_ROLLS, Strength, Warnings};

    pub(crate) fn dice(s: &str) -> DiceRolls {
        let mut d = DiceRolls::new();
        for c in s.bytes() {
            assert!(d.push(c - b'0'));
        }
        d
    }

    fn hex(bytes: &[u8]) -> [u8; 64] {
        const D: &[u8; 16] = b"0123456789abcdef";
        let mut out = [0u8; 64];
        for (i, b) in bytes.iter().enumerate() {
            out[2 * i] = D[usize::from(b >> 4)];
            out[2 * i + 1] = D[usize::from(b & 15)];
        }
        out
    }

    // Expected digests were produced while writing the test with
    // `python3 -c 'import hashlib; print(hashlib.sha256(b"<rolls>").hexdigest())'`,
    // the same computation as `echo -n <rolls> | sha256sum`.
    pub(crate) const RANDOM_50: &str = "32461151351521144121541512665155412152342515356215";
    const RANDOM_50_SHA256: &str =
        "b4b7bf155511dfa13688d93685fea50b2d56414be59fbde4658b84b42f355fa7";
    const CYCLE_50: &str = "12345612345612345612345612345612345612345612345611";
    const CYCLE_50_SHA256: &str =
        "1f9103d58953998e93a27fc574b67f8842abb85a0d965073d4a5139f90bdf99d";
    const RUN_99: &str = "666666665321453214532145321453214532145321453214532145321453214532145321453214532145321453214532141";
    const RUN_99_SHA256: &str = "65ef0c573b13fcbf023be89af49b7046e03f9f76621bfa2d3f8d2e0f4b83f13e";

    #[test]
    fn dice_entropy_is_sha256_of_the_ascii_rolls() {
        let d = dice(RANDOM_50);
        let e = d.entropy(Strength::Bits128).unwrap();
        assert_eq!(e.as_bytes().len(), 16);
        assert_eq!(&hex(e.as_bytes())[..32], &RANDOM_50_SHA256.as_bytes()[..32]);
        assert_eq!(e.strength(), Strength::Bits128);
        // 256 bits needs 99 rolls.
        assert_eq!(
            d.entropy(Strength::Bits256).err(),
            Some(Error::TooFew { have: 50, need: 99 })
        );
        let d = dice(RUN_99);
        let e = d.entropy(Strength::Bits256).unwrap();
        assert_eq!(&hex(e.as_bytes())[..], RUN_99_SHA256.as_bytes());
        let e = dice(CYCLE_50).entropy(Strength::Bits128).unwrap();
        assert_eq!(&hex(e.as_bytes())[..32], &CYCLE_50_SHA256.as_bytes()[..32]);
    }

    // EntropyLab's "Dice [1-6] / Hashed rolls" over the same two
    // transcripts, computed by running its own `hodlDiceEntropy` with
    // method `coleman` (tools/reference/dice/README.md).
    const RANDOM_50_SIX_AS_ZERO: &str = "8bc19a488ea41da254da5f35c041ba3a";
    const RUN_99_SIX_AS_ZERO: &str =
        "dc08d10f0056ddb466b66682cef727f213a4da9ff8c0997e0f3a1fe1d0dfac89";

    #[test]
    fn six_as_zero_hashes_the_rolls_with_every_six_written_as_a_zero() {
        let d = dice(RANDOM_50);
        let e = d
            .entropy_under(Strength::Bits128, DiceProcedure::SixAsZero)
            .unwrap();
        assert_eq!(
            &hex(e.as_bytes())[..32],
            RANDOM_50_SIX_AS_ZERO.as_bytes(),
            "six as zero"
        );
        // A transcript with no six is the hashed procedure's own result.
        let plain = dice("12345123451234512345123451234512345123451234512345");
        assert_eq!(
            plain
                .entropy_under(Strength::Bits128, DiceProcedure::SixAsZero)
                .unwrap()
                .as_bytes(),
            plain.entropy(Strength::Bits128).unwrap().as_bytes(),
        );
        let d = dice(RUN_99);
        let e = d
            .entropy_under(Strength::Bits256, DiceProcedure::SixAsZero)
            .unwrap();
        assert_eq!(&hex(e.as_bytes())[..], RUN_99_SIX_AS_ZERO.as_bytes());
        assert_eq!(
            dice(RANDOM_50)
                .entropy_under(Strength::Bits256, DiceProcedure::SixAsZero)
                .err(),
            Some(Error::TooFew { have: 50, need: 99 })
        );
    }

    #[test]
    fn direct_selection_names_a_word_from_five_faces_and_a_coin() {
        // Six rolls a word, for every word.
        assert_eq!(DiceProcedure::Words.needed(Strength::Bits128), 72);
        assert_eq!(DiceProcedure::Words.needed(Strength::Bits256), 144);
        assert_eq!(DiceProcedure::Hashed.needed(Strength::Bits128), 50);
        assert_eq!(DiceProcedure::SixAsZero.needed(Strength::Bits256), 99);

        // The lowest and the highest word of the list, and the index one
        // step above the lowest, as EntropyLab's own lookup gives them.
        let mut d = DiceRolls::new();
        for face in [1, 1, 1, 1, 1, 1] {
            assert!(d.push_under(face, DiceProcedure::Words));
        }
        assert_eq!(d.word_index(0), Some(0));
        for face in [1, 1, 1, 1, 2, 3] {
            assert!(d.push_under(face, DiceProcedure::Words));
        }
        assert_eq!(d.word_index(1), Some(2));
        for face in [4, 4, 4, 4, 4, 6] {
            assert!(d.push_under(face, DiceProcedure::Words));
        }
        assert_eq!(d.word_index(2), Some(2047));
        assert_eq!(d.word_count(), 3);

        // A 5 or a 6 in a word's first five places is rolled again and
        // is not kept; the sixth roll takes all six faces, 1 to 3 heads
        // and 4 to 6 tails.
        let mut d = DiceRolls::new();
        for face in [5, 6, 1, 5, 1, 1, 1, 6, 1, 4] {
            d.push_under(face, DiceProcedure::Words);
        }
        assert_eq!(d.rolls(), &[1, 1, 1, 1, 1, 4]);
        assert_eq!(d.word_index(0), Some(1), "4 on the sixth roll is tails");
        assert!(!DiceProcedure::Words.accepts(0, 5));
        assert!(DiceProcedure::Words.accepts(5, 5));
        assert!(DiceProcedure::Hashed.accepts(0, 5));
    }

    /// The rolls of EntropyLab's BitBox transcript name every word, the
    /// last included, and the entropy is what SeedSigner's
    /// `calculate_checksum` makes of those words: embit's
    /// `mnemonic_to_bytes(..., ignore_checksum=True)`
    /// (`tools/reference/dice/README.md`).
    #[test]
    fn direct_selection_rolls_the_last_word_and_keeps_its_high_bits() {
        const ROLLS_12: &str =
            "123411234122341233412344123415234126341231412342123413234124341235432141";
        let mut d = DiceRolls::new();
        for c in ROLLS_12.bytes() {
            assert!(d.push_under(c - b'0', DiceProcedure::Words));
        }
        assert!(d.is_enough_under(Strength::Bits128, DiceProcedure::Words));
        let e = d
            .entropy_under(Strength::Bits128, DiceProcedure::Words)
            .unwrap();
        assert_eq!(
            e.as_bytes(),
            &[
                0x1b, 0x0d, 0x8a, 0xc6, 0x63, 0x71, 0xb2, 0xd8, 0xec, 0x66, 0x36, 0x1b, 0x0d, 0x8e,
                0xc6, 0xf2
            ]
        );

        // Twenty-four words: the same 72 rolls twice.
        for c in ROLLS_12.bytes() {
            assert!(d.push_under(c - b'0', DiceProcedure::Words));
        }
        let e = d
            .entropy_under(Strength::Bits256, DiceProcedure::Words)
            .unwrap();
        assert_eq!(
            e.as_bytes(),
            &[
                0x1b, 0x0d, 0x8a, 0xc6, 0x63, 0x71, 0xb2, 0xd8, 0xec, 0x66, 0x36, 0x1b, 0x0d, 0x8e,
                0xc6, 0xf2, 0x61, 0xb0, 0xd8, 0xac, 0x66, 0x37, 0x1b, 0x2d, 0x8e, 0xc6, 0x63, 0x61,
                0xb0, 0xd8, 0xec, 0x6f
            ]
        );

        // Eleven words' rolls are not enough for twelve.
        let mut d = DiceRolls::new();
        for c in ROLLS_12[..66].bytes() {
            d.push_under(c - b'0', DiceProcedure::Words);
        }
        assert_eq!(
            d.entropy_under(Strength::Bits128, DiceProcedure::Words)
                .err(),
            Some(Error::TooFew { have: 66, need: 72 })
        );
    }

    #[test]
    fn dice_bit_accounting() {
        let mut d = DiceRolls::new();
        assert_eq!(d.bits_collected(), 0.0);
        assert!(!d.push(0) && !d.push(7));
        for _ in 0..49 {
            assert!(d.push(3));
        }
        assert!((d.bits_collected() - 126.66).abs() < 0.1);
        assert!(!d.is_enough(Strength::Bits128));
        assert!(d.push(4));
        assert!(d.is_enough(Strength::Bits128));
        assert!((d.bits_collected() - 129.25).abs() < 0.1);
        assert_eq!(DiceRolls::needed(Strength::Bits256), 99);
        assert_eq!(d.pop(), Some(4));
        assert_eq!(d.len(), 49);
        for _ in 0..(MAX_ROLLS - 49) {
            assert!(d.push(1));
        }
        assert!(!d.push(1), "full at {MAX_ROLLS}");
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn dice_warnings_fire_on_constructed_sequences_only() {
        let s = dice(RANDOM_50).stats();
        assert_eq!(s.counts, [15, 8, 4, 6, 13, 4]);
        assert_eq!(s.longest_run, 2);
        assert!((s.chi_square - 13.12).abs() < 0.01);
        assert_eq!(
            dice(RANDOM_50).warnings(Strength::Bits128),
            Warnings::default()
        );

        let w = dice(CYCLE_50).warnings(Strength::Bits128);
        assert!(w.sequential && !w.long_run && !w.skewed && !w.too_few);
        let down = "654321654321654321654321654321654321654321654321";
        assert!(dice(down).stats().longest_sequence >= 12);
        assert!(dice(down).warnings(Strength::Bits128).sequential);
        // 6-5-4-3-2-1 ten times with a break every eleven: no sequence.
        assert!(
            !dice("65432165432265432165432265432165432265432165432265")
                .warnings(Strength::Bits128)
                .sequential
        );

        let w = dice(RUN_99).warnings(Strength::Bits256);
        assert!(w.long_run && !w.sequential && !w.skewed);
        assert_eq!(dice(RUN_99).stats().run_face, 6);
        assert!(
            !dice("6666666").warnings(Strength::Bits128).long_run,
            "seven is not a run"
        );

        // 22 ones spread among 28 other faces: no run, but chi-square ≈ 22.
        let mut skewed = DiceRolls::new();
        let (mut ones, mut other) = (0, 2u8);
        for i in 0..50 {
            if i % 2 == 0 && ones < 22 {
                skewed.push(1);
                ones += 1;
            } else {
                skewed.push(other);
                other = if other == 6 { 2 } else { other + 1 };
            }
        }
        let s = skewed.stats();
        assert_eq!(s.counts[0], 22);
        assert!(s.chi_square > DICE_CHI_SQUARE_LIMIT, "{}", s.chi_square);
        let w = skewed.warnings(Strength::Bits128);
        assert!(w.skewed && !w.long_run && !w.sequential);
        // Under 50 rolls the skew check is silent.
        let mut few = DiceRolls::new();
        for _ in 0..20 {
            few.push(1);
            few.push(2);
        }
        let w = few.warnings(Strength::Bits128);
        assert!(w.too_few && !w.skewed);
        assert!(!dice("1").warnings(Strength::Bits128).skewed);
    }
}
