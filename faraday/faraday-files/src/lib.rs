//! A stick's files as Faraday reads and writes them (`docs/history/PLAN.md` §4.3,
//! `docs/VAULT.md` §6), the same rules whatever holds them:
//!
//! - a [`Dir`], a folder: the desktop app's sticks;
//! - a [`Fat`], a FAT16 or FAT32 volume read by `faraday-fat`: the
//!   disk process's partitions on the device.
//!
//! Only the top level of a stick is used. A write is a new file, read
//! back and compared, then renamed into place; it goes over a file
//! already there only when the person chose Replace for it
//! ([`write_replacing`]), a sealed vault going back over its own file,
//! found by its SHA-256 among the copies the app has seen, and the settings file over the settings
//! file. A vault write-back a pulled stick interrupted is finished on the
//! next visit. Pictures are decoded for QR codes here, and only here: the
//! app gets the codes' contents and the picture's bytes as they are, to
//! keep and write out again, and never decodes an image (`docs/QR.md` §4).
//!
//! [`proto`] is how the app's shell and the disk process talk.

pub mod proto;

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use faraday_codec::vault_file;
use faraday_crypto::work::{Compute, Work};
use faraday_strings::{EN, fill};

/// Why the computer refused a folder's file, in the strings file's
/// words: the operating system's own message is not shown.
fn io_text(e: std::io::Error) -> String {
    use std::io::ErrorKind as K;
    match e.kind() {
        K::NotFound => EN.fat_not_found,
        K::AlreadyExists => EN.fat_exists,
        K::StorageFull => EN.fat_full,
        K::PermissionDenied => EN.files_denied,
        K::ReadOnlyFilesystem => EN.files_read_only,
        _ => EN.fat_io,
    }
    .to_string()
}

/// The largest file read from a stick other than a vault, which has its own
/// limit ([`MAX_VAULT_READ`]). A PSBT or a wallet is kilobytes.
pub const MAX_READ: u64 = 18 * 1024 * 1024;

/// The largest file read from a stick that begins as KDBX 4: the longest
/// file a vault has (`docs/VAULT.md` §2). It travels between the app and
/// the disk process in pieces of [`proto::PIECE`], since a frame keeps its
/// limit (F206 point 7).
pub const MAX_VAULT_READ: u64 = vault_file::MAX_FILE_LEN;

/// The largest picture read for QR codes, in pixels: 4096 × 4096, about
/// 16.8 million. That takes in a phone's screenshot (1440 × 3200 is 4.6
/// million), a 4K or 5K screen's (8.3 and 14.7 million) and a 12-megapixel
/// photograph, with room. Decoding holds about 6 bytes a pixel at once
/// (the pixels as RGBA, their grey copy, and the QR reader's own
/// thresholded copy), so a picture at the limit takes about 100 MB of
/// the disk process, which a 1 GB Pi has to spare. Its header is checked against
/// this before any pixel is decoded.
pub const MAX_PIXELS: usize = 4096 * 4096;

/// A stick's top level.
pub trait Place {
    /// Every file, hidden ones included, with its size.
    fn files(&mut self) -> Result<Vec<(String, u64)>, String>;
    /// A file's bytes; refused over `max`.
    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, String>;
    /// A file's length and its `len` bytes from `from` (fewer where it
    /// ends first); refused where the file is over `max`. Where a place
    /// cannot read a part, it reads the whole.
    fn read_part(
        &mut self,
        name: &str,
        from: u64,
        len: usize,
        max: u64,
    ) -> Result<(u64, Vec<u8>), String> {
        let b = self.read(name, max)?;
        let total = b.len() as u64;
        let start = usize::try_from(from).unwrap_or(usize::MAX).min(b.len());
        let end = start.saturating_add(len).min(b.len());
        Ok((total, b[start..end].to_vec()))
    }
    /// A file's first `n` bytes (fewer where it is shorter); refused
    /// over `max` in all. Where a place cannot read a part, it reads the
    /// whole.
    fn head(&mut self, name: &str, n: usize, max: u64) -> Result<Vec<u8>, String> {
        let mut b = self.read(name, max)?;
        b.truncate(n);
        Ok(b)
    }
    /// A new file, on the medium when this returns. Refused when the name
    /// is taken.
    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String>;
    /// Removes a file.
    fn remove(&mut self, name: &str) -> Result<(), String>;
    /// Overwrites a file with zeros, reads them back, and removes it.
    fn wipe(&mut self, name: &str) -> Result<(), String>;
    /// Renames a file; refused when the new name is taken.
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String>;
    /// The data space no file holds, and all of it, in bytes.
    fn space(&mut self) -> Result<(u64, u64), String>;
    /// One step of zeros over the free space from cluster `from`.
    fn zero_free(&mut self, from: u32) -> Result<Zeroed, String>;
    /// Every file gone and the volume empty, its layout and label kept.
    fn make_empty(&mut self) -> Result<(), String>;
}

pub use faraday_fat::Zeroed;

/// About how much one step of zeroing writes: a step is a pause of a
/// second or so on a slow stick, between which the app draws.
pub(crate) const ZERO_STEP: u64 = 8 << 20;

/// A folder standing for a stick.
pub struct Dir(pub PathBuf);

