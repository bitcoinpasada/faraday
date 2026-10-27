//! What the app does with the files a removed feature left behind
//! (docs/decisions/F127.md): a threshold wallet's record, a partly signed
//! carry file and a PSBT carrying Faraday's own `osk` records are each
//! refused, and nothing of them is signed or loaded.

use faraday_core::testkit;
use faraday_core::wallet::{FileKind, Origin};
use faraday_core::{Action, Screen, StorageEvent};
use faraday_psbt::WarningKind;

/// A threshold wallet's record as an earlier build wrote it, public test
/// data: what a person might still carry in.
const RECORD: &str = "osk-threshold 1\nthreshold 2\ngroup 03f74b3a5d82b000a1a9078fbf6a7fd26a43dfc5727ea716204d511dd7669a57bc\nshare 0 02a798ae6f5eb7446492a5c8baf71c59e1d259985e663923513f2cfdadbc482187\nshare 1 037e4ced9ae783bf2071688c61a3526ea01da26348b38980d99a126e5896de3583\nshare 2 02bcbf572cb04013d8d1a6b1fd281e60336fd51cbd130fcccabf391565b1e0aaa0\ntr(tpubD6NzVbkrYhZ4XgHkCEtfpuZPJDLaLPxu5ZBEtAbub9GcUX1mTS2t3eCnBaYxKDfyvecpxgGsf4MtTC3gyZz9FD4qq6Km68VSDnpD5vpWm3e/<0;1>/*)#edvh7ggu";

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> faraday_core::Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

#[test]
fn a_threshold_record_is_no_wallet() {
    let c = testkit::compute();
    assert!(faraday_core::wallet::read_wallet(&c, RECORD).is_err());
    let mut app = with_inbox(vec![(
        "threshold-wallet.txt".to_string(),
        RECORD.as_bytes().to_vec(),
    )]);
    assert_ne!(app.inbox()[0].kind, FileKind::Wallet);
    app.press(Action::LoadWallet(0));
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_carry_file_does_not_open_a_spend() {
    let mut carry = b"OSKC\x01".to_vec();
    carry.extend_from_slice(&[0, 0]);
    let mut app = with_inbox(vec![("partly-signed.osk".to_string(), carry)]);
    assert_ne!(app.inbox()[0].kind, FileKind::Psbt);
    app.press(Action::StartSpend(0));
    assert_ne!(app.screen, Screen::Spend);
    assert!(app.spend().is_none());
}

/// The Taproot test wallet's spend with its key loaded, and with an `osk`
/// record on its input when `osk`: signed when it has none, and refused,
/// with its reason, when it has one.
#[test]
fn a_psbt_with_osk_records_is_not_signed_and_says_why() {
    let c = testkit::compute();
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "taproot")
        .expect("the Taproot test wallet");
    for osk in [false, true] {
        let (mut psbt, _) = testkit::spend_of(&kit).expect("its spend");
        if osk {
            let key = faraday_psbt::bitcoin::psbt::raw::ProprietaryKey {
                prefix: b"osk".to_vec(),
                subtype: 0,
                key: vec![2; 66],
            };
            psbt.inner_mut().inputs[0]
                .proprietary
                .insert(key, vec![0; 66]);
        }
        let before = psbt.to_bytes();
        let mut app = with_inbox(vec![("taproot-unsigned.psbt".to_string(), before.clone())]);
        testkit::move_to(&mut app.session, testkit::NET).expect("the test network");
        let (word, label) = testkit::TEST_SEEDS[0];
        app.session
            .keys
            .add_words(&c, &testkit::test_words(word), label, None)
            .expect("the key");
        app.session
            .add_wallet(&c, kit.name, &kit.descriptor, "Test wallet", Origin::Named)
            .expect("the wallet");
        app.press(Action::StartSpend(0));
        assert_eq!(app.screen, Screen::Spend);
        app.press(Action::SignHere);
        let s = app.spend().expect("the spend");
        if osk {
            // The review says why: the input is blocked, and its line
            // names the records.
            assert!(s.inspection.warnings.iter().any(|w| {
                w.kind == WarningKind::UnreadRecords
                    && w.level == faraday_psbt::Level::Blocked
                    && w.text.contains("osk")
            }));
            assert!(!s.complete, "signed");
            assert!(s.spend.signed_here.is_empty(), "signed here");
            assert_eq!(s.spend.psbt.to_bytes(), before, "written to");
            assert!(s.error.is_some(), "signing is refused");
        } else {
            assert!(s.complete, "{:?}", s.error);
        }
    }
}
