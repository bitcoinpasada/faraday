//! The carry across a lock (02 §4.2): named files in the RAM directory, each written whole
//! (a temporary file, then a rename) in the order given, read once at the next start and
//! removed.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_crypto::{Secret, Zeroize};
use faraday_files::checked_for_write;
use faraday_port::Failure;
use faraday_port::storage::{Carry, Files};
use faraday_text::Text;

/// The three folders of a carry, in the order they are written.
const FOLDERS: [&str; 3] = ["kept", "inbox", "outbox"];

/// Where the carry waits between processes: a RAM directory on the device, memory alone where a
/// lock does not end the process.
pub enum Keeping {
    Dir(PathBuf),
    Memory(Option<Carry>),
}

impl Keeping {
    pub fn dir(path: &str) -> Self {
        Keeping::Dir(PathBuf::from(path))
    }

    pub fn memory() -> Self {
        Keeping::Memory(None)
    }

    /// Keeps the carry. Every file goes through a temporary file and a rename, in the order the
    /// app gave. A failed keep removes what it wrote, zeroing each file, and hands the carry
    /// back (`Err((failure, carry))`), so the app still holds it.
    pub fn keep(&mut self, carry: Carry) -> Result<(), (Failure, Carry)> {
        match self {
            Keeping::Memory(held) => {
                *held = Some(carry);
                Ok(())
            }
            Keeping::Dir(dir) => match write_all(dir, &carry) {
                Ok(()) => {
                    // The adapter's copy is the files; the carry's own wipes as it drops.
                    Ok(())
                }
                Err(why) => {
                    remove_all(dir);
                    Err((why, carry))
                }
            },
        }
    }

    /// Removes every carried file, zeros first.
    pub fn forget(&mut self) {
        match self {
            Keeping::Memory(held) => *held = None,
            Keeping::Dir(dir) => remove_all(dir),
        }
    }

    /// The carry the previous process left, once: the files are read and then removed. `None`
    /// when nothing was kept.
    pub fn take(&mut self) -> Option<Carry> {
        match self {
            Keeping::Memory(held) => held.take(),
            Keeping::Dir(dir) => {
                let carry = Carry {
                    inbox: load(dir, "inbox"),
                    outbox: load(dir, "outbox"),
                    kept: load(dir, "kept"),
                };
                remove_all(dir);
                let empty =
                    carry.inbox.is_empty() && carry.outbox.is_empty() && carry.kept.is_empty();
                (!empty).then_some(carry)
            }
        }
    }
}

fn write_all(dir: &Path, carry: &Carry) -> Result<(), Failure> {
    for (sub, files) in [
        ("kept", &carry.kept),
        ("inbox", &carry.inbox),
        ("outbox", &carry.outbox),
    ] {
        let folder = dir.join(sub);
        fs::create_dir_all(&folder).map_err(|_| Failure::Gone)?;
        for (name, bytes) in files {
            let name = checked_for_write(name).map_err(|_| Failure::Adapter)?;
            write_replacing(&folder, name, bytes).map_err(|_| Failure::NoRoom)?;
        }
    }
    Ok(())
}

/// Writes `bytes` as `name` in `dir` through a temporary file and a rename, so a file already
/// there is whole until the new one is.
fn write_replacing(dir: &Path, name: &str, bytes: &Secret<Vec<u8>>) -> std::io::Result<()> {
    let tmp = dir.join(format!(".{name}.tmp"));
    let wrote = fs::write(&tmp, bytes.expose()).and_then(|()| fs::rename(&tmp, dir.join(name)));
    if wrote.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    wrote
}

fn load(dir: &Path, sub: &str) -> Files {
    let Ok(entries) = fs::read_dir(dir.join(sub)) else {
        return Vec::new();
    };
    let mut files: Files = Vec::new();
    for entry in entries.flatten() {
        let file = entry.file_name();
        let name = file.to_string_lossy();
        // A temporary file of a keep that was cut off is not kept.
        if name.starts_with('.') && name.ends_with(".tmp") {
            continue;
        }
        if let Ok(bytes) = fs::read(entry.path()) {
            files.push((Text::of(name.as_ref()), Secret::new(bytes)));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Every file of the three folders: zeros over it, then removed.
fn remove_all(dir: &Path) {
    for sub in FOLDERS {
        let Ok(entries) = fs::read_dir(dir.join(sub)) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&path)
                && meta.is_file()
                && let Ok(mut file) = fs::OpenOptions::new().write(true).open(&path)
            {
                use std::io::Write;
                let mut zeros = vec![0u8; usize::try_from(meta.len()).unwrap_or(0)];
                let _ = file.write_all(&zeros).and_then(|()| file.sync_all());
                zeros.zeroize();
            }
            let _ = fs::remove_file(&path);
        }
    }
}
