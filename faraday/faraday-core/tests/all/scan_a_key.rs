//! Scan a key (`docs/history/SCAN-TO-SIGN.md` §2): a wallet short of keys offers
//! one Scan a key, which takes any of its missing keys and works out
//! which; a match loads at once with no passphrase question, the camera
//! staying on while more are needed; a seed of no key of the wallet asks
//! for its passphrase; the words can be typed instead; and a wallet that
//! can spend says so on its card and in the list.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Fingerprint;
use faraday_core::seeds::read::Choice;
use faraday_core::testkit;
use faraday_core::wallet::fp_text;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// Answers the app's reads from the test vault, the one file on the
/// boot stick.
fn pump(app: &mut Faraday, vault: &[u8]) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: vault.to_vec(),
            });
        }
    }
}

/// Started from the boot stick, its vault unlocked and Load ticked: every
/// test wallet loaded, with Test key 1 here.
fn unlocked() -> Faraday {
    let vault = testkit::test_vault().unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: vec![("vault.ofv".into(), vault.len() as u64)],
    }]));
    pump(&mut app, &vault);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app.press(Action::Import(faraday_core::boot_import::ImportAction::Go));
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Import(
        faraday_core::boot_import::ImportAction::Submit,
    ));
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
    let _ = app.frame();
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    // Its Load sheet, everything ticked (F62).
    app.press(Action::Vault(faraday_core::vault::VaultAction::LoadTicked));
    app
}

/// Unlocked, on the 2-of-3 Savings wallet's card, Test key 1 here.
fn on_savings() -> (Faraday, usize) {
    let mut app = unlocked();
    let w = app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Savings")
        .unwrap();
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    (app, w)
}

/// The texts on screen once what moves has come to rest.
fn drawn(app: &mut Faraday) -> String {
    let _ = app.frame();
    app.settle();
    let _ = app.frame();
    app.drawn_texts().join("\n")
}

fn fingerprint(word: &str) -> Fingerprint {
    let c = testkit::compute();
    let mut s = testkit::session();
    s.keys
        .add_words(&c, &testkit::test_words(word), "", None)
        .unwrap()
}

/// The place of the key `word` makes among wallet `w`'s, from one.
fn key_number(app: &Faraday, w: usize, word: &str) -> usize {
    let fp = fingerprint(word);
    app.session
        .slots(&app.session.wallets[w])
        .iter()
        .position(|s| s.fingerprint == Some(fp))
        .unwrap()
        + 1
}

/// The camera reads these words' SeedQR.
fn scan_seedqr(app: &mut Faraday, words: &str) {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, words).unwrap();
    app.event(Event::Scanned {
        bytes: faraday_codec::seedqr::to_digits(&m)
            .expose()
            .as_bytes()
            .to_vec(),
    });
    let _ = app.frame();
}

fn camera_on(app: &Faraday) -> bool {
    app.sheet == Some(Sheet::Scan) && app.scan.is_some()
}

fn note(app: &Faraday) -> String {
    app.scan
        .as_ref()
        .and_then(|s| s.note.clone())
        .unwrap_or_default()
}

/// The Savings wallet alone loaded, with none of its keys.
fn savings_only() -> (Faraday, usize) {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![("savings.txt".to_string(), testkit::savings().into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    let _ = app.frame();
    (app, 0)
}

const OTHER: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                     abandon abandon about";

#[test]
fn a_wallet_short_of_keys_offers_one_scan_a_key_and_it_turns_the_camera_on() {
    let (mut app, w) = on_savings();
    let text = drawn(&mut app);
    assert!(text.contains("1 of 2 signatures possible here"), "{text}");
    assert!(text.contains("Scan a key"), "{text}");
    assert!(!text.contains("Add its key"), "{text}");
    assert!(app.offers(Action::ScanKey(w)));
    app.press(Action::ScanKey(w));
    assert!(camera_on(&app), "the camera did not turn on");
    let text = drawn(&mut app);
    assert!(text.contains("Scan a key for Savings"), "{text}");
    let two = fp_text(fingerprint("zebra"));
    assert!(
        text.contains(&format!("Key {} · {two}", key_number(&app, w, "zebra"))),
        "{text}"
    );
    assert!(app.offers(Action::EntryWallet(w)), "Type the words instead");
}

#[test]
fn a_missing_keys_seedqr_loads_at_once_and_the_wallet_can_spend() {
    let (mut app, w) = on_savings();
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("zebra"));
    assert_ne!(
        app.sheet,
        Some(Sheet::SeedRead),
        "it asked for a passphrase"
    );
    assert!(!camera_on(&app), "the camera stayed on");
    assert_eq!(app.session.keys.len(), 2);
    let n = key_number(&app, w, "zebra");
    assert_eq!(
        app.toast_text(),
        Some(format!("Key {n} added · Can spend").as_str())
    );
    let text = drawn(&mut app);
    assert!(text.contains("Can spend"), "{text}");
    assert!(!app.offers(Action::ScanKey(w)), "Scan a key still offered");
    // The list's row says so, its sub-line the shape alone.
    assert!(text.contains("2 of 3 · native SegWit multisig"), "{text}");
    assert!(
        !text.contains("2 of 3 · native SegWit multisig · 2 of"),
        "{text}"
    );
}

#[test]
fn a_key_already_here_is_named_and_the_camera_stays_on() {
    let (mut app, w) = on_savings();
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("bacon"));
    let n = key_number(&app, w, "bacon");
    assert_eq!(note(&app), format!("Key {n} · already here"));
    assert!(camera_on(&app));
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_seed_of_no_key_of_the_wallet_asks_its_passphrase_and_refuses_it_without_one() {
    let c = testkit::compute();
    let (mut app, w) = on_savings();
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, OTHER);
    assert!(!camera_on(&app));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    let mut probe = testkit::session();
    let fp = fp_text(probe.keys.add_words(&c, OTHER, "", None).unwrap());
    let line = format!("{fp} is not one of Savings's keys");
    let text = drawn(&mut app);
    assert!(text.contains(&line), "{text}");
    app.press(Action::SeedRead(Choice::Plain));
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "the sheet closed");
    assert_eq!(app.session.keys.len(), 1, "a key of no slot loaded");
    let text = drawn(&mut app);
    assert!(text.contains(&line), "{text}");
}

