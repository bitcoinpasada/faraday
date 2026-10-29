//! User-supplied entropy: dice rolls, coin flips and raw hex, accumulated in fixed
//! buffers that are erased on drop, turned into the 128, 160, 192, 224
//! or 256 bits of [`RawEntropy`] a BIP-39 key of 12, 15, 18, 21 or 24
//! words takes, and checked for the mistakes real hands make.
//!
//! Conventions:
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
//!   writes every 6 as a 0 first (Keystone). The third names every word
//!   outright from five rolls of 1-4 and a sixth read as a coin
//!   (BitBox), 72 rolls at 128 bits and 144 at 256, and the checksum
//!   replaces the last word's low bits (§16.139).
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
//! - **Pointer and keys** ([`PointerKeys`]): not a key's source but one
//!   a person adds to the session's randomness; a batch's commitment is
//!   SHA-256 over its events' records, each with the time and a running
//!   count.
//!
//! Every source states a **commitment**: SHA-256 of its raw input, which
//! is what a mix combines and what a person can write down before the
//! words appear and check afterwards. For dice, cards, the camera, the
//! device and a mix the source's own entropy is that commitment
//! truncated; coins and hex are defined otherwise (packed bits, the
//! digits themselves) and their commitment is a separate statement of
//! what went in.
//!
//! [`Source`] names each of these, and [`SOURCE_ROWS`], [`MIX_SOURCES`]
//! and [`WORD_COUNTS`] are the lists a key-making flow offers, in the
//! order it offers them.
//!
//! The sanity checks ([`DiceRolls::warnings`], [`CoinFlips::warnings`])
//! are cautions for the user, never hard blocks, except `too_few`. The
//! crate is `no_std` and does not allocate.

#![no_std]

use faraday_crypto::{Zeroize, ZeroizeOnDrop};

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
/// Cards in a row, each the one after (or each the one before) the
/// last in the deck's own order, that draw a caution: a deck out of its
/// box, or one barely shuffled, comes out so. A well-shuffled deck shows
/// such a run about once in seven million draws.
pub const CARDS_IN_ORDER: u32 = 5;

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
/// `faraday_bip::bip39::Mnemonic::from_entropy`.
pub struct RawEntropy {
    bytes: [u8; 32],
    len: u8,
}

impl RawEntropy {
    pub(crate) fn new(src: &[u8]) -> Self {
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
    /// [`CARDS_IN_ORDER`] cards in a row in the deck's own order, up or
    /// down (cards only).
    pub in_order: bool,
}

impl Warnings {
    /// Whether any flag is set.
    pub fn any(self) -> bool {
        self.too_few || self.long_run || self.skewed || self.sequential || self.in_order
    }
}

pub mod booklet;
mod camera;
mod cards;
mod coins;
mod device;
mod dice;
mod hex;
#[cfg(test)]
mod known;
mod mix;
mod pointer_keys;
mod xor;

pub use booklet::{
    BOOKLET_DICE, BOOKLET_FACES, BOOKLET_MAX_FACES, BOOKLET_ROLLS, BookletChar, BookletError,
    TREE as BOOKLET_TREE, character as booklet_character, uniform_face,
};
pub use camera::{
    CAMERA_FRAMES_128, CAMERA_FRAMES_256, CAMERA_MIN_DISTINCT, CameraNoise, FrameStats, MAX_FRAMES,
};
pub use cards::{
    CardDraws, CardStats, DECK, MAX_CARDS, RANKS, SUITS, card_bits, card_index, card_parts,
};
pub use coins::{CoinFlips, CoinStats};
pub use device::DeviceRandom;
pub use dice::{DiceProcedure, DiceRolls, DiceStats};
pub use hex::RawHex;
pub use mix::{MAX_MIX, MIN_MIX, Mixed};
pub use pointer_keys::{POINTER_KEYS_BATCH, POINTER_KEYS_RECORD, PointerKeys};
pub use xor::{MAX_XOR_PARTS, MIN_XOR_PARTS, SeedXor};

// ----- what a flow offers -----

/// Where a key's entropy comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Six-sided dice ([`DiceRolls`], under a [`DiceProcedure`]).
    Dice,
    /// Coin flips, packed ([`CoinFlips`]).
    Coins,
    /// Hex digits typed as they are ([`RawHex`]).
    Hex,
    /// Seed XOR parts, typed as words and XORed back into the key
    /// ([`SeedXor`]).
    SeedXor,
    /// Cards drawn from a shuffled deck ([`CardDraws`]).
    Cards,
    /// The noise in a run of camera frames ([`CameraNoise`]): a mix's
    /// member only, never a key's only source (F158).
    Camera,
    /// Several sources combined ([`Mixed`]).
    Mix,
    /// The shell's own generator ([`DeviceRandom`]).
    Device,
}

/// The sources a new key can be made from, in the order they are
/// listed. Seed XOR is not among them: combining parts gives back a key
/// that already existed, so it is a way of loading one. Nor is the
/// camera: nobody can check what it delivers, so its frames only add to
/// a mix or to the system's randomness (F158).
pub const SOURCE_ROWS: [Source; 6] = [
    Source::Dice,
    Source::Coins,
    Source::Hex,
    Source::Cards,
    Source::Mix,
    Source::Device,
];

/// The sources a mix offers, in the order they are listed and in the
/// order their steps then run. A mix makes a key only with dice or the
/// device among the sources it takes (F158). Hex is not among them: a mix is a way of
/// not having to trust one source, and typed hex is the person's own
/// bytes, which they can mix themselves before typing. Seed XOR is not
/// a source of randomness at all.
pub const MIX_SOURCES: [Source; 5] = [
    Source::Dice,
    Source::Coins,
    Source::Cards,
    Source::Camera,
    Source::Device,
];

/// The BIP-39 word counts on offer, in the order they are listed: the
/// two most keys use, then the rest in ascending order.
pub const WORD_COUNTS: [u8; 5] = [12, 24, 15, 18, 21];

#[cfg(test)]
mod tests {
    use super::*;

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
}
