//! User-supplied entropy (`docs/PLANNING.md` §2.2 "weak entropy", §8.1
//! #1, #2, #6): dice rolls, coin flips and raw hex, accumulated in fixed
//! buffers that are erased on drop, turned into the 128, 160, 192, 224
//! or 256 bits of [`RawEntropy`] a BIP-39 key of 12, 15, 18, 21 or 24
//! words takes, and checked for the mistakes real hands make.
//!
//! Conventions (`docs/PLANNING.md` §16.17):
//!
//! - **Dice** ([`DiceRolls`], [`DiceProcedure`]): three published
//!   procedures read the same rolls (§16.115). The default is the
//!   entropy as SHA-256 of the rolls written
//!   as the ASCII digits `'1'..='6'`, truncated to the strength's bytes.
//!   This is what Coldcard and SeedSigner do, so a user can reproduce the
//!   result elsewhere: `echo -n 3246115135… | sha256sum`. The count is the
//!   bits over log2 6 ≈ 2.585, rounded up: 50, 62, 75 and 87. 256 bits
//!   asks for 99 rather than the 100 the rule gives, because 99 rolls
//!   carry 255.9 bits and that is what the other tools accept. The second
//!   writes every 6 as a 0 first (Keystone). The third names a word
//!   outright from five rolls of 1-4 and a sixth read as a coin
//!   (BitBox), 66 rolls at 128 bits and 138 at 256, and the last word
//!   is the one the checksum leaves that the person chooses.
//! - **Coins** ([`CoinFlips`]): each flip is one bit, packed MSB-first
//!   (the first flip is the top bit of the first byte), no hashing. A fair
//!   coin already gives uniform bits, and packing is what every other tool
//!   does, so a user can check the hex by hand. One flip a bit: 128, 160,
//!   192, 224 or 256.
//! - **Hex** ([`RawHex`]): two digits a byte — 32, 40, 48, 56 or 64 —
//!   taken as they are.
//! - **Cards** ([`CardDraws`]): the entropy is SHA-256 of the draws'
//!   indices, one byte each. A deck tracker refuses a card already out,
//!   so each draw is worth log2 of the cards left: 25, 31, 39, 50 and 58
//!   draws, the last more than one deck holds.
//! - **Camera** ([`CameraNoise`]): the entropy is SHA-256 over the
//!   SHA-256 digests of the frames taken, in order. Three frames per 128
//!   bits, rounded up: three at 128, six at 256.
//! - **The device** ([`DeviceRandom`]): the 32 bytes the shell answered
//!   the core's entropy request with, hashed.
//! - **A mix** ([`Mixed`]): SHA-256 over the sources' commitments,
//!   concatenated in the order they were taken.
//!
//! Every source states a **commitment**: SHA-256 of its raw input, which
//! is what a mix combines and what a person can write down before the
//! words appear and check afterwards. For dice, cards, the camera, the
//! device and a mix the source's own entropy is that commitment
//! truncated; coins and hex are defined otherwise (packed bits, the
//! digits themselves) and their commitment is a separate statement of
//! what went in.
//!
//! The sanity checks ([`DiceRolls::warnings`], [`CoinFlips::warnings`])
//! are cautions for the user, never hard blocks, except `too_few`. The
//! crate is `no_std` and does not allocate.

#![no_std]

use osk_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Most dice rolls an accumulator holds.
pub const MAX_ROLLS: usize = 200;
/// Most coin flips an accumulator holds.
pub const MAX_FLIPS: usize = 256;
/// Most hex digits an accumulator holds.
pub const MAX_HEX: usize = 64;

/// Bits per d6 roll, log2(6).
pub const BITS_PER_ROLL: f32 = 2.584_962_5;

/// Identical rolls in a row that draw a caution.
pub const DICE_LONG_RUN: u32 = 8;
/// Consecutive rolls following 1-2-3-4-5-6 (or the reverse) that draw a
/// caution.
pub const DICE_SEQUENTIAL: u32 = 12;
/// Chi-square over 5 degrees of freedom at the 0.999 quantile (20.515):
/// rolls with a larger statistic are skewed beyond what one fair sequence
/// in a thousand would show. Checked only from [`DICE_SKEW_MIN`] rolls.
pub const DICE_CHI_SQUARE_LIMIT: f32 = 20.5;
/// Fewest rolls for the chi-square check to mean anything.
pub const DICE_SKEW_MIN: usize = 50;
/// Identical flips in a row that draw a caution.
pub const COIN_LONG_RUN: u32 = 12;

/// Rolls one word of [`DiceProcedure::Words`] takes: five that name the
/// word and a sixth read as a coin.
pub const DICE_WORD_ROLLS: usize = 6;
/// Rolls of a word that name it; the rest of [`DICE_WORD_ROLLS`] is the
/// coin.
pub const DICE_WORD_FACES: usize = 5;
/// Chi-square over 3 degrees of freedom at the 0.999 quantile (16.266):
/// the limit for the four faces [`DiceProcedure::Words`] keeps.
pub const DICE_FOUR_CHI_SQUARE_LIMIT: f32 = 16.2;

/// How much entropy the caller wants: one per BIP-39 length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    /// 128 bits: 12 words.
    Bits128,
    /// 160 bits: 15 words.
    Bits160,
    /// 192 bits: 18 words.
    Bits192,
    /// 224 bits: 21 words.
    Bits224,
    /// 256 bits: 24 words.
    Bits256,
}

