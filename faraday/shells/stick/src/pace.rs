//! The step of the frame grid, chosen from what frames cost on this
//! machine (`docs/MOTION.md` §3.2).
//!
//! A PC's firmware framebuffer has no vsync to pace to, so while
//! something moves the loop draws on a grid of its own. The Pi's waits
//! for the screen's vblank (`refresh.rs`): the writer thread measures
//! the refresh, the frame below is that length, and the loop puts its
//! deadlines on the vblank's grid; the step logic is the same. A grid
//! finer than the
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
const KEEP: f64 = 1.15;
/// The step is shortened only when a pass fits the shorter one with this
/// much room, so the step does not flip at the boundary.
const SHORTEN: f64 = 1.4;

/// With the refresh wait working, the step is judged by the screen, not
/// by the cost: how many of a level's last passes are looked at.
const WINDOW: u32 = 8;
/// A step is lengthened when this many of the last [`WINDOW`] passes
/// were late against the vblank: a machine or a screen that misses most
/// frames. One stall, such as a sheet opening, is one or two.
const COARSEN_AT: u32 = 5;
/// A step is shortened after this many passes in a row, each on time,
/// whose smoothed cost fits the shorter step with [`KEEP`] room: long
/// enough that a stall is not mistaken for recovery, and a cost that
/// only just fits stays on the step it is on instead of flipping.
const RECOVER_AFTER: u32 = 12;

/// How many of a fade's recent steps the median of their cost is taken
/// over.
const BLENDS_KEPT: usize = 5;

/// What a pass costs, smoothed, and the step that holds it.
pub struct Pace {
    /// What recent passes that drew took, weighted towards the newest:
    /// `None` before the first.
    cost: Option<Duration>,
    frames: u32,
    /// One refresh: [`FRAME`] until the writer measures the screen's.
    frame: Duration,
    /// What the last few passes whose frame was a fade's step took, two
    /// kept frames blended, oldest first: a fade is paced by the median
    /// of what its steps cost, not by the frame that showed the change,
    /// often the slowest there is, and not by one step held up.
    blends: Vec<Duration>,
    /// The last pass that drew was a fade's step.
    blended: bool,
    /// The refresh wait has measured the screen and works: steps are
    /// judged by lateness against the vblank.
    waits: bool,
    /// Whether the frame the next pass reports was late, from `saw`.
    late: bool,
    /// The step for full frames and for a fade's steps, with the wait on.
    full: Level,
    fade: Level,
    /// Counted for `--timings` since last taken.
    events: Events,
}

/// What happened since `--timings` last asked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// Passes whose frame was shown a vblank later than its step allowed.
    pub late: u64,
    /// Steps lengthened after repeated misses.
    pub coarsened: u64,
    /// Steps shortened after frames fit again.
    pub recovered: u64,
}

/// A step chosen under the wait: lateness over the last [`WINDOW`]
/// passes lengthens it, a run of passes that fit shortens it.
struct Level {
    frames: u32,
    /// The last passes, newest in bit 0: set when late.
    history: u32,
    cost: Option<Duration>,
    fitting: u32,
}

impl Level {
    const fn new(frames: u32) -> Level {
        Level {
            frames,
            history: 0,
            cost: None,
            fitting: 0,
        }
    }

    /// A pass took `took` and was `late` or not. `Some(true)` when the
    /// step was lengthened, `Some(false)` when shortened.
    fn observe(&mut self, took: Duration, late: bool, frame: Duration) -> Option<bool> {
        let took = took.min(frame * MOST);
        let cost = match self.cost {
            None => took,
            Some(c) => (c * 4 + took) / 5,
        };
        self.cost = Some(cost);
        self.history = (self.history << 1) | u32::from(late);
        let recent = (self.history & ((1 << WINDOW) - 1)).count_ones();
        if recent >= COARSEN_AT && self.frames < MOST {
            *self = Level {
                frames: self.frames + 1,
                cost: self.cost,
                ..Level::new(0)
            };
            return Some(true);
        }
        let finer_fits = self.frames > 1 && frame * (self.frames - 1) >= cost.mul_f64(KEEP);
        if finer_fits && !late {
            self.fitting += 1;
            if self.fitting >= RECOVER_AFTER {
                *self = Level {
                    frames: self.frames - 1,
                    cost: self.cost,
                    ..Level::new(0)
                };
                return Some(false);
            }
        } else {
            self.fitting = 0;
        }
        None
    }
}

impl Pace {
    /// The grid at its finest, before any frame has been timed.
    pub const fn new() -> Pace {
        Pace {
            cost: None,
            frames: 1,
            frame: FRAME,
            blends: Vec::new(),
            blended: false,
            waits: false,
            late: false,
            full: Level::new(1),
            fade: Level::new(1),
            events: Events {
                late: 0,
                coarsened: 0,
                recovered: 0,
            },
        }
    }

