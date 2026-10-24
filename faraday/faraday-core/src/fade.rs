//! A change of screen or sheet cross-fading in (`docs/MOTION.md` §3.5):
//! when a frame shows a change, the old frame kept to fade from and the
//! new one kept aside; the steps after, which blend the two without
//! drawing either again; and whether this machine shows a fade's steps
//! close enough together for it to read as one.

use faraday_draw::Canvas;
use faraday_shell_api::Command;

use crate::ui::Theme;
use crate::{Faraday, ScreenKey, Sheet, motion};

/// A change of screen, sheet or theme fading in, with the frosted page under an
/// open sheet and the screen the last frame drew. Its fields are private.
#[derive(Default)]
pub(crate) struct Fade {
    /// The frame before a change of screen or sheet, kept while the new
    /// one fades in over it, and when the fade's clock started: `None`
    /// for the frame that showed the change, whose cost (frosting the
    /// page under a sheet is the slowest frame there is) is not taken
    /// out of the fade.
    from: Option<(Vec<u8>, Option<u64>)>,
    /// The new frame of the fade under way, as last drawn in full, kept
    /// aside so that a step of the fade nothing else asked a frame for
    /// blends the two kept frames instead of drawing the screen again
    /// (`Faraday::fade_step`). Drawn again, and kept again, by every
    /// frame drawn in full while the fade runs; empty with no fade.
    after: Vec<u8>,
    /// The fade asked for its next step and nothing has asked for a
    /// frame drawn in full since (`dirty` is not set).
    due: bool,
    /// The tick of the last fade step, while the fade's steps follow one
    /// another with no frame drawn in full between: the gap to the next
    /// is how fast this machine shows a fade.
    step_at: Option<u64>,
    /// The last few gaps between the steps of fades, in milliseconds,
    /// each with the tick it was taken at, oldest first: how fast this
    /// machine shows a fade. Only steps that blend the two kept frames
    /// are timed, so what a screen costs to draw, and the frame a change
    /// is drawn on, never count; their median is what counts, and gaps
    /// older than `motion::FADE_GAPS_FRESH_MS` are forgotten, so a run
    /// of slow ones, the machine busy for a moment, does not turn fades
    /// off for good.
    gaps: Vec<(u64, u16)>,
    /// What fades have done since a shell's timings last took it
    /// (`Faraday::take_fade_counts`).
    counts: FadeCounts,
    /// The frosted page under the open sheet, and the screen, sheet and
    /// theme it was made for.
    frost: Option<((ScreenKey, Theme), Vec<u8>)>,
    /// The screen and sheet the last frame drew, the Learn sheet's page
    /// and the theme, so that a change is known at the next.
    last_drawn: Option<(ScreenKey, usize, Theme)>,
    /// Whether the fade under way is a change of theme, which takes
    /// longer and is eased; set when a fade starts.
    theme_fade: bool,
}

impl Fade {
    /// How far through the fade under way the blend is, 0 to 1, and
    /// whether the fade is over: a change of theme is eased over
    /// `motion::THEME_FADE_MS`, a change of screen linear over
    /// `motion::FADE_MS`.
    fn blend(&self, at: Option<u64>, now_ms: u64) -> (f32, bool) {
        let ms = if self.theme_fade {
            motion::THEME_FADE_MS
        } else {
            motion::FADE_MS
        };
        let t = motion::progress(at, now_ms, ms);
        let shown = if self.theme_fade {
            motion::ease_move(t)
        } else {
            t
        };
        (shown, t >= 1.0)
    }

    /// The fade under way ends where it is: the next frame is the
    /// screen at rest.
    pub(crate) fn end(&mut self) {
        self.from = None;
        self.after = Vec::new();
    }

    /// The next frame is the screen at rest, not part of the way from
    /// the one before.
    pub(crate) fn restart(&mut self) {
        self.end();
        self.last_drawn = None;
    }

    /// The display changed: nothing kept is of this size. A fade's step
    /// costs a blend of the whole canvas, and what it cost at another
    /// size says nothing of this one.
    pub(crate) fn display_changed(&mut self) {
        self.frost = None;
        self.restart();
        self.gaps.clear();
    }

