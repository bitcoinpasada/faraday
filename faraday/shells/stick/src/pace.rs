//! The step of the frame grid, chosen from what frames cost on this
//! machine (`docs/MOTION.md` §3.2).
//!
//! The firmware framebuffer has no vsync to pace to, so while something
//! moves the loop draws on a grid of its own. A grid finer than the
//! machine can keep gives uneven frames: a step of one frame, then one
//! of two when a pass runs late, and the content stutters however exact
//! its positions. So the step is the fewest whole frames that hold a
//! pass with headroom: one on a machine that renders in a few
//! milliseconds, two (30 Hz) on one that takes twenty, up to four. The
//! positions are computed from the clock, so a coarser step moves the
//! content the same distance in the same time, in fewer, even moves.

use std::time::Duration;

/// One frame at 60 Hz: the finest step.
pub const FRAME: Duration = Duration::from_micros(16_667);
/// The coarsest step, in frames (15 Hz). A machine slower than that
/// draws as fast as it can.
const MOST: u32 = 4;
/// A pass must fit the step with this much room before the step is kept;
/// a step that no longer holds it is lengthened at once.
const KEEP: f64 = 1.25;
/// The step is shortened only when a pass fits the shorter one with this
/// much room, so the step does not flip at the boundary.
const SHORTEN: f64 = 1.6;

/// Whether a fade's steps are timed apart from frames drawn in full.
/// On the PC stick only, for now: the Pi keeps one grid for both.
const BLENDS_APART: bool = cfg!(target_arch = "x86_64");

/// How many of a fade's recent steps the median of their cost is taken
/// over.
const BLENDS_KEPT: usize = 5;

/// What a pass costs, smoothed, and the step that holds it.
pub struct Pace {
    /// What recent passes that drew took, weighted towards the newest:
    /// `None` before the first.
    cost: Option<Duration>,
    frames: u32,
    /// What the last few passes whose frame was a fade's step took, two
    /// kept frames blended, oldest first: a fade is paced by the median
    /// of what its steps cost, not by the frame that showed the change,
    /// often the slowest there is, and not by one step held up.
    blends: Vec<Duration>,
    /// The last pass that drew was a fade's step.
    blended: bool,
}

impl Pace {
    /// The grid at its finest, before any frame has been timed.
    pub const fn new() -> Pace {
        Pace {
            cost: None,
            frames: 1,
            blends: Vec::new(),
            blended: false,
        }
    }

    /// A pass that drew took `took`, from its deadline to its frame
    /// written. One slow pass, such as the frame a sheet opens on, moves
    /// the smoothed cost a fifth of the way, so a single late frame does
    /// not change the step; a machine that is slow every frame does.
    pub fn drew(&mut self, took: Duration) {
        self.blended = false;
        let cost = match self.cost {
            None => took,
            Some(c) => (c * 4 + took) / 5,
        };
        self.cost = Some(cost);
        let holds = |frames: u32, room: f64| FRAME * frames >= cost.mul_f64(room);
        if !holds(self.frames, KEEP) {
            self.frames = (self.frames + 1..=MOST)
                .find(|&n| holds(n, KEEP))
                .unwrap_or(MOST);
        } else if self.frames > 1 && holds(self.frames - 1, SHORTEN) {
            self.frames -= 1;
        }
    }

    /// A pass whose frame was a fade's step took `took`. Where fades are
    /// timed apart, the step after it holds the median of what the last
    /// few such steps cost; elsewhere it counts as any pass that drew.
    pub fn stepped_fade(&mut self, took: Duration) {
        if !BLENDS_APART {
            self.drew(took);
            return;
        }
        if self.blends.len() == BLENDS_KEPT {
            self.blends.remove(0);
        }
        self.blends.push(took);
        self.blended = true;
    }

    /// The median of the last few fade steps' costs, when there are any.
    fn blend_cost(&self) -> Option<Duration> {
        let mut costs = self.blends.clone();
        costs.sort_unstable();
        costs.get(costs.len() / 2).copied()
    }

    /// The time from one frame's deadline to the next.
    pub fn step(&self) -> Duration {
        FRAME * self.frames()
    }

    /// The step in whole frames: after a fade's step, the fewest that
    /// hold what such a step costs with room; else the grid's.
    pub fn frames(&self) -> u32 {
        match self.blend_cost() {
            Some(cost) if self.blended => (1..=MOST)
                .find(|&n| FRAME * n >= cost.mul_f64(KEEP))
                .unwrap_or(MOST),
            _ => self.frames,
        }
    }

    /// The smoothed cost the step was chosen from, for `--timings`.
    pub fn cost(&self) -> Duration {
        self.cost.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_quick_machine_draws_every_frame_and_a_slow_one_every_other() {
        let mut pace = Pace::new();
        assert_eq!(pace.step(), FRAME);
        for _ in 0..20 {
            pace.drew(ms(4));
        }
        assert_eq!(pace.step(), FRAME, "4 ms a frame fits one frame");
        for _ in 0..20 {
            pace.drew(ms(20));
        }
        assert_eq!(pace.step(), FRAME * 2, "20 ms a frame needs two");
        for _ in 0..20 {
            pace.drew(ms(45));
        }
        assert_eq!(pace.step(), FRAME * 4, "45 ms a frame needs four");
        for _ in 0..20 {
            pace.drew(ms(200));
        }
        assert_eq!(pace.step(), FRAME * 4, "and no step is longer");
        for _ in 0..40 {
            pace.drew(ms(4));
        }
        assert_eq!(pace.step(), FRAME, "quick again, the grid is fine again");
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn a_fade_steps_at_what_its_steps_cost_not_at_the_frame_that_showed_it() {
        let mut pace = Pace::new();
        // Screens drawn in full at 20 ms a frame: a step of two frames.
        for _ in 0..20 {
            pace.drew(ms(20));
        }
        assert_eq!(pace.step(), FRAME * 2);
        // A sheet opens on a frame of 40 ms, and its fade's steps blend
        // in 10: each comes a frame after the last.
        pace.drew(ms(40));
        for _ in 0..5 {
            pace.stepped_fade(ms(10));
            assert_eq!(pace.step(), FRAME);
        }
        // One step held up, a write to the stick, does not stretch the
        // steps after it.
        pace.stepped_fade(ms(120));
        assert_eq!(pace.step(), FRAME);
        // The fade over, frames drawn in full keep their own step.
        pace.drew(ms(20));
        assert_eq!(pace.step(), FRAME * 2);
    }

    #[test]
    fn one_slow_frame_does_not_change_the_step() {
        let mut pace = Pace::new();
        for _ in 0..20 {
            pace.drew(ms(4));
        }
        // The frame a sheet opens on: forty milliseconds, once.
        pace.drew(ms(40));
        assert_eq!(pace.step(), FRAME);
        pace.drew(ms(4));
        assert_eq!(pace.step(), FRAME);
    }

    #[test]
    fn a_cost_at_the_boundary_does_not_flip_the_step() {
        let mut pace = Pace::new();
        // 13 ms with the room asked for is just under one frame; 14 ms
        // just over. A machine wavering between them keeps one step.
        for _ in 0..20 {
            pace.drew(ms(14));
        }
        assert_eq!(pace.step(), FRAME * 2);
        for _ in 0..20 {
            pace.drew(ms(13));
        }
        assert_eq!(
            pace.step(),
            FRAME * 2,
            "13 ms does not fit one frame with room to spare"
        );
        for _ in 0..20 {
            pace.drew(ms(9));
        }
        assert_eq!(pace.step(), FRAME, "9 ms does");
    }
}
