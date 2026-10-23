//! A key share on a stick or a camera: SLIP-39 `Share::parse` and codex32
//! `Codex32::parse` over every line, then `recover` over all that parsed.
//! Files and codes both reach these before a press (Files classifies
//! every file that comes in; a scanned code goes to the seed readers).
//!
//! A share can ask for up to 2^15 times 10,000 PBKDF2 rounds, which the
//! standard allows; the target recovers only sets whose exponent is 0 so
//! a run stays fast. Parsing runs on every line whatever it asks.

#![no_main]

use faraday_bip::codex32::Codex32;
use faraday_bip::slip39::{self, Share};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

/// Lines read per input.
const LINES: usize = 64;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let c = Compute::unchecked();
    let mut slip = Vec::new();
    let mut codex = Vec::new();
    for line in text.lines().take(LINES) {
        if let Ok(s) = Share::parse(line) {
            slip.push(s);
        }
        if let Ok(s) = Codex32::parse(line) {
            codex.push(s);
        }
    }
    if !slip.is_empty() && slip.iter().all(|s| s.iteration_exponent() == 0) {
        let _ = slip39::recover(&c, &slip, b"");
    }
    if !codex.is_empty() {
        let _ = Codex32::recover(&codex);
    }
    // The whole text as one share, words split on any whitespace.
    let _ = Share::parse(text);
});
