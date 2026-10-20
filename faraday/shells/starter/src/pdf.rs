//! A sheet's PDF on this computer: a blank template (`docs/STARTER.md`
//! §3.8) or the Start page's things to write down (§3.4, F47). Made in
//! the system's temporary folder for Print, and opened in the system's
//! PDF viewer, which prints to any printer or to a file. The PDF holds
//! nothing of any wallet. The viewer is the system's own "open with the
//! default program" (`xdg-open`, `open`, `explorer.exe`); never a print
//! verb that sends a file to the default printer unasked.
//!
//! A sheet that carries the BitLocker recovery key (F49) goes into a
//! folder named `faraday-key-…` of its own: the Starter overwrites and
//! removes it when it exits ([`remove_key_sheets`]), and at its next
//! start removes any an earlier run left behind (a crash, a power cut)
//! ([`sweep_key_sheets`]).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use faraday_starter::api::Reason;

fn file_reason(path: &Path, e: impl ToString) -> Reason {
    Reason::File {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

/// The folder prefix of a sheet that carries the recovery key.
const KEY_PREFIX: &str = "faraday-key-";

/// Writes `pdf` as `name` in a new folder of its own under the system's
/// temporary folder. Returns its path. A file write.
pub fn write_temp(name: &str, pdf: &[u8]) -> Result<PathBuf, Reason> {
    write_in(std::env::temp_dir(), "faraday-print-", name, pdf)
}

/// Writes a sheet that carries the recovery key (F49) as `name` in a new
/// `faraday-key-…` folder under the system's temporary folder, which
/// [`remove_key_sheets`] removes. Returns its path. A file write.
pub fn write_key_sheet(name: &str, pdf: &[u8]) -> Result<PathBuf, Reason> {
    write_in(std::env::temp_dir(), KEY_PREFIX, name, pdf)
}

/// Overwrites every file in `dir` with zeros, then removes the folder.
/// Best effort: a file the viewer still holds open on Windows may stay
/// until the next start's sweep. Reads and writes files.
fn wipe_dir(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if let Ok(len) = std::fs::metadata(&p).map(|m| m.len()) {
                let _ = std::fs::write(&p, vec![0u8; len as usize]);
            }
            let _ = std::fs::remove_file(&p);
        }
    }
    let _ = std::fs::remove_dir(dir);
}

/// Removes the key sheets' folders this run made, each file overwritten
/// first. Called when the Starter exits. Writes and removes files.
pub fn remove_key_sheets(dirs: &[PathBuf]) {
    for d in dirs {
        wipe_dir(d);
    }
}

/// Removes every `faraday-key-…` folder in `base` (the system's
/// temporary folder), each file overwritten first: what an earlier run
/// could not remove. Called at start. Reads, writes and removes files.
pub fn sweep_key_sheets(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for e in entries.flatten() {
        let named = e
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with(KEY_PREFIX));
        if named && e.file_type().is_ok_and(|t| t.is_dir()) {
            wipe_dir(&e.path());
        }
    }
}

/// Writes `pdf` as `name` in a new folder `prefix…` of its own under
/// `base`. Returns its path. A file write.
pub fn write_in(base: PathBuf, prefix: &str, name: &str, pdf: &[u8]) -> Result<PathBuf, Reason> {
    let name = Path::new(name)
        .file_name()
        .ok_or_else(|| file_reason(&base, "no file name"))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = base.join(format!("{prefix}{}-{stamp}", std::process::id()));
    std::fs::create_dir(&dir).map_err(|e| file_reason(&dir, e))?;
    let path = dir.join(name);
    std::fs::write(&path, pdf).map_err(|e| file_reason(&path, e))?;
    Ok(path)
}

/// The program that opens a file with the system's default one.
fn opener() -> PathBuf {
    #[cfg(windows)]
    {
        // From the system folder, not from the search path.
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("explorer.exe")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/usr/bin/open")
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        PathBuf::from("xdg-open")
    }
}

/// Opens a file in the system's default program for it.
pub fn open(path: &Path) -> Result<(), Reason> {
    let program = opener();
    let mut child = Command::new(&program)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => Reason::Missing(program.display().to_string()),
            _ => file_reason(path, e),
        })?;
    // Reaped when it ends, without holding the Starter up.
    std::thread::spawn(move || child.wait());
    Ok(())
}