#[test]
fn with_two_keys_missing_the_first_match_leaves_the_camera_on_for_one_more() {
    let (mut app, w) = savings_only();
    assert!(app.offers(Action::ScanKey(w)));
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("bacon"));
    let n = key_number(&app, w, "bacon");
    assert_eq!(note(&app), format!("Key {n} added · 1 more"));
    assert!(camera_on(&app));
    scan_seedqr(&mut app, &testkit::test_words("summer"));
    assert!(!camera_on(&app));
    assert!(app.can_spend(w));
}

#[test]
fn the_words_typed_instead_load_any_missing_key_and_refuse_another_seed() {
    let c = testkit::compute();
    let (mut app, w) = on_savings();
    app.press(Action::ScanKey(w));
    let _ = app.frame();
    app.press(Action::EntryWallet(w));
    assert!(!camera_on(&app));
    assert_eq!(app.screen, Screen::Entry);
    let text = drawn(&mut app);
    assert!(text.contains("Add a key for Savings"), "{text}");
    assert!(
        app.offers(Action::ScanKey(w)),
        "its Scan scans for the wallet"
    );
    // Another seed's words.
    type_text(&mut app, OTHER);
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Entry);
    let mut probe = testkit::session();
    let fp = fp_text(probe.keys.add_words(&c, OTHER, "", None).unwrap());
    assert_eq!(
        app.entry.error.as_deref(),
        Some(format!("These words give {fp}, not a key of Savings").as_str())
    );
    assert_eq!(app.session.keys.len(), 1);
    // Test key 3's.
    app.press(Action::EntryClear);
    type_text(&mut app, &testkit::test_words("summer"));
    app.press(Action::EntryAdd);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.session.keys.len(), 2);
    assert!(app.can_spend(w));
}

#[test]
fn with_a_stick_attached_scan_a_key_asks_for_it_to_be_pulled() {
    let (mut app, w) = on_savings();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "s".into(),
        label: "STICK".into(),
        boot: false,
        files: Vec::new(),
    }]));
    let _ = app.frame();
    app.press(Action::ScanKey(w));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert!(app.scan.is_none(), "the camera turned on");
}

#[test]
fn the_wallets_column_offers_scan_a_key_for_the_wallet_while_it_is_short() {
    let (mut app, w) = savings_only();
    let text = drawn(&mut app);
    assert!(text.contains("Scan a key for Savings"), "{text}");
    app.press(Action::ScanKey(w));
    scan_seedqr(&mut app, &testkit::test_words("bacon"));
    scan_seedqr(&mut app, &testkit::test_words("zebra"));
    let text = drawn(&mut app);
    assert!(!text.contains("Scan a key for Savings"), "{text}");
    assert!(app.offers(Action::KeyGen(None)), "Add a key");
}

#[test]
fn add_a_key_offers_scan_a_seedqr_on_start() {
    use faraday_core::keygen::Start;
    let (mut app, _) = on_savings();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    assert!(app.offers(Action::KStart(Start::Scan.index())));
    app.press(Action::KStart(Start::Scan.index()));
    app.press(Action::KNext);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));
}
