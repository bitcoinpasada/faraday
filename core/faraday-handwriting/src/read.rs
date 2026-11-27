//! Reading one handwritten word from a grey crop: clean-up, the network, the
//! scores over a word list, and the decision whether the answer is clear.
//!
//! No checksum is used: whether the words of a seed fit together is not asked
//! here.

use core::fmt;

use faraday_bip::bip39::Language;
use zeroize::Zeroizing;

use crate::clean::{locate, prepare, unread};
use crate::decode::{Candidate, check_list, log_softmax, word_scores_in};
use crate::word::{CLASSES, Logits, word_logits};
use crate::{BEAM, Error};

/// How far the best word must be ahead of the second, in log units.
///
/// Provisional: not yet measured on real handwriting.
pub const CLEAR_MARGIN: f32 = 4.0;
/// How far below the network's own free reading the best word's score may be,
/// in log units. The free reading takes the likeliest class at each frame
/// with no word list; a word that is not in the list is pulled to a listed
/// one that scores far below it.
///
/// Provisional: not yet measured on real handwriting.
pub const CLEAR_FREE_GAP: f32 = 3.0;
/// The most faint ink left out of the word, in heights of the text, for a
/// clear answer.
///
/// Provisional: not yet measured on real handwriting.
pub const CLEAR_UNREAD_INK: f32 = 0.3;
/// Candidates an unclear answer lists.
pub const UNCLEAR_CANDIDATES: usize = 5;

/// What a crop read as.
pub enum Reading {
    /// One word, clearly ahead of the others and matching what the network
    /// read.
    Clear(Candidate),
    /// No word stands out: the best candidates, likeliest first. Empty if the
    /// crop holds no word.
    Unclear(Zeroizing<Vec<Candidate>>),
}

impl fmt::Debug for Reading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Reading::Clear(_) => "Reading::Clear(..)",
            Reading::Unclear(_) => "Reading::Unclear(..)",
        })
    }
}

/// The 2,048 words of the English BIP39 list, the list the networks were
/// checked against.
pub fn bip39_words() -> &'static [&'static str] {
    Language::English.words()
}

/// The 1,024 words of Faraday's SLIP-39 list. The list is accepted by
/// `word_scores_in` and `read_crop`; nothing is claimed about how well the
/// networks read these words.
pub fn slip39_words() -> Vec<&'static str> {
    (0..faraday_bip::slip39::WORDLIST_LEN as u16)
        .map(faraday_bip::slip39::word)
        .collect()
}

/// The log-likelihood of the network's free reading: the likeliest class at
/// each frame, with no word list and no collapsing of repeats.
fn free_reading(lp: &Logits) -> f32 {
    let frames = lp.frames();
    let v = lp.values();
    let mut total = 0.0f32;
    for t in 0..frames {
        let mut m = f32::NEG_INFINITY;
        for k in 0..CLASSES {
            if let Some(&x) = v.get(k * frames + t) {
                m = if x > m { x } else { m };
            }
        }
        total += m;
    }
    total
}

/// First digit class.
const FIRST_DIGIT: usize = 27;
/// A score of a path that cannot be taken.
const NEG: f32 = -1e30;
/// Anything at or below this is a path that cannot be taken.
const UNREACHABLE: f32 = -1e29;

/// The log-likelihood of the one likeliest alignment of `classes` (blank is
/// 0, letters 1 to 26, digits 27 to 36) to the frames: the Viterbi path of
/// the same states as the word scores, with a maximum where they add. Before
/// the first class stand blanks, and with `digits_before` also digits (the
/// line number before a word). A repeated class needs a blank between. The
/// path ends on the last class or a blank after it. `NEG` if the frames are
/// too few for it.
///
/// Every frame of the path takes at most the frame's likeliest class, so the
/// result is never above `free_reading`.
pub(crate) fn best_path(lp: &Logits, classes: &[usize], digits_before: bool) -> f32 {
    let frames = lp.frames();
    let v = lp.values();
    let at = |k: usize, t: usize| {
        k.checked_mul(frames)
            .and_then(|o| o.checked_add(t))
            .and_then(|i| v.get(i))
            .copied()
            .unwrap_or(NEG)
    };
    let before = |t: usize| {
        let mut m = at(0, t);
        if digits_before {
            for k in FIRST_DIGIT..CLASSES {
                m = m.max(at(k, t));
            }
        }
        m
    };
    let n = classes.len();
    if n == 0 || frames == 0 {
        return NEG;
    }
    let class = |i: usize| classes.get(i).copied().unwrap_or(0);
    let mut l: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut b: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut l2: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut b2: Zeroizing<Vec<f32>> = Zeroizing::new(vec![NEG; n]);
    let mut p = before(0);
    if let Some(first) = l.first_mut() {
        *first = at(class(0), 0);
    }
    for t in 1..frames {
        let blank = at(0, t);
        for i in 0..n {
            let here = l.get(i).copied().unwrap_or(NEG);
            let mut enter = here;
            if let Some(j) = i.checked_sub(1) {
                enter = enter.max(b.get(j).copied().unwrap_or(NEG));
                if class(j) != class(i) {
                    enter = enter.max(l.get(j).copied().unwrap_or(NEG));
                }
            } else {
                enter = enter.max(p);
            }
            let stay = b.get(i).copied().unwrap_or(NEG).max(here);
            if let Some(x) = l2.get_mut(i) {
                *x = (enter + at(class(i), t)).max(NEG);
            }
            if let Some(x) = b2.get_mut(i) {
                *x = (stay + blank).max(NEG);
            }
        }
        p = (p + before(t)).max(NEG);
        core::mem::swap(&mut l, &mut l2);
        core::mem::swap(&mut b, &mut b2);
    }
    let last = n.saturating_sub(1);
    l.get(last)
        .copied()
        .unwrap_or(NEG)
        .max(b.get(last).copied().unwrap_or(NEG))
}

