//! Every file that comes into Files: `wallet::classify_read` over any
//! bytes under a chosen name. It runs the vault header, backup, PDF,
//! KDBX, EFI image, APK, certificate, PSBT, GPG, signed message,
//! multisig share, wallet, key, words, entries, seed part and raw
//! transaction readers in order, on every file of a visited or booted
//! stick, with no press for the boot stick's.
//!
//! The first byte picks the name's extension. Run it with
//! `-max_len=1048576 -rss_limit_mb=2048 -timeout=10` so a size that
//! multiplies shows (`just fuzz-release` does).

#![no_main]

use faraday_core::wallet::classify_read;
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

const NAMES: [&str; 8] = [
    "a.txt", "a.psbt", "a.efi", "a.fvlt", "a.json", "a.png", "a.apk",
];

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let c = Compute::unchecked();
    let _ = classify_read(&c, NAMES[usize::from(first) % NAMES.len()], rest);
});
