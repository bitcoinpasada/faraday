//! The desktop file channel (`docs/PLANNING.md` §16.16): one directory,
//! listed for the core and read one file at a time.
//!
//! This shell has no file dialog and takes no GUI toolkit for one
//! (§16.16), so the core's Files screen is the picker: the directory is
//! listed, newest first, and the name the person taps comes back to be
//! read. The rules are the Pi's, which is the same channel over a card:
//! a name ending in `.psbt` for a PSBT, anything but a dot-file for
//! `FileKind::Any`, files only, and never a name that was not listed.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use osk_shell_api::{FileEntry, FileKind};

/// The most that will be handed to the core from one file. A PSBT is
/// kilobytes; anything of this size is not what was asked for.
const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// The most files one listing carries, newest first, so what is cut is
/// the oldest. A directory of downloads can hold thousands.
const MAX_ENTRIES: usize = 64;

/// Every file in `dir` that could be what was asked for, newest first.
pub fn list(dir: &Path, kind: FileKind) -> Result<Vec<FileEntry>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("cannot read {}: {e}", dir.display()))?;
    let mut found: Vec<(SystemTime, FileEntry)> = Vec::new();
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !wanted(name, kind) {
            continue;
        }
        let time = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        found.push((
            time,
            FileEntry {
                name: String::from(name),
                size: meta.len(),
                modified: seconds(time),
            },
        ));
    }
    // Newest first; two files written in the same instant are settled by
    // name, so the list is the same on every run.
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    found.truncate(MAX_ENTRIES);
    Ok(found.into_iter().map(|(_, e)| e).collect())
}

/// The bytes of one file from [`list`], by the name it gave. A name that
/// is not on the list — one with a path in it, one the rule does not
/// allow, one that is gone — is no file at all.
pub fn read(dir: &Path, kind: FileKind, name: &str) -> Result<Vec<u8>, String> {
    if !list(dir, kind)?.iter().any(|e| e.name == name) {
        return Err(format!("{name} is not in {}", dir.display()));
    }
    let path = dir.join(name);
    let size = fs::metadata(&path).map_or(0, |m| m.len());
    if size > MAX_BYTES {
        return Err(format!(
            "{} is {size} bytes, larger than the {MAX_BYTES} this reads",
            path.display()
        ));
    }
    fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

/// The directory the channel starts in: the person's downloads, where
/// a coordinator's transaction lands, and otherwise wherever the shell
/// was started.
pub fn default_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        let downloads = PathBuf::from(home).join("Downloads");
        if downloads.is_dir() {
            return downloads;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// A file's modification time in seconds since the epoch. A desktop has
/// a clock, so this is only `None` for a time before the epoch.
fn seconds(time: SystemTime) -> Option<u64> {
    Some(time.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs())
}

/// Whether a file directly in the directory could be what was asked for.
fn wanted(name: &str, kind: FileKind) -> bool {
    match kind {
        FileKind::Psbt => {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".psbt") && lower.len() > ".psbt".len()
        }
        // Nothing asks for a text *file*: the kind names a clipboard
        // payload, and a listing of it is a listing of everything.
        FileKind::Text | FileKind::Any => !name.starts_with('.'),
        FileKind::Png => {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".png") && lower.len() > ".png".len()
        }
    }
}
