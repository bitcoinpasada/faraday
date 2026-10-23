//! A change of screen or sheet cross-fading in (`docs/MOTION.md` §3.5):
//! when a frame shows a change, the old frame kept to fade from and the
//! new one kept aside; the steps after, which blend the two without
//! drawing either again; and whether this machine shows a fade's steps
//! close enough together for it to read as one.

use faraday_draw::Canvas;
use faraday_shell_api::Command;

use crate::{Faraday, Sheet, motion};

/// A change of screen or sheet fading in, kept on `Faraday`.
#[derive(Default)]
pub(crate) struct Fade {
    /// The frame before a change of screen or sheet, kept while the new
    /// one fades in over it, and when the fade's clock started: `None`
    /// for the frame that showed the change, whose cost (frosting the
    /// page under a sheet is the slowest frame there is) is not taken
    /// out of the fade.
    pub(crate) from: Option<(Vec<u8>, Option<u64>)>,
    /// The new frame of the fade under way, as last drawn in full, kept
    /// aside so that a step of the fade nothing else asked a frame for
    /// blends the two kept frames instead of drawing the screen again
    /// (`Faraday::fade_step`). Drawn again, and kept again, by every
    /// frame drawn in full while the fade runs; empty with no fade.
    after: Vec<u8>,
    /// The fade asked for its next step and nothing has asked for a
    /// frame drawn in full since (`dirty` is not set).
    pub(crate) due: bool,
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
    pub(crate) gaps: Vec<(u64, u16)>,
    /// What fades have done since a shell's timings last took it
    /// (`Faraday::take_fade_counts`).
    counts: FadeCounts,
}

impl Fade {
    /// The fade under way ends where it is: the next frame is the
    /// screen at rest.
    pub(crate) fn end(&mut self) {
        self.from = None;
        self.after = Vec::new();
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
            self.learn.page
        } else {
            0
        };
        let drawn_key = ((self.screen, self.sheet), page);
        // Drawn in full: what asked for this frame is drawn as it is now,
        // and the gap across it is not a fade's step.
        self.fade.due = false;
        self.fade.step_at = None;
        if self.last_drawn == Some(drawn_key) {
            return;
        }
        let fades = self.last_drawn.is_some() && self.fades();
        if self.last_drawn.is_some() {
            let n = &mut self.fade.counts;
            match (fades, self.reduce_motion) {
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
        self.last_drawn = Some(drawn_key);
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
        let t = at.map_or(0.0, |at| {
            motion::progress(Some(at), self.now_ms, motion::FADE_MS)
        });
        if t >= 1.0 {
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
        let t = at.map_or(0.0, |at| {
            motion::progress(Some(at), self.now_ms, motion::FADE_MS)
        });
        if t >= 1.0 {
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
        !self.out.dirty && self.fade.due
    }

    /// What fades have done since this was last called, and the counts
    /// set back to nothing. Costs a sort of the few gaps kept.
    pub fn take_fade_counts(&mut self) -> FadeCounts {
        let mut gaps = self.fresh_gaps();
        gaps.sort_unstable();
        FadeCounts {
            gaps_known: gaps.len() as u8,
            median_gap_ms: gaps.get(gaps.len() / 2).copied(),
            on: self.fades(),
            ..std::mem::take(&mut self.fade.counts)
        }
    }

    /// The gaps between fade steps recent enough to count, oldest first.
    fn fresh_gaps(&self) -> Vec<u16> {
        self.fade
            .gaps
            .iter()
            .filter(|(at, _)| self.now_ms.saturating_sub(*at) <= motion::FADE_GAPS_FRESH_MS)
            .map(|(_, gap)| *gap)
            .collect()
    }

    /// Whether a change of screen or sheet fades in: not with reduce
    /// motion, and not on a machine whose last fades' steps came too far
    /// apart for a fade to read as one rather than as steps. Until a few
    /// steps have been timed lately, it does.
    pub(crate) fn fades(&self) -> bool {
        if self.reduce_motion {
            return false;
        }
        let mut gaps = self.fresh_gaps();
        if gaps.len() < motion::FADE_GAPS_KNOWN {
            return true;
        }
        gaps.sort_unstable();
        f32::from(gaps[gaps.len() / 2]) <= motion::FADE_GAP_MS
    }
}
