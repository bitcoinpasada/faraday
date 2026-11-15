//! The table of published keys in `faraday-keys` holds every fingerprint
//! the published vectors and Faraday's samples give. The vectors' are
//! worked out by the `bip39` and `bitcoin` crates or stated by the vectors
//! themselves (SLIP-39 with no passphrase: pinned below, reproduced by
//! `python3 -B tools/crosscheck/slip39_plain_pins.py`), none by Faraday's code; the samples'
//! are the ones the samples hold, which the samples' own tests check
//! against their words. Every entry of the table is one of these.

#![expect(
    clippy::disallowed_types,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::{fingerprint_of_seed, json, reference_fingerprint, unhex};
use faraday_bip::bitcoin::bip32::Xpriv;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_keys::published::{is_published, table};
use faraday_samples::SAMPLES;
use std::str::FromStr;
use unicode_normalization::UnicodeNormalization;

fn language(name: &str) -> Option<bip39::Language> {
    Some(match name {
        "english" => bip39::Language::English,
        "chinese_simplified" => bip39::Language::SimplifiedChinese,
        "chinese_traditional" => bip39::Language::TraditionalChinese,
        "czech" => bip39::Language::Czech,
        "french" => bip39::Language::French,
        "italian" => bip39::Language::Italian,
        "japanese" => bip39::Language::Japanese,
        "korean" => bip39::Language::Korean,
        "portuguese" => bip39::Language::Portuguese,
        "spanish" => bip39::Language::Spanish,
        _ => return None,
    })
}

/// The fingerprint of a root key as a vector states it.
#[expect(
    clippy::disallowed_methods,
    reason = "another implementation's master key, to check Faraday's table against (docs/COMPUTE.md §7)"
)]
fn stated(xprv: &str) -> [u8; 4] {
    let root = Xpriv::from_str(xprv).expect("a root key");
    root.fingerprint(&Secp256k1::new()).to_bytes()
}

#[test]
fn every_bip39_vector_is_in_the_table_with_trezor_and_with_no_passphrase() {
    let all = json("bip39/vectors.json");
    let mut seen = 0;
    for (name, cases) in all.as_object().expect("languages") {
        let Some(lang) = language(name) else {
            continue;
        };
        for case in cases.as_array().expect("cases") {
            let words: String = case[1].as_str().expect("words").nfkd().collect();
            let seed = unhex(case[2].as_str().expect("seed"));
            let trezor = stated(case[3].as_str().expect("root key"));
            assert_eq!(trezor, fingerprint_of_seed(&seed), "{name}: {words}");
            assert_eq!(trezor, reference_fingerprint(lang, &words, "TREZOR"));
            assert!(is_published(trezor), "{name} with TREZOR: {words}");
            let plain = reference_fingerprint(lang, &words, "");
            assert!(is_published(plain), "{name} with none: {words}");
            seen += 1;
        }
    }
    assert_eq!(seen, 240, "ten languages of 24 vectors");
}

/// The master fingerprints of the recovering SLIP-39 vectors with no
/// passphrase, worked out by python-shamir-mnemonic and btclib, in the
/// vectors' order (vectors 1, 4, 17 to 20, 23, 36 to 38, 41 to 45).
/// `python3 -B tools/crosscheck/slip39_plain_pins.py`, run from the
/// repository root, prints these 15 lines from the vendored references
/// (`tools/reference/slip39`, btclib), and they match.
const SLIP39_PLAIN: [[u8; 4]; 15] = [
    [0xb9, 0xb6, 0xcb, 0xc8],
    [0x81, 0x33, 0xa6, 0xd5],
    [0xec, 0x27, 0x64, 0x92],
    [0xec, 0x27, 0x64, 0x92],
    [0xec, 0x27, 0x64, 0x92],
    [0x82, 0x76, 0xdc, 0xd1],
    [0x9c, 0x12, 0x13, 0x55],
    [0xf5, 0x92, 0x75, 0x3f],
    [0xf5, 0x92, 0x75, 0x3f],
    [0xf5, 0x92, 0x75, 0x3f],
    [0x4a, 0xbc, 0x99, 0xb3],
    [0x37, 0xbb, 0x5f, 0xa5],
    [0x78, 0x65, 0xd3, 0xb3],
    [0x30, 0x52, 0x47, 0x03],
    [0x5a, 0x4a, 0x07, 0x28],
];

#[test]
fn every_slip39_vector_that_recovers_is_in_the_table_with_trezor_and_with_no_passphrase() {
    let mut seen = 0;
    for case in json("slip39/vectors.json").as_array().expect("cases") {
        if case[2].as_str().expect("master secret").is_empty() {
            continue;
        }
        let fp = stated(case[3].as_str().expect("root key"));
        assert!(is_published(fp), "{} with TREZOR", case[0]);
        assert!(is_published(SLIP39_PLAIN[seen]), "{} with none", case[0]);
        seen += 1;
    }
    assert_eq!(seen, 15);
}

