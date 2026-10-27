//! Which Faraday a stick holds (§4.1): the kernel's release string on
//! its boot partition (`docs/DECISIONS.md` F5), found by
//! `faraday_boot::find_release`.
//! Only the boot partition is read, read-only, and only these two
//! files on it; the data partition is never opened.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use faraday_boot::{MAX_PARTITION, MAX_RELEASE, find_release};

/// The Pi's boot partition names its kernel's release here (§1.3): its
/// zImage is compressed, so the string is not in its bytes.
pub const BUILD_FILE: &str = "faraday-build.txt";
/// The PC stick's one file, the kernel with the whole system inside,
/// folder by folder (joined one at a time, so a Windows volume's
/// `\\?\Volume{…}\` root takes it as it takes any path).
pub const KERNEL_FILE: [&str; 3] = ["EFI", "BOOT", "BOOTX64.EFI"];

/// What is read at a time.
const CHUNK: usize = 1 << 20;

/// The release string on a boot partition mounted at `root`: the Pi's
/// build file, else the PC stick's kernel searched.
pub fn release_at(root: &Path) -> Option<String> {
    let build = root.join(BUILD_FILE);
    if let Ok(f) = File::open(&build) {
        let mut text = Vec::new();
        if f.take(MAX_RELEASE as u64 * 2)
            .read_to_end(&mut text)
            .is_ok()
            && let Some(r) = find_release(&text)
        {
            return Some(r);
        }
    }
    let kernel = KERNEL_FILE
        .iter()
        .fold(root.to_path_buf(), |p, c| p.join(c));
    release_in_file(&kernel)
}

/// The first release string in the first [`MAX_PARTITION`] bytes of
/// `path` (a kernel, a boot partition image, a whole stick image's
/// first bytes), read a chunk at a time.
pub fn release_in_file(path: &Path) -> Option<String> {
    let mut f = File::open(path).ok()?;
    // Each chunk keeps the end of the one before, so a string that
    // straddles two is still seen whole.
    let overlap = MAX_RELEASE * 2;
    let mut buf = vec![0u8; CHUNK + overlap];
    let mut kept = 0usize;
    let mut read = 0u64;
    loop {
        let mut n = kept;
        while n < buf.len() {
            match f.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => n += k,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => return None,
            }
        }
        if let Some(r) = find_release(&buf[..n]) {
            return Some(r);
        }
        read += (n - kept) as u64;
        if n < buf.len() || read >= MAX_PARTITION {
            return None;
        }
        buf.copy_within(n - overlap..n, 0);
        kept = overlap;
    }
}