/// How far below the network's free reading the likeliest alignment of
/// `classes` is, 0 or more; `None` if no alignment fits the frames.
pub(crate) fn path_gap(
    lp: &Logits,
    free: f32,
    classes: &[usize],
    digits_before: bool,
) -> Option<f32> {
    let path = best_path(lp, classes, digits_before);
    // The two sums add in another order: a rounding error under 0 is 0.
    (path > UNREACHABLE).then(|| (free - path).max(0.0))
}

/// The classes of a lowercase word, `None` if a byte is not a-z.
pub(crate) fn letters_of(word: &str) -> Option<Vec<usize>> {
    word.bytes()
        .map(|c| {
            c.is_ascii_lowercase()
                .then(|| usize::from(c.wrapping_sub(b'a')).saturating_add(1))
        })
        .collect()
}

/// A crop's word network output with what the clear rule needs besides the
/// word list: read once, judged against any number of lists.
pub(crate) struct Logged {
    pub(crate) lp: Logits,
    pub(crate) free: f32,
    unread: Option<f32>,
}

/// Cleans a crop and runs the word network on it once. `None` if the crop
/// holds no ink worth reading.
pub(crate) fn log_crop(width: usize, height: usize, gray: &[u8]) -> Result<Option<Logged>, Error> {
    let Some(prepared) = prepare(width, height, gray)? else {
        return Ok(None);
    };
    let Some(cleaned) = locate(&prepared) else {
        return Ok(None);
    };
    let unread = unread(&prepared);
    drop(prepared);
    let mut lp = word_logits(cleaned.ink(), cleaned.width())?;
    drop(cleaned);
    log_softmax(&mut lp);
    let free = free_reading(&lp);
    Ok(Some(Logged { lp, free, unread }))
}

impl Logged {
    /// The clear or unclear answer against `words`, by the rule of
    /// `read_crop`.
    pub(crate) fn judge(&self, words: &[&str]) -> Result<Reading, Error> {
        self.judge_gap(words, false).map(|(r, _)| r)
    }

    /// Whether the faint ink left out of the word is known and small enough
    /// for a clear answer.
    pub(crate) fn little_unread(&self) -> bool {
        self.unread.is_some_and(|u| u <= CLEAR_UNREAD_INK)
    }

    /// `judge`, and how far the best word's own likeliest alignment is below
    /// the network's free reading (`path_gap`: 0 or more, `None` if the list
    /// gave no word or no alignment fits the frames). With `never_clear` the
    /// answer is never `Clear` and lists the candidates an unclear answer
    /// lists.
    pub(crate) fn judge_gap(
        &self,
        words: &[&str],
        never_clear: bool,
    ) -> Result<(Reading, Option<f32>), Error> {
        let scores = word_scores_in(&self.lp, BEAM, words)?;
        let mut top = scores.best(UNCLEAR_CANDIDATES.max(2));
        let gap = top
            .first()
            .and_then(|b| letters_of(b.word()))
            .and_then(|c| path_gap(&self.lp, self.free, &c, true));
        // One word alone is never clear: nothing stands against it.
        let clear = match (top.first(), top.get(1)) {
            (Some(best), Some(second)) => {
                !never_clear
                    && best.score - second.score >= CLEAR_MARGIN
                    && self.little_unread()
                    && gap.is_some_and(|g| g <= CLEAR_FREE_GAP)
            }
            _ => false,
        };
        if clear {
            return Ok((Reading::Clear(top.swap_remove(0)), gap));
        }
        top.truncate(UNCLEAR_CANDIDATES);
        // A word the beam dropped has no score: it is not a candidate.
        top.retain(|c| c.score > UNREACHABLE);
        Ok((Reading::Unclear(top), gap))
    }
}

/// Costs `clean_crop` and `unread_ink`, one run of the word network and one
/// decode over `words` (`word_scores_in`), and the likeliest alignment of the
/// best word.
///
/// Reads the word in a grey crop (`width` by `height`, row-major, 0 black to
/// 255 white) against `words` (lowercase a-z, none empty or repeated).
///
/// The answer is `Clear` only when the best word is ahead of the second by
/// `CLEAR_MARGIN` (a list of one word is never clear), the best word's own
/// likeliest alignment is within `CLEAR_FREE_GAP` of the network's free
/// reading, and the faint ink left out of the word is known and under
/// `CLEAR_UNREAD_INK` heights of the text. Otherwise it is `Unclear`, with
/// the `UNCLEAR_CANDIDATES` likeliest words.
pub fn read_crop(
    width: usize,
    height: usize,
    gray: &[u8],
    words: &[&str],
) -> Result<Reading, Error> {
    // A bad list is refused even for a crop without ink.
    check_list(words)?;
    match log_crop(width, height, gray)? {
        Some(logged) => logged.judge(words),
        None => Ok(Reading::Unclear(Zeroizing::new(Vec::new()))),
    }
}
