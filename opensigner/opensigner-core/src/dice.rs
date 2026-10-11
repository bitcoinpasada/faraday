//! Tools › Dice passphrase: a passphrase rolled word by word from one of
//! the three EFF diceware lists (`docs/PLANNING.md` §8.1 #11).
//!
//! The rolls are the secret. They live in a [`DiceRolls`] that zeroizes
//! on drop, and the words they name are `&'static str` of the baked list,
//! looked up as the screen draws. Nothing is stored: the tool issues no
//! `StoreSecret`, and leaving it drops the rolls.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::diceware::List;
use osk_entropy::{DICE_LONG_RUN, DICE_SEQUENTIAL, DiceRolls};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// How long the passphrase may be, in words. Four is the fewest worth
/// rolling and ten is as many as the panel holds.
pub const WORD_COUNTS: [usize; 6] = [4, 5, 6, 7, 8, 10];

/// The count the Choice opens with: six long-list words, 77 bits.
pub const DEFAULT_WORDS: usize = 2;

/// How long the newest roll shows before it masks, in milliseconds. The
/// Create wizard's own run uses the same window.
const MASK_MS: u64 = 500;

/// Where the tool is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Which list.
    List,
    /// How many words.
    Words,
    /// The dice pad.
    Rolls,
    /// The passphrase, on its panel.
    Result,
    /// Leaving with rolls entered: keep them or discard them.
    Discard,
}

/// The dice passphrase tool's state.
pub struct DicePassphrase {
    step: Step,
    /// The step the discard confirm came from, which "Not now" returns
    /// to.
    resume: Step,
    list: List,
    /// Index into [`WORD_COUNTS`].
    words: usize,
    rolls: DiceRolls,
    /// When the newest roll was thrown, so it can show for [`MASK_MS`].
    typed_at: u64,
}

impl Zeroize for DicePassphrase {
    fn zeroize(&mut self) {
        self.rolls.zeroize();
        self.typed_at = 0;
    }
}

impl Drop for DicePassphrase {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for DicePassphrase {}

impl Default for DicePassphrase {
    fn default() -> Self {
        Self::new()
    }
}

impl DicePassphrase {
    /// The tool as it opens: the list step, the long list checked, six
    /// words.
    pub fn new() -> Self {
        DicePassphrase {
            step: Step::List,
            resume: Step::Rolls,
            list: List::Large,
            words: DEFAULT_WORDS,
            rolls: DiceRolls::new(),
            typed_at: 0,
        }
    }

    /// Which step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Goes to `step`.
    pub fn go(&mut self, step: Step) {
        self.step = step;
    }

    /// Which list.
    pub fn list(&self) -> List {
        self.list
    }

    /// Sets the list, which starts the rolls over: a roll of five dice
    /// names nothing on a list of four.
    pub fn set_list(&mut self, list: List) {
        if list != self.list {
            self.list = list;
            self.rolls.clear();
        }
    }

    /// Index into [`WORD_COUNTS`] of the length being rolled.
    pub fn words_index(&self) -> usize {
        self.words
    }

    /// Sets the length, which drops any roll past the new target.
    pub fn set_words(&mut self, index: usize) {
        if index < WORD_COUNTS.len() && index != self.words {
            self.words = index;
            self.rolls.clear();
        }
    }

    /// Words the passphrase will have.
    pub fn word_count(&self) -> usize {
        WORD_COUNTS[self.words]
    }

    /// Bits `words` words of this list carry, rounded down.
    pub fn bits_of(&self, words: usize) -> u32 {
        self.list.bits(words)
    }

    /// Bits the whole passphrase carries.
    pub fn bits(&self) -> u32 {
        self.bits_of(self.word_count())
    }

    /// Rolls the passphrase needs.
    pub fn needed(&self) -> usize {
        self.word_count() * self.list.dice_per_word()
    }

    /// Rolls thrown so far.
    pub fn len(&self) -> usize {
        self.rolls.len()
    }

    /// Whether nothing has been rolled, which is what Back asks about.
    pub fn is_empty(&self) -> bool {
        self.rolls.is_empty()
    }

    /// Whether every roll the passphrase needs has been thrown.
    pub fn ready(&self) -> bool {
        self.len() >= self.needed()
    }

    /// Adds a roll (`'1'`–`'6'`) at `now_ms`.
    pub fn push(&mut self, c: char, now_ms: u64) -> bool {
        let Some(d) = c.to_digit(10) else {
            return false;
        };
        if !(1..=6).contains(&d) || self.len() >= self.needed() || !self.rolls.push(d as u8) {
            return false;
        }
        self.typed_at = now_ms;
        true
    }

    /// Removes the last roll.
    pub fn pop(&mut self) {
        self.rolls.pop();
    }

    /// The rolls as digits, for the masked run above the pad.
    pub fn entry_chars(&self) -> impl Iterator<Item = char> + '_ {
        self.rolls.rolls().iter().map(|&r| (b'0' + r) as char)
    }

    /// The newest roll while it is still unmasked at `now_ms`.
    pub fn visible_last(&self, now_ms: u64) -> Option<char> {
        if self.rolls.is_empty() || now_ms >= self.typed_at + MASK_MS {
            return None;
        }
        self.entry_chars().last()
    }

    /// When the newest roll masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        (!self.rolls.is_empty()).then_some(self.typed_at + MASK_MS)
    }

    /// The words every completed group of rolls names, in order. A group
    /// still short of its dice names nothing yet.
    pub fn words(&self) -> Vec<&'static str> {
        self.rolls
            .rolls()
            .chunks_exact(self.list.dice_per_word())
            .filter_map(|group| self.list.word(group))
            .collect()
    }

    /// The passphrase as one line, which is what the panel holds and
    /// what a person copies.
    pub fn passphrase(&self) -> String {
        self.words().join(" ")
    }

    /// The sanity flags the Create wizard's dice run raises, over this
    /// run: a face repeated eight times, or twelve rolls counting up or
    /// down. A run of fifty rolls at most is too short for the skew test
    /// the Create wizard applies, so that one is not made here.
    pub fn warnings(&self) -> (bool, bool) {
        let s = self.rolls.stats();
        (
            s.longest_run >= DICE_LONG_RUN,
            s.longest_sequence >= DICE_SEQUENTIAL,
        )
    }

    /// The longest run of one face, and which face it was.
    pub fn longest_run(&self) -> (u32, u8) {
        let s = self.rolls.stats();
        (s.longest_run, s.run_face)
    }

    /// One step back. `false` leaves the tool; the discard confirm is a
    /// step of its own, because leaving with rolls entered throws them
    /// away.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            Step::List => return false,
            Step::Words => Step::List,
            Step::Rolls if self.rolls.is_empty() => Step::Words,
            Step::Rolls | Step::Result => {
                self.resume = self.step;
                Step::Discard
            }
            Step::Discard => self.resume,
        };
        true
    }

    /// "Not now" on the discard confirm: back to the step it covered.
    pub fn keep(&mut self) {
        self.step = self.resume;
    }
}
