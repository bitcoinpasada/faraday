//! `osk_codec::classify` over a scanned payload's bytes.
//!
//! Every QR the camera reads reaches this function before anything else
//! looks at it, so it is the widest untrusted-input surface in the core.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = osk_codec::classify(data);
});
