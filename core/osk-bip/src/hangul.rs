//! Hangul: the two-set key sequence a syllable is typed with, and the
//! automaton that composes those keys back into syllables.
//!
//! The published Korean list is conjoining jamo (U+1100–U+11C2). A
//! syllable is an initial, a medial and an optional final; the two-set
//! keyboard has one key per simple jamo, and the compound medials and
//! finals are typed as the two keys they are made of. [`keys_of`] is
//! that table, and [`compose`] is the automaton an IME runs: it is what
//! the entry field shows while a word is being typed.
//!
//! Composition is arithmetic over U+AC00, so there is no table of
//! syllables.

/// First Hangul syllable, `가`.
const SYLLABLE_BASE: u32 = 0xAC00;
/// Medials per initial.
const MEDIALS: u32 = 21;
/// Finals per medial, the empty final included.
const FINALS: u32 = 28;

/// First conjoining initial, `ᄀ`.
const L_BASE: u32 = 0x1100;
/// First conjoining medial, `ᅡ`.
const V_BASE: u32 = 0x1161;
/// First conjoining final, `ᆨ`.
const T_BASE: u32 = 0x11A8;

/// The compatibility jamo of each initial, in initial order. Every one
/// of them is a single key on the two-set keyboard.
const INITIAL_KEYS: [char; 19] = [
    'ㄱ', 'ㄲ', 'ㄴ', 'ㄷ', 'ㄸ', 'ㄹ', 'ㅁ', 'ㅂ', 'ㅃ', 'ㅅ', 'ㅆ', 'ㅇ', 'ㅈ', 'ㅉ', 'ㅊ', 'ㅋ',
    'ㅌ', 'ㅍ', 'ㅎ',
];

/// The keys of each medial, in medial order: one for a simple vowel,
/// two for a compound one (`ᅪ` is ㅗ then ㅏ).
const MEDIAL_KEYS: [&[char]; 21] = [
    &['ㅏ'],
    &['ㅐ'],
    &['ㅑ'],
    &['ㅒ'],
    &['ㅓ'],
    &['ㅔ'],
    &['ㅕ'],
    &['ㅖ'],
    &['ㅗ'],
    &['ㅗ', 'ㅏ'],
    &['ㅗ', 'ㅐ'],
    &['ㅗ', 'ㅣ'],
    &['ㅛ'],
    &['ㅜ'],
    &['ㅜ', 'ㅓ'],
    &['ㅜ', 'ㅔ'],
    &['ㅜ', 'ㅣ'],
    &['ㅠ'],
    &['ㅡ'],
    &['ㅡ', 'ㅣ'],
    &['ㅣ'],
];

/// The keys of each final, in final order: one for a simple consonant,
/// two for a compound one (`ᆪ` is ㄱ then ㅅ).
const FINAL_KEYS: [&[char]; 27] = [
    &['ㄱ'],
    &['ㄲ'],
    &['ㄱ', 'ㅅ'],
    &['ㄴ'],
    &['ㄴ', 'ㅈ'],
    &['ㄴ', 'ㅎ'],
    &['ㄷ'],
    &['ㄹ'],
    &['ㄹ', 'ㄱ'],
    &['ㄹ', 'ㅁ'],
    &['ㄹ', 'ㅂ'],
    &['ㄹ', 'ㅅ'],
    &['ㄹ', 'ㅌ'],
    &['ㄹ', 'ㅍ'],
    &['ㄹ', 'ㅎ'],
    &['ㅁ'],
    &['ㅂ'],
    &['ㅂ', 'ㅅ'],
    &['ㅅ'],
    &['ㅆ'],
    &['ㅇ'],
    &['ㅈ'],
    &['ㅊ'],
    &['ㅋ'],
    &['ㅌ'],
    &['ㅍ'],
    &['ㅎ'],
];

/// The keys that type `jamo`, or `None` when it is not a conjoining
/// jamo of a modern syllable.
pub fn keys_of(jamo: char) -> Option<&'static [char]> {
    let c = jamo as u32;
    if (L_BASE..L_BASE + 19).contains(&c) {
        return Some(&INITIAL_KEYS[(c - L_BASE) as usize..(c - L_BASE) as usize + 1]);
    }
    if (V_BASE..V_BASE + MEDIALS).contains(&c) {
        return Some(MEDIAL_KEYS[(c - V_BASE) as usize]);
    }
    if (T_BASE..T_BASE + 27).contains(&c) {
        return Some(FINAL_KEYS[(c - T_BASE) as usize]);
    }
    None
}

