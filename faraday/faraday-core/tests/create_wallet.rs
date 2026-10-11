//! Create a wallet opens on Kind, whatever opened it
//! (`docs/NEW-WALLET.md` §2.1, reversing `SIMPLIFY.md` §2.1 for this
//! flow): Kind defaults to single key, Multisig is the other everyday
//! choice, and the other eight kinds are behind More kinds. It ends in
//! the backup plan (`SIMPLIFY.md` §2.3): Keys' Continue makes the
//! wallet, Check follows, and Back up offers the plan's presets.

use faraday_core::create::NewKind;
use faraday_core::plan::{Answers, Preset};
use faraday_core::{Action, Faraday, Screen, cstep, testkit};
use osk_shell_api::{App, Event, Key};

fn frost_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Threshold)
        .expect("FROST is one of the ten kinds") as u8
}

fn multi_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .expect("Multisig is one of the ten kinds") as u8
}

/// Loads a test seed as a key in the session, through Add a key.
fn add_key(app: &mut Faraday, seed: &str) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(seed).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn opened() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    app
}

#[test]
fn a_fresh_create_opens_on_kind_with_the_single_key_kind() {
    let app = opened();
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::NativeSegwit);
    assert_eq!(
        c.open,
        Some(cstep::KIND),
        "Create opens on Kind, whatever opened it"
    );
    assert!(!c.done[cstep::KIND as usize], "not answered yet");
}

#[test]
fn continue_closes_kind_and_change_reopens_it_keeping_the_choice() {
    let mut app = opened();
    app.press(Action::CKind(1)); // Taproot, ticked while Kind is open
    app.press(Action::CNext(cstep::KIND)); // Kind's own Continue
    assert_eq!(app.create.as_ref().map(|c| c.kind), Some(NewKind::Taproot));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::KEYS),
        "Continue closes Kind as done and opens Keys"
    );
    app.press(Action::CStep(cstep::KIND));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::KIND),
        "Change opens Kind"
    );
    assert_eq!(
        app.create.as_ref().map(|c| c.kind),
        Some(NewKind::Taproot),
        "the kind chosen earlier is kept"
    );
}

#[test]
fn more_kinds_offers_frost() {
    let mut app = opened();
    // Kind is already open: a fresh Create opens on it.
    let _ = app.frame();
    assert!(
        !app.offers(Action::CKind(frost_index())),
        "FROST is behind More kinds"
    );
    app.press(Action::CMoreKinds);
    let _ = app.frame();
    assert!(
        app.offers(Action::CKind(frost_index())),
        "More kinds offers FROST"
    );
}

#[test]
fn a_tools_tile_for_musig2_opens_create_on_kind_with_musig2_ticked() {
    let mut app = testkit::started();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "MuSig2")
        .expect("a MuSig2 tile");
    app.press(Action::Catalog(i as u8));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::MuSig);
    assert_eq!(
        c.open,
        Some(cstep::KIND),
        "Create opens on Kind, with MuSig2 already ticked"
    );
    assert!(!c.done[cstep::KIND as usize]);
}

#[test]
fn a_tools_tile_for_multisig_ticks_its_quorum_default_too() {
    let mut app = testkit::started();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Multisig")
        .expect("a Multisig tile");
    app.press(Action::Catalog(i as u8));
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::Multi);
    assert_eq!(c.m, 2, "2 of 3 default");
    assert_eq!(c.n, 3);
    assert!(!c.done[cstep::QUORUM as usize]);
    assert_eq!(c.open, Some(cstep::KIND), "the flow opens on Kind");
}

/// A single-key wallet made from Create over a key typed in, stopped on
/// Check; returns the wallet's index.
fn made() -> (Faraday, usize) {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND)); // single key, the default: opens Keys
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let c = app.create.as_ref().expect("create");
    let built = c.built.expect("Keys' Continue makes the wallet");
    assert_eq!(c.open, Some(cstep::CHECK), "Check follows Keys");
    (app, built)
}

#[test]
fn after_check_the_next_card_is_back_up_with_three_presets() {
    let (mut app, _) = made();
    app.press(Action::CNext(cstep::CHECK));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::BACKUP),
        "Back up follows Check"
    );
    let _ = app.frame();
    for k in 0..3u8 {
        assert!(app.offers(Action::CBackup(k)), "preset {k} is offered");
    }
    assert!(!app.offers(Action::CBackup(3)), "three presets, no more");
}

#[test]
fn paper_and_vault_opens_the_backup_on_the_new_wallet_with_that_preset() {
    let (mut app, built) = made();
    app.press(Action::CNext(cstep::CHECK));
    let k = Preset::ALL
        .iter()
        .position(|p| *p == Preset::PaperVault)
        .expect("Paper and vault") as u8;
    app.press(Action::CBackup(k));
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().expect("backup");
    assert_eq!(b.wallet, built, "the backup is of the wallet just made");
    assert_eq!(
        b.answers,
        Answers::preset(&app.plan_shape(built), Preset::PaperVault),
        "the preset is applied"
    );
}

