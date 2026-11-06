//! The screen's refresh, as the writer thread sees it: the wait for the
//! vertical blank, the refresh length it measures, and the frame grid
//! tied to it (`docs/FRAMES.md` §6, `docs/MOTION.md` §3.2).
//!
//! The writer thread waits for a vblank before it writes a frame
//! ([`Vblank`]) and tells [`Beat`] when the wait returned. The main loop
//! reads the last vblank and the refresh length from the same place and
//! puts its next deadline on the refresh's own grid ([`Grid::align`]),
//! so a frame is ready just before a vblank instead of at a moment of
//! its own. Any error from the wait, or waits that do not fall on a
//! grid of refreshes, turn the wait off for the run; the loop then keeps
//! its own timer as on a framebuffer without the call.

use std::fs::File;
use std::io;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::pace::FRAME;

/// A wait for the screen's next vertical blank.
pub trait Vblank: Send {
    /// Returns when a vblank has come, or soon after.
    fn wait(&mut self) -> io::Result<()>;
}

/// The framebuffer's own wait: a handle of its own on the device.
pub struct FbVblank(pub File);

impl Vblank for FbVblank {
    fn wait(&mut self) -> io::Result<()> {
        faraday_v4l2::wait_vblank(&self.0)
    }
}

/// Waits further apart than this many refreshes are a pause in the
/// motion, not a run of frames.
const RUN: f64 = 8.0;
/// A wait is off the grid when it falls further than this part of a
/// refresh from a whole number of refreshes after the one before.
const OFF_GRID: f64 = 0.25;
/// This many waits in a row off the grid turn the wait off.
const BAD_WAITS: u32 = 8;
/// Waits that are not about one refresh apart are counted unjudged this
/// many times before the refresh is learned.
const UNLEARNED: u32 = 8;
/// A vblank older than this is no reference for the grid.
const STALE: Duration = Duration::from_secs(2);
/// The shortest and longest refresh the loop's grid takes.
const SHORTEST: Duration = Duration::from_millis(8);
const LONGEST: Duration = Duration::from_millis(34);

/// What the waits came to since the last block of `--timings`.
#[derive(Default)]
struct Tally {
    /// Refreshes that passed while frames were being written.
    seen: u64,
    /// Of them, the ones a frame was written after.
    carried: u64,
    /// Refreshes a frame was due on, by the grid's step, that carried
    /// none.
    missed: u64,
    /// The measured refresh lengths: how many, their sum, least, most.
    lengths: u64,
    length_sum: Duration,
    least: Option<Duration>,
    most: Duration,
    /// Writes finished: how many, the time from vblank to the write's
    /// end summed, the longest.
    writes: u64,
    write_sum: Duration,
    write_worst: Duration,
}

struct State {
    on: bool,
    /// Why the wait is off.
    why: String,
    /// The last vblank, and the refresh length smoothed.
    last: Option<Instant>,
    refresh: Option<Duration>,
    /// The grid's step in frames, as the loop last set it.
    step: u32,
    bad: u32,
    /// Waits too far apart to learn the refresh from, before it was
    /// learned.
    unlearned: u32,
    tally: Tally,
}

/// The refresh as measured; shared by the writer thread and the loop.
pub struct Beat(Mutex<State>);

/// The last vblank and the refresh length: what the loop's grid is
/// tied to.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    pub vblank: Instant,
    pub refresh: Duration,
}

fn plus(t: Instant, secs: f64) -> Instant {
    if secs >= 0.0 {
        t + Duration::from_secs_f64(secs)
    } else {
        t - Duration::from_secs_f64(-secs)
    }
}

fn since(a: Instant, b: Instant) -> f64 {
    if a >= b {
        a.duration_since(b).as_secs_f64()
    } else {
        -b.duration_since(a).as_secs_f64()
    }
}

