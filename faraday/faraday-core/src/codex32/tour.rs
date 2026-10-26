//! The tour of the shares made and S, past the last initial share's
//! checksum (`docs/CODEX32.md` §5.7, owner 2026-10-24): the translation
//! worksheets fill live as the initial shares are entered (§6.4.4), and
//! once the last checksum box is shown, Next opens the first derived
//! share's tab, already filled, and goes through it as the booklet works
//! it: each initial share's translation symbol, each share's characters
//! translated box by box, the sum box by box, then the share's checksum
//! worked as the booklet's verification (§7.1) to its final row; then the
//! next share made, then S, with its recovery symbols and their fusion
//! (§7.3). After S's last box, Next goes on to the worksheets. Previous
//! goes back through the same steps, and from the first one back to the
//! last checksum box. **Show the other shares and S as they fill** (on by
//! default, kept on the flow) turns the tour off: the tabs still fill
//! live.
//!
//! **One model.** The Practice walk (`walk.rs`) and the person's own tracker
//! (`tracker.rs`) both go on into these steps, through the Working area's
//! Previous and Next and the walk's Play; the check's steps are the
//! walk's own [`Step`]s, read on the share's own checksum worksheet.
//!
//! **Built once.** The steps are worked out when they become reachable,
//! as the set is made ([`Codex32Make::tour_build`], from `complete`), and
//! forgotten when it is unmade (a character taken back): they hold
//! places, not values. Each share made's checksum check is its
//! verification worksheet ([`StringCheck`]), worked out there once and
//! kept here. Drawing reads the steps, the kept translation worksheets
//! and the checks (`tour_screen.rs`).

use faraday_crypto::work::Compute;
use faraday_keys::{Reveal, StringCheck};

use super::walk::{Step, step_cells};
use super::worksheet_screen::paper;
use super::{Card, Cell, Codex32Make};
use crate::fresh::roller::five::Reach;

/// The number a share made's checksum check gives its cells: past any
/// initial share's (there are at most 31), the `t`-th sheet's.
pub(crate) const CHECK_SHEET: u8 = 64;

/// One step of the tour, by place: what it reads is drawn from the kept
/// worksheets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourStep {
    /// Translation worksheet `t`'s symbol for initial share `i`.
    Symbol {
        /// The sheet, by its place among the targets.
        t: u8,
        /// The initial share.
        i: u8,
    },
    /// Initial share `i`'s character at box `b` times its symbol.
    Translate {
        /// The sheet.
        t: u8,
        /// The initial share.
        i: u8,
        /// The box.
        b: u8,
    },
    /// The translated characters at box `b` added: the share made's box.
    Sum {
        /// The sheet.
        t: u8,
        /// The box.
        b: u8,
    },
    /// A step of share `t`'s checksum check: a sum row's cell (the final
    /// row's past the last step) or a table row.
    Check {
        /// The sheet.
        t: u8,
        /// The step on the check's worksheet.
        step: Step,
    },
    /// Share `t`'s checksum check done: its final row read.
    Holds {
        /// The sheet.
        t: u8,
    },
}

impl TourStep {
    /// The translation worksheet it is of, by its place among the targets.
    pub fn sheet(self) -> usize {
        let t = match self {
            TourStep::Symbol { t, .. }
            | TourStep::Translate { t, .. }
            | TourStep::Sum { t, .. }
            | TourStep::Check { t, .. }
            | TourStep::Holds { t } => t,
        };
        usize::from(t)
    }

    /// Whether it is of a share's checksum check.
    pub fn on_check(self) -> bool {
        matches!(self, TourStep::Check { .. } | TourStep::Holds { .. })
    }

    /// Where it stands, for how far Play runs: its sheet, its part (the
    /// translation, or the check) and its row within the part.
    fn group(self) -> (usize, u8, usize) {
        let t = self.sheet();
        match self {
            TourStep::Symbol { .. } => (t, 0, 0),
            TourStep::Translate { i, .. } => (t, 0, 1 + usize::from(i)),
            TourStep::Sum { .. } => (t, 0, usize::MAX),
            TourStep::Check {
                step: Step::Sum { j, .. } | Step::Table { j },
                ..
            } => (t, 1, usize::from(j)),
            TourStep::Check { .. } | TourStep::Holds { .. } => (t, 1, usize::MAX),
        }
    }
}

/// The tour's state: its steps, the one shown, the choice, and each share
/// made's checksum check.
pub struct Tour {
    steps: Vec<TourStep>,
    /// The step shown, from 0; none while the tour shows nothing.
    at: Option<usize>,
    /// **Show the other shares and S as they fill**.
    on: bool,
    /// Each target's checksum check, by its place: none for S.
    checks: Vec<Option<StringCheck>>,
}

impl Default for Tour {
    fn default() -> Tour {
        Tour {
            steps: Vec::new(),
            at: None,
            on: true,
            checks: Vec::new(),
        }
    }
}

