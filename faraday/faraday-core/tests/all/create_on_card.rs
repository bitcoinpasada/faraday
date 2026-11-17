//! Make a new wallet's Keys card makes the wallet's first key where it is
//! chosen: 12 words and dice already chosen, the count of rolls still
//! needed, Make the words, Show words, an optional passphrase, and the
//! slot's button. New key stays behind Other ways.

use crate::common::type_text;
use faraday_core::create::{BackupForm, NewKind, Source};
use faraday_core::{Action, Faraday, Screen, Sheet, cstep, testkit};
use faraday_shell_api::App;

fn opened() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::CNext(cstep::KIND));
    assert_eq!(app.screen, Screen::Create);
    app
}

fn multi_on_keys() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CStep(cstep::KIND));
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .expect("Multisig is one of the kinds") as u8;
    crate::common::ckind(&mut app, multi);
    app.press(Action::CNext(cstep::QUORUM));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    app
}

/// Rolls `n` dice on the card, the faces varied.
fn roll(app: &mut Faraday, n: usize) {
    for i in 0..n {
        let before = app.keygen().unwrap().dice.len();
        for f in 0..6u8 {
            app.press(Action::KRoll(((i * 5 + f as usize) % 6) as u8 + 1));
            if app.keygen().unwrap().dice.len() > before {
                break;
            }
        }
    }
}

fn roll_all(app: &mut Faraday) {
    let need = app.keygen().unwrap().dice_needed();
    roll(app, need);
    assert!(app.keygen().unwrap().ready(), "the count is in");
}

fn words_made(app: &mut Faraday) {
    roll_all(app);
    app.press(Action::KNext);
    assert!(
        app.keygen().unwrap().mnemonic.is_some(),
        "the words are made"
    );
}

#[test]
fn a_single_key_opens_keys_with_12_words_and_dice_chosen_and_the_count_needed() {
    let mut app = opened();
    let k = app.keygen().expect("the making is open on the card");
    assert!(k.on_card);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(k.words, 12);
    assert!(k.length_chosen);
    assert_eq!(k.dice_needed(), 70);
    let texts = app.drawn_texts();
    for want in ["12 words", "24 words", "Make the words", "Other ways"] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
    assert!(
        texts.iter().any(|t| t.contains("0 of 70")),
        "the count needed: {texts:?}"
    );
}

#[test]
fn rolling_the_count_then_make_the_words_shows_them_masked_with_show_words() {
    let mut app = opened();
    let _ = app.frame();
    assert!(
        !app.offers(Action::KNext),
        "Make the words waits for the count"
    );
    roll_all(&mut app);
    let _ = app.frame();
    assert!(app.offers(Action::KNext));
    app.press(Action::KNext);
    let k = app.keygen().unwrap();
    assert!(k.mnemonic.is_some());
    assert!(!k.shown, "masked until Show words");
    let _ = app.frame();
    assert!(app.offers(Action::KShow));
    app.press(Action::KShow);
    assert!(app.keygen().unwrap().shown);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Make again"), "{texts:?}");
}

#[test]
fn make_the_wallet_on_the_card_makes_the_wallet_and_opens_check() {
    let mut app = opened();
    words_made(&mut app);
    let fp = app.keygen().unwrap().fingerprint.expect("a fingerprint");
    app.press(Action::KLock);
    assert_eq!(app.screen, Screen::Create);
    assert!(app.keygen().is_none(), "the making is over");
    let c = app.create().unwrap();
    assert!(c.built.is_some(), "the wallet is made");
    assert_eq!(c.slots[0], Source::Here(fp));
    assert_eq!(c.open, Some(cstep::CHECK));
    assert!(
        (&app.session.keys)
            .into_iter()
            .any(|k| k.fingerprint.0 == fp)
    );
}

#[test]
fn twenty_four_words_after_rolls_drops_the_rolls_and_needs_140() {
    let mut app = opened();
    roll(&mut app, 7);
    assert_eq!(app.keygen().unwrap().dice.len(), 7);
    app.press(Action::KWords(24));
    let k = app.keygen().unwrap();
    assert_eq!(k.words, 24);
    assert!(k.dice.is_empty(), "the rolls went");
    assert_eq!(k.dice_needed(), 140);
    assert!(k.on_card);
}