/// Whether `key` starts a syllable: every consonant key does.
fn initial_index(key: char) -> Option<u32> {
    INITIAL_KEYS
        .iter()
        .position(|&k| k == key)
        .map(|i| i as u32)
}

/// The medial a single vowel key is, if it is one on its own.
fn medial_index(key: char) -> Option<u32> {
    MEDIAL_KEYS
        .iter()
        .position(|m| m == &[key])
        .map(|i| i as u32)
}

/// The medial `first` and `second` join into (`ㅗ` and `ㅏ` are `ㅘ`).
fn join_medial(first: u32, second: char) -> Option<u32> {
    let a = MEDIAL_KEYS[first as usize];
    MEDIAL_KEYS
        .iter()
        .position(|m| m.len() == 2 && m[0] == a[0] && a.len() == 1 && m[1] == second)
        .map(|i| i as u32)
}

/// The final a single consonant key is, if a syllable can end in it.
/// One-based, as the syllable arithmetic counts finals.
fn final_index(key: char) -> Option<u32> {
    FINAL_KEYS
        .iter()
        .position(|f| f == &[key])
        .map(|i| i as u32 + 1)
}

/// The final `first` and `second` join into (`ㄱ` and `ㅅ` are `ㄳ`).
fn join_final(first: u32, second: char) -> Option<u32> {
    let a = FINAL_KEYS[first as usize - 1];
    FINAL_KEYS
        .iter()
        .position(|f| f.len() == 2 && f[0] == a[0] && a.len() == 1 && f[1] == second)
        .map(|i| i as u32 + 1)
}

/// The key a compound final's second half is typed with, which is the
/// consonant that moves to the next syllable when a vowel follows.
fn split_final(t: u32) -> Option<(u32, char)> {
    let f = FINAL_KEYS[t as usize - 1];
    (f.len() == 2).then(|| (final_index(f[0]).expect("a simple final"), f[1]))
}

/// Longest run [`compose`] returns: one syllable takes at least two
/// keys, but a lone jamo takes one, so the run is never longer than the
/// keys given.
pub const MAX_COMPOSED: usize = 16;

/// A composed run of Hangul, inline.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Composed {
    buf: [char; MAX_COMPOSED],
    len: u8,
}

impl Composed {
    /// The composed characters.
    pub fn as_chars(&self) -> &[char] {
        &self.buf[..usize::from(self.len)]
    }

    fn push(&mut self, c: char) {
        if usize::from(self.len) < MAX_COMPOSED {
            self.buf[usize::from(self.len)] = c;
            self.len += 1;
        }
    }
}

/// The syllable an initial, a medial and a final make.
fn syllable(l: u32, v: u32, t: u32) -> char {
    char::from_u32(SYLLABLE_BASE + (l * MEDIALS + v) * FINALS + t).expect("a Hangul syllable")
}

/// What is on the field so far: a syllable being built, or a key that
/// stands alone.
#[derive(Clone, Copy)]
enum State {
    Empty,
    /// An initial with no medial yet: the consonant shows on its own.
    Lead(u32, char),
    /// An initial and a medial, with a final once there is one.
    Syllable(u32, u32, u32),
    /// A vowel with no initial before it, which the lists never produce
    /// but a finger can type.
    Vowel(u32),
}

