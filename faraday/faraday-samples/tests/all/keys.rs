//! Practice takes every sample the way a person would load it, and nothing
//! else: the public set is complete and no other key is in it.

use crate::common::{PASSPHRASE, hex, unhex};
use faraday_bip::bip39::Language;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_core::forms::Form;
use faraday_core::samples::{SampleForm, SampleKey};
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
        SampleForm::Form(Form::Words) => keys
            .add_words_with(&c, sample.parts[0], passphrase, "", None)
            .map_err(refused),
        SampleForm::Form(Form::Xor) => keys
            .add_words_with(&c, sample.combined, passphrase, "", None)
            .map_err(refused),
        SampleForm::Form(Form::Slip39) => {
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
        SampleForm::Form(Form::Codex32) => {
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
        SampleForm::Aezeed | SampleForm::Form(Form::Krux | Form::Xprv | Form::HexSeed) => {
            Err("loads no key".to_string())
        }
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
    for sample in SAMPLES.keys.iter().filter(|k| k.with_passphrase.is_some()) {
        let mut keys = demo_keys();
        let fp = load(&mut keys, sample, PASSPHRASE)
            .unwrap_or_else(|e| panic!("{} with the passphrase: {e}", sample.name));
        assert_eq!(Some(fp.0), sample.with_passphrase, "{}", sample.name);
    }
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

/// Every BIP-85 words child at index 0 that the BIP-85 screen offers for
/// every sample key, as its fingerprint with no passphrase: what a person
/// types bare. Sorted, without repeats.
fn index_zero_children() -> Vec<[u8; 4]> {
    use faraday_keys::bip85::App;
    let c = compute();
    let mut out = Vec::new();
    for sample in SAMPLES.keys {
        for (passphrase, held) in [("", sample.plain), (PASSPHRASE, sample.with_passphrase)] {
            if held.is_none() {
                continue;
            }
            let mut keys = Keys::on(Network::Testnet);
            load(&mut keys, sample, passphrase).unwrap_or_else(|e| panic!("{}: {e}", sample.name));
            for n in faraday_bip::bip85::WORD_COUNTS {
                let words = keys
                    .bip85_child(&c, 0, App::Words(n), 0, faraday_keys::Reveal::Bip85Child)
                    .expect("a child");
                let mut child = Keys::on(Network::Testnet);
                let fp = child
                    .add_words(&c, words.as_str(), "", None)
                    .expect("the child's words load");
                out.push(fp.0);
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The device refuses a sample's BIP-85 child typed bare on Mainnet from
/// its own short list: it must be exactly what the samples derive.
#[test]
fn the_devices_list_of_sample_children_is_every_index_zero_child_the_samples_give() {
    let derived = index_zero_children();
    let listed = faraday_keys::published::sample_children();
    let literal: String = derived
        .iter()
        .map(|f| {
            format!(
                "    [{:#04x}, {:#04x}, {:#04x}, {:#04x}],\n",
                f[0], f[1], f[2], f[3]
            )
        })
        .collect();
    assert!(listed == derived.as_slice(), "the list is:\n{literal}");
}

/// The device refuses the samples' Lightning nodes on Mainnet from its own
/// short list: LND's cipher seed as it reads on Mainnet, and Core
/// Lightning's `hsm_secret` samples and the nodes of their words.
#[test]
fn the_devices_list_of_sample_nodes_is_the_lnd_node_on_mainnet_and_the_hsm_and_words_nodes() {
    let c = compute();
    let sample = crate::common::key("sample_aezeed");
    let mut idx = [0u16; faraday_keys::lightning::AEZEED_WORDS];
    for (slot, w) in idx.iter_mut().zip(sample.parts[0].split_whitespace()) {
        *slot = Language::English.index_of(w).expect("an English word");
    }
    let lnd = faraday_keys::lightning::node_from_aezeed(&c, &idx, b"", Network::Mainnet)
        .expect("the node");
    let mut derived: Vec<[u8; 33]> = SAMPLES.inputs.hsm.iter().map(|h| h.node).collect();
    derived.extend(SAMPLES.inputs.hsm_words_nodes.iter().copied());
    derived.push(*lnd.public());
    derived.sort_unstable();
    derived.dedup();
    let mut listed = faraday_keys::published::sample_nodes().to_vec();
    listed.sort_unstable();
    let literal: String = derived.iter().map(|n| format!("    {n:?},\n")).collect();
    assert!(listed == derived, "the list is:\n{literal}");
}

/// Convert holds an aezeed as the source of its conversions: LND's public
/// sample is refused on Mainnet and held on Testnet.
#[test]
fn lnds_sample_cipher_seed_is_not_held_for_convert_on_mainnet_but_is_on_testnet() {
    use faraday_intake::secret_buf::SecretText;
    let c = compute();
    let sample = crate::common::key("sample_aezeed");
    let mut idx = [0u16; faraday_keys::lightning::AEZEED_WORDS];
    for (slot, w) in idx.iter_mut().zip(sample.parts[0].split_whitespace()) {
        *slot = Language::English.index_of(w).expect("an English word");
    }
    for (network, held) in [(Network::Mainnet, false), (Network::Testnet, true)] {
        let mut keys = Keys::on(network);
        let read = keys.aezeed_seed_job(&idx, SecretText::of("")).run(&c);
        let got = keys.aezeed_hold(read);
        assert_eq!(got.is_ok(), held, "{network:?}");
        if !held {
            let said = got.expect_err("a refusal");
            assert_eq!(
                said.as_str(),
                faraday_core::strings::EN.published_sample_node
            );
        }
    }
}

/// The device refuses the samples' SLIP-39 sets on Mainnet by identifier and
/// iteration exponent, from its own short list.
#[test]
fn the_devices_list_of_slip39_sample_sets_is_every_identifier_the_samples_give() {
    let mut derived: Vec<(u16, u8)> = SAMPLES
        .keys
        .iter()
        .filter(|k| matches!(k.form, SampleForm::Form(Form::Slip39)))
        .flat_map(|k| k.parts.iter())
        .map(|p| match part_of(p) {
            Some(Part::Slip39(s)) => {
                assert!(!s.is_extendable(), "the samples are not extendable");
                (s.identifier(), s.iteration_exponent())
            }
            _ => panic!("a SLIP-39 share"),
        })
        .collect();
    derived.sort_unstable();
    derived.dedup();
    let mut listed = faraday_keys::published::sample_slip39_sets().to_vec();
    listed.sort_unstable();
    assert!(listed == derived, "the list is: {derived:?}");
}

/// Convert holds a SLIP-39 set as the source of its conversions: a sample's
/// is refused on Mainnet, with nothing held, and held on Testnet.
#[test]
fn a_sample_slip39_set_is_not_held_for_convert_on_mainnet_but_is_on_testnet() {
    use faraday_keys::slip39_set::SetRefused;
    for k in SAMPLES
        .keys
        .iter()
        .filter(|k| matches!(k.form, SampleForm::Form(Form::Slip39)))
    {
        let mut main = Keys::on(Network::Mainnet);
        assert_eq!(
            main.slip39_set_hold(k.parts[0]).err(),
            Some(SetRefused::Sample),
            "{}",
            k.name
        );
        assert!(main.slip39_set_facts().is_none(), "{}", k.name);
        let mut test = Keys::on(Network::Testnet);
        assert!(test.slip39_set_hold(k.parts[0]).is_ok(), "{}", k.name);
        assert!(test.slip39_set_facts().is_some(), "{}", k.name);
    }
}
