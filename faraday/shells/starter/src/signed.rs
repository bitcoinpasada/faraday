//! A boot file the person signed with their own Secure Boot db key, in
//! place of the release's `EFI/BOOT/BOOTX64.EFI` (`docs/STARTER.md` §4.5,
//! `docs/DECISIONS.md` F133).
//!
//! Only its signature is taken from it. Its Authenticode image hash,
//! which leaves the signature out, must be the image hash of the
//! release's `BOOTX64.EFI` as the payload checked against its manifest
//! holds it, and its certificate table must be one signature alone at its
//! end. The file written is then built from the release's own bytes,
//! padded as signing pads them, with that file's checksum, certificate
//! table entry and certificate table (`faraday_sb::pe::transplant`), so
//! every byte the signature does not cover is the release's by
//! construction; the signature must hold over what is built, by a
//! certificate it carries, whose name and fingerprint are shown for the
//! person to compare with their own db key's. Anything else is refused
//! by name. Which key signed it is not judged here: a computer's db
//! decides that when it starts the stick.
//!
//! The window checks it when it is chosen ([`choose`]); the writer, the
//! elevated copy on Windows, reads it again, refuses it unless it still
//! has the SHA-256 that was chosen, and builds and checks it again
//! ([`prepare`]) before anything is written. The elevated copy opens only
//! a path on a drive (`X:\…`, `windows::elevated`), refuses a reparse
//! point or anything but a file, and says no system's words back. The
//! boot partition written is the release's with that one file written
//! over where it is (`faraday_fat::Volume::rewrite_in`): its clusters,
//! the clusters it grows by, its folder entry and the FATs differ,
//! nothing else.

use std::fs::{File, OpenOptions};
use std::io::{Cursor, Read};
use std::path::Path;

use faraday_payload::ImageKind;
use faraday_sb::pe::{self, Signer, Transplant};
use faraday_starter::api::{BootFile, BootSigner, Reason};

use crate::writer::{self, Job, Source};

/// Where the boot file is in a stick's boot partition.
pub const BOOT_PATH: [&str; 3] = ["EFI", "BOOT", "BOOTX64.EFI"];

/// The largest boot file taken: a whole boot partition (F80).
pub const MAX_BOOT_FILE: u64 = faraday_boot::MAX_PARTITION;

/// The most characters of a certificate's name shown.
const MAX_NAME: usize = 64;

/// The release's boot partition with the signed file in it: where the
/// partition starts in what the source writes, and its bytes.
#[derive(Debug, Clone)]
pub struct Patch {
    /// The partition's first byte in the source's decompressed stream.
    pub at: u64,
    /// The partition as it is written.
    pub bytes: Vec<u8>,
}

