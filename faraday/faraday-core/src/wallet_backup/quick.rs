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
    /// More ways to back up: the board.
    More,
}

/// Which of the two screens is open.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Write the words down.
    #[default]
    Write,
    /// Check my copy.
    Check,
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
            b.quick_shown = false;
            b.board = crate::board::BoardState::default();
        }
    }

    /// The copy item stays the open one while the short path is under
    /// way: the checklist's first item not done is the templates, which
    /// the short path does not show.
    pub(crate) fn quick_keeps_open(&mut self) {
        if let Some(b) = self.state.as_mut().filter(|b| b.quick) {
            b.open = Some(plan::Item::Copy(0));
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