#[test]
fn back_from_the_backup_returns_to_the_wallet_card() {
    let (mut app, built) = made();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    let _ = app.frame();
    assert!(
        app.offers(Action::OpenWallet(built)),
        "the way back is offered"
    );
    app.press(Action::OpenWallet(built));
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet, built, "on the wallet just made");
}

#[test]
fn checks_address_rows_are_named_the_wallets_first_addresses() {
    let (mut app, _) = made();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "The wallet's first addresses"),
        "Check names whose addresses the rows are: {texts:?}"
    );
}

/// A 2-of-3 multisig, Create opened on Kind and advanced to Keys.
fn multisig_on_keys() -> Faraday {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CNext(cstep::QUORUM)); // 2 of 3, the default
    app
}

#[test]
fn a_key_in_one_slot_is_offered_in_no_other_and_clearing_offers_it_again() {
    let mut app = multisig_on_keys();
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    let _ = app.frame();
    assert!(
        !app.offers(Action::CSlotHere(1, fp)),
        "Key 1's key is not offered for Key 2"
    );
    app.press(Action::CSlotClear(0));
    let _ = app.frame();
    assert!(
        app.offers(Action::CSlotHere(1, fp)),
        "Clear on Key 1 offers it again"
    );
}

#[test]
fn the_keys_foot_button_opens_new_key_for_the_first_empty_slot() {
    let mut app = multisig_on_keys();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "New key for Key 1"),
        "the foot button names the first empty slot: {texts:?}"
    );
    let fp = app.session.keys[0].master.fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "New key for Key 2"),
        "the foot button moves to the next empty slot: {texts:?}"
    );
}

/// New key on its own, from this device's generator: locked in with no
/// passphrase, the quiz skipped, and Done. Returns the key's fingerprint.
fn key_made_on_its_own(app: &mut Faraday) -> [u8; 4] {
    use faraday_core::keygen::Way;
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.event(Event::Entropy(osk_shell_api::EntropyBytes::new([0x6c; 32])));
    app.press(Action::KNext);
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::KNext);
    app.press(Action::KSkip);
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert!(app.keygen.is_none(), "New key is done");
    app.session.keys[0].master.fingerprint().0
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn a_key_made_on_its_own_takes_a_passphrase_in_a_single_key_slot() {
    let mut app = testkit::started();
    let first = key_made_on_its_own(&mut app);
    let words = app.session.keys[0].words.clone().expect("words");
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CSlotHere(0, first));
    let _ = app.frame();
    assert!(app.offers(Action::CPassOpen(0)));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "Ride the 7 bus");
    app.press(Action::CPassField(1));
    type_text(&mut app, "Ride the 7 bus");
    app.press(Action::CPassLock);
    let with = faraday_core::wallet::Session::default()
        .add_words_with(&words, "Ride the 7 bus", "", None)
        .unwrap()
        .0;
    assert_ne!(with, first);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots[0], faraday_core::create::Source::Here(with));
    assert!(
        app.session
            .keys
            .iter()
            .any(|k| k.master.fingerprint().0 == first),
        "the key without the passphrase stays loaded"
    );
    assert!(app.drawn_texts().iter().any(|t| t == "Locked in"));
    app.press(Action::CNext(cstep::KEYS));
    let w = app
        .create
        .as_ref()
        .unwrap()
        .built
        .expect("the wallet is made");
    let fps: Vec<[u8; 4]> = app
        .session
        .slots(&app.session.wallets[w])
        .iter()
        .filter_map(|s| s.fingerprint.map(|f| f.0))
        .collect();
    assert_eq!(fps, vec![with], "the wallet is the passphrase key's");
}

#[test]
fn a_slot_whose_key_has_a_passphrase_offers_no_add_a_passphrase() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    let plain = app.session.keys[0].master.fingerprint().0;
    let words = testkit::test_words("bacon");
    let with = app
        .session
        .add_words_with(&words, "Ride the 7 bus", "with", None)
        .unwrap()
        .0;
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CSlotHere(0, with));
    let _ = app.frame();
    assert!(!app.offers(Action::CPassOpen(0)));
    app.press(Action::CSlotHere(0, plain));
    let _ = app.frame();
    assert!(app.offers(Action::CPassOpen(0)));
}

#[test]
fn unequal_passphrases_in_a_slot_lock_nothing_in() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    let plain = app.session.keys[0].master.fingerprint().0;
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CSlotHere(0, plain));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "one");
    app.press(Action::CPassField(1));
    type_text(&mut app, "two");
    app.press(Action::CPassLock);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.create.as_ref().unwrap().slots[0],
        faraday_core::create::Source::Here(plain)
    );
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "The two passphrases differ")
    );
}
