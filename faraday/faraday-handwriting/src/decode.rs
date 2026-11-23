//! Dictionary decoding: the CTC log-likelihood of each of the 2,048 BIP39
//! words given a strip's logits.
//!
//! The dictionary is a prefix tree. Each node carries two CTC states, L (its
//! letter) and B (a blank after its letter); the root carries the prefix state
//! P (blank or digit, looping), so that a line number before the word is
//! allowed. L of a node is entered from L of itself, B of its parent, L of its
//! parent if the letter differs, and P if the parent is the root. The score of
//! a word ending at node n is logadd(L_n, B_n) at the last frame. States more
//! than the beam below the best state of the previous frame are dropped.

use core::fmt;

use faraday_bip::bip39::Language;
use faraday_text::Text;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::Error;
use crate::mathf::{expf, logf};
use crate::word::{CLASSES, Logits};

/// First digit class.
const FIRST_DIGIT: usize = 27;
/// A score of a state that was dropped.
const NEG: f32 = -1e30;
const LUT_SIZE: usize = 1024;

static LUT_BYTES: &[u8] = include_bytes!("../weights/logadd.bin");

const _: () = assert!(LUT_BYTES.len() == 4 * (LUT_SIZE + 1));

/// log(1 + exp(-d)) for d = i / 64, as the reference's table, bit for bit.
static LUT: [f32; LUT_SIZE + 1] = build_lut();

const fn build_lut() -> [f32; LUT_SIZE + 1] {
    let mut out = [0f32; LUT_SIZE + 1];
    let mut i = 0;
    while i <= LUT_SIZE {
        let b = [
            LUT_BYTES[4 * i],
            LUT_BYTES[4 * i + 1],
            LUT_BYTES[4 * i + 2],
            LUT_BYTES[4 * i + 3],
        ];
        out[i] = f32::from_bits(u32::from_le_bytes(b));
        i += 1;
    }
    out
}

/// log(exp(a) + exp(b)) by table and linear interpolation.
pub(crate) fn logadd(a: f32, b: f32) -> f32 {
    let m = if a > b { a } else { b };
    let d = if a > b { a - b } else { b - a };
    // With NaN in `a` or `b` the comparisons are false, so `m` is `b`, and
    // `d` is NaN: `b` is returned.
    if d.is_nan() || d >= 16.0 {
        return m;
    }
    let f = d * 64.0;
    let i = (f as usize).min(LUT_SIZE - 1);
    let (lo, hi) = (LUT[i], LUT[i + 1]);
    m + lo + (f - i as f32) * (hi - lo)
}

/// Refuses a list that `word_scores_in` would refuse.
pub(crate) fn check_list(words: &[&str]) -> Result<(), Error> {
    Trie::new(words).map(|_| ())
}

/// Costs a pass over the table (an exponential and a logarithm for each value);
/// the in-place log-softmax over the classes of each frame.
pub fn log_softmax(logits: &mut Logits) {
    let frames = logits.frames;
    for t in 0..frames {
        let at = |k: usize| k * frames + t;
        let mut m = logits.data.get(t).copied().unwrap_or(0.0);
        for k in 1..CLASSES {
            let v = logits.data.get(at(k)).copied().unwrap_or(m);
            m = if v > m { v } else { m };
        }
        let mut s = 0.0f32;
        for k in 0..CLASSES {
            s += expf(logits.data.get(at(k)).copied().unwrap_or(m) - m);
        }
        let l = m + logf(s);
        for k in 0..CLASSES {
            if let Some(v) = logits.data.get_mut(at(k)) {
                *v -= l;
            }
        }
    }
}

/// The most letters all the words of a list may hold together; a list past it
/// is refused (`Error::WordList`) before the tree is built, so the tree has
/// at most this many nodes and its size is bounded for any input. The 2,048
/// BIP39 words hold under 14,000.
pub(crate) const MAX_LIST_LETTERS: usize = 1 << 18;

/// The dictionary as a prefix tree, nodes stored parent before child.
struct Trie {
    parent: Vec<u32>,
    letter: Vec<u8>,
    word_end: Vec<u32>,
}

impl Trie {
    /// The tree of `words`: lowercase a-z only, none empty, none twice, at
    /// least one, at most 65,535, and at most `MAX_LIST_LETTERS` letters in all.
    fn new(words: &[&str]) -> Result<Self, Error> {
        if words.is_empty() || words.len() > usize::from(u16::MAX) {
            return Err(Error::WordList);
        }
        let mut letters = 0usize;
        for word in words {
            letters = letters
                .checked_add(word.len())
                .filter(|&n| n <= MAX_LIST_LETTERS)
                .ok_or(Error::WordList)?;
        }
        let mut t = Trie {
            parent: vec![0],
            letter: vec![0],
            word_end: Vec::with_capacity(words.len()),
        };
        let mut child: Vec<[u32; 26]> = vec![[0; 26]];
        let mut ended = vec![false];
        for word in words {
            if word.is_empty() || !word.bytes().all(|c| c.is_ascii_lowercase()) {
                return Err(Error::WordList);
            }
            let mut node = 0usize;
            for &ch in word.as_bytes() {
                let k = usize::from(ch.wrapping_sub(b'a'));
                let slot = child
                    .get(node)
                    .and_then(|c| c.get(k))
                    .copied()
                    .ok_or(Error::Internal)?;
                node = if slot != 0 {
                    slot as usize
                } else {
                    let new = t.parent.len();
                    t.parent
                        .push(u32::try_from(node).map_err(|_| Error::WordList)?);
                    t.letter.push(ch - b'a' + 1);
                    child.push([0; 26]);
                    ended.push(false);
                    if let Some(s) = child.get_mut(node).and_then(|c| c.get_mut(k)) {
                        *s = u32::try_from(new).map_err(|_| Error::WordList)?;
                    }
                    new
                };
            }
            // A word ending where another did is a repeat.
            if ended.get(node).copied() != Some(false) {
                return Err(Error::WordList);
            }
            if let Some(e) = ended.get_mut(node) {
                *e = true;
            }
            t.word_end.push(node as u32);
        }
        Ok(t)
    }
}

