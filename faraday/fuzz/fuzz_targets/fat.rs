//! `faraday-fat` over a partition's bytes: the boot sector, the FATs and
//! the root directory are all the stick's to choose (`docs/STANDARDS.md` §0).
//! Every file listed is read, and a file is created, renamed, deleted,
//! wiped and the free space zeroed, the label set and the volume made
//! empty, so the write paths walk the same untrusted structures; a file
//! read before them reads the same after. The boot sector's label is read
//! on its own as well.

#![no_main]

use faraday_fat::Volume;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The label read from the first sector alone, without opening the
    // volume: what the disk process does to keep FARADAYBOOT back.
    let _ = faraday_fat::boot_sector_label(data);
    let Ok(mut v) = Volume::open(data.to_vec()) else {
        return;
    };
    let _ = v.label();
    let Ok(list) = v.list() else { return };
    for e in list.iter().take(64) {
        let _ = v.read(&e.name, 1 << 20);
        // A piece of the file, where and how long the input says.
        let at = |i: usize| data.get(i).copied().unwrap_or(0) as u64;
        let from = at(0) | at(1) << 8 | at(2) << 16;
        let len = (at(3) | at(4) << 8 | at(5) << 16) as usize;
        let _ = v.read_part(&e.name, from, len, 1 << 20);
    }
    let before: Vec<_> = list
        .iter()
        .take(64)
        .filter(|e| !e.name.to_ascii_lowercase().starts_with("fuzz"))
        .map(|e| (e.name.clone(), v.read_checked(&e.name, 1 << 20).ok()))
        .collect();
    let _ = v.head("fuzz.txt", 12, 1 << 20);
    let _ = v.space();
    let _ = v.volume_label();
    if v.create("fuzz.txt", b"faraday").is_ok() {
        let _ = v.head("fuzz.txt", 3, 1 << 20);
        let _ = v.rename("fuzz.txt", "fuzzed.txt");
        let _ = v.delete("fuzzed.txt");
    }
    let _ = v.set_label(faraday_fat::label_bytes("FUZZ").unwrap_or(None));
    let mut from = 2;
    for _ in 0..8 {
        match v.zero_free(from, 1 << 16) {
            Ok(z) if !z.done => from = z.next,
            _ => break,
        }
    }
    // What was read before the writes reads the same after them.
    for (name, bytes) in &before {
        if let Some(b) = bytes {
            assert_eq!(v.read(name, 1 << 20).as_ref().ok(), Some(b));
        }
    }
    if let Some((name, _)) = before.first() {
        let _ = v.wipe(name);
    }
    let _ = v.make_empty();
});
