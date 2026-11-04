//! A vault file's clear header, read from any file Files lists:
//! `file::inspect` runs first in the classifier, on every file, with no
//! key. The cost it states (Argon2 memory, passes, lanes) and the length
//! are the file's word.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(facts) = faraday_vault::file::inspect(data) {
        let _ = facts.header;
        let _ = facts.size;
    }
});
