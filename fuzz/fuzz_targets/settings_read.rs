//! `opensigner_core::settings::read`: the settings file, which on a
//! Tier A card and on a desktop is writable by whoever holds the
//! storage.

#![no_main]

use libfuzzer_sys::fuzz_target;
use opensigner_core::settings;

fuzz_target!(|data: &[u8]| {
    let _ = settings::read(data);
});