impl Strength {
    /// Every strength, shortest first.
    pub const ALL: [Strength; 5] = [
        Strength::Bits128,
        Strength::Bits160,
        Strength::Bits192,
        Strength::Bits224,
        Strength::Bits256,
    ];

    /// Bits.
    pub fn bits(self) -> usize {
        match self {
            Strength::Bits128 => 128,
            Strength::Bits160 => 160,
            Strength::Bits192 => 192,
            Strength::Bits224 => 224,
            Strength::Bits256 => 256,
        }
    }

    /// Bytes.
    pub fn bytes(self) -> usize {
        self.bits() / 8
    }

    /// BIP-39 words this strength yields: the bits plus a checksum of
    /// one bit per four bytes, over eleven bits a word.
    pub fn words(self) -> usize {
        (self.bits() + self.bytes() / 4) / 11
    }

    /// The strength for a BIP-39 word count: 12, 15, 18, 21 or 24.
    pub fn for_words(words: usize) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.words() == words)
    }

    /// The strength for a byte count: 16, 20, 24, 28 or 32.
    pub fn for_bytes(bytes: usize) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.bytes() == bytes)
    }

    /// Dice rolls needed: the bits divided by [`BITS_PER_ROLL`], rounded
    /// up — 50, 62, 75 and 87. 256 bits is the exception: the rule gives
    /// 100 and 99 is asked for, because 99 rolls carry 255.91 bits and
    /// that is the count Coldcard and SeedSigner take.
    pub fn rolls(self) -> usize {
        if self == Strength::Bits256 {
            return 99;
        }
        let mut n = 0;
        while (n as f32) * BITS_PER_ROLL < self.bits() as f32 {
            n += 1;
        }
        n
    }

    /// Coin flips needed: one per bit.
    pub fn flips(self) -> usize {
        self.bits()
    }

    /// Hex digits needed: two per byte.
    pub fn hex_digits(self) -> usize {
        self.bytes() * 2
    }
}

/// Why entropy could not be produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not enough rolls, flips or digits yet.
    TooFew {
        /// How many there are.
        have: usize,
        /// How many are needed.
        need: usize,
    },
    /// The count of hex digits or of bytes is not one a BIP-39 length
    /// takes.
    BadLength,
}

/// 16, 20, 24, 28 or 32 bytes of entropy, erased on drop. Feed it to
/// `osk_bip::bip39::Mnemonic::from_entropy`.
pub struct RawEntropy {
    bytes: [u8; 32],
    len: u8,
}

impl RawEntropy {
    fn new(src: &[u8]) -> Self {
        let mut e = RawEntropy {
            bytes: [0u8; 32],
            len: src.len() as u8,
        };
        e.bytes[..src.len()].copy_from_slice(src);
        e
    }

    /// The bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }

    /// The strength, which the length says.
    pub fn strength(&self) -> Strength {
        Strength::for_bytes(usize::from(self.len)).unwrap_or(Strength::Bits256)
    }
}

impl AsRef<[u8]> for RawEntropy {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Zeroize for RawEntropy {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.len.zeroize();
    }
}

impl Drop for RawEntropy {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for RawEntropy {}

/// What the sanity checks found. Every flag is a caution to show the
/// user; only `too_few` should stop them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Warnings {
    /// Fewer than the strength needs.
    pub too_few: bool,
    /// [`DICE_LONG_RUN`] (or [`COIN_LONG_RUN`]) identical values in a row.
    pub long_run: bool,
    /// The distribution is far from uniform (see the constants).
    pub skewed: bool,
    /// [`DICE_SEQUENTIAL`] rolls that count up or down (dice only).
    pub sequential: bool,
}

impl Warnings {
    /// Whether any flag is set.
    pub fn any(self) -> bool {
        self.too_few || self.long_run || self.skewed || self.sequential
    }

    /// Whether any caution (a flag other than `too_few`) is set.
    pub fn any_caution(self) -> bool {
        self.long_run || self.skewed || self.sequential
    }
}

// ----- dice -----

/// How rolls of a six-sided die become a key
/// (`docs/PLANNING.md` §16.115). Each is another signer's published
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
    /// positions is a reroll and is not kept. The rolls name every word
    /// but the last, whose free bits and checksum are the choice among
    /// the words the checksum leaves. EntropyLab's "BitBox diceware /
    /// Direct word selection".
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
    /// for every word but the last.
    pub fn needed(self, strength: Strength) -> usize {
        match self {
            DiceProcedure::Hashed | DiceProcedure::SixAsZero => strength.rolls(),
            DiceProcedure::Words => DICE_WORD_ROLLS * (strength.words() - 1),
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
        }
    }

    // ----- the other published procedures (`docs/PLANNING.md` §16.115)

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
    /// [`DiceProcedure::Words`] names words rather than bytes and has no
    /// entropy of its own; ask it for [`word_indices`](Self::word_indices).
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
            DiceProcedure::Words => Err(Error::BadLength),
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
        }
    }
}

// ----- coins -----

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

// ----- hex -----

/// Accumulates hex digits typed by the user.
pub struct RawHex {
    digits: [u8; MAX_HEX],
    len: u8,
}

