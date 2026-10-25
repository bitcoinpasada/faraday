//! The step-through of the "Demo only" card (`docs/CODEX32.md` §5.5,
//! owner 2026-10-24): on Faraday Desktop the computer's five dice roll a
//! character's values first, as the roller rolls them (this computer's
//! randomness, `Faraday::fresh`), and the entry then takes them one step
//! at a time: each die's first roll and its marker's pad, each second
//! roll with its comparison and bit, the five bits read down the tree
//! into their box, and the checksum worksheet's cells that character
//! fills, a sum or a table row at a time. Previous shows the state before
//! and never rolls again; Play runs the steps at a speed, as far as the
//! extent chosen (a pass, a character, a share, the end).
//!
//! **Kept in step.** A step is a place of the entry: the faces of its
//! character entered so far (the last face of a character held, whole,
//! out of its box until the character's own step), or the character in
//! its box. Every move of the walk goes through [`Codex32Make::walk_go`],
//! which brings the entry to the step's place a face or a character at a
//! time and marks each face as this computer's. Any other change of the
//! entry (a face of the person's own dice, Backspace, a change of dice,
//! Use a sample) forgets the walk ([`Codex32Make::act`]).
//!
//! **Built once.** A character's steps are worked out when its values
//! are rolled: the dice's places from its faces, and which cells of the
//! checksum worksheet its box fills, from the worksheet's shape. The
//! steps hold places, not values; drawing reads the values from the
//! faces kept here and from the worksheet the work set keeps
//! (`walk_screen.rs`), as a cell's working does (§7.4).
//!
//! **One model.** The person's own entry has the same steps, built from
//! their own faces by [`Codex32Make::char_steps`] in `tracker.rs`
//! (§5.7); the sheets mark the step [`Codex32Make::shown_step`] gives,
//! the walk's while it shows one, else the person's. Past the last
//! initial share's checksum both go on into the same tour of the shares
//! made and S (`tour.rs`).

use faraday_crypto::work::Compute;
use faraday_draw::tokens as t;
use faraday_entropy::booklet::{BOOKLET_DICE, BookletChar};
use faraday_keys::Reveal;
use zeroize::{Zeroize, Zeroizing};

use super::worksheet_screen::{self, Paper};
use super::{Card, Cell, Codex32Make, DieRow, Way};
use crate::fresh::roller::five::{Draw, Reach};

/// A press on the step-through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkAct {
    /// The step before: the entry as it was, nothing rolled.
    Previous,
    /// The next step, a character's values rolled first where none are
    /// left to show.
    Next,
    /// The steps run at the speed chosen, as far as the extent.
    Play,
    /// Play stops where it is.
    Pause,
    /// How far Play runs.
    Reach(Reach),
    /// How fast Play runs, by its place in [`SPEEDS`].
    Speed(u8),
}

/// How fast Play runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Speed {
    /// A step a second.
    Slow,
    /// The default.
    #[default]
    Normal,
    /// Quick.
    Fast,
    /// A step every frame.
    Fastest,
}

/// The speeds, in the order they are offered.
pub const SPEEDS: [Speed; 4] = [Speed::Slow, Speed::Normal, Speed::Fast, Speed::Fastest];

impl Speed {
    /// The time between two steps, ms.
    pub fn ms(self) -> u64 {
        match self {
            Speed::Slow => t::CODEX32_WALK_SLOW_MS,
            Speed::Normal => t::CODEX32_WALK_NORMAL_MS,
            Speed::Fast => t::CODEX32_WALK_FAST_MS,
            Speed::Fastest => 0,
        }
    }
}

/// One step of a character, by place: what it reads is drawn from the
/// character's faces and the worksheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Face `k` (of the character's faces) is die `die`'s first roll.
    First {
        /// The face, by its place in the character's faces.
        k: u8,
        /// The die, from 0.
        die: u8,
        /// The pass ends with it (Play's One pass stops here).
        pass_end: bool,
    },
    /// Face `k` is die `die`'s second roll; its first is face `first`.
    Second {
        /// The face.
        k: u8,
        /// The die's first roll, by its place.
        first: u8,
        /// The die, from 0.
        die: u8,
        /// The pass ends with it.
        pass_end: bool,
    },
    /// The five bits read down the tree, the character into its box.
    Char,
    /// Cell `i` of sum row `j` of the checksum worksheet.
    Sum {
        /// The row.
        j: u8,
        /// The cell.
        i: u8,
    },
    /// The checksum table's row selected by sum row `j`'s leading cells.
    Table {
        /// The row.
        j: u8,
    },
    /// Checksum box `b`, worked from below.
    Below {
        /// The box.
        b: u8,
    },
}

