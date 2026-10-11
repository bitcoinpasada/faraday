//! The file channel (`docs/PLANNING.md` §16.16): every directory a FAT
//! partition is mounted at, read one file at a time and written one file
//! at a time.
//!
//! On the card there is one such directory, `/mnt/microsd`, where init
//! mounted the boot medium's exchange partition. On the stick there is
//! that one and a directory under `/mnt/usb` for every FAT partition on
//! every other USB disk, which init mounts as the disks arrive, so a
//! stick plugged in after the app is up is read the next time the person
//! opens Read a file. The list is built fresh on every request; nothing
//! here polls.
//!
//! The core asks and the shell answers exactly once. The shell knows no
//! formats: a PSBT is bytes and a descriptor is bytes. Which files could
//! be what was asked for is decided here and the list goes to the core,
//! newest first; the core draws it and asks for one of them by name, and
//! a name that is not on the list is not read. When more than one place
//! is mounted a name is `PLACE/FILE`, so two files called `signed.psbt`
//! on two sticks are two names; when one is, it is the file's own name.
//!
//! The exchange partition holds the settings the app keeps
//! (`docs/PLANNING.md` §6) and, on a dev medium, the timings `--timings`
//! records, both under names of this shell's choosing. They are the
//! device's files rather than the user's, so "Read a file" never offers
//! them, and they are never written to a plugged-in stick.
//!
//! Nothing here is `unsafe`, nothing here is a dependency, and no file's
//! contents ever reach a diagnostic.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use osk_shell_api::{FileEntry, FileKind};

/// The most that will be handed to the core from one file. A PSBT is
/// kilobytes; anything of this size on the card is not what was asked
/// for, and the device has a few tens of megabytes of RAM to lose.
const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// The note the image writes onto the partition. It is not a file anyone
/// meant to sign, so it is never offered.
const NOTE: &str = "README.txt";

/// The settings the app keeps (`docs/PLANNING.md` §6). It is the
/// device's own file, not one anyone meant to sign, so it is never
/// offered either.
pub const SETTINGS: &str = "opensigner-settings.txt";

/// Where the settings are written before being renamed into place.
const SETTINGS_TEMP: &str = "opensigner-settings.tmp";

/// What `--timings` writes on a dev card (`docs/PLANNING.md` §16.35).
/// It is the device's own file too, so it is never offered.
pub const TIMINGS: &str = "opensigner-timings.txt";

/// The name a hint that survives sanitising to nothing is written under.
const FALLBACK_NAME: &str = "output";

/// How many `-2`, `-3`, … names are tried before giving up. A card with
/// this many signed PSBTs on it is a card with a problem.
const MAX_ATTEMPTS: u32 = 1000;

/// The most files one listing carries, over every place together. A card
/// with more than this on it is a card nobody scrolls through, and the
/// list is newest first, so what is cut is the oldest.
const MAX_ENTRIES: usize = 64;

/// Which partition `rcS` mounts at the exchange directory, as the image
/// wrote it: a node, or `LABEL=OSKDATA` for a medium whose node name is
/// not fixed. It is the only place the device records what the file
/// channel is called.
const EXCHANGE: &str = "/etc/opensigner/exchange";

/// Where init mounts every other FAT partition it finds on a USB disk,
/// one directory per partition, named by the partition's label or by its
/// node.
const USB: &str = "/mnt/usb";

/// What the core is told when there is nowhere to read or write at all.
/// It reaches only `--verbose`; the screen says there is no file.
const NOTHING_MOUNTED: &str = "no exchange partition is mounted";

/// The first second of 2020, UTC. A board with no real-time clock boots
/// at the epoch, so a file dated before this is a file the device cannot
/// date: the listing says so rather than showing 1970.
const CLOCK_SET: u64 = 1_577_836_800;

/// One directory files are exchanged through, and the name a person
/// finds it by from another computer.
struct Place {
    /// The exchange partition's label, or the mount directory's name for
    /// a plugged-in partition. A device whose exchange line names a node
    /// rather than a label has no such name to give.
    name: Option<String>,
    dir: PathBuf,
}

impl Place {
    /// What goes before a file's name when there is more than one place
    /// to tell apart.
    fn prefix(&self) -> String {
        self.name
            .clone()
            .or_else(|| file_name(&self.dir))
            .unwrap_or_default()
    }
}