impl Place for Dir {
    fn files(&mut self) -> Result<Vec<(String, u64)>, String> {
        Ok(fs::read_dir(&self.0)
            .map_err(io_text)?
            .filter_map(Result::ok)
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                meta.is_file()
                    .then(|| (e.file_name().to_string_lossy().into_owned(), meta.len()))
            })
            .collect())
    }

    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, String> {
        let path = self.0.join(name);
        let meta = fs::metadata(&path).map_err(io_text)?;
        if meta.len() > max {
            return Err(fill(
                EN.files_larger_than,
                &[&(max / 1024 / 1024).to_string()],
            ));
        }
        fs::read(&path).map_err(io_text)
    }

    fn head(&mut self, name: &str, n: usize, max: u64) -> Result<Vec<u8>, String> {
        use std::io::Read;
        let path = self.0.join(name);
        let meta = fs::metadata(&path).map_err(io_text)?;
        if meta.len() > max {
            return Err(fill(
                EN.files_larger_than,
                &[&(max / 1024 / 1024).to_string()],
            ));
        }
        let mut out = Vec::new();
        fs::File::open(&path)
            .map_err(io_text)?
            .take(n as u64)
            .read_to_end(&mut out)
            .map_err(io_text)?;
        Ok(out)
    }

    fn read_part(
        &mut self,
        name: &str,
        from: u64,
        len: usize,
        max: u64,
    ) -> Result<(u64, Vec<u8>), String> {
        use std::io::{Read, Seek, SeekFrom};
        let path = self.0.join(name);
        let meta = fs::metadata(&path).map_err(io_text)?;
        if meta.len() > max {
            return Err(fill(
                EN.files_larger_than,
                &[&(max / 1024 / 1024).to_string()],
            ));
        }
        let mut file = fs::File::open(&path).map_err(io_text)?;
        file.seek(SeekFrom::Start(from.min(meta.len())))
            .map_err(io_text)?;
        let mut out = Vec::new();
        out.try_reserve_exact(len.min(meta.len() as usize))
            .map_err(|_| EN.fat_io.to_string())?;
        file.take(len as u64)
            .read_to_end(&mut out)
            .map_err(io_text)?;
        Ok((meta.len(), out))
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let mut f = fs::File::create_new(self.0.join(name)).map_err(io_text)?;
        f.write_all(bytes).map_err(io_text)?;
        f.sync_all().map_err(io_text)
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        fs::remove_file(self.0.join(name)).map_err(io_text)
    }

    fn wipe(&mut self, name: &str) -> Result<(), String> {
        use std::io::{Read, Seek, SeekFrom};
        let path = self.0.join(name);
        let mut f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(io_text)?;
        let len = f.metadata().map_err(io_text)?.len();
        let zeros = [0u8; 64 * 1024];
        let mut left = len;
        while left > 0 {
            let n = left.min(zeros.len() as u64) as usize;
            f.write_all(&zeros[..n]).map_err(io_text)?;
            left -= n as u64;
        }
        f.sync_all().map_err(io_text)?;
        f.seek(SeekFrom::Start(0)).map_err(io_text)?;
        let mut back = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut back).map_err(io_text)?;
            if n == 0 {
                break;
            }
            if back[..n].iter().any(|&b| b != 0) {
                return Err(EN.fat_fault_unwritten.to_string());
            }
        }
        drop(f);
        fs::remove_file(path).map_err(io_text)
    }

    // A folder standing for a stick has no free space of its own to
    // write over: the computer's disk holds it.
    fn space(&mut self) -> Result<(u64, u64), String> {
        Ok((0, 0))
    }

    fn zero_free(&mut self, _from: u32) -> Result<Zeroed, String> {
        Ok(Zeroed {
            next: 2,
            total: 0,
            done: true,
        })
    }

    fn make_empty(&mut self) -> Result<(), String> {
        for (name, _) in self.files()? {
            self.wipe(&name)?;
        }
        Ok(())
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        if self.0.join(to).exists() {
            return Err(fill(EN.files_already_there, &[to]));
        }
        fs::rename(self.0.join(from), self.0.join(to)).map_err(io_text)
    }
}

/// Why `faraday-fat` refused, in a few words from the strings file: its
/// errors are kinds, worded here for every caller that shows one (the
/// app's Files, the disk process, the Starter's boot file).
pub fn fat_text(e: faraday_fat::Error) -> String {
    use faraday_fat::{BadName, Boot, Error, Fault};
    let s = match e {
        Error::Io => EN.fat_io,
        Error::Fat12 => EN.fat_12,
        Error::NotFound => EN.fat_not_found,
        Error::Exists => EN.fat_exists,
        Error::Full => EN.fat_full,
        Error::DirFull => EN.fat_dir_full,
        Error::TooLarge => EN.fat_too_large,
        Error::NotFat(b) => match b {
            Boot::Signature => EN.fat_boot_signature,
            Boot::SectorSize => EN.fat_boot_sector_size,
            Boot::ClusterSize => EN.fat_boot_cluster_size,
            Boot::NoFat => EN.fat_boot_no_fat,
            Boot::EmptyFat => EN.fat_boot_empty_fat,
            Boot::FatsLarger => EN.fat_boot_fats_larger,
            Boot::TooManyClusters => EN.fat_boot_too_many_clusters,
            Boot::Fat32Root => EN.fat_boot_fat32_root,
            Boot::Fat16Clusters => EN.fat_boot_fat16_clusters,
            Boot::Fat16NoRoot => EN.fat_boot_fat16_no_root,
            Boot::VolumeLarger => EN.fat_boot_volume_larger,
            Boot::FatSmall => EN.fat_boot_fat_small,
            Boot::RootOutside => EN.fat_boot_root_outside,
        },
        Error::Corrupt(f) => match f {
            Fault::ClusterNumber => EN.fat_fault_cluster_number,
            Fault::ClusterOutside => EN.fat_fault_cluster_outside,
            Fault::Loop => EN.fat_fault_loop,
            Fault::TopFolderLong => EN.fat_fault_top_folder_long,
            Fault::FolderLong => EN.fat_fault_folder_long,
            Fault::FileLonger => EN.fat_fault_file_longer,
            Fault::EmptyRoot => EN.fat_fault_empty_root,
            Fault::Unwritten => EN.fat_fault_unwritten,
        },
        Error::Name(n) => match n {
            BadName::Empty => EN.fat_name_empty,
            BadName::Long => EN.fat_name_long,
            BadName::Ending => EN.fat_name_ending,
            BadName::Character => EN.fat_name_character,
        },
    };
    s.to_string()
}

