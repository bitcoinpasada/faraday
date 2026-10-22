//! What each part of a loop pass costs, for the dev card (`--timings`).
//!
//! The release card gives nothing back: no console, no network, no login,
//! and the owner has no serial adapter. The one channel out is the
//! `FARADAYDATA` partition, so a dev card records how long the loop's calls
//! take into a file there, and the owner reads it on a computer after
//! using the device for a minute.
//!
//! Eight buckets, all timed with `Instant` around calls the loop already
//! makes, and a count of passes and touches; how often the frame grid
//! stood at each step, and what the fades did (`docs/MOTION.md` §3.5). Every five seconds of the
//! shell's clock the totals become one block of plain text and reset, so
//! a long session is a short file: the card is slow to write and nothing
//! here is worth a row per pass.

use std::time::Duration;

use faraday_core::FadeCounts;

/// How much of the shell's clock one block covers.
pub const INTERVAL: Duration = Duration::from_secs(5);

/// How often the whole file goes to the boot stick through the disk
/// process, which holds the loop while it writes: rarely enough that the
/// blocks it lands in are not what it measures.
pub const WRITE_EVERY: Duration = Duration::from_secs(30);

/// The part of a loop pass a measurement belongs to.
#[derive(Clone, Copy)]
pub enum Bucket {
    /// One `Event::Touch` through the core, layout and paint included.
    Touch,
    /// One `Event::Tick`.
    Tick,
    /// One `Event::CameraFrame`: the preview conversion, on every frame.
    Camera,
    /// One `Event::Scanned`: routing a code the scanner's worker read.
    Scanned,
    /// The core's frame converted to the panel's pixel format.
    Convert,
    /// The `pwrite` of that frame to the framebuffer.
    Write,
    /// The core's frame drawn in full.
    Frame,
    /// The core's frame as a fade's step: two kept frames blended.
    Blend,
}

/// Where the kernel says how the first CPU's clock is driven.
const CPUFREQ: &str = "/sys/devices/system/cpu/cpu0/cpufreq";

/// One of the first CPU's clock files, trimmed; `-` when there is none
/// (a kernel without cpufreq, a machine with no driver for its CPU).
fn cpufreq(name: &str) -> String {
    std::fs::read_to_string(format!("{CPUFREQ}/{name}"))
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "-".to_owned())
}

/// The bucket names, in the order the block prints them.
const NAMES: [&str; 8] = [
    "touch", "tick", "camera", "scanned", "convert", "write", "frame", "blend",
];

/// How many fade steps one block lists.
const FADE_STEPS_KEPT: usize = 40;

/// The frame grid's steps a block counts, in frames at 60 Hz: one to
/// four (`pace.rs`).
const STEPS: usize = 4;

/// What one bucket has cost since the last block.
#[derive(Clone, Copy, Default)]
struct Tally {
    count: u64,
    total: Duration,
    worst: Duration,
}

/// The buckets, the counters, and when the next block is due.
pub struct Timings {
    tallies: [Tally; NAMES.len()],
    passes: u64,
    touches: u64,
    /// Passes that drew, by the frame grid's step after them: one frame
    /// to four.
    steps: [u64; STEPS],
    /// The frame grid's smoothed cost of a pass that drew, as last seen.
    pace_cost: Duration,
    /// The shell's clock when the next block is due.
    next: Duration,
    /// The line that goes above the first block of this run.
    header: String,
    /// Whether that line has been written.
    headed: bool,
    /// The first CPU's clock in kHz as sampled at each pass that drew
    /// since the last block: how many samples, their sum, the highest.
    clock: (u64, u64, u64),
    /// Of those samples, how many came while the shell held the clock
    /// raised (`clock.rs`).
    raised: u64,
    /// Each fade step since the last block that followed another: the
    /// gap since that one, what the pass cost, and the step chosen after
    /// it, in milliseconds; the first `FADE_STEPS_KEPT`.
    fade_steps: Vec<(u16, u16, u16)>,
    /// Every block so far, for a medium written whole each time.
    kept: String,
    /// The shell's clock when the whole file was last written.
    wrote: Option<Duration>,
}

impl Timings {
    /// A fresh set of buckets, headed with what produced them.
    pub fn new(version: &str, width: u16, height: u16) -> Timings {
        Timings {
            tallies: [Tally::default(); NAMES.len()],
            passes: 0,
            touches: 0,
            steps: [0; STEPS],
            pace_cost: Duration::ZERO,
            clock: (0, 0, 0),
            raised: 0,
            fade_steps: Vec::new(),
            next: INTERVAL,
            header: format!(
                "faraday {version} panel {width}x{height} \
                 columns: bucket count mean_ms worst_ms\n\
                 cpu0 driver {} governor {} min_khz {} max_khz {}",
                cpufreq("scaling_driver"),
                cpufreq("scaling_governor"),
                cpufreq("cpuinfo_min_freq"),
                cpufreq("cpuinfo_max_freq"),
            ),
            headed: false,
            kept: String::new(),
            wrote: None,
        }
    }