/// Opens a file to read: on Windows the file itself, never what a
/// reparse point leads to.
#[cfg(windows)]
fn open(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(not(windows))]
fn open(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

/// Whether what was opened is a reparse point (Windows).
#[cfg(windows)]
fn reparse(meta: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn reparse(_: &std::fs::Metadata) -> bool {
    false
}

/// Costs a read of the file at `path`: its bytes, refused past
/// [`MAX_BOOT_FILE`], and when it is not a plain file. A failure says
/// which file, and none of the system's words.
fn read(path: &Path) -> Result<Vec<u8>, Reason> {
    let shown = path.display().to_string();
    let unread = || Reason::BootFileRead(shown.clone());
    let f = open(path).map_err(|_| unread())?;
    let meta = f.metadata().map_err(|_| unread())?;
    if !meta.is_file() || reparse(&meta) {
        return Err(unread());
    }
    let mut bytes = Vec::new();
    f.take(MAX_BOOT_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unread())?;
    if bytes.len() as u64 > MAX_BOOT_FILE {
        return Err(Reason::BootFileLarge(shown));
    }
    Ok(bytes)
}

/// Costs a SHA-256 of `bytes`, hex.
fn sha256(bytes: &[u8]) -> String {
    faraday_payload::hash_image(Cursor::new(bytes), ImageKind::Plain)
        .map(|s| s.sha256.to_string())
        .unwrap_or_default()
}

/// A certificate's common name as it may be shown: no control or
/// direction characters, at most [`MAX_NAME`] characters. The name is
/// whatever the certificate's maker chose; the fingerprint is what is
/// compared.
pub fn shown_name(cn: &str) -> String {
    let bidi = |c: char| matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}');
    cn.chars()
        .filter(|&c| !c.is_control() && !bidi(c))
        .take(MAX_NAME)
        .collect()
}

/// Costs a read of the release's boot file from its boot partition.
fn release_file(partition: &[u8]) -> Result<Vec<u8>, Reason> {
    let mut v = faraday_fat::Volume::open(partition.to_vec())
        .map_err(|e| Reason::ReleaseBootFile(e.reason().to_string()))?;
    v.read_in(&BOOT_PATH, MAX_BOOT_FILE as u32)
        .map_err(|e| Reason::ReleaseBootFile(e.reason().to_string()))
}

/// Costs two Authenticode image hashes and an RSA verification per
/// certificate the signature carries: the boot file to write, built from
/// `release` with `signed`'s signature, and who signed it, when `signed`
/// is the release's program with one signature that holds.
pub fn build(release: &[u8], signed: &[u8]) -> Result<(Vec<u8>, BootSigner), Reason> {
    let built = pe::transplant(release, signed).map_err(|t| match t {
        Transplant::Release => Reason::ReleaseBootFile(faraday_sb::Error::Image.reason().into()),
        Transplant::Other => Reason::BootFileOther,
        Transplant::Unsigned => Reason::BootFileUnsigned,
        Transplant::Layout => Reason::BootFileLayout,
    })?;
    match pe::signer(&built) {
        Ok(Signer::By(cert)) => {
            let signer = BootSigner {
                subject: shown_name(&faraday_sb::common_name(&cert).unwrap_or_default()),
                fingerprint: sha256(&cert),
            };
            Ok((built, signer))
        }
        Ok(Signer::Unsigned) => Err(Reason::BootFileUnsigned),
        Ok(Signer::Broken) => Err(Reason::BootFileBroken),
        Err(_) => Err(Reason::BootFileOther),
    }
}

/// Costs a listing of `folder`: its `.efi` files (any case), as full
/// paths, the newest first, for What to write's "One I signed". Folders
/// in it are not entered.
pub fn list(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<(std::time::SystemTime, String)> = entries
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".efi")
        })
        .filter_map(|e| {
            let meta = e.metadata().ok().filter(std::fs::Metadata::is_file)?;
            let when = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            Some((when, e.path().display().to_string()))
        })
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    found.into_iter().map(|(_, p)| p).collect()
}

/// Costs a decompression of the whole image `job` holds, its SHA-256, a
/// read of the file at `path` and [`build`]: the boot file the person
/// chose, as the order names it, and who signed it, for Confirm. The
/// window's check; the writer makes it again ([`prepare`]).
pub fn choose(path: &Path, job: &Job) -> Result<(BootFile, BootSigner), Reason> {
    // The elevated writer opens only a path on a drive: say so now.
    if cfg!(windows) && !crate::windows::elevated::drive_path(&path.display().to_string()) {
        return Err(Reason::BootFileRead(path.display().to_string()));
    }
    let signed = read(path)?;
    let (_, partition) = writer::boot_partition(&job.source())?;
    let (_, signer) = build(&release_file(&partition)?, &signed)?;
    let file = BootFile {
        path: path.display().to_string(),
        sha256: sha256(&signed),
    };
    Ok((file, signer))
}

/// Costs a read of the boot file, a decompression of the whole source,
/// its SHA-256 and [`build`]: the writer's own check of the boot file an
/// order names, made before the disk is touched, and the boot partition
/// it writes in place of the release's. Refused when the file is not
/// the bytes chosen, then as [`build`] refuses it.
pub fn prepare(file: &BootFile, source: &Source<'_>) -> Result<Patch, Reason> {
    let signed = read(Path::new(&file.path))?;
    if sha256(&signed) != file.sha256 {
        return Err(Reason::BootFileChanged(file.path.clone()));
    }
    let (at, partition) = writer::boot_partition(source)?;
    let (built, _) = build(&release_file(&partition)?, &signed)?;
    let mut v = faraday_fat::Volume::open(partition)
        .map_err(|e| Reason::ReleaseBootFile(e.reason().to_string()))?;
    v.rewrite_in(&BOOT_PATH, &built)
        .map_err(|e| Reason::ReleaseBootFile(e.reason().to_string()))?;
    Ok(Patch {
        at,
        bytes: v.into_disk(),
    })
}
