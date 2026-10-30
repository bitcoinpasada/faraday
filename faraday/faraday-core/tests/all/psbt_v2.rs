//! A version 2 PSBT (BIP-370, `docs/INTEROP.md` §5 I10) is read wherever
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
        inbox: vec![
            ("spend.psbt".to_string(), bytes),
            (
                "spending-wallet.txt".to_string(),
                kit().descriptor.into_bytes(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
    add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
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
        .clone();
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