/// The file channel: the exchange directory, the directory the mounts of
/// plugged-in partitions appear under, and whether a directory has to be
/// a mount point before it is touched.
pub struct Files {
    dir: PathBuf,
    /// `/mnt/usb` on the device; `None` when `--files` named one plain
    /// directory on a build box, which is then the only place there is.
    usb: Option<PathBuf>,
    /// True on the device, where a directory is only real if a partition
    /// mounted there; false when `--files` named a plain directory on a
    /// build box.
    require_mount: bool,
}

impl Files {
    pub fn new(dir: PathBuf, require_mount: bool) -> Files {
        Files {
            dir,
            usb: require_mount.then(|| PathBuf::from(USB)),
            require_mount,
        }
    }

    /// The same channel over directories a test makes, with no mount to
    /// check: `dir` stands in for the exchange partition and `usb` for
    /// the directory a plugged-in stick's partition is mounted under.
    #[cfg(test)]
    pub fn at(dir: PathBuf, usb: PathBuf) -> Files {
        Files {
            dir,
            usb: Some(usb),
            require_mount: false,
        }
    }

    /// Where this shell looked, as a person would find it from another
    /// computer: the names of every mounted place, in the order the list
    /// carries them, joined with commas.
    pub fn place(&self) -> Option<String> {
        let names: Vec<String> = self.places().into_iter().filter_map(|p| p.name).collect();
        (!names.is_empty()).then(|| names.join(", "))
    }

    /// Every file in every mounted place that could be what was asked
    /// for, newest first, with the size and date the core's Files screen
    /// shows. The reason for an empty answer is for `--verbose`; it never
    /// carries contents.
    pub fn list(&self, kind: FileKind) -> Result<Vec<FileEntry>, String> {
        Ok(self
            .found(kind)?
            .into_iter()
            .map(|(_, entry, _)| entry)
            .collect())
    }

    /// The bytes of one file from [`Files::list`], by the name it gave.
    /// A name that is not on the list — one with a path in it, one the
    /// rule does not allow, one that is gone — is no file at all. The
    /// reason is for `--verbose`; it never carries contents.
    pub fn read(&self, kind: FileKind, name: &str) -> Result<Vec<u8>, String> {
        let found = self
            .found(kind)?
            .into_iter()
            .find(|(_, entry, _)| entry.name == name);
        let Some((_, entry, path)) = found else {
            return Err(format!("{name} is on none of the mounted partitions"));
        };
        if entry.size > MAX_BYTES {
            let size = entry.size;
            return Err(format!(
                "{} is {size} bytes, larger than the {MAX_BYTES} this reads",
                path.display()
            ));
        }
        fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    }

