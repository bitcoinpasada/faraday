//! `faraday_payload::PayloadFile::parse_line`: one file's line of the
//! Starter's payload `MANIFEST`.

#![no_main]

use faraday_payload::PayloadFile;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(line) = std::str::from_utf8(data) else {
        return;
    };
    let _ = PayloadFile::parse_line(line);
});
