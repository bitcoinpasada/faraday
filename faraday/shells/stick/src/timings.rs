//! What each part of a loop pass costs, for the dev card (`--timings`).
//!
//! The release card gives nothing back: no console, no network, no login,
//! and the owner has no serial adapter. The one channel out is the
//! `FARADAYDATA` partition, so a dev card records how long the loop's calls
//! take into a file there, and the owner reads it on a computer after
//! using the device for a minute.
//!
//! Eight buckets, all timed with `Instant` around calls the loop already
//! makes, and a count of passes and touches; a `work` row for each kind
//! of costly work the core did (a vault made or unlocked, a key made, a
//! transaction signed, a code decoded, a file written: `costly.rs`); how often the frame grid
//! stood at each step, and what the fades did (`docs/MOTION.md` §3.5). Every five seconds of the
//! shell's clock the totals become one block of plain text and reset, so
//! a long session is a short file: the card is slow to write and nothing
//! here is worth a row per pass.

use std::path::Path;
use std::time::{Duration, Instant};

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
    /// The `pwrite` of that frame to the framebuffer: its own time, on
    /// whichever thread it ran. On a panel written by a thread of its
    /// own (`writer.rs`) it overlaps the loop and is not in the pass.
    Write,
    /// The core's frame drawn in full.
    Frame,
    /// The core's frame as a fade's step: two kept frames blended.
    Blend,
    /// The loop waiting for the write before to end, before the next
    /// frame's rows are kept (`writer.rs`).
    Wait,
    /// A frame compared with the panel's kept copy and its changed rows
    /// copied into it, for the writer thread to write.
    Stage,
}

/// A call the loop makes outside the buckets, timed as a whole.
#[derive(Clone, Copy)]
pub enum Part {
    /// `rescan_inputs`: a look for input devices plugged in.
    Rescan,
    /// `scan_sticks`: a look at the storage plugged in.
    Sticks,
    /// `report`: building the timings block and, when due, writing it.
    Report,
    /// `write_boot_report`: sampling the system and, when due, writing
    /// the report.
    BootReport,
    /// One write of the CPU's lowest clock (`clock.rs`), made inside
    /// the pass that raised or lowered it.
    ClockWrite,
}

/// The names of the parts, in the order the block prints them.
const PART_NAMES: [&str; 5] = ["rescan", "sticks", "report", "bootreport", "clockwrite"];

/// The moment a call starts, when `--timings`
/// asked for it (`stop` records it).
pub fn start(t: &Option<Timings>) -> Option<Instant> {
    t.is_some().then(Instant::now)
}

/// Records the call `start` began as `part`.
pub fn stop(t: &mut Option<Timings>, part: Part, start: Option<Instant>) {
    if let (Some(start), Some(t)) = (start, t.as_mut()) {
        t.part(part, start.elapsed());
    }
}

/// Where the kernel says how the first CPU's clock is driven.
/// The file in the shell's run directory that carries the runs of this
/// boot across a lock: the number of the last run on its first line, then
/// every block so far. It exists only with `--timings`.
pub const CARRY: &str = "timings-carry";

/// The most the file keeps; past it the oldest runs go, or, when only one
/// run is left, its oldest blocks.
pub const MAX_KEPT: usize = 1 << 20;

/// The line that says the oldest runs were dropped.
const DROPPED: &str = "dropped: the oldest runs, to keep the file under 1 MiB";

const CPUFREQ: &str = "/sys/devices/system/cpu/cpu0/cpufreq";

/// One of the first CPU's clock files, trimmed; `-` when there is none
/// (a kernel without cpufreq, a machine with no driver for its CPU).
fn cpufreq(name: &str) -> String {
    std::fs::read_to_string(format!("{CPUFREQ}/{name}"))
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|_| "-".to_owned())
}

/// The bucket names, in the order the block prints them.
const NAMES: [&str; 10] = [
    "touch", "tick", "camera", "scanned", "convert", "write", "frame", "blend", "wait", "stage",
];

/// The most rows of costly work one block keeps; an interval that did
/// more kinds than this puts the rest on the last row's name.
const WORK_ROWS: usize = 24;