/// A FAT16 or FAT32 volume.
pub struct Fat<D: faraday_fat::Disk>(pub faraday_fat::Volume<D>);

impl<D: faraday_fat::Disk> Place for Fat<D> {
    fn files(&mut self) -> Result<Vec<(String, u64)>, String> {
        Ok(self
            .0
            .list()
            .map_err(fat_text)?
            .into_iter()
            .filter(|e| !e.dir)
            .map(|e| (e.name, u64::from(e.size)))
            .collect())
    }

    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, String> {
        let max = u32::try_from(max).unwrap_or(u32::MAX);
        self.0.read(name, max).map_err(fat_text)
    }

    fn head(&mut self, name: &str, n: usize, max: u64) -> Result<Vec<u8>, String> {
        let max = u32::try_from(max).unwrap_or(u32::MAX);
        self.0.head(name, n, max).map_err(fat_text)
    }

    fn read_part(
        &mut self,
        name: &str,
        from: u64,
        len: usize,
        max: u64,
    ) -> Result<(u64, Vec<u8>), String> {
        let max = u32::try_from(max).unwrap_or(u32::MAX);
        self.0
            .read_part(name, from, len, max)
            .map(|(total, b)| (u64::from(total), b))
            .map_err(fat_text)
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.0
            .create(name, bytes)
            .and_then(|()| self.0.flush())
            .map_err(fat_text)
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        self.0
            .delete(name)
            .and_then(|()| self.0.flush())
            .map_err(fat_text)
    }

    fn wipe(&mut self, name: &str) -> Result<(), String> {
        self.0.wipe(name).map_err(fat_text)
    }

    fn space(&mut self) -> Result<(u64, u64), String> {
        self.0.space().map_err(fat_text)
    }

    fn zero_free(&mut self, from: u32) -> Result<Zeroed, String> {
        self.0.zero_free(from, ZERO_STEP).map_err(fat_text)
    }

    fn make_empty(&mut self) -> Result<(), String> {
        self.0.make_empty().map_err(fat_text)
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.0
            .rename(from, to)
            .and_then(|()| self.0.flush())
            .map_err(fat_text)
    }
}

/// Deletes a file the person chose from a stick, zeros over it first
/// ([`Place::wipe`]). Only a name the app could have written: no hidden
/// file, nothing outside the top folder.
pub fn wipe(place: &mut dyn Place, name: &str) -> Result<(), String> {
    place.wipe(checked(name)?)
}

/// The names Windows gives its own files and folders at the top of a
/// FAT volume it has a drive letter for, compared ignoring case
/// (`docs/STARTER.md` §4.4, D11): the indexer's and System Restore's
/// folder (`IndexerVolumeGuid`, `WPSettings.dat`), the Recycle Bin's
/// folder (and XP's), Explorer's folder settings and its thumbnail
/// cache.
const WINDOWS_OWN: [&str; 5] = [
    "System Volume Information", // not wording: the name Windows gives its folder
    "$RECYCLE.BIN",              // not wording: the name Windows gives its folder
    "RECYCLER",
    "desktop.ini",
    "Thumbs.db",
];

/// Whether a name at a stick's top level is an operating system's own
/// file rather than one a person put there (`docs/STARTER.md` §4.4,
/// D11; owner, 2026-10-19). Such a file is never listed, never read and
/// never counted, and a write leaves it where it is. Decided by name
/// alone, never by the hidden or system attribute: every name beginning
/// with a dot (macOS's `.Spotlight-V100`, `.fseventsd`, `.Trashes`,
/// `.DS_Store` and `._` AppleDouble files, a Linux desktop's
/// `.Trash-1000`, Faraday's own part files) and Windows' own names.
pub fn foreign(name: &str) -> bool {
    name.starts_with('.') || WINDOWS_OWN.iter().any(|w| w.eq_ignore_ascii_case(name))
}

/// Whether Windows takes `name` for a device (`CON`, `NUL`, `COM1`,
/// `LPT3`...), whatever follows its first dot.
pub(crate) fn device_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or("").trim_end();
    let numbered = |prefix: &str| {
        stem.len() == 4
            && stem.as_bytes()[..3].eq_ignore_ascii_case(prefix.as_bytes())
            && matches!(stem.as_bytes()[3], b'1'..=b'9')
    };
    ["CON", "PRN", "AUX", "NUL"]
        .iter()
        .any(|d| d.eq_ignore_ascii_case(stem))
        || numbered("COM")
        || numbered("LPT")
}