    /// Whether the refresh wait works (the writer has measured the
    /// screen's refresh). Steps are then judged by lateness against the
    /// vblank (`saw`); without it the margins [`KEEP`] and [`SHORTEN`]
    /// choose them from the cost.
    pub fn set_wait(&mut self, works: bool) {
        if self.waits != works {
            self.waits = works;
            self.full = Level::new(self.frames);
            self.fade = Level::new(1);
        }
    }

    /// The frame the next pass reports was shown `late`: a vblank after
    /// the one its step allowed.
    pub fn saw(&mut self, late: bool) {
        self.late = late;
    }

    /// What was counted since the last call, for `--timings`.
    pub fn take_events(&mut self) -> Events {
        std::mem::take(&mut self.events)
    }

    fn judged(&mut self, fade: bool, took: Duration) {
        let late = std::mem::take(&mut self.late);
        self.events.late += u64::from(late);
        let frame = self.frame;
        let level = if fade { &mut self.fade } else { &mut self.full };
        match level.observe(took, late, frame) {
            Some(true) => self.events.coarsened += 1,
            Some(false) => self.events.recovered += 1,
            None => {}
        }
    }

    /// A pass that drew took `took`, from its deadline to its frame
    /// written. One slow pass, such as the frame a sheet opens on, moves
    /// the smoothed cost a fifth of the way, so a single late frame does
    /// not change the step; a machine that is slow every frame does. A
    /// pass counts for no more than the coarsest step holds: any longer
    /// one chooses the same step, and counted in full it would keep the
    /// grid coarse for a second after one stall.
    pub fn drew(&mut self, took: Duration) {
        self.blended = false;
        let counted = took.min(self.frame * MOST);
        let cost = match self.cost {
            None => counted,
            Some(c) => (c * 4 + counted) / 5,
        };
        self.cost = Some(cost);
        if self.waits {
            self.judged(false, took);
            self.frames = self.full.frames;
            return;
        }
        let frame = self.frame;
        let holds = |frames: u32, room: f64| frame * frames >= cost.mul_f64(room);
        if !holds(self.frames, KEEP) {
            self.frames = (self.frames + 1..=MOST)
                .find(|&n| holds(n, KEEP))
                .unwrap_or(MOST);
        } else if self.frames > 1 && holds(self.frames - 1, SHORTEN) {
            self.frames -= 1;
        }
    }

    /// A pass whose frame was a fade's step took `took`. Fades are timed
    /// apart on every stick: the step after it holds the median of what
    /// the last few such steps cost, not what frames drawn in full cost.
    pub fn stepped_fade(&mut self, took: Duration) {
        if self.blends.len() == BLENDS_KEPT {
            self.blends.remove(0);
        }
        self.blends.push(took);
        self.blended = true;
        if self.waits {
            self.judged(true, took);
        }
    }

    /// The median of the last few fade steps' costs, when there are any.
    fn blend_cost(&self) -> Option<Duration> {
        let mut costs = self.blends.clone();
        costs.sort_unstable();
        costs.get(costs.len() / 2).copied()
    }

    /// The time from one frame's deadline to the next.
    pub fn step(&self) -> Duration {
        self.frame * self.frames()
    }

    /// The screen's refresh is `refresh`, as the writer thread measured
    /// it: a step is whole refreshes of that length.
    pub fn set_refresh(&mut self, refresh: Duration) {
        self.frame = refresh;
    }