/// One character the walk rolled, and its steps.
pub struct WalkChar {
    /// Characters entered before it, of every share.
    pub(super) g: usize,
    /// Its initial share.
    pub share: usize,
    /// Its place among the share's data characters, from 0.
    pub n: usize,
    /// Every face of it, in order: the person's first, where they had
    /// begun it, then the computer's.
    faces: Zeroizing<Vec<u8>>,
    /// How many faces were the person's.
    k0: usize,
    /// Its symbol.
    value: u8,
    /// Its whole sheet, for its own step.
    dice: BookletChar,
    /// Its pairs, struck ones included.
    rows: Zeroizing<Vec<DieRow>>,
    /// Its steps, in order.
    steps: Vec<Step>,
}

impl Drop for WalkChar {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

impl WalkChar {
    /// Its steps.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// Face `k`, 1 to the dice's faces.
    pub fn face(&self, k: u8) -> u8 {
        self.faces.get(usize::from(k)).copied().unwrap_or(0)
    }

    /// Its symbol.
    pub fn value(&self) -> u8 {
        self.value
    }

    /// Whether it is whole: its own step and its cells are among its
    /// steps. The person's character in progress is not.
    pub fn whole(&self) -> bool {
        self.steps.contains(&Step::Char)
    }

    /// Whether it was typed: no faces.
    pub fn typed(&self) -> bool {
        self.faces.is_empty()
    }

    /// The steps that enter a face.
    pub(super) fn face_steps(&self) -> usize {
        self.faces.len() - self.k0
    }
}

/// The step-through's state: the characters it rolled, contiguous, and
/// the step shown.
#[derive(Default)]
pub struct Walk {
    pub(super) chars: Vec<WalkChar>,
    /// The step shown: (character, steps of it shown); 0 steps only for
    /// the first character, before any is shown.
    pub(super) at: (usize, usize),
    playing: bool,
    /// When Play takes its next step, ms.
    next_ms: u64,
    reach: Reach,
    speed: Speed,
    /// Next found no answer of this computer's randomness yet: the next
    /// tick tries again.
    wanted: bool,
}

impl Walk {
    /// Forgets every character and stops Play: the entry changed by
    /// another way.
    pub(super) fn clear(&mut self) {
        self.chars.clear();
        self.at = (0, 0);
        self.playing = false;
        self.wanted = false;
    }

    /// How far Play runs.
    pub fn reach(&self) -> Reach {
        self.reach
    }

    /// How fast Play runs.
    pub fn speed(&self) -> Speed {
        self.speed
    }

    /// Whether Play runs.
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// Whether the walk wants a tick: Play, or a Next waiting for
    /// randomness.
    pub fn active(&self) -> bool {
        self.playing || self.wanted
    }

    /// The step shown, with its character and its number from 1.
    pub fn shown(&self) -> Option<(&WalkChar, usize, Step)> {
        let (slot, s) = self.at;
        let ch = self.chars.get(slot)?;
        let step = *ch.steps.get(s.checked_sub(1)?)?;
        Some((ch, s, step))
    }
}

/// Where the entry stands: characters entered in all, and faces into the
/// one in progress.
type Pos = (usize, usize);

impl Codex32Make {
    /// The step-through.
    pub fn walk(&self) -> &Walk {
        &self.walk
    }

    /// Faces entered for the character in progress, struck pairs
    /// included.
    pub fn seq_len(&self) -> usize {
        self.seq.len()
    }

    /// Whether the step-through takes a press: the entry open on dice.
    pub fn walk_open(&self) -> bool {
        self.open == Some(Card::Enter) && self.way == Way::Dice && self.work.is_some()
    }

