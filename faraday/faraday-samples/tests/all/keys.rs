//! Practice takes every sample the way a person would load it, and nothing
//! else: the public set is complete and no other key is in it.

use crate::common::{PASSPHRASE, hex, unhex};
use faraday_bip::bip39::Language;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_core::samples::{SampleForm, SampleKey, Samples};
use faraday_core::testkit::compute;
use faraday_intake::{Part, part_of};
use faraday_keys::Keys;
use faraday_samples::SAMPLES;

fn demo_keys() -> Keys {
    let mut keys = Keys::on(Network::Testnet);
    keys.set_demo(&SAMPLES.handouts(), &SAMPLES.public(), &SAMPLES.nodes());
    keys
}

/// Loads `sample` into `keys` through the path its form takes, with
/// `passphrase` where the form takes one.
fn load(keys: &mut Keys, sample: &SampleKey, passphrase: &str) -> Result<Fingerprint, String> {
    let c = compute();
    let refused = |e: faraday_intake::wallet::Refusal| format!("{e:?}");
    match sample.form {
        SampleForm::Words => keys
            .add_words_with(&c, sample.parts[0], passphrase, "", None)
            .map_err(refused),
        SampleForm::Xor => keys
            .add_words_with(&c, sample.combined, passphrase, "", None)
            .map_err(refused),
        SampleForm::Slip39 => {
            let shares: Vec<_> = sample
                .parts
                .iter()
                .map(|p| match part_of(p) {
                    Some(Part::Slip39(s)) => s,
                    _ => panic!("a SLIP-39 share"),
                })
                .collect();
            let seed = faraday_keys::slip39_seed(&c, &shares, passphrase.as_bytes())
                .map_err(|e| format!("{e:?}"))?;
            keys.add_seed(&c, &seed, "").map_err(refused)
        }
        SampleForm::Codex32 => {
            let parts: Vec<_> = sample
                .parts
                .iter()
                .map(|p| match part_of(p) {
                    Some(Part::Codex32(s)) => s,
                    _ => panic!("a codex32 string"),
                })
                .collect();
            let seed = faraday_keys::codex32_seed(&parts).map_err(|e| format!("{e:?}"))?;
            keys.add_seed(&c, &seed, "").map_err(refused)
        }
        SampleForm::Seed => keys
            .add_seed(&c, &unhex(sample.parts[0]), "")
            .map_err(refused),
        SampleForm::Aezeed => Err("loads no key".to_string()),
    }
}

#[test]
fn demo_takes_every_sample_key_in_the_form_it_comes_in() {
    for sample in SAMPLES.keys.iter().filter(|k| k.plain.is_some()) {
        let mut keys = demo_keys();
        let fp = load(&mut keys, sample, "").unwrap_or_else(|e| panic!("{}: {e}", sample.name));
        assert_eq!(
            Some(fp.0),
            sample.plain,
            "{}: {:?}",
            sample.name,
            sample.parts
        );
    }
}

#[test]
fn demo_takes_the_keys_that_take_a_passphrase_with_it_too() {
    let mut with = 0;
    for sample in SAMPLES.keys.iter().filter(|k| k.with_passphrase.is_some()) {
        let mut keys = demo_keys();
        let fp = load(&mut keys, sample, PASSPHRASE)
            .unwrap_or_else(|e| panic!("{} with the passphrase: {e}", sample.name));
        assert_eq!(Some(fp.0), sample.with_passphrase, "{}", sample.name);
        with += 1;
    }
    // The guide's key, the 21 repeated words, the 12-word vector, the
    // Japanese one, the three SLIP-39 samples and Seed XOR.
    assert_eq!(with, 1 + 21 + 2 + 3 + 1);
    // And the forms that take none have none.
    for sample in SAMPLES.keys.iter() {
        let takes = sample.form.takes_passphrase();
        assert_eq!(sample.with_passphrase.is_some(), takes, "{}", sample.name);
    }
}

#[test]
fn demo_refuses_a_key_that_is_not_a_sample_and_a_sample_with_another_passphrase() {
    let c = compute();
    let mut keys = demo_keys();
    // The BIP-39 all-zero key with a passphrase no sample is under.
    let abandon = SAMPLES
        .keys
        .iter()
        .find(|k| k.name == "sample_abandon")
        .expect("abandon");
    assert!(
        keys.add_words_with(&c, abandon.parts[0], "TREZOR", "", None)
            .is_err()
    );
    // Nor under `password`: the passphrase is `faraday`.
    assert!(
        keys.add_words_with(&c, abandon.parts[0], "password", "", None)
            .is_err()
    );
    assert!(
        keys.add_words_with(&c, abandon.parts[0], "", "", None)
            .is_ok()
    );
    assert!(
        keys.add_words_with(&c, abandon.parts[0], "faraday", "", None)
            .is_ok()
    );
    // Words that are valid and are in no sample.
    let other = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
    assert!(keys.add_words(&c, other, "", None).is_err());
    assert!(keys.add_seed(&c, &[7u8; 32], "").is_err());
}