#[test]
fn a_passphrase_on_the_card_makes_the_key_with_it_and_unequal_ones_hold_the_button() {
    let mut app = opened();
    words_made(&mut app);
    let plain = app.keygen().unwrap().fingerprint.expect("a fingerprint");
    app.press(Action::KPassField(0));
    type_text(&mut app, "correct horse");
    app.press(Action::KPassField(1));
    type_text(&mut app, "correct hors");
    app.press(Action::KLock);
    assert!(app.session.keys.is_empty(), "unequal: held");
    assert!(app.keygen().is_some());
    type_text(&mut app, "e");
    app.press(Action::PassEye);
    app.press(Action::KLock);
    let c = app.create().unwrap();
    assert!(c.built.is_some(), "equal: made");
    let Source::Here(fp) = c.slots[0] else {
        panic!("the slot holds the key");
    };
    assert_ne!(fp, plain, "the passphrase is in the key");
}

#[test]
fn leaving_make_a_new_wallet_drops_the_rolls_and_words() {
    let mut app = opened();
    roll(&mut app, 5);
    app.press(Action::Nav(Screen::Home));
    assert!(app.keygen().is_none(), "Nav away drops the rolls");
    // Kind changed: the making ends.
    let mut app = opened();
    words_made(&mut app);
    app.press(Action::CStep(cstep::KIND));
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .expect("Multisig") as u8;
    crate::common::ckind(&mut app, multi);
    assert!(
        app.keygen()
            .is_none_or(|k| k.mnemonic.is_none() && k.dice.is_empty()),
        "the words did not follow the Kind change"
    );
}

#[test]
fn cancel_with_rolls_on_the_card_asks_first() {
    let mut app = opened();
    roll(&mut app, 5);
    app.press(Action::CancelFlow);
    assert!(app.sheet.is_some(), "it asks first");
    assert!(
        app.keygen().is_some(),
        "nothing is dropped before the answer"
    );
}

#[test]
fn other_ways_opens_new_key_for_the_slot() {
    let mut app = opened();
    let _ = app.frame();
    assert!(app.offers(Action::KeyGen(Some(0))));
    app.press(Action::KeyGen(Some(0)));
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert!(!k.on_card);
    assert_eq!(k.slot, Some(0));
    // With rolls in it asks before they go.
    let mut app = opened();
    roll(&mut app, 4);
    app.press(Action::KeyGen(Some(0)));
    assert_eq!(app.sheet, Some(Sheet::CancelKey));
}

#[test]
fn shares_keep_new_key_on_the_slot() {
    let mut app = opened();
    app.press(Action::CBackupForm(BackupForm::Codex32.index()));
    assert!(
        app.keygen().is_none_or(|k| !k.on_card),
        "no making for shares"
    );
    let _ = app.frame();
    assert!(
        app.offers(Action::KeyGen(Some(0))),
        "New key is on the slot"
    );
    app.press(Action::KeyGen(Some(0)));
    assert!(app.keygen().unwrap().codex32_offered);
}

#[test]
fn a_multisig_key_1_locks_in_on_the_card_and_key_2_keeps_scan_its_xpub_first() {
    let mut app = multi_on_keys();
    assert!(app.keygen().is_some_and(|k| k.on_card));
    words_made(&mut app);
    app.press(Action::KLock);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    assert!(matches!(app.create().unwrap().slots[0], Source::Here(_)));
    assert!(app.keygen().is_none(), "Key 2 has no making");
    let _ = app.frame();
    assert!(app.offers(Action::ScanFor(faraday_core::Ask::Xpub(2))));
}

#[test]
fn the_card_refuses_before_the_self_test_passes() {
    let mut app = opened();
    words_made(&mut app);
    app.run_selftest_with(&[faraday_selftest::Check {
        name: "Injected failure",
        run: |_| false,
    }]);
    app.press(Action::KLock);
    assert!(app.create().is_none_or(|c| c.built.is_none()), "no wallet");
}
