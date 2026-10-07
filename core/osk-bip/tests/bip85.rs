//! BIP-85's own test vectors, extracted from the BIP text into
//! `tools/vectors/bip85/vectors.json` by the script beside it: the
//! child words of application 39', the HD-seed WIF, the extended
//! private key, raw hex and the two passwords.
//!
//! Every vector is derived from the one master key the BIP publishes,
//! and each application's value is compared against the one the BIP
//! states.

mod common;

use common::{hex, sentence};
use osk_bip::bip39::Language;
use osk_bip::bip85;
use osk_bip::keys::MasterKey;
use serde_json::Value;

const VECTORS: &str = include_str!("../../../tools/vectors/bip85/vectors.json");

fn vectors() -> Value {
    serde_json::from_str(VECTORS).unwrap()
}

fn master() -> MasterKey {
    MasterKey::decode(vectors()["master"].as_str().unwrap()).expect("the BIP's master key")
}

/// The entries the BIP publishes for one application.
fn cases(app: &str) -> Vec<Value> {
    vectors()["applications"][app]
        .as_array()
        .unwrap()
        .to_owned()
}

fn index(case: &Value) -> u32 {
    case["index"].as_u64().unwrap() as u32
}

fn count(case: &Value, name: &str) -> usize {
    case[name].as_u64().unwrap() as usize
}

#[test]
fn the_published_children_of_application_39() {
    for case in cases("bip39") {
        let words = count(&case, "words");
        let m = bip85::child_mnemonic(&master(), Language::English, words, index(&case))
            .expect("a child mnemonic");
        assert_eq!(
            sentence(Language::English, m.indices()),
            case["mnemonic"].as_str().unwrap(),
            "{}",
            case["section"]
        );
    }
}

#[test]
fn each_index_is_a_different_child() {
    let child = |index| {
        let m = bip85::child_mnemonic(&master(), Language::English, 12, index).unwrap();
        sentence(Language::English, m.indices())
    };
    assert_ne!(child(1), child(0));
}

#[test]
fn a_word_count_bip39_does_not_define_is_refused() {
    for words in [0, 11, 13, 16, 25] {
        assert_eq!(
            bip85::child_mnemonic(&master(), Language::English, words, 0).err(),
            Some(bip85::Error::WordCount),
            "{words} words"
        );
    }
}

#[test]
fn the_published_hd_seed_wif() {
    for case in cases("wif") {
        let wif = bip85::child_wif(&master(), index(&case)).expect("a WIF");
        assert_eq!(wif.as_str(), case["wif"].as_str().unwrap());
    }
}

#[test]
fn the_published_extended_private_key() {
    for case in cases("xprv") {
        let xprv = bip85::child_xprv(&master(), index(&case)).expect("an xprv");
        assert_eq!(xprv.as_str(), case["xprv"].as_str().unwrap());
    }
}

#[test]
fn the_published_hex_entropy() {
    for case in cases("hex") {
        let bytes = count(&case, "bytes");
        let out = bip85::child_hex(&master(), bytes, index(&case)).expect("hex entropy");
        assert_eq!(hex(out.as_bytes()), case["entropy"].as_str().unwrap());
    }
}

#[test]
fn each_byte_count_is_a_child_of_its_own() {
    // The byte count is a step of the path, so asking for fewer bytes
    // is a different child rather than the same one cut short.
    let whole = bip85::child_hex(&master(), 64, 0).unwrap();
    for bytes in [16, 32, 63] {
        let short = bip85::child_hex(&master(), bytes, 0).unwrap();
        assert_eq!(short.as_bytes().len(), bytes);
        assert_ne!(
            short.as_bytes(),
            &whole.as_bytes()[..bytes],
            "{bytes} bytes"
        );
    }
}

#[test]
fn the_published_passwords() {
    for case in cases("base64") {
        let length = count(&case, "length");
        let pwd = bip85::child_password_base64(&master(), length, index(&case)).expect("base64");
        assert_eq!(pwd.as_str(), case["password"].as_str().unwrap());
    }
    for case in cases("base85") {
        let length = count(&case, "length");
        let pwd = bip85::child_password_base85(&master(), length, index(&case)).expect("base85");
        assert_eq!(pwd.as_str(), case["password"].as_str().unwrap());
    }
}

#[test]
fn a_password_is_as_long_as_it_was_asked_for() {
    // The length is a step of the path, so a longer password is another
    // password and not the same one with more of it.
    for length in [20, 21, 40, 86] {
        let pwd = bip85::child_password_base64(&master(), length, 0).unwrap();
        assert_eq!(pwd.as_str().chars().count(), length);
    }
    for length in [10, 12, 30, 80] {
        let pwd = bip85::child_password_base85(&master(), length, 0).unwrap();
        assert_eq!(pwd.as_str().chars().count(), length);
    }
    let a = bip85::child_password_base64(&master(), 21, 0).unwrap();
    let b = bip85::child_password_base64(&master(), 22, 0).unwrap();
    assert!(!b.as_str().starts_with(a.as_str()));
}

#[test]
fn a_parameter_outside_the_bips_range_is_refused() {
    for bytes in [0, 15, 65] {
        assert_eq!(
            bip85::child_hex(&master(), bytes, 0).err(),
            Some(bip85::Error::ByteCount),
            "{bytes} bytes"
        );
    }
    for length in [0, 19, 87] {
        assert_eq!(
            bip85::child_password_base64(&master(), length, 0).err(),
            Some(bip85::Error::PasswordLength),
            "base64 {length}"
        );
    }
    for length in [0, 9, 81] {
        assert_eq!(
            bip85::child_password_base85(&master(), length, 0).err(),
            Some(bip85::Error::PasswordLength),
            "base85 {length}"
        );
    }
}
