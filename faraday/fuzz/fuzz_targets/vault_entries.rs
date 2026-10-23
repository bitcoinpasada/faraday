//! A vault entries file and a TOTP setup code: `entries::parse` over the
//! text (a file from a stick, or a scanned code for a vault entry), and
//! `totp::parse` over the text and over each of its lines (an `otpauth`
//! URI or a bare base32 key).

#![no_main]

use faraday_vault::{entries, totp};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = entries::looks_like_entries(text);
    let _ = entries::parse(text);
    let _ = totp::parse(text);
    for line in text.lines().take(256) {
        let _ = totp::parse(line);
    }
});