/// The row an interval's costly work goes on past [`WORK_ROWS`].
const OTHER_WORK: &str = "other work";

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

/// The most rows of the table by screen one block keeps; a block that
/// drew more screens than this puts the rest on the last row.
const SCREEN_ROWS: usize = 24;

/// The row of the table by screen past [`SCREEN_ROWS`].
const OTHER_SCREENS: &str = "other screens";

/// The passes that drew one kind of frame since the last block: how
/// many, and what their `frame` and `write` calls cost.
#[derive(Clone, Copy, Default)]
struct KindTally {
    frame: Tally,
    write: Tally,
}

impl KindTally {
    fn add(&mut self, frame: Duration, write: Duration) {
        for (tally, took) in [(&mut self.frame, frame), (&mut self.write, write)] {
            tally.count += 1;
            tally.total += took;
            tally.worst = tally.worst.max(took);
        }
    }
}

/// The passes that drew one screen since the last block.
struct ScreenTally {
    name: String,
    passes: KindTally,
    moved: u64,
}

/// The buckets, the counters, and when the next block is due.
pub struct Timings {
    tallies: [Tally; NAMES.len()],
    /// The costly work since the last block, a row for each kind of it
    /// that came, in the order they first came (`costly.rs`).
    work: Vec<(String, Tally)>,
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
    /// This process's run number since boot, from 1.
    run: u32,
    /// The calls outside the buckets since the last block.
    parts: [Tally; PART_NAMES.len()],
    /// What the buckets and parts timed on the loop's thread in the
    /// pass now running: what the pass's total can be held against.
    explained: Duration,
    /// The slowest pass that drew since the last block: its total, how
    /// much of it no bucket or part explains, and what it was.
    slowest: Option<(Duration, Duration, String)>,
    /// The passes that drew a moved frame and a whole frame since the
    /// last block.
    moved: KindTally,
    whole: KindTally,
    /// The passes that drew, by screen name, in the order the screens
    /// first came.
    screens: Vec<ScreenTally>,
    /// What each bucket and each part timed in the pass now running.
    pass_buckets: [Duration; NAMES.len()],
    pass_parts: [Duration; PART_NAMES.len()],
    /// The frame grid's step after the last pass that drew, in frames.
    last_step: Option<u32>,
    /// How many times the step changed since the last block.
    step_changes: u64,
    /// Why the step changed, and the passes shown late, with the refresh
    /// wait on.
    paced: crate::pace::Events,
    /// The screen's refresh as the writer thread measures it, when it
    /// waits for one: its lines go in each block.
    beat: Option<std::sync::Arc<crate::refresh::Beat>>,
}

/// What kind of frame a pass drew.
#[derive(Clone, Copy)]
pub enum Frame {
    /// Drawn in full.
    Whole,
    /// A frame whose moved strip was painted as it was called.
    Moved,
    /// A fade's step: two kept frames blended.
    Fade,
}

impl Timings {
    /// A fresh set of buckets, headed with what produced them.
    pub fn new(version: &str, width: u16, height: u16) -> Timings {
        Timings {
            tallies: [Tally::default(); NAMES.len()],
            work: Vec::new(),
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
                 costly work columns: work count mean_ms worst_ms kind (the costliest an event did)\n\
                 cpu0 driver {} governor {} min_khz {} max_khz {}\n\
                 parts columns: part count mean_ms worst_ms name (calls outside the buckets; clockwrite is a write of the CPU's lowest clock, inside a pass)\n\
                 moved and whole columns: kind count frame_mean_ms frame_worst_ms write_mean_ms write_worst_ms (passes that drew a moved or a whole frame)\n\
                 by_screen columns: by_screen count frame_mean_ms write_mean_ms moved_percent name (passes that drew, by screen)\n\
                 slowest: the slowest pass that drew, how much of it no bucket or part explains, and what it was (the screen, the kind of frame, whether the screen changed since the last drawn pass, the largest bucket or part in it)",
                cpufreq("scaling_driver"),
                cpufreq("scaling_governor"),
                cpufreq("cpuinfo_min_freq"),
                cpufreq("cpuinfo_max_freq"),
            ),
            headed: false,
            kept: String::new(),
            wrote: None,
            run: 1,
            parts: [Tally::default(); PART_NAMES.len()],
            explained: Duration::ZERO,
            slowest: None,
            moved: KindTally::default(),
            whole: KindTally::default(),
            screens: Vec::new(),
            pass_buckets: [Duration::ZERO; NAMES.len()],
            pass_parts: [Duration::ZERO; PART_NAMES.len()],
            last_step: None,
            step_changes: 0,
            paced: crate::pace::Events::default(),
            beat: None,
        }
    }