    /// Whether Previous has a step to go back to.
    pub fn walk_can_back(&self) -> bool {
        let (slot, s) = self.walk.at;
        self.walk.chars.get(slot).is_some() && s >= 1
    }

    /// Whether Next has a step to show, or a character to roll, or the
    /// tour of the shares made to open or step through.
    pub fn walk_can_next(&self) -> bool {
        let tour = self.tour_shown().is_some() || self.tour_ready();
        self.walk_open() && (self.walk_has_more() || self.front().is_some() || tour)
    }

    /// Whether the walk has a step of its own after the one shown.
    pub fn walk_has_more(&self) -> bool {
        let (slot, s) = self.walk.at;
        self.walk
            .chars
            .get(slot)
            .is_some_and(|c| s < c.steps.len() || slot + 1 < self.walk.chars.len())
    }

    /// Where the entry stands now.
    pub(super) fn entry_pos(&self) -> Pos {
        (self.entered_all(), self.seq.len())
    }

    /// Forgets the walk where the entry changed by another way than its
    /// own: `before` is where the entry stood before the press.
    pub(super) fn walk_check(&mut self, before: Pos) {
        if self.entry_pos() != before {
            self.walk.clear();
        }
    }

    /// The place of the entry step `s` of character `slot` stands for.
    fn pos_of(&self, slot: usize, s: usize) -> Option<Pos> {
        let ch = self.walk.chars.get(slot)?;
        Some(if s <= ch.face_steps() {
            (ch.g, ch.k0 + s)
        } else {
            (ch.g + 1, 0)
        })
    }

    /// Whether the entry stands where the walk's step says.
    fn walk_in_step(&self) -> bool {
        let (slot, s) = self.walk.at;
        self.walk.chars.is_empty() || self.pos_of(slot, s) == Some(self.entry_pos())
    }

    /// Costs what the walk's presses cost: a worksheet column for a
    /// character put in its box, a fresh answer (a hash, through `draw`)
    /// where Next rolls a character. A press on the step-through at
    /// `now_ms`. Returns whether the entry moved.
    pub(crate) fn walk_act(
        &mut self,
        c: &Compute,
        act: WalkAct,
        now_ms: u64,
        draw: &mut Draw,
    ) -> bool {
        if !self.walk_open() {
            return false;
        }
        match act {
            // The tour of the shares made is the same model's steps past
            // the last checksum (`tour.rs`).
            WalkAct::Previous | WalkAct::Next if self.tour_shown().is_some() => {
                self.walk.playing = false;
                let d = if act == WalkAct::Next { 1 } else { -1 };
                self.tour_step(c, d, true)
            }
            WalkAct::Previous => {
                self.walk.playing = false;
                self.walk_back(c)
            }
            WalkAct::Next => {
                self.walk.playing = false;
                self.walk_next(c, draw)
            }
            WalkAct::Play => {
                if self.walk_can_next() {
                    self.walk.playing = true;
                    self.walk.next_ms = now_ms;
                }
                false
            }
            WalkAct::Pause => {
                self.walk.playing = false;
                false
            }
            WalkAct::Reach(r) => {
                self.walk.reach = r;
                false
            }
            WalkAct::Speed(i) => {
                if let Some(&s) = SPEEDS.get(usize::from(i)) {
                    self.walk.speed = s;
                }
                false
            }
        }
    }

    /// Costs what a step costs (see [`Self::walk_act`]). Play's tick, and
    /// a Next that waited for randomness: at most one step a tick.
    /// Returns whether the entry moved.
    pub(crate) fn walk_tick(&mut self, c: &Compute, now_ms: u64, draw: &mut Draw) -> bool {
        if !self.walk_open() {
            self.walk.playing = false;
            self.walk.wanted = false;
            return false;
        }
        if !self.walk.playing {
            return self.walk.wanted && self.walk_next(c, draw);
        }
        if now_ms < self.walk.next_ms {
            return false;
        }
        // Play goes through the tour as stepping does, and stops at its
        // end rather than leave the card.
        let moved = if self.tour_shown().is_some() {
            self.tour_step(c, 1, false)
        } else {
            self.walk_next(c, draw)
        };
        if !moved {
            // Waiting for randomness plays on; the end stops it.
            self.walk.playing &= self.walk.wanted;
            return false;
        }
        let ms = self.walk.speed.ms();
        self.walk.next_ms = (self.walk.next_ms + ms).max(now_ms + ms / 2);
        if self.walk_reached() {
            self.walk.playing = false;
        }
        true
    }

