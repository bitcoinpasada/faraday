//! The backup quiz (UX.md D5, D6, K1; `docs/PLANNING.md` §8.2 #7): every
//! word of a mnemonic, in random order, each chosen from four candidates.
//! The Create wizard runs it after the words are shown; the Backup flow
//! runs it for any loaded key at any time.
//!
//! The words come from a list rather than from a language: BIP-39's
//! words or a SLIP-39 share's, which is the same question over a
//! different list (`docs/PLANNING.md` §16.107).
//!
//! The order and the decoys come from a small deterministic generator
//! (xorshift64*) seeded from the tick clock and a hash of the word
//! indices ([`Quiz::seed`]): there is no `rand` crate in the core, and
//! the quiz needs unpredictability only against a user guessing the next
//! question, not against an attacker. A fixed clock in a review script
//! therefore reproduces the same quiz.
//!
//! Word indices live in fixed arrays zeroized on drop. This file is listed
//! in `tools/lint-secrets.sh` and may hold no heap text.

use osk_bip::slip39::MAX_WORDS;
use osk_crypto::{Zeroize, ZeroizeOnDrop, sha256};

use crate::load::EntryList;

/// Candidates per question.
pub const CHOICES: usize = 4;

/// Where the quiz is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuizState {
    /// A question is on screen.
    Asking,
    /// The last answer was wrong; the same word is asked again on retry.
    Wrong,
    /// Every word was answered correctly.
    Passed,
}

/// One quiz over a mnemonic's words.
pub struct Quiz {
    /// Word positions in the order they are asked.
    order: [u8; MAX_WORDS],
    len: u8,
    /// Questions answered correctly so far.
    done: u8,
    /// Wordlist indices of the candidates on screen.
    choices: [u16; CHOICES],
    /// Slot of the right word.
    correct: u8,
    /// Slot the user picked wrongly, while in `Wrong`.
    wrong: u8,
    /// The slot the check is on, before Continue confirms it
    /// (`docs/DESIGN.md` §2.7).
    picked: Option<u8>,
    state: QuizState,
    /// K1: someone else holds the device; nothing is revealed.
    helper: bool,
    rng: u64,
}

impl Zeroize for Quiz {
    fn zeroize(&mut self) {
        self.order.zeroize();
        self.choices.zeroize();
        self.len = 0;
        self.done = 0;
        self.correct = 0;
        self.wrong = 0;
        self.picked = None;
        self.rng = 0;
    }
}

impl Drop for Quiz {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Quiz {}

impl Quiz {
    /// A seed for [`new`](Self::new): the clock mixed with SHA-256 of the
    /// word indices, so that two runs on the same key at different times
    /// ask in different orders.
    pub fn seed(now_ms: u64, indices: &[u16]) -> u64 {
        let mut bytes = [0u8; MAX_WORDS * 2];
        for (i, &w) in indices.iter().take(MAX_WORDS).enumerate() {
            bytes[2 * i..2 * i + 2].copy_from_slice(&w.to_le_bytes());
        }
        let mut hash = sha256(&bytes[..indices.len().min(MAX_WORDS) * 2]);
        bytes.zeroize();
        let mut seed = [0u8; 8];
        seed.copy_from_slice(&hash[..8]);
        hash.zeroize();
        now_ms ^ u64::from_le_bytes(seed)
    }

    /// Shuffles the positions of `indices` and poses the first question.
    pub fn new(indices: &[u16], list: EntryList, helper: bool, seed: u64) -> Self {
        let len = indices.len().min(MAX_WORDS);
        let mut q = Quiz {
            order: [0; MAX_WORDS],
            len: len as u8,
            done: 0,
            choices: [0; CHOICES],
            correct: 0,
            wrong: 0,
            picked: None,
            state: QuizState::Asking,
            helper,
            rng: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        };
        for (i, p) in q.order.iter_mut().enumerate().take(len) {
            *p = i as u8;
        }
        // Fisher–Yates.
        for i in (1..len).rev() {
            let j = q.next_below(i as u32 + 1) as usize;
            q.order.swap(i, j);
        }
        q.pose(indices, list);
        q
    }

