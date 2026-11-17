//! A single-key spend whose key is not loaded: the Sign card offers the
//! ways to load it, typing the words loads it and the card signs, a key
//! that is not the wallet's is refused, a vault opened from the card comes
//! back to the backup flow with the key loaded, and a key already held is
//! not asked for.

use faraday_core::spend_backup::SpendBackupAction as F;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_files(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
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
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn spending_files() -> Vec<(String, Vec<u8>)> {
    let k = kit("spending");
    let (u, _) = testkit::spend_of(&k).unwrap();
    vec![
        ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ("spending-unsigned.psbt".to_string(), u.to_bytes()),
    ]
}

fn on_the_sign_card() -> Faraday {
    let mut app = with_files(spending_files());
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    assert!(
        app.spend().is_some(),
        "the PSBT in Files is the one to sign"
    );
    app.press(Action::Step(step::SIGN));
    let _ = app.frame();
    app
}

fn type_words(app: &mut Faraday, n: usize) {
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

#[test]
fn the_sign_card_without_the_key_offers_the_ways_to_load_it() {
    let mut app = on_the_sign_card();
    assert!(app.offers(Action::EntryWallet(0)), "Type the words");
    assert!(app.offers(Action::ScanKey(0)), "Scan a SeedQR");
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Type the words"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Scan a SeedQR"), "{texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with("Collect")), "{texts:?}");
}

#[test]
fn typing_the_words_loads_the_key_and_the_card_signs() {
    let mut app = on_the_sign_card();
    app.press(Action::EntryWallet(0));
    assert_eq!(app.screen, Screen::Entry);
    type_words(&mut app, 0);
    assert_eq!(app.screen, Screen::SpendBackup);
    let _ = app.frame();
    assert!(app.offers(Action::SignHere), "Sign with the key");
    assert!(!app.offers(Action::EntryWallet(0)));
    app.press(Action::SignHere);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Load the key"), "{texts:?}");
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
}

#[test]
fn a_key_already_held_is_not_asked_for_on_the_sign_card() {
    let mut app = on_the_sign_card();
    app.press(Action::EntryWallet(0));
    type_words(&mut app, 0);
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
    let _ = app.frame();
    assert!(!app.offers(Action::EntryWallet(0)));
    assert!(!app.offers(Action::ScanKey(0)));
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Type the words"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Scan a SeedQR"), "{texts:?}");
}

#[test]
fn opening_a_vault_from_the_card_comes_back_to_the_backup_flow_with_the_key() {
    let mut files = spending_files();
    files.push(("vault.kdbx".to_string(), testkit::test_vault().unwrap()));
    let mut app = with_files(files);
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    app.press(Action::Step(step::SIGN));
    let _ = app.frame();
    let open = Action::Vault(V::OpenFrom(0, Screen::SpendBackup));
    assert!(app.offers(open), "Open the vault");
    app.press(open);
    assert_eq!(app.screen, Screen::Unlock);
    for c in testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.screen, Screen::SpendBackup, "unlocking comes back");
    let _ = app.frame();
    let fp = app.session.slots(&app.session.wallets[0])[0]
        .fingerprint
        .unwrap();
    let load = Action::Vault(V::LoadKeyOf(
        fp.0,
        faraday_core::vault::SlotOf::Loaded(0, 0),
    ));
    assert!(app.offers(load), "Load from the vault");
    app.press(load);
    let _ = app.frame();
    let held = app.session.slots(&app.session.wallets[0])[0]
        .held_by
        .is_some();
    assert!(held, "the wallet's key is loaded");
    assert!(app.offers(Action::SignHere), "Sign with the key");
}

#[test]
fn another_keys_words_are_refused_and_the_key_stays_unloaded() {
    let mut app = on_the_sign_card();
    app.press(Action::EntryWallet(0));
    type_words(&mut app, 1);
    assert_eq!(app.screen, Screen::Entry);
    app.press(Action::Nav(Screen::SpendBackup));
    let _ = app.frame();
    assert!(!app.offers(Action::SignHere));
}
