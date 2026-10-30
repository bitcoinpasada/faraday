//! The camera path as a whole: a `Faraday` with the scan sheet open for
//! each scan purpose in turn, the input's lines sent to it as
//! `Event::Scanned`, then the same text pasted as the desktop does. A
//! scanned code goes to the classifier, the UR and BBQr assemblers, the
//! seed readers (SeedQR, words, SLIP-39, codex32), the address note, a
//! vault entry's TOTP reader and Transfer's receiver, all of it before
//! any confirmation.
//!
//! The first byte picks the purpose.

#![no_main]

use faraday_core::testkit;
use faraday_core::{ScanPurpose, StorageEvent};
use faraday_shell_api::{App, Event};
use libfuzzer_sys::fuzz_target;

/// Lines sent from one input.
const LINES: usize = 64;

fn purpose(n: u8) -> ScanPurpose {
    match n % 10 {
        0 => ScanPurpose::Files,
        1 => ScanPurpose::Seed,
        2 => ScanPurpose::KeyPart,
        3 => ScanPurpose::VaultEntry,
        4 => ScanPurpose::CheckCopy(faraday_bip::keys::Fingerprint([0u8; 4])),
        5 => ScanPurpose::Transfer,
        6 => ScanPurpose::WalletKey(0),
        7 => ScanPurpose::Intake,
        8 => ScanPurpose::GpgPublic,
        _ => ScanPurpose::Files,
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let mut app = testkit::started();
    let _ = testkit::move_to(&mut app.session, testkit::NET);
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::open_scan(&mut app, purpose(first));
    for line in rest.split(|&b| b == b'\n').take(LINES) {
        app.event(Event::Scanned {
            bytes: line.to_vec(),
        });
    }
    if let Ok(text) = std::str::from_utf8(rest) {
        app.event(Event::Clipboard {
            kind: faraday_shell_api::FileKind::Text,
            text: text.to_string(),
        });
    }
});
