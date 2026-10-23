//! What Files works out from several files together: the input is split
//! into files at each 0x1e byte, they come into the Inbox as a stick's
//! files do (each classified), `inbox_found_now` puts them together
//! (multisig shares merged, SLIP-39 and codex32 groups recovered and
//! given a fingerprint, cosigner keys read), and Load then loads what
//! was found. A boot import draws the same count every frame with no
//! press.
//!
//! A SLIP-39 share can ask for far more PBKDF2 rounds than a run has
//! time for, as the standard allows; that cost is the product's to
//! bound, and the target drops files that name an exponent above 0.

#![no_main]

use faraday_bip::slip39::Share;
use faraday_core::testkit;
use faraday_core::{Action, StorageEvent};
use faraday_crypto::work::Compute;
use libfuzzer_sys::fuzz_target;

/// Files taken from one input.
const FILES: usize = 16;

/// Whether a file holds a SLIP-39 share that asks for more than the
/// base PBKDF2 rounds.
fn asks_for_rounds(part: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(part) else {
        return false;
    };
    text.lines()
        .any(|l| Share::parse(l).is_ok_and(|s| s.iteration_exponent() > 0))
}

fuzz_target!(|data: &[u8]| {
    let files: Vec<(String, Vec<u8>)> = data
        .split(|&b| b == 0x1e)
        .take(FILES)
        .filter(|part| !asks_for_rounds(part))
        .enumerate()
        .map(|(i, part)| (format!("f{i}.txt"), part.to_vec()))
        .collect();
    let c = Compute::unchecked();
    let mut app = testkit::started();
    let _ = testkit::move_to(&mut app.session, testkit::NET);
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.inbox_found_now(&c);
    app.press(Action::InboxLoad);
});
