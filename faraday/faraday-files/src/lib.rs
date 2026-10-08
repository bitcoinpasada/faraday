//! A stick's files as Faraday reads and writes them (`PLAN.md` §4.3,
//! `docs/VAULT.md` §6), the same rules whatever holds them:
//!
//! - a [`Dir`], a folder: the desktop app's sticks;
//! - a [`Fat`], a FAT16 or FAT32 volume read by `faraday-fat`: the
//!   disk process's partitions on the device.
//!
//! Only the top level of a stick is used. A write is a new file, read
//! back and compared, then renamed into place; it never goes over a file
//! already there, except a sealed vault going back over its own file,
//! found by salt and length, and the settings file over the settings
//! file. A vault write-back a pulled stick interrupted is finished on the
//! next visit. Pictures are decoded for QR codes here, so only the codes'
//! contents reach the app (`docs/QR.md` §4).
//!
//! [`proto`] is how the app's shell and the disk process talk.

pub mod proto;

use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

/// The largest file read from a stick. A PSBT or a wallet is kilobytes; a
/// vault with four 4 MiB slots is the largest thing Faraday writes.
pub const MAX_READ: u64 = 18 * 1024 * 1024;

/// The largest picture read for QR codes, in pixels.
pub const MAX_PIXELS: usize = 40_000_000;

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
    /// Renames a file; refused when the new name is taken.
    fn rename(&mut self, from: &str, to: &str) -> Result<(), String>;
}

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

    fn rename(&mut self, from: &str, to: &str) -> Result<(), String> {
        self.0
            .rename(from, to)
            .and_then(|()| self.0.flush())
            .map_err(|e| e.reason().to_string())
    }
}

/// A name the app may use: one path component, no hidden files.
pub fn checked(name: &str) -> Result<&str, String> {
    if name.is_empty()
        || name.starts_with('.')
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0')
    {
        return Err(format!("{name:?} is not a file name"));
    }
    Ok(name)
}

/// The files a person sees: no hidden ones, sorted by name.
pub fn listing(place: &mut dyn Place) -> Vec<(String, u64)> {
    let mut files: Vec<(String, u64)> = place
        .files()
        .unwrap_or_default()
        .into_iter()
        .filter(|(n, _)| !n.starts_with('.'))
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
    // read back whole takes its name.
    place.remove(&original)?;
    place.rename(&temp, &original)?;
    Ok(original)
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
        if place.read(n, *len).is_ok_and(|b| whole_vault(&b)) {
            let _ = place.rename(n, name);
        } else {
            let _ = place.remove(n);
        }
    }
}

/// The settings file's name (`faraday_core::stick_settings::FILE`).
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
/// own file, the settings over the settings, anything else as a new file.
pub fn write_any(place: &mut dyn Place, name: &str, bytes: &[u8]) -> Result<String, String> {
    if name.ends_with(".ofv") && bytes.starts_with(b"OFVT") {
        write_vault(place, name, bytes)
    } else if name.eq_ignore_ascii_case(SETTINGS_FILE) && bytes.starts_with(b"faraday-settings ") {
        write_settings(place, bytes)
    } else {
        write(place, name, bytes)
    }
}

/// The QR codes in a PNG on a stick, as their payload bytes.
pub fn read_qr_png(place: &mut dyn Place, name: &str) -> Result<Vec<Vec<u8>>, String> {
    qr_in_png(&read(place, name)?)
}

/// The QR codes in a PNG's bytes.
pub fn qr_in_png(bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
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
    let codes: Vec<Vec<u8>> = osk_codec::decode::decode_luma(w, h, &luma)
        .into_iter()
        .map(|d| d.bytes)
        .collect();
    if !codes.is_empty() {
        return Ok(codes);
    }
    // A code printed light on dark reads once the picture is inverted.
    let inverted: zeroize::Zeroizing<Vec<u8>> =
        zeroize::Zeroizing::new(luma.iter().map(|b| 255 - b).collect());
    Ok(osk_codec::decode::decode_luma(w, h, &inverted)
        .into_iter()
        .map(|d| d.bytes)
        .collect())
}
