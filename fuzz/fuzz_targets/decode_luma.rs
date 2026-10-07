//! `osk_codec::decode_luma`: `rqrr` over a camera frame, which on a
//! Tier A device is whatever the sensor sees and on a phone is whatever
//! an attacker can hold in front of it.
//!
//! The first two bytes give the frame's width and height, each 1 to 256;
//! the rest is the frame, zero-padded or truncated to fit.

#![no_main]

use libfuzzer_sys::fuzz_target;

/// The largest frame this target builds, in pixels per side. A real
/// preview is larger, but the shapes `rqrr` walks are the same and a
/// bigger frame only makes each run slower.
const MAX: usize = 256;

fuzz_target!(|data: &[u8]| {
    let [w, h, rest @ ..] = data else {
        return;
    };
    let width = usize::from(*w) % MAX + 1;
    let height = usize::from(*h) % MAX + 1;
    let mut frame = vec![0u8; width * height];
    let n = rest.len().min(frame.len());
    frame[..n].copy_from_slice(&rest[..n]);
    let _ = osk_codec::decode_luma(width, height, &frame);
});
