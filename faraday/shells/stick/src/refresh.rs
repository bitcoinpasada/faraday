//! The screen's refresh, as the writer thread sees it: the wait for the
//! vertical blank, the refresh length it measures once, and the frame grid
//! tied to it (`docs/FRAMES.md` §6, `docs/MOTION.md` §3.2).
//!
//! The writer thread first waits for sixteen vblanks back to back
//! ([`measure`]); the median gap is the refresh for the run. After that it
//! waits for a vblank before it writes a frame ([`Vblank`]) and tells
//! [`Beat`] when the wait returned; those gaps only count refreshes and
//! never change the length. The main loop
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
/// A vblank older than this is no reference for the grid.
const STALE: Duration = Duration::from_secs(2);
/// The shortest and longest refresh the loop's grid takes.
const SHORTEST: Duration = Duration::from_millis(8);
const LONGEST: Duration = Duration::from_millis(34);
/// Gaps between back-to-back waits taken to measure the refresh.
const SAMPLES: usize = 16;
/// The measured gaps show a steady beat when at least this many of them
/// are within [`OFF_GRID`] of the median.
const STEADY: usize = SAMPLES / 2;

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
    /// The last vblank, and the refresh length measured at the start.
    last: Option<Instant>,
    refresh: Option<Duration>,
    /// How the refresh was measured: waits taken, least gap, most gap.
    basis: Option<(usize, Duration, Duration)>,
    /// The grid's step in frames, as the loop last set it.
    step: u32,
    bad: u32,
    /// Frames written a vblank later than the step allowed, not yet
    /// taken by the loop.
    late: u64,
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
            basis: None,
            step: 1,
            bad: 0,
            late: 0,
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

    /// Whether the wait is on and has measured the screen's refresh: only
    /// then does a frame's lateness against the vblank say anything.
    pub fn measured(&self) -> bool {
        let st = self.state();
        st.on && st.refresh.is_some()
    }

    /// How many frames were written a vblank later than the step allowed
    /// since the last call.
    pub fn take_late(&self) -> u64 {
        std::mem::take(&mut self.state().late)
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

    /// The refresh as the loop uses it: the measured length while the
    /// wait is on, otherwise the nominal one.
    pub fn length(&self) -> Duration {
        let st = self.state();
        match st.refresh {
            Some(r) if st.on => r.clamp(SHORTEST, LONGEST),
            _ => FRAME,
        }
    }

    /// The refresh is `gaps` apart at their median, the last vblank was
    /// `last`: the length for the run, or the wait off when the median is
    /// outside the range the grid takes or the gaps show no steady beat.
    fn learn(&self, gaps: &[Duration], last: Instant) {
        let mut sorted = gaps.to_vec();
        sorted.sort();
        let Some(&median) = sorted.get(sorted.len() / 2) else {
            return self.off("no waits to measure");
        };
        if !(SHORTEST..=LONGEST).contains(&median) {
            return self.off(&format!(
                "refresh of {:.1} ms is out of range",
                median.as_secs_f64() * 1000.0
            ));
        }
        let m = median.as_secs_f64();
        let steady = sorted
            .iter()
            .filter(|g| (g.as_secs_f64() - m).abs() <= OFF_GRID * m)
            .count();
        if steady < STEADY {
            return self.off("no steady beat");
        }
        let mut st = self.state();
        st.refresh = Some(median);
        st.basis = Some((sorted.len(), sorted[0], sorted[sorted.len() - 1]));
        st.last = Some(last);
    }

    /// A wait returned at `at`. The gap from the one before only counts
    /// refreshes; the length stays as measured.
    pub fn vblank(&self, at: Instant) {
        let mut st = self.state();
        let r = st.refresh.unwrap_or(FRAME).as_secs_f64();
        let step = u64::from(st.step);
        let mut seen = 1;
        if let Some(prev) = st.last {
            let g = at.saturating_duration_since(prev).as_secs_f64();
            if g <= RUN * r {
                let n = (g / r).round().max(1.0);
                if (g - n * r).abs() > OFF_GRID * r {
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
            }
        }
        st.tally.seen += seen;
        st.tally.carried += 1;
        st.tally.missed += seen.saturating_sub(step);
        st.late += u64::from(seen > step);
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
        if let (Some(r), Some((n, least, most))) = (st.refresh, st.basis) {
            out.push_str(&format!(
                "refresh  {:>7.2} ms, median of {n} back-to-back waits at the start (least {:.2}, most {:.2})\n",
                ms(r),
                ms(least),
                ms(most),
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

/// Measures the refresh once: waits back to back, `SAMPLES` gaps, and
/// gives the median to `beat`. A failed wait turns the wait off. `clock`
/// says when each wait returned.
pub fn measure(v: &mut dyn Vblank, beat: &Beat, clock: &mut dyn FnMut() -> Instant) {
    let mut times = Vec::with_capacity(SAMPLES + 1);
    for _ in 0..=SAMPLES {
        if let Err(e) = v.wait() {
            return beat.off(&format!("error: {e}"));
        }
        times.push(clock());
    }
    let gaps: Vec<Duration> = times
        .windows(2)
        .map(|w| w[1].saturating_duration_since(w[0]))
        .collect();
    beat.learn(&gaps, times[SAMPLES]);
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

    /// A wait that returns at once; the clock says when it did.
    struct Instant0;

    impl Vblank for Instant0 {
        fn wait(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A beat that measured waits returning at `times` ms after the start.
    fn measured(times: &[f64]) -> (Beat, Instant) {
        let beat = Beat::new();
        let base = Instant::now();
        let mut at = times.iter();
        measure(&mut Instant0, &beat, &mut || {
            base + ms(*at.next().expect("a time"))
        });
        (beat, base)
    }

    /// 17 returns of a refresh `r` ms, `late` ms added at the given ones.
    fn beat_of(r: f64, late: &[(usize, f64)]) -> Vec<f64> {
        (0..=SAMPLES)
            .map(|k| {
                r * k as f64
                    + late
                        .iter()
                        .filter(|(i, _)| *i == k)
                        .map(|(_, l)| l)
                        .sum::<f64>()
            })
            .collect()
    }

    #[test]
    fn waits_with_a_few_late_wake_ups_keep_the_true_refresh_and_the_wait_on() {
        let (beat, base) = measured(&beat_of(11.47, &[(3, 3.5), (8, 4.0), (13, 2.5)]));
        assert!(beat.on());
        let g = beat.grid(base + ms(200.0)).expect("a grid");
        assert!((g.refresh.as_secs_f64() - 0.01147).abs() < 0.0003);
    }

    #[test]
    fn a_wait_much_later_than_the_rest_does_not_change_the_refresh() {
        let (beat, base) = measured(&beat_of(11.47, &[]));
        // A frame that took three refreshes, then a run of single ones.
        let mut t = 16.0 * 11.47 + 34.4;
        beat.vblank(base + ms(t));
        for _ in 0..20 {
            t += 11.47;
            beat.vblank(base + ms(t));
        }
        let g = beat.grid(base + ms(t)).expect("a grid");
        assert!((g.refresh.as_secs_f64() - 0.01147).abs() < 0.0001);
        assert!(beat.on());
    }

    #[test]
    fn waits_a_whole_refresh_apart_count_what_carried() {
        let (beat, base) = measured(&beat_of(20.0, &[]));
        beat.set_step(2);
        // Frames written on every other refresh.
        for k in 0..20 {
            beat.vblank(base + ms(360.0 + 40.0 * f64::from(k)));
        }
        let text = beat.report();
        assert!(text.contains("wait on"), "{text}");
        assert!(text.contains("carried 20"), "{text}");
        assert!(text.contains("missed 0"), "{text}");
        assert!(text.contains("refresh    20.00 ms"), "{text}");
    }

    #[test]
    fn waits_with_no_steady_beat_turn_the_wait_off() {
        let gaps = [
            5.0, 20.0, 8.0, 30.0, 12.0, 6.0, 25.0, 9.0, 18.0, 7.0, 28.0, 10.0, 22.0, 5.0, 15.0,
            31.0,
        ];
        let mut t = 0.0;
        let mut times = vec![0.0];
        for g in gaps {
            t += g;
            times.push(t);
        }
        let (beat, base) = measured(&times);
        assert!(!beat.on());
        assert!(beat.grid(base + ms(t)).is_none());
        assert!(beat.report().contains("off ("));
    }

    #[test]
    fn a_steady_beat_outside_the_range_turns_the_wait_off() {
        for r in [4.0, 40.0] {
            let (beat, _) = measured(&beat_of(r, &[]));
            assert!(!beat.on(), "{r} ms");
        }
    }

    #[test]
    fn waits_that_fall_off_the_grid_again_and_again_turn_the_wait_off() {
        let (beat, base) = measured(&beat_of(20.0, &[]));
        let mut t = 320.0;
        for k in 0..20 {
            t += if k % 2 == 0 { 7.0 } else { 33.0 };
            beat.vblank(base + ms(t));
        }
        assert!(!beat.on());
    }
}