/// A name the app may read or remove: one path component, no hidden
/// files, and none of an operating system's own ([`foreign`]).
pub fn checked(name: &str) -> Result<&str, String> {
    if name.is_empty()
        || foreign(name)
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(fill(EN.files_not_a_name, &[name]));
    }
    Ok(name)
}

/// A name the app may write: [`checked`], and one Windows can open: no
/// `: < > " | ? *` or control character, no trailing space or dot, and
/// not a device's name ([`device_name`]). A file already on a stick under
/// a name that fails this is still read and removed under [`checked`].
pub fn checked_for_write(name: &str) -> Result<&str, String> {
    checked(name)?;
    if name
        .chars()
        .any(|c| c.is_control() || ":<>\"|?*".contains(c))
        || name.ends_with(' ')
        || name.ends_with('.')
        || device_name(name)
    {
        return Err(fill(EN.files_not_a_name, &[name]));
    }
    Ok(name)
}

/// Costs a read of the first twelve bytes of each file that could be a
/// vault: the names in `files` that are a vault by its contents
/// (`docs/VAULT-HARDENING.md` §2.1), whatever the name, a `.kdbx` too.
/// A file is read only where its length is a length a vault can have. Made once when the listing is, and kept with it.
pub fn vault_names(c: &Compute, place: &mut dyn Place, files: &[(String, u64)]) -> Vec<String> {
    let mut out = Vec::new();
    for (n, len) in files {
        let named = n.to_ascii_lowercase().ends_with(".kdbx");
        if (n.starts_with('.') && !named) || foreign(n) || !vault_file::is_vault_len(*len) {
            continue;
        }
        c.count(Work::FileRead);
        if place
            .head(n, vault_file::HEAD_LEN, MAX_VAULT_READ)
            .is_ok_and(|b| vault_file::kdbx_major(&b) == Some(4))
        {
            out.push(n.clone());
        }
    }
    out.sort();
    out
}

/// The files a person sees: none of an operating system's own
/// ([`foreign`]), sorted by name.
pub fn listing(place: &mut dyn Place) -> Vec<(String, u64)> {
    let mut files: Vec<(String, u64)> = place
        .files()
        .unwrap_or_default()
        .into_iter()
        .filter(|(n, _)| !foreign(n))
        .collect();
    files.sort();
    files
}

fn has(place: &mut dyn Place, name: &str) -> Result<bool, String> {
    Ok(place
        .files()?
        .iter()
        .any(|(n, _)| n.eq_ignore_ascii_case(name)))
}

/// Reads one file the app asked for.
pub fn read(place: &mut dyn Place, name: &str) -> Result<Vec<u8>, String> {
    if name.to_ascii_lowercase().ends_with(".kdbx") {
        finish_vault_writes(place);
    }
    place.read(checked(name)?, MAX_READ)
}

/// Reads one file whole, whatever a frame holds: a file that begins as
/// KDBX 4 up to [`MAX_VAULT_READ`], any other to [`MAX_READ`]. For a place
/// that does not send the file through a frame.
pub fn read_whole(place: &mut dyn Place, name: &str) -> Result<Vec<u8>, String> {
    if name.to_ascii_lowercase().ends_with(".kdbx") {
        finish_vault_writes(place);
    }
    let name = checked(name)?;
    let max = read_limit(place, name);
    place.read(name, max)
}

/// The most that may be read of `name`: [`MAX_VAULT_READ`] where it
/// begins as KDBX 4, else [`MAX_READ`].
fn read_limit(place: &mut dyn Place, name: &str) -> u64 {
    if has_vault_head(place, name) {
        MAX_VAULT_READ
    } else {
        MAX_READ
    }
}

/// Reads up to `len` bytes of `name` from `from`, and the file's length,
/// for a file sent in pieces: refused over [`MAX_READ`] unless it begins
/// as KDBX 4, then over [`MAX_VAULT_READ`]; and a piece over
/// [`proto::PIECE`] is cut to it.
pub fn read_part(
    place: &mut dyn Place,
    name: &str,
    from: u64,
    len: usize,
) -> Result<(u64, Vec<u8>), String> {
    if name.to_ascii_lowercase().ends_with(".kdbx") && from == 0 {
        finish_vault_writes(place);
    }
    let name = checked(name)?;
    let max = read_limit(place, name);
    place.read_part(name, from, len.min(proto::PIECE), max)
}

