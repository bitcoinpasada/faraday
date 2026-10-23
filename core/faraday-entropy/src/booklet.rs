//! The codex32 booklet's dice de-biasing worksheet: ten rolls of five dice make one character.
//!
//! The booklet ("Shamir Secret Sharing Codex", SSS32.ps of 2023-03-07,
//! "Dice De-biasing Worksheet") has five dice, each with its own track.
//! All five are rolled and each die's value is noted (its first value),
//! then all five are rolled again (the second value). A die whose two
//! values are equal is rolled twice again, both rolls. Each die gives
//! one bit, 0 when its second value is lower than its first and 1 when
//! it is higher; die 1's bit is the most significant, and the five bits
//! `16·b1 + 8·b2 + 4·b3 + 2·b4 + b5` walk the booklet's tree to a leaf,
//! the bech32 character of that value. [`TREE`] is the leaves left to
//! right, the `code` array of SSS32.ps. Only six-faced dice are read.
//!
//! `tools/reference/booklet-dice/README.md` quotes the worksheet.
//!
//! Rolls are secret while a character is being made: [`BookletChar`]
//! erases them on drop.

use faraday_crypto::{Zeroize, ZeroizeOnDrop};

/// Dice on the worksheet, die 1 to die 5.
pub const BOOKLET_DICE: usize = 5;
/// Rolls entered for one character: five dice, twice.
pub const BOOKLET_ROLLS: usize = 2 * BOOKLET_DICE;

/// The booklet's tree, its leaves left to right: the bech32 alphabet in
/// value order (the `code` array of SSS32.ps). The leaf reached by the
/// five bits `b1..b5` is `TREE[16·b1 + 8·b2 + 4·b3 + 2·b4 + b5]`.
pub const TREE: [u8; 32] = *b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Why a roll or a character was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookletError {
    /// A face that is not 1 to 6.
    BadFace,
    /// All ten places are filled; nothing more to enter.
    Full,
    /// This die's two values are equal; both rolls are done again
    /// ([`BookletChar::redo`]). The die is numbered from 0.
    Equal(usize),
    /// A place is still empty.
    Incomplete,
}

/// One die's bit: 0 when the second value is lower than the first, 1
/// when higher, nothing when they are equal.
pub fn bit(first: u8, second: u8) -> Option<u8> {
    match second.cmp(&first) {
        core::cmp::Ordering::Less => Some(0),
        core::cmp::Ordering::Greater => Some(1),
        core::cmp::Ordering::Equal => None,
    }
}

/// The character's bech32 value 0 to 31 from the five bits, die 1's
/// the most significant.
pub fn value_of_bits(bits: [u8; BOOKLET_DICE]) -> u8 {
    bits.iter().fold(0, |v, b| (v << 1) | (b & 1))
}

/// The character the worksheet gives the rolls: each die's first and
/// second value (die 1 first), as a lowercase bech32 character. Refuses
/// a face outside 1 to 6 and the first die whose values are equal.
pub fn character(
    first: [u8; BOOKLET_DICE],
    second: [u8; BOOKLET_DICE],
) -> Result<u8, BookletError> {
    let mut bits = [0u8; BOOKLET_DICE];
    for d in 0..BOOKLET_DICE {
        if !(1..=6).contains(&first[d]) || !(1..=6).contains(&second[d]) {
            return Err(BookletError::BadFace);
        }
        bits[d] = bit(first[d], second[d]).ok_or(BookletError::Equal(d))?;
    }
    Ok(TREE[value_of_bits(bits) as usize])
}

/// The rolls of one character in progress, erased on drop. Entered in
/// the booklet's order: die 1 to die 5 for the first roll, then die 1 to
/// die 5 for the second.
pub struct BookletChar {
    /// Entered values 1 to 6, 0 for a place not yet filled: first roll
    /// in places 0 to 4, second in 5 to 9.
    rolls: [u8; BOOKLET_ROLLS],
}

impl Zeroize for BookletChar {
    fn zeroize(&mut self) {
        self.rolls.zeroize();
    }
}

impl Drop for BookletChar {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for BookletChar {}

impl Default for BookletChar {
    fn default() -> Self {
        Self::new()
    }
}

impl BookletChar {
    /// No rolls yet.
    pub fn new() -> Self {
        BookletChar {
            rolls: [0; BOOKLET_ROLLS],
        }
    }

    /// Enters the next roll, in the first empty place of the booklet's
    /// order. Refuses a face outside 1 to 6 and a character already full.
    pub fn push(&mut self, face: u8) -> Result<(), BookletError> {
        if !(1..=6).contains(&face) {
            return Err(BookletError::BadFace);
        }
        match self.rolls.iter().position(|r| *r == 0) {
            Some(p) => {
                self.rolls[p] = face;
                Ok(())
            }
            None => Err(BookletError::Full),
        }
    }

    /// Takes back the last place filled, if any.
    pub fn pop(&mut self) {
        if let Some(p) = self.rolls.iter().rposition(|r| *r != 0) {
            self.rolls[p] = 0;
        }
    }

    /// Rolls entered so far.
    pub fn len(&self) -> usize {
        self.rolls.iter().filter(|r| **r != 0).count()
    }

    /// Whether nothing is entered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Die `die` (from 0): its first value, if entered.
    pub fn first(&self, die: usize) -> Option<u8> {
        self.rolls.get(die).copied().filter(|r| *r != 0)
    }