    /// Whether the step shown is as far as Play runs.
    fn walk_reached(&self) -> bool {
        if self.tour_shown().is_some() {
            return self.tour_reached(self.walk.reach);
        }
        let Some((ch, s, step)) = self.walk.shown() else {
            return false;
        };
        let last = s == ch.steps.len();
        let data = self.length.data_len();
        match self.walk.reach {
            Reach::Pass => matches!(
                step,
                Step::First { pass_end: true, .. } | Step::Second { pass_end: true, .. }
            ),
            Reach::Character => last,
            Reach::Share => last && ch.n + 1 == data,
            Reach::End => last && self.front().is_none(),
        }
    }

    /// Costs what [`Self::walk_go`] costs, and a fresh answer for a
    /// character rolled. The next step, rolling the next character's
    /// values where the walk has shown every step it has.
    fn walk_next(&mut self, c: &Compute, draw: &mut Draw) -> bool {
        if !self.walk_in_step() {
            self.walk.clear();
        }
        let (slot, s) = self.walk.at;
        if let Some(ch) = self.walk.chars.get(slot) {
            if s < ch.steps.len() {
                return self.walk_go(c, slot, s + 1);
            }
            if slot + 1 < self.walk.chars.len() {
                return self.walk_go(c, slot + 1, 1);
            }
        }
        // Every character is in and every step of the last shown: on to
        // the shares made and S (`tour.rs`).
        if self.tour_ready() {
            self.tour_enter();
            return true;
        }
        self.walk_roll(c, draw)
    }

    /// Costs what [`Self::walk_go`] costs. The step before.
    fn walk_back(&mut self, c: &Compute) -> bool {
        if !self.walk_in_step() {
            self.walk.clear();
            return false;
        }
        let (slot, s) = self.walk.at;
        if s > 1 || (slot == 0 && s == 1) {
            self.walk_go(c, slot, s - 1)
        } else if slot > 0 {
            let len = self.walk.chars[slot - 1].steps.len();
            self.walk_go(c, slot - 1, len)
        } else {
            false
        }
    }

    /// Costs a fresh answer (a hash, through `draw`) for every 31 faces
    /// at most, and what [`Self::walk_go`] costs. The next character's
    /// faces rolled, its steps worked out, and its first step shown.
    fn walk_roll(&mut self, c: &Compute, draw: &mut Draw) -> bool {
        let Some((share, n)) = self.front() else {
            return false;
        };
        let Some(more) = self.roller.roll_character(&self.seq, draw) else {
            self.walk.wanted = true;
            return false;
        };
        self.walk.wanted = false;
        let g = self.entered_all();
        let k0 = self.seq.len();
        let mut faces = Zeroizing::new(self.seq.to_vec());
        faces.extend_from_slice(&more);
        let Some(ch) = self.walk_char(faces, k0, g, (share, n)) else {
            return false;
        };
        if self.walk.chars.last().is_some_and(|l| l.g + 1 != g) {
            self.walk.clear();
        }
        if ch.face_steps() == 0 {
            return false;
        }
        self.walk.chars.push(ch);
        let slot = self.walk.chars.len() - 1;
        self.walk_go(c, slot, 1)
    }

    /// The character of `faces` (the first `k0` the person's), the `g`-th
    /// entered, data character `n` of share `share`: its sheet, its value
    /// and its steps. Arithmetic on its faces and the worksheet's shape.
    fn walk_char(
        &self,
        faces: Zeroizing<Vec<u8>>,
        k0: usize,
        g: usize,
        place: (usize, usize),
    ) -> Option<WalkChar> {
        self.char_steps(faces, k0, g, place, true)
    }

