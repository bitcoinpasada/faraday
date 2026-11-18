//! The keys of a wallet of several keys say what vouches for each: a
//! loaded key gives it, the person ticked it, or nothing does; the
//! confirmation sheet counts the keys not checked and takes the ticks.

use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::{KeyBlockOf, KeyCheck};
use faraday_core::{Action, Faraday, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

use crate::common::texts;

fn multisig() -> Kit {
    let p = "m/48'/1'/0'/0'";
    Kit {
        id: "ticks",
        name: "Ticked",
        descriptor: format!(
            "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(0, p),
            testkit::key(1, p),
            testkit::key(2, p)
        ),
        cosigner: None,
        fee: 1_820,
    }
}

fn app_with_wallet_file() -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "a-wallet.txt".to_string(),
            multisig().descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn shown(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

fn tick(app: &mut Faraday, key: u8) {
    app.press(Action::KeyTick(KeyBlockOf::Asked, key));
}

#[test]
fn a_loaded_key_is_yours_and_the_line_counts_the_others() {
    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    let s = shown(&mut app);
    assert!(s.contains("Yours, can sign"), "{s}");
    assert!(s.contains("Not checked"), "{s}");
    assert!(s.contains("2 keys not checked"), "{s}");
}

#[test]
fn every_key_checked_reads_only_once_no_key_is_unchecked() {
    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    app.press(Action::LoadWallet(0));
    tick(&mut app, 1);
    let s = shown(&mut app);
    assert!(s.contains("1 key not checked"), "{s}");
    assert!(s.contains("Checked by you"), "{s}");
    assert!(!s.contains("Every key checked"), "{s}");
    tick(&mut app, 2);
    let s = shown(&mut app);
    assert!(s.contains("Every key checked"), "{s}");
    tick(&mut app, 1);
    let s = shown(&mut app);
    assert!(s.contains("1 key not checked"), "{s}");
    assert!(!s.contains("Every key checked"), "{s}");
    // A key a loaded key gives is not the person's to tick.
    tick(&mut app, 0);
    let s = shown(&mut app);
    assert!(s.contains("1 key not checked"), "{s}");
}

#[test]
fn the_hold_adds_the_wallet_and_the_toast_says_how_many_keys_are_not_checked() {
    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    app.press(Action::InboxLoad);
    tick(&mut app, 1);
    app.press(Action::WalletHold);
    assert_eq!(app.session.wallets.len(), 1);
    let toast = app.toast_text().unwrap();
    assert!(toast.contains("1 key not checked"), "{toast}");
    assert_eq!(app.session.wallets[0].keys_unchecked(), 1);

    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    app.press(Action::InboxLoad);
    app.press(Action::WalletHold);
    let toast = app.toast_text().unwrap();
    assert!(toast.contains("2 keys not checked"), "{toast}");

    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    app.press(Action::InboxLoad);
    tick(&mut app, 1);
    tick(&mut app, 2);
    app.press(Action::WalletHold);
    let toast = app.toast_text().unwrap();
    assert!(toast.ends_with("added"), "{toast}");
    assert_eq!(app.session.wallets[0].keys_unchecked(), 0);
}

#[test]
fn a_seed_loaded_later_makes_its_key_yours_and_the_ticks_stay() {
    let mut app = app_with_wallet_file();
    app.press(Action::LoadWallet(0));
    tick(&mut app, 1);
    app.press(Action::WalletHold);
    assert_eq!(
        app.session.wallets[0].key_checks(),
        [KeyCheck::Unchecked, KeyCheck::Checked, KeyCheck::Unchecked]
    );
    add_key(&mut app, 0);
    let w = &app.session.wallets[0];
    assert_eq!(
        w.key_checks(),
        [KeyCheck::Yours, KeyCheck::Checked, KeyCheck::Unchecked]
    );
    assert_eq!(w.key_ticks(), [false, true, false]);
    // The person's tick of an unchecked key is kept apart.
    app.session.tick_key(0, 2);
    app.session.tick_key(0, 0);
    let w = &app.session.wallets[0];
    assert_eq!(w.key_ticks(), [false, true, true]);
    assert_eq!(w.keys_unchecked(), 0);
}

fn last_fingerprint(app: &Faraday) -> faraday_bip::keys::Fingerprint {
    app.session.keys.last().unwrap().fingerprint
}

#[test]
fn a_key_made_here_reads_signs_after_its_backup_until_the_backup_is_done() {
    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    let fp = last_fingerprint(&app);
    testkit::mark_made_here(&mut app, fp);
    app.press(Action::LoadWallet(0));
    let s = shown(&mut app);
    assert!(s.contains("Yours, signs after its backup"), "{s}");
    assert!(!s.contains("Yours, can sign"), "{s}");
    assert!(s.contains("2 keys not checked"), "{s}");
    testkit::release_made(&mut app, fp);
    tick(&mut app, 1);
    let s = shown(&mut app);
    assert!(s.contains("Yours, can sign"), "{s}");
}

#[test]
fn a_key_loaded_from_words_reads_can_sign_in_a_wallet_beside_a_key_made_here() {
    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    let made = last_fingerprint(&app);
    testkit::mark_made_here(&mut app, made);
    app.press(Action::LoadWallet(0));
    let s = shown(&mut app);
    assert!(s.contains("Yours, can sign"), "{s}");
    assert!(s.contains("Yours, signs after its backup"), "{s}");
    assert!(s.contains("1 key not checked"), "{s}");
}

/// The same wallet as `multisig()` with its keys listed in the other order.
fn multisig_reversed() -> String {
    let p = "m/48'/1'/0'/0'";
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(2, p),
        testkit::key(1, p),
        testkit::key(0, p)
    )
}

#[test]
fn a_tick_on_a_sheet_for_the_loaded_wallet_in_another_order_lands_on_the_same_key() {
    let mut app = app_with_wallet_file();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                "a-wallet.txt".to_string(),
                multisig().descriptor.into_bytes(),
            ),
            ("b-wallet.txt".to_string(), multisig_reversed().into_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    // The sheet asks for the keys last first; while it waits, the same wallet
    // arrives written in the other order, with a tick of its own.
    app.press(Action::LoadWallet(1));
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    app.session
        .add_wallet_confirmed(
            &testkit::compute(),
            "Ticked",
            &multisig().descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.session.tick_key(0, 1);
    let third = app.session.wallets[0].policy().keys()[2].fingerprint();
    // The sheet's first key is the loaded wallet's third.
    tick(&mut app, 0);
    app.press(Action::WalletHold);
    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert_eq!(w.policy().keys()[2].fingerprint(), third);
    assert_eq!(w.key_ticks(), [false, true, true]);
    assert_eq!(
        w.key_checks(),
        [KeyCheck::Unchecked, KeyCheck::Checked, KeyCheck::Checked]
    );
}

#[test]
fn a_tick_press_for_a_place_after_its_wallet_was_taken_out_changes_no_wallet() {
    let mut app = testkit::started();
    let c = testkit::compute();
    let p = "m/48'/1'/0'/0'";
    let own = testkit::key(0, p);
    let forged_key = format!(
        "{}]{}",
        own.split_once(']').unwrap().0,
        testkit::key(1, p).split_once(']').unwrap().1
    );
    let forged = format!(
        "wsh(sortedmulti(2,{forged_key}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(2, p)
    );
    let real = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(1, p),
        testkit::key(2, p)
    );
    for (name, d) in [("Forged", &forged), ("Real", &real)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "test", faraday_core::wallet::Origin::Named)
            .expect("no key is loaded: not checked");
    }
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    // The forged wallet is taken out: the press drawn at its place, with no
    // frame since, is not for the wallet that moved up into it.
    app.press(Action::Back);
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::KeyTick(KeyBlockOf::Wallet(0), 0));
    assert_eq!(app.session.wallets[0].key_ticks(), [false, false]);
}

