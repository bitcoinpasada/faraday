//! The elevated writer's arguments (`docs/STARTER.md` §2): the Starter
//! starts itself through the elevation prompt with `--write` and the
//! confirmed order, and the elevated copy reads them back before
//! anything else in it starts. Everything here is plain text, so the
//! tests check the round trip and the refusals on any system.
//!
//! ```text
//! --write --pipe \\.\pipe\faraday-starter-… --disk \\.\PhysicalDrive2
//!   --device "Disk 2" --serial S --size N --mode whole|keep-data
//!   (--found PATH --img-sha256 HEX | --image PATH --image-size N) --sha256 HEX
//!   [--boot-file-sha256 HEX] [--name NAME]
//!   [--clear-first yes]
//! ```
//!
//! `--boot-file-sha256` says a boot file the person signed (§4.5) is to
//! be written, and the SHA-256 it had when it was chosen and checked. No
//! path is passed: the window sends the file's bytes over the pipe once
//! the elevated copy has connected (`boot_block`), and the elevated
//! writer refuses them unless they have that sum, and checks them
//! against the release again before anything is written.
//!
//! Keep data on a stick whose version Windows did not let the window
//! read starts it with `--read-version` first:
//!
//! ```text
//! --read-version --pipe \\.\pipe\faraday-starter-… --disk \\.\PhysicalDrive2
//!   --serial S --size N
//! ```
//!
//! Each key once, in any order; nothing else. A value that is not what
//! the key takes refuses the whole list before any disk or file is
//! opened. The writer then checks the disk again itself (§4.1).

use faraday_starter::api::{BootFile, Mode, Source, WriteOrder};

use super::disks::{disk_device, disk_number};

/// The first argument of the elevated copy.
pub const WRITE: &str = "--write";
/// The first argument of the elevated copy that reads a stick's version.
pub const READ_VERSION: &str = "--read-version";
/// Every pipe the Starter makes starts so.
pub const PIPE_PREFIX: &str = r"\\.\pipe\faraday-starter-";

/// The longest serial and image path taken.
const MAX_SERIAL: usize = 256;
const MAX_PATH: usize = 1024;

/// A confirmed write, as the elevated copy is given it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Elevated {
    /// The pipe progress goes back over.
    pub pipe: String,
    /// The order.
    pub order: WriteOrder,
    /// A developer's image's size, as Confirm showed it.
    pub image_size: Option<u64>,
}

impl Elevated {
    /// The arguments, `--write` first.
    pub fn args(&self) -> Vec<String> {
        let o = &self.order;
        let mut a: Vec<String> = vec![
            WRITE.into(),
            "--pipe".into(),
            self.pipe.clone(),
            "--disk".into(),
            o.disk.clone(),
            "--device".into(),
            o.device.clone(),
            "--serial".into(),
            o.serial.clone(),
            "--size".into(),
            o.size.to_string(),
            "--mode".into(),
            match o.mode {
                Mode::Whole => "whole",
                Mode::KeepData => "keep-data",
            }
            .into(),
        ];
        match &o.source {
            Source::Found {
                path,
                xz_sha256,
                img_sha256,
            } => {
                a.extend(["--found".into(), path.clone()]);
                a.extend(["--sha256".into(), xz_sha256.clone()]);
                a.extend(["--img-sha256".into(), img_sha256.clone()]);
            }
            Source::Image { path, sha256 } => {
                a.extend(["--image".into(), path.clone()]);
                a.extend([
                    "--image-size".into(),
                    self.image_size.unwrap_or(0).to_string(),
                ]);
                a.extend(["--sha256".into(), sha256.clone()]);
            }
        }
        if let Some(f) = &o.boot_file {
            a.extend(["--boot-file-sha256".into(), f.sha256.clone()]);
        }
        if let Some(n) = &o.name {
            a.extend(["--name".into(), n.clone()]);
        }
        if o.clear_first {
            a.extend(["--clear-first".into(), "yes".into()]);
        }
        a
    }