    /// Writes `bytes` under a name derived from `name_hint`, never over a
    /// file already there, and returns the path written. It goes to the
    /// exchange partition when that is mounted and to the first
    /// plugged-in partition otherwise, which is the order [`Files::list`]
    /// reads in.
    pub fn write(&self, name_hint: &str, bytes: &[u8]) -> Result<PathBuf, String> {
        let places = self.places();
        let place = places.first().ok_or(NOTHING_MOUNTED)?;
        let path = free_path(&place.dir, &sanitise(name_hint))?;
        let mut file =
            File::create(&path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
        file.write_all(bytes)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        // vfat is mounted with `flush`, but that only promises the write
        // goes out when the file is closed; sync_all waits for it, so the
        // card is safe to pull the moment the core's screen says saved.
        file.sync_all()
            .map_err(|e| format!("cannot flush {}: {e}", path.display()))?;
        Ok(path)
    }

    /// The settings kept on the exchange partition, if it is mounted and
    /// the file is there. The reason for an empty answer is for
    /// `--verbose`; it never carries contents.
    #[allow(dead_code)] // Faraday keeps no settings file yet.
    pub fn read_settings(&self) -> Result<Vec<u8>, String> {
        self.mounted()?;
        let path = self.dir.join(SETTINGS);
        fs::read(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    }

    /// Keeps `bytes` as the settings, replacing what was there. The
    /// temporary file is written and flushed first and then renamed over
    /// the settings, so a card pulled mid-write keeps the old ones.
    pub fn write_settings(&self, bytes: &[u8]) -> Result<PathBuf, String> {
        self.mounted()?;
        let temp = self.dir.join(SETTINGS_TEMP);
        let path = self.dir.join(SETTINGS);
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
    /// block is flushed as the settings are: a card pulled a moment later
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

    /// Every place there is to exchange files through, the exchange
    /// partition first and the plugged-in partitions after it in the
    /// order of their names, so the same media give the same list on
    /// every request.
    fn places(&self) -> Vec<Place> {
        let mounts = if self.require_mount {
            mount_points()
        } else {
            Vec::new()
        };
        // A place is a directory files can be exchanged through: one a
        // partition is mounted at on the device, and one that is there
        // at all when `--files` named a directory on a build box.
        let mounted = |dir: &Path| {
            if self.require_mount {
                mounts.iter().any(|p| p == dir)
            } else {
                dir.is_dir()
            }
        };
        let mut places = Vec::new();
        if mounted(&self.dir) {
            places.push(Place {
                name: self.exchange_name(),
                dir: self.dir.clone(),
            });
        }
        if let Some(usb) = &self.usb {
            let mut dirs: Vec<PathBuf> = fs::read_dir(usb)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect();
            dirs.sort();
            for dir in dirs {
                if !mounted(&dir) {
                    continue;
                }
                places.push(Place {
                    name: file_name(&dir),
                    dir,
                });
            }
        }
        places
    }

    /// The name of the exchange partition: its label on the device, the
    /// directory's own name when `--files` named one.
    fn exchange_name(&self) -> Option<String> {
        if self.require_mount {
            label_of(&fs::read_to_string(EXCHANGE).unwrap_or_default())
        } else {
            file_name(&self.dir)
        }
    }

    /// Every file that could be what was asked for, with the path it is
    /// at, newest first. A file's name carries the place it is in when
    /// there is more than one place, and is the file's own name when
    /// there is one.
    fn found(&self, kind: FileKind) -> Result<Vec<(SystemTime, FileEntry, PathBuf)>, String> {
        let places = self.places();
        if places.is_empty() {
            return Err(String::from(NOTHING_MOUNTED));
        }
        let many = places.len() > 1;
        let mut found: Vec<(SystemTime, FileEntry, PathBuf)> = Vec::new();
        let mut reasons: Vec<String> = Vec::new();
        for place in &places {
            let entries = match fs::read_dir(&place.dir) {
                Ok(entries) => entries,
                Err(e) => {
                    reasons.push(format!("cannot read {}: {e}", place.dir.display()));
                    continue;
                }
            };
            let prefix = place.prefix();
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
                let listed = if many {
                    format!("{prefix}/{name}")
                } else {
                    String::from(name)
                };
                found.push((
                    time,
                    FileEntry {
                        name: listed,
                        size: meta.len(),
                        modified: seconds(time),
                    },
                    entry.path(),
                ));
            }
        }
        if found.is_empty() && !reasons.is_empty() {
            return Err(reasons.join("; "));
        }
        // Newest first; two files written in the same instant are
        // settled by name, so the list is the same on every request.
        found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
        found.truncate(MAX_ENTRIES);
        Ok(found)
    }

    /// On the device the exchange directory is only the medium's
    /// partition if something is mounted there. A card someone
    /// repartitioned still boots into the app, and the app should say
    /// there is no file rather than write into the initramfs, which no
    /// one can read afterwards.
    fn mounted(&self) -> Result<(), String> {
        if !self.require_mount || mount_points().iter().any(|point| point == &self.dir) {
            Ok(())
        } else {
            Err(String::from("the exchange partition is not mounted"))
        }
    }
}

/// `name`, or `name-2`, `name-3`, … in `dir`: the counter goes before the
/// last extension, so `signed.psbt` becomes `signed-2.psbt` and stays a
/// PSBT to whatever opens it.
fn free_path(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let direct = dir.join(name);
    if !direct.exists() {
        return Ok(direct);
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, Some(ext)),
        _ => (name, None),
    };
    for n in 2..=MAX_ATTEMPTS {
        let candidate = match ext {
            Some(ext) => format!("{stem}-{n}.{ext}"),
            None => format!("{stem}-{n}"),
        };
        let path = dir.join(candidate);
        if !path.exists() {
            return Ok(path);
        }
    }
    Err(format!("{} already holds {name}", dir.display()))
}

/// Every directory something is mounted at, as the kernel lists them.
fn mount_points() -> Vec<PathBuf> {
    fs::read_to_string("/proc/mounts")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .map(PathBuf::from)
        .collect()
}

/// The last component of a path, as a name.
fn file_name(dir: &Path) -> Option<String> {
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

/// A file's modification time in seconds since the epoch, or `None`
/// where the clock says a year before 2020: a board with no real-time
/// clock is at the epoch until something sets it, and a date from then
/// is a date to leave off the screen.
fn seconds(time: SystemTime) -> Option<u64> {
    let secs = time.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs();
    (secs >= CLOCK_SET).then_some(secs)
}

/// Whether a file directly in the directory could be what was asked for.
fn wanted(name: &str, kind: FileKind) -> bool {
    match kind {
        // A computer writes PSBTs under either case, and FAT does not
        // care which one the card kept.
        FileKind::Psbt => {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".psbt") && lower.len() > ".psbt".len()
        }
        // Nothing asks this shell for a text file: the kind names a
        // clipboard payload, which this build has none of.
        // Anything the user put there, and nothing the card came with or
        // an operating system left behind.
        FileKind::Text | FileKind::Any => {
            !name.starts_with('.')
                && name != NOTE
                && name != SETTINGS
                && name != SETTINGS_TEMP
                && name != TIMINGS
        }
        FileKind::Png => {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".png") && lower.len() > ".png".len()
        }
    }
}