    /// The frosted page kept for `key`, taken out to be used by the
    /// frame; `None` when it was made for another screen, sheet or
    /// theme.
    pub(crate) fn take_frost(&mut self, key: &(ScreenKey, Theme)) -> Option<Vec<u8>> {
        self.frost
            .take()
            .filter(|(k, _)| k == key)
            .map(|(_, page)| page)
    }

    /// Keeps the frosted page the frame made for `key`.
    pub(crate) fn keep_frost(&mut self, key: (ScreenKey, Theme), page: Option<Vec<u8>>) {
        self.frost = page.map(|page| (key, page));
    }

    /// A tick: a fade whose clock has not started starts it, a frame
    /// back, so this frame is the first step of it rather than the old
    /// screen again.
    pub(crate) fn tick_clock(&mut self, now_ms: u64) {
        if let Some((_, at @ None)) = self.from.as_mut() {
            *at = Some(now_ms.saturating_sub(motion::FADE_FIRST_STEP_MS));
        }
    }

    /// A tick: a fade under way asks for its next step; returns whether
    /// it does.
    pub(crate) fn tick_due(&mut self) -> bool {
        if self.from.is_some() {
            self.due = true;
        }
        self.from.is_some()
    }

    /// Whether the next frame is a fade's step.
    pub(crate) fn is_due(&self) -> bool {
        self.due
    }

    /// The gaps between fade steps recent enough to count, oldest first.
    fn fresh_gaps(&self, now_ms: u64) -> Vec<u16> {
        self.gaps
            .iter()
            .filter(|(at, _)| now_ms.saturating_sub(*at) <= motion::FADE_GAPS_FRESH_MS)
            .map(|(_, gap)| *gap)
            .collect()
    }

    /// Whether a change fades, given reduce motion: not with it, and not
    /// on a machine whose last fades' steps came too far apart for a
    /// fade to read as one rather than as steps. Until a few steps have
    /// been timed lately, it does.
    fn fades(&self, reduce_motion: bool, now_ms: u64) -> bool {
        if reduce_motion {
            return false;
        }
        let mut gaps = self.fresh_gaps(now_ms);
        if gaps.len() < motion::FADE_GAPS_KNOWN {
            return true;
        }
        gaps.sort_unstable();
        f32::from(gaps[gaps.len() / 2]) <= motion::FADE_GAP_MS
    }
}

/// What fades have done since a shell last took them, for its timings
/// (`Faraday::take_fade_counts`): how often a change faded, and how
/// fast this machine has lately shown a fade's steps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FadeCounts {
    /// Changes of screen or sheet that faded in.
    pub started: u32,
    /// Changes drawn at once because the last fades' steps came too far
    /// apart.
    pub skipped: u32,
    /// Changes drawn at once because reduce motion is on.
    pub reduced: u32,
    /// Steps of a fade drawn by blending the two kept frames.
    pub steps: u32,
    /// How many timed gaps between steps are recent enough to count.
    pub gaps_known: u8,
    /// The median of those gaps, milliseconds, when there are any.
    pub median_gap_ms: Option<u16>,
    /// Whether the next change would fade.
    pub on: bool,
}

impl Faraday {
    /// Before a frame is drawn in full on `canvas`, which still holds the
    /// frame before it: a change of screen or sheet, or of the Learn
    /// sheet's tab, starts a fade from that frame. The first frame ever,
    /// and the first after a resize, have nothing to fade from. Costs a
    /// copy of the whole canvas when a fade starts.
    pub(crate) fn fade_begin(&mut self, canvas: &Canvas) {
        let page = if self.sheet == Some(Sheet::Learn) {
            self.learn.state().page
        } else {
            0
        };
        let theme = self.settings.theme().theme();
        let drawn_key = ((self.screen, self.sheet), page, theme);
        // Drawn in full: what asked for this frame is drawn as it is now,
        // and the gap across it is not a fade's step.
        self.fade.due = false;
        self.fade.step_at = None;
        if self.fade.last_drawn == Some(drawn_key) {
            return;
        }
        let fades = self.fade.last_drawn.is_some() && self.fades();
        self.fade.theme_fade = self.fade.last_drawn.is_some_and(|(_, _, t)| t != theme);
        if self.fade.last_drawn.is_some() {
            let n = &mut self.fade.counts;
            match (fades, self.settings.reduce_motion()) {
                (true, _) => n.started += 1,
                (false, true) => n.reduced += 1,
                (false, false) => n.skipped += 1,
            }
        }
        // This frame is all the old one; the fade's clock starts at the
        // next tick (`Event::Tick`), however long this frame takes.
        self.fade.from = match (fades, self.fade.from.take()) {
            (false, _) => None,
            (true, Some((mut before, _))) => {
                canvas.snapshot_into(&mut before);
                Some((before, None))
            }
            (true, None) => Some((canvas.snapshot(), None)),
        };
        self.fade.last_drawn = Some(drawn_key);
    }