    /// The steps of `faces` (the first `k0` taken as shown already), the
    /// `g`-th character entered, data character `n` of share `share`:
    /// each face's step, then, where the faces make the character whole,
    /// its own step and the worksheet's cells its box fills. Not `whole`,
    /// faces that do not yet make a character give their faces' steps
    /// alone (the person's character in progress, `tracker.rs`); `whole`,
    /// they give none. Arithmetic on its faces and the worksheet's shape.
    pub(super) fn char_steps(
        &self,
        faces: Zeroizing<Vec<u8>>,
        k0: usize,
        g: usize,
        (share, n): (usize, usize),
        whole: bool,
    ) -> Option<WalkChar> {
        let mut dice = BookletChar::with_faces(self.dice_choice.faces).ok()?;
        let mut rows = Zeroizing::new(Vec::new());
        let mut steps = Vec::new();
        let mut first_at = [0u8; BOOKLET_DICE];
        let place_of = |d: &BookletChar| {
            (0..2 * BOOKLET_DICE).find(|&p| {
                if p < BOOKLET_DICE {
                    d.first(p).is_none()
                } else {
                    d.second(p - BOOKLET_DICE).is_none()
                }
            })
        };
        for (k, &face) in faces.iter().enumerate() {
            let place = place_of(&dice)?;
            dice.push(face).ok()?;
            let die = place % BOOKLET_DICE;
            let mut equal = false;
            if place < BOOKLET_DICE {
                first_at[die] = k as u8;
            } else {
                let row = DieRow {
                    die: die as u8,
                    first: dice.first(die).unwrap_or(0),
                    second: face,
                };
                rows.push(row);
                if row.bit().is_none() {
                    dice.redo(die);
                    equal = true;
                }
            }
            let pass_end = equal
                || dice.character().is_ok()
                || place_of(&dice).is_some_and(|p| p / BOOKLET_DICE != place / BOOKLET_DICE);
            if k < k0 {
                continue;
            }
            steps.push(if place < BOOKLET_DICE {
                Step::First {
                    k: k as u8,
                    die: die as u8,
                    pass_end,
                }
            } else {
                Step::Second {
                    k: k as u8,
                    first: first_at[die],
                    die: die as u8,
                    pass_end,
                }
            });
        }
        let value = match dice.character() {
            Ok(c) => {
                steps.push(Step::Char);
                self.cell_steps(share, 10 + n, &mut steps);
                super::symbol_of(char::from(c))?
            }
            Err(_) if !whole => 0,
            Err(_) => return None,
        };
        Some(WalkChar {
            g,
            share,
            n,
            faces,
            k0,
            value,
            dice,
            rows,
            steps,
        })
    }

    /// The steps of a character typed (no faces), the `g`-th entered,
    /// data character `n` of share `share`, value `value`: its own step,
    /// into its box, and the worksheet's cells its box fills. Arithmetic
    /// on the worksheet's shape.
    pub(super) fn typed_steps(
        &self,
        g: usize,
        (share, n): (usize, usize),
        value: u8,
    ) -> Option<WalkChar> {
        let mut steps = vec![Step::Char];
        self.cell_steps(share, 10 + n, &mut steps);
        Some(WalkChar {
            g,
            share,
            n,
            faces: Zeroizing::new(Vec::new()),
            k0: 0,
            value,
            dice: BookletChar::with_faces(self.dice_choice.faces).ok()?,
            rows: Zeroizing::new(Vec::new()),
            steps,
        })
    }

    /// The checksum worksheet's cells box `b` fills, in the order they
    /// are worked: each sum row's cell in its column, top down, with the
    /// table row a row's leading cells select once the last of them is
    /// in; at the last data box, the checksum boxes worked from below.
    /// Arithmetic on the worksheet's shape.
    pub(super) fn cell_steps(&self, share: usize, b: usize, steps: &mut Vec<Step>) {
        let Some(sh) = self
            .work
            .as_ref()
            .and_then(|w| w.worksheet(share, Reveal::PaperForm))
        else {
            return;
        };
        let len = sh.checksum_len();
        for j in 0..sh.steps() {
            let from = sh.sum_box(j);
            if (from..from + len).contains(&b) {
                let i = b - from;
                steps.push(Step::Sum {
                    j: j as u8,
                    i: i as u8,
                });
                if i + 1 == sh.step_width(j) {
                    steps.push(Step::Table { j: j as u8 });
                }
            }
        }
        if b == sh.last_data_box() {
            for c in b + 1..=sh.chars() {
                steps.push(Step::Below { b: c as u8 });
            }
        }
    }

