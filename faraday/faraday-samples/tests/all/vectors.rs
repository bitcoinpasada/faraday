//! The samples that come from published vectors equal those vectors, read
//! from `tools/vectors` when the test runs, and the fingerprints they
//! state are the ones another implementation gives.

use crate::common::{
    PASSPHRASE, entropy_of, fingerprint_of_seed, hex, json, key, reference_fingerprint, unhex,
    vector, xprv_of_seed,
};
use faraday_bip::slip39::Share;
use faraday_core::testkit::compute;
use faraday_intake::{Part, part_of};
use unicode_normalization::UnicodeNormalization;

#[test]
fn abandon_is_the_first_bip39_vector_and_its_seed_is_the_published_one() {
    let sample = key("sample_abandon");
    let case = &json("bip39/vectors.json")["english"][0];
    assert_eq!(sample.parts[0], case[1].as_str().expect("words"));
    // The vectors' own passphrase, `TREZOR`, gives the published seed.
    let m = bip39::Mnemonic::parse_in_normalized(bip39::Language::English, sample.parts[0])
        .expect("words");
    assert_eq!(entropy_of(&m), unhex(case[0].as_str().expect("entropy")));
    let seed = m.to_seed_normalized("TREZOR");
    assert_eq!(hex(&seed), case[2].as_str().expect("seed"));
    assert_eq!(xprv_of_seed(&seed), case[3].as_str().expect("xprv"));
    // The sample's fingerprints are those of the same words under none
    // and under `faraday`.
    let lang = bip39::Language::English;
    assert_eq!(
        sample.plain,
        Some(reference_fingerprint(lang, sample.parts[0], ""))
    );
    assert_eq!(
        sample.with_passphrase,
        Some(reference_fingerprint(lang, sample.parts[0], PASSPHRASE))
    );
}

#[test]
fn the_japanese_sample_is_the_first_bip32jp_vector() {
    let sample = key("sample_japanese");
    let case = &json("bip39/test_JP_BIP39.json")[0];
    let published = case["mnemonic"].as_str().expect("words");
    // Faraday reads a Japanese word in its published or its NFKD form; the
    // sample is the vector's words in the second, one space between them.
    let nfkd: String = published.nfkd().collect();
    assert_eq!(sample.parts[0], nfkd);
    let lang = bip39::Language::Japanese;
    let m = bip39::Mnemonic::parse_in_normalized(lang, sample.parts[0]).expect("words");
    assert_eq!(
        entropy_of(&m),
        unhex(case["entropy"].as_str().expect("entropy"))
    );
    let passphrase: String = case["passphrase"]
        .as_str()
        .expect("passphrase")
        .nfkd()
        .collect();
    let seed = m.to_seed_normalized(&passphrase);
    assert_eq!(hex(&seed), case["seed"].as_str().expect("seed"));
    assert_eq!(
        xprv_of_seed(&seed),
        case["bip32_xprv"].as_str().expect("xprv")
    );
    assert_eq!(
        sample.plain,
        Some(reference_fingerprint(lang, sample.parts[0], ""))
    );
    assert_eq!(
        sample.with_passphrase,
        Some(reference_fingerprint(lang, sample.parts[0], PASSPHRASE))
    );
}

/// The case of `slip39/vectors.json` whose description begins `number.`.
fn slip39_case(number: usize) -> serde_json::Value {
    let v = json("slip39/vectors.json");
    let case = &v[number - 1];
    assert!(
        case[0]
            .as_str()
            .expect("description")
            .starts_with(&format!("{number}. ")),
        "{}",
        case[0]
    );
    case.clone()
}

fn slip39_sample_is_case(name: &str, number: usize) {
    let sample = key(name);
    let case = slip39_case(number);
    let published: Vec<&str> = case[1]
        .as_array()
        .expect("mnemonics")
        .iter()
        .map(|m| m.as_str().expect("text"))
        .collect();
    assert_eq!(sample.parts, published.as_slice(), "{name}");
    let shares: Vec<Share> = sample
        .parts
        .iter()
        .map(|p| match part_of(p) {
            Some(Part::Slip39(s)) => s,
            _ => panic!("a SLIP-39 share"),
        })
        .collect();
    let c = compute();
    // Under the vectors' passphrase the shares give the published master
    // secret and its root key.
    let master = faraday_keys::slip39_seed(&c, &shares, b"TREZOR").expect("recovers");
    assert_eq!(hex(&master), case[2].as_str().expect("master secret"));
    assert_eq!(xprv_of_seed(&master), case[3].as_str().expect("xprv"));
    // And the sample's fingerprints are those of the secret under none and
    // under `faraday`.
    for (pass, want) in [("", sample.plain), (PASSPHRASE, sample.with_passphrase)] {
        let secret = faraday_keys::slip39_seed(&c, &shares, pass.as_bytes()).expect("recovers");
        assert_eq!(
            Some(fingerprint_of_seed(&secret)),
            want,
            "{name} under {pass:?}"
        );
    }
}

