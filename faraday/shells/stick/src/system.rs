//! The boot report's system block (`--boot-report`): how long the board
//! took to the app's first frame, when the kernel's random generator was
//! ready, the temperature, the first CPU's clock, the power supply, and
//! the memory, each now and at its worst since the shell started.
//!
//! Every figure is read from the kernel's own files under `/proc` and
//! `/sys`, once a second, at most, and written into the report at each
//! of its writes. What this kernel does not expose is said to be not
//! available, with where it was looked for: the firmware's throttling
//! flags are reached only through `/dev/vcio`, which no Faraday kernel
//! builds (`boards/pi3/kernel.forbidden`). Nothing here is the person's:
//! these are the board's numbers.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// How often the figures are read while the app runs.
const SAMPLE_EVERY: Duration = Duration::from_secs(1);

/// The most thermal zones followed.
const ZONES: usize = 8;

/// A figure now and the lowest and highest it has been since the shell
/// started.
#[derive(Clone, Copy, Default)]
struct Seen {
    now: i64,
    low: i64,
    high: i64,
}

impl Seen {
    fn add(slot: &mut Option<Seen>, v: i64) {
        *slot = Some(match *slot {
            None => Seen {
                now: v,
                low: v,
                high: v,
            },
            Some(s) => Seen {
                now: v,
                low: s.low.min(v),
                high: s.high.max(v),
            },
        });
    }
}

/// What the shell has seen of the board since it started.
pub struct System {
    /// Where `/proc` and `/sys` are: `/` on the device, a folder in a
    /// test.
    root: PathBuf,
    /// The shell's clock when the figures are read next.
    next: Duration,
    /// The kernel's uptime, ms, when the app's first frame was drawn.
    first_frame_ms: Option<u64>,
    /// Each thermal zone's type and its temperature in millidegrees.
    zones: Vec<(String, Option<Seen>)>,
    /// The first CPU's clock, kHz.
    clock: Option<Seen>,
    /// The Raspberry Pi's undervoltage alarm: now and whether it has been
    /// raised since the shell started; `None` without the sensor.
    undervoltage: Option<(bool, bool)>,
    /// `MemAvailable`, KiB.
    available: Option<Seen>,
}

impl System {
    /// Nothing seen yet of the board whose files are under `root`.
    pub fn new(root: &Path) -> System {
        System {
            root: root.to_path_buf(),
            next: Duration::ZERO,
            first_frame_ms: None,
            zones: Vec::new(),
            clock: None,
            undervoltage: None,
            available: None,
        }
    }

