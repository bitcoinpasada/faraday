//! A vault file's header, read from any file Files lists: `read_header`
//! runs first in the classifier, on every file, with no key. The cost it
//! states (Argon2 memory, passes, lanes) and the slot length are the
//! file's word.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(header) = faraday_vault::read_header(data) {
        let _ = header.cost;
        let _ = header.slot_len;
    }
});
