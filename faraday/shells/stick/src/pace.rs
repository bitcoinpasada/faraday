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

/// What a pass costs, smoothed, and the step that holds it.
pub struct Pace {
    /// What recent passes that drew took, weighted towards the newest:
    /// `None` before the first.
    cost: Option<Duration>,
    frames: u32,
}

impl Pace {
    /// The grid at its finest, before any frame has been timed.
    pub const fn new() -> Pace {
        Pace {
            cost: None,
            frames: 1,
        }
    }

    /// A pass that drew took `took`, from its deadline to its frame
    /// written. One slow pass, such as the frame a sheet opens on, moves
    /// the smoothed cost a fifth of the way, so a single late frame does
    /// not change the step; a machine that is slow every frame does.
    pub fn drew(&mut self, took: Duration) {
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

    /// The time from one frame's deadline to the next.
    pub fn step(&self) -> Duration {
        FRAME * self.frames
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
