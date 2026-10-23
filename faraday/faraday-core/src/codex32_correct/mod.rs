//! Codex32 error correction, the tick off by default (`docs/CODEX32.md`
//! §8.4, F94): the tick **Suggest corrections**, the proposal box by box,
//! the re-entry from the paper, and the marks a corrected string and its
//! key carry.
//!
//! [`Correction`] is the whole of it, held by the form a codex32 string
//! is typed on (Add a key's codex32 form, Check a string): the form owns
//! one, fresh (the tick off) whenever it opens, and calls it through the
//! interface below. It holds no key and loads nothing: the decoder
//! (`faraday_bip::codex32::correct`) proposes, a person confirms, and
//! what the form is handed back ([`Taken`]) goes in through
//! [`faraday_bip::codex32::Codex32::parse`] like any string typed.
//!
//! The flow, with the tick on:
//!
//! 1. The form offers a string that does not parse ([`Correction::offer`]):
//!    one decode. A refusal is kept and drawn as its own line, with
//!    **Check the boxes**, which hands the entry back for editing
//!    ([`Outcome::Edit`]).
//! 2. A proposal is never applied. A run of more than eight unreadable
//!    boxes first shows its caution ([`Step::Caution`]); then the pairs,
//!    "Box 17 · q → p", four to a page, every page seen before a way on
//!    is offered ([`Step::Pairs`]).
//! 3. **I mistyped**: the changed boxes are cleared, every other box is
//!    locked, and the person types the changed boxes again from the paper
//!    ([`Step::Retype`]); only an exact match is taken, marked
//!    [`Mark::Corrected`]. **The paper is damaged**: the pairs as repairs
//!    to write on the paper ([`Step::Repairs`]), then the same re-entry,
//!    marked [`Mark::Repaired`]. **Unreadable on the paper**, offered only
//!    where every changed box was marked `?`: a caution
//!    ([`Step::FromChecksum`]), then the proposal as it is, marked
//!    [`Mark::Repaired`].
//!
//! The screen is `screen.rs`: the tick's row, the panel that stands in
//! place of the form's field while a proposal or a refusal is up, and the
//! line beside a fingerprint. Drawing reads this state and works nothing
//! out; the one costly call, the decode, is made by a press.

use faraday_bip::codex32::{
    CORRECTION_BOUND, Codex32, Codex32Ascii, Edit, MAX_EDITS, MAX_STRING_LEN, Proposal, Refusal,
    UNREADABLE, correct,
};
use faraday_crypto::work::Compute;
use faraday_shell_api::Key;

use crate::strings::{EN, fill};

pub mod screen;

/// The pairs on one page of a proposal (§8.4: "four to a page").
pub const PAIRS_PER_PAGE: usize = 4;

/// The longest run of unreadable boxes the regular checksum fills.
const RUN: usize = 13;
/// The longest the long checksum of a 127-character string fills.
const RUN_LONG: usize = 15;

/// The bech32 alphabet: what a box can hold.
const BECH32: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// What a string, or the key it made, carries once it went in by a
/// correction (§8.4). It never goes: a fingerprint matched later beside
/// it does not remove it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mark {
    /// The paper was right, the typing wrong: the changed boxes were
    /// typed again from the paper and matched ("Share entry corrected").
    Corrected,
    /// The paper itself was repaired, or its unreadable boxes filled from
    /// the checksum: nothing read them ("Share repairs unchecked").
    Repaired,
}

impl Mark {
    /// The line beside the key's fingerprint where it loads, and beside
    /// any fingerprint comparison after.
    pub fn fingerprint_line(self) -> &'static str {
        match self {
            Mark::Corrected => EN.codex32_correct_entry_corrected,
            Mark::Repaired => EN.codex32_correct_repairs_unchecked,
        }
    }

    /// The line beside the string among the strings in, where it has one:
    /// a repaired string is "Repaired, not checked".
    pub fn string_line(self) -> Option<&'static str> {
        match self {
            Mark::Corrected => None,
            Mark::Repaired => Some(EN.codex32_correct_repaired_not_checked),
        }
    }
}