impl Default for RawHex {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for RawHex {
    fn zeroize(&mut self) {
        self.digits.zeroize();
        self.len.zeroize();
    }
}

impl Drop for RawHex {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for RawHex {}

impl RawHex {
    /// No digits yet.
    pub fn new() -> Self {
        RawHex {
            digits: [0u8; MAX_HEX],
            len: 0,
        }
    }

    /// Parses `text`, which must be exactly 32, 40, 48, 56 or 64 hex
    /// digits.
    pub fn parse(text: &str) -> Result<RawEntropy, Error> {
        let mut h = RawHex::new();
        if text.len() > MAX_HEX {
            return Err(Error::BadLength);
        }
        for c in text.chars() {
            if !h.push(c) {
                return Err(Error::BadLength);
            }
        }
        h.entropy()
    }

    /// Adds a hex digit (either case). Returns `false` for anything else
    /// or when full.
    pub fn push(&mut self, c: char) -> bool {
        if !c.is_ascii_hexdigit() || usize::from(self.len) >= MAX_HEX {
            return false;
        }
        self.digits[usize::from(self.len)] = c.to_ascii_lowercase() as u8;
        self.len += 1;
        true
    }

    /// Removes the last digit.
    pub fn pop(&mut self) -> Option<char> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let c = self.digits[usize::from(self.len)] as char;
        self.digits[usize::from(self.len)] = 0;
        Some(c)
    }

    /// Forgets every digit.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Number of digits.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether there are no digits.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The digits so far, lower-case.
    pub fn digits(&self) -> &str {
        core::str::from_utf8(&self.digits[..self.len()]).expect("ascii")
    }

    /// The last digit, if any.
    pub fn last(&self) -> Option<char> {
        self.digits().chars().last()
    }

    /// The strength the digit count corresponds to, two digits a byte:
    /// 32, 40, 48, 56 or 64 digits.
    pub fn strength(&self) -> Option<Strength> {
        self.len()
            .is_multiple_of(2)
            .then(|| Strength::for_bytes(self.len() / 2))
            .flatten()
    }

    /// SHA-256 of the digits as typed, lower-case.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(self.digits().as_bytes())
    }

    /// The bytes the digits spell.
    pub fn entropy(&self) -> Result<RawEntropy, Error> {
        let strength = self.strength().ok_or(Error::BadLength)?;
        let mut bytes = [0u8; 32];
        for (i, pair) in self.digits().as_bytes().chunks_exact(2).enumerate() {
            bytes[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
        }
        let e = RawEntropy::new(&bytes[..strength.bytes()]);
        bytes.zeroize();
        Ok(e)
    }
}

/// Most parts a Seed XOR split or combine takes
/// (`docs/PLANNING.md` §8.2 item 14).
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

// ----- playing cards -----

/// Cards in one deck.
pub const DECK: usize = 52;

/// Most cards an accumulator holds: two decks, which is what 256 bits
/// needs ([`CardDraws::needed`]), and more than any shorter length does.
pub const MAX_CARDS: usize = 2 * DECK;

/// The ranks, in the order [`CardDraws`] numbers them. `T` is the ten,
/// so that every rank is one character and the strip of cards entered
/// keeps one width.
pub const RANKS: [char; 13] = [
    'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
];

/// The suits, in the order [`CardDraws`] numbers them: ♠ ♥ ♦ ♣.
pub const SUITS: [char; 4] = ['\u{2660}', '\u{2665}', '\u{2666}', '\u{2663}'];

/// log2 of 0..=52, for the bits a draw from a deck of that many carries.
/// `LOG2[0]` is a placeholder: no draw is made from an empty deck.
const LOG2: [f32; 53] = [
    0.0,
    0.0,
    1.0,
    1.5849625,
    2.0,
    2.321928,
    2.5849626,
    2.807355,
    3.0,
    3.169925,
    core::f32::consts::LOG2_10,
    3.4594316,
    3.5849626,
    3.7004397,
    3.807355,
    3.9068906,
    4.0,
    4.087463,
    4.169925,
    4.2479277,
    4.321928,
    4.3923173,
    4.4594316,
    4.523562,
    4.5849624,
    4.643856,
    4.70044,
    4.7548876,
    4.807355,
    4.857981,
    4.9068904,
    4.9541965,
    5.0,
    5.044394,
    5.087463,
    5.129283,
    5.169925,
    5.2094536,
    5.2479277,
    5.2854023,
    5.321928,
    5.357552,
    5.3923173,
    5.426265,
    5.4594316,
    5.491853,
    5.523562,
    5.554589,
    5.5849624,
    5.61471,
    5.643856,
    5.6724253,
    5.70044,
];

/// The card index a rank and a suit make, 0..52.
pub fn card_index(rank: u8, suit: u8) -> Option<u8> {
    (usize::from(rank) < RANKS.len() && usize::from(suit) < SUITS.len())
        .then(|| suit * RANKS.len() as u8 + rank)
}

/// The rank and the suit of a card index.
pub fn card_parts(index: u8) -> (u8, u8) {
    (index % RANKS.len() as u8, index / RANKS.len() as u8)
}

/// Bits the first `draws` cards of a shuffled deck carry: the first is
/// one of 52, the second one of the 51 left, and a fresh deck follows
/// the fifty-second. The fifty-second card of a deck carries none — it
/// is the one the other fifty-one left behind.
pub fn card_bits(draws: usize) -> f32 {
    let mut bits = 0.0;
    for i in 0..draws {
        bits += LOG2[DECK - i % DECK];
    }
    bits
}

/// Per-suit counts of a run of draws, and where the run stands in the
/// deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardStats {
    /// How often each suit came up, `suits[0]` for ♠.
    pub suits: [u32; 4],
    /// Which deck the next card is drawn from, counting from 1.
    pub deck: u8,
    /// Cards already drawn from that deck.
    pub in_deck: u32,
}

