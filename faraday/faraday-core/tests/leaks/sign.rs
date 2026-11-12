//! Signing: a PSBT of a one-key wallet whose key is the test seed,
//! reviewed and signed with the seed loaded, then Lock.

use crate::common::type_text;
use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::MasterKey;
use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, StorageEvent};

/// The one-key native SegWit wallet of the test seed, its key as the
/// descriptor names it.
fn kit() -> Kit {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    let master = MasterKey::from_seed(&c, &m.to_seed(&c, b"").expect("seed"), testkit::net());
    let path = testkit::on_net("m/84'/1'/0'");
    let p: DerivationPath = path.parse().expect("path");
    let shown = path.trim_start_matches("m/").replace('\'', "h");
    let key = format!(
        "[{}/{shown}]{}",
        master.fingerprint(),
        master.derive(&c, &p).to_xpub(&c)
    );
    Kit {
        id: "leak",
        name: "Leak",
        descriptor: format!("wpkh({key}/<0;1>/*)"),
        cosigner: None,
        fee: 1_410,
    }
}

#[test]
fn a_psbt_signed_with_the_seed_leaves_no_copy_after_lock() {
    let kit = kit();
    let psbt = testkit::unsigned(&kit).expect("the spend");
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spend.psbt".to_string(), psbt.to_bytes().into()),
            ("leak-wallet.txt".to_string(), kit.descriptor.into_bytes().into()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(1));
    app.press(Action::Entry(None));
    type_text(&mut app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    app.press(Action::StartSpend(0));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    assert!(
        !app.spend().expect("the spend").spend.signed_here.is_empty(),
        "nothing was signed"
    );
    crate::seed_load::lock(&mut app);
    finish("psbt signed", armed, true);
}
