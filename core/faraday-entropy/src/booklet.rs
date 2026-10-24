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
//! right, the `code` array of SSS32.ps.
//!
//! The comparison does not depend on how many faces the dice have. The
//! booklet's tracks have pads 1 to 6 and, for dice of seven faces or
//! more ("Die Pads (d7+)"), pads 7 to 20, so a die of 2 to 20 faces
//! ([`BOOKLET_MAX_FACES`]) is read: [`BookletChar::with_faces`]. One
//! die of the five rolls faces 1 to N on both its rolls.
//!
//! [`uniform_face`] is how Faraday Desktop's roller turns a byte of its
//! own randomness into a face of an N-faced die without bias.
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
/// The most faces a die may have: the booklet's last pad is 20.
pub const BOOKLET_MAX_FACES: u8 = 20;
/// The fewest: a die of one face never gives a bit.
pub const BOOKLET_MIN_FACES: u8 = 2;
/// The faces a die has where none is said: the six-faced die.
pub const BOOKLET_FACES: u8 = 6;

/// The booklet's tree, its leaves left to right: the bech32 alphabet in
/// value order (the `code` array of SSS32.ps). The leaf reached by the
/// five bits `b1..b5` is `TREE[16·b1 + 8·b2 + 4·b3 + 2·b4 + b5]`.
pub const TREE: [u8; 32] = *b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// Why a roll or a character was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookletError {
    /// A face that is not 1 to the dice's number of faces.
    BadFace,
    /// A number of faces outside 2 to [`BOOKLET_MAX_FACES`].
    BadFaces,
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

/// Whether a die of `faces` faces is read (2 to 20).
pub fn faces_ok(faces: u8) -> bool {
    (BOOKLET_MIN_FACES..=BOOKLET_MAX_FACES).contains(&faces)
}

/// A byte of fresh randomness as a face 1 to `faces`, uniformly: `None`
/// where the byte is one of the `256 mod faces` highest, which are
/// skipped (rejection sampling), so that no face is likelier than
/// another; also `None` for a number of faces outside 2 to 20.
pub fn uniform_face(byte: u8, faces: u8) -> Option<u8> {
    if !faces_ok(faces) {
        return None;
    }
    let n = u16::from(faces);
    let limit = 256 - 256 % n;
    let b = u16::from(byte);
    (b < limit).then(|| (b % n) as u8 + 1)
}