#[test]
fn every_fingerprint_of_the_public_set_belongs_to_a_sample_and_none_repeats() {
    let public = SAMPLES.public();
    let mut seen = std::collections::HashSet::new();
    for fp in &public {
        assert!(seen.insert(fp.0), "{} twice", hex(&fp.0));
    }
    let mut want = 0;
    for k in SAMPLES.keys {
        want += usize::from(k.plain.is_some()) + usize::from(k.with_passphrase.is_some());
    }
    assert_eq!(public.len(), want);
    let keys = demo_keys();
    for fp in &public {
        assert!(keys.is_public(*fp));
    }
}

#[test]
fn the_lnd_cipher_seed_is_the_one_public_node_and_another_is_refused() {
    let c = compute();
    let sample = crate::common::key("sample_aezeed");
    let words: Vec<&str> = sample.parts[0].split_whitespace().collect();
    assert_eq!(words.len(), faraday_keys::lightning::AEZEED_WORDS);
    let mut idx = [0u16; faraday_keys::lightning::AEZEED_WORDS];
    for (slot, w) in idx.iter_mut().zip(&words) {
        *slot = Language::English.index_of(w).expect("an English word");
    }
    let node = demo_keys()
        .node_from_aezeed(&c, &idx, b"")
        .expect("the public node");
    assert_eq!(Some(*node.public()), sample.node);
    // The same cipher seed is not public where the set lists no node.
    let mut none = Keys::on(Network::Testnet);
    none.set_demo(&SAMPLES.handouts(), &SAMPLES.public(), &[]);
    assert!(none.node_from_aezeed(&c, &idx, b"").is_err());
    // Without Practice, as on the stick, any cipher seed reads.
    assert!(
        Keys::on(Network::Testnet)
            .node_from_aezeed(&c, &idx, b"")
            .is_ok()
    );
    // A node id one byte off is not public.
    let mut id = *node.public();
    id[5] ^= 1;
    assert!(demo_keys().is_public_node(node.public()));
    assert!(!demo_keys().is_public_node(&id));
}

#[test]
fn a_sample_carries_what_a_flow_needs_to_load_it() {
    for k in SAMPLES.keys {
        match k.form {
            SampleForm::Xor => assert!(!k.combined.is_empty() && k.parts.len() == 2),
            _ => assert!(!k.parts.is_empty(), "{}", k.name),
        }
        assert_eq!(k.node.is_some(), k.form == SampleForm::Aezeed);
        assert_eq!(k.plain.is_none(), k.form == SampleForm::Aezeed);
        assert!(k.name.starts_with("sample_"));
    }
    let listed = SAMPLES.keys.iter().filter(|k| k.listed).count();
    // The guide's key, 21 repeated words, abandon, Japanese, three
    // SLIP-39, two codex32, Seed XOR and the cipher seed.
    assert_eq!(listed, 1 + 21 + 2 + 3 + 2 + 1 + 1);
    let _: &Samples = &SAMPLES;
}

#[test]
fn the_size_a_key_sample_records_is_what_its_data_holds() {
    use faraday_core::samples::SampleSize as S;
    let words = |t: &str| u8::try_from(t.split_whitespace().count()).expect("a count");
    for k in SAMPLES.keys {
        let parts = u8::try_from(k.parts.len()).expect("a count");
        match k.size {
            S::Words(n) | S::JapaneseWords(n) => assert_eq!(n, words(k.parts[0]), "{}", k.name),
            S::Shares(n) | S::GroupShares(n) | S::Parts(n) => assert_eq!(n, parts, "{}", k.name),
            S::Secret => assert_eq!(parts, 1, "{}", k.name),
            S::Bytes(n) => assert_eq!(usize::from(n), unhex(k.parts[0]).len(), "{}", k.name),
            S::Threshold { need, total } => {
                // A share states how many shares recover its key; the set
                // it came from is as large as the vector says.
                let share = match part_of(k.parts[0]) {
                    Some(Part::Slip39(s)) => s,
                    _ => panic!("a SLIP-39 share"),
                };
                assert_eq!(need, share.member_threshold(), "{}", k.name);
                assert!(total >= need && parts >= need, "{}", k.name);
            }
        }
    }
    // A Seed XOR sample is as long as the key its parts make.
    let xor = crate::common::key("sample_xor");
    assert_eq!(words(xor.combined), 12);
}

#[test]
fn only_the_japanese_sample_is_in_the_japanese_list() {
    use faraday_core::samples::SampleSize as S;
    for k in SAMPLES.keys {
        assert_eq!(
            matches!(k.size, S::JapaneseWords(_)),
            k.name == "sample_japanese",
            "{}",
            k.name
        );
    }
}