    fn read(&self, rel: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(rel))
            .ok()
            .map(|s| s.trim().to_owned())
    }

    /// Notes the app's first frame, by the kernel's uptime, the first
    /// time it is called. Costs a read of `/proc/uptime`.
    pub fn first_frame(&mut self) {
        if self.first_frame_ms.is_none() {
            self.first_frame_ms = self.read("proc/uptime").and_then(|t| uptime_ms(&t));
        }
    }

    /// Costs a few reads of `/proc` and `/sys`: reads the figures, if a
    /// second of the shell's clock has gone by since they were last read.
    pub fn sample(&mut self, elapsed: Duration) {
        if elapsed < self.next {
            return;
        }
        self.next = elapsed + SAMPLE_EVERY;
        self.sample_zones();
        if let Some(khz) = self
            .read("sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq")
            .and_then(|s| s.parse().ok())
        {
            Seen::add(&mut self.clock, khz);
        }
        if let Some(alarm) = self.undervoltage_alarm() {
            let ever = self.undervoltage.is_some_and(|(_, e)| e) || alarm;
            self.undervoltage = Some((alarm, ever));
        }
        if let Some(kib) = self
            .read("proc/meminfo")
            .and_then(|m| meminfo_kib(&m, "MemAvailable"))
        {
            Seen::add(&mut self.available, kib);
        }
    }

    fn sample_zones(&mut self) {
        let dir = self.root.join("sys/class/thermal");
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.starts_with("thermal_zone"))
            .collect();
        names.sort();
        names.truncate(ZONES);
        for name in names {
            let kind = self
                .read(&format!("sys/class/thermal/{name}/type"))
                .unwrap_or_else(|| name.clone());
            let Some(milli) = self
                .read(&format!("sys/class/thermal/{name}/temp"))
                .and_then(|t| t.parse::<i64>().ok())
            else {
                continue;
            };
            let at = match self.zones.iter().position(|(k, _)| *k == kind) {
                Some(i) => i,
                None => {
                    self.zones.push((kind, None));
                    self.zones.len() - 1
                }
            };
            Seen::add(&mut self.zones[at].1, milli);
        }
    }

    /// The Raspberry Pi's undervoltage alarm, from the firmware's hwmon
    /// sensor (`rpi_volt`), which reads through the in-kernel firmware
    /// driver and not through `/dev/vcio`. `None` where there is none.
    fn undervoltage_alarm(&self) -> Option<bool> {
        let dir = self.root.join("sys/class/hwmon");
        let mut hw: Vec<String> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect();
        hw.sort();
        hw.into_iter().find_map(|h| {
            (self.read(&format!("sys/class/hwmon/{h}/name"))? == "rpi_volt")
                .then(|| self.read(&format!("sys/class/hwmon/{h}/in0_lcrit_alarm")))
                .flatten()
                .map(|a| a != "0")
        })
    }

    /// The block, with what the kernel log `kernel` says of the random
    /// generator and the power supply. Costs a read of
    /// `/proc/self/status`.
    pub fn block(&self, kernel: Option<&str>) -> String {
        let mut out = String::from("\n== system (sampled once a second) ==\n");
        line(
            &mut out,
            "first frame",
            &self.first_frame_ms.map_or_else(
                || String::from("not drawn yet"),
                |ms| format!("{} s after the kernel started", secs(ms)),
            ),
        );
        let random = kernel.map(|log| log_stamp(log, &["crng init done", "crng init: done"]));
        line(
            &mut out,
            "random ready",
            &match random {
                None => String::from("not known: no kernel log"),
                Some(None) => String::from("not in the kernel log (no \"crng init done\")"),
                Some(Some(s)) => format!("{s} s after the kernel started (crng init done)"),
            },
        );
        if self.zones.is_empty() {
            line(
                &mut out,
                "temperature",
                "not available: no /sys/class/thermal zone",
            );
        }
        for (kind, seen) in &self.zones {
            let Some(s) = seen else { continue };
            let text = format!("{kind} now {} C, highest {} C", milli(s.now), milli(s.high));
            line(&mut out, "temperature", &text);
        }
        line(
            &mut out,
            "cpu0 clock",
            &match self.clock {
                None => String::from("not available: no cpufreq for cpu0"),
                Some(s) => format!(
                    "now {} MHz, lowest {} MHz, highest {} MHz",
                    s.now / 1000,
                    s.low / 1000,
                    s.high / 1000
                ),
            },
        );
        self.power(&mut out, kernel);
        self.memory(&mut out);
        out
    }

    /// The power supply's lines: the undervoltage alarm and the
    /// warnings the kernel log holds, and the throttling flags.
    fn power(&self, out: &mut String, kernel: Option<&str>) {
        let warned = kernel.map_or(0, |log| {
            log.lines()
                .filter(|l| {
                    let l = l.to_ascii_lowercase();
                    l.contains("undervoltage detected") || l.contains("under-voltage detected")
                })
                .count()
        });
        line(
            out,
            "undervoltage",
            &match self.undervoltage {
                Some((now, ever)) => format!(
                    "now {}, since start {}, kernel log warnings {warned}",
                    yes(now),
                    yes(ever)
                ),
                None => format!(
                    "not available: no rpi_volt sensor in /sys/class/hwmon; kernel log warnings {warned}"
                ),
            },
        );
        line(
            out,
            "throttling",
            "not available: the firmware tells it only through /dev/vcio, which is not built",
        );
    }

    /// The memory's lines: what the kernel has free, and what the app has
    /// held. Costs a read of `/proc/self/status`.
    fn memory(&self, out: &mut String) {
        line(
            out,
            "memory available",
            &match self.available {
                None => String::from("not available: no MemAvailable in /proc/meminfo"),
                Some(s) => format!("now {} MiB, lowest {} MiB", s.now / 1024, s.low / 1024),
            },
        );
        let status = self.read("proc/self/status");
        let peak = status.as_deref().and_then(|s| meminfo_kib(s, "VmHWM"));
        let rss = status.as_deref().and_then(|s| meminfo_kib(s, "VmRSS"));
        line(
            out,
            "app memory",
            &match (peak, rss) {
                (Some(p), Some(r)) => format!("peak {} MiB, now {} MiB", p / 1024, r / 1024),
                _ => String::from("not available: no VmHWM in /proc/self/status"),
            },
        );
    }
}

/// One line of the block: what, in a column, then the figures.
fn line(out: &mut String, what: &str, text: &str) {
    out.push_str(&format!("{what:<18} {text}\n"));
}

/// `yes` or `no`.
fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// Milliseconds as seconds to a hundredth.
fn secs(ms: u64) -> String {
    format!("{}.{:02}", ms / 1000, ms % 1000 / 10)
}

/// Millidegrees as degrees to a tenth.
fn milli(m: i64) -> String {
    let sign = if m < 0 { "-" } else { "" };
    let m = m.abs();
    format!("{sign}{}.{}", m / 1000, m % 1000 / 100)
}

/// `/proc/uptime`'s first figure, ms.
fn uptime_ms(text: &str) -> Option<u64> {
    let (whole, frac) = text.split_whitespace().next()?.split_once('.')?;
    let frac: String = frac.chars().chain("000".chars()).take(3).collect();
    Some(whole.parse::<u64>().ok()? * 1000 + frac.parse::<u64>().ok()?)
}