    /// Takes up the runs before this one from the carry file in `dir` and
    /// numbers this run after them. A file that is missing, cannot be
    /// read or does not parse leaves this run the first. Costs a file
    /// read.
    pub fn resume(&mut self, dir: &Path) {
        let Ok(text) = std::fs::read_to_string(dir.join(CARRY)) else {
            return;
        };
        let (first, rest) = text.split_once('\n').unwrap_or((&text, ""));
        let Some(run) = first
            .trim()
            .parse::<u32>()
            .ok()
            .and_then(|n| n.checked_add(1))
        else {
            return;
        };
        self.run = run;
        self.kept = rest.to_string();
        trim(&mut self.kept);
    }

    /// Writes what the next process carries on: this run's number and the
    /// blocks so far. Costs a file write; a file that cannot be written
    /// leaves the next process on run 1.
    pub fn save(&self, dir: &Path) {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(dir.join(CARRY), format!("{}\n{}", self.run, self.kept));
    }

    /// Keeps `block` with the ones before it, and gives every block so
    /// far when the whole file is due to be written again (`last`: at
    /// exit, whenever it is).
    pub fn keep(&mut self, block: &str, elapsed: Duration, last: bool) -> Option<&str> {
        self.kept.push_str(block);
        trim(&mut self.kept);
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
        self.explained += took;
        self.pass_buckets[bucket as usize] += took;
    }

    /// Records one call of a bucket that ran beside the loop on another
    /// thread: counted in the bucket's line, not in what the pass
    /// explains.
    pub fn add_beside(&mut self, bucket: Bucket, took: Duration) {
        let tally = &mut self.tallies[bucket as usize];
        tally.count += 1;
        tally.total += took;
        tally.worst = tally.worst.max(took);
    }

    /// Records one call outside the buckets; it counts towards what the
    /// pass it is in explains.
    pub fn part(&mut self, part: Part, took: Duration) {
        let tally = &mut self.parts[part as usize];
        tally.count += 1;
        tally.total += took;
        tally.worst = tally.worst.max(took);
        self.explained += took;
        self.pass_parts[part as usize] += took;
    }

    /// Records the clock writes made since the last call.
    pub fn clock_writes(&mut self, took: impl IntoIterator<Item = Duration>) {
        for d in took {
            self.part(Part::ClockWrite, d);
        }
    }

    /// A pass begins, at the end of its wait: what it explains starts
    /// from nothing.
    pub fn begin_pass(&mut self) {
        self.explained = Duration::ZERO;
        self.pass_buckets = [Duration::ZERO; NAMES.len()];
        self.pass_parts = [Duration::ZERO; PART_NAMES.len()];
    }