/// The first of `name`, `stem-2.ext`, `stem-3.ext`… not on the stick.
fn free_name(place: &mut dyn Place, name: &str) -> Result<String, String> {
    let taken: Vec<String> = place
        .files()?
        .into_iter()
        .map(|(n, _)| n.to_ascii_lowercase())
        .collect();
    if !taken.contains(&name.to_ascii_lowercase()) {
        return Ok(name.to_string());
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    (2..100)
        .map(|n| format!("{stem}-{n}{ext}"))
        .find(|c| !taken.contains(&c.to_ascii_lowercase()))
        .ok_or_else(|| EN.files_no_free_name.to_string())
}

/// Writes `bytes` under the hidden name `temp`, reads them back and
/// compares. The temp file is gone again on any failure.
fn put_checked(place: &mut dyn Place, temp: &str, bytes: &[u8]) -> Result<(), String> {
    if has(place, temp)? {
        place.remove(temp)?;
    }
    place.create(temp, bytes)?;
    let back = place.read(temp, bytes.len() as u64);
    if back.as_deref() != Ok(bytes) {
        let _ = place.remove(temp);
        return Err(EN.files_read_back.to_string());
    }
    Ok(())
}

/// Writes a file as a new file, reads it back, compares, and renames it
/// into place. Returns the name it was written under.
pub fn write(place: &mut dyn Place, name: &str, bytes: &[u8]) -> Result<String, String> {
    let name = free_name(place, checked_for_write(name)?)?;
    let temp = format!(".faraday-{name}.part");
    put_checked(place, &temp, bytes)?;
    if has(place, &name)? {
        let _ = place.remove(&temp);
        return Err(fill(EN.files_appeared, &[&name]));
    }
    if let Err(e) = place.rename(&temp, &name) {
        let _ = place.remove(&temp);
        return Err(e);
    }
    Ok(name)
}

/// Writes a file over the one of the same name on the stick, ignoring
/// case, when there is one: the new bytes under a hidden temp name, read
/// back and compared, then the old file removed and the temp renamed to
/// its name, as [`write_vault`] does. A read-back that differs leaves the
/// old file as it was. With no file of that name, the same as [`write`].
/// Returns the name it was written under.
pub fn write_replacing(place: &mut dyn Place, name: &str, bytes: &[u8]) -> Result<String, String> {
    let name = checked_for_write(name)?;
    let Some(old) = place
        .files()?
        .into_iter()
        .map(|(n, _)| n)
        .find(|n| n.eq_ignore_ascii_case(name))
    else {
        return write(place, name, bytes);
    };
    // A vault on the stick is never removed by a replacing write: only
    // `write_vault` goes over a vault, and only over a copy of it.
    if is_vault_on(place, &old) {
        return write(place, name, bytes);
    }
    let temp = format!(".faraday-{old}.part");
    put_checked(place, &temp, bytes)?;
    // FAT has no atomic replace: the old file goes, then the copy that
    // read back whole takes its name.
    if let Err(e) = place.remove(&old) {
        let _ = place.remove(&temp);
        return Err(e);
    }
    place.rename(&temp, &old)?;
    Ok(old)
}

/// Whether the file `name` on the place is held as a vault: named
/// `.kdbx`, whatever it holds (erring safe), or one of the vault lengths
/// and beginning with KDBX 4's signature (§2.1). Reads twelve bytes of a file that
/// is not named so.
fn is_vault_on(place: &mut dyn Place, name: &str) -> bool {
    if name.to_ascii_lowercase().ends_with(".kdbx") {
        return true;
    }
    let sized = place.files().is_ok_and(|f| {
        f.iter()
            .any(|(n, l)| n == name && vault_file::is_vault_len(*l))
    });
    sized
        && place
            .head(name, vault_file::HEAD_LEN, MAX_VAULT_READ)
            .is_ok_and(|b| vault_file::kdbx_major(&b) == Some(4))
}

/// Whether a file begins with KDBX's signatures and major version 4:
/// reads its first twelve bytes only.
fn has_vault_head(place: &mut dyn Place, name: &str) -> bool {
    place
        .head(name, vault_file::HEAD_LEN, MAX_VAULT_READ)
        .is_ok_and(|b| vault_file::kdbx_major(&b) == Some(4))
}

/// Costs a read and a SHA-256 pass over each file that begins as a vault
/// does on the stick. The names of the files on the stick whose SHA-256 is among
/// `copies`, the SHA-256 of whole files (`docs/VAULT.md` §6).
fn copies_on(
    c: &Compute,
    place: &mut dyn Place,
    copies: &[[u8; 32]],
) -> Result<Vec<String>, String> {
    let mut found: Vec<String> = Vec::new();
    if copies.is_empty() {
        return Ok(found);
    }
    for (n, len) in place.files()? {
        if n.starts_with('.') || !vault_file::is_vault_len(len) || len > MAX_VAULT_READ {
            continue;
        }
        // Only a file that begins as a vault does is read whole.
        if !has_vault_head(place, &n) {
            continue;
        }
        if let Ok(b) = place.read(&n, len)
            && copies.contains(&faraday_crypto::sha256_whole(c, &b))
        {
            found.push(n);
        }
    }
    Ok(found)
}

/// Costs what [`write_vault_over`] costs. Writes a sealed vault back
/// (`docs/VAULT.md` §6): over the file on the stick whose SHA-256 is
/// one of `copies`, or as a new file when there is none, beside a file
/// of its name ([`OnTaken::Beside`]). The new bytes
/// are written beside the original, read back and compared before the
/// original is replaced. Two copies on one stick are refused, naming both.
pub fn write_vault(
    c: &Compute,
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    copies: &[[u8; 32]],
) -> Result<String, String> {
    write_vault_over(c, place, name, bytes, copies, true, false, OnTaken::Beside)
        .map_err(WriteRefused::text)
}

/// What a vault's write does when a file has its name and nothing proves
/// that file is a copy of the vault (`docs/VAULT.md` §6 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnTaken {
    /// Nothing is written: the write is refused as [`WriteRefused::NameTaken`].
    Ask,
    /// The vault goes under a free name beside that file.
    Beside,
    /// The vault goes over that file, which must still have this SHA-256.
    Over([u8; 32]),
}

