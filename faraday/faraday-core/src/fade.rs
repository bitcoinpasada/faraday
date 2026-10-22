//! A change of screen or sheet cross-fading in (`docs/MOTION.md` §3.5):
//! when a frame shows a change, the old frame kept to fade from and the
//! new one kept aside; the steps after, which blend the two without
//! drawing either again; and whether this machine shows a fade's steps
//! close enough together for it to read as one.

use faraday_draw::Canvas;
use faraday_shell_api::Command;

use crate::{Faraday, Sheet, motion};

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
        self.fade_due = false;
        self.fade_step_at = None;
        if self.last_drawn == Some(drawn_key) {
            return;
        }
        // This frame is all the old one; the fade's clock starts at the
        // next tick (`Event::Tick`), however long this frame takes.
        self.fade = match (self.last_drawn.is_some() && self.fades(), self.fade.take()) {
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
        let Some((before, at)) = &self.fade else {
            return;
        };
        let t = at.map_or(0.0, |at| {
            motion::progress(Some(at), self.now_ms, motion::FADE_MS)
        });
        if t >= 1.0 {
            self.fade = None;
            self.fade_after = Vec::new();
        } else {
            canvas.swap_pixels(&mut self.fade_after);
            canvas.blend_between(before, &self.fade_after, t);
            self.fade_due = true;
            self.commands.push_back(Command::Draw);
        }
    }

    /// Draws the next step of the fade under way when nothing but the
    /// fade asked for a frame: the new frame kept aside when it was last
    /// drawn in full, blended with the old one, so a step costs a blend
    /// of the whole canvas, not a draw of the screen. Times the step
    /// from the one before it for [`Self::fades`].
    pub(crate) fn fade_step(&mut self) {
        self.fade_due = false;
        let (Some(canvas), Some((before, at))) = (self.canvas.as_mut(), self.fade.as_ref()) else {
            return;
        };
        if let Some(last) = self.fade_step_at
            && self.now_ms > last
        {
            let gap = (self.now_ms - last).min(1000) as u16;
            if self.frame_gaps.len() == motion::FADE_GAPS_KEPT {
                self.frame_gaps.remove(0);
            }
            self.frame_gaps.push((self.now_ms, gap));
        }
        self.fade_step_at = Some(self.now_ms);
        let t = at.map_or(0.0, |at| {
            motion::progress(Some(at), self.now_ms, motion::FADE_MS)
        });
        if t >= 1.0 {
            // The new frame at rest, as it was drawn.
            canvas.swap_pixels(&mut self.fade_after);
            self.fade = None;
            self.fade_after = Vec::new();
            self.fade_step_at = None;
        } else {
            canvas.blend_between(before, &self.fade_after, t);
            self.fade_due = true;
            self.commands.push_back(Command::Draw);
        }
    }

    /// Whether a change of screen or sheet fades in: not with reduce
    /// motion, and not on a machine whose last fades' steps came too far
    /// apart for a fade to read as one rather than as steps. Until a few
    /// steps have been timed lately, it does.
    pub(crate) fn fades(&self) -> bool {
        if self.reduce_motion {
            return false;
        }
        let mut gaps: Vec<u16> = self
            .frame_gaps
            .iter()
            .filter(|(at, _)| self.now_ms.saturating_sub(*at) <= motion::FADE_GAPS_FRESH_MS)
            .map(|(_, gap)| *gap)
            .collect();
        if gaps.len() < motion::FADE_GAPS_KNOWN {
            return true;
        }
        gaps.sort_unstable();
        f32::from(gaps[gaps.len() / 2]) <= motion::FADE_GAP_MS
    }
}