/// The keys of a Korean word, or of a prefix being typed, as a reader
/// sees them: the two-set automaton, which is what any Korean IME runs.
/// A consonant that has no vowel after it yet shows as itself, and a
/// consonant that has become a final shows inside its syllable.
pub fn compose(keys: impl IntoIterator<Item = char>) -> Composed {
    let mut out = Composed {
        buf: ['\0'; MAX_COMPOSED],
        len: 0,
    };
    let mut state = State::Empty;
    let flush = |out: &mut Composed, state: State| match state {
        State::Empty => {}
        State::Lead(_, key) => out.push(key),
        State::Syllable(l, v, t) => out.push(syllable(l, v, t)),
        State::Vowel(v) => out.push(MEDIAL_KEYS[v as usize][0]),
    };
    for key in keys {
        match (state, medial_index(key), initial_index(key)) {
            // A vowel after a bare initial completes a syllable.
            (State::Lead(l, _), Some(v), _) => state = State::Syllable(l, v, 0),
            // A vowel after a syllable with no final joins the medial
            // where the two make a compound one, and otherwise starts a
            // syllable of its own.
            (State::Syllable(l, v, 0), Some(next), _) => match join_medial(v, key) {
                Some(joined) => state = State::Syllable(l, joined, 0),
                None => {
                    flush(&mut out, state);
                    state = State::Vowel(next);
                }
            },
            // A vowel after a final takes that final as its initial: the
            // consonant belonged to this syllable only until a vowel
            // followed it.
            (State::Syllable(l, v, t), Some(next), _) => {
                let (kept, moved) = split_final(t).unwrap_or((0, FINAL_KEYS[t as usize - 1][0]));
                out.push(syllable(l, v, kept));
                state = match initial_index(moved) {
                    Some(l2) => State::Syllable(l2, next, 0),
                    None => State::Vowel(next),
                };
            }
            (State::Vowel(v), Some(next), _) => match join_medial(v, key) {
                Some(joined) => state = State::Vowel(joined),
                None => {
                    flush(&mut out, state);
                    state = State::Vowel(next);
                }
            },
            (State::Empty, Some(v), _) => state = State::Vowel(v),
            // A consonant with a syllable open becomes its final, or
            // extends the final it already has, or starts the next
            // syllable.
            (State::Syllable(l, v, 0), None, Some(next)) => match final_index(key) {
                Some(t) => state = State::Syllable(l, v, t),
                None => {
                    out.push(syllable(l, v, 0));
                    state = State::Lead(next, key);
                }
            },
            (State::Syllable(l, v, t), None, Some(next)) => match join_final(t, key) {
                Some(joined) => state = State::Syllable(l, v, joined),
                None => {
                    out.push(syllable(l, v, t));
                    state = State::Lead(next, key);
                }
            },
            (_, None, Some(next)) => {
                flush(&mut out, state);
                state = State::Lead(next, key);
            }
            // Not a key of this keyboard: keep it where it was typed.
            (_, None, None) => {
                flush(&mut out, state);
                state = State::Empty;
                out.push(key);
            }
        }
    }
    flush(&mut out, state);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bip39::Language;
    use alloc::vec::Vec;

    fn keys_of_word(word: &str) -> Vec<char> {
        word.chars()
            .flat_map(|c| keys_of(c).expect("a modern jamo").iter().copied())
            .collect()
    }

    /// Every Korean word types as a sequence of two-set keys, and those
    /// keys compose back into the word a reader sees.
    #[test]
    fn every_korean_word_types_and_reads_back() {
        for i in 0..2048u16 {
            let keys = keys_of_word(Language::Korean.word_nfkd(i));
            let shown: Vec<char> = Language::Korean.word_display(i).chars().collect();
            assert_eq!(
                compose(keys).as_chars(),
                shown.as_slice(),
                "word {i}: {}",
                Language::Korean.word_display(i)
            );
        }
    }

    /// A consonant that ends one syllable and a vowel that starts the
    /// next are the same keys either way, so the field follows the
    /// fingers: `ㄱㅏㄱ` is 각 and one more vowel makes it 가거.
    #[test]
    fn a_final_moves_to_the_next_syllable_when_a_vowel_follows() {
        assert_eq!(compose(['ㄱ', 'ㅏ']).as_chars(), ['가']);
        assert_eq!(compose(['ㄱ', 'ㅏ', 'ㄱ']).as_chars(), ['각']);
        assert_eq!(compose(['ㄱ', 'ㅏ', 'ㄱ', 'ㅓ']).as_chars(), ['가', '거']);
        assert_eq!(compose(['ㄱ']).as_chars(), ['ㄱ']);
        // A compound final splits: 앉 then ㅏ is 안자.
        assert_eq!(compose(['ㅇ', 'ㅏ', 'ㄴ', 'ㅈ']).as_chars(), ['앉']);
        assert_eq!(
            compose(['ㅇ', 'ㅏ', 'ㄴ', 'ㅈ', 'ㅏ']).as_chars(),
            ['안', '자']
        );
        // A compound medial is two vowel keys.
        assert_eq!(compose(['ㄱ', 'ㅗ', 'ㅏ']).as_chars(), ['과']);
    }
}
