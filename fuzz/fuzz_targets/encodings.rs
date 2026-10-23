//! The Tools screen's encodings reader: `encodings::read` on text (hex,
//! base58check, bech32 and the rest) and `read_input` under each way of
//! reading it.

#![no_main]

use faraday_codec::encodings::{self, ReadAs};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let _ = encodings::read(text);
    let _ = encodings::from_hex(text);
    for as_ in [ReadAs::Auto, ReadAs::Text, ReadAs::Hex] {
        let _ = encodings::read_input(text, as_);
    }
});