/// Where a shown proposal is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// A run of more than eight unreadable boxes: "This assumes every
    /// other box is right", before the pairs.
    Caution,
    /// "Possible correction": the pairs, a page at a time.
    Pairs,
    /// The paper is damaged: the pairs as repairs to write on it.
    Repairs,
    /// The changed boxes typed again from the paper.
    Retype,
    /// Unreadable on the paper: the caution before the proposal is taken
    /// as it is.
    FromChecksum,
}

/// A press on the correction's screens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// The tick, **Suggest corrections**.
    Tick,
    /// **Continue**: past the run's caution, or past the repairs to the
    /// re-entry.
    Continue,
    /// The proposal's previous page.
    Previous,
    /// The proposal's next page.
    Next,
    /// **I mistyped**: the paper is right.
    Mistyped,
    /// **The paper is damaged**.
    Damaged,
    /// **Unreadable on the paper**.
    Unreadable,
    /// **Take the correction**, after the caution of Unreadable.
    Take,
    /// **Check the boxes**: the entry back, as typed, for editing.
    CheckBoxes,
}

/// What the form does after a press or a key.
pub enum Outcome {
    /// Nothing: the correction's screen stays, or nothing changed.
    Stay,
    /// The correction's screen closed with the entry as it was typed,
    /// kept for editing.
    Edit,
    /// A string to take in place of the one typed, and its mark: the
    /// form reads it with `Codex32::parse` as it reads a string typed,
    /// and, once it is in, calls [`Correction::note`].
    Take(Taken),
}

/// A correction the person confirmed: the string, lowercase, and its
/// mark. Wiped on drop (its string is).
pub struct Taken {
    text: Codex32Ascii,
    mark: Mark,
}

impl Taken {
    /// The string.
    pub fn text(&self) -> &str {
        self.text.as_str()
    }

    /// How it was confirmed.
    pub fn mark(&self) -> Mark {
        self.mark
    }
}

/// What [`Correction::offer`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Offered {
    /// The string holds as typed and has no unreadable box: the form
    /// takes it as it takes any string.
    Holds,
    /// A proposal or a refusal is shown in place of the field.
    Shown,
}

/// A box of the string as the re-entry shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxShown {
    /// A box not changed: locked, its character as typed.
    Locked(u8),
    /// A changed box not typed again yet; the next one typed into when
    /// `next`.
    Open {
        /// Typing goes here now.
        next: bool,
    },
    /// A changed box typed again.
    Typed(u8),
}

/// The changed boxes as typed again, wiped on drop.
#[derive(Default)]
struct Retyped {
    chars: [u8; MAX_EDITS],
    n: usize,
}

impl Retyped {
    fn clear(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.chars);
        self.n = 0;
    }
}

impl Drop for Retyped {
    fn drop(&mut self) {
        self.clear();
    }
}

/// A proposal on screen. Its string, as text, is kept with it once (the
/// re-entry shows the locked boxes from it); both are wiped on drop.
struct Shown {
    proposal: Proposal,
    text: Codex32Ascii,
    step: Step,
    /// The page of pairs on show, from 0.
    page: usize,
    /// The furthest page seen, from 0.
    seen: usize,
    /// The mark the re-entry ends in.
    mark: Mark,
    retyped: Retyped,
    /// The last re-entry did not match the proposal.
    differs: bool,
}

impl Shown {
    fn pages(&self) -> usize {
        self.proposal.edits().len().div_ceil(PAIRS_PER_PAGE).max(1)
    }
}

/// What is on screen in place of the form's field.
enum Stage {
    /// Nothing: the form as it is.
    Closed,
    /// A refusal, with the string's length (the run bound depends on it).
    Refused(Refusal, usize),
    /// A proposal.
    Shown(Box<Shown>),
}

/// One string that went in by a correction, by what names it without its
/// data: identifier and share index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Noted {
    identifier: [u8; 4],
    index: u8,
    mark: Mark,
}

/// Error correction for one codex32 form (§8.4). The form holds it; it
/// starts with the tick off.
pub struct Correction {
    on: bool,
    stage: Stage,
    noted: Vec<Noted>,
}

impl Default for Correction {
    fn default() -> Self {
        Correction::new()
    }
}

impl Correction {
    /// The tick off, nothing shown, nothing noted: as every form opens.
    pub fn new() -> Correction {
        Correction {
            on: false,
            stage: Stage::Closed,
            noted: Vec::new(),
        }
    }

    // ---- What the form reads ----

    /// The tick: corrections are suggested.
    pub fn on(&self) -> bool {
        self.on
    }

