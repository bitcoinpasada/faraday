//! `osk_codec::ur::Decoder`: the animated-QR receiver, which is fed one
//! untrusted part per camera frame and keeps state across them.
//!
//! The input is offered twice: as a single part, and split on newlines
//! into a sequence of parts against one decoder, which is the shape a
//! hostile animation takes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use osk_codec::ur::{self, Decoder};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = core::str::from_utf8(data) else {
        return;
    };
    let _ = ur::is_ur(text);

    let mut one = Decoder::new();
    let _ = one.receive(text);
    let _ = one.progress();
    let _ = one.ur_type();
    let _ = one.into_message();

    let mut many = Decoder::new();
    for part in text.split('\n') {
        if matches!(many.receive(part), Ok(true)) {
            break;
        }
    }
    let _ = many.progress();
    let _ = many.into_message();
});