/// Why a write was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteRefused {
    /// A file has the vault's name, ignoring case, and nothing proves it
    /// is a copy of the vault; nothing on the stick changed.
    NameTaken {
        /// That file's name on the stick.
        on_stick: String,
        /// Its SHA-256; none when it is over [`MAX_READ`] or cannot be read.
        sha: Option<[u8; 32]>,
    },
    /// Anything else, as the words that say why.
    Failed(String),
}

impl WriteRefused {
    /// The words for why, where nothing asks.
    pub fn text(self) -> String {
        match self {
            WriteRefused::NameTaken { on_stick, .. } => fill(EN.files_appeared, &[&on_stick]),
            WriteRefused::Failed(why) => why,
        }
    }
}

impl From<String> for WriteRefused {
    fn from(why: String) -> Self {
        WriteRefused::Failed(why)
    }
}

/// Costs a read and a SHA-256 pass over each file that begins as a vault
/// does on the stick, and over a file of the vault's name. [`write_vault`],
/// refusing to go over a copy when `over` is
/// false. Over a copy the file keeps the name it has on the stick,
/// unless `renamed`: the person renamed the vault's file For the stick,
/// and the file takes that name (`docs/FLOWS.md` decision 61 item 2), or
/// `name-2` where another file has it. With no copy on the stick, a file
/// of the vault's name, ignoring case, is dealt with as `on_taken` says.
#[expect(
    clippy::too_many_arguments,
    reason = "one write, as the protocol carries it"
)]
pub fn write_vault_over(
    c: &Compute,
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    copies: &[[u8; 32]],
    over: bool,
    renamed: bool,
    on_taken: OnTaken,
) -> Result<String, WriteRefused> {
    finish_vault_writes(place);
    if !whole_vault(bytes) {
        return Err(EN.files_not_a_vault.to_string().into());
    }
    let mut same = copies_on(c, place, copies)?;
    let original = match same.len() {
        0 => return write_vault_new(c, place, name, bytes, renamed, on_taken),
        1 if !over => {
            return Err(fill(EN.files_vault_not_ours, &[&same[0]]).into());
        }
        1 => same.remove(0),
        _ => {
            return Err(fill(EN.files_vault_twice, &[&same[0], &same[1]]).into());
        }
    };
    Ok(replace_file(place, &original, name, bytes, renamed)?)
}

/// Costs a read and a SHA-256 pass over the file of the vault's name.
/// A vault no copy of which is on the stick: a new file, or what
/// `on_taken` says where a file has its name.
fn write_vault_new(
    c: &Compute,
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    renamed: bool,
    on_taken: OnTaken,
) -> Result<String, WriteRefused> {
    let asked = checked_for_write(name)?;
    let taken = place
        .files()?
        .into_iter()
        .find(|(n, _)| !n.starts_with('.') && n.eq_ignore_ascii_case(asked));
    let Some((old, len)) = taken else {
        return Ok(write(place, name, bytes)?);
    };
    let sha_of = |place: &mut dyn Place| {
        let max = if len <= MAX_READ || has_vault_head(place, &old) {
            MAX_VAULT_READ
        } else {
            MAX_READ
        };
        (len <= max)
            .then(|| place.read(&old, max).ok())
            .flatten()
            .map(|b| faraday_crypto::sha256_whole(c, &b))
    };
    match on_taken {
        OnTaken::Beside => Ok(write(place, name, bytes)?),
        OnTaken::Ask => {
            let sha = sha_of(place);
            Err(WriteRefused::NameTaken { on_stick: old, sha })
        }
        OnTaken::Over(shown) => {
            if sha_of(place) != Some(shown) {
                return Err(fill(EN.files_taken_changed, &[&old]).into());
            }
            Ok(replace_file(place, &old, name, bytes, renamed)?)
        }
    }
}

/// The new bytes under a hidden temp name, read back and compared; then
/// the file `original` removed and the temp renamed to its name, or to
/// the person's new one where `renamed`.
fn replace_file(
    place: &mut dyn Place,
    original: &str,
    name: &str,
    bytes: &[u8],
    renamed: bool,
) -> Result<String, String> {
    let temp = format!(".faraday-{original}.part");
    put_checked(place, &temp, bytes)?;
    // FAT has no atomic replace: the original goes, then the copy that
    // read back whole takes its name, or the person's new one.
    place.remove(original)?;
    let to = if renamed {
        free_name(place, checked_for_write(name)?)?
    } else {
        original.to_string()
    };
    place.rename(&temp, &to)?;
    Ok(to)
}

/// Why a rename on a stick was refused (`docs/FLOWS.md` decision 61
/// item 4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unrenamed {
    /// Another file on the stick has the new name, ignoring case.
    Taken,
    /// That other file is the same vault: both files' SHA-256 are among
    /// the copies of one vault the session knows (`docs/VAULT.md` §6).
    SameVault,
    /// The stick refused, or a name is not one the app may use.
    Failed(String),
}