    /// Whether a box may be typed as unreadable, `?`: with the tick on,
    /// and not while changed boxes are typed again from the paper. With
    /// the tick off `?` is not on the keyboard.
    pub fn takes_unreadable(&self) -> bool {
        self.on && self.step() != Some(Step::Retype)
    }

    /// Whether the correction's screen stands in place of the form's
    /// field: a proposal or a refusal is up, and keys go to
    /// [`Correction::key`].
    pub fn is_open(&self) -> bool {
        !matches!(self.stage, Stage::Closed)
    }

    /// The refusal on screen, if one is.
    pub fn refusal(&self) -> Option<Refusal> {
        match self.stage {
            Stage::Refused(r, _) => Some(r),
            _ => None,
        }
    }

    /// The refusal's line, if one is up: each refusal its own.
    pub fn refusal_line(&self) -> Option<String> {
        let Stage::Refused(r, len) = self.stage else {
            return None;
        };
        Some(match r {
            Refusal::NotFound => EN.codex32_correct_not_found.to_string(),
            Refusal::TooManyUnreadable => {
                let run = if len == MAX_STRING_LEN { RUN_LONG } else { RUN };
                fill(
                    EN.codex32_correct_too_many_unreadable,
                    &[&CORRECTION_BOUND.to_string(), &run.to_string()],
                )
            }
            Refusal::HeaderFixed => EN.codex32_correct_header_fixed.to_string(),
            Refusal::Malformed(e) => fill(EN.codex32_correct_malformed, &[&e.to_string()]),
            Refusal::NotAString(e) => fill(EN.codex32_correct_not_a_string, &[&e.to_string()]),
        })
    }

    fn shown(&self) -> Option<&Shown> {
        match &self.stage {
            Stage::Shown(s) => Some(s),
            _ => None,
        }
    }

    fn shown_mut(&mut self) -> Option<&mut Shown> {
        match &mut self.stage {
            Stage::Shown(s) => Some(s),
            _ => None,
        }
    }

    /// Where the proposal on screen is, if one is.
    pub fn step(&self) -> Option<Step> {
        self.shown().map(|s| s.step)
    }

    /// Every box the proposal changes, first box first.
    pub fn edits(&self) -> &[Edit] {
        self.shown().map_or(&[], |s| s.proposal.edits())
    }

    /// The page of pairs on show and how many there are, from 0.
    pub fn page(&self) -> Option<(usize, usize)> {
        self.shown().map(|s| (s.page, s.pages()))
    }

    /// The pairs on the page on show.
    pub fn page_edits(&self) -> &[Edit] {
        let Some(s) = self.shown() else {
            return &[];
        };
        let all = s.proposal.edits();
        let from = (s.page * PAIRS_PER_PAGE).min(all.len());
        &all[from..(from + PAIRS_PER_PAGE).min(all.len())]
    }

    /// Whether every page of the proposal has been on screen: the ways on
    /// are offered only then.
    pub fn all_seen(&self) -> bool {
        self.shown().is_some_and(|s| s.seen + 1 >= s.pages())
    }

    /// Whether the proposal fills a run of more than eight unreadable
    /// boxes, which assumes every other box is right.
    pub fn assumes_the_rest(&self) -> bool {
        self.shown().is_some_and(|s| s.proposal.assumes_the_rest())
    }

    /// Whether **Unreadable on the paper** is offered: every box the
    /// proposal changes was marked `?`.
    pub fn offers_unreadable(&self) -> bool {
        self.shown()
            .is_some_and(|s| s.proposal.substitutions() == 0 && !s.proposal.edits().is_empty())
    }

    /// The re-entry's boxes typed and how many there are to type.
    pub fn retyped(&self) -> (usize, usize) {
        self.shown()
            .map_or((0, 0), |s| (s.retyped.n, s.proposal.edits().len()))
    }

    /// Whether the last re-entry differed from the proposal.
    pub fn differs(&self) -> bool {
        self.shown().is_some_and(|s| s.differs)
    }

    /// The string's length in boxes, while a proposal is up.
    pub fn len(&self) -> usize {
        self.shown().map_or(0, |s| s.text.as_str().len())
    }

