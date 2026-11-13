//! A version 2 PSBT (BIP-370, `docs/INTEROP.md` §5, PSBT version 2) is read wherever
//! a PSBT is: BIP-370's valid vectors read as the transaction they
//! describe, with its locktime, and its invalid ones are refused; a
//! wallet's spend offered as version 2 signs as its version 0 form does,
//! and the signed PSBT goes back out as version 2, the PSBT that came in
//! with only the signatures added.

use std::path::PathBuf;

use crate::common::add_key;
use faraday_core::testkit;
use faraday_core::wallet::{Spend, read_psbt};
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_psbt::Psbt;
use faraday_psbt::v2::PsbtV2;

/// BIP-370's own vectors (`tools/vectors/psbt-bips/vectors.json`):
/// (what the BIP expects, the bytes).
fn bip370() -> Vec<(String, Vec<u8>)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/psbt-bips/vectors.json");
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .filter(|c| c["bip"] == 370)
        .map(|c| {
            let hex = c["hex"].as_str().unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            (c["expect"].as_str().unwrap().to_string(), bytes)
        })
        .collect()
}

#[test]
fn bip370s_vectors_read_as_the_bip_says_binary_and_base64() {
    let c = testkit::compute();
    let mut seen = [0usize; 3];
    for (expect, bytes) in bip370() {
        let text = faraday_bip::base64::encode(&bytes);
        for form in [bytes.clone(), text.into_bytes()] {
            let read = read_psbt(&c, &form);
            match expect.as_str() {
                "invalid" | "locktime none" => {
                    assert!(read.is_none(), "{expect}: read");
                    seen[0] += 1;
                }
                "valid" => {
                    let p = read.unwrap_or_else(|| panic!("a valid PSBT refused"));
                    let v2 = PsbtV2::parse_bytes(&c, &bytes).unwrap();
                    assert_eq!(p.unsigned_tx(), &v2.unsigned_tx().unwrap());
                    seen[1] += 1;
                }
                lock => {
                    let want: u32 = lock.strip_prefix("locktime ").unwrap().parse().unwrap();
                    let p = read.unwrap_or_else(|| panic!("{lock}: refused"));
                    assert_eq!(p.unsigned_tx().lock_time.to_consensus_u32(), want);
                    seen[2] += 1;
                }
            }
        }
    }
    // Both forms of 24 invalid and one with no locktime, 13 valid and 9
    // locktimes.
    assert_eq!(seen, [50, 26, 18]);
}

fn kit() -> testkit::Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

#[test]
fn a_testkit_psbt_made_version_2_and_back_is_byte_identical() {
    let c = testkit::compute();
    for k in testkit::kits() {
        let v0 = testkit::unsigned(&k).unwrap();
        let v2 = PsbtV2::from_v0(&c, &v0);
        let read = PsbtV2::parse_bytes(&c, &v2.to_bytes()).unwrap();
        assert_eq!(
            read.to_v0(&c).unwrap().to_bytes(),
            v0.to_bytes(),
            "{}",
            k.id
        );
    }
}

/// Spending's PSBT offered as `bytes`, the wallet loaded, test key 1
/// added and signed here: the PSBT signed, the Outbox's copy.
fn signed_from(bytes: Vec<u8>) -> (Faraday, Vec<u8>) {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), bytes),
            (
                faraday_text::Text::of("spending-wallet.txt"),
                kit().descriptor.into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(
        app.spend().unwrap().spend.finished.is_some(),
        "signed and finished"
    );
    let out = app
        .outbox
        .iter()
        .find(|i| i.name == "spend-signed.psbt")
        .expect("the signed PSBT For the stick")
        .bytes
        .to_vec();
    (app, out)
}

#[test]
fn a_version_2_spend_signs_as_its_version_0_form_and_goes_back_out_as_version_2() {
    let c = testkit::compute();
    let v0 = testkit::unsigned(&kit()).unwrap();
    let v2 = PsbtV2::from_v0(&c, &v0).to_bytes();
    let (from_v0, out_v0) = signed_from(v0.to_bytes());
    let (from_v2, out_v2) = signed_from(v2.clone());
    // The same signatures and the same finished transaction.
    assert_eq!(
        from_v0.spend().unwrap().spend.finished_hex(),
        from_v2.spend().unwrap().spend.finished_hex()
    );
    let back = PsbtV2::parse_bytes(&c, &out_v2).expect("written as version 2");
    assert_eq!(
        back.to_v0(&c).unwrap(),
        Psbt::parse_bytes(&c, &out_v0).unwrap()
    );
    // The version 2 PSBT written out is the one that came in with the
    // partial signatures added and nothing else.
    let mut without = back.clone();
    for map in &mut without.inputs {
        assert!(map.all_of(0x02).count() > 0, "each input signed");
        map.remove_type(0x02);
    }
    assert_eq!(without.to_bytes(), v2);
}