    /// Reads the arguments after the program's name, `--write` first,
    /// or says what is wrong with them.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub fn parse(args: &[String]) -> Result<Elevated, String> {
        let (first, rest) = args.split_first().ok_or("no arguments")?;
        if first != WRITE {
            return Err(format!("{first}: not {WRITE}"));
        }
        if !rest.len().is_multiple_of(2) {
            return Err(String::from("a key without its value"));
        }
        const KEYS: [&str; 14] = [
            "--pipe",
            "--disk",
            "--device",
            "--serial",
            "--size",
            "--mode",
            "--image",
            "--image-size",
            "--sha256",
            "--boot-file-sha256",
            "--name",
            "--clear-first",
            "--found",
            "--img-sha256",
        ];
        let mut got: [Option<&str>; 14] = [None; 14];
        for pair in rest.chunks(2) {
            let i = KEYS
                .iter()
                .position(|k| *k == pair[0])
                .ok_or_else(|| format!("{}: not a key", pair[0]))?;
            if got[i].replace(pair[1].as_str()).is_some() {
                return Err(format!("{} twice", pair[0]));
            }
        }
        let need = |i: usize| got[i].ok_or_else(|| format!("{} missing", KEYS[i]));
        let pipe = need(0)?;
        check_pipe(pipe)?;
        let disk = need(1)?;
        let n = disk_number(disk).ok_or_else(|| format!("--disk {disk}: not a disk"))?;
        let device = need(2)?;
        if device != disk_device(n) {
            return Err(format!("--device {device}: not {disk}'s"));
        }
        let serial = need(3)?;
        check_serial(serial)?;
        let size = number(need(4)?).ok_or("--size: not a size")?;
        let mode = match need(5)? {
            "whole" => Mode::Whole,
            "keep-data" => Mode::KeepData,
            m => return Err(format!("--mode {m}: not a mode")),
        };
        let sha256 = need(8)?;
        if !hex_sum(sha256) {
            return Err(String::from("--sha256: not a SHA-256"));
        }
        // The file itself arrives over the pipe; the order names it by
        // its sum alone, and the window words a refusal with its path.
        let boot_file = match got[9] {
            None => None,
            Some(sum) if hex_sum(sum) => Some(BootFile {
                path: String::new(),
                sha256: sum.to_string(),
            }),
            Some(_) => return Err(String::from("--boot-file-sha256: not a SHA-256")),
        };
        let name = match got[10] {
            None => None,
            Some(n) if crate::ids::label_of(n).is_some() => Some(n.to_string()),
            Some(n) => return Err(format!("--name {n}: not a data partition's name")),
        };
        let clear_first = match got[11] {
            None => false,
            Some("yes") if mode == Mode::Whole => true,
            Some(c) => return Err(format!("--clear-first {c}: not asked for here")),
        };
        let (source, image_size) = match (got[6], got[7]) {
            (None, None) if got[12].is_some() => {
                let (path, img) = (got[12].unwrap_or_default(), got[13].unwrap_or_default());
                if !absolute(path) || path.len() > MAX_PATH || path.chars().any(char::is_control) {
                    return Err(String::from("--found: not a full path"));
                }
                if !hex_sum(img) {
                    return Err(String::from("--img-sha256: not a SHA-256"));
                }
                (
                    Source::Found {
                        path: path.to_string(),
                        xz_sha256: sha256.to_string(),
                        img_sha256: img.to_string(),
                    },
                    None,
                )
            }
            _ if got[12].is_some() || got[13].is_some() => {
                return Err(String::from("--found stands alone, with --img-sha256"));
            }
            (Some(path), Some(len)) => {
                if !absolute(path) || path.len() > MAX_PATH || path.chars().any(char::is_control) {
                    return Err(String::from("--image: not a full path"));
                }
                let len = number(len).ok_or("--image-size: not a size")?;
                (
                    Source::Image {
                        path: path.to_string(),
                        sha256: sha256.to_string(),
                    },
                    Some(len),
                )
            }
            _ => {
                return Err(String::from("one of --found or --image with --image-size"));
            }
        };
        Ok(Elevated {
            pipe: pipe.to_string(),
            order: WriteOrder {
                source,
                disk: disk.to_string(),
                device: device.to_string(),
                serial: serial.to_string(),
                size,
                mode,
                boot_file,
                name,
                clear_first,
            },
            image_size,
        })
    }
}

/// A stick's version to read, as the elevated copy is given it: the
/// disk the window listed, by its serial and size as listed then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadVersion {
    /// The pipe the answer goes back over.
    pub pipe: String,
    /// The disk (`\\.\PhysicalDriveN`).
    pub disk: String,
    /// Its serial as listed: the elevated copy refuses another.
    pub serial: String,
    /// Its size in bytes as listed: the elevated copy refuses another.
    pub size: u64,
}

impl ReadVersion {
    /// The arguments, `--read-version` first.
    pub fn args(&self) -> Vec<String> {
        vec![
            READ_VERSION.into(),
            "--pipe".into(),
            self.pipe.clone(),
            "--disk".into(),
            self.disk.clone(),
            "--serial".into(),
            self.serial.clone(),
            "--size".into(),
            self.size.to_string(),
        ]
    }