impl Tour {
    /// Its steps, in order.
    pub fn steps(&self) -> &[TourStep] {
        &self.steps
    }

    /// Whether the choice is on.
    pub fn on(&self) -> bool {
        self.on
    }
}

impl Codex32Make {
    /// The tour of the shares made and S.
    pub fn tour(&self) -> &Tour {
        &self.tour
    }

    /// Costs a checksum worksheet of each share made. The tour's steps,
    /// worked out once as the set is made: for each target, its symbols,
    /// each initial share translated box by box, the sum box by box, and,
    /// for a share made, its checksum worked as the booklet's
    /// verification. Kept until the set is unmade ([`Self::tour_drop`]).
    pub(super) fn tour_build(&mut self, c: &Compute) {
        self.tour_drop();
        // A set made of strings given opens on the worksheets: no entry,
        // no tour.
        let Some(work) = self.work.as_ref().filter(|w| !w.is_given()) else {
            return;
        };
        let chars = work.scheme().length().chars();
        let mut steps = Vec::new();
        let mut checks = Vec::new();
        for (t, tg) in work.targets().into_iter().enumerate() {
            let Some(ts) = work.live_translation(t, Reveal::PaperForm) else {
                break;
            };
            let k = ts.threshold();
            let t = t as u8;
            for i in 0..k as u8 {
                steps.push(TourStep::Symbol { t, i });
            }
            for i in 0..k as u8 {
                for b in 4..=chars as u8 {
                    steps.push(TourStep::Translate { t, i, b });
                }
            }
            for b in 4..=chars as u8 {
                steps.push(TourStep::Sum { t, b });
            }
            let made = (tg != b's')
                .then(|| {
                    self.strings()?
                        .iter()
                        .find(|s| s.as_bytes().get(8) == Some(&tg))
                })
                .flatten();
            let check = made.and_then(|s| StringCheck::of(c, s));
            if let Some(sh) = check
                .as_ref()
                .and_then(|ch| ch.worksheet(Reveal::PaperForm))
            {
                let (rows, len) = (sh.steps(), sh.checksum_len());
                for j in 0..rows as u8 {
                    for i in 0..len as u8 {
                        let step = Step::Sum { j, i };
                        steps.push(TourStep::Check { t, step });
                    }
                    steps.push(TourStep::Check {
                        t,
                        step: Step::Table { j },
                    });
                }
                for i in 0..len as u8 {
                    let step = Step::Sum { j: rows as u8, i };
                    steps.push(TourStep::Check { t, step });
                }
                steps.push(TourStep::Holds { t });
            }
            checks.push(check);
        }
        self.tour.steps = steps;
        self.tour.checks = checks;
    }

    /// The set is unmade: the tour's steps and checks are forgotten.
    pub(super) fn tour_drop(&mut self) {
        if self.tour.at.take().is_some() {
            self.view = 0;
        }
        self.tour.steps.clear();
        self.tour.checks.clear();
    }

    /// The step the tour shows, with its number from 1, while the entry
    /// is open. A lookup.
    pub fn tour_shown(&self) -> Option<(usize, TourStep)> {
        if self.open != Some(Card::Enter) {
            return None;
        }
        let at = self.tour.at?;
        Some((at + 1, *self.tour.steps.get(at)?))
    }

    /// Whether Next would open the tour: the choice on, every character
    /// in, the tour not shown. The caller knows the step shown before it
    /// is the last. A lookup.
    pub fn tour_ready(&self) -> bool {
        self.tour.on
            && self.tour.at.is_none()
            && !self.tour.steps.is_empty()
            && self.open == Some(Card::Enter)
            && self.front().is_none()
    }

    /// The tour opens at its first step, on the first share made's tab.
    pub(super) fn tour_enter(&mut self) {
        self.tour_go(0);
    }

    /// Step `at` shown, its sheet's tab with it; the page follows its
    /// cells once drawn.
    fn tour_go(&mut self, at: usize) {
        let Some(step) = self.tour.steps.get(at) else {
            return;
        };
        self.view = step.sheet() as u8 + 1;
        self.tour.at = Some(at);
        self.cell = None;
        self.page_due = true;
    }

    /// Costs what the Enter card's Continue costs at the tour's last step
    /// with `leave`. The step `d` (−1 or 1) from the one shown: from the
    /// first, Previous leaves the tour for the last checksum box; at the
    /// last, Next with `leave` goes on to the worksheets, as Continue
    /// does, and without it (Play) stops. Returns whether it moved.
    pub(super) fn tour_step(&mut self, c: &Compute, d: i8, leave: bool) -> bool {
        let Some(at) = self.tour.at else {
            return false;
        };
        if d < 0 {
            if at == 0 {
                self.tour.at = None;
                self.view = 0;
                self.page_due = true;
            } else {
                self.tour_go(at - 1);
            }
            return true;
        }
        if at + 1 < self.tour.steps.len() {
            self.tour_go(at + 1);
            return true;
        }
        if !leave {
            return false;
        }
        self.tour.at = None;
        self.view = 0;
        self.next(c);
        true
    }

