//! The few files this shell writes itself where there is no disk process:
//! the dev medium's timings (`--timings`) and boot report
//! (`--boot-report`), each under a name of this shell's choosing. Nothing
//! is mounted on the device, so on it every write here is refused; the
//! one directory this writes to is the one `--files DIR` names on a build
//! box. The sticks' files, and everything a person reads or saves, go
//! through the disk process (`faraday-storage`).
//!
//! Nothing here is `unsafe`, nothing here is a dependency, and no file's
//! contents ever reach a diagnostic.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// What `--timings` writes on a dev card.
pub const TIMINGS: &str = "faraday-timings.txt";

/// What `--boot-report` writes on a dev card, whole, every 30 seconds.
pub const BOOT_REPORT: &str = "faraday-boot-report.txt";

/// The directory the dev files are written to, and whether it is the
/// device's own medium.
pub struct Files {
    dir: PathBuf,
    /// True on the device, where nothing is mounted and so nothing is
    /// written; false when `--files` named a plain directory on a build
    /// box.
    on_device: bool,
}

impl Files {
    pub fn new(dir: PathBuf, on_device: bool) -> Files {
        Files { dir, on_device }
    }

    /// Keeps `bytes` as the file `name` on the exchange partition, whole,
    /// over the one before it: the dev
    /// boot report, where the image runs no disk process.
    pub fn write_named(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        self.write_whole(name, &format!("{name}.tmp"), bytes)
    }

    fn write_whole(&self, name: &str, temp: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        self.mounted()?;
        let temp = self.dir.join(temp);
        let path = self.dir.join(name);
        let mut file =
            File::create(&temp).map_err(|e| format!("cannot create {}: {e}", temp.display()))?;
        file.write_all(bytes)
            .map_err(|e| format!("cannot write {}: {e}", temp.display()))?;
        file.sync_all()
            .map_err(|e| format!("cannot flush {}: {e}", temp.display()))?;
        fs::rename(&temp, &path).map_err(|e| format!("cannot rename {}: {e}", path.display()))?;
        Ok(path)
    }

    /// Adds one block to the timings file, keeping what earlier runs
    /// left. The exchange partition is the device's own medium, so the
    /// block is flushed: a card pulled a moment later
    /// still holds it.
    pub fn append_timings(&self, text: &str) -> Result<PathBuf, String> {
        self.mounted()?;
        let path = self.dir.join(TIMINGS);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
        file.write_all(text.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        file.sync_all()
            .map_err(|e| format!("cannot flush {}: {e}", path.display()))?;
        Ok(path)
    }

    /// On the device the exchange directory would be the initramfs, which
    /// no one can read afterwards, so a write there is refused; only
    /// the directory `--files` named is written to.
    fn mounted(&self) -> Result<(), String> {
        if self.on_device {
            Err(String::from("the exchange partition is not mounted"))
        } else {
            Ok(())
        }
    }
}
