//! What each part of a loop pass costs, for the dev card (`--timings`).
//!
//! The release card gives nothing back: no console, no network, no login,
//! and the owner has no serial adapter. The one channel out is the
//! `OSKDATA` partition, so a dev card records how long the loop's calls
//! take into a file there, and the owner reads it on a computer after
//! using the device for a minute.
//!
//! Five buckets, all timed with `Instant` around calls the loop already
//! makes, and a count of passes and touches. Every five seconds of the
//! shell's clock the totals become one block of plain text and reset, so
//! a long session is a short file: the card is slow to write and nothing
//! here is worth a row per pass.

use std::time::Duration;

/// How much of the shell's clock one block covers.
pub const INTERVAL: Duration = Duration::from_secs(5);

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
}

/// The bucket names, in the order the block prints them.
const NAMES: [&str; 6] = ["touch", "tick", "camera", "scanned", "convert", "write"];

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
    /// The shell's clock when the next block is due.
    next: Duration,
    /// The line that goes above the first block of this run.
    header: String,
    /// Whether that line has been written.
    headed: bool,
}

impl Timings {
    /// A fresh set of buckets, headed with what produced them.
    pub fn new(version: &str, width: u16, height: u16) -> Timings {
        Timings {
            tallies: [Tally::default(); NAMES.len()],
            passes: 0,
            touches: 0,
            next: INTERVAL,
            header: format!(
                "opensigner-pi {version} panel {width}x{height} \
                 columns: bucket count mean_ms worst_ms"
            ),
            headed: false,
        }
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

    /// Records one touch delivered to the core.
    pub fn touch(&mut self) {
        self.touches += 1;
    }

    /// The block to append if five seconds have gone by, and nothing
    /// otherwise. Taking it resets the buckets.
    pub fn due(&mut self, elapsed: Duration) -> Option<String> {
        if elapsed < self.next {
            return None;
        }
        while self.next <= elapsed {
            self.next += INTERVAL;
        }
        Some(self.block(elapsed))
    }

    /// The last block, however little of an interval it covers.
    pub fn last(&mut self, elapsed: Duration) -> String {
        self.block(elapsed)
    }

    /// One block, and the buckets emptied behind it.
    fn block(&mut self, elapsed: Duration) -> String {
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
        out.push('\n');
        self.tallies = [Tally::default(); NAMES.len()];
        self.passes = 0;
        self.touches = 0;
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