/// Accumulates cards drawn from a shuffled deck, refusing one that is
/// already out of it.
///
/// The entropy is SHA-256 of the draws' indices, one byte each, as dice
/// are the SHA-256 of their digits. The index is `suit * 13 + rank` with
/// the suits in [`SUITS`] order and the ranks in [`RANKS`] order, so a
/// person can reproduce it from a written-down list of cards.
pub struct CardDraws {
    cards: [u8; MAX_CARDS],
    len: u8,
}

impl Default for CardDraws {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for CardDraws {
    fn zeroize(&mut self) {
        self.cards.zeroize();
        self.len.zeroize();
    }
}

impl Drop for CardDraws {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CardDraws {}

impl CardDraws {
    /// No draws yet.
    pub fn new() -> Self {
        CardDraws {
            cards: [0u8; MAX_CARDS],
            len: 0,
        }
    }

    /// The cards drawn so far, as indices.
    pub fn cards(&self) -> &[u8] {
        &self.cards[..self.len()]
    }

    /// Number of draws.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether nothing has been drawn.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Draws from the current deck, counting from 1.
    pub fn in_deck(&self) -> usize {
        self.len() % DECK
    }

    /// Which deck the next card comes from, counting from 1: a deck is
    /// spent after fifty-two draws and the next card is the first of a
    /// freshly shuffled one.
    pub fn deck(&self) -> usize {
        self.len() / DECK + 1
    }

    /// Whether `index` is already out of the deck being drawn from.
    pub fn drawn(&self, index: u8) -> bool {
        let start = self.len() - self.in_deck();
        self.cards[start..self.len()].contains(&index)
    }

    /// Adds a draw. Returns `false` for an index past the deck, for a
    /// card already drawn from this deck, or when full.
    pub fn push(&mut self, index: u8) -> bool {
        if usize::from(index) >= DECK || self.len() >= MAX_CARDS || self.drawn(index) {
            return false;
        }
        self.cards[self.len()] = index;
        self.len += 1;
        true
    }

    /// Removes the last draw, putting it back in its deck.
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let c = self.cards[self.len()];
        self.cards[self.len()] = 0;
        Some(c)
    }

    /// Forgets every draw.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// The last card, if any.
    pub fn last(&self) -> Option<u8> {
        self.cards().last().copied()
    }

    /// Bits gathered so far ([`card_bits`]).
    pub fn bits_collected(&self) -> f32 {
        card_bits(self.len())
    }

    /// Draws needed for `strength`: the fewest whose [`card_bits`] reach
    /// it — 25, 31, 39, 50 and 58. The count climbs faster than the bits
    /// do because a draw is worth less as the deck empties. 58 is more
    /// than a deck holds, so 256 bits asks for a second shuffled deck.
    pub fn needed(strength: Strength) -> usize {
        let mut n = 0;
        while card_bits(n) < strength.bits() as f32 {
            n += 1;
        }
        n
    }

    /// Whether enough cards are in for `strength`.
    pub fn is_enough(&self, strength: Strength) -> bool {
        self.len() >= Self::needed(strength)
    }

    /// SHA-256 of the draws' indices, one byte each.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(self.cards())
    }

    /// The commitment truncated to the strength.
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

    /// Per-suit counts and where the run stands in the deck.
    pub fn stats(&self) -> CardStats {
        let mut suits = [0u32; 4];
        for &c in self.cards() {
            suits[usize::from(card_parts(c).1)] += 1;
        }
        CardStats {
            suits,
            deck: self.deck() as u8,
            in_deck: self.in_deck() as u32,
        }
    }

    /// The sanity flags for `strength`. A shuffled deck is not a die:
    /// its suits come out even by construction, so there is nothing to
    /// test but the count.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        Warnings {
            too_few: !self.is_enough(strength),
            ..Warnings::default()
        }
    }
}

// ----- camera noise -----

/// Most frames an accumulator holds.
pub const MAX_FRAMES: usize = 8;

/// Frames asked for at 128 bits.
///
/// A camera sensor's read noise is worth a fraction of a bit per pixel
/// and a frame carries hundreds of thousands of pixels, so one frame
/// already holds far more than 256 bits — if the sensor is what it
/// claims to be. The count is not an entropy calculation, then, but a
/// guard against a single frame that happens to be still: three
/// pictures of a moving scene differ in the sensor's noise and in what
/// the person pointed the camera at.
pub const CAMERA_FRAMES_128: usize = 3;

/// Frames asked for at 256 bits, for the same reason twice over. Every
/// length in between takes [`CAMERA_FRAMES_128`] frames per 128 bits,
/// rounded up, which is what [`CameraNoise::needed`] asks for.
pub const CAMERA_FRAMES_256: usize = 6;

/// Distinct luma values a frame must have to be taken. A covered lens
/// or a saturated one gives a frame of a handful of values, and such a
/// frame carries no noise worth hashing.
pub const CAMERA_MIN_DISTINCT: u32 = 32;