    /// Costs a worksheet column for each character put in its box on the
    /// way. Brings the entry to step `s` of character `slot`, a face or
    /// a character at a time, and shows it; a way that does not get
    /// there forgets the walk. Returns whether it got there.
    pub(super) fn walk_go(&mut self, c: &Compute, slot: usize, s: usize) -> bool {
        let Some(to) = self.pos_of(slot, s) else {
            return false;
        };
        for _ in 0..4 * faraday_entropy::booklet::BOOKLET_ROLLS + 8 {
            let at = self.entry_pos();
            if at == to {
                break;
            }
            let moved = if at > to {
                self.walk_back_one()
            } else {
                self.walk_fwd_one(c)
            };
            if !moved {
                break;
            }
        }
        if self.entry_pos() == to {
            self.walk.at = (slot, s);
            // The walk's step is the one shown: the person's own steps
            // are worked out again at their next entry.
            self.track.clear();
            true
        } else {
            self.walk.clear();
            false
        }
    }

    /// The walk's character the entry is in now.
    fn walk_slot_of(&self, g: usize) -> Option<usize> {
        self.walk.chars.iter().position(|c| c.g == g)
    }

    /// Costs a worksheet column where the character goes in its box. One
    /// face on, the walk's next, held out of its box when it makes the
    /// character whole; or the character held, into its box.
    fn walk_fwd_one(&mut self, c: &Compute) -> bool {
        let (g, k) = self.entry_pos();
        let Some(slot) = self.walk_slot_of(g) else {
            return false;
        };
        let (face, mine, len) = {
            let ch = &self.walk.chars[slot];
            (ch.faces.get(k).copied(), k >= ch.k0, ch.faces.len())
        };
        match face {
            Some(f) if k < len => {
                // Marked first: the sheet worked again reads the mark.
                if mine {
                    self.roller.mark(g, k);
                }
                self.push_face(f);
                true
            }
            _ => {
                self.put_whole(c);
                self.entry_pos() != (g, k)
            }
        }
    }

    /// One face back; from a character in its box, the character out of
    /// it, whole and held.
    fn walk_back_one(&mut self) -> bool {
        let (g, k) = self.entry_pos();
        if k > 0 {
            self.seq.pop();
            self.replay();
            self.roller.trim(g, self.seq.len());
            return true;
        }
        let Some(slot) = g.checked_sub(1).and_then(|p| self.walk_slot_of(p)) else {
            return false;
        };
        self.undo_entry();
        let (last, mine) = {
            let ch = &self.walk.chars[slot];
            let at = ch.faces.len().saturating_sub(1);
            (ch.faces.get(at).copied(), at >= ch.k0)
        };
        let done = self.entered_all();
        self.roller.trim(done, self.seq.len());
        if let Some(f) = last {
            if mine {
                self.roller.mark(done, self.seq.len());
            }
            self.push_face(f);
        }
        true
    }

    /// The step shown, the one model's: the Demo walk's while it shows
    /// one, else the person's own entry's ([`Self::tracker`]), with its
    /// character and its number from 1. A lookup.
    pub fn shown_step(&self) -> Option<(&WalkChar, usize, Step)> {
        // While the tour shows a step, it is the one shown (`tour.rs`).
        if self.tour_shown().is_some() {
            return None;
        }
        self.walk.shown().or_else(|| self.track.shown())
    }

    /// The character the step shown is of, while that character is whole
    /// on its own sheet: at its own step, or at a face's step once it is
    /// in its box (the person's character before the one in progress).
    fn shown_whole(&self) -> Option<&WalkChar> {
        match self.shown_step()? {
            (ch, _, Step::Char) => Some(ch),
            (ch, _, Step::First { .. } | Step::Second { .. }) if ch.g < self.entered_all() => {
                Some(ch)
            }
            _ => None,
        }
    }

    /// The dice sheet drawn while the step shown is of a character whole
    /// (its own step, or a face of one in its box): that character, with
    /// its path through the tree; else the character in progress.
    pub fn shown_dice(&self) -> &BookletChar {
        self.shown_whole().map_or(&self.dice, |ch| &ch.dice)
    }

