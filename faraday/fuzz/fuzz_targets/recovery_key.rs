//! `faraday_starter::api::RecoveryKey::parse`: a BitLocker recovery key as
//! Windows writes it, for a drive. The input is split at its first
//! newline into the drive and the key.

#![no_main]

use faraday_starter::api::RecoveryKey;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let (drive, key) = text.split_once('\n').unwrap_or((text, text));
    let _ = RecoveryKey::parse(drive, key);
});
