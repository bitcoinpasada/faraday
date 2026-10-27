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
//! found by salt and length, and the settings file over the settings
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

use faraday_crypto::work::{Compute, Work};

/// The largest file read from a stick. A PSBT or a wallet is kilobytes; a
/// vault with four 4 MiB slots is the largest thing Faraday writes.
pub const MAX_READ: u64 = 18 * 1024 * 1024;

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
pub const ZERO_STEP: u64 = 8 << 20;

/// A folder standing for a stick.
pub struct Dir(pub PathBuf);

impl Place for Dir {
    fn files(&mut self) -> Result<Vec<(String, u64)>, String> {
        Ok(fs::read_dir(&self.0)
            .map_err(|e| e.to_string())?
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
        let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
        if meta.len() > max {
            return Err(format!("larger than {} MB", max / 1024 / 1024));
        }
        fs::read(&path).map_err(|e| e.to_string())
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let mut f = fs::File::create_new(self.0.join(name)).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        fs::remove_file(self.0.join(name)).map_err(|e| e.to_string())
    }

    fn wipe(&mut self, name: &str) -> Result<(), String> {
        use std::io::{Read, Seek, SeekFrom};
        let path = self.0.join(name);
        let mut f = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        let zeros = [0u8; 64 * 1024];
        let mut left = len;
        while left > 0 {
            let n = left.min(zeros.len() as u64) as usize;
            f.write_all(&zeros[..n]).map_err(|e| e.to_string())?;
            left -= n as u64;
        }
        f.sync_all().map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut back = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut back).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if back[..n].iter().any(|&b| b != 0) {
                return Err("a block kept its old bytes".to_string());
            }
        }
        drop(f);
        fs::remove_file(path).map_err(|e| e.to_string())
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
            return Err(format!("{to} is already there"));
        }
        fs::rename(self.0.join(from), self.0.join(to)).map_err(|e| e.to_string())
    }
}

/// A FAT16 or FAT32 volume.
pub struct Fat<D: faraday_fat::Disk>(pub faraday_fat::Volume<D>);

impl<D: faraday_fat::Disk> Place for Fat<D> {
    fn files(&mut self) -> Result<Vec<(String, u64)>, String> {
        Ok(self
            .0
            .list()
            .map_err(|e| e.reason().to_string())?
            .into_iter()
            .filter(|e| !e.dir)
            .map(|e| (e.name, u64::from(e.size)))
            .collect())
    }

    fn read(&mut self, name: &str, max: u64) -> Result<Vec<u8>, String> {
        let max = u32::try_from(max).unwrap_or(u32::MAX);
        self.0.read(name, max).map_err(|e| e.reason().to_string())
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        self.0
            .create(name, bytes)
            .and_then(|()| self.0.flush())
            .map_err(|e| e.reason().to_string())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        self.0
            .delete(name)
            .and_then(|()| self.0.flush())
            .map_err(|e| e.reason().to_string())
    }

    fn wipe(&mut self, name: &str) -> Result<(), String> {
        self.0.wipe(name).map_err(|e| e.reason().to_string())
    }

    fn space(&mut self) -> Result<(u64, u64), String> {
        self.0.space().map_err(|e| e.reason().to_string())
    }

    fn zero_free(&mut self, from: u32) -> Result<Zeroed, String> {
        self.0
            .zero_free(from, ZERO_STEP)
            .map_err(|e| e.reason().to_string())
    }

    fn make_empty(&mut self) -> Result<(), String> {
        self.0.make_empty().map_err(|e| e.reason().to_string())
    }

    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.0
            .rename(from, to)
            .and_then(|()| self.0.flush())
            .map_err(|e| e.reason().to_string())
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
    "System Volume Information",
    "$RECYCLE.BIN",
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

/// A name the app may use: one path component, no hidden files, and
/// none of an operating system's own ([`foreign`]).
pub fn checked(name: &str) -> Result<&str, String> {
    if name.is_empty()
        || foreign(name)
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(format!("{name:?} is not a file name"));
    }
    Ok(name)
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
    if name.to_ascii_lowercase().ends_with(".ofv") {
        finish_vault_writes(place);
    }
    place.read(checked(name)?, MAX_READ)
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
        .ok_or_else(|| "no free name".to_string())
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
        return Err("read back different from what was written".to_string());
    }
    Ok(())
}

