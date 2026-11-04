//! A key share on a stick or a camera: SLIP-39 `Share::parse` and codex32
//! `Codex32::parse` over every line, then `recover` over all that parsed.
//! Files and codes both reach these before a press (Files classifies
//! every file that comes in; a scanned code goes to the seed readers).
//! Convert's SLIP-39 set actions take typed shares further: the set's
//! refusals, a share at an index and a new split of the EMS, in shapes and
//! under randomness the input gives.
//!
//! A share can ask for up to 2^15 times 10,000 PBKDF2 rounds, which the
//! standard allows; the target recovers only sets whose exponent is 0 so
//! a run stays fast. Parsing runs on every line whatever it asks.

#![no_main]

use faraday_bip::codex32::Codex32;
use faraday_bip::slip39::{self, GroupSpec, Share};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

/// Lines read per input.
const LINES: usize = 64;

/// Convert's set actions over the shares that parsed; no PBKDF2 runs in
/// any of them. `data` is not empty, since a share parsed from it.
fn set_actions(shares: &[Share], data: &[u8]) {
    let _ = slip39::check_gathered(shares);
    let one_group: Vec<Share> = shares
        .iter()
        .filter(|s| s.group_index() == shares[0].group_index())
        .cloned()
        .collect();
    let _ = slip39::derive_member(&one_group, data[0] & 31);
    let Ok(ems) = slip39::recover_ems(shares) else {
        return;
    };
    let byte = |at: usize| data[at % data.len()];
    let identifier = u16::from_le_bytes([byte(1), byte(2)]);
    let renamed = ems.with_identifier(identifier);
    let ems = renamed.as_ref().unwrap_or(&ems);
    let groups: Vec<GroupSpec> = (0..usize::from(byte(3) & 31))
        .map(|g| GroupSpec {
            threshold: byte(2 * g + 4) & 31,
            count: byte(2 * g + 5) & 31,
        })
        .collect();
    let total: usize = groups.iter().map(|g| usize::from(g.count)).sum();
    let mut out = Vec::new();
    out.resize_with(total, Share::default);
    let mut at = 0;
    let mut random = |bytes: &mut [u8]| {
        for b in bytes {
            *b = byte(at);
            at += 1;
        }
    };
    let _ = slip39::split_ems(ems, byte(4) & 31, &groups, &mut random, &mut out);
}

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
    if !slip.is_empty() {
        set_actions(&slip, data);
    }
    if !codex.is_empty() {
        let _ = Codex32::recover(&codex);
    }
    // The whole text as one share, words split on any whitespace.
    let _ = Share::parse(text);
});
