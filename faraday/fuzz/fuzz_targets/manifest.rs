//! The Starter's payload `MANIFEST`: `Manifest::parse` over any bytes.
//! It is built into the binary and unpacked from a payload, so the input
//! is the build's own; it must still refuse rather than panic.

#![no_main]

use faraday_payload::Manifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = Manifest::parse(data);
});
