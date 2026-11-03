//! The TOML inside a vault: the QR vault's keys on up to two lines, and
//! the three TOML values a file vault keeps in KDBX custom data (a backup
//! plan, a seed's codex32 facts, its BIP-85 facts). Whatever the bytes,
//! each read gives a value or a refusal; a value read is written again and
//! read back as the same fields.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_vault::Record;
use faraday_vault::contents::{
    read_child_facts, read_codex32_facts, read_plan, read_qr, write_child_facts,
    write_codex32_facts, write_plan, write_qr,
};
use libfuzzer_sys::fuzz_target;

fn fields(r: &Record) -> Vec<(u8, Vec<u8>)> {
    r.fields
        .iter()
        .map(|f| (f.number, f.bytes.to_vec()))
        .collect()
}

fuzz_target!(|data: &[u8]| {
    let c = Compute::unchecked();
    if let Ok(item) = read_qr(&c, data) {
        let text = write_qr(&item).expect("a QR vault read is written");
        let back = read_qr(&c, &text).expect("a QR vault written reads back");
        assert_eq!(back.kind, item.kind);
        assert_eq!(fields(&back), fields(&item));
    }
    if let Ok(plan) = read_plan(data) {
        let text = write_plan(&plan).expect("a plan read is written");
        let back = read_plan(&text).expect("a plan written reads back");
        assert_eq!(fields(&back), fields(&plan));
    }
    if let Ok(set) = read_codex32_facts(data) {
        let text = write_codex32_facts(&set).expect("codex32 facts read are written");
        let back = read_codex32_facts(&text).expect("codex32 facts written read back");
        assert_eq!(
            (back.threshold, back.identifier, back.length, back.unknown),
            (set.threshold, set.identifier, set.length, set.unknown)
        );
        assert_eq!(back.strings, set.strings);
    }
    if let Ok(child) = read_child_facts(data) {
        let text = write_child_facts(&child).expect("child facts read are written");
        let back = read_child_facts(&text).expect("child facts written read back");
        assert_eq!(
            (back.parent, back.language, back.words, back.index),
            (child.parent, child.language, child.words, child.index)
        );
    }
});