    /// Whether the tour has a step `d` (−1 or 1) from the one shown, or
    /// Next opens it. A lookup.
    pub fn tour_can(&self, d: i8) -> bool {
        match self.tour_shown() {
            Some(_) => true,
            None => d > 0 && self.tour_ready(),
        }
    }

    /// **Show the other shares and S as they fill**, turned on or off:
    /// off, a tour shown closes back to the last checksum box.
    pub(super) fn tour_toggle(&mut self) {
        self.tour.on = !self.tour.on;
        if !self.tour.on && self.tour.at.take().is_some() {
            self.view = 0;
        }
    }

    /// Whether the tour's step shown is as far as Play runs at `reach`:
    /// a row of a sheet (Pass), a part, the translation or the check
    /// (Character), a share (Share), the end. A lookup.
    pub(super) fn tour_reached(&self, reach: Reach) -> bool {
        let Some(at) = self.tour.at else {
            return true;
        };
        let (Some(now), Some(next)) = (self.tour.steps.get(at), self.tour.steps.get(at + 1)) else {
            return true;
        };
        let (a, b) = (now.group(), next.group());
        match reach {
            Reach::Pass => a != b,
            Reach::Character => (a.0, a.1) != (b.0, b.1),
            Reach::Share => a.0 != b.0,
            Reach::End => false,
        }
    }

    /// Share made `t`'s checksum check, once the set is made. A lookup.
    pub fn tour_check(&self, t: usize) -> Option<&StringCheck> {
        self.tour.checks.get(t)?.as_ref()
    }

    /// The share made whose checksum check the tour's step shown is on.
    pub fn tour_on_check(&self) -> Option<usize> {
        let (_, step) = self.tour_shown()?;
        step.on_check().then(|| step.sheet())
    }

    /// Whether the tour's step shown is on a translation worksheet: the
    /// page follows the grid's marked cells.
    pub(crate) fn tour_on_grid(&self) -> bool {
        self.tour_shown().is_some_and(|(_, s)| !s.on_check())
    }

    /// The cells of translation worksheet `t` the tour's step shown
    /// touches, each with whether it is the one worked out: a symbol; an
    /// initial share's character, its symbol and the product; the
    /// products added and the sum. Arithmetic on the step.
    pub fn tour_cells(&self, t: usize) -> Vec<(Cell, bool)> {
        let Some((_, step)) = self.tour_shown().filter(|(_, s)| s.sheet() == t) else {
            return Vec::new();
        };
        let k = self
            .work
            .as_ref()
            .and_then(|w| w.live_translation(t, Reveal::PaperForm))
            .map_or(0, |ts| ts.threshold());
        let sheet = t as u8;
        match step {
            TourStep::Symbol { i, .. } => vec![(Cell::Symbol { sheet, i }, true)],
            TourStep::Translate { i, b, .. } => vec![
                (Cell::Given { sheet: i, b }, false),
                (Cell::Symbol { sheet, i }, false),
                (Cell::Translated { sheet, i, b }, true),
            ],
            TourStep::Sum { b, .. } => {
                let mut out: Vec<(Cell, bool)> = (0..k as u8)
                    .map(|i| (Cell::Translated { sheet, i, b }, false))
                    .collect();
                out.push((Cell::TSum { sheet, b }, true));
                out
            }
            _ => Vec::new(),
        }
    }

    /// The cells of share made `t`'s checksum check the tour's step shown
    /// touches, each with whether it is the one worked out; at the check's
    /// end, its final row. Arithmetic on the worksheet's shape.
    pub fn tour_check_cells(&self, t: usize) -> Vec<(Cell, bool)> {
        let Some((_, step)) = self.tour_shown().filter(|(_, s)| s.sheet() == t) else {
            return Vec::new();
        };
        let Some(sh) = self
            .tour_check(t)
            .and_then(|ch| ch.worksheet(Reveal::PaperForm))
        else {
            return Vec::new();
        };
        let sheet = CHECK_SHEET + t as u8;
        match step {
            TourStep::Check { step, .. } => paper!(sh, |p| step_cells(p, sheet, step, 0)),
            TourStep::Holds { .. } => {
                let j = sh.steps() as u8;
                (0..sh.checksum_len() as u8)
                    .map(|i| (Cell::Sum { sheet, j, i }, true))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Costs what [`Self::tour_step`] costs: the Enter card's Continue at
    /// the tour's last step, else a lookup. The Working area's Previous
    /// or Next (`C32::Step`) outside the Practice
    /// walk: the tour's step while it shows one; at the tracker's last
    /// step, Next opens the tour where it is ready; else the tracker's.
    pub(super) fn step_press(&mut self, c: &Compute, d: i8) {
        if self.tour_shown().is_some() {
            self.tour_step(c, d, true);
        } else if d > 0 && !self.track_can(1) && self.tour_ready() {
            self.tour_enter();
        } else {
            self.track_step(d);
        }
    }
}
