//! The CPU's lowest clock, raised while something moves
//! (`docs/MOTION.md` §3.5).
//!
//! On a CPU whose clock the kernel's schedutil governor sets, the clock
//! rises only once work has run a while, so a fade, which starts from
//! idle, draws its first steps slowly and is turned off. While input
//! arrives or frames are drawn, the shell sets each CPU's lowest clock to
//! its highest, and puts it back a second after the last. Init lets the
//! app write `scaling_min_freq` only on a PC whose CPUs run schedutil; a
//! CPU whose driver sets its own clock, and the Pi, are left alone, and
//! with nothing writable this does nothing.

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Where the kernel lists each group of CPUs that share a clock.
const CPUFREQ: &str = "/sys/devices/system/cpu/cpufreq";

/// How long the clock stays raised after the last input or frame.
const HOLD: Duration = Duration::from_secs(1);

/// One group of CPUs: its `scaling_min_freq`, and the lowest and highest
/// clocks it can be set to, as the kernel wrote them.
struct Policy {
    min_file: PathBuf,
    low: String,
    high: String,
}

/// The groups whose lowest clock this shell may raise, and whether it is
/// raised.
pub struct Boost {
    policies: Vec<Policy>,
    raised: bool,
    last: Option<Instant>,
}

impl Boost {
    /// Costs a read of each CPU group's files in sysfs, once at start:
    /// the groups that run schedutil.
    pub fn new() -> Boost {
        let read = |p: &PathBuf, name: &str| {
            std::fs::read_to_string(p.join(name)).map(|s| s.trim().to_owned())
        };
        let mut policies = Vec::new();
        for entry in std::fs::read_dir(CPUFREQ).into_iter().flatten().flatten() {
            let dir = entry.path();
            let is_policy = dir
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("policy"));
            if !is_policy || read(&dir, "scaling_governor").ok().as_deref() != Some("schedutil") {
                continue;
            }
            if let (Ok(low), Ok(high)) = (
                read(&dir, "cpuinfo_min_freq"),
                read(&dir, "cpuinfo_max_freq"),
            ) {
                policies.push(Policy {
                    min_file: dir.join("scaling_min_freq"),
                    low,
                    high,
                });
            }
        }
        Boost {
            policies,
            raised: false,
            last: None,
        }
    }

    /// Costs a write to sysfs per CPU group when the clock is not raised
    /// yet: something moves at `now`.
    pub fn moving(&mut self, now: Instant) {
        self.last = Some(now);
        if !self.raised {
            self.raised = true;
            self.set(true);
        }
    }

    /// Costs a write to sysfs per CPU group when the clock comes down:
    /// a pass at `now` with nothing moving.
    pub fn still(&mut self, now: Instant) {
        if self.raised && self.last.is_none_or(|t| now.duration_since(t) >= HOLD) {
            self.raised = false;
            self.set(false);
        }
    }

    /// Whether the clock is raised now, for `--timings`.
    pub fn raised(&self) -> bool {
        self.raised
    }

    /// Sets each group's lowest clock to its highest, or back. A group
    /// that cannot be written is dropped: init did not hand it over.
    fn set(&mut self, high: bool) {
        self.policies.retain(|p| {
            let value = if high { &p.high } else { &p.low };
            std::fs::write(&p.min_file, value).is_ok()
        });
    }
}
