//! The step tracker of the person's own entry (`docs/CODEX32.md` §5.7,
//! owner 2026-10-24): what their last entry did and what they would do
//! next on paper, a step at a time, with the sheet marks the Demo walk
//! uses. It is the walk's own model ([`Walk`], [`Step`], [`WalkChar`]),
//! with the person's faces as its source: each value entered shows its
//! step ("Die 2, second roll: 5. 5 is higher than 3, so bit 1."), a
//! character completed shows its leaf and its box, and Next then walks
//! the worksheet's cells that character fills, a sum or a table row at a
//! time, at the person's pace. Previous and Next only move the step
//! shown: they never enter, take back or roll a value, in any mode; the
//! next value entered moves the tracker to it.
//!
//! **Kept in step.** The tracker holds at most two characters: the last
//! one in its box (from its record, [`super::Made`]) and the one in
//! progress (from `seq`). Both are worked out again, once, at every
//! change of the entry by the person ([`Codex32Make::act`] calls
//! [`Codex32Make::track`]); drawing and the sheet marks only read them
//! (`tracker_screen.rs`). A step of the Demo walk shown clears it
//! (`walk.rs`), as the walk's step is the one the sheets show then.

use zeroize::Zeroizing;

use super::walk::Walk;
use super::{Codex32Make, Way};

impl Codex32Make {
    /// The person's own entry's steps.
    pub fn tracker(&self) -> &Walk {
        &self.track
    }

    /// The `g`-th character entered, of every share: its share and its
    /// place among that share's data characters. A lookup over at most 31
    /// shares.
    fn entered_place(&self, g: usize) -> Option<(usize, usize)> {
        let work = self.work.as_ref()?;
        let mut before = 0;
        for s in 0..work.inputs().len() {
            let e = work.entered(s);
            if g < before + e {
                return Some((s, g - before));
            }
            before += e;
        }
        None
    }

    /// The steps of the `g`-th character entered, in its box: from its
    /// faces where it was rolled, its cells alone where typed.
    /// Arithmetic on its faces and the worksheet's shape.
    fn entered_steps(&self, g: usize) -> Option<super::WalkChar> {
        let (share, n) = self.entered_place(g)?;
        let made = self.made.get(share)?.get(n)?;
        if made.typed {
            return self.typed_steps(g, (share, n), made.value);
        }
        let faces = Zeroizing::new(made.seq.to_vec());
        self.char_steps(faces, 0, g, (share, n), true)
    }

    /// Works the tracker out again after a change of the entry by the
    /// person, from `before`, where the entry stood: the character last
    /// put in its box and the one in progress, each with its steps; the
    /// step shown is the last value's, or, where a character has just gone
    /// into its box, its own step (the first of its cells where it was
    /// typed). A change of more than one character (Demo's sample) shows
    /// none. Arithmetic on the faces and the worksheet's shape.
    pub(super) fn track(&mut self, before: (usize, usize)) {
        self.track.clear();
        let (g, k) = self.entry_pos();
        if self.way == Way::Shares || g.abs_diff(before.0) > 1 || self.walk.shown().is_some() {
            return;
        }
        let mut chars = Vec::new();
        if let Some(prev) = g.checked_sub(1).and_then(|p| self.entered_steps(p))
            && !prev.steps().is_empty()
        {
            chars.push(prev);
        }
        if k > 0
            && let Some(front) = self.front()
            && let Some(cur) = self.char_steps(self.seq.clone(), 0, g, front, false)
        {
            chars.push(cur);
        }
        let last = chars.len().saturating_sub(1);
        let at = match chars.last() {
            Some(cur) if !cur.whole() => (last, cur.steps().len()),
            Some(ch) => (last, (ch.face_steps() + 1).min(ch.steps().len())),
            None => (0, 0),
        };
        self.track.chars = chars;
        self.track.at = at;
    }

    /// Whether the tracker has a step `d` (−1 or 1) from the one shown.
    pub fn track_can(&self, d: i8) -> bool {
        let (slot, s) = self.track.at;
        let Some(ch) = self.track.chars.get(slot) else {
            return false;
        };
        if d < 0 {
            s > 1 || slot > 0
        } else {
            s < ch.steps().len() || slot + 1 < self.track.chars.len()
        }
    }

    /// The step `d` (−1 or 1) from the one shown, the next character's
    /// first or the one before's last across the two. Moves only what is
    /// shown: the entry stays as it is. A lookup.
    pub(super) fn track_step(&mut self, d: i8) {
        if !self.track_can(d) {
            return;
        }
        let (slot, s) = self.track.at;
        self.track.at = if d < 0 {
            if s > 1 {
                (slot, s - 1)
            } else {
                let len = self.track.chars[slot - 1].steps().len();
                (slot - 1, len)
            }
        } else if s < self.track.chars[slot].steps().len() {
            (slot, s + 1)
        } else {
            (slot + 1, 1)
        };
    }

    /// What the person enters next: die `die`'s (from 0) first or second
    /// roll (`second`) of the character in progress, by dice; none while
    /// no character is left, or characters are typed. A lookup over the
    /// sheet's ten places.
    pub fn next_roll(&self) -> Option<(u8, bool)> {
        use faraday_entropy::booklet::BOOKLET_DICE;
        if self.way != Way::Dice || self.front().is_none() {
            return None;
        }
        // A character whole on the sheet goes into its box first.
        if self.dice.character().is_ok() {
            return Some((0, false));
        }
        (0..2 * BOOKLET_DICE).find_map(|p| {
            let free = if p < BOOKLET_DICE {
                self.dice.first(p).is_none()
            } else {
                self.dice.second(p - BOOKLET_DICE).is_none()
            };
            free.then_some(((p % BOOKLET_DICE) as u8, p >= BOOKLET_DICE))
        })
    }

    /// Whether the step shown is the tracker's last: nothing further for
    /// Next, the next value the person's.
    pub fn track_at_end(&self) -> bool {
        !self.track_can(1)
    }

    /// The step the tracker shows, by place, for tests.
    #[cfg(test)]
    pub(super) fn track_shown(&self) -> Option<super::walk::Step> {
        self.track.shown().map(|(_, _, s)| s)
    }
}
