//! A stick's partition table, read by the Starter from whatever stick is
//! attached: `gpt::read` over the input as a disk image, then
//! `faraday_boot` on the table it reads. The header's and the entries'
//! CRCs must hold or the reader stops at the first check, so the target
//! rewrites both for each sector size before the call, as a hostile
//! writer would. The Starter shell is a binary crate, so `gpt.rs` is
//! included by path. The same bytes as a boot partition's, searched for
//! the release string the Starter reads a stick's version by
//! (`faraday_boot::find_release`).

#![no_main]

#[path = "../../shells/starter/src/gpt.rs"]
#[allow(dead_code, missing_docs)]
mod gpt;

use faraday_crypto::crc32;
use libfuzzer_sys::fuzz_target;

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// The header's and the entries' CRCs made right for `sector`-byte
/// sectors, where the fields they depend on are in range.
fn fix(disk: &mut [u8], sector: usize) {
    if disk.len() < 2 * sector {
        return;
    }
    let Some(size) = le32(disk, sector + 12).map(|s| s as usize) else {
        return;
    };
    let (Some(lba), Some(count), Some(entry)) = (
        le64(disk, sector + 72),
        le32(disk, sector + 80),
        le32(disk, sector + 84),
    ) else {
        return;
    };
    if !(92..=sector).contains(&size) || count == 0 || count > 1024 {
        return;
    }
    let entries_len = count as usize * entry as usize;
    let Some(at) = (lba as usize).checked_mul(sector) else {
        return;
    };
    if let Some(end) = at.checked_add(entries_len)
        && end <= disk.len()
    {
        let sum = crc32(&disk[at..end]).to_le_bytes();
        disk[sector + 88..sector + 92].copy_from_slice(&sum);
    }
    disk[sector + 16..sector + 20].fill(0);
    let sum = crc32(&disk[sector..sector + size]).to_le_bytes();
    disk[sector + 16..sector + 20].copy_from_slice(&sum);
}

fn table(disk: &[u8]) {
    let len = disk.len() as u64;
    let mut read_at = |at: u64, buf: &mut [u8]| -> std::io::Result<()> {
        let from = usize::try_from(at).map_err(|_| std::io::ErrorKind::UnexpectedEof)?;
        let to = from
            .checked_add(buf.len())
            .ok_or(std::io::ErrorKind::UnexpectedEof)?;
        buf.copy_from_slice(disk.get(from..to).ok_or(std::io::ErrorKind::UnexpectedEof)?);
        Ok(())
    };
    if let Ok(t) = gpt::read(&mut read_at, len) {
        let _ = gpt::faraday_boot(&t, 1 << 20);
        let _ = gpt::faraday_boot(&t, u64::MAX);
    }
}

fuzz_target!(|data: &[u8]| {
    if let Some(release) = faraday_boot::find_release(data) {
        assert!(release.len() <= faraday_boot::MAX_RELEASE);
        assert!(release.contains("-faraday-"));
    }
    table(data);
    for sector in [512, 4096] {
        let mut disk = data.to_vec();
        fix(&mut disk, sector);
        table(&disk);
    }
});