    /// After a frame is drawn in full on `canvas`: while a fade is under
    /// way, the new frame is kept aside and the canvas shows it part of
    /// the way from the old, and the frame asks for the next step, so the
    /// shell keeps drawing from the first one. Costs a blend of the whole
    /// canvas while a fade is under way.
    pub(crate) fn fade_end(&mut self, canvas: &mut Canvas) {
        let Some((before, at)) = &self.fade.from else {
            return;
        };
        let (t, over) = at.map_or((0.0, false), |at| self.fade.blend(Some(at), self.now_ms));
        if over {
            self.fade.from = None;
            self.fade.after = Vec::new();
        } else {
            canvas.swap_pixels(&mut self.fade.after);
            canvas.blend_between(before, &self.fade.after, t);
            self.fade.due = true;
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Draws the next step of the fade under way when nothing but the
    /// fade asked for a frame: the new frame kept aside when it was last
    /// drawn in full, blended with the old one, so a step costs a blend
    /// of the whole canvas, not a draw of the screen. Times the step
    /// from the one before it for [`Self::fades`].
    pub(crate) fn fade_step(&mut self) {
        self.fade.due = false;
        let (Some(canvas), Some((before, at))) = (self.canvas.as_mut(), self.fade.from.as_ref())
        else {
            return;
        };
        if let Some(last) = self.fade.step_at
            && self.now_ms > last
        {
            let gap = (self.now_ms - last).min(1000) as u16;
            if self.fade.gaps.len() == motion::FADE_GAPS_KEPT {
                self.fade.gaps.remove(0);
            }
            self.fade.gaps.push((self.now_ms, gap));
        }
        self.fade.step_at = Some(self.now_ms);
        self.fade.counts.steps += 1;
        let (t, over) = at.map_or((0.0, false), |at| self.fade.blend(Some(at), self.now_ms));
        if over {
            // The new frame at rest, as it was drawn.
            canvas.swap_pixels(&mut self.fade.after);
            self.fade.from = None;
            self.fade.after = Vec::new();
            self.fade.step_at = None;
        } else {
            canvas.blend_between(before, &self.fade.after, t);
            self.fade.due = true;
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Whether the next frame the shell asks for is a fade's step, a
    /// blend of the two kept frames, rather than a frame drawn in full.
    pub fn next_frame_blends(&self) -> bool {
        !self.out.dirty && self.fade.is_due()
    }

    /// What fades have done since this was last called, and the counts
    /// set back to nothing. Costs a sort of the few gaps kept.
    pub fn take_fade_counts(&mut self) -> FadeCounts {
        let mut gaps = self.fade.fresh_gaps(self.now_ms);
        gaps.sort_unstable();
        FadeCounts {
            gaps_known: gaps.len() as u8,
            median_gap_ms: gaps.get(gaps.len() / 2).copied(),
            on: self.fades(),
            ..std::mem::take(&mut self.fade.counts)
        }
    }

    /// Whether a change of screen or sheet fades in: not with reduce
    /// motion, and not on a machine whose last fades' steps came too far
    /// apart for a fade to read as one rather than as steps. Until a few
    /// steps have been timed lately, it does.
    pub(crate) fn fades(&self) -> bool {
        self.fade.fades(self.settings.reduce_motion(), self.now_ms)
    }
}