#[test]
fn the_slip39_samples_are_vectors_1_4_and_37() {
    slip39_sample_is_case("sample_slip39_one", 1);
    slip39_sample_is_case("sample_slip39_two_of_three", 4);
    slip39_sample_is_case("sample_slip39_groups", 37);
}

#[test]
fn the_codex32_samples_are_bip93_vectors_1_and_2_with_their_seed_and_xprv() {
    let v = json("bip93/vectors.json");
    let secret = &v["secrets"][0];
    assert_eq!(secret["vector"], "1");
    let set = &v["sets"][0];
    assert_eq!(set["vector"], "2");
    for (name, strings, seed, xprv) in [
        (
            "sample_codex32_secret",
            vec![secret["string"].as_str().expect("string")],
            secret["seed"].as_str().expect("seed"),
            secret["xprv"].as_str().expect("xprv"),
        ),
        (
            "sample_codex32_shares",
            set["shares"]
                .as_array()
                .expect("shares")
                .iter()
                .map(|s| s.as_str().expect("text"))
                .collect(),
            set["seed"].as_str().expect("seed"),
            set["xprv"].as_str().expect("xprv"),
        ),
    ] {
        let sample = key(name);
        assert_eq!(sample.parts, strings.as_slice(), "{name}");
        let parts: Vec<_> = sample
            .parts
            .iter()
            .map(|p| match part_of(p) {
                Some(Part::Codex32(s)) => s,
                _ => panic!("a codex32 string"),
            })
            .collect();
        let got = faraday_keys::codex32_seed(&parts).expect("a seed");
        assert_eq!(hex(&got), seed, "{name}");
        assert_eq!(xprv_of_seed(&got), xprv, "{name}");
        assert_eq!(sample.plain, Some(fingerprint_of_seed(&got)), "{name}");
        assert_eq!(sample.with_passphrase, None);
    }
}

#[test]
fn the_seed_xor_sample_is_the_two_part_file_and_its_parts_xor_to_the_key() {
    let sample = key("sample_xor");
    let text = vector("xor/12-words-2-parts.txt");
    let line = |tag: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(tag))
            .expect("a line")
            .trim()
            .to_string()
    };
    assert_eq!(
        sample.parts,
        [line("part 1 words: "), line("part 2 words: ")]
    );
    assert_eq!(sample.combined, line("seed words: "));
    let lang = bip39::Language::English;
    let entropy = |words: &str| {
        entropy_of(&bip39::Mnemonic::parse_in_normalized(lang, words).expect("words"))
    };
    let (a, b) = (entropy(sample.parts[0]), entropy(sample.parts[1]));
    let xored: Vec<u8> = a.iter().zip(&b).map(|(x, y)| x ^ y).collect();
    assert_eq!(xored, entropy(sample.combined));
    assert_eq!(hex(&xored), line("seed entropy: "));
    assert_eq!(
        sample.plain,
        Some(reference_fingerprint(lang, sample.combined, ""))
    );
    assert_eq!(
        sample.with_passphrase,
        Some(reference_fingerprint(lang, sample.combined, PASSPHRASE))
    );
}

#[test]
fn the_three_of_five_keys_are_the_master_seeds_of_the_backup_shares_test_wallet() {
    let fives: Vec<_> = faraday_samples::SAMPLES
        .keys
        .iter()
        .filter(|k| k.name == "sample_three_of_five")
        .collect();
    assert_eq!(fives.len(), 5);
    for (b, k) in (1u8..=5).zip(&fives) {
        assert_eq!(k.parts, [hex(&[b; 64])]);
        assert_eq!(k.plain, Some(fingerprint_of_seed(&[b; 64])));
        assert!(!k.listed, "the sheet lists only the wallet");
    }
}
