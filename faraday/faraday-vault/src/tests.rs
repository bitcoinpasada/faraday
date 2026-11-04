use alloc::vec;

use super::records::{Fault, Record, field, kind};
use super::*;

/// A token for the tests' key work (`docs/COMPUTE.md` §3), counting
/// nothing: faraday-core's `testkit::compute` is out of this crate's
/// reach. Cheap.
#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> faraday_crypto::work::Compute {
    faraday_crypto::work::Compute::new(|_| {})
}

fn abandon() -> faraday_bip::bip39::Mnemonic {
    faraday_bip::bip39::Mnemonic::parse(
        &compute(),
        faraday_bip::bip39::Language::English,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap()
}

/// Records of every type the rules list are accepted, and what is read
/// from them (words, the load-at-unlock flag, text) is what was put in.
#[test]
fn records_of_every_type_are_accepted_and_read_back() {
    let words = abandon();
    let sheet = crate::layouts::sheet_payload(b"wpkh(...)", b"Savings", b"").unwrap();
    let all = vec![
        Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Main"),
        Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(&words))
            .with(field::KEY_LABEL, b"Test")
            .with(field::KEY_FLAGS, &[records::NO_LOAD_AT_UNLOCK]),
        Record::new(kind::WALLET)
            .with(field::WALLET, b"wpkh(@0/**)")
            .with(field::WALLET_NAME, b"Spending")
            .with(field::WALLET_FLAGS, &[records::NO_LOAD_AT_UNLOCK]),
        Record::new(kind::NOTE).with(field::NOTE, "Recovery codes · ünïcode".as_bytes()),
        Record::new(kind::SHEET).with(field::SHEET, &sheet),
        Record::new(kind::ENTRY)
            .with(field::TITLE, b"Email")
            .with(field::TOTP, b"otpauth://totp/x?secret=AB"),
        Record::new(kind::GPG)
            .with(1, &[1; 32])
            .with(2, &[0; 4])
            .with(5, b"A <a@b>")
            .with(5, b"B <b@c>"),
        Record::new(kind::SECURE_BOOT)
            .with(1, &[2; 16])
            .with(3, &[0x30, 0]),
        Record::new(kind::ROUND)
            .with(1, &[2])
            .with(2, &[3; 32])
            .with(3, &[4; 4])
            .with(4, &[5; 10]),
        Record::new(kind::AMOUNTS)
            .with(1, &[6; 32])
            .with(2, &[0; 8])
            .with(2, &[1; 8]),
    ];
    for r in &all {
        assert_eq!(r.check(), Ok(()), "type {}", r.kind);
    }
    assert_eq!(records::words_of(&all[1]).unwrap().word_count(), 12);
    // A key and a wallet set not to load at unlock read back so; one
    // with no flags loads.
    assert!(!records::loads_at_unlock(&all[1]));
    assert!(!records::loads_at_unlock(&all[2]));
    let mut wallet = all[2].clone();
    records::set_loads_at_unlock(&mut wallet, true);
    assert!(wallet.field(field::WALLET_FLAGS).is_none());
    assert!(records::loads_at_unlock(&wallet));
    assert_eq!(all[3].text(field::NOTE), Some("Recovery codes · ünïcode"));
    assert_eq!(all[6].fields.iter().filter(|f| f.number == 5).count(), 2);
}

/// A type no build knows is refused, the retired type 12 (an APK signing
/// key) among them, and so is a record without a required field.
#[test]
fn an_unknown_type_and_a_missing_field_are_refused() {
    assert_eq!(
        Record::new(42).with(1, &[1]).check(),
        Err(Fault::UnknownType(42))
    );
    assert_eq!(
        Record::new(12).with(1, &[1]).check(),
        Err(Fault::UnknownType(12))
    );
    assert_eq!(
        Record::new(kind::ENTRY)
            .with(field::USERNAME, b"me")
            .check(),
        Err(Fault::Missing(kind::ENTRY, 1))
    );
}

/// A wallet share (type 13) holds every field; one without its text is
/// refused (`docs/BACKUP-FLOW.md` §15.6).
#[test]
fn a_wallet_share_without_its_text_is_refused() {
    let whole = Record::new(kind::WALLET_SHARE)
        .with(field::SHARE, b"wsh(sortedmulti(2,A,B))")
        .with(field::SHARE_WALLET_NAME, b"Savings")
        .with(field::SHARE_NUMBER, &[1])
        .with(field::SHARE_OF, &[3])
        .with(field::SHARE_WALLET, b"wsh(sortedmulti(2,A,B,C))");
    assert_eq!(whole.check(), Ok(()));
    let no_text = Record::new(kind::WALLET_SHARE)
        .with(field::SHARE_NUMBER, &[1])
        .with(field::SHARE_OF, &[3]);
    assert_eq!(no_text.check(), Err(Fault::Missing(kind::WALLET_SHARE, 1)));
}

/// A passphrase kept on its own (type 14) holds the fingerprint of its
/// key and the passphrase; one without its passphrase, or with a
/// fingerprint of another length, is refused (`docs/BACKUP-FLOW.md`
/// §17.12 item 4).
#[test]
fn a_passphrase_record_without_its_passphrase_is_refused() {
    let whole = Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &[0x9a, 0x6a, 0x25, 0x80])
        .with(field::PASSPHRASE, b"TREZOR");
    assert_eq!(whole.check(), Ok(()));
    let none = Record::new(kind::PASSPHRASE).with(field::PASSPHRASE_KEY, &[1, 2, 3, 4]);
    assert_eq!(none.check(), Err(Fault::Missing(kind::PASSPHRASE, 2)));
    let short = Record::new(kind::PASSPHRASE)
        .with(field::PASSPHRASE_KEY, &[1, 2, 3])
        .with(field::PASSPHRASE, b"x");
    assert!(short.check().is_err());
}

/// A child seed record is accepted when it fits its key and refused when
/// it does not: 24 words recorded for a key of 12, another application,
/// an index past BIP-85's, a seed in place of words.
#[test]
fn a_child_seed_record_that_does_not_fit_its_key_is_refused() {
    use crate::child::{self, ChildOf};
    let words = abandon();
    let of = ChildOf {
        parent: [0x73, 0xc5, 0xda, 0x0a],
        language: 0,
        words: 12,
        index: 7,
    };
    let key = |bytes: &[u8]| {
        Record::new(kind::KEY)
            .with(field::KEY, &records::words_payload(&words))
            .with(field::KEY_CHILD, bytes)
    };
    let fits = key(&child::payload(&of).unwrap());
    assert_eq!(fits.check(), Ok(()));
    assert_eq!(
        fits.field(field::KEY_CHILD).and_then(child::child_of),
        Some(of)
    );
    let refused = |b: &[u8]| {
        assert_eq!(
            key(b).check(),
            Err(Fault::Value(kind::KEY, field::KEY_CHILD))
        );
    };
    refused(&child::payload(&ChildOf { words: 24, ..of }).unwrap());
    let mut other = child::payload(&of).unwrap();
    other[4] = 32;
    refused(&other);
    other = child::payload(&of).unwrap();
    other[10..].copy_from_slice(&0x8000_0000u32.to_le_bytes());
    refused(&other);
    let seed = Record::new(kind::KEY)
        .with(field::KEY, &crate::layouts::seed_payload(&[1; 32]).unwrap())
        .with(field::KEY_CHILD, &child::payload(&of).unwrap());
    assert!(seed.check().is_err());
}
