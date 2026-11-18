//! The quick paper backup (`docs/BACKUP-FLOW.md` §19): a single key backed
//! up as words takes Paper only without the board, in two screens, Write
//! the words down and Check my copy. The plan is the one the board would
//! have made, so everything that reads a plan reads it as Paper only.

use super::WalletBackupArea;
use crate::plan;
use faraday_text::Text;

/// A press of the quick paper backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quick {
    /// Back up wallet `n`: the short path where the wallet takes it, else
    /// the board.
    Open(usize),
    /// The step card: nothing to open or close.
    Card,
    /// Show the seed, from either screen: the first screen with the words
    /// shown.
    Show,
    /// I wrote them down: the words are hidden and the typed check opens.
    Wrote,
    /// Done: on to the end card.
    Done,
    /// Blank template: the sheet that says what to print.
    Template,
    /// Show the SeedQR: the plan keeps the seed's SeedQR beside its words
    /// and the first screen shows both.
    SeedQr,
    /// Krux code: the plan keeps seed 0's encrypted backup as a code to
    /// copy onto paper, at the words' place, and a third card is part of
    /// the flow.
    Code,
    /// Next: from the check of the copy to the Krux code's card.
    Next,
    /// The check of the copy, from the Krux code's card.
    Copy,
    /// Make a backup plan: the board.
    More,
}

/// Which of the two screens is open.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Make a copy.
    #[default]
    Write,
    /// Check my copy.
    Check,
    /// The Krux code, where the plan keeps one.
    Code,
}

impl WalletBackupArea {
    /// Whether the backup under way is on the short path.
    pub(crate) fn is_quick(&self) -> bool {
        self.state.as_ref().is_some_and(|b| b.quick)
    }

    /// The checklist made, on the short path's first screen with the copy
    /// item the one open.
    pub(super) fn quick_begin(&mut self) {
        self.checklist();
        if let Some(b) = self.state.as_mut() {
            b.quick = true;
            b.quick_step = Step::Write;
            b.quick_shown = false;
            b.quick_code = false;
            b.open = Some(plan::Item::Copy(0));
        }
    }

    /// The backup of the wallet with checksum `sum` is the board's for
    /// good.
    pub(super) fn quick_left_for(&mut self, sum: &str) {
        if !self.more_ways.iter().any(|s| s.as_str() == sum) {
            self.more_ways.push(Text::of(sum));
        }
        if let Some(b) = self.state.as_mut() {
            b.quick = false;
            b.quick_done = false;
            b.quick_shown = false;
            b.quick_code = false;
            b.board = crate::board::BoardState::default();
        }
    }

    /// The plan keeps the seed's SeedQR beside its words, as the board's
    /// SeedQR answer does (`docs/BACKUP-FLOW.md` §19.2), and the first
    /// screen shows the seed. Whether the short path made it.
    pub(super) fn quick_seedqr(&mut self) -> bool {
        let Some(b) = self.state.as_mut().filter(|b| b.quick) else {
            return false;
        };
        // A plan with a map (a Krux code put on it) keeps it in the map:
        // the caller puts it there.
        b.answers.seeds[plan::seeds::SEEDQR] = true;
        b.quick_step = Step::Write;
        b.scanned = None;
        b.check = None;
        b.check_at = None;
        b.reveal = true;
        b.quick_shown = true;
        true
    }

    /// The copy item stays the open one while the short path is under
    /// way, the code's on its card: the checklist's first item not done
    /// is the templates, which the short path does not show.
    pub(crate) fn quick_keeps_open(&mut self) {
        if let Some(b) = self.state.as_mut().filter(|b| b.quick) {
            b.open = Some(if b.quick_step == Step::Code {
                plan::Item::KefCode(0)
            } else {
                plan::Item::Copy(0)
            });
        }
    }

    /// The match of the words completes the short path's backup: the
    /// templates' and the envelopes' presses are recorded with it.
    /// Whether a match was of the words and the backup is the short
    /// path's is the caller's.
    pub(super) fn quick_complete(&mut self) -> bool {
        if !self.is_quick() {
            return false;
        }
        self.mark_done(plan::Item::Templates);
        self.mark_done(plan::Item::Envelopes);
        true
    }
}