/// A `Name:   1234 kB` line's figure, KiB.
fn meminfo_kib(text: &str, name: &str) -> Option<i64> {
    text.lines().find_map(|l| {
        let rest = l.strip_prefix(name)?.strip_prefix(':')?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

/// The stamp, in seconds as the log prints it, of the first kernel log
/// line holding one of `marks`: `[    2.415327] random: crng init done`
/// gives `2.415327`.
fn log_stamp(log: &str, marks: &[&str]) -> Option<String> {
    log.lines()
        .find(|l| marks.iter().any(|m| l.contains(m)))
        .and_then(|l| {
            let inside = l.trim_start().strip_prefix('[')?.split(']').next()?;
            Some(inside.trim().to_owned())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake `/` with `files` written under it.
    fn tree(name: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("faraday-sys-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in files {
            let p = root.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        root
    }

    fn set(root: &Path, path: &str, text: &str) {
        std::fs::write(root.join(path), text).unwrap();
    }

    fn row<'a>(block: &'a str, what: &str) -> Vec<&'a str> {
        block
            .lines()
            .filter(|l| l.starts_with(what))
            .map(|l| l[18..].trim_start())
            .collect()
    }

    const LOG: &str = "[    0.000000] Booting Linux\n\
                       [    2.415327] random: crng init done\n\
                       [   40.100000] hwmon hwmon1: Undervoltage detected!\n";

    #[test]
    fn the_block_gives_each_figure_now_and_at_its_worst() {
        let root = tree(
            "worst",
            &[
                ("proc/uptime", "6.83 12.00\n"),
                ("sys/class/thermal/thermal_zone0/type", "cpu-thermal\n"),
                ("sys/class/thermal/thermal_zone0/temp", "48312\n"),
                (
                    "sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
                    "1400000\n",
                ),
                ("sys/class/hwmon/hwmon1/name", "rpi_volt\n"),
                ("sys/class/hwmon/hwmon1/in0_lcrit_alarm", "0\n"),
                (
                    "proc/meminfo",
                    "MemTotal: 948000 kB\nMemAvailable: 729088 kB\n",
                ),
                (
                    "proc/self/status",
                    "VmHWM:\t 190464 kB\nVmRSS:\t 94208 kB\n",
                ),
            ],
        );
        let mut s = System::new(&root);
        s.first_frame();
        s.sample(Duration::ZERO);
        set(&root, "sys/class/thermal/thermal_zone0/temp", "61250\n");
        set(
            &root,
            "sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
            "600000\n",
        );
        set(&root, "sys/class/hwmon/hwmon1/in0_lcrit_alarm", "1\n");
        set(&root, "proc/meminfo", "MemAvailable: 411648 kB\n");
        // Within the second: not read again.
        s.sample(Duration::from_millis(500));
        assert_eq!(
            row(&s.block(None), "cpu0 clock"),
            ["now 1400 MHz, lowest 1400 MHz, highest 1400 MHz"]
        );
        s.sample(Duration::from_secs(1));
        set(&root, "sys/class/thermal/thermal_zone0/temp", "52000\n");
        set(
            &root,
            "sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq",
            "1200000\n",
        );
        set(&root, "sys/class/hwmon/hwmon1/in0_lcrit_alarm", "0\n");
        set(&root, "proc/meminfo", "MemAvailable: 524288 kB\n");
        s.sample(Duration::from_secs(2));
        let block = s.block(Some(LOG));
        assert_eq!(
            row(&block, "first frame"),
            ["6.83 s after the kernel started"]
        );
        assert_eq!(
            row(&block, "random ready"),
            ["2.415327 s after the kernel started (crng init done)"]
        );
        assert_eq!(
            row(&block, "temperature"),
            ["cpu-thermal now 52.0 C, highest 61.2 C"]
        );
        assert_eq!(
            row(&block, "cpu0 clock"),
            ["now 1200 MHz, lowest 600 MHz, highest 1400 MHz"]
        );
        assert_eq!(
            row(&block, "undervoltage"),
            ["now no, since start yes, kernel log warnings 1"]
        );
        assert_eq!(
            row(&block, "memory available"),
            ["now 512 MiB, lowest 402 MiB"]
        );
        assert_eq!(row(&block, "app memory"), ["peak 186 MiB, now 92 MiB"]);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn what_the_kernel_does_not_expose_is_said_to_be_not_available() {
        let root = tree("bare", &[("proc/uptime", "3.10 1.00\n")]);
        let mut s = System::new(&root);
        s.sample(Duration::ZERO);
        let block = s.block(Some("[    0.0] Booting Linux\n"));
        assert_eq!(row(&block, "first frame"), ["not drawn yet"]);
        assert_eq!(
            row(&block, "random ready"),
            ["not in the kernel log (no \"crng init done\")"]
        );
        for what in [
            "temperature",
            "cpu0 clock",
            "undervoltage",
            "throttling",
            "memory available",
            "app memory",
        ] {
            let r = row(&block, what);
            assert!(
                r.len() == 1 && r[0].starts_with("not available"),
                "{what}: {r:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