    /// Whether no proposal is up.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Box `n` (from 1) as the re-entry shows it: locked with its
    /// character, open, or typed again. `None` past the string's end or
    /// with no proposal up. A lookup.
    pub fn box_at(&self, n: usize) -> Option<BoxShown> {
        let s = self.shown()?;
        let ch = *s.text.as_str().as_bytes().get(n.checked_sub(1)?)?;
        let edits = s.proposal.edits();
        match edits.iter().position(|e| e.position() == n) {
            None => Some(BoxShown::Locked(ch)),
            Some(i) if i < s.retyped.n => Some(BoxShown::Typed(s.retyped.chars[i])),
            Some(i) => Some(BoxShown::Open {
                next: i == s.retyped.n,
            }),
        }
    }

    /// The mark of a string noted in, by its identifier and share index.
    pub fn mark_of(&self, string: &Codex32) -> Option<Mark> {
        let (identifier, index) = (string.identifier(), string.share_index());
        self.noted
            .iter()
            .find(|n| n.identifier == identifier && n.index == index)
            .map(|n| n.mark)
    }

    /// The mark the key these strings make carries: the strongest of the
    /// strings noted, `None` where none went in by a correction.
    pub fn key_mark(&self) -> Option<Mark> {
        self.noted.iter().map(|n| n.mark).max()
    }

    // ---- What the form does ----

    /// The tick pressed. Turned off, whatever is shown closes and the
    /// entry stays as typed.
    pub fn toggle(&mut self) {
        self.on = !self.on;
        if !self.on {
            self.stage = Stage::Closed;
        }
    }

    /// Costs a decode of one string (`faraday_bip::codex32::correct`):
    /// syndromes, Berlekamp–Massey and a search of every box, or a solve
    /// of at most 15 unknowns. With the tick on, the form hands over the
    /// string typed, `?` marking boxes that cannot be read, when it does
    /// not parse as typed; `held` is a string of the set already in,
    /// whose threshold and identifier boxes are fixed.
    ///
    /// A string that holds with no box unreadable is [`Offered::Holds`]
    /// and shows nothing. Otherwise the proposal or the refusal is shown.
    /// With the tick off nothing is decoded and the string is
    /// [`Offered::Holds`]: the form refuses it as it always has.
    pub fn offer(&mut self, c: &Compute, text: &str, held: Option<&Codex32>) -> Offered {
        if !self.on {
            return Offered::Holds;
        }
        self.stage = match correct(c, text, held) {
            Ok(p) if p.edits().is_empty() => return Offered::Holds,
            Ok(proposal) => {
                let text = proposal.string().encode();
                let step = if proposal.assumes_the_rest() {
                    Step::Caution
                } else {
                    Step::Pairs
                };
                Stage::Shown(Box::new(Shown {
                    proposal,
                    text,
                    step,
                    page: 0,
                    seen: 0,
                    mark: Mark::Corrected,
                    retyped: Retyped::default(),
                    differs: false,
                }))
            }
            Err(r) => Stage::Refused(r, text.len()),
        };
        Offered::Shown
    }

    /// A press on the correction's screens. A lookup: nothing is decoded
    /// again.
    pub fn press(&mut self, p: Press) -> Outcome {
        if p == Press::Tick {
            self.toggle();
            return Outcome::Stay;
        }
        if p == Press::CheckBoxes {
            return if self.is_open() {
                self.stage = Stage::Closed;
                Outcome::Edit
            } else {
                Outcome::Stay
            };
        }
        let all_seen = self.all_seen();
        let unreadable = self.offers_unreadable();
        let Some(s) = self.shown_mut() else {
            return Outcome::Stay;
        };
        match (s.step, p) {
            (Step::Caution, Press::Continue) => s.step = Step::Pairs,
            (Step::Repairs, Press::Continue) => s.step = Step::Retype,
            (Step::Pairs | Step::Repairs, Press::Next) if s.page + 1 < s.pages() => {
                s.page += 1;
                s.seen = s.seen.max(s.page);
            }
            (Step::Pairs | Step::Repairs, Press::Previous) => s.page = s.page.saturating_sub(1),
            (Step::Pairs, Press::Mistyped) if all_seen => {
                s.mark = Mark::Corrected;
                s.step = Step::Retype;
            }
            (Step::Pairs, Press::Damaged) if all_seen => {
                s.mark = Mark::Repaired;
                s.step = Step::Repairs;
                s.page = 0;
            }
            (Step::Pairs, Press::Unreadable) if all_seen && unreadable => {
                s.step = Step::FromChecksum;
            }
            (Step::FromChecksum, Press::Take) => return self.take(Mark::Repaired),
            _ => {}
        }
        Outcome::Stay
    }