    /// Keeps `block` with the ones before it, and gives every block so
    /// far when the whole file is due to be written again (`last`: at
    /// exit, whenever it is).
    pub fn keep(&mut self, block: &str, elapsed: Duration, last: bool) -> Option<&str> {
        self.kept.push_str(block);
        let due = last || self.wrote.is_none_or(|w| elapsed >= w + WRITE_EVERY);
        if !due {
            return None;
        }
        self.wrote = Some(elapsed);
        Some(&self.kept)
    }

    /// Records one call.
    pub fn add(&mut self, bucket: Bucket, took: Duration) {
        let tally = &mut self.tallies[bucket as usize];
        tally.count += 1;
        tally.total += took;
        if took > tally.worst {
            tally.worst = took;
        }
    }

    /// Records one pass of the main loop.
    pub fn pass(&mut self) {
        self.passes += 1;
    }

    /// Records a pass that drew, the frame grid's step after it in
    /// frames, and the smoothed cost the step was chosen from.
    pub fn stepped(&mut self, frames: u32, cost: Duration) {
        let i = (frames.clamp(1, STEPS as u32) - 1) as usize;
        self.steps[i] += 1;
        self.pace_cost = cost;
    }

    /// Costs a read of a sysfs file: samples the first CPU's clock, at
    /// a pass that drew, and whether the shell held it `raised`.
    pub fn sample_clock(&mut self, raised: bool) {
        self.raised += u64::from(raised);
        if let Ok(khz) = cpufreq("scaling_cur_freq").parse::<u64>() {
            let (n, sum, top) = &mut self.clock;
            *n += 1;
            *sum += khz;
            *top = (*top).max(khz);
        }
    }

    /// Records a fade step that followed another `gap` after it, the
    /// pass costing `cost`, with the grid's next step `step`.
    pub fn fade_step(&mut self, gap: Duration, cost: Duration, step: Duration) {
        if self.fade_steps.len() < FADE_STEPS_KEPT {
            let ms = |d: Duration| d.as_millis().min(9999) as u16;
            self.fade_steps.push((ms(gap), ms(cost), ms(step)));
        }
    }

    /// Records one touch delivered to the core.
    pub fn touch(&mut self) {
        self.touches += 1;
    }

    /// The block to append if five seconds have gone by, and nothing
    /// otherwise; `fade` is asked only then. Taking it resets the
    /// buckets.
    pub fn due(
        &mut self,
        elapsed: Duration,
        fade: impl FnOnce() -> Option<FadeCounts>,
    ) -> Option<String> {
        if elapsed < self.next {
            return None;
        }
        while self.next <= elapsed {
            self.next += INTERVAL;
        }
        Some(self.block(elapsed, fade()))
    }

    /// The last block, however little of an interval it covers.
    pub fn last(&mut self, elapsed: Duration, fade: Option<FadeCounts>) -> String {
        self.block(elapsed, fade)
    }

    /// One block, and the buckets emptied behind it.
    fn block(&mut self, elapsed: Duration, fade: Option<FadeCounts>) -> String {
        let mut out = String::new();
        if !self.headed {
            self.headed = true;
            out.push_str(&self.header);
            out.push('\n');
        }
        out.push_str(&format!(
            "{:>6}s passes {:>7} touches {:>7}\n",
            elapsed.as_secs(),
            self.passes,
            self.touches,
        ));
        for (name, tally) in NAMES.iter().zip(&self.tallies) {
            out.push_str(&format!(
                "{name:<8} {:>7} {:>9.2} {:>9.2}\n",
                tally.count,
                mean_ms(*tally),
                ms(tally.worst),
            ));
        }
        let [s1, s2, s3, s4] = self.steps;
        out.push_str(&format!(
            "grid     16.7ms {s1} 33.3ms {s2} 50.0ms {s3} 66.7ms {s4} cost {:.2}\n",
            ms(self.pace_cost),
        ));
        let (n, sum, top) = self.clock;
        out.push_str(&format!(
            "clock    samples {n} mean_mhz {} max_mhz {} raised {}\n",
            sum.checked_div(n).unwrap_or(0) / 1000,
            top / 1000,
            self.raised,
        ));
        if !self.fade_steps.is_empty() {
            out.push_str("steps   ");
            for (gap, cost, step) in &self.fade_steps {
                out.push_str(&format!(" {gap}/{cost}/{step}"));
            }
            out.push('\n');
        }
        if let Some(f) = fade {
            let median = f
                .median_gap_ms
                .map_or_else(|| "-".to_owned(), |g| g.to_string());
            out.push_str(&format!(
                "fade     started {} skipped {} reduced {} steps {} gaps {} median_ms {median} {}\n",
                f.started,
                f.skipped,
                f.reduced,
                f.steps,
                f.gaps_known,
                if f.on { "on" } else { "off" },
            ));
        }
        out.push('\n');
        self.tallies = [Tally::default(); NAMES.len()];
        self.passes = 0;
        self.touches = 0;
        self.steps = [0; STEPS];
        self.clock = (0, 0, 0);
        self.raised = 0;
        self.fade_steps.clear();
        out
    }
}

/// A duration in milliseconds, for one column of the block.
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// A bucket's mean call in milliseconds; a bucket with no calls is zero
/// rather than a division by one.
fn mean_ms(tally: Tally) -> f64 {
    if tally.count == 0 {
        0.0
    } else {
        ms(tally.total) / tally.count as f64
    }
}