/// Costs a read and a SHA-256 pass over both files where both are vaults
/// of one length. Renames a file on a stick in place, its directory entry
/// alone, as the person asked on a visit (`docs/FLOWS.md` decision 61
/// item 4). Refused, nothing written, where another file has the new
/// name, ignoring case: [`Unrenamed::SameVault`] where both are copies of
/// one of `vaults`.
pub fn rename(
    c: &Compute,
    place: &mut dyn Place,
    from: &str,
    to: &str,
    vaults: &[Vec<[u8; 32]>],
) -> Result<(), Unrenamed> {
    let from = checked(from).map_err(Unrenamed::Failed)?;
    let to = checked_for_write(to).map_err(Unrenamed::Failed)?;
    let files = place.files().map_err(Unrenamed::Failed)?;
    let Some(&(_, len)) = files.iter().find(|(n, _)| n == from) else {
        return Err(Unrenamed::Failed(fill(EN.files_not_on_stick, &[from])));
    };
    if let Some((other, olen)) = files
        .iter()
        .find(|(n, _)| n != from && n.eq_ignore_ascii_case(to))
    {
        let both = len == *olen
            && vault_file::is_vault_len(len)
            && len <= MAX_VAULT_READ
            && has_vault_head(place, from)
            && has_vault_head(place, other);
        let same = both && {
            let a = place.read(from, MAX_VAULT_READ).ok();
            let b = place.read(other, MAX_VAULT_READ).ok();
            match (a, b) {
                (Some(a), Some(b)) => {
                    let (a, b) = (
                        faraday_crypto::sha256_whole(c, &a),
                        faraday_crypto::sha256_whole(c, &b),
                    );
                    vaults.iter().any(|v| v.contains(&a) && v.contains(&b))
                }
                _ => false,
            }
        };
        return Err(if same {
            Unrenamed::SameVault
        } else {
            Unrenamed::Taken
        });
    }
    place.rename(from, to).map_err(Unrenamed::Failed)
}

/// Whether these bytes are a whole vault file: the header's own fields
/// end it, and the stream of blocks after its hash and HMAC runs to the
/// empty block that ends it, at the end of the file.
fn whole_vault(b: &[u8]) -> bool {
    if !vault_file::is_vault_file(b) {
        return false;
    }
    let u32_at = |at: usize| {
        b.get(at..at + 4)
            .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]) as usize)
    };
    let mut at = vault_file::HEAD_LEN;
    loop {
        let (Some(&id), Some(size)) = (b.get(at), u32_at(at + 1)) else {
            return false;
        };
        let Some(next) = (at + 5).checked_add(size) else {
            return false;
        };
        at = next;
        if id == 0 {
            break;
        }
    }
    // The header's SHA-256 and HMAC.
    at += 64;
    loop {
        let Some(size) = u32_at(at + 32) else {
            return false;
        };
        let Some(next) = (at + 36).checked_add(size) else {
            return false;
        };
        at = next;
        if size == 0 {
            return at == b.len();
        }
    }
}

/// Finishes a vault write-back a pulled stick interrupted
/// (`docs/VAULT.md` §6 step 4). A part file whose original is gone is a
/// whole vault that only missed its rename, and takes the name; a part
/// file beside its original is a write that never finished, and goes.
pub fn finish_vault_writes(place: &mut dyn Place) {
    let Ok(files) = place.files() else { return };
    for (n, len) in &files {
        let Some(name) = n
            .strip_prefix(".faraday-")
            .and_then(|r| r.strip_suffix(".part"))
        else {
            continue;
        };
        // A part file of a vault under another name is told by its
        // contents; a part file of any other file is not this function's.
        let named = name.to_ascii_lowercase().ends_with(".kdbx");
        if !named && !(vault_file::is_vault_len(*len) && *len <= MAX_VAULT_READ) {
            continue;
        }
        // A part file larger than any vault is a crafted one: it goes
        // without being read. The rest are read once.
        let whole =
            *len <= MAX_VAULT_READ && place.read(n, MAX_VAULT_READ).is_ok_and(|b| whole_vault(&b));
        if !named && !whole {
            continue;
        }
        if files.iter().any(|(o, _)| o.eq_ignore_ascii_case(name)) || !whole {
            let _ = place.remove(n);
            continue;
        }
        // A whole vault takes the name it was written for only where a
        // write could have given that name. The name of a vault that
        // was not `.kdbx` is gone with the original, so its copy takes
        // `.kdbx` after it, and never a name that could be another
        // kind of file's.
        let target = if named {
            name.to_string()
        } else {
            format!("{name}.kdbx")
        };
        let taken = files.iter().any(|(o, _)| o.eq_ignore_ascii_case(&target));
        if checked_for_write(&target).is_ok() && !taken {
            let _ = place.rename(n, &target);
        } else if named {
            let _ = place.remove(n);
        }
    }
}

/// The settings file's name (`faraday_core::stick::settings::FILE`).
pub const SETTINGS_FILE: &str = "faraday-settings.txt";

/// Writes the settings file over the one already there: the new bytes
/// beside it, read back and compared, then the old file goes and the new
/// one takes its name. A stick pulled between the two keeps no settings
/// file, and the next visit writes one.
pub fn write_settings(place: &mut dyn Place, bytes: &[u8]) -> Result<String, String> {
    let temp = format!(".faraday-{SETTINGS_FILE}.part");
    put_checked(place, &temp, bytes)?;
    let old: Vec<String> = place
        .files()?
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| n.eq_ignore_ascii_case(SETTINGS_FILE))
        .collect();
    for n in old {
        if let Err(e) = place.remove(&n) {
            let _ = place.remove(&temp);
            return Err(e);
        }
    }
    place.rename(&temp, SETTINGS_FILE)?;
    Ok(SETTINGS_FILE.to_string())
}