#[test]
fn a_tick_press_for_a_place_drawn_before_remove_wallet_changes_no_wallet() {
    let mut app = testkit::started();
    let c = testkit::compute();
    let p = "m/48'/1'/0'/0'";
    for (name, a, b) in [("First", 0, 1), ("Second", 1, 2)] {
        let d = format!(
            "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(a, p),
            testkit::key(b, p)
        );
        app.session
            .add_wallet_confirmed(&c, name, &d, "test", faraday_core::wallet::Origin::Named)
            .expect("no key is loaded: not checked");
    }
    // The first wallet is removed: the press drawn at its place, with no
    // frame since, is not for the wallet that moved up into it.
    app.press(Action::RemoveWallet(0));
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::KeyTick(KeyBlockOf::Wallet(0), 0));
    assert_eq!(app.session.wallets[0].key_ticks(), [false, false]);
}

#[test]
fn a_key_made_here_reads_what_is_true_once_it_is_removed_and_loaded_from_its_words() {
    let mut app = app_with_wallet_file();
    add_key(&mut app, 0);
    let fp = last_fingerprint(&app);
    testkit::mark_made_here(&mut app, fp);
    app.press(Action::InboxLoad);
    app.press(Action::WalletHold);
    app.press(Action::OpenWallet(0));
    let s = shown(&mut app);
    assert!(s.contains("Key 1 · 9a6a2580 Waiting for its backup"), "{s}");
    app.press(Action::RemoveKeys);
    app.press(Action::RemoveKeysHold);
    app.press(Action::OpenWallet(0));
    let s = shown(&mut app);
    assert!(!s.contains("Waiting for its backup"), "{s}");
    // Its words typed again are still a key whose backup is not done, until it is.
    add_key(&mut app, 0);
    app.press(Action::OpenWallet(0));
    assert!(shown(&mut app).contains("Key 1 · 9a6a2580 Waiting for its backup"));
    testkit::release_made(&mut app, fp);
    app.press(Action::OpenWallet(0));
    let s = shown(&mut app);
    assert!(s.contains("Key 1 · 9a6a2580 Can sign here"), "{s}");
    assert!(!s.contains("Waiting for its backup"), "{s}");
}
