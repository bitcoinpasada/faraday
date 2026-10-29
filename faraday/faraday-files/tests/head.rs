//! Telling a vault by its first four bytes reads four bytes of a FAT
//! stick, not the file: a vault is up to 16.8 MB, and a listing asks of
//! every file of a vault's length (`docs/VAULT-HARDENING.md` §2.1).
//!
//! The volume is made by `mkfs.fat`, found as `foreign.rs` finds it; a
//! machine without it says so and skips.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{cell::Cell, rc::Rc};

use faraday_codec::vault_file::{MAGIC, file_len};
use faraday_crypto::work::Compute;
use faraday_fat::{Disk, Error, Volume};
use faraday_files::{Fat, Place, vault_names};

/// A volume in memory that counts the bytes read from it.
struct Counting(Vec<u8>, Rc<Cell<usize>>);

impl Disk for Counting {
    fn size(&self) -> u64 {
        self.0.size()
    }
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), Error> {
        self.1.set(self.1.get() + buf.len());
        self.0.read_at(offset, buf)
    }
    fn write_at(&mut self, offset: u64, buf: &[u8]) -> Result<(), Error> {
        self.0.write_at(offset, buf)
    }
}

fn mkfs() -> PathBuf {
    if let Some(dir) = std::env::var_os("FARADAY_FATTOOLS") {
        return Path::new(&dir).join("mkfs.fat");
    }
    ["/usr/sbin", "/sbin"]
        .iter()
        .map(|d| Path::new(d).join("mkfs.fat"))
        .find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("mkfs.fat"))
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-files' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

#[test]
fn a_listing_reads_four_bytes_of_a_vault_on_a_fat_stick_and_not_the_file() {
    let dir = std::env::temp_dir().join(format!("faraday-files-head-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let img = dir.join("data.img");
    let made = Command::new(mkfs())
        .args(["-C", "-F", "32", "-n", "FARADAYDATA"])
        .arg(&img)
        .arg("131072")
        .status();
    if !made.is_ok_and(|s| s.success()) {
        eprintln!("mkfs.fat is not installed: the FAT check is skipped");
        let _ = std::fs::remove_dir_all(&dir);
        return;
    }
    let reads = Rc::new(Cell::new(0));
    let disk = Counting(std::fs::read(&img).unwrap(), reads.clone());
    let mut place = Fat(Volume::open(disk).unwrap());
    let mut vault = vec![0u8; file_len(65_536)];
    vault[..4].copy_from_slice(&MAGIC);
    place.create("VAULT~1.FVL", &vault).unwrap();
    let files = place.files().unwrap();

    reads.set(0);
    let names = vault_names(&compute(), &mut place, &files);
    assert_eq!(names, ["VAULT~1.FVL"]);
    assert!(
        reads.get() < vault.len() / 4,
        "{} bytes were read to tell a vault of {}",
        reads.get(),
        vault.len()
    );
    assert_eq!(place.head("VAULT~1.FVL", 4, u64::MAX).unwrap(), MAGIC);
    let _ = std::fs::remove_dir_all(&dir);
}