/// What one camera frame looked like, stated on the result rather than
/// judged (except [`FrameStats::distinct`], which is the one refusal).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStats {
    /// Pixels in the frame.
    pub pixels: u32,
    /// How many of the 256 luma values appear at all.
    pub distinct: u32,
    /// Mean luma.
    pub mean: f32,
    /// Variance of the luma.
    pub variance: f32,
}

impl FrameStats {
    /// Reads a frame's luma bytes.
    pub fn of(luma: &[u8]) -> Self {
        let mut counts = [0u32; 256];
        for &b in luma {
            counts[usize::from(b)] += 1;
        }
        let n = luma.len() as u32;
        let distinct = counts.iter().filter(|&&c| c > 0).count() as u32;
        let (mut sum, mut sum_sq) = (0f64, 0f64);
        for (v, &c) in counts.iter().enumerate() {
            let (v, c) = (v as f64, f64::from(c));
            sum += v * c;
            sum_sq += v * v * c;
        }
        let (mean, variance) = if n == 0 {
            (0.0, 0.0)
        } else {
            let m = sum / f64::from(n);
            (m, sum_sq / f64::from(n) - m * m)
        };
        FrameStats {
            pixels: n,
            distinct,
            mean: mean as f32,
            variance: variance as f32,
        }
    }

    /// Whether the frame carries enough variation to be taken.
    pub fn usable(self) -> bool {
        self.distinct >= CAMERA_MIN_DISTINCT
    }
}

/// Accumulates camera frames.
///
/// Each frame's luma bytes are hashed as they arrive — a frame is
/// hundreds of kilobytes and nothing here allocates — and the entropy
/// is SHA-256 over those per-frame digests in order, truncated to the
/// strength. The digests are what a person would have to keep to
/// reproduce the result, and they are what the mix commits to.
///
/// This source trusts the camera and the shell that delivers its
/// frames, which dice do not.
pub struct CameraNoise {
    digests: [[u8; 32]; MAX_FRAMES],
    len: u8,
    last: Option<FrameStats>,
}

impl Default for CameraNoise {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for CameraNoise {
    fn zeroize(&mut self) {
        for d in &mut self.digests {
            d.zeroize();
        }
        self.len.zeroize();
        self.last = None;
    }
}

impl Drop for CameraNoise {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CameraNoise {}

impl CameraNoise {
    /// No frames yet.
    pub fn new() -> Self {
        CameraNoise {
            digests: [[0u8; 32]; MAX_FRAMES],
            len: 0,
            last: None,
        }
    }

    /// Takes one frame's luma bytes. Returns the frame's statistics,
    /// and whether it was taken: a frame with fewer than
    /// [`CAMERA_MIN_DISTINCT`] luma values is refused, and so is one
    /// past [`MAX_FRAMES`].
    pub fn push(&mut self, luma: &[u8]) -> (FrameStats, bool) {
        let stats = FrameStats::of(luma);
        self.last = Some(stats);
        if !stats.usable() || self.len() >= MAX_FRAMES {
            return (stats, false);
        }
        self.digests[self.len()] = sha256(luma);
        self.len += 1;
        (stats, true)
    }

    /// Frames taken.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether no frame has been taken.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The last frame's statistics, taken or refused.
    pub fn last(&self) -> Option<FrameStats> {
        self.last
    }

    /// Forgets every frame.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Frames needed for `strength`: [`CAMERA_FRAMES_128`] per 128
    /// bits, rounded up, which is 3 at 128 bits and
    /// [`CAMERA_FRAMES_256`] at 256 — 4, 5 and 6 in between.
    pub fn needed(strength: Strength) -> usize {
        (CAMERA_FRAMES_128 * strength.bits()).div_ceil(128)
    }

    /// Whether enough frames are in for `strength`.
    pub fn is_enough(&self, strength: Strength) -> bool {
        self.len() >= Self::needed(strength)
    }

    /// SHA-256 over the frames' digests, in order.
    pub fn commitment(&self) -> [u8; 32] {
        let mut buf = [0u8; MAX_FRAMES * 32];
        for (i, d) in self.digests[..self.len()].iter().enumerate() {
            buf[i * 32..(i + 1) * 32].copy_from_slice(d);
        }
        let h = sha256(&buf[..self.len() * 32]);
        buf.zeroize();
        h
    }

    /// The commitment truncated to the strength.
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

    /// The sanity flags for `strength`: a frame is a picture, and the
    /// only thing to say about a run of them is whether there are
    /// enough.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        Warnings {
            too_few: !self.is_enough(strength),
            ..Warnings::default()
        }
    }
}

// ----- the device's own generator -----

/// The 32 bytes the shell answered the core's entropy request with
/// (`osk_shell_api::Command::RequestEntropy`), taken as a key's
/// entropy.
///
/// The entropy is SHA-256 of those bytes, truncated to the strength, so
/// that the key is not the shell's bytes themselves and one definition
/// covers both word counts. This source trusts the device: nobody
/// watching can tell a faulty or a rigged generator from a good one.
pub struct DeviceRandom {
    bytes: [u8; 32],
    have: bool,
}

impl Default for DeviceRandom {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for DeviceRandom {
    fn zeroize(&mut self) {
        self.bytes.zeroize();
        self.have = false;
    }
}

impl Drop for DeviceRandom {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for DeviceRandom {}

impl DeviceRandom {
    /// Nothing answered yet.
    pub fn new() -> Self {
        DeviceRandom {
            bytes: [0u8; 32],
            have: false,
        }
    }