    /// The pass that drew took `total` from its start to its frame
    /// written: kept if it is the slowest of the block, with what it
    /// was. `frame` is the kind of frame, `changed` whether the screen
    /// differs from the last pass that drew, and `screen` gives the
    /// screen's name (nothing a person typed). The pass is also counted
    /// by its kind of frame and by screen, with what its `frame` and
    /// `write` calls took.
    pub fn drawn_pass(
        &mut self,
        total: Duration,
        frame: Frame,
        changed: bool,
        screen: impl FnOnce() -> String,
    ) {
        let frame_took = self.pass_buckets[Bucket::Frame as usize];
        // What the pass paid for the write: all of it on the loop's own
        // thread, the keeping and the wait when a thread writes.
        let write_took: Duration = [Bucket::Write, Bucket::Stage, Bucket::Wait]
            .iter()
            .map(|&b| self.pass_buckets[b as usize])
            .sum();
        let moved = matches!(frame, Frame::Moved);
        match frame {
            Frame::Moved => self.moved.add(frame_took, write_took),
            Frame::Whole => self.whole.add(frame_took, write_took),
            Frame::Fade => {}
        }
        let name = screen();
        let row = match self.screens.iter().position(|r| r.name == name) {
            Some(i) => Some(i),
            None if self.screens.len() < SCREEN_ROWS => {
                self.screens.push(ScreenTally {
                    name: name.clone(),
                    passes: KindTally::default(),
                    moved: 0,
                });
                Some(self.screens.len() - 1)
            }
            None => self.screens.iter().position(|r| r.name == OTHER_SCREENS),
        };
        // Past the rows, the screens go on one more row of their own.
        let row = row.unwrap_or_else(|| {
            self.screens.push(ScreenTally {
                name: OTHER_SCREENS.to_owned(),
                passes: KindTally::default(),
                moved: 0,
            });
            self.screens.len() - 1
        });
        self.screens[row].passes.add(frame_took, write_took);
        self.screens[row].moved += u64::from(moved);
        if self.slowest.as_ref().is_none_or(|(t, _, _)| total > *t) {
            let top = NAMES
                .iter()
                .zip(&self.pass_buckets)
                .chain(PART_NAMES.iter().zip(&self.pass_parts))
                .filter(|(_, d)| !d.is_zero())
                .max_by_key(|(_, d)| **d)
                .map_or_else(
                    || "-".to_owned(),
                    |(name, d)| format!("{name} {:.2}", ms(*d)),
                );
            let frame = match frame {
                Frame::Whole => "whole",
                Frame::Moved => "moved",
                Frame::Fade => "fade",
            };
            let after = if changed { "yes" } else { "no" };
            let what = format!(
                "screen {} frame {frame} after_change {after} largest_ms {top}",
                name
            );
            self.slowest = Some((total, total.saturating_sub(self.explained), what));
        }
    }

    /// Records one piece of costly work on the row called `row`.
    pub fn work(&mut self, row: &str, took: Duration) {
        let i = match self.work.iter().position(|(name, _)| name == row) {
            Some(i) => i,
            None if self.work.len() < WORK_ROWS => {
                self.work.push((row.to_owned(), Tally::default()));
                self.work.len() - 1
            }
            None => match self.work.iter().position(|(name, _)| name == OTHER_WORK) {
                Some(i) => i,
                None => {
                    let last = self.work.len() - 1;
                    self.work[last] = (OTHER_WORK.to_owned(), self.work[last].1);
                    last
                }
            },
        };
        let tally = &mut self.work[i].1;
        tally.count += 1;
        tally.total += took;
        tally.worst = tally.worst.max(took);
    }

    /// Adds the lines of the screen's refresh (`refresh.rs`) to each
    /// block, when the writer thread waits for it.
    pub fn watch_refresh(&mut self, beat: Option<std::sync::Arc<crate::refresh::Beat>>) {
        self.beat = beat;
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
        if self.last_step.is_some_and(|s| s != frames) {
            self.step_changes += 1;
        }
        self.last_step = Some(frames);
    }

