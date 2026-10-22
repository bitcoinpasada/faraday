//! The sample wallets are the testkit's, each with its unsigned spend, and
//! a session loads each of them with the keys the sheet loads beside it.

use crate::common::{hex, key};
use faraday_core::testkit::{self, compute};
use faraday_core::wallet::{Origin, Session};
use faraday_samples::SAMPLES;

/// What the testkit makes, in the order the samples list it: the ten
/// wallets of `kits()`, MuSig2, FROST and the 3 of 5, each as its name,
/// its text and its unsigned spend.
fn testkit_wallets() -> Vec<(&'static str, String, Vec<u8>)> {
    let names = [
        "sample_savings",
        "sample_spending",
        "sample_taproot",
        "sample_nested",
        "sample_legacy",
        "sample_nested_multisig",
        "sample_legacy_multisig",
        "sample_taproot_multisig",
        "sample_inheritance",
        "sample_tree",
    ];
    let kits = testkit::kits();
    assert_eq!(kits.len(), names.len());
    let mut all: Vec<_> = kits
        .iter()
        .zip(names)
        .map(|(k, name)| {
            let spend = testkit::unsigned(k).expect("a spend").to_bytes();
            (name, k.descriptor.clone(), spend)
        })
        .collect();
    all.push((
        "sample_musig",
        testkit::musig_policy()
            .expect("musig")
            .to_descriptor_checksummed(),
        testkit::musig_unsigned().expect("a spend").to_bytes(),
    ));
    all.push((
        "sample_frost",
        testkit::threshold_record().expect("a record").to_text(),
        testkit::threshold_unsigned().expect("a spend").to_bytes(),
    ));
    all.push(("sample_three_of_five", testkit::three_of_five(), Vec::new()));
    all
}

#[test]
fn the_sample_wallets_and_their_unsigned_spends_are_the_testkits() {
    let want = testkit_wallets();
    assert_eq!(SAMPLES.wallets.len(), want.len());
    for (have, (name, text, spend)) in SAMPLES.wallets.iter().zip(&want) {
        assert_eq!(have.name, *name);
        assert_eq!(have.text, text, "{name}");
        assert_eq!(have.spend, spend.as_slice(), "{name}'s spend");
    }
}

#[test]
fn a_session_loads_every_sample_wallet_and_the_spends_read_as_psbts() {
    let c = compute();
    let mut session = Session::on(faraday_bip::keys::Network::Testnet);
    for w in SAMPLES.wallets {
        session
            .add_wallet(&c, w.name, w.text, "Sample", Origin::Statement)
            .unwrap_or_else(|e| panic!("{}: {e:?}", w.name));
        if !w.spend.is_empty() {
            faraday_psbt::Psbt::parse_bytes(&c, w.spend)
                .unwrap_or_else(|e| panic!("{}'s spend: {e:?}", w.name));
        }
    }
    assert_eq!(session.wallets.len(), SAMPLES.wallets.len());
}

#[test]
fn a_wallets_keys_are_the_sample_keys_its_text_names() {
    let sample_fps: Vec<[u8; 4]> = SAMPLES.keys.iter().filter_map(|k| k.plain).collect();
    for w in SAMPLES.wallets {
        assert!(!w.keys.is_empty(), "{}", w.name);
        for fp in w.keys {
            assert!(sample_fps.contains(fp), "{}: {}", w.name, hex(fp));
            // A FROST record names its shares, not the keys' fingerprints.
            if w.name != "sample_frost" {
                assert!(w.text.contains(&hex(fp)), "{}: {}", w.name, hex(fp));
            }
        }
        // And every sample key the text names is there.
        if w.name != "sample_frost" {
            for fp in &sample_fps {
                assert_eq!(
                    w.text.contains(&hex(fp)),
                    w.keys.contains(fp),
                    "{}: {}",
                    w.name,
                    hex(fp)
                );
            }
        }
    }
    // The Savings wallet is the 2 of 3 over the first three test seeds;
    // FROST's shares 1 and 2 are bacon and zebra.
    let bacon = key("sample_words").plain.expect("bacon");
    let savings = SAMPLES.wallets[0];
    assert!(savings.keys.contains(&bacon));
    assert_eq!(savings.keys.len(), 3);
    let frost = SAMPLES
        .wallets
        .iter()
        .find(|w| w.name == "sample_frost")
        .expect("frost");
    assert_eq!(frost.keys.len(), 2);
    assert!(frost.keys.contains(&bacon));
}

#[test]
fn the_three_of_five_has_no_spend_and_every_other_wallet_has_one() {
    for w in SAMPLES.wallets {
        assert_eq!(
            w.spend.is_empty(),
            w.name == "sample_three_of_five",
            "{}",
            w.name
        );
    }
}

#[test]
fn the_kind_and_script_a_wallet_sample_records_are_what_its_policy_reads_as() {
    use faraday_bip::keys::ScriptType;
    use faraday_core::samples::{SampleKind as K, SampleScript as S};
    let c = compute();
    for w in SAMPLES.wallets {
        let p = faraday_core::wallet::read_wallet(&c, w.text).expect(w.name);
        let script = match p.script_type() {
            ScriptType::Legacy => S::Legacy,
            ScriptType::NestedSegwit => S::NestedSegwit,
            ScriptType::NativeSegwit => S::NativeSegwit,
            ScriptType::Taproot if w.name == "sample_tree" => S::TaprootTree,
            ScriptType::Taproot => S::Taproot,
        };
        assert_eq!(w.script, script, "{}", w.name);
        let quorum = p.tapscript_quorum().or_else(|| p.quorum());
        match w.kind {
            K::Multisig { need, total } => {
                let want = (usize::from(need), usize::from(total));
                assert_eq!(quorum, Some(want), "{}", w.name);
            }
            K::MuSig(keys) => assert_eq!(usize::from(keys), p.keys().len(), "{}", w.name),
            K::Single | K::Timelock | K::Frost { .. } => assert_eq!(quorum, None, "{}", w.name),
        }
    }
    // A FROST group is 2 of 3 and the inheritance wallet is the timelocked
    // one.
    let kind = |name: &str| {
        SAMPLES
            .wallets
            .iter()
            .find(|w| w.name == name)
            .expect(name)
            .kind
    };
    assert_eq!(kind("sample_frost"), K::Frost { need: 2, total: 3 });
    assert_eq!(kind("sample_inheritance"), K::Timelock);
}