/// Writes a file as a new file, reads it back, compares, and renames it
/// into place. Returns the name it was written under.
pub fn write(place: &mut dyn Place, name: &str, bytes: &[u8]) -> Result<String, String> {
    let name = free_name(place, checked(name)?)?;
    let temp = format!(".faraday-{name}.part");
    put_checked(place, &temp, bytes)?;
    if has(place, &name)? {
        let _ = place.remove(&temp);
        return Err(format!("{name} appeared while writing"));
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
    let name = checked(name)?;
    let Some(old) = place
        .files()?
        .into_iter()
        .map(|(n, _)| n)
        .find(|n| n.eq_ignore_ascii_case(name))
    else {
        return write(place, name, bytes);
    };
    // A vault on the stick is never removed by a replacing write: only
    // `write_vault` goes over a vault, and only over the same one.
    if old.to_ascii_lowercase().ends_with(".ofv") {
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

/// A vault's identity: its salt and its length (`docs/VAULT.md` §6).
/// `None` for anything that is not a vault.
fn vault_identity(bytes: &[u8], len: u64) -> Option<([u8; 32], u64)> {
    if bytes.len() < 53 || bytes[0..4] != *b"OFVT" {
        return None;
    }
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&bytes[21..53]);
    Some((salt, len))
}

/// Writes a sealed vault back (`docs/VAULT.md` §6): over the vault on the
/// stick with the same salt and length, or as a new file when there is
/// none. The new bytes are written beside the original, read back and
/// compared before the original is replaced. Two vaults with the same
/// identity on one stick are refused, naming both.
pub fn write_vault(place: &mut dyn Place, name: &str, bytes: &[u8]) -> Result<String, String> {
    write_vault_over(place, name, bytes, true, false)
}

/// [`write_vault`], refusing to go over the same vault when `over` is
/// false. Over the same vault the file keeps the name it has on the
/// stick, unless `renamed`: the person renamed the vault's file For the
/// stick, and the file takes that name (`docs/FLOWS.md` decision 61
/// item 2), or `name-2.ofv` where another file has it.
pub fn write_vault_over(
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    over: bool,
    renamed: bool,
) -> Result<String, String> {
    finish_vault_writes(place);
    let want = vault_identity(bytes, bytes.len() as u64).ok_or("not a vault")?;
    let mut same: Vec<String> = Vec::new();
    for (n, len) in place.files()? {
        if !n.to_ascii_lowercase().ends_with(".ofv") || n.starts_with('.') || len != want.1 {
            continue;
        }
        if let Ok(b) = place.read(&n, len)
            && vault_identity(&b, len) == Some(want)
        {
            same.push(n);
        }
    }
    let original = match same.len() {
        0 => return write(place, name, bytes),
        1 if !over => {
            return Err(format!(
                "{} is this vault, and this session did not unlock or write it: not replaced",
                same[0]
            ));
        }
        1 => same.remove(0),
        _ => {
            return Err(format!(
                "{} and {} are the same vault; remove one",
                same[0], same[1]
            ));
        }
    };
    let temp = format!(".faraday-{original}.part");
    put_checked(place, &temp, bytes)?;
    // FAT has no atomic replace: the original goes, then the copy that
    // read back whole takes its name, or the person's new one.
    place.remove(&original)?;
    let to = if renamed {
        free_name(place, checked(name)?)?
    } else {
        original
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
    /// That other file is the same vault, by identity (`docs/VAULT.md`
    /// §6).
    SameVault,
    /// The stick refused, or a name is not one the app may use.
    Failed(String),
}

/// Renames a file on a stick in place, its directory entry alone, as the
/// person asked on a visit (`docs/FLOWS.md` decision 61 item 4). Refused,
/// nothing written, where another file has the new name, ignoring case:
/// [`Unrenamed::SameVault`] where both are the same vault. Reads both
/// files where both are vaults of one length.
pub fn rename(place: &mut dyn Place, from: &str, to: &str) -> Result<(), Unrenamed> {
    let from = checked(from).map_err(Unrenamed::Failed)?;
    let to = checked(to).map_err(Unrenamed::Failed)?;
    let files = place.files().map_err(Unrenamed::Failed)?;
    let Some(&(_, len)) = files.iter().find(|(n, _)| n == from) else {
        return Err(Unrenamed::Failed(format!("{from} is not on the stick")));
    };
    if let Some((other, olen)) = files
        .iter()
        .find(|(n, _)| n != from && n.eq_ignore_ascii_case(to))
    {
        let vaults = from.to_ascii_lowercase().ends_with(".ofv")
            && other.to_ascii_lowercase().ends_with(".ofv")
            && len == *olen
            && len <= MAX_READ;
        let same = vaults && {
            let a = place.read(from, MAX_READ).ok();
            let b = place.read(other, MAX_READ).ok();
            match (a, b) {
                (Some(a), Some(b)) => {
                    vault_identity(&a, len).is_some()
                        && vault_identity(&a, len) == vault_identity(&b, len)
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

/// Whether these bytes are a whole vault file: its length is the one its
/// slot size gives.
fn whole_vault(b: &[u8]) -> bool {
    b.len() >= 21 && b[0..4] == *b"OFVT" && {
        let slot = u32::from_le_bytes([b[17], b[18], b[19], b[20]]) as usize;
        b.len() == 53 + 4 * (slot + 40)
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
        if !name.to_ascii_lowercase().ends_with(".ofv") {
            continue;
        }
        if files.iter().any(|(o, _)| o.eq_ignore_ascii_case(name)) {
            let _ = place.remove(n);
            continue;
        }
        // A part file larger than any vault is a crafted one: it goes
        // without being read.
        if *len <= MAX_READ && place.read(n, MAX_READ).is_ok_and(|b| whole_vault(&b)) {
            let _ = place.rename(n, name);
        } else {
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

/// Writes what the app put in the Outbox: a sealed vault back over its
/// own file, the settings over the settings, anything else over a file of
/// its name when `replace` ([`write_replacing`]) or else as a new file.
/// A vault is never written over another vault of another identity; over
/// the same one (salt and length) only when `replace`, which the app says
/// of a vault its session unlocked or wrote.
pub fn write_any(
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    replace: bool,
) -> Result<String, String> {
    write_file(place, name, bytes, replace, false)
}

/// [`write_any`], for a file whose name the person changed For the stick
/// when `renamed`: a vault going over its same vault takes that name
/// ([`write_vault_over`]).
pub fn write_file(
    place: &mut dyn Place,
    name: &str,
    bytes: &[u8],
    replace: bool,
    renamed: bool,
) -> Result<String, String> {
    if name.ends_with(".ofv") && bytes.starts_with(b"OFVT") {
        write_vault_over(place, name, bytes, replace, renamed)
    } else if name.eq_ignore_ascii_case(SETTINGS_FILE) && bytes.starts_with(b"faraday-settings ") {
        write_settings(place, bytes)
    } else if replace {
        write_replacing(place, name, bytes)
    } else {
        write(place, name, bytes)
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
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let (w, h) = {
        let info = reader.info();
        (info.width as usize, info.height as usize)
    };
    if w.saturating_mul(h) > MAX_PIXELS {
        return Err("the image is too large".into());
    }
    // The picture may be of a SeedQR: every copy of its pixels is wiped.
    let mut buf = zeroize::Zeroizing::new(vec![
        0u8;
        reader
            .output_buffer_size()
            .ok_or("the image is too large")?
    ]);
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
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
        png::ColorType::Indexed => return Err("a palette image was not expanded".into()),
    });
    if luma.len() != w * h {
        return Err("the image's pixels do not add up".into());
    }
    // A code printed light on dark is read too.
    Ok(zeroize::Zeroizing::new(
        faraday_codec::decode::decode_picture(c, w, h, &luma)
            .into_iter()
            .map(|mut d| std::mem::take(&mut *d.bytes))
            .collect(),
    ))
}