    /// The pairs drawn with [`Self::shown_dice`].
    pub fn shown_rows(&self) -> &[DieRow] {
        self.shown_whole().map_or(&self.rows, |ch| &ch.rows)
    }

    /// The initial share whose string and checksum worksheet the entry
    /// shows, and the box being filled: the share being entered and its
    /// front; while the walk shows a step of a character in its box, that
    /// character's share, with no front (its own cells are marked).
    pub fn entry_share(&self) -> Option<(usize, Option<usize>)> {
        if let Some((ch, _, step)) = self.shown_step()
            && !matches!(step, Step::First { .. } | Step::Second { .. })
        {
            return Some((ch.share, None));
        }
        match self.front() {
            Some((s, n)) => Some((s, Some(10 + n))),
            None => Some((self.work.as_ref()?.inputs().len().checked_sub(1)?, None)),
        }
    }

    /// The die and the pad the step shown touches on the dice sheet.
    pub fn walk_die_mark(&self) -> Option<(usize, u8)> {
        match self.shown_step()? {
            (ch, _, Step::First { k, die, .. } | Step::Second { k, die, .. }) => {
                Some((usize::from(die), ch.face(k)))
            }
            _ => None,
        }
    }

    /// The cells of share `share`'s checksum worksheet the step shown
    /// touches, each with whether it is the one worked out: its inputs
    /// and its result. Arithmetic on the worksheet's shape.
    pub fn walk_cells(&self, share: usize) -> Vec<(Cell, bool)> {
        let Some((ch, _, step)) = self.shown_step().filter(|(c, _, _)| c.share == share) else {
            return Vec::new();
        };
        let Some(sh) = self
            .work
            .as_ref()
            .and_then(|w| w.worksheet(share, Reveal::PaperForm))
        else {
            return Vec::new();
        };
        let n = ch.n;
        worksheet_screen::paper!(sh, |p| step_cells(p, share as u8, step, n))
    }
}

/// The cells of the lent worksheet `sh` (numbered `sheet` for its cells)
/// that `step` touches, each with whether it is the one worked out; `n`
/// the data character of a character's own step. Arithmetic on the
/// worksheet's shape.
pub(super) fn step_cells(sh: Paper<'_>, sheet: u8, step: Step, n: usize) -> Vec<(Cell, bool)> {
    let len = sh.checksum_len();
    // Cells by their row and place, from 0.
    let sum = |j: usize, i: usize| Cell::Sum {
        sheet,
        j: j as u8,
        i: i as u8,
    };
    let look = |j: usize, i: usize| Cell::Lookup {
        sheet,
        j: j as u8,
        i: i as u8,
    };
    let given = |b: usize| Cell::Given { sheet, b: b as u8 };
    match step {
        Step::Char => vec![(given(10 + n), true)],
        Step::Sum { j, i } => {
            let (j, i) = (usize::from(j), usize::from(i));
            let b = sh.sum_box(j) + i;
            let mut out = if j == 0 {
                vec![(given(b), false), (Cell::Hrp { sheet, i: i as u8 }, false)]
            } else {
                let w = sh.step_width(j - 1);
                let cell = if i + w < len {
                    sum(j - 1, i + w)
                } else {
                    given(b)
                };
                vec![(cell, false), (look(j - 1, i), false)]
            };
            out.push((sum(j, i), true));
            out
        }
        Step::Table { j } => {
            let j = usize::from(j);
            let lead = (0..sh.step_width(j)).map(|i| (sum(j, i), false));
            lead.chain((0..len).map(|i| (look(j, i), true))).collect()
        }
        Step::Below { b } => {
            let b = usize::from(b);
            let Some(j) =
                (1..=sh.steps()).find(|&j| (sh.sum_box(j)..sh.sum_box(j) + len).contains(&b))
            else {
                return Vec::new();
            };
            vec![
                (sum(j, b - sh.sum_box(j)), false),
                (look(j - 1, b - sh.lookup_box(j - 1)), false),
                (given(b), true),
            ]
        }
        _ => Vec::new(),
    }
}
