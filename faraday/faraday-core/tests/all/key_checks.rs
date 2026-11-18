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
