//! Create a wallet opens on the first card that needs a person, not on
//! Kind (`docs/SIMPLIFY.md` §2.1, §2.2): Kind defaults to single key,
//! Multisig is the other everyday choice, and the other eight kinds are
//! behind More kinds. It ends in the backup plan (§2.3): Keys' Continue
//! makes the wallet, Check follows, and Back up offers the plan's presets.

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

fn opened() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    app
}

#[test]
fn a_fresh_create_opens_on_keys_with_the_single_key_kind() {
    let app = opened();
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::NativeSegwit);
    assert_eq!(c.open, Some(cstep::KEYS), "Kind defaults closed");
    assert!(c.done[cstep::KIND as usize], "Kind counts as answered");
}

#[test]
fn change_opens_kind_and_keeps_the_cards_after_it() {
    let mut app = opened();
    // Fill the one key slot so Keys' own value is something to keep.
    app.press(Action::CKind(1)); // from a closed Kind: re-opens on Keys
    let kept_kind = app.create.as_ref().map(|c| c.kind);
    assert_eq!(kept_kind, Some(NewKind::Taproot));
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
    app.press(Action::CStep(cstep::KIND));
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
fn a_tools_tile_for_musig2_opens_create_with_musig2_chosen_and_kind_closed() {
    let mut app = testkit::started();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "MuSig2")
        .expect("a MuSig2 tile");
    app.press(Action::Catalog(i as u8));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::MuSig);
    assert_ne!(c.open, Some(cstep::KIND), "Kind is closed");
    assert!(c.done[cstep::KIND as usize]);
}

#[test]
fn choosing_multisig_from_a_closed_kind_closes_quorum_on_its_default_too() {
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
    assert!(c.done[cstep::QUORUM as usize]);
    assert_eq!(c.open, Some(cstep::KEYS), "the flow opens on Keys");
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