    /// The step in whole frames: after a fade's step, the fewest that
    /// hold what such a step costs with room; else the grid's.
    pub fn frames(&self) -> u32 {
        if self.waits {
            return if self.blended {
                self.fade.frames
            } else {
                self.frames
            };
        }
        match self.blend_cost() {
            Some(cost) if self.blended => (1..=MOST)
                .find(|&n| self.frame * n >= cost.mul_f64(KEEP))
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
            pace.drew(ms(55));
        }
        assert_eq!(pace.step(), FRAME * 4, "55 ms a frame needs four");
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
    fn one_stalled_pass_among_quick_ones_is_worked_off_within_a_few_frames() {
        let mut pace = Pace::new();
        for _ in 0..20 {
            pace.drew(ms(4));
        }
        pace.drew(ms(500));
        for _ in 0..5 {
            pace.drew(ms(4));
        }
        assert_eq!(pace.step(), FRAME);
    }

    #[test]
    fn a_machine_that_stalls_every_frame_still_goes_to_the_coarsest_step() {
        let mut pace = Pace::new();
        for _ in 0..20 {
            pace.drew(ms(500));
        }
        assert_eq!(pace.step(), FRAME * 4);
    }

    #[test]
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
    fn a_screen_that_refreshes_slower_than_sixty_hertz_steps_in_its_own_refreshes() {
        let mut pace = Pace::new();
        let refresh = Duration::from_micros(20_000);
        pace.set_refresh(refresh);
        for _ in 0..20 {
            pace.drew(ms(4));
        }
        assert_eq!(pace.step(), refresh);
        for _ in 0..20 {
            pace.drew(ms(25));
        }
        assert_eq!(pace.step(), refresh * 2);
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
        // A pass moves down to one frame under about 11.9 ms and leaves
        // it above about 14.5 ms; 14 ms is between, and keeps the step it
        // is on, whichever side it came from.
        for _ in 0..20 {
            pace.drew(ms(20));
        }
        assert_eq!(pace.step(), FRAME * 2);
        for _ in 0..20 {
            pace.drew(ms(14));
        }
        assert_eq!(pace.step(), FRAME * 2, "14 ms stays on two from above");
        for _ in 0..20 {
            pace.drew(ms(9));
        }
        assert_eq!(pace.step(), FRAME, "9 ms fits one");
        for _ in 0..20 {
            pace.drew(ms(14));
        }
        assert_eq!(pace.step(), FRAME, "14 ms stays on one from below");
        for _ in 0..20 {
            pace.drew(ms(16));
        }
        assert_eq!(pace.step(), FRAME * 2, "16 ms no longer fits one");
        for _ in 0..20 {
            pace.drew(ms(11));
        }
        assert_eq!(pace.step(), FRAME, "11 ms fits one again");
    }

    /// A pace with the refresh wait working.
    fn waiting() -> Pace {
        let mut pace = Pace::new();
        pace.set_wait(true);
        pace
    }

    fn pass(pace: &mut Pace, took: u64, late: bool) {
        pace.saw(late);
        pace.drew(ms(took));
    }

    #[test]
    fn with_the_wait_frames_that_fit_stay_at_one_refresh_even_when_they_cost_nearly_one() {
        let mut pace = waiting();
        for _ in 0..60 {
            pass(&mut pace, 15, false);
        }
        assert_eq!(pace.step(), FRAME);
        assert_eq!(pace.take_events(), Events::default());
    }

    #[test]
    fn with_the_wait_one_stall_does_not_coarsen_and_costs_one_late_frame() {
        let mut pace = waiting();
        for _ in 0..20 {
            pass(&mut pace, 8, false);
        }
        pass(&mut pace, 300, true);
        for _ in 0..20 {
            pass(&mut pace, 8, false);
            assert_eq!(pace.step(), FRAME);
        }
        let events = pace.take_events();
        assert_eq!(events.late, 1);
        assert_eq!(events.coarsened, 0);
    }

    #[test]
    fn with_the_wait_a_run_of_misses_coarsens_and_fitting_frames_bring_it_back() {
        let mut pace = waiting();
        for _ in 0..8 {
            pass(&mut pace, 25, true);
        }
        assert_eq!(pace.step(), FRAME * 2);
        for _ in 0..40 {
            pass(&mut pace, 25, false);
        }
        assert_eq!(pace.step(), FRAME * 2, "25 ms still needs two refreshes");
        for _ in 0..40 {
            pass(&mut pace, 8, false);
        }
        assert_eq!(pace.step(), FRAME);
        let events = pace.take_events();
        assert_eq!((events.coarsened, events.recovered), (1, 1));
    }

    #[test]
    fn with_the_wait_a_screen_costing_about_one_refresh_does_not_flip_between_steps() {
        let mut pace = waiting();
        for _ in 0..8 {
            pass(&mut pace, 17, true);
        }
        assert_eq!(pace.step(), FRAME * 2);
        for _ in 0..200 {
            pass(&mut pace, 16, false);
        }
        assert_eq!(pace.step(), FRAME * 2, "16 ms is held on two from above");
        assert_eq!(pace.take_events().recovered, 0);
    }

    #[test]
    fn with_the_wait_a_fade_follows_its_own_misses_and_fitting() {
        let mut pace = waiting();
        for _ in 0..10 {
            pace.saw(false);
            pace.stepped_fade(ms(9));
            assert_eq!(pace.step(), FRAME);
        }
        for _ in 0..8 {
            pace.saw(true);
            pace.stepped_fade(ms(25));
        }
        assert_eq!(pace.step(), FRAME * 2);
        pass(&mut pace, 8, false);
        assert_eq!(
            pace.step(),
            FRAME,
            "frames drawn in full keep their own step"
        );
    }

    #[test]
    fn without_the_wait_lateness_is_ignored_and_the_cost_chooses_the_step() {
        let mut pace = Pace::new();
        for _ in 0..20 {
            pass(&mut pace, 4, true);
        }
        assert_eq!(pace.step(), FRAME);
        for _ in 0..20 {
            pass(&mut pace, 20, false);
        }
        assert_eq!(pace.step(), FRAME * 2);
        assert_eq!(pace.take_events(), Events::default());
    }
}
