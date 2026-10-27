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
//! ([`sweep_key_sheets`]). Where files have owners the sheet is written
//! only into a folder made just then, under a name nobody can foresee,
//! that only this user can open ([`key_sheet_in`]); a folder that is not
//! so is refused. The sweep and the wipe never follow a link, and the
//! sweep walks only such folders.

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

/// The folder the key sheets' folders are made in and swept from. On
/// Windows the system's temporary folder (the user's own). Elsewhere the
/// user's runtime folder (`XDG_RUNTIME_DIR`) when it is theirs and no one
/// else can open it, else the system's temporary folder. Reads.
pub fn key_base() -> PathBuf {
    key_base_from(std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from))
}

/// [`key_base`] with the runtime folder given: used where it is a folder
/// of this user's that no one else can open.
pub fn key_base_from(runtime: Option<PathBuf>) -> PathBuf {
    #[cfg(unix)]
    if let Some(p) = runtime.filter(|p| p.is_absolute() && private_dir(p)) {
        return p;
    }
    #[cfg(not(unix))]
    let _ = runtime;
    std::env::temp_dir()
}

/// Whether `path` is itself a folder (not a link) of this user's that no
/// one else can open. Reads metadata, and makes and removes a file in
/// the temporary folder to learn this user's number.
#[cfg(unix)]
fn private_dir(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let Some(me) = my_uid() else {
        return false;
    };
    path.symlink_metadata()
        .is_ok_and(|m| m.is_dir() && m.uid() == me && m.mode() & 0o077 == 0)
}

/// This user's number, found by making a file in the temporary folder
/// under a name nobody can foresee. A file write.
#[cfg(unix)]
fn my_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    let probe = std::env::temp_dir().join(format!(".faraday-uid-{}", random_name()?));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .ok()?;
    let uid = probe.symlink_metadata().ok().map(|m| m.uid());
    let _ = std::fs::remove_file(&probe);
    uid
}

/// Sixteen random bytes as hex, from the system. `None` if it gives none.
/// Reads a file.
#[cfg(unix)]
fn random_name() -> Option<String> {
    use std::io::Read;
    let mut b = [0u8; 16];
    std::fs::File::open("/dev/urandom")
        .ok()?
        .read_exact(&mut b)
        .ok()?;
    Some(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Writes a sheet that carries the recovery key (F49) as `name` in a new
/// `faraday-key-…` folder in [`key_base`], which [`remove_key_sheets`]
/// removes. Returns its path. A file write.
pub fn write_key_sheet(name: &str, pdf: &[u8]) -> Result<PathBuf, Reason> {
    key_sheet_in(&key_base(), name, pdf)
}

/// Writes a key sheet as `name` into a folder `base` holds. Where files
/// have owners the folder is new, named at random, made with `create_dir`
/// (which fails if the name is taken) for this user alone, and checked
/// afterwards to be a folder (not a link) of this user's that no one else
/// can open; if anything is off nothing is written
/// ([`Reason::KeyFolder`]). Elsewhere as [`write_in`]. A file write.
pub fn key_sheet_in(base: &Path, name: &str, pdf: &[u8]) -> Result<PathBuf, Reason> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        let name = Path::new(name)
            .file_name()
            .ok_or_else(|| file_reason(base, "no file name"))?;
        let random = random_name().ok_or(Reason::KeyFolder)?;
        let dir = base.join(format!("{KEY_PREFIX}{random}"));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(|e| file_reason(&dir, e))?;
        if !private_dir(&dir) {
            let _ = std::fs::remove_dir(&dir);
            return Err(Reason::KeyFolder);
        }
        let path = dir.join(name);
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| file_reason(&path, e))?;
        f.write_all(pdf).map_err(|e| file_reason(&path, e))?;
        Ok(path)
    }
    #[cfg(not(unix))]
    write_in(base.to_path_buf(), KEY_PREFIX, name, pdf)
}

/// Overwrites every regular file in `dir` with zeros, then removes the
/// folder if nothing else is left in it. A link, a folder or any other
/// entry is left as it is, and a file is checked, once open, to be the one listed.
/// Best effort: a file the viewer still holds open on Windows may stay
/// until the next start's sweep. Reads and writes files.
fn wipe_dir(dir: &Path) {
    if !dir.symlink_metadata().is_ok_and(|m| m.is_dir()) {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            let Ok(meta) = p.symlink_metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            if let Ok(mut f) = open_same(&p, &meta) {
                use std::io::Write;
                let zeros = vec![0u8; meta.len() as usize];
                let _ = f.write_all(&zeros);
                let _ = f.sync_all();
            }
            let _ = std::fs::remove_file(&p);
        }
    }
    let _ = std::fs::remove_dir(dir);
}

/// Opens the regular file `path`, whose link-free metadata is `meta`,
/// for writing, and refuses it when what was opened is not that file (a
/// link put there in between). Nothing is truncated. Opens a file.
fn open_same(path: &Path, meta: &std::fs::Metadata) -> std::io::Result<std::fs::File> {
    let f = std::fs::OpenOptions::new().write(true).open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let now = f.metadata()?;
        if now.dev() != meta.dev() || now.ino() != meta.ino() || !now.is_file() {
            return Err(std::io::Error::other("not the file that was listed"));
        }
    }
    #[cfg(not(unix))]
    let _ = meta;
    Ok(f)
}

/// Removes the key sheets' folders this run made, each file overwritten
/// first. Called when the Starter exits. Writes and removes files.
pub fn remove_key_sheets(dirs: &[PathBuf]) {
    for d in dirs {
        wipe_dir(d);
    }
}

/// Removes every `faraday-key-…` folder in `base` ([`key_base`]), each
/// regular file in it overwritten first: what an earlier run could not
/// remove. A link is never followed, and where files have owners only a
/// folder of this user's that no one else can open is walked. Called at
/// start. Reads, writes and removes files.
pub fn sweep_key_sheets(base: &Path) {
    let Ok(entries) = std::fs::read_dir(base) else {
        return;
    };
    for e in entries.flatten() {
        let named = e
            .file_name()
            .to_str()
            .is_some_and(|n| n.starts_with(KEY_PREFIX));
        let p = e.path();
        let is_dir = p.symlink_metadata().is_ok_and(|m| m.is_dir());
        #[cfg(unix)]
        let ours = private_dir(&p);
        #[cfg(not(unix))]
        let ours = true;
        if named && is_dir && ours {
            wipe_dir(&p);
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
