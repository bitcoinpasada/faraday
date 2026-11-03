//! The text inside an age vault: a file vault's TOML document, the QR
//! vault's top-level keys on up to two lines, and which of the two a
//! plaintext is. Whatever the bytes, each read gives a document or a
//! refusal; a document read is written again and read back as the same
//! number of records and seals, and a QR vault read is written again as
//! the same item.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_vault::Record;
use faraday_vault::contents::{Size, form, read, read_qr, write, write_qr};
use libfuzzer_sys::fuzz_target;

fn fields(r: &Record) -> Vec<(u8, Vec<u8>)> {
    r.fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    let _ = form(data);
    if let Ok(item) = read_qr(&c, data) {
        let text = write_qr(&item).expect("a QR vault read is written");
        let back = read_qr(&c, &text).expect("a QR vault written reads back");
        assert_eq!(back.kind, item.kind);
        assert_eq!(fields(&back), fields(&item));
    }
    if let Ok(doc) = read(&c, data) {
        if let Ok(text) = write(&doc, Size::Mib4) {
            let back = read(&c, &text).expect("a document written reads back");
            assert_eq!(back.contents.records.len(), doc.contents.records.len());
            assert_eq!(back.seals.count, doc.seals.count);
            assert_eq!(back.seals.last, doc.seals.last);
        }
    }
});