/// The fingerprint of the master key of an xpub a BIP-32 vector states.
fn stated_xpub(xpub: &str) -> [u8; 4] {
    faraday_bip::bitcoin::bip32::Xpub::from_str(xpub)
        .expect("an extended public key")
        .fingerprint()
        .to_bytes()
}

/// Every codex32 master seed BIP-93 publishes, as its vectors state it.
fn codex32_seeds() -> Vec<(String, Vec<u8>, Option<String>)> {
    let all = json("bip93/vectors.json");
    let mut out = Vec::new();
    for kind in ["secrets", "sets", "alternates"] {
        for v in all[kind].as_array().expect(kind) {
            out.push((
                format!("codex32 vector {}", v["vector"]),
                unhex(v["seed"].as_str().expect("seed")),
                v["xprv"].as_str().map(str::to_owned),
            ));
        }
    }
    out
}

#[test]
fn every_codex32_master_seed_is_in_the_table_as_the_standard_states_it() {
    let seeds = codex32_seeds();
    assert_eq!(seeds.len(), 10);
    for (name, seed, xprv) in seeds {
        let fp = fingerprint_of_seed(&seed);
        if let Some(xprv) = xprv {
            assert_eq!(fp, stated(&xprv), "{name}");
        }
        assert!(is_published(fp), "{name}");
    }
}

#[test]
fn every_bip32_vector_seed_is_in_the_table_as_the_standard_states_it() {
    let mut seen = 0;
    for v in json("bip32/vectors.json").as_array().expect("vectors") {
        let seed = unhex(v["seed"].as_str().expect("seed"));
        let master = &v["chains"][0];
        assert_eq!(master["path"], "m");
        let fp = fingerprint_of_seed(&seed);
        assert_eq!(fp, stated_xpub(master["xpub"].as_str().expect("xpub")));
        assert!(is_published(fp), "{}", v["seed"]);
        seen += 1;
    }
    assert_eq!(seen, 4);
}

/// The fingerprints BIP-85's vectors give: the master key, the child words
/// of each length, the child xprv and the child hex as a seed.
fn bip85_fingerprints() -> Vec<[u8; 4]> {
    let all = json("bip85/vectors.json");
    let apps = &all["applications"];
    let mut out = vec![stated(all["master"].as_str().expect("master"))];
    for w in apps["bip39"].as_array().expect("words") {
        let words = w["mnemonic"].as_str().expect("mnemonic");
        out.push(reference_fingerprint(bip39::Language::English, words, ""));
    }
    for x in apps["xprv"].as_array().expect("xprv") {
        out.push(stated(x["xprv"].as_str().expect("xprv")));
    }
    for h in apps["hex"].as_array().expect("hex") {
        out.push(fingerprint_of_seed(&unhex(
            h["entropy"].as_str().expect("hex"),
        )));
    }
    out
}

#[test]
fn every_bip85_vector_key_is_in_the_table_as_the_standard_states_it() {
    let fps = bip85_fingerprints();
    assert_eq!(fps.len(), 1 + 3 + 1 + 1);
    for fp in fps {
        assert!(is_published(fp), "{fp:02x?}");
    }
}

#[test]
fn every_entry_of_the_table_is_a_vectors_or_a_samples() {
    let mut known: Vec<[u8; 4]> = SLIP39_PLAIN.to_vec();
    known.extend(bip85_fingerprints());
    known.extend(
        codex32_seeds()
            .iter()
            .map(|(_, s, _)| fingerprint_of_seed(s)),
    );
    for v in json("bip32/vectors.json").as_array().expect("vectors") {
        known.push(fingerprint_of_seed(&unhex(
            v["seed"].as_str().expect("seed"),
        )));
    }
    for (name, cases) in json("bip39/vectors.json").as_object().expect("languages") {
        let Some(lang) = language(name) else {
            continue;
        };
        for case in cases.as_array().expect("cases") {
            let words: String = case[1].as_str().expect("words").nfkd().collect();
            known.push(reference_fingerprint(lang, &words, "TREZOR"));
            known.push(reference_fingerprint(lang, &words, ""));
        }
    }
    for case in json("slip39/vectors.json").as_array().expect("cases") {
        if !case[2].as_str().expect("master secret").is_empty() {
            known.push(stated(case[3].as_str().expect("root key")));
        }
    }
    for sample in SAMPLES.keys {
        known.extend([sample.plain, sample.with_passphrase].into_iter().flatten());
    }
    for fp in table() {
        assert!(known.contains(fp), "{fp:02x?} is no vector's or sample's");
    }
}

#[test]
fn every_sample_key_is_in_the_table() {
    for sample in SAMPLES.keys {
        for fp in [sample.plain, sample.with_passphrase].into_iter().flatten() {
            assert!(is_published(fp), "{}", sample.name);
        }
    }
}

#[test]
fn a_fresh_key_is_not_in_the_table() {
    let seed = [0x42u8; 32];
    assert!(!is_published(fingerprint_of_seed(&seed)));
}