    fn next_below(&mut self, n: u32) -> u32 {
        // xorshift64*.
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) % u64::from(n.max(1))) as u32
    }

    /// Whether the helper variant is on (K1).
    pub fn helper(&self) -> bool {
        self.helper
    }

    /// The state.
    pub fn state(&self) -> QuizState {
        self.state
    }

    /// 0-based position of the word being asked.
    pub fn position(&self) -> usize {
        usize::from(self.order[usize::from(self.done.min(self.len.saturating_sub(1)))])
    }

    /// The candidates' wordlist indices, [`CHOICES`] of them.
    pub fn choices(&self) -> &[u16] {
        &self.choices
    }

    /// The slot holding the right word.
    pub fn correct_slot(&self) -> usize {
        usize::from(self.correct)
    }

    /// The slot picked wrongly, while in [`QuizState::Wrong`].
    pub fn wrong_slot(&self) -> Option<usize> {
        (self.state == QuizState::Wrong).then_some(usize::from(self.wrong))
    }

    /// `(answered correctly, total)`.
    pub fn progress(&self) -> (usize, usize) {
        (usize::from(self.done), usize::from(self.len))
    }

    /// The slot the check is on, before Continue confirms it.
    pub fn picked(&self) -> Option<usize> {
        self.picked.map(usize::from)
    }

    /// Checks `slot`. Nothing is answered until
    /// [`confirm`](Self::confirm): a choice is checked by the tap and
    /// confirmed by Continue (`docs/DESIGN.md` §2.7).
    pub fn pick(&mut self, slot: usize) {
        if self.state == QuizState::Asking && slot < CHOICES {
            self.picked = Some(slot as u8);
        }
    }

    /// Answers with the checked slot. Returns whether it was right; a
    /// right answer advances to the next word (or passes), a wrong one
    /// waits for [`retry`](Self::retry).
    pub fn confirm(&mut self, indices: &[u16], list: EntryList) -> bool {
        let Some(slot) = self.picked.map(usize::from) else {
            return false;
        };
        self.answer(slot, indices, list)
    }

    /// Answers with `slot`, whether or not it was checked first.
    fn answer(&mut self, slot: usize, indices: &[u16], list: EntryList) -> bool {
        if self.state != QuizState::Asking || slot >= CHOICES {
            return false;
        }
        self.picked = None;
        if slot == self.correct_slot() {
            self.done += 1;
            if self.done >= self.len {
                self.state = QuizState::Passed;
                self.choices.zeroize();
            } else {
                self.pose(indices, list);
            }
            true
        } else {
            self.wrong = slot as u8;
            self.state = QuizState::Wrong;
            false
        }
    }

    /// Asks the same word again with fresh decoys.
    pub fn retry(&mut self, indices: &[u16], list: EntryList) {
        if self.state == QuizState::Wrong {
            self.state = QuizState::Asking;
            self.picked = None;
            self.pose(indices, list);
        }
    }

    /// Fills the candidates for the current word: the right word, one
    /// decoy sharing its first letter, two drawn from the whole list.
    fn pose(&mut self, indices: &[u16], list: EntryList) {
        let Some(&right) = indices.get(self.position()) else {
            return;
        };
        let mut picks = [right; CHOICES];
        // Same first letter (in the published form; for the non-Latin
        // lists the first character).
        let mut prefix = [0u8; 4];
        let word = list.word(right);
        let first = word.chars().next().unwrap_or('a');
        let prefix = first.encode_utf8(&mut prefix);
        let similar = list.candidates(prefix).filter(|&i| i != right).count();
        picks[1] = if similar > 0 {
            let k = self.next_below(similar as u32) as usize;
            list.candidates(prefix)
                .filter(|&i| i != right)
                .nth(k)
                .unwrap_or(right)
        } else {
            self.random_word_except(&picks[..1], list)
        };
        picks[2] = self.random_word_except(&picks[..2], list);
        picks[3] = self.random_word_except(&picks[..3], list);
        // Place the right word in a random slot; the decoys keep their
        // relative order in the remaining slots.
        let slot = self.next_below(CHOICES as u32) as usize;
        let mut next = 1;
        for (s, c) in self.choices.iter_mut().enumerate() {
            if s == slot {
                *c = picks[0];
            } else {
                *c = picks[next];
                next += 1;
            }
        }
        self.correct = slot as u8;
        picks.zeroize();
    }

    fn random_word_except(&mut self, taken: &[u16], list: EntryList) -> u16 {
        loop {
            let w = self.next_below(u32::from(list.word_count())) as u16;
            if !taken.contains(&w) {
                return w;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osk_bip::bip39::{Language, Mnemonic};

    fn abandon() -> Mnemonic {
        Mnemonic::from_entropy(Language::English, &[0u8; 16]).unwrap()
    }

    #[test]
    fn every_word_is_asked_once_in_a_seeded_order() {
        let m = abandon();
        let lang = EntryList::Bip39(Language::English);
        let seed = Quiz::seed(1000, m.indices());
        let mut q = Quiz::new(m.indices(), lang, false, seed);
        let mut seen = [false; 12];
        for step in 0..12 {
            assert_eq!(q.progress(), (step, 12));
            assert_eq!(q.state(), QuizState::Asking);
            let p = q.position();
            assert!(!seen[p], "asked twice: {p}");
            seen[p] = true;
            let c = q.choices();
            assert_eq!(c[q.correct_slot()], m.indices()[p]);
            // Four distinct candidates; one decoy shares the first letter.
            for i in 0..4 {
                for j in i + 1..4 {
                    assert_ne!(c[i], c[j]);
                }
            }
            let first = lang.word(m.indices()[p]).chars().next().unwrap();
            assert!(
                c.iter()
                    .enumerate()
                    .any(|(s, &w)| s != q.correct_slot() && lang.word(w).starts_with(first))
            );
            q.pick(q.correct_slot());
            assert_eq!(q.picked(), Some(q.correct_slot()));
            assert!(q.confirm(m.indices(), lang));
            assert_eq!(q.picked(), None, "the check clears with the answer");
        }
        assert_eq!(q.state(), QuizState::Passed);
        assert!(seen.iter().all(|&s| s));
        // The same seed asks in the same order; another seed differs.
        let a = Quiz::new(m.indices(), lang, false, seed);
        let b = Quiz::new(m.indices(), lang, false, seed);
        assert_eq!(a.position(), b.position());
        assert_eq!(a.choices(), b.choices());
        let orders: alloc::vec::Vec<usize> = (0..50u64)
            .map(|t| {
                Quiz::new(m.indices(), lang, false, Quiz::seed(t * 7919, m.indices())).position()
            })
            .collect();
        assert!(
            orders.iter().any(|&p| p != orders[0]),
            "seeds vary the order"
        );
    }

    #[test]
    fn a_choice_is_checked_then_confirmed_and_a_wrong_one_waits_for_retry() {
        let m = abandon();
        let lang = EntryList::Bip39(Language::English);
        let mut q = Quiz::new(m.indices(), lang, true, 42);
        assert!(q.helper());
        // Continue does nothing until a row is checked.
        assert!(!q.confirm(m.indices(), lang));
        assert_eq!(q.progress(), (0, 12));
        let wrong = (q.correct_slot() + 1) % CHOICES;
        q.pick(wrong);
        assert!(!q.confirm(m.indices(), lang));
        assert_eq!(q.state(), QuizState::Wrong);
        assert_eq!(q.wrong_slot(), Some(wrong));
        assert_eq!(q.progress(), (0, 12));
        // Answers are ignored until retry.
        q.pick(q.correct_slot());
        assert_eq!(q.picked(), None, "a wrong screen has nothing to check");
        assert!(!q.confirm(m.indices(), lang));
        let p = q.position();
        q.retry(m.indices(), lang);
        assert_eq!(q.state(), QuizState::Asking);
        assert_eq!(q.position(), p, "same word again");
        assert_eq!(q.wrong_slot(), None);
        q.pick(q.correct_slot());
        assert!(q.confirm(m.indices(), lang));
        assert_eq!(q.progress(), (1, 12));
    }
}
