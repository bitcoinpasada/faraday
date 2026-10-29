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
    /// is drawn on, never count; their median is what turns fades off,
    /// and gaps older than `motion::FADE_GAPS_FRESH_MS` no longer count.
    /// Emptied when fades turn off or back on.
    gaps: Vec<(u64, u16)>,
    /// When the gaps turned fades off; `None` while they are on. Off,
    /// they stay off at least `motion::FADE_HOLD_MS` and come back only
    /// after `motion::FADE_ON_RUN` frames in a row each within
    /// `motion::FADE_GAP_MS` of the one before (`Fade::shown`): the gate
    /// turns on and off on different evidence, so it cannot flap.
    off: Option<u64>,
    /// While fades are off, the frames in a row drawn within
    /// `motion::FADE_GAP_MS` of the one before.
    run: u16,
    /// The tick of the last frame drawn or blended, for the gap to the
    /// next.
    shown_at: Option<u64>,
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
        self.off = None;
        self.run = 0;
        self.shown_at = None;
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
    /// while the gaps between fades' steps have turned fades off
    /// ([`Fade::step_timed`]). Until a few steps have been timed, it does.
    fn fades(&self, reduce_motion: bool) -> bool {
        !reduce_motion && self.off.is_none()
    }

    /// A step of a fade came `gap` ms after the one before it, at
    /// `now_ms`: kept with the last few, and fades turned off where the
    /// median of those still fresh, once a few are known, is wider than
    /// `motion::FADE_GAP_MS`. A sort of the few gaps kept.
    fn step_timed(&mut self, now_ms: u64, gap: u16) {
        if self.gaps.len() == motion::FADE_GAPS_KEPT {
            self.gaps.remove(0);
        }
        self.gaps.push((now_ms, gap));
        if self.off.is_some() {
            return;
        }
        let mut gaps = self.fresh_gaps(now_ms);
        if gaps.len() < motion::FADE_GAPS_KNOWN {
            return;
        }
        gaps.sort_unstable();
        if f32::from(gaps[gaps.len() / 2]) > motion::FADE_GAP_MS {
            self.off = Some(now_ms);
            self.run = 0;
            self.gaps.clear();
        }
    }

    /// A frame was drawn or blended at `now_ms`. While fades are off, its
    /// gap from the one before counts towards turning them back on: within
    /// `motion::FADE_GAP_MS` it lengthens the run of fast frames, wider
    /// breaks it, and a pause (over `motion::FADE_PAUSE_MS`) does neither;
    /// a run of `motion::FADE_ON_RUN` once fades have been off
    /// `motion::FADE_HOLD_MS` turns them on again.
    pub(crate) fn shown(&mut self, now_ms: u64) {
        let last = self.shown_at.replace(now_ms);
        let Some(since) = self.off else {
            return;
        };
        let Some(gap) = last.map(|l| now_ms.saturating_sub(l)).filter(|&g| g > 0) else {
            return;
        };
        if gap > motion::FADE_PAUSE_MS {
            return;
        }
        if gap as f32 > motion::FADE_GAP_MS {
            self.run = 0;
            return;
        }
        self.run = self.run.saturating_add(1);
        if self.run >= motion::FADE_ON_RUN && now_ms.saturating_sub(since) >= motion::FADE_HOLD_MS {
            self.off = None;
            self.run = 0;
            self.gaps.clear();
        }
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
            self.learn.state().tab()
        } else {
            0
        };
        let theme = self.settings.theme();
        let drawn_key = ((self.screen, self.sheet), page, theme);
        // Drawn in full: what asked for this frame is drawn as it is now,
        // and the gap across it is not a fade's step.
        self.fade.due = false;
        self.fade.step_at = None;
        if self.fade.last_drawn == Some(drawn_key) {
            return;
        }
        let own_motion = false;
        let fades = self.fade.last_drawn.is_some() && self.fades() && !own_motion;
        self.fade.theme_fade = self.fade.last_drawn.is_some_and(|(_, _, t)| t != theme);
        if self.fade.last_drawn.is_some() && !own_motion {
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
    /// canvas while a fade is under way. The frame is timed for the gate
    /// (`Fade::shown`).
    pub(crate) fn fade_end(&mut self, canvas: &mut Canvas) {
        self.fade.shown(self.now_ms);
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
        self.fade.shown(self.now_ms);
        if self.canvas.is_none() || self.fade.from.is_none() {
            return;
        }
        if let Some(last) = self.fade.step_at
            && self.now_ms > last
        {
            let gap = (self.now_ms - last).min(1000) as u16;
            self.fade.step_timed(self.now_ms, gap);
        }
        let (Some(canvas), Some((before, at))) = (self.canvas.as_mut(), self.fade.from.as_ref())
        else {
            return;
        };
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

    /// Whether the next frame asked for changes the app's pixels: drawn in
    /// full, or a fade's step. Otherwise the canvas is shown as it is.
    pub(crate) fn paints_next(&self) -> bool {
        self.out.dirty || self.fade.is_due()
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
    /// motion, and not on a machine whose fades' steps lately came too
    /// far apart for a fade to read as one rather than as steps, until it
    /// has shown a run of frames fast enough again (`Fade::shown`).
    /// Until a few steps have been timed, it does.
    pub(crate) fn fades(&self) -> bool {
        self.fade.fades(self.settings.reduce_motion())
    }
}

#[cfg(test)]
mod tests {
    use super::Fade;
    use crate::motion;

    /// A fade's steps `gap` ms apart after `now`, each one shown; returns
    /// the time of the last.
    fn fade_steps(f: &mut Fade, mut now: u64, gap: u64, steps: usize) -> u64 {
        for _ in 0..steps {
            now += gap;
            f.step_timed(now, gap as u16);
            f.shown(now);
        }
        now
    }

    /// `n` frames drawn `gap` ms apart after `now`, none a fade's step;
    /// returns the time of the last.
    fn frames(f: &mut Fade, mut now: u64, gap: u64, n: u16) -> u64 {
        for _ in 0..n {
            now += gap;
            f.shown(now);
        }
        now
    }

    /// Fades turned off by a fade whose steps came 50 ms apart; returns
    /// when they turned off, at the fifth step, and when it ended.
    fn slow_fade(f: &mut Fade) -> (u64, u64) {
        let end = fade_steps(f, 1_000, 50, 9);
        assert!(!f.fades(false), "a slow fade turns fades off");
        (1_250, end)
    }

    #[test]
    fn a_run_of_slow_steps_turns_fades_off() {
        let mut f = Fade::default();
        assert!(f.fades(false), "on until a few steps are timed");
        slow_fade(&mut f);
    }

    #[test]
    fn frames_at_a_screens_refresh_never_turn_fades_off() {
        // At 60 Hz with a refresh missed now and then, and at 30 Hz:
        // fades of nine steps, two seconds apart, for an hour.
        let paces: [&[u64]; 2] = [&[17, 16, 17, 33, 17, 16, 17, 17, 16], &[33, 34, 33]];
        for gaps in paces {
            let mut f = Fade::default();
            let mut now = 1_000;
            for i in 0..16_200 {
                now = fade_steps(&mut f, now, gaps[i % gaps.len()], 1);
                if i % 9 == 8 {
                    now += 2_000;
                }
            }
            assert!(f.fades(false), "steps {gaps:?} ms apart leave fades on");
        }
    }

    #[test]
    fn fades_turned_off_do_not_come_back_on_one_fast_frame() {
        let mut f = Fade::default();
        let (off, _) = slow_fade(&mut f);
        // The first frame after a pause, then one fast one.
        let now = frames(&mut f, off + motion::FADE_HOLD_MS, 16, 2);
        assert!(!f.fades(false), "one fast frame");
        let now = frames(&mut f, now, 16, 5);
        let now = frames(&mut f, now, 50, 1);
        assert!(!f.fades(false), "a short run, broken by a slow frame");
        frames(&mut f, now, 16, motion::FADE_ON_RUN - 1);
        assert!(!f.fades(false), "a run a frame short");
    }

    #[test]
    fn fades_stay_off_a_while_however_fast_the_frames() {
        let mut f = Fade::default();
        let (off, end) = slow_fade(&mut f);
        // Every refresh from the fade's end to just before the hold's.
        let n = ((off + motion::FADE_HOLD_MS - end) / 16 - 1) as u16;
        frames(&mut f, end, 16, n);
        assert!(!f.fades(false), "held off {} ms", motion::FADE_HOLD_MS);
    }

    #[test]
    fn fades_come_back_after_a_sustained_run_of_fast_frames() {
        let mut f = Fade::default();
        let (off, _) = slow_fade(&mut f);
        // Nothing drawn until the hold is over; then a frame after that
        // pause, a run, a pause in it, which neither counts nor breaks
        // it, and the rest of it.
        let half = motion::FADE_ON_RUN / 2;
        let now = frames(&mut f, off + motion::FADE_HOLD_MS, 16, half + 1);
        let now = frames(&mut f, now + 5_000, 16, motion::FADE_ON_RUN - half);
        assert!(!f.fades(false), "a frame short of the run");
        let now = frames(&mut f, now, 16, 1);
        assert!(f.fades(false), "back on after the run");
        // A slow fade after turns them off again, judged on its own
        // steps alone.
        let now = fade_steps(&mut f, now + 1_000, 50, 4);
        assert!(f.fades(false), "four steps are too few to judge");
        fade_steps(&mut f, now, 50, 1);
        assert!(!f.fades(false));
    }

    #[test]
    fn with_reduce_motion_nothing_fades() {
        assert!(!Fade::default().fades(true));
    }
}
