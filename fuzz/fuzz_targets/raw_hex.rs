//! `faraday_entropy::RawHex::parse`: hex digits typed as entropy.

#![no_main]

use faraday_entropy::RawHex;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = RawHex::parse(text);
});