    /// Die `die` (from 0): its second value, if entered.
    pub fn second(&self, die: usize) -> Option<u8> {
        self.rolls
            .get(BOOKLET_DICE + die)
            .copied()
            .filter(|r| *r != 0)
    }

    /// Die `die`'s bit once both its values are in: `Some(None)` when
    /// they are equal and the die must be rolled again.
    pub fn die_bit(&self, die: usize) -> Option<Option<u8>> {
        Some(bit(self.first(die)?, self.second(die)?))
    }

    /// The dice, from 0, whose two values are equal.
    pub fn equal_dice(&self) -> impl Iterator<Item = usize> + '_ {
        (0..BOOKLET_DICE).filter(|d| matches!(self.die_bit(*d), Some(None)))
    }

    /// Clears both values of a die ("You must redo both rolls!"); the
    /// next two rolls entered fill its places, first then second.
    pub fn redo(&mut self, die: usize) {
        if die < BOOKLET_DICE {
            self.rolls[die] = 0;
            self.rolls[BOOKLET_DICE + die] = 0;
        }
    }

    /// The character, once all ten are in and no die has equal values:
    /// a lowercase bech32 character.
    pub fn character(&self) -> Result<u8, BookletError> {
        if self.rolls.contains(&0) {
            return Err(BookletError::Incomplete);
        }
        let mut first = [0u8; BOOKLET_DICE];
        let mut second = [0u8; BOOKLET_DICE];
        first.copy_from_slice(&self.rolls[..BOOKLET_DICE]);
        second.copy_from_slice(&self.rolls[BOOKLET_DICE..]);
        let c = character(first, second);
        first.zeroize();
        second.zeroize();
        c
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;

    /// The booklet's tree as the worksheet draws it, leaves left to
    /// right (as in the specification, upper case).
    const LEAVES: &str = "Q P Z R Y 9 X 8 G F 2 T V D W 0 S 3 J N 5 4 K H C E 6 M U A 7 L";

    /// Walks the tree level by level, halving the leaves, with no
    /// arithmetic on the bits.
    fn walk(bits: [u8; 5]) -> u8 {
        let mut leaves: Vec<u8> = LEAVES.split(' ').map(|s| s.as_bytes()[0]).collect();
        for b in bits {
            let half = leaves.len() / 2;
            leaves = if b == 0 {
                leaves[..half].to_vec()
            } else {
                leaves[half..].to_vec()
            };
        }
        assert_eq!(leaves.len(), 1);
        leaves[0]
    }

    #[test]
    fn all_32_paths_reach_the_booklets_leaf() {
        for v in 0..32u8 {
            let bits = [v >> 4 & 1, v >> 3 & 1, v >> 2 & 1, v >> 1 & 1, v & 1];
            let mut first = [0u8; 5];
            let mut second = [0u8; 5];
            for d in 0..5 {
                // A lower second value is bit 0, a higher one bit 1.
                (first[d], second[d]) = if bits[d] == 0 { (4, 3) } else { (3, 4) };
            }
            let c = character(first, second).unwrap();
            assert_eq!(c.to_ascii_uppercase(), walk(bits));
        }
    }

    #[test]
    fn the_tree_is_the_leaves_of_the_booklet() {
        let leaves: Vec<u8> = LEAVES
            .split(' ')
            .map(|s| s.as_bytes()[0].to_ascii_lowercase())
            .collect();
        assert_eq!(leaves, TREE);
    }

    #[test]
    fn a_character_made_in_the_booklets_order() {
        // First roll 2 5 1 6 3, second roll 4 1 6 2 3: die 5 is equal,
        // so it is rolled again: 5 then 2 (bit 0). Bits 1 0 1 0 0 = 20,
        // the leaf 5 in the booklet's tree.
        let mut c = BookletChar::new();
        for f in [2, 5, 1, 6, 3, 4, 1, 6, 2, 3] {
            c.push(f).unwrap();
        }
        assert_eq!(c.push(1), Err(BookletError::Full));
        assert_eq!(c.character(), Err(BookletError::Equal(4)));
        assert_eq!(c.equal_dice().collect::<Vec<_>>(), [4]);
        c.redo(4);
        assert_eq!(c.len(), 8);
        assert_eq!(c.character(), Err(BookletError::Incomplete));
        c.push(5).unwrap();
        c.push(2).unwrap();
        assert_eq!(c.character(), Ok(b'5'));
    }

    #[test]
    fn a_die_with_equal_values_is_never_read() {
        for f in 1..=6u8 {
            let mut first = [1, 1, 1, 1, 1];
            let mut second = [2, 2, 2, 2, 2];
            first[2] = f;
            second[2] = f;
            assert_eq!(character(first, second), Err(BookletError::Equal(2)));
        }
    }

    #[test]
    fn faces_outside_one_to_six_are_refused() {
        let mut c = BookletChar::new();
        assert_eq!(c.push(0), Err(BookletError::BadFace));
        assert_eq!(c.push(7), Err(BookletError::BadFace));
        assert_eq!(
            character([1, 1, 1, 1, 7], [2, 2, 2, 2, 2]),
            Err(BookletError::BadFace)
        );
        assert!(c.is_empty());
    }

    #[test]
    fn pop_takes_back_the_last_roll() {
        let mut c = BookletChar::new();
        c.push(3).unwrap();
        c.push(4).unwrap();
        c.pop();
        assert_eq!(c.first(0), Some(3));
        assert_eq!(c.first(1), None);
    }
}