    /// A key while the correction's screen is up. In the re-entry: a
    /// bech32 character fills the next changed box, Backspace clears the
    /// last, and once every changed box is typed it is compared with the
    /// proposal, taken only where it matches exactly. Escape is **Check
    /// the boxes**; Enter goes on where one way does.
    pub fn key(&mut self, k: Key) -> Outcome {
        if k == Key::Escape {
            return self.press(Press::CheckBoxes);
        }
        let Some(s) = self.shown_mut() else {
            return Outcome::Stay;
        };
        match (s.step, k) {
            (Step::Retype, Key::Char(ch)) => {
                let ch = ch.to_ascii_lowercase();
                let total = s.proposal.edits().len();
                if !ch.is_ascii() || !BECH32.contains(&(ch as u8)) || s.retyped.n >= total {
                    return Outcome::Stay;
                }
                s.retyped.chars[s.retyped.n] = ch as u8;
                s.retyped.n += 1;
                s.differs = false;
                if s.retyped.n == total {
                    let matches = s
                        .proposal
                        .edits()
                        .iter()
                        .zip(&s.retyped.chars)
                        .all(|(e, &t)| e.now() == t);
                    if matches {
                        let mark = s.mark;
                        return self.take(mark);
                    }
                    s.retyped.clear();
                    s.differs = true;
                }
                Outcome::Stay
            }
            (Step::Retype, Key::Backspace) => {
                if s.retyped.n > 0 {
                    s.retyped.n -= 1;
                    s.retyped.chars[s.retyped.n] = 0;
                }
                Outcome::Stay
            }
            (Step::Caution | Step::Repairs, Key::Enter) => self.press(Press::Continue),
            (Step::Pairs, Key::Enter) => self.press(Press::Next),
            (Step::FromChecksum, Key::Enter) => self.press(Press::Take),
            _ => Outcome::Stay,
        }
    }

    /// The proposal taken with `mark`: the screen closes and the string
    /// is the form's to read.
    fn take(&mut self, mark: Mark) -> Outcome {
        let Stage::Shown(s) = std::mem::replace(&mut self.stage, Stage::Closed) else {
            return Outcome::Stay;
        };
        Outcome::Take(Taken {
            text: s.proposal.string().encode(),
            mark,
        })
    }

    /// The form took `string` in with `mark`: it carries the mark, and so
    /// does the key the strings make ([`Correction::key_mark`]). A mark
    /// never goes, and a stronger one replaces a weaker.
    pub fn note(&mut self, string: &Codex32, mark: Mark) {
        let (identifier, index) = (string.identifier(), string.share_index());
        match self
            .noted
            .iter_mut()
            .find(|n| n.identifier == identifier && n.index == index)
        {
            Some(n) => n.mark = n.mark.max(mark),
            None => self.noted.push(Noted {
                identifier,
                index,
                mark,
            }),
        }
    }
}

/// The marks of keys loaded from corrected strings, by fingerprint: what
/// a session keeps so that the line stays beside the key's fingerprint
/// wherever it is shown or compared (§8.4). A mark is never taken away;
/// a stronger one replaces a weaker.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct KeyMarks(Vec<([u8; 4], Mark)>);

impl KeyMarks {
    /// Key `fp` loaded from strings carrying `mark`.
    pub fn note(&mut self, fp: [u8; 4], mark: Mark) {
        match self.0.iter_mut().find(|(f, _)| *f == fp) {
            Some((_, m)) => *m = (*m).max(mark),
            None => self.0.push((fp, mark)),
        }
    }

    /// The mark key `fp` carries, if any. A lookup.
    pub fn of(&self, fp: [u8; 4]) -> Option<Mark> {
        self.0.iter().find(|(f, _)| *f == fp).map(|&(_, m)| m)
    }
}

/// Whether `ch` may be typed into a codex32 string with the tick as it
/// is: `?` only while [`Correction::takes_unreadable`].
pub fn typable(c: &Correction, ch: char) -> bool {
    ch == char::from(UNREADABLE) && c.takes_unreadable()
}