#[test]
fn signatures_that_do_not_go_back_into_the_version_2_psbt_write_nothing() {
    let c = testkit::compute();
    let kits = testkit::kits();
    let (ours, other) = (kit(), kits.iter().find(|k| k.id != "spending").unwrap());
    let v0 = testkit::unsigned(&ours).unwrap();
    let as_read = PsbtV2::from_v0(&c, &v0);
    // A version 2 PSBT of another transaction beside this spend's version
    // 0 PSBT: its signatures have nowhere to go, so nothing is written,
    // version 2 or version 0.
    let elsewhere = PsbtV2::from_v0(&c, &testkit::unsigned(other).unwrap());
    let spend = Spend::new_kept(v0.clone(), Some(elsewhere), "spend.psbt");
    assert!(spend.out_bytes().is_err());
    // Its own version 2 PSBT goes back out as version 2.
    let spend = Spend::new_kept(v0, Some(as_read.clone()), "spend.psbt");
    assert_eq!(spend.out_bytes().unwrap(), as_read.to_bytes());
}

#[test]
fn a_psbt_with_bip375_silent_payment_fields_is_refused() {
    let c = testkit::compute();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors/bip375/bip375_test_vectors.json");
    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let valid = v["valid"].as_array().unwrap();
    assert!(!valid.is_empty());
    for case in valid {
        let text = case["psbt"].as_str().unwrap();
        let bytes = faraday_bip::base64::decode(text).unwrap();
        for form in [bytes.clone(), text.as_bytes().to_vec()] {
            assert!(
                faraday_core::wallet::read_psbt_kept(&c, &form).is_none(),
                "{}: read",
                case["description"]
            );
            assert!(faraday_core::wallet::is_silent_payment_psbt(&c, &form));
        }
    }
}

/// A version 2 PSBT of a spend, every output with its script, that
/// carries one BIP-375 field and nothing else of silent payments.
fn v2_with_field(place: &str, key_type: u8) -> Vec<u8> {
    let c = testkit::compute();
    let kit = testkit::kits().into_iter().next().unwrap();
    let mut v2 = PsbtV2::from_v0(&c, &testkit::unsigned(&kit).unwrap());
    assert!(!v2.has_silent_fields());
    let map = match place {
        "global" => &mut v2.global,
        "input" => &mut v2.inputs[0],
        _ => &mut v2.outputs[0],
    };
    map.set(vec![key_type], vec![0]);
    v2.to_bytes()
}

#[test]
fn a_psbt_with_only_one_bip375_field_is_refused() {
    let c = testkit::compute();
    for (place, ty) in [
        ("global", 0x07),
        ("global", 0x08),
        ("input", 0x1d),
        ("input", 0x1e),
        ("output", 0x09),
        ("output", 0x0a),
    ] {
        let bytes = v2_with_field(place, ty);
        assert!(
            faraday_core::wallet::read_psbt_kept(&c, &bytes).is_none(),
            "{place} {ty:#x}: read"
        );
        assert!(faraday_core::wallet::is_silent_payment_psbt(&c, &bytes));
    }
}

#[test]
fn a_version_0_psbt_with_a_bip375_field_is_refused() {
    let c = testkit::compute();
    let kit = testkit::kits().into_iter().next().unwrap();
    let plain = testkit::unsigned(&kit).unwrap();
    assert!(faraday_core::wallet::read_psbt_kept(&c, &plain.to_bytes()).is_some());
    for (place, ty) in [
        ("global", 0x07),
        ("global", 0x08),
        ("input", 0x1d),
        ("input", 0x1e),
        ("output", 0x09),
        ("output", 0x0a),
    ] {
        let mut v0 = plain.clone();
        let unknown = match place {
            "global" => &mut v0.inner_mut().unknown,
            "input" => &mut v0.inner_mut().inputs[0].unknown,
            _ => &mut v0.inner_mut().outputs[0].unknown,
        };
        unknown.insert(
            faraday_bip::bitcoin::psbt::raw::Key {
                type_value: ty,
                key: vec![],
            },
            vec![0],
        );
        for form in [v0.to_bytes(), v0.to_base64().into_bytes()] {
            assert!(
                faraday_core::wallet::read_psbt_kept(&c, &form).is_none(),
                "{place} {ty:#x}: read"
            );
            assert!(faraday_core::wallet::is_silent_payment_psbt(&c, &form));
        }
    }
}
