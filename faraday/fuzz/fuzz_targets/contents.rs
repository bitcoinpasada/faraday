//! The text inside an age vault: a file vault's TOML document, the QR
//! vault's one line, and which of the two a plaintext is. Whatever the
//! bytes, each read gives a document or a refusal; a document read is
//! written again and read back as the same number of records.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_vault::contents::{Size, form, read, read_qr, write};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    let _ = form(data);
    let _ = read_qr(&c, data);
    if let Ok(doc) = read(&c, data) {
        if let Ok(text) = write(&doc, Size::Mib4) {
            let back = read(&c, &text).expect("a document written reads back");
            assert_eq!(back.contents.records.len(), doc.contents.records.len());
        }
    }
});