/// The character the worksheet gives the rolls of dice of `faces`
/// faces: each die's first and second value (die 1 first), as a
/// lowercase bech32 character. Refuses a number of faces outside 2 to
/// 20, a face outside 1 to `faces`, and the first die whose values are
/// equal.
pub fn character(
    first: [u8; BOOKLET_DICE],
    second: [u8; BOOKLET_DICE],
    faces: u8,
) -> Result<u8, BookletError> {
    if !faces_ok(faces) {
        return Err(BookletError::BadFaces);
    }
    let mut bits = [0u8; BOOKLET_DICE];
    for d in 0..BOOKLET_DICE {
        if !(1..=faces).contains(&first[d]) || !(1..=faces).contains(&second[d]) {
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
    /// Entered values 1 to `faces`, 0 for a place not yet filled: first
    /// roll in places 0 to 4, second in 5 to 9.
    rolls: [u8; BOOKLET_ROLLS],
    /// The dice's faces, 2 to 20.
    faces: u8,
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
    /// No rolls yet, of six-faced dice.
    pub fn new() -> Self {
        BookletChar {
            rolls: [0; BOOKLET_ROLLS],
            faces: BOOKLET_FACES,
        }
    }

    /// No rolls yet, of dice of `faces` faces (2 to 20).
    pub fn with_faces(faces: u8) -> Result<Self, BookletError> {
        if !faces_ok(faces) {
            return Err(BookletError::BadFaces);
        }
        Ok(BookletChar {
            rolls: [0; BOOKLET_ROLLS],
            faces,
        })
    }

    /// The dice's faces.
    pub fn faces(&self) -> u8 {
        self.faces
    }

    /// Enters the next roll, in the first empty place of the booklet's
    /// order. Refuses a face outside 1 to the dice's faces and a
    /// character already full.
    pub fn push(&mut self, face: u8) -> Result<(), BookletError> {
        if !(1..=self.faces).contains(&face) {
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
        let c = character(first, second, self.faces);
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
            let c = character(first, second, 6).unwrap();
            assert_eq!(c.to_ascii_uppercase(), walk(bits));
        }
    }

    #[test]
    fn a_twenty_faced_die_reads_by_the_same_comparison() {
        // The bit is the comparison of the two rolls, whatever the
        // number of faces: 17 then 3 is lower (0), 3 then 17 higher (1).
        for v in 0..32u8 {
            let bits = [v >> 4 & 1, v >> 3 & 1, v >> 2 & 1, v >> 1 & 1, v & 1];
            let mut c = BookletChar::with_faces(20).unwrap();
            for b in bits {
                c.push(if b == 0 { 17 } else { 3 }).unwrap();
            }
            for b in bits {
                c.push(if b == 0 { 3 } else { 17 }).unwrap();
            }
            assert_eq!(c.character().unwrap().to_ascii_uppercase(), walk(bits));
        }
    }

    #[test]
    fn faces_past_the_die_are_refused() {
        let mut c = BookletChar::with_faces(20).unwrap();
        assert_eq!(c.push(21), Err(BookletError::BadFace));
        c.push(20).unwrap();
        assert_eq!(c.first(0), Some(20));
        let mut c = BookletChar::with_faces(8).unwrap();
        assert_eq!(c.push(9), Err(BookletError::BadFace));
        assert_eq!(
            character([1, 1, 1, 1, 9], [2, 2, 2, 2, 2], 8),
            Err(BookletError::BadFace)
        );
        for n in [0, 1, 21] {
            assert!(BookletChar::with_faces(n).is_err());
            assert_eq!(character([1; 5], [2; 5], n), Err(BookletError::BadFaces));
        }
    }

    #[test]
    fn every_face_of_every_die_is_equally_likely_from_a_byte() {
        // Every byte value once: each face comes up exactly 256 / N
        // times (rounded down) and the rest are skipped, so a uniform
        // byte gives a uniform face.
        for n in BOOKLET_MIN_FACES..=BOOKLET_MAX_FACES {
            let mut count = [0u32; 21];
            let mut skipped = 0u32;
            for b in 0..=255u8 {
                match uniform_face(b, n) {
                    Some(f) => count[usize::from(f)] += 1,
                    None => skipped += 1,
                }
            }
            let each = 256 / u32::from(n);
            assert!(count[1..=usize::from(n)].iter().all(|&c| c == each), "d{n}");
            assert_eq!(count[0], 0);
            assert!(count[usize::from(n) + 1..].iter().all(|&c| c == 0));
            assert_eq!(skipped, 256 % u32::from(n), "d{n}");
        }
    }

    #[test]
    fn uniform_face_known_answers() {
        // (byte, faces, face): the byte modulo the faces plus one, below
        // the largest multiple of the faces; past it, skipped.
        let cases: [(u8, u8, Option<u8>); 14] = [
            (0, 6, Some(1)),
            (5, 6, Some(6)),
            (6, 6, Some(1)),
            (251, 6, Some(6)),
            (252, 6, None),
            (255, 6, None),
            (255, 8, Some(8)),
            (249, 10, Some(10)),
            (250, 10, None),
            (251, 12, Some(12)),
            (252, 12, None),
            (239, 20, Some(20)),
            (240, 20, None),
            (7, 0, None),
        ];
        for (b, n, want) in cases {
            assert_eq!(uniform_face(b, n), want, "byte {b}, d{n}");
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
            assert_eq!(character(first, second, 6), Err(BookletError::Equal(2)));
        }
    }

    #[test]
    fn faces_outside_one_to_six_are_refused() {
        let mut c = BookletChar::new();
        assert_eq!(c.push(0), Err(BookletError::BadFace));
        assert_eq!(c.push(7), Err(BookletError::BadFace));
        assert_eq!(
            character([1, 1, 1, 1, 7], [2, 2, 2, 2, 2], 6),
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
