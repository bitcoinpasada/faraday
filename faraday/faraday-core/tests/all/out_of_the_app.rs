//! FROST, and MuSig2 signing across places, are compiled out of the app
//! until their standards mature (docs/decisions/F109.md): a person
//! reaches neither. These run in the build every shipped app is, without
//! `unreleased`; the code itself is tested with it turned on.
#![cfg(not(feature = "unreleased"))]

use faraday_core::testkit;
use faraday_core::wallet::{FileKind, Refusal};
use faraday_core::{Action, Screen, StorageEvent};

/// The Demo's FROST 2-of-3 record, public test data: what a person would
/// carry in from a wallet made by a build with FROST.
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
fn a_frost_record_is_no_wallet_here_though_the_core_still_reads_it() {
    let c = testkit::compute();
    assert!(
        faraday_bip::policy::WalletPolicy::parse_any(&c, RECORD).is_ok(),
        "the core crate keeps FROST"
    );
    assert!(matches!(
        faraday_core::wallet::read_wallet(&c, RECORD),
        Err(Refusal::NotAWallet(_))
    ));
    let mut app = with_inbox(vec![(
        "threshold-wallet.txt".to_string(),
        RECORD.as_bytes().to_vec(),
    )]);
    assert_ne!(app.inbox()[0].kind, FileKind::Wallet);
    app.press(Action::LoadWallet(0));
    assert!(app.session.wallets.is_empty());
}

#[test]
fn a_frost_carry_file_does_not_open_a_spend() {
    let mut carry = b"OSKC\x01".to_vec();
    carry.extend_from_slice(&[0, 0]);
    let mut app = with_inbox(vec![("partly-signed.osk".to_string(), carry)]);
    assert_ne!(app.inbox()[0].kind, FileKind::Psbt);
    app.press(Action::StartSpend(0));
    assert_ne!(app.screen, Screen::Spend);
    assert!(app.spend().is_none());
}

#[test]
fn a_24_word_key_is_no_frost_share() {
    let c = testkit::compute();
    let mut app = testkit::started();
    let words = faraday_core::samples::repeated("bacon", 24);
    app.session
        .keys
        .add_words(&c, &words, "bacon", None)
        .expect("24 words");
    assert!(app.session.keys.info(0).expect("the key").share.is_none());
}

#[test]
fn learn_has_no_frost_page_and_tools_no_frost_tool() {
    assert!(
        faraday_learn::pages::EN
            .pages()
            .iter()
            .all(|p| p.title != "FROST")
    );
    assert!(
        faraday_core::catalog::TILES
            .iter()
            .all(|t| !t.name.contains("FROST") && !t.tags.contains(&"BIP-445"))
    );
}
