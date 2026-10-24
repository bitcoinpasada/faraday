//! The dev image's boot report: `faraday-boot-report.txt` on the boot
//! stick, the kernel log and then what the shell found, written again
//! whole every [`WRITE_EVERY`] while the app runs and once more when it
//! ends.
//!
//! A laptop has no serial port, so a dev stick that will not come up, or
//! one that came up and did something odd, has nowhere to say so. The
//! person boots it, uses it, pulls it and reads this file on another
//! computer. It holds two things and nothing else: the kernel log as
//! `dmesg` prints it, which in the dev image includes the app's own
//! error output (the dev inittab sends it to `/dev/kmsg`), and the
//! inventory: the panel, every input device with what it was taken for,
//! every capture device. The app prints no key material: the core crates
//! may not print at all (`tools/lint-secrets.sh`, rule 1), and the shell
//! prints sizes, names, counts and reasons.
//!
//! The release image passes no `--boot-report`, so none of this runs.
//! The dev inittab also lets the app read the kernel log
//! (`dmesg_restrict`), which the release image keeps closed.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// How often the whole file is written again while the app runs.
pub const WRITE_EVERY: Duration = Duration::from_secs(30);

/// How often a first write is tried while there is nowhere to put it yet.
const RETRY: Duration = Duration::from_secs(1);

/// The most of the kernel log kept: the newest. The ring buffer is
/// smaller than this on a stock kernel; the cap is for a log that is not.
const MAX_LOG: usize = 1024 * 1024;

/// The program that prints the kernel log, which the dev image's BusyBox
/// has (`variants/dev/busybox.fragment`).
pub const DMESG: &str = "/bin/dmesg";

/// The report this run keeps: its file's name, what the shell found
/// (which does not change while it runs), and when it is written next.
pub struct Report {
    pub name: String,
    pub inventory: String,
    pub schedule: Schedule,
    /// A failure has been said already, so it is not said every time.
    pub said: bool,
}

impl Report {
    pub fn new(inventory: String) -> Report {
        Report {
            name: String::from(crate::files::BOOT_REPORT),
            inventory,
            schedule: Schedule::new(),
            said: false,
        }
    }
}

/// When the report is written next.
pub struct Schedule {
    /// The shell's clock when the report was last written.
    wrote: Option<Duration>,
    /// The shell's clock when a first write was last tried, so a stick
    /// that is not listed yet is asked about once a second and not once a
    /// pass.
    tried: Option<Duration>,
}

impl Schedule {
    pub fn new() -> Schedule {
        Schedule {
            wrote: None,
            tried: None,
        }
    }

    /// Whether to write now: the first time as soon as there is
    /// somewhere to (tried once a second until then), then every
    /// [`WRITE_EVERY`], and at the end whenever it is.
    pub fn due(&self, elapsed: Duration, last: bool) -> bool {
        if last {
            return true;
        }
        match (self.wrote, self.tried) {
            (Some(w), _) => elapsed >= w + WRITE_EVERY,
            (None, Some(t)) => elapsed >= t + RETRY,
            (None, None) => true,
        }
    }

    /// Records a try at `elapsed` and whether it wrote.
    pub fn tried(&mut self, elapsed: Duration, wrote: bool) {
        self.tried = Some(elapsed);
        if wrote {
            self.wrote = Some(elapsed);
        }
    }
}

/// Costs a process: `program`'s output, the newest [`MAX_LOG`] bytes of
/// it, as text. The reason when it cannot be run.
pub fn kernel_log(program: &Path) -> Result<String, String> {
    let out = Command::new(program)
        .output()
        .map_err(|e| format!("cannot run {}: {e}", program.display()))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "{} ended with {}: {}",
            program.display(),
            out.status,
            why.trim()
        ));
    }
    let mut bytes = out.stdout.as_slice();
    if bytes.len() > MAX_LOG {
        bytes = &bytes[bytes.len() - MAX_LOG..];
        // Whole lines: the cut is at the next line start.
        if let Some(n) = bytes.iter().position(|&b| b == b'\n') {
            bytes = &bytes[n + 1..];
        }
    }
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// The file: the kernel log (or why there is none), then the inventory.
/// `elapsed` is the shell's clock, so a reader can tell how recent the
/// write is against the log's own stamps.
pub fn compose(kernel: &Result<String, String>, inventory: &str, elapsed: Duration) -> String {
    let mut out = String::from("== kernel log ==\n");
    match kernel {
        Ok(log) => {
            out.push_str(log);
            if !log.ends_with('\n') {
                out.push('\n');
            }
        }
        Err(why) => out.push_str(&format!("no kernel log: {why}\n")),
    }
    out.push_str(&format!(
        "\n== faraday (written {} s after the shell started) ==\n",
        elapsed.as_secs()
    ));
    out.push_str(inventory);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_write_is_tried_at_once_and_then_every_second_until_it_lands() {
        let mut s = Schedule::new();
        assert!(s.due(Duration::from_millis(10), false));
        s.tried(Duration::from_millis(10), false);
        assert!(!s.due(Duration::from_millis(500), false));
        assert!(s.due(Duration::from_millis(1100), false));
        s.tried(Duration::from_millis(1100), true);
        assert!(!s.due(Duration::from_secs(30), false));
        assert!(s.due(Duration::from_millis(31_100), false));
    }

    #[test]
    fn the_last_write_is_always_due() {
        let mut s = Schedule::new();
        s.tried(Duration::from_secs(5), true);
        assert!(!s.due(Duration::from_secs(6), false));
        assert!(s.due(Duration::from_secs(6), true));
    }

    #[test]
    fn the_report_is_the_kernel_log_then_the_inventory_and_nothing_else() {
        let text = compose(
            &Ok(String::from("[    0.1] boot\n[    1.0] faraday: up")),
            "panel 800x600\n",
            Duration::from_secs(42),
        );
        assert_eq!(
            text,
            "== kernel log ==\n[    0.1] boot\n[    1.0] faraday: up\n\n\
             == faraday (written 42 s after the shell started) ==\npanel 800x600\n"
        );
    }

    #[test]
    fn a_missing_kernel_log_is_said_and_the_inventory_still_written() {
        let text = compose(
            &Err(String::from("no dmesg")),
            "input: none\n",
            Duration::ZERO,
        );
        assert!(text.contains("no kernel log: no dmesg"));
        assert!(text.ends_with("input: none\n"));
    }

    #[test]
    fn the_kernel_log_is_what_the_program_prints() {
        let dir = std::env::temp_dir().join(format!("faraday-dmesg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("dmesg");
        std::fs::write(&program, "#!/bin/sh\necho '[    0.5] hello'\n").unwrap();
        Command::new("chmod")
            .arg("+x")
            .arg(&program)
            .status()
            .unwrap();
        assert_eq!(kernel_log(&program).unwrap(), "[    0.5] hello\n");
        assert!(kernel_log(&dir.join("absent")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
