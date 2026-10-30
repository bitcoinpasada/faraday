//! `faraday_codec::ur::decode_single`: one `ur:` string, as a QR code
//! carries a single-part message.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = faraday_codec::ur::decode_single(text);
});