    /// The shell answered.
    pub fn set(&mut self, bytes: &[u8; 32]) {
        self.bytes = *bytes;
        self.have = true;
    }

    /// Whether an answer is in.
    pub fn is_enough(&self, _strength: Strength) -> bool {
        self.have
    }

    /// Forgets the answer.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// SHA-256 of the bytes.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(&self.bytes)
    }

    /// The commitment truncated to the strength.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if !self.have {
            return Err(Error::TooFew { have: 0, need: 1 });
        }
        let mut hash = self.commitment();
        let e = RawEntropy::new(&hash[..strength.bytes()]);
        hash.zeroize();
        Ok(e)
    }

    /// The sanity flags: whether an answer is in, and nothing else. No
    /// test of a generator's output tells anyone whether to trust it.
    pub fn warnings(&self, strength: Strength) -> Warnings {
        Warnings {
            too_few: !self.is_enough(strength),
            ..Warnings::default()
        }
    }
}

// ----- mixing -----

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

fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dice(s: &str) -> DiceRolls {
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
    const RANDOM_50: &str = "32461151351521144121541512665155412152342515356215";
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
        // Six rolls a word, for every word but the last.
        assert_eq!(DiceProcedure::Words.needed(Strength::Bits128), 66);
        assert_eq!(DiceProcedure::Words.needed(Strength::Bits256), 138);
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

    /// Twenty-five cards with no repeat: the whole spade suit and the
    /// first twelve hearts.
    const TWENTY_FIVE: &str = "A\u{2660}2\u{2660}3\u{2660}4\u{2660}5\u{2660}6\u{2660}7\u{2660}8\u{2660}9\u{2660}T\u{2660}J\u{2660}Q\u{2660}K\u{2660}A\u{2665}2\u{2665}3\u{2665}4\u{2665}5\u{2665}6\u{2665}7\u{2665}8\u{2665}9\u{2665}T\u{2665}J\u{2665}Q\u{2665}";

    /// A second implementation of the card definition, written from the
    /// documentation rather than from the code above: the index of a
    /// card, the SHA-256 of the indices, and the bits a run of draws
    /// carries.
    fn cards_by_hand(spec: &str) -> ([u8; 32], f64) {
        let mut bytes = [0u8; MAX_CARDS];
        let mut n = 0;
        let mut chars = spec.chars();
        while let (Some(r), Some(u)) = (chars.next(), chars.next()) {
            let rank = RANKS.iter().position(|&c| c == r).unwrap();
            let suit = SUITS.iter().position(|&c| c == u).unwrap();
            bytes[n] = (suit * 13 + rank) as u8;
            n += 1;
        }
        let mut bits = 0.0f64;
        for i in 0..n {
            bits += log2(f64::from((52 - i % 52) as u32));
        }
        (sha256(&bytes[..n]), bits)
    }

    /// log2 by repeated squaring, so that the expected bit counts are
    /// worked out here rather than read from the table the crate ships.
    fn log2(v: f64) -> f64 {
        let (mut x, mut exp) = (v, 0f64);
        while x >= 2.0 {
            x /= 2.0;
            exp += 1.0;
        }
        let (mut frac, mut w, mut y) = (0f64, 0.5f64, x);
        for _ in 0..52 {
            y *= y;
            if y >= 2.0 {
                y /= 2.0;
                frac += w;
            }
            w /= 2.0;
        }
        exp + frac
    }

    fn deal(spec: &str) -> CardDraws {
        let mut d = CardDraws::new();
        let mut chars = spec.chars();
        while let (Some(r), Some(u)) = (chars.next(), chars.next()) {
            let rank = RANKS.iter().position(|&c| c == r).unwrap() as u8;
            let suit = SUITS.iter().position(|&c| c == u).unwrap() as u8;
            assert!(d.push(card_index(rank, suit).unwrap()), "{r}{u}");
        }
        d
    }

    #[test]
    fn cards_hash_their_indices_and_count_the_deck_down() {
        let d = deal(TWENTY_FIVE);
        assert_eq!(d.len(), 25);
        assert!(d.is_enough(Strength::Bits128));
        assert!(!d.is_enough(Strength::Bits256));
        assert_eq!(d.stats().suits, [13, 12, 0, 0]);
        let (hash, bits) = cards_by_hand(TWENTY_FIVE);
        assert_eq!(d.commitment(), hash);
        assert!(
            (f64::from(d.bits_collected()) - bits).abs() < 0.01,
            "{bits}"
        );
        assert!(bits > 128.0);
        let e = d.entropy(Strength::Bits128).unwrap();
        assert_eq!(e.as_bytes(), &hash[..16]);
        assert_eq!(
            d.entropy(Strength::Bits256).err(),
            Some(Error::TooFew { have: 25, need: 58 })
        );
        // The counts are where the definition crosses the two targets.
        assert!(card_bits(24) < 128.0 && card_bits(25) >= 128.0);
        assert!(card_bits(57) < 256.0 && card_bits(58) >= 256.0);
    }

    #[test]
    fn the_deck_tracker_refuses_a_card_already_out() {
        let mut d = CardDraws::new();
        let ace_of_spades = card_index(0, 0).unwrap();
        assert!(d.push(ace_of_spades));
        assert!(d.drawn(ace_of_spades));
        assert!(!d.push(ace_of_spades), "already drawn");
        assert_eq!(d.len(), 1);
        assert!(!d.push(52), "no such card");
        // Putting it back frees it again.
        assert_eq!(d.pop(), Some(ace_of_spades));
        assert!(!d.drawn(ace_of_spades));
        assert!(d.push(ace_of_spades));
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn twenty_four_words_from_cards_asks_for_a_second_deck() {
        assert!(CardDraws::needed(Strength::Bits256) > DECK);
        let mut d = CardDraws::new();
        for i in 0..DECK as u8 {
            assert!(d.push(i));
        }
        assert_eq!(d.deck(), 2, "the fifty-third card is the second deck's");
        assert_eq!(d.in_deck(), 0);
        // The whole first deck is back in play.
        assert!(!d.drawn(0));
        assert!(d.push(0));
        assert_eq!(d.deck(), 2);
        assert!(!d.push(0), "and out of the second deck now");
        for i in 1..6u8 {
            assert!(d.push(i));
        }
        assert_eq!(d.len(), 58);
        assert!(d.is_enough(Strength::Bits256));
        assert_eq!(d.entropy(Strength::Bits256).unwrap().as_bytes().len(), 32);
        assert_eq!(d.stats().suits.iter().sum::<u32>(), 58);
    }

    #[test]
    fn a_covered_lens_is_refused_and_frames_hash_in_order() {
        let mut c = CameraNoise::new();
        let covered = [7u8; 4096];
        let (stats, taken) = c.push(&covered);
        assert!(!taken);
        assert_eq!(stats.distinct, 1);
        assert_eq!(stats.mean, 7.0);
        assert_eq!(stats.variance, 0.0);
        assert_eq!(stats.pixels, 4096);
        assert!(c.is_empty());

        // A gradient of 32 values is the least a frame may carry.
        let edge: [u8; 256] = core::array::from_fn(|i| (i % 32) as u8);
        let (stats, taken) = c.push(&edge);
        assert_eq!(stats.distinct, 32);
        assert!(taken);

        let mut c = CameraNoise::new();
        let frames: [[u8; 512]; 3] =
            core::array::from_fn(|f| core::array::from_fn(|i| (i * 7 + f * 3) as u8));
        for f in &frames {
            assert!(c.push(f).1);
        }
        assert_eq!(c.len(), 3);
        assert!(c.is_enough(Strength::Bits128));
        assert!(!c.is_enough(Strength::Bits256));
        // The definition, computed here: SHA-256 over the frames'
        // SHA-256 digests, in order.
        let mut buf = [0u8; 96];
        for (i, f) in frames.iter().enumerate() {
            buf[i * 32..(i + 1) * 32].copy_from_slice(&sha256(f));
        }
        assert_eq!(c.commitment(), sha256(&buf));
        assert_eq!(
            c.entropy(Strength::Bits128).unwrap().as_bytes(),
            &sha256(&buf)[..16]
        );
        assert_eq!(
            c.entropy(Strength::Bits256).err(),
            Some(Error::TooFew { have: 3, need: 6 })
        );
    }

    #[test]
    fn the_device_answers_differently_every_time_and_hashes_what_it_answered() {
        let mut a = DeviceRandom::new();
        assert!(!a.is_enough(Strength::Bits128));
        assert!(a.warnings(Strength::Bits128).too_few);
        assert_eq!(
            a.entropy(Strength::Bits128).err(),
            Some(Error::TooFew { have: 0, need: 1 })
        );
        let first = [1u8; 32];
        a.set(&first);
        assert_eq!(a.commitment(), sha256(&first));
        let key_a = a.entropy(Strength::Bits256).unwrap();
        assert_eq!(key_a.as_bytes(), &sha256(&first)[..]);

        // A second run gets a second answer, and so a second key.
        let mut b = DeviceRandom::new();
        let second = [2u8; 32];
        b.set(&second);
        let key_b = b.entropy(Strength::Bits256).unwrap();
        assert_ne!(key_a.as_bytes(), key_b.as_bytes());
        a.clear();
        assert!(!a.is_enough(Strength::Bits256));
    }

    #[test]
    fn a_mix_commits_to_its_sources_and_hashes_the_commitments() {
        let dice = dice(RANDOM_50);
        let mut coins = CoinFlips::new();
        for i in 0..128 {
            coins.push(i % 3 == 0);
        }
        // Each commitment is SHA-256 of that source's raw input.
        assert_eq!(dice.commitment(), sha256(RANDOM_50.as_bytes()));
        let mut ht = [0u8; 128];
        for (i, b) in ht.iter_mut().enumerate() {
            *b = if i % 3 == 0 { b'H' } else { b'T' };
        }
        assert_eq!(coins.commitment(), sha256(&ht));

        let mut m = Mixed::new();
        assert_eq!(
            m.entropy(Strength::Bits128).err(),
            Some(Error::TooFew { have: 0, need: 2 })
        );
        assert!(m.push(dice.commitment()));
        assert_eq!(
            m.entropy(Strength::Bits128).err(),
            Some(Error::TooFew { have: 1, need: 2 }),
            "one source is not a mix"
        );
        assert!(m.push(coins.commitment()));
        assert_eq!(m.commitments(), &[dice.commitment(), coins.commitment()]);
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&dice.commitment());
        buf[32..].copy_from_slice(&coins.commitment());
        assert_eq!(m.commitment(), sha256(&buf));
        assert_eq!(
            m.entropy(Strength::Bits128).unwrap().as_bytes(),
            &sha256(&buf)[..16]
        );
        // The order is part of the definition.
        let mut n = Mixed::new();
        assert!(n.push(coins.commitment()) && n.push(dice.commitment()));
        assert_ne!(n.commitment(), m.commitment());
        m.clear();
        assert!(m.is_empty());
    }

    /// A 15-, 18- or 21-word key asks each source for a count of its
    /// own, and the count is what the source then takes: one short is
    /// refused, the count itself makes the key's bytes.
    #[test]
    fn the_three_middle_lengths_ask_each_source_for_its_own_count() {
        for (strength, words, bytes, rolls, flips, digits, cards, frames) in [
            (Strength::Bits160, 15, 20, 62, 160, 40, 31, 4),
            (Strength::Bits192, 18, 24, 75, 192, 48, 39, 5),
            (Strength::Bits224, 21, 28, 87, 224, 56, 50, 6),
        ] {
            assert_eq!(strength.words(), words);
            assert_eq!(Strength::for_words(words), Some(strength));
            assert_eq!(strength.bytes(), bytes);
            assert_eq!(Strength::for_bytes(bytes), Some(strength));
            assert_eq!(DiceRolls::needed(strength), rolls);
            assert_eq!(CoinFlips::needed(strength), flips);
            assert_eq!(strength.hex_digits(), digits);
            assert_eq!(CardDraws::needed(strength), cards);
            assert_eq!(CameraNoise::needed(strength), frames);

            let mut d = DiceRolls::new();
            for i in 0..rolls - 1 {
                assert!(d.push((i % 6 + 1) as u8));
            }
            assert_eq!(
                d.entropy(strength).err(),
                Some(Error::TooFew {
                    have: rolls - 1,
                    need: rolls
                })
            );
            assert!(d.push(4));
            let e = d.entropy(strength).unwrap();
            assert_eq!(e.as_bytes(), &d.commitment()[..bytes]);
            assert_eq!(e.strength(), strength);

            let mut c = CoinFlips::new();
            for i in 0..flips {
                assert!(c.push(i % 3 == 0));
            }
            assert_eq!(c.entropy(strength).unwrap().as_bytes().len(), bytes);

            let mut h = RawHex::new();
            for _ in 0..digits - 1 {
                assert!(h.push('a'));
            }
            assert_eq!(h.entropy().err(), Some(Error::BadLength));
            assert!(h.push('a'));
            assert_eq!(h.strength(), Some(strength));
            let e = h.entropy().unwrap();
            assert_eq!(e.as_bytes().len(), bytes);
            assert!(e.as_bytes().iter().all(|&b| b == 0xaa));
        }
    }

    /// A hand of cards reaches 160 bits at the thirty-first draw and not
    /// before, by the bits this file works out for itself.
    #[test]
    fn cards_reach_160_bits_at_the_thirty_first_draw() {
        let mut d = CardDraws::new();
        for i in 0..30u8 {
            assert!(d.push(i));
        }
        assert!(!d.is_enough(Strength::Bits160), "thirty draws is short");
        assert!(d.push(30));
        assert!(d.is_enough(Strength::Bits160));

        // The bits a run of draws from a fresh deck carries, summed here:
        // the first card is one of 52, the thirty-first one of 22.
        let mut thirty = 0f64;
        for i in 0..30 {
            thirty += log2(f64::from(52 - i as u32));
        }
        let all = thirty + log2(22.0);
        assert!(thirty < 160.0 && all >= 160.0, "{thirty} then {all}");

        let mut indices = [0u8; 31];
        for (i, b) in indices.iter_mut().enumerate() {
            *b = i as u8;
        }
        let hash = sha256(&indices);
        assert_eq!(d.commitment(), hash);
        assert_eq!(
            d.entropy(Strength::Bits160).unwrap().as_bytes(),
            &hash[..20]
        );
    }

    #[test]
    fn hex_parsing() {
        let e = RawHex::parse("00000000000000000000000000000000").unwrap();
        assert_eq!(e.as_bytes(), &[0u8; 16]);
        let e = RawHex::parse("FFfF00000000000000000000000000000000000000000000000000000000ab1C")
            .unwrap();
        assert_eq!(e.as_bytes().len(), 32);
        assert_eq!(e.as_bytes()[0], 0xff);
        assert_eq!(e.as_bytes()[31], 0x1c);
        assert_eq!(RawHex::parse("0000").err(), Some(Error::BadLength));
        assert_eq!(
            RawHex::parse("0000000000000000000000000000000g").err(),
            Some(Error::BadLength)
        );
        assert_eq!(RawHex::parse("").err(), Some(Error::BadLength));
        let mut h = RawHex::new();
        assert!(!h.push('g'));
        assert!(h.push('A'));
        assert_eq!(h.digits(), "a");
        assert_eq!(h.strength(), None);
        assert_eq!(h.entropy().err(), Some(Error::BadLength));
        assert_eq!(h.pop(), Some('a'));
        assert!(h.is_empty());
        for _ in 0..64 {
            assert!(h.push('0'));
        }
        assert!(!h.push('0'));
        assert_eq!(h.strength(), Some(Strength::Bits256));
    }
}