    /// Adds what the pacer counted since the last pass that drew: frames
    /// shown late against the vblank and why the step changed.
    pub fn paced(&mut self, events: crate::pace::Events) {
        self.paced.late += events.late;
        self.paced.coarsened += events.coarsened;
        self.paced.recovered += events.recovered;
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

    /// The lines for the passes that drew: by kind of frame, then by
    /// screen.
    fn drawn_lines(&self) -> String {
        let mut out = String::new();
        for (name, kind) in [("moved", &self.moved), ("whole", &self.whole)] {
            if kind.frame.count > 0 {
                out.push_str(&format!(
                    "{name:<8} {:>7} {:>9.2} {:>9.2} {:>9.2} {:>9.2}\n",
                    kind.frame.count,
                    mean_ms(kind.frame),
                    ms(kind.frame.worst),
                    mean_ms(kind.write),
                    ms(kind.write.worst),
                ));
            }
        }
        for row in &self.screens {
            let count = row.passes.frame.count;
            out.push_str(&format!(
                "by_screen {count:>6} {:>9.2} {:>9.2} {:>5.0}%  {}\n",
                mean_ms(row.passes.frame),
                mean_ms(row.passes.write),
                row.moved as f64 * 100.0 / count.max(1) as f64,
                row.name,
            ));
        }
        out
    }

    /// One block, and the buckets emptied behind it.
    fn block(&mut self, elapsed: Duration, fade: Option<FadeCounts>) -> String {
        let mut out = String::new();
        if !self.headed {
            self.headed = true;
            out.push_str(&format!("run {}\n", self.run));
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
        for (name, tally) in &self.work {
            out.push_str(&format!(
                "work     {:>7} {:>9.2} {:>9.2}  {name}\n",
                tally.count,
                mean_ms(*tally),
                ms(tally.worst),
            ));
        }
        let [s1, s2, s3, s4] = self.steps;
        out.push_str(&format!(
            "grid     16.7ms {s1} 33.3ms {s2} 50.0ms {s3} 66.7ms {s4} cost {:.2} changes {} late {} coarsened {} recovered {}\n",
            ms(self.pace_cost),
            self.step_changes,
            self.paced.late,
            self.paced.coarsened,
            self.paced.recovered,
        ));
        if let Some(beat) = &self.beat {
            out.push_str(&beat.report());
        }
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
        for (name, tally) in PART_NAMES.iter().zip(&self.parts) {
            if tally.count > 0 {
                out.push_str(&format!(
                    "part     {:>7} {:>9.2} {:>9.2}  {name}\n",
                    tally.count,
                    mean_ms(*tally),
                    ms(tally.worst),
                ));
            }
        }
        out.push_str(&self.drawn_lines());
        if let Some((total, unexplained, what)) = &self.slowest {
            out.push_str(&format!(
                "slowest  pass_ms {:.2} unexplained_ms {:.2} {what}\n",
                ms(*total),
                ms(*unexplained),
            ));
        }
        out.push('\n');
        self.parts = [Tally::default(); PART_NAMES.len()];
        self.slowest = None;
        self.moved = KindTally::default();
        self.whole = KindTally::default();
        self.screens.clear();
        self.tallies = [Tally::default(); NAMES.len()];
        self.work.clear();
        self.passes = 0;
        self.touches = 0;
        self.steps = [0; STEPS];
        self.step_changes = 0;
        self.paced = crate::pace::Events::default();
        self.clock = (0, 0, 0);
        self.raised = 0;
        self.fade_steps.clear();
        out
    }
}

/// A duration in milliseconds, for one column of the block.
/// Whether a line opens a run (`run 2`) or a block (`    5s passes ...`).
fn opens(line: &str, runs: bool) -> bool {
    if runs {
        return line.starts_with("run ");
    }
    let t = line.trim_start();
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && t[digits..].starts_with("s passes ")
}

/// Drops the oldest runs from `kept` until it fits [`MAX_KEPT`], leaving
/// the newest, and a line saying so. When one run is left, its oldest
/// blocks go instead.
fn trim(kept: &mut String) {
    if kept.len() <= MAX_KEPT {
        return;
    }
    if kept.starts_with(DROPPED) {
        kept.drain(..(DROPPED.len() + 1).min(kept.len()));
    }
    while kept.len() + DROPPED.len() + 1 > MAX_KEPT {
        let starts = |runs: bool| {
            let mut at = 0;
            let mut found = Vec::new();
            for line in kept.split_inclusive('\n') {
                if at > 0 && opens(line, runs) {
                    found.push(at);
                }
                at += line.len();
            }
            found
        };
        let mut cuts = starts(true);
        if cuts.is_empty() {
            cuts = starts(false);
        }
        let Some(cut) = cuts.first().copied() else {
            kept.clear();
            break;
        };
        kept.drain(..cut);
    }
    kept.insert_str(0, &format!("{DROPPED}\n"));
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn work_rows(block: &str) -> Vec<&str> {
        block.lines().filter(|l| l.starts_with("work ")).collect()
    }

    #[test]
    fn a_block_has_a_row_for_each_kind_of_costly_work_and_the_next_starts_empty() {
        let mut t = Timings::new("0.0.0", 480, 640);
        t.work(
            "vault unlocked Light 64 MiB 3 passes",
            Duration::from_millis(2500),
        );
        t.work("file written", Duration::from_millis(40));
        t.work("file written", Duration::from_millis(80));
        let block = t.last(Duration::from_secs(5), None);
        assert_eq!(
            work_rows(&block),
            [
                "work           1   2500.00   2500.00  vault unlocked Light 64 MiB 3 passes",
                "work           2     60.00     80.00  file written",
            ]
        );
        let next = t.last(Duration::from_secs(10), None);
        assert!(work_rows(&next).is_empty());
    }

    #[test]
    fn work_past_the_rows_a_block_keeps_is_counted_on_one_more() {
        let mut t = Timings::new("0.0.0", 480, 640);
        for i in 0..WORK_ROWS + 3 {
            t.work(&format!("kind {i}"), Duration::from_millis(1));
        }
        let block = t.last(Duration::from_secs(5), None);
        let rows = work_rows(&block);
        assert_eq!(rows.len(), WORK_ROWS);
        assert_eq!(
            rows[WORK_ROWS - 1],
            "work           4      1.00      1.00  other work"
        );
    }

    #[test]
    fn a_block_says_what_the_calls_outside_the_buckets_cost() {
        let mut t = Timings::new("0.0.0", 480, 640);
        t.part(Part::Rescan, Duration::from_millis(3));
        t.part(Part::Rescan, Duration::from_millis(5));
        t.clock_writes([Duration::from_millis(2)]);
        let block = t.last(Duration::from_secs(5), None);
        let rows: Vec<&str> = block.lines().filter(|l| l.starts_with("part ")).collect();
        assert_eq!(
            rows,
            [
                "part           2      4.00      5.00  rescan",
                "part           1      2.00      2.00  clockwrite",
            ]
        );
        assert!(!block.contains("slowest  "), "no pass drew");
        let next = t.last(Duration::from_secs(10), None);
        assert!(!next.contains("part  "));
    }

    #[test]
    fn the_grid_line_counts_how_often_the_step_changed_in_the_block() {
        let mut t = Timings::new("0.0.0", 480, 640);
        for frames in [1, 1, 2, 2, 1] {
            t.stepped(frames, Duration::from_millis(10));
        }
        let block = t.last(Duration::from_secs(5), None);
        assert!(
            block.contains("cost 10.00 changes 2 late 0 coarsened 0 recovered 0\n"),
            "{block}"
        );
        t.stepped(1, Duration::from_millis(10));
        let next = t.last(Duration::from_secs(10), None);
        assert!(next.contains("changes 0 late 0"), "{next}");
    }

    #[test]
    fn a_block_counts_moved_and_whole_frames_and_each_screen() {
        let mut t = Timings::new("0.0.0", 480, 640);
        for (frame, screen, took) in [
            (Frame::Whole, "Home", 20),
            (Frame::Moved, "Home", 4),
            (Frame::Moved, "Home", 6),
            (Frame::Whole, "Backup / Qr", 30),
            (Frame::Fade, "Home", 9),
        ] {
            t.begin_pass();
            t.add(Bucket::Frame, Duration::from_millis(took));
            t.add(Bucket::Write, Duration::from_millis(2));
            t.drawn_pass(Duration::from_millis(took + 2), frame, false, || {
                screen.into()
            });
        }
        let block = t.last(Duration::from_secs(5), None);
        assert!(
            block.contains("moved          2      5.00      6.00      2.00      2.00\n"),
            "{block}"
        );
        assert!(
            block.contains("whole          2     25.00     30.00      2.00      2.00\n"),
            "{block}"
        );
        assert!(
            block.contains("by_screen      4      9.75      2.00    50%  Home\n"),
            "{block}"
        );
        let next = t.last(Duration::from_secs(10), None);
        assert!(!next.contains("by_screen "), "{next}");
    }

    #[test]
    fn the_slowest_pass_that_drew_says_how_much_of_it_nothing_explains() {
        let mut t = Timings::new("0.0.0", 480, 640);
        t.begin_pass();
        t.add(Bucket::Tick, Duration::from_millis(4));
        t.drawn_pass(Duration::from_millis(30), Frame::Whole, false, || {
            "Home".into()
        });
        t.begin_pass();
        t.add(Bucket::Tick, Duration::from_millis(10));
        t.part(Part::Rescan, Duration::from_millis(20));
        t.clock_writes([Duration::from_millis(100)]);
        t.drawn_pass(Duration::from_millis(500), Frame::Fade, true, || {
            "Backup / Qr".into()
        });
        t.begin_pass();
        t.drawn_pass(Duration::from_millis(40), Frame::Moved, false, || {
            "Home".into()
        });
        let block = t.last(Duration::from_secs(5), None);
        assert!(
            block.contains(
                "slowest  pass_ms 500.00 unexplained_ms 370.00 \
                 screen Backup / Qr frame fade after_change yes largest_ms clockwrite 100.00\n"
            ),
            "{block}"
        );
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("faraday-timings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// One process: resumes, writes a block and keeps it, saves at exit.
    fn a_run(dir: &Path, secs: u64) -> String {
        let mut t = Timings::new("0.0.0", 480, 640);
        t.resume(dir);
        let block = t.last(Duration::from_secs(secs), None);
        let whole = t
            .keep(&block, Duration::from_secs(secs), true)
            .unwrap()
            .to_string();
        t.save(dir);
        whole
    }

    fn run_lines(text: &str) -> Vec<&str> {
        text.lines().filter(|l| l.starts_with("run ")).collect()
    }

    fn pass_lines(text: &str) -> Vec<&str> {
        text.lines()
            .filter(|l| l.contains("s passes "))
            .map(str::trim_start)
            .map(|l| l.split(' ').next().unwrap())
            .collect()
    }

    #[test]
    fn each_run_since_boot_is_in_the_file_in_order_under_its_number() {
        let dir = scratch("runs");
        let first = a_run(&dir, 5);
        assert_eq!(run_lines(&first), ["run 1"]);
        let second = a_run(&dir, 6);
        assert_eq!(run_lines(&second), ["run 1", "run 2"]);
        let third = a_run(&dir, 7);
        assert_eq!(run_lines(&third), ["run 1", "run 2", "run 3"]);
        assert_eq!(pass_lines(&third), ["5s", "6s", "7s"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_carry_file_that_is_missing_or_garbage_starts_at_run_one() {
        let dir = scratch("garbage");
        assert_eq!(run_lines(&a_run(&dir.join("none"), 5)), ["run 1"]);
        for bad in [&b"not a number\nx"[..], b"\xff\xfe", b"", b"99999999999\nx"] {
            std::fs::write(dir.join(CARRY), bad).unwrap();
            assert_eq!(run_lines(&a_run(&dir, 5)), ["run 1"]);
        }
        std::fs::remove_file(dir.join(CARRY)).unwrap();
        std::fs::create_dir(dir.join(CARRY)).unwrap();
        assert_eq!(run_lines(&a_run(&dir, 5)), ["run 1"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn past_the_limit_the_oldest_runs_go_with_a_line_saying_so() {
        let dir = scratch("limit");
        let mut text = String::new();
        let mut runs = 0;
        while runs < 4000 {
            text = a_run(&dir, 5);
            runs += 1;
            if text.starts_with(DROPPED) {
                break;
            }
        }
        assert!(text.starts_with(DROPPED), "no limit after {runs} runs");
        assert!(text.len() <= MAX_KEPT);
        let lines = run_lines(&text);
        assert_eq!(lines.last().copied(), Some(format!("run {runs}").as_str()));
        assert_ne!(lines.first().copied(), Some("run 1"));
        assert_eq!(text.matches(DROPPED).count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
