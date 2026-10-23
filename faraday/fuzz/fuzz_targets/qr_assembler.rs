//! Scanned parts put together: the input's lines are fed one by one to
//! one `faraday_qr::Assembler` (BBQr with its base32, hex and inflate;
//! SeedSigner's numbered parts), and a finished transfer is kept as the
//! Inbox keeps it (`faraday_intake::arrival`). The input as a whole goes
//! to `classify_text` and the transaction-in-hex reader, and as bytes to
//! the Faraday file envelope.

#![no_main]

use faraday_qr::{Assembler, Step, classify_text, envelope, transaction_hex};
use libfuzzer_sys::fuzz_target;

/// Lines fed from one input.
const LINES: usize = 1024;

fuzz_target!(|data: &[u8]| {
    let _ = envelope::looks_like(data);
    let _ = envelope::unpack(data);
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut assembler = Assembler::default();
    for line in text.lines().take(LINES) {
        if let Step::Done(arrived) = assembler.feed(line) {
            let _ = faraday_intake::arrival(arrived);
            assembler = Assembler::default();
        }
    }
    if let Step::Done(arrived) = Assembler::default().feed(text) {
        let _ = faraday_intake::arrival(arrived);
    }
    let _ = classify_text(text);
    let _ = transaction_hex(text);
});