/// Blank or digit at frame `t`.
fn prefix_logprob(lp: &[f32], frames: usize, t: usize) -> f32 {
    let at = |k: usize| lp.get(k * frames + t).copied().unwrap_or(NEG);
    let mut p = at(0);
    for c in FIRST_DIGIT..CLASSES {
        p = logadd(p, at(c));
    }
    p
}

/// The log-likelihood of each word of the list the scores were made from, in
/// the order of that list, and the list itself (so that `best` names its own
/// words), wiped when dropped.
pub struct Scores {
    data: Zeroizing<Vec<f32>>,
    words: Vec<Text>,
}

/// A word and its score.
pub struct Candidate {
    /// Index in the list the scores were made from.
    pub index: u16,
    /// Log-likelihood, higher is likelier.
    pub score: f32,
    word: Text,
}

impl Zeroize for Candidate {
    fn zeroize(&mut self) {
        self.index.zeroize();
        self.score.zeroize();
        self.word.zeroize();
    }
}

impl Drop for Candidate {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Candidate {}

impl Candidate {
    /// The word, from the list the scores were made from.
    pub fn word(&self) -> &str {
        self.word.as_str()
    }
}

impl fmt::Debug for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Candidate(..)")
    }
}

impl Scores {
    /// The scores, in the order of the word list.
    pub fn values(&self) -> &[f32] {
        &self.data
    }

    /// The `n` likeliest words, likeliest first (equal scores: the lower
    /// index first). Words are those of the list the scores were made from.
    pub fn best(&self, n: usize) -> Zeroizing<Vec<Candidate>> {
        let mut all: Zeroizing<Vec<Candidate>> = Zeroizing::new(
            self.data
                .iter()
                .zip(self.words.iter())
                .enumerate()
                .map(|(i, (&score, word))| Candidate {
                    index: i as u16,
                    score,
                    word: word.duplicate(),
                })
                .collect(),
        );
        all.sort_by(|a, b| b.score.total_cmp(&a.score).then(a.index.cmp(&b.index)));
        all.truncate(n);
        all
    }
}

impl fmt::Debug for Scores {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Scores({})", self.data.len())
    }
}

/// Costs `word_scores_in` over the 2,048 words; their scores given `logits` that `log_softmax` has turned
/// into log-probabilities. `beam` drops states that many below the best
/// (0: exact).
pub fn word_scores(lp: &Logits, beam: f32) -> Result<Scores, Error> {
    word_scores_in(lp, beam, Language::English.words())
}

/// Costs a walk of the decoder over the frames and the list's letters (more
/// for a longer list and a wider beam); the scores of the words of `words`
/// (lowercase a-z, none empty or repeated,
/// at most 65,535), in the order of the list. The scores keep a copy of the
/// list: `Scores::best` names words of it.
pub fn word_scores_in(lp: &Logits, beam: f32, words: &[&str]) -> Result<Scores, Error> {
    let frames = lp.frames;
    if frames == 0 || lp.data.len() != CLASSES * frames {
        return Err(Error::Size);
    }
    let t = Trie::new(words)?;
    let n = t.parent.len();
    let lpv: &[f32] = &lp.data;
    let letter_lp = |node: usize, f: usize| -> f32 {
        let k = usize::from(t.letter.get(node).copied().unwrap_or(0));
        lpv.get(k * frames + f).copied().unwrap_or(NEG)
    };

    let mut l: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut b: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut l2: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut b2: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);

    let mut p = prefix_logprob(lpv, frames, 0);
    let mut best = p;
    for node in 1..n {
        if t.parent.get(node).copied() == Some(0) {
            l[node] = letter_lp(node, 0);
        }
        best = if l[node] > best { l[node] } else { best };
    }
    for f in 1..frames {
        let blank = lpv.get(f).copied().unwrap_or(NEG);
        let pref = prefix_logprob(lpv, frames, f);
        let floor = if beam > 0.0 { best - beam } else { NEG };
        let mut next_best = p + pref;
        l2[0] = NEG;
        b2[0] = NEG;
        for node in 1..n {
            let par = t.parent[node] as usize;
            let letter = t.letter[node];
            let parent_low = if par == 0 {
                p < floor
            } else {
                l[par] < floor && b[par] < floor
            };
            if l[node] < floor && b[node] < floor && parent_low {
                l2[node] = NEG;
                b2[node] = NEG;
                continue;
            }
            let mut v = l[node];
            if par == 0 {
                v = logadd(v, p);
            } else {
                v = logadd(v, b[par]);
                if t.letter[par] != letter {
                    v = logadd(v, l[par]);
                }
            }
            l2[node] = v + letter_lp(node, f);
            b2[node] = logadd(b[node], l[node]) + blank;
            next_best = if l2[node] > next_best {
                l2[node]
            } else {
                next_best
            };
            next_best = if b2[node] > next_best {
                b2[node]
            } else {
                next_best
            };
        }
        best = next_best;
        p += pref;
        core::mem::swap(&mut l, &mut l2);
        core::mem::swap(&mut b, &mut b2);
    }
    let data: Vec<f32> = t
        .word_end
        .iter()
        .map(|&e| logadd(l[e as usize], b[e as usize]))
        .collect();
    Ok(Scores {
        data: Zeroizing::new(data),
        words: words.iter().map(|w| Text::of(w)).collect(),
    })
}
