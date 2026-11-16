//! Wallet files classified: seed words in a JSON member or a comment
//! line beside a descriptor, and a descriptor holding an extended private
//! key (as `xprv` and as `zprv`), leave no freed copy of the words or the
//! key.

#![expect(
    clippy::disallowed_types,
    reason = "a test or example builds keys directly and is not shipped (docs/KEYS.md)"
)]

use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_bip::bitcoin::base58;
use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_core::testkit;
use faraday_core::wallet::{FileKind, classify};
use faraday_leak_scan::Form;
use std::str::FromStr;

const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";

/// The key as `zprv` text: the same payload under SLIP-132's version.
fn zprv() -> String {
    let mut raw = base58::decode_check(XPRV).expect("the vector's xprv");
    raw[..4].copy_from_slice(&[0x04, 0xb2, 0x43, 0x0c]);
    base58::encode_check(&raw)
}

fn key_forms(texts: &[&str]) -> Vec<Form> {
    let key = Xpriv::from_str(XPRV).expect("the vector's xprv");
    let mut forms: Vec<Form> = texts
        .iter()
        .map(|t| Form::new("planted key text", t.as_bytes().to_vec()))
        .collect();
    forms.push(Form::new("planted xprv", key.encode().to_vec()));
    forms.push(Form::new(
        "planted private key",
        key.private_key.secret_bytes().to_vec(),
    ));
    forms.push(Form::new(
        "planted chain code",
        key.chain_code.as_bytes().to_vec(),
    ));
    forms
}

fn descriptor() -> &'static str {
    include_str!("../vectors/interop/bluewallet/coldcardmk4-descriptor.txt").trim()
}

#[test]
fn seed_words_in_a_json_member_beside_a_descriptor_leave_no_copy() {
    let file = format!(
        r#"{{"descriptor": "{}", "note": "{PHRASE}"}}"#,
        descriptor()
    );
    let c = testkit::compute();
    let forms = seed_forms("");
    let armed = arm(&forms);
    let kind = classify(&c, "wallet.json", file.as_bytes());
    assert_eq!(kind, FileKind::SecretWallet);
    finish("seed in a wallet json member", armed, false);
}

#[test]
fn seed_words_in_a_comment_line_beside_a_descriptor_leave_no_copy() {
    let file = format!("# {PHRASE}\n{}\n", descriptor());
    let c = testkit::compute();
    let forms = seed_forms("");
    let armed = arm(&forms);
    let kind = classify(&c, "wallet.txt", file.as_bytes());
    assert_eq!(kind, FileKind::SecretWallet);
    finish("seed in a wallet comment line", armed, false);
}

fn key_in_descriptor(key: &str, name: &str) {
    let file = format!(r#"{{"descriptor": "wpkh({key}/84h/0h/0h/<0;1>/*)"}}"#);
    let c = testkit::compute();
    let forms = key_forms(&[XPRV, &zprv()]);
    let armed = arm(&forms);
    let kind = classify(&c, "wallet.json", file.as_bytes());
    assert_ne!(kind, FileKind::Wallet, "a private key is no plain wallet");
    finish(name, armed, false);
}

#[test]
fn an_xprv_in_a_wallet_files_descriptor_leaves_no_copy() {
    key_in_descriptor(XPRV, "xprv in a wallet descriptor");
}

#[test]
fn a_zprv_in_a_wallet_files_descriptor_leaves_no_copy() {
    key_in_descriptor(&zprv(), "zprv in a wallet descriptor");
}
