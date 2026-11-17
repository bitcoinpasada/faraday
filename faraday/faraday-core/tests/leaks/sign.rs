//! Signing: a PSBT of a one-key wallet whose key is the test seed,
//! reviewed and signed with the seed loaded, then Lock.

#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::type_text;
use crate::forms::{PHRASE, seed_forms};
use crate::{arm, finish, finish_left};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::MasterKey;
use faraday_core::testkit::{self, Kit};
use faraday_core::{Action, StorageEvent};
use faraday_shell_api::App;

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

/// What a loaded key rightly holds once its words are done: its words in
/// the key store (the backup shows them again) and the seed and keys worked
/// out from it. The typed copies in the entry state are not among them:
/// the other forms of the words (indices, entropy) fail if live.
pub const KEY_FORMS: &[&str] = &[
    "phrase text",
    "BIP-39 seed",
    "master private key",
    "master chain code",
    "master key as xprv",
    "master key as tprv",
    "xprv text",
    "tprv text",
];

/// The test seed typed into Add a key and a spend of the wallet signed.
fn signed(app: &mut faraday_core::Faraday) {
    let kit = kit();
    let psbt = testkit::unsigned(&kit).expect("the spend");
    app.storage(StorageEvent::Restored {
        inbox: vec![
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes().into()),
            (
                faraday_text::Text::of("leak-wallet.txt"),
                kit.descriptor.into_bytes().into(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(app, 1);
    app.press(Action::Entry(None));
    type_text(app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    app.press(Action::StartSpend(0));
    testkit::steps_before_sign_done(app);
    app.press(Action::SignHere);
    assert!(
        !app.spend().expect("the spend").spend.signed_here.is_empty(),
        "nothing was signed"
    );
}

#[test]
fn a_psbt_signed_with_the_seed_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    signed(&mut app);
    crate::seed_load::lock(&mut app);
    finish("psbt signed", armed, true);
}

/// A spend signed and its flow ended (Cancel is not offered once it is
/// signed, so the arrow, Escape and the mouse's back leave it in
/// progress): the typed words stay nowhere but in the key.
#[test]
fn the_words_typed_before_a_spend_was_signed_are_gone_once_the_spend_is_left() {
    for (way, leave) in crate::seed_load::ways_out() {
        let forms = seed_forms("");
        let armed = arm(&forms);
        let mut app = testkit::started();
        signed(&mut app);
        leave(&mut app);
        let _ = app.frame();
        finish_left(&format!("psbt signed, left by {way}"), armed, KEY_FORMS);
    }
}