impl Grid {
    /// `deadline` moved to the nearest moment `lead` before a vblank of
    /// the grid, and past `now`: a pass that starts then ends just ahead
    /// of the vblank its frame is written after. The step between
    /// deadlines stays whole refreshes of the measured length, so the
    /// grid does not drift against the screen whatever the length is.
    pub fn align(&self, deadline: Instant, lead: Duration, now: Instant) -> Instant {
        let r = self.refresh.as_secs_f64();
        let lead = lead.as_secs_f64();
        let at = since(deadline, self.vblank) + lead;
        let moved = (at / r).round() * r - at;
        let mut t = plus(deadline, moved);
        if t <= now {
            let late = since(now, t);
            t = plus(t, ((late / r).floor() + 1.0) * r);
        }
        t
    }
}

impl Beat {
    /// A beat with the wait on.
    pub fn new() -> Beat {
        Beat(Mutex::new(State {
            on: true,
            why: String::new(),
            last: None,
            refresh: None,
            step: 1,
            bad: 0,
            unlearned: 0,
            tally: Tally::default(),
        }))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Whether the writer still waits.
    pub fn on(&self) -> bool {
        self.state().on
    }

    /// Turns the wait off for the run, and says why.
    pub fn off(&self, why: &str) {
        let mut st = self.state();
        if st.on {
            st.on = false;
            st.why = why.to_owned();
        }
    }

    /// The loop's grid step in frames, for counting what was missed.
    pub fn set_step(&self, frames: u32) {
        self.state().step = frames.max(1);
    }

    /// The last vblank and the refresh length, while the wait is on and
    /// the vblank is recent.
    pub fn grid(&self, now: Instant) -> Option<Grid> {
        let st = self.state();
        let vblank = st
            .last
            .filter(|v| st.on && now.saturating_duration_since(*v) < STALE)?;
        Some(Grid {
            vblank,
            refresh: st.refresh.unwrap_or(FRAME).clamp(SHORTEST, LONGEST),
        })
    }

    /// A wait returned at `at`.
    pub fn vblank(&self, at: Instant) {
        let mut st = self.state();
        let r = st.refresh.unwrap_or(FRAME);
        let step = u64::from(st.step);
        let mut seen = 1;
        if let Some(prev) = st.last {
            let gap = at.saturating_duration_since(prev);
            let g = gap.as_secs_f64();
            // Before the refresh is known, only a wait about one nominal
            // refresh after the last is taken as a measure of it; the
            // others are counted against the nominal length, unjudged,
            // for a few waits.
            let judged = st.refresh.is_some()
                || st.unlearned >= UNLEARNED
                || (0.6 * r.as_secs_f64()..1.5 * r.as_secs_f64()).contains(&g);
            if !judged {
                st.unlearned += 1;
                seen = (g / r.as_secs_f64()).round().max(1.0) as u64;
            } else if g <= RUN * r.as_secs_f64() {
                let n = (gap.as_secs_f64() / r.as_secs_f64()).round().max(1.0);
                let off = (gap.as_secs_f64() - n * r.as_secs_f64()).abs();
                if off > OFF_GRID * r.as_secs_f64() {
                    st.bad += 1;
                    if st.bad >= BAD_WAITS && st.on {
                        st.on = false;
                        st.why = "waits off the refresh".to_owned();
                    }
                    st.last = Some(at);
                    return;
                }
                st.bad = 0;
                seen = n as u64;
                let one = gap.div_f64(n);
                let smooth = match st.refresh {
                    Some(old) if n < 5.0 => (old * 7 + one) / 8,
                    Some(old) => old,
                    None => one,
                };
                if n < 5.0 {
                    st.refresh = Some(smooth);
                    st.tally.lengths += 1;
                    st.tally.length_sum += one;
                    st.tally.least = Some(st.tally.least.map_or(one, |l| l.min(one)));
                    st.tally.most = st.tally.most.max(one);
                }
            }
        }
        st.tally.seen += seen;
        st.tally.carried += 1;
        st.tally.missed += seen.saturating_sub(step);
        st.last = Some(at);
    }

    /// The write that followed the vblank at `vblank` ended now.
    pub fn written(&self, vblank: Instant) {
        let took = vblank.elapsed();
        let mut st = self.state();
        st.tally.writes += 1;
        st.tally.write_sum += took;
        st.tally.write_worst = st.tally.write_worst.max(took);
    }

    /// The lines for a block of `--timings`, and the tally emptied.
    pub fn report(&self) -> String {
        let mut st = self.state();
        let t = std::mem::take(&mut st.tally);
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        let wait = if st.on {
            "on".to_owned()
        } else {
            format!("off ({})", st.why)
        };
        let mut out = format!(
            "vblank   wait {wait} seen {} carried {} missed {} carried_pct {}\n",
            t.seen,
            t.carried,
            t.missed,
            (t.carried * 100)
                .checked_div(t.seen)
                .map_or("-".to_owned(), |p| p.to_string()),
        );
        if t.lengths > 0 {
            out.push_str(&format!(
                "refresh  {:>7} {:>9.2} {:>9.2} {:>9.2}\n",
                t.lengths,
                ms(t.length_sum) / t.lengths as f64,
                ms(t.least.unwrap_or_default()),
                ms(t.most),
            ));
        }
        if t.writes > 0 {
            out.push_str(&format!(
                "vb_write {:>7} {:>9.2} {:>9.2}\n",
                t.writes,
                ms(t.write_sum) / t.writes as f64,
                ms(t.write_worst),
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: f64) -> Duration {
        Duration::from_secs_f64(n / 1000.0)
    }

    #[test]
    fn a_grid_on_a_longer_refresh_keeps_its_phase_with_the_screen() {
        let base = Instant::now();
        let r = ms(17.2);
        let grid = Grid {
            vblank: base,
            refresh: r,
        };
        let lead = ms(5.0);
        let now = base + ms(1000.0);
        let mut d = grid.align(now, lead, now);
        let first = d;
        for k in 1..=600 {
            d = grid.align(d + r, lead, d);
            let drift = since(d, first) - f64::from(k) * r.as_secs_f64();
            assert!(drift.abs() < 1e-6, "{k} frames on, off by {drift} s");
        }
        // Every deadline is the lead before a vblank.
        let into = since(d + lead, base) / r.as_secs_f64();
        assert!((into - into.round()).abs() < 1e-3);
    }

    #[test]
    fn a_pass_that_ran_late_waits_for_the_next_vblank_not_a_moment_of_its_own() {
        let base = Instant::now();
        let grid = Grid {
            vblank: base,
            refresh: ms(16.667),
        };
        let now = base + ms(100.0);
        let d = grid.align(now + ms(16.667), ms(4.0), now);
        let at = since(d + ms(4.0), base) / 0.016_667;
        assert!(d > now && (at - at.round()).abs() < 1e-3);
    }

    #[test]
    fn waits_a_whole_refresh_apart_measure_the_refresh_and_count_what_carried() {
        let beat = Beat::new();
        let base = Instant::now();
        // 20 ms refreshes; frames written on every other one.
        for k in 0..5 {
            beat.vblank(base + ms(20.0 * f64::from(k)));
        }
        for k in 0..20 {
            beat.vblank(base + ms(80.0 + 40.0 * f64::from(k)));
        }
        let g = beat.grid(base + ms(900.0)).expect("a grid");
        assert!((g.refresh.as_secs_f64() - 0.020).abs() < 0.002);
        beat.set_step(2);
        let text = beat.report();
        assert!(text.contains("wait on"), "{text}");
        assert!(text.contains("carried_pct"), "{text}");
    }

    #[test]
    fn waits_that_fall_off_the_grid_again_and_again_turn_the_wait_off() {
        let beat = Beat::new();
        let base = Instant::now();
        let mut t = 0.0;
        for k in 0..20 {
            t += if k % 2 == 0 { 7.0 } else { 9.0 };
            beat.vblank(base + ms(t));
        }
        assert!(!beat.on());
        assert!(beat.grid(base + ms(t)).is_none());
        assert!(beat.report().contains("off ("));
    }
}