/// Costs what [`write_file`] costs. Writes what the app put in the
/// Outbox, a vault as one that no copy is known of: the settings over
/// the settings, anything else over a file of its name when `replace`
/// ([`write_replacing`]) or else as a new file.
pub fn write_any(
    c: &Compute,
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    replace: bool,
) -> Result<String, String> {
    write_file(c, place, name, bytes, &[], replace, false, OnTaken::Beside)
        .map_err(WriteRefused::text)
}

/// Costs what [`write_vault_over`] costs for a vault. [`write_any`] for
/// a file whose name the person changed For the stick when `renamed`: a
/// vault goes over the file on the stick whose SHA-256 is among `copies`,
/// only when `replace`, which the app says of a vault its session
/// unlocked or wrote, and then takes that name ([`write_vault_over`]).
/// `on_taken` is for a vault only: what to do where a file the copies do
/// not prove has its name.
#[expect(
    clippy::too_many_arguments,
    reason = "one write, as the protocol carries it"
)]
pub fn write_file(
    c: &Compute,
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    copies: &[[u8; 32]],
    replace: bool,
    renamed: bool,
    on_taken: OnTaken,
) -> Result<String, WriteRefused> {
    if vault_file::is_vault_file(bytes) {
        write_vault_over(c, place, name, bytes, copies, replace, renamed, on_taken)
    } else if name.eq_ignore_ascii_case(SETTINGS_FILE) && bytes.starts_with(b"faraday-settings ") {
        Ok(write_settings(place, bytes)?)
    } else if replace {
        Ok(write_replacing(place, name, bytes)?)
    } else {
        Ok(write(place, name, bytes)?)
    }
}

/// A PNG on a stick: its bytes as they are, and its QR codes' payloads.
/// A picture whose codes cannot be read (not a PNG, too many pixels) is
/// still read, with no codes: it is copied as a file.
pub type Picture = (Vec<u8>, Vec<Vec<u8>>);

/// Costs a read of the file and [`qr_in_png`]: a PNG on a stick, read,
/// its bytes and its QR codes ([`Picture`]).
pub fn read_qr_png(c: &Compute, place: &mut dyn Place, name: &str) -> Result<Picture, String> {
    let bytes = read(place, name)?;
    // Moved, not copied, to whoever holds the picture.
    let mut codes = qr_in_png(c, &bytes).unwrap_or_default();
    Ok((bytes, std::mem::take(&mut *codes)))
}

/// Costs [`qr_in_png`]: a picture's bytes as the disk process answers
/// them ([`proto::Response::Qr`]), the bytes as they are, and its QR
/// codes, none when they cannot be read.
pub fn picture_answer(c: &Compute, bytes: Vec<u8>) -> proto::Response {
    // Moved into the answer, which wipes them when it is dropped.
    let mut codes = qr_in_png(c, &bytes).unwrap_or_default();
    let codes = std::mem::take(&mut *codes);
    proto::Response::Qr { bytes, codes }
}

/// Costs a PNG decode and [`faraday_codec::decode::decode_picture`]: a
/// third of a second for a 12-megapixel photograph, seconds on a Pi, so
/// a shell reads pictures off its own thread. The QR codes in a PNG's
/// bytes, dark on light or light on dark, wiped when dropped: a code may
/// be a SeedQR.
pub fn qr_in_png(c: &Compute, bytes: &[u8]) -> Result<zeroize::Zeroizing<Vec<Vec<u8>>>, String> {
    c.count(Work::PngDecode);
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| EN.files_picture_unread)?;
    let (w, h) = {
        let info = reader.info();
        (info.width as usize, info.height as usize)
    };
    if w.saturating_mul(h) > MAX_PIXELS {
        return Err(EN.files_picture_large.into());
    }
    // The picture may be of a SeedQR: every copy of its pixels is wiped.
    let mut buf = zeroize::Zeroizing::new(vec![
        0u8;
        reader
            .output_buffer_size()
            .ok_or(EN.files_picture_large)?
    ]);
    let info = reader
        .next_frame(&mut buf)
        .map_err(|_| EN.files_picture_unread)?;
    let px = &buf[..info.buffer_size()];
    let luma_of =
        |r: u8, g: u8, b: u8| ((u32::from(r) * 3 + u32::from(g) * 6 + u32::from(b)) / 10) as u8;
    let luma: zeroize::Zeroizing<Vec<u8>> = zeroize::Zeroizing::new(match info.color_type {
        png::ColorType::Grayscale => px.to_vec(),
        png::ColorType::GrayscaleAlpha => px.chunks_exact(2).map(|p| p[0]).collect(),
        png::ColorType::Rgb => px
            .chunks_exact(3)
            .map(|p| luma_of(p[0], p[1], p[2]))
            .collect(),
        png::ColorType::Rgba => px
            .chunks_exact(4)
            .map(|p| luma_of(p[0], p[1], p[2]))
            .collect(),
        png::ColorType::Indexed => return Err(EN.files_picture_unread.into()),
    });
    if luma.len() != w * h {
        return Err(EN.files_picture_unread.into());
    }
    // A code printed light on dark is read too.
    Ok(zeroize::Zeroizing::new(
        faraday_codec::decode::decode_picture(c, w, h, &luma)
            .into_iter()
            .map(|mut d| std::mem::take(&mut *d.bytes))
            .collect(),
    ))
}