    /// Reads the arguments after the program's name, `--read-version`
    /// first, or says what is wrong with them.
    pub fn parse(args: &[String]) -> Result<ReadVersion, String> {
        let (first, rest) = args.split_first().ok_or("no arguments")?;
        if first != READ_VERSION {
            return Err(format!("{first}: not {READ_VERSION}"));
        }
        if !rest.len().is_multiple_of(2) {
            return Err(String::from("a key without its value"));
        }
        const KEYS: [&str; 4] = ["--pipe", "--disk", "--serial", "--size"];
        let mut got: [Option<&str>; 4] = [None; 4];
        for pair in rest.chunks(2) {
            let i = KEYS
                .iter()
                .position(|k| *k == pair[0])
                .ok_or_else(|| format!("{}: not a key", pair[0]))?;
            if got[i].replace(pair[1].as_str()).is_some() {
                return Err(format!("{} twice", pair[0]));
            }
        }
        let need = |i: usize| got[i].ok_or_else(|| format!("{} missing", KEYS[i]));
        let pipe = need(0)?;
        check_pipe(pipe)?;
        let disk = need(1)?;
        disk_number(disk).ok_or_else(|| format!("--disk {disk}: not a disk"))?;
        let serial = need(2)?;
        check_serial(serial)?;
        let size = number(need(3)?).ok_or("--size: not a size")?;
        Ok(ReadVersion {
            pipe: pipe.to_string(),
            disk: disk.to_string(),
            serial: serial.to_string(),
            size,
        })
    }
}

/// A pipe the Starter made: [`PIPE_PREFIX`] and its own hex id.
fn check_pipe(pipe: &str) -> Result<(), String> {
    let ok = pipe.strip_prefix(PIPE_PREFIX).is_some_and(|id| {
        !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
    });
    if ok {
        Ok(())
    } else {
        Err(format!("--pipe {pipe}: not the starter app's pipe"))
    }
}

/// A serial: no longer than [`MAX_SERIAL`], no control characters.
fn check_serial(serial: &str) -> Result<(), String> {
    if serial.chars().count() > MAX_SERIAL || serial.chars().any(char::is_control) {
        Err(String::from("--serial: not a serial"))
    } else {
        Ok(())
    }
}

/// A SHA-256 as `sha256sum` prints it: 64 lower-case hex digits.
fn hex_sum(s: &str) -> bool {
    !s.bytes().any(|b| b.is_ascii_uppercase())
        && faraday_crypto::hex::decode_exact::<32>(s).is_some()
}

/// A size: digits, more than 0.
fn number(s: &str) -> Option<u64> {
    let ok = !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| s.parse().ok()).flatten().filter(|&n| n > 0)
}

/// A Windows full path: `C:\…` or `\\server\…`.
fn absolute(p: &str) -> bool {
    let b = p.as_bytes();
    (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\')
        || p.starts_with(r"\\")
}

/// One argument as Windows' command line carries it, so the program
/// reads it back whole: quoted when it is empty or holds a space, a tab
/// or a quote; a quote escaped by a backslash, and backslashes before a
/// quote doubled.
pub fn quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::from("\"");
    let mut slashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', slashes * 2 + 1));
                out.push('"');
                slashes = 0;
                continue;
            }
            _ => {}
        }
        if c != '\\' {
            out.extend(std::iter::repeat_n('\\', slashes));
            slashes = 0;
            out.push(c);
        }
    }
    out.extend(std::iter::repeat_n('\\', slashes * 2));
    out.push('"');
    out
}

/// The arguments as one command line (`ShellExecuteExW`'s parameters).
pub fn command_line(args: &[String]) -> String {
    args.iter().map(|a| quote(a)).collect::<Vec<_>>().join(" ")
}

/// How a program reads its arguments back from a command line after its
/// own name (the rules Rust's `std::env::args` follows on Windows).
pub fn split(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut started = false;
    let mut quoted = false;
    let mut slashes = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                slashes += 1;
                started = true;
            }
            '"' => {
                cur.extend(std::iter::repeat_n('\\', slashes / 2));
                if !slashes.is_multiple_of(2) {
                    cur.push('"');
                } else if quoted && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    quoted = !quoted;
                }
                slashes = 0;
                started = true;
            }
            ' ' | '\t' if !quoted => {
                cur.extend(std::iter::repeat_n('\\', slashes));
                slashes = 0;
                if started {
                    args.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            _ => {
                cur.extend(std::iter::repeat_n('\\', slashes));
                slashes = 0;
                cur.push(c);
                started = true;
            }
        }
    }
    cur.extend(std::iter::repeat_n('\\', slashes));
    if started {
        args.push(cur);
    }
    args
}
