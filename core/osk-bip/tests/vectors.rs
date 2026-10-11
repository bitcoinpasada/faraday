//! Published BIP-39 test vectors, run through entropy → mnemonic,
//! mnemonic → entropy, and mnemonic + passphrase → seed.
//!
//! - `tools/vectors/bip39/vectors.json`: Trezor's python-mnemonic vectors,
//!   passphrase "TREZOR", for every language we support (the file also has
//!   Russian and Turkish, which BIP-39 does not list; they are skipped).
//! - `tools/vectors/bip39/test_JP_BIP39.json`: the bip32JP Japanese
//!   vectors. Their words are written in NFC (precomposed dakuten) while the
//!   published wordlist is NFKD, so sentences are NFKD-normalized before
//!   parsing; the passphrase is not ASCII and so goes through the test-only
//!   `to_seed_unchecked` in NFKD form.

mod common;

use common::{hex, sentence, unhex};
use osk_bip::bip39::{Language, Mnemonic};
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

const TREZOR: &str = include_str!("../../../tools/vectors/bip39/vectors.json");
const JAPANESE: &str = include_str!("../../../tools/vectors/bip39/test_JP_BIP39.json");

/// The bip32JP passphrase as it appears in the file, and its NFKD form.
/// NFKD replaces ㍍ with メートル and splits every voiced kana into base
/// kana plus combining dakuten/handakuten (U+3099 / U+309A).
const JP_PASSPHRASE: &str = "㍍ガバヴァぱばぐゞちぢ十人十色";
const JP_PASSPHRASE_NFKD_HEX: &str = "e383a1e383bce38388e383abe382abe38299e3838fe38299e382a6e38299e382a1\
    e381afe3829ae381afe38299e3818fe38299e3829de38299e381a1e381a1e38299e58d81e4babae58d81e889b2";

fn nfkd(s: &str) -> String {
    s.nfkd().collect()
}

fn check_vector(
    lang: Language,
    entropy_hex: &str,
    mnemonic: &str,
    seed_hex: &str,
    passphrase_nfkd: &[u8],
) {
    let entropy = unhex(entropy_hex);

    let from_entropy = Mnemonic::from_entropy(lang, &entropy).unwrap();
    let ours = sentence(lang, from_entropy.indices());
    // The vector files differ in composition form and separator; both
    // collapse under NFKD (which also maps U+3000 to U+0020).
    assert_eq!(
        nfkd(&ours),
        nfkd(mnemonic),
        "{}: entropy → mnemonic",
        lang.name()
    );
    if lang == Language::Japanese {
        assert!(ours.contains('\u{3000}'));
    } else {
        assert_eq!(ours, mnemonic);
    }

    let parsed = Mnemonic::parse(lang, &nfkd(mnemonic)).unwrap();
    assert_eq!(parsed.indices(), from_entropy.indices());
    let parsed_display = Mnemonic::parse(lang, &ours).unwrap();
    assert_eq!(parsed_display.indices(), from_entropy.indices());
    assert_eq!(parsed.word_count(), entropy.len() * 3 / 4);
    assert_eq!(parsed.language(), lang);
    assert_eq!(
        hex(parsed.entropy().expose().as_bytes()),
        entropy_hex,
        "{}: mnemonic → entropy",
        lang.name()
    );

    let seed = parsed.to_seed_unchecked(passphrase_nfkd).unwrap();
    assert_eq!(hex(seed.expose()), seed_hex, "{}: seed", lang.name());
}

#[test]
fn trezor_vectors_all_languages() {
    let json: Value = serde_json::from_str(TREZOR).unwrap();
    let mut count = 0;
    for lang in Language::ALL {
        let vectors = json[lang.name()]
            .as_array()
            .unwrap_or_else(|| panic!("no vectors for {}", lang.name()));
        assert!(!vectors.is_empty());
        for v in vectors {
            let v = v.as_array().unwrap();
            check_vector(
                lang,
                v[0].as_str().unwrap(),
                v[1].as_str().unwrap(),
                v[2].as_str().unwrap(),
                b"TREZOR",
            );
            count += 1;
        }
    }
    assert_eq!(count, 24 * Language::ALL.len());
}

#[test]
fn trezor_english_seed_through_public_api() {
    let json: Value = serde_json::from_str(TREZOR).unwrap();
    for v in json["english"].as_array().unwrap() {
        let m = Mnemonic::parse(Language::English, v[1].as_str().unwrap()).unwrap();
        assert_eq!(
            hex(m.to_seed(b"TREZOR").unwrap().expose()),
            v[2].as_str().unwrap()
        );
    }
}

#[test]
fn japanese_vectors() {
    let json: Value = serde_json::from_str(JAPANESE).unwrap();
    let vectors = json.as_array().unwrap();
    assert_eq!(vectors.len(), 24);
    let passphrase_nfkd = unhex(JP_PASSPHRASE_NFKD_HEX);
    assert_eq!(nfkd(JP_PASSPHRASE).as_bytes(), passphrase_nfkd);
    for v in vectors {
        assert_eq!(v["passphrase"].as_str().unwrap(), JP_PASSPHRASE);
        let mnemonic = v["mnemonic"].as_str().unwrap();
        assert!(
            mnemonic.contains('\u{3000}'),
            "JP vectors use the ideographic space"
        );
        check_vector(
            Language::Japanese,
            v["entropy"].as_str().unwrap(),
            mnemonic,
            v["seed"].as_str().unwrap(),
            &passphrase_nfkd,
        );
    }
}

#[test]
fn japanese_passphrase_is_rejected_by_public_api() {
    let json: Value = serde_json::from_str(JAPANESE).unwrap();
    let m = Mnemonic::parse(
        Language::Japanese,
        &nfkd(json[0]["mnemonic"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(
        m.to_seed(JP_PASSPHRASE.as_bytes()).err(),
        Some(osk_bip::bip39::Error::PassphraseNotAscii)
    );
}
