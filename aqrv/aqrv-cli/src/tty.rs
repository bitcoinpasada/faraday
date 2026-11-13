//! Reading a passphrase from the terminal, not from an argument.
//!
//! On Unix the terminal is `/dev/tty` and echo is turned off with `stty`,
//! run through `std::process::Command`, so the crate has no `unsafe`. The
//! settings it found are put back by a guard's `Drop`, which runs on every
//! way out of [`prompt`]. Interrupt is read as a character while the
//! prompt is up, so that the terminal is never left silent by a signal.
//! On Windows the line is read with echo on and a note says so.

use std::io::{self, BufRead, BufReader, Write};

use faraday_text::Text;
use zeroize::Zeroizing;

/// The character a terminal sends for interrupt (Ctrl-C).
const INTERRUPT: u8 = 0x03;

/// Asks `label` on the terminal and returns one line, without its line
/// ending. A line holding an interrupt character is an error.
pub fn prompt(label: &str) -> io::Result<Text> {
    let mut tty = open()?;
    write!(tty.output, "{label}")?;
    tty.output.flush()?;
    let line = {
        let _quiet = quiet(&tty)?;
        let mut reader = BufReader::new(&tty.input);
        let mut bytes = Zeroizing::new(Vec::new());
        reader.read_until(b'\n', &mut bytes)?;
        bytes
    };
    writeln!(tty.output)?;
    finish(line)
}

fn finish(mut bytes: Zeroizing<Vec<u8>>) -> io::Result<Text> {
    if bytes.contains(&INTERRUPT) {
        return Err(io::Error::new(io::ErrorKind::Interrupted, "interrupted"));
    }
    if bytes.ends_with(b"\n") {
        bytes.pop();
        if bytes.ends_with(b"\r") {
            bytes.pop();
        }
    }
    core::str::from_utf8(&bytes)
        .map(Text::of)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "input is not UTF-8"))
}

#[cfg(unix)]
struct Tty {
    input: std::fs::File,
    output: std::fs::File,
}

#[cfg(unix)]
fn open() -> io::Result<Tty> {
    let input = std::fs::File::open("/dev/tty")?;
    let output = std::fs::OpenOptions::new().write(true).open("/dev/tty")?;
    Ok(Tty { input, output })
}

/// Echo and signals off while it lives, the saved settings back after.
#[cfg(unix)]
struct Quiet {
    input: std::fs::File,
    saved: Text,
}

#[cfg(unix)]
fn quiet(tty: &Tty) -> io::Result<Quiet> {
    use std::process::{Command, Stdio};

    let saved = Command::new("stty")
        .arg("-g")
        .stdin(Stdio::from(tty.input.try_clone()?))
        .stderr(Stdio::null())
        .output()?;
    if !saved.status.success() {
        return Err(io::Error::other("stty did not read the terminal"));
    }
    let saved = Text::of(Text::from_utf8_lossy(&saved.stdout).trim());
    // The guard exists before echo goes off, so a failure half way restores.
    let guard = Quiet {
        input: tty.input.try_clone()?,
        saved,
    };
    let status = Command::new("stty")
        .args(["-echo", "-isig"])
        .stdin(Stdio::from(tty.input.try_clone()?))
        .status()?;
    if !status.success() {
        return Err(io::Error::other("stty did not turn echo off"));
    }
    Ok(guard)
}

#[cfg(unix)]
impl Drop for Quiet {
    fn drop(&mut self) {
        use std::process::{Command, Stdio};

        if let Ok(input) = self.input.try_clone() {
            let _ = Command::new("stty")
                .arg(self.saved.as_str())
                .stdin(Stdio::from(input))
                .status();
        }
    }
}

#[cfg(not(unix))]
struct Tty {
    input: std::fs::File,
    output: io::Stderr,
}

#[cfg(not(unix))]
fn open() -> io::Result<Tty> {
    let input = std::fs::File::open("CONIN$")?;
    let mut output = io::stderr();
    writeln!(output, "Note: the passphrase is shown as it is typed.")?;
    Ok(Tty { input, output })
}

#[cfg(not(unix))]
struct Quiet;

#[cfg(not(unix))]
fn quiet(_tty: &Tty) -> io::Result<Quiet> {
    Ok(Quiet)
}