/// A name FAT holds: letters, digits, dot, dash and underscore, with
/// everything else replaced rather than dropped, so two hints that differ
/// only in punctuation do not become one name.
fn sanitise(hint: &str) -> String {
    let name: String = hint
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() || name == "." || name == ".." {
        String::from(FALLBACK_NAME)
    } else {
        name
    }
}

/// The label an exchange line names, if it names one.
fn label_of(line: &str) -> Option<String> {
    let label = line.trim().strip_prefix("LABEL=")?.trim();
    (!label.is_empty()).then(|| String::from(label))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The empty Files screen names where the shell looked, so a person
    /// knows which partition to put a `.psbt` on. With a stick plugged in
    /// as well as the boot medium, it names both.
    #[test]
    fn a_listing_names_every_partition_it_looked_on() {
        assert_eq!(label_of("LABEL=OSKDATA\n").as_deref(), Some("OSKDATA"));
        assert_eq!(
            label_of("/dev/mmcblk0p2\n"),
            None,
            "a node is not a name anyone would recognise"
        );
        let root = std::env::temp_dir().join("opensigner-files-places");
        let _ = fs::remove_dir_all(&root);
        let given = root.join("psbt");
        fs::create_dir_all(&given).unwrap();
        assert_eq!(
            Files::new(given, false).place().as_deref(),
            Some("psbt"),
            "--files names the directory it was given"
        );

        let card = root.join("OSKDATA");
        let usb = root.join("usb");
        fs::create_dir_all(&card).unwrap();
        fs::create_dir_all(usb.join("KINGSTON")).unwrap();
        assert_eq!(
            Files::at(card, usb).place().as_deref(),
            Some("OSKDATA, KINGSTON")
        );
    }

    /// Where a signed transaction is saved, which is where a person then
    /// looks for it from another computer: the device's own medium
    /// whenever that is mounted, whichever stick the unsigned one came
    /// from, and the plugged-in stick when the device's medium is not
    /// there at all.
    #[test]
    fn a_save_goes_to_the_devices_own_medium_while_it_is_there() {
        let root = std::env::temp_dir().join("opensigner-files-save");
        let _ = fs::remove_dir_all(&root);
        let card = root.join("OSKDATA");
        let usb = root.join("usb");
        let stick = usb.join("KINGSTON");
        fs::create_dir_all(&card).unwrap();
        fs::create_dir_all(&stick).unwrap();
        fs::write(stick.join("unsigned.psbt"), b"psbt\xff").unwrap();

        let files = Files::at(card.clone(), usb.clone());
        let written = files.write("signed.psbt", b"psbt\xffsigned").unwrap();
        assert_eq!(
            written.parent(),
            Some(card.as_path()),
            "the save goes to the device's own medium, not to the stick the \
             unsigned transaction was read from"
        );

        fs::remove_dir_all(&card).unwrap();
        let alone = Files::at(card, usb);
        let written = alone.write("signed.psbt", b"psbt\xffsigned").unwrap();
        assert_eq!(
            written.parent(),
            Some(stick.as_path()),
            "with the device's own medium gone, the save goes to the one \
             stick that is there"
        );
    }
}
