//! Make a new wallet's Keys card makes the wallet's first key where it is
//! chosen: neither count chosen until one is pressed, then the count of
//! rolls still needed, Make the words, Show words, an optional passphrase, and the
//! slot's button. New key stays behind Other ways.

use crate::common::type_text;
use faraday_core::create::{BackupForm, NewKind, Source};
use faraday_core::{Action, Faraday, Screen, Sheet, cstep, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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
    if !app.keygen().unwrap().length_chosen {
        app.press(Action::KWords(12));
    }
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

/// Scrolls the page down, as a person would, until a frame offers `action`.
fn scroll_to(app: &mut Faraday, action: Action) {
    for _ in 0..40 {
        let _ = app.frame();
        if app.offers(action) {
            return;
        }
        app.event(Event::Scroll {
            x: 640,
            y: 400,
            dy: 120,
        });
    }
    panic!("the page never offers {action:?}");
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
fn a_single_key_opens_keys_with_neither_count_chosen_and_no_rolls_to_enter() {
    let mut app = opened();
    let k = app.keygen().expect("the making is open on the card");
    assert!(k.on_card);
    assert_eq!(app.screen, Screen::Create);
    assert!(!k.length_chosen, "neither count is chosen");
    let texts = app.drawn_texts();
    for want in ["12 words", "24 words", "Other ways"] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
    assert!(
        !texts
            .iter()
            .any(|t| t == "Make the words" || t.contains("0 of")),
        "no rolls or Make the words before a pick: {texts:?}"
    );
    let _ = app.frame();
    assert!(!app.offers(Action::KRoll(3)), "no pad before a pick");
    assert!(!app.offers(Action::KNext), "no Make the words");
    app.press(Action::KWords(12));
    let k = app.keygen().unwrap();
    assert!(k.length_chosen);
    assert_eq!(k.words, 12);
    assert_eq!(k.dice_needed(), 70);
    // Wide windows start typed; the pad is Buttons'.
    app.press(Action::KTyping(false));
    let _ = app.frame();
    let texts = app.drawn_texts();
    for want in ["Make the words", "Other ways"] {
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
    scroll_to(&mut app, Action::KNext);
    app.press(Action::KNext);
    let k = app.keygen().unwrap();
    assert!(k.mnemonic.is_some());
    assert!(!k.shown, "masked until Show words");
    scroll_to(&mut app, Action::KShow);
    app.press(Action::KShow);
    assert!(app.keygen().unwrap().shown);
    scroll_to(&mut app, Action::KAgain);
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
fn other_ways_opens_the_keys_cards_inside_make_a_new_wallet() {
    let mut app = opened();
    let _ = app.frame();
    assert!(app.offers(Action::KeyOtherWays(0)));
    app.press(Action::KeyOtherWays(0));
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert!(!k.on_card);
    assert_eq!(k.slot, Some(0));
    assert!(k.hosted(), "Make a new wallet is the flow it is a step of");
    // With rolls in it asks before they go.
    let mut app = opened();
    roll(&mut app, 4);
    app.press(Action::KeyOtherWays(0));
    assert_eq!(app.sheet, Some(Sheet::CancelKey));
}

#[test]
fn other_ways_carries_the_length_chosen_on_the_card() {
    let mut app = opened();
    app.press(Action::KWords(24));
    app.press(Action::KeyOtherWays(0));
    let k = app.keygen().unwrap();
    assert!(k.hosted());
    assert!(k.length_chosen);
    assert_eq!(k.words, 24);
    // Discard on the sheet opens it the same way, the length kept.
    let mut app = opened();
    app.press(Action::KWords(24));
    roll(&mut app, 3);
    app.press(Action::KeyOtherWays(0));
    assert_eq!(app.sheet, Some(Sheet::CancelKey));
    app.press(Action::CancelFlowNow);
    let k = app.keygen().unwrap();
    assert!(k.hosted() && k.length_chosen && k.words == 24);
    assert!(k.dice.is_empty());
}

#[test]
fn back_from_the_first_card_returns_to_keys_with_the_answers_kept() {
    let mut app = opened();
    let backup = app.create().unwrap().backup;
    app.press(Action::KeyOtherWays(0));
    assert_eq!(app.screen, Screen::KeyGen);
    let _ = app.frame();
    let back = app.back_link().expect("the first card has a way back");
    app.press(back);
    assert_eq!(app.screen, Screen::Create);
    let c = app.create().unwrap();
    assert_eq!(c.open, Some(cstep::KEYS));
    assert_eq!(c.backup, backup);
    assert_eq!(c.slots[0], Source::Empty);
}

#[test]
fn other_ways_pressed_with_a_stick_in_opens_the_keys_cards_once_it_is_out() {
    let mut app = opened();
    let _ = app.frame();
    app.storage(boot_stick(true));
    app.press(Action::KeyOtherWays(0));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    app.storage(boot_stick(false));
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    let k = app.keygen().unwrap();
    assert!(
        k.hosted() && !k.on_card,
        "the key's cards, not the card's dice"
    );
}

#[test]
fn a_stick_plugged_in_while_the_keys_cards_are_open_asks_to_lock_and_stays_in_the_wallet() {
    let mut app = opened();
    app.press(Action::KeyOtherWays(0));
    assert!(app.keygen().unwrap().hosted());
    app.storage(plain_stick(true));
    assert_eq!(app.sheet, Some(Sheet::Lock));
    assert_eq!(
        app.screen,
        Screen::KeyGen,
        "the key and the wallet are kept"
    );
    assert!(app.create().is_some());
}

fn boot_stick(on: bool) -> faraday_core::StorageEvent {
    stick(on, true)
}

fn plain_stick(on: bool) -> faraday_core::StorageEvent {
    stick(on, false)
}

fn stick(on: bool, boot: bool) -> faraday_core::StorageEvent {
    faraday_core::StorageEvent::Sticks(if on {
        vec![faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("a"),
            label: faraday_text::Text::of("STICK"),
            boot,
            files: Vec::new(),
            vaults: Vec::new(),
        }]
    } else {
        Vec::new()
    })
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
    assert!(app.keygen().unwrap().hosted());
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

#[test]
fn new_words_on_key_2_opens_the_making_there_after_key_1_is_a_cosigners_xpub() {
    let mut app = multi_on_keys();
    // Key 1 is a cosigner's xpub, so no key made here holds a slot.
    app.press(Action::CSlotLater(0));
    assert!(app.keygen().is_none(), "Key 1 has no making");
    let _ = app.frame();
    assert!(app.offers(Action::ScanFor(faraday_core::Ask::Xpub(2))));
    assert!(app.offers(Action::KeyOnCard(1)), "Key 2 offers New words");
    app.press(Action::KeyOnCard(1));
    let k = app.keygen().expect("the making opens");
    assert!(k.on_card);
    assert_eq!(k.slot, Some(1));
    assert_eq!(app.screen, Screen::Create);
    words_made(&mut app);
    app.press(Action::KLock);
    assert!(matches!(app.create().unwrap().slots[1], Source::Here(_)));
    assert!(app.keygen().is_none());
}

#[test]
fn new_words_on_another_slot_drops_the_first_making_asking_first() {
    let mut app = multi_on_keys();
    assert_eq!(app.keygen().and_then(|k| k.slot), Some(0));
    // Nothing rolled: it moves without asking.
    let _ = app.frame();
    app.press(Action::KeyOnCard(1));
    assert_eq!(app.keygen().and_then(|k| k.slot), Some(1));
    assert_eq!(app.sheet, None);
    // With rolls in it asks; keeping them leaves the first making.
    roll(&mut app, 4);
    app.press(Action::KeyOnCard(0));
    assert_eq!(app.sheet, Some(Sheet::CancelKey));
    assert_eq!(app.keygen().and_then(|k| k.slot), Some(1));
    // Discard moves the making to the other slot, empty.
    app.press(Action::CancelFlowNow);
    assert_eq!(app.keygen().and_then(|k| k.slot), Some(0));
    assert_eq!(app.keygen().unwrap().dice.len(), 0);
}

/// Rolls the faces 2 to 6 in turn, `n` of them, so that no face reads as
/// a bit by itself.
fn roll_high(app: &mut Faraday, n: usize) {
    if !app.keygen().unwrap().length_chosen {
        app.press(Action::KWords(12));
    }
    for i in 0..n {
        let before = app.keygen().unwrap().dice.len();
        for f in 0..3u8 {
            app.press(Action::KRoll((i as u8 + f) % 3 + 2));
            if app.keygen().unwrap().dice.len() > before {
                break;
            }
        }
    }
}

#[test]
fn rolls_part_way_show_words_so_far_with_the_first_words_bits_and_both_entry_ways() {
    let mut app = opened();
    roll_high(&mut app, 6);
    assert_eq!(app.keygen().unwrap().dice.len(), 6);
    app.press(Action::KTyping(false));
    let _ = app.frame();
    let texts = app.drawn_texts();
    for want in ["Words so far", "Typed", "Buttons"] {
        assert!(texts.iter().any(|t| t == want), "{want}: {texts:?}");
    }
    assert!(
        texts
            .iter()
            .any(|t| !t.is_empty() && t.chars().all(|c| c == '0' || c == '1')),
        "the bits of the first word's rolls: {texts:?}"
    );
    assert!(app.offers(Action::KTyping(true)));
    assert!(app.offers(Action::KTyping(false)));
    assert!(app.offers(Action::KRoll(3)), "the pad is there to press");
    scroll_to(&mut app, Action::KClear);
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Undo"), "{texts:?}");
    assert!(
        texts
            .iter()
            .any(|t| t.contains("No need to write the words down yet")),
        "{texts:?}"
    );
}

#[test]
fn typed_faces_on_the_card_count_as_rolls() {
    let mut app = opened();
    app.press(Action::KWords(12));
    let _ = app.frame();
    app.press(Action::KTyping(true));
    let _ = app.frame();
    type_text(&mut app, "2342");
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.keygen().unwrap().dice.len(), 4);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("4 of 70")),
        "the count: {texts:?}"
    );
}

/// The card opened on a display `width` x `height` at `dpi`.
fn opened_on(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app.press(Action::CNext(cstep::KIND));
    app
}

#[test]
fn on_a_wide_window_the_cards_rolls_start_typed_and_digits_land_as_rolls() {
    let mut app = opened_on(1280, 800, 96);
    app.press(Action::KWords(12));
    let _ = app.frame();
    type_text(&mut app, "2342");
    assert_eq!(app.keygen().unwrap().dice.len(), 4);
}

#[test]
fn a_new_keys_coin_flips_open_as_before_on_a_wide_window_and_the_pis_panel() {
    use faraday_core::keygen::{Start, Way};
    for (w, h, dpi, typed) in [(1280, 800, 96, true), (1024, 600, 170, false)] {
        let mut app = testkit::started();
        app.event(Event::Display(DisplayInfo {
            width: w,
            height: h,
            dpi,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: SecureHardware::None,
            boot: BootState::Unknown,
            memory_mib: None,
        }));
        let _ = app.frame();
        app.press(Action::Entry(None));
        app.press(Action::KeyGen(None));
        app.press(Action::KStart(Start::Make.index()));
        app.press(Action::KWords(12));
        app.press(Action::KNext);
        app.press(Action::KWay(Way::Coins.index()));
        app.press(Action::KNext);
        assert_eq!(app.keygen().unwrap().typing, typed, "{w}x{h}");
    }
}

#[test]
fn on_the_pis_panel_the_cards_rolls_start_on_buttons() {
    let mut app = opened_on(1024, 600, 170);
    app.press(Action::KWords(12));
    let _ = app.frame();
    assert!(!app.keygen().unwrap().typing, "Buttons");
    assert!(app.offers(Action::KRoll(3)), "the pad is there to press");
    app.press(Action::KRoll(3));
    assert_eq!(app.keygen().unwrap().dice.len(), 1);
}

#[test]
fn undo_takes_one_roll_back_and_clear_all_of_them() {
    let mut app = opened();
    roll_high(&mut app, 5);
    scroll_to(&mut app, Action::KClear);
    assert!(app.offers(Action::KUndo));
    app.press(Action::KUndo);
    assert_eq!(app.keygen().unwrap().dice.len(), 4);
    app.press(Action::KClear);
    assert_eq!(app.keygen().unwrap().dice.len(), 0);
}

#[test]
fn a_key_made_through_other_ways_fills_the_slot_and_a_single_key_opens_check() {
    let mut app = opened();
    app.press(Action::KeyOtherWays(0));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 5 + i / 3) % 2 == 0));
    }
    app.press(Action::KNext);
    for _ in 0..3 {
        if app.screen != Screen::KeyGen {
            break;
        }
        app.press(Action::KNext);
    }
    assert_eq!(
        app.screen,
        Screen::Create,
        "back in Make a new wallet: {:?}",
        app.keygen().map(|k| k.open)
    );
    let c = app.create().unwrap();
    assert_ne!(c.slots[0], Source::Empty, "the slot is filled");
    assert_eq!(c.open, Some(cstep::CHECK));
}
