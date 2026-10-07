//! `faraday-fat` over a partition's bytes: the boot sector, the FATs and
//! the root directory are all the stick's to choose (`PLAN.md` §4.4).
//! Every file listed is read, and a file is created, renamed and deleted,
//! so the write paths walk the same untrusted structures.

#![no_main]

use faraday_fat::Volume;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(mut v) = Volume::open(data.to_vec()) else {
        return;
    };
    let _ = v.label();
    let Ok(list) = v.list() else { return };
    for e in list.iter().take(64) {
        let _ = v.read(&e.name, 1 << 20);
    }
    if v.create("fuzz.txt", b"faraday").is_ok() {
        let _ = v.rename("fuzz.txt", "fuzzed.txt");
        let _ = v.delete("fuzzed.txt");
    }
});
