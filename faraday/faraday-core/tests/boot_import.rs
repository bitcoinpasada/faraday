//! The boot import (F60): the first look at the boot stick copies its
//! files into memory, nothing in Files, and the sheet lists every one
//! ticked, a seed's too. Pulling the stick changes nothing. Import brings
//! in what is ticked, its wallets and keys with it, and the vaults into
//! Files, locked, each then offered its Unlock with Cancel, an unlock
//! followed by its Load sheet (F62); Later brings in nothing, and Home
//! leads with the import. A lock drops it.

use faraday_core::boot_import::{self, ImportAction as I};
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
use osk_shell_api::{App, Event, Key, TouchPhase};

const BOOT: &str = "boot";

/// A single-key wallet over test key 2, which the vault does not hold.
fn zebra_wallet() -> String {
    format!("wpkh({}/<0;1>/*)", testkit::key(1, "m/84'/1'/0'"))
}

/// Test seed `n`'s master fingerprint, as the app shows it.
fn fp(n: usize) -> String {
    let k = testkit::key(n, "m/84'/1'/0'");
    k[1..9].to_string()
}

/// Test seed `n`'s SeedQR: each word's index as four digits.
fn seed_seedqr(n: usize) -> Vec<u8> {
    let m = osk_bip::bip39::Mnemonic::parse(
        osk_bip::bip39::Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[n].0),
    )
    .unwrap();
    m.indices()
        .iter()
        .map(|i| format!("{i:04}"))
        .collect::<String>()
        .into_bytes()
}

/// Test key 2's SeedQR: each word's index as four digits.
fn zebra_seedqr() -> Vec<u8> {
    seed_seedqr(1)
}

/// The boot stick's files: the test vault, a wallet whose key is in a
/// picture, a key no wallet on the stick uses, a PSBT, a note and a photo.
fn stick_files() -> Vec<(String, Vec<u8>)> {
    let psbt = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "savings-unsigned.psbt")
        .unwrap();
    vec![
        ("notes.txt".into(), b"Shopping list\nmilk\nbread\n".to_vec()),
        ("photo.jpg".into(), vec![0xff, 0xd8, 0xff, 0xe0]),
        psbt,
        (
            "summer-words.txt".into(),
            testkit::test_words(testkit::TEST_SEEDS[2].0).into_bytes(),
        ),
        ("vault.ofv".into(), testkit::test_vault().unwrap()),
        ("zebra-seedqr.png".into(), b"a picture".to_vec()),
        (
            "zebra-wallet.txt".into(),
            format!("{}\n", zebra_wallet()).into_bytes(),
        ),
    ]
}

fn boot_stick(files: &[(String, Vec<u8>)]) -> StickInfo {
    StickInfo {
        serial: None,
        id: BOOT.into(),
        label: "FARADAY".into(),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }
}

/// Answers the app's reads from the stick's files, and its picture's
/// codes with test key 2's SeedQR.
fn pump(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
                app.storage(StorageEvent::Read { stick, name, bytes });
            }
            StorageCommand::ReadQr { name, .. } => {
                app.storage(StorageEvent::QrRead {
                    name,
                    bytes: Vec::new(),
                    payloads: vec![zebra_seedqr()],
                });
            }
            _ => {}
        }
    }
}

/// A fresh process with a boot stick holding `files` in.
fn booted_with(files: &[(String, Vec<u8>)]) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Sticks(vec![boot_stick(files)]));
    pump(&mut app, files);
    app
}

/// A fresh process with the boot stick in.
fn booted() -> Faraday {
    booted_with(&stick_files())
}

/// Booted, and the stick pulled.
fn pulled() -> Faraday {
    let mut app = booted();
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app
}

/// Types `passphrase` into the field typing goes to.
fn type_in(app: &mut Faraday, passphrase: &str) {
    for c in passphrase.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Lets the unlock asked for run: a frame, then the tick after it.
fn run_unlock(app: &mut Faraday) {
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
    let _ = app.frame();
}

/// Pulled, Import pressed, put off with Later, and the vault unlocked on
/// Vaults.
fn unlocked_elsewhere() -> Faraday {
    let mut app = pulled();
    app.press(Action::Import(I::Go));
    app.press(Action::Import(I::Later));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    run_unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app
}

/// A second vault, of its own salt: one slot with one entry.
fn spare_vault() -> Vec<u8> {
    use faraday_vault::records::{field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let slot = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Spare"),
            Record::new(kind::ENTRY)
                .with(field::TITLE, b"Locker")
                .with(field::PASSWORD, b"4321"),
        ],
    };
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 1,
        lanes: 1,
    };
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[b"spare".as_slice()],
        &[slot],
        &[0x5b; 32],
    )
    .unwrap()
}

fn drawn(app: &mut Faraday) -> String {
    app.drawn_texts().join("\n")
}

fn file<'a>(view: &'a boot_import::ImportView, name: &str) -> &'a boot_import::ImportFile {
    view.files.iter().find(|f| f.name == name).unwrap()
}

/// The place of `name` in the sheet's list.
fn row(app: &Faraday, name: &str) -> usize {
    let view = app.import_view().unwrap();
    view.files.iter().position(|f| f.name == name).unwrap()
}

fn inbox_names(app: &Faraday) -> Vec<String> {
    let mut v: Vec<String> = app.inbox.iter().map(|i| i.name.clone()).collect();
    v.sort();
    v
}

fn key_fps(app: &Faraday) -> Vec<String> {
    let mut v: Vec<String> = app
        .session
        .keys
        .iter()
        .map(|k| faraday_core::wallet::fp_text(k.fingerprint()))
        .collect();
    v.sort();
    v
}

#[test]
fn the_boot_stick_is_copied_into_memory_and_nothing_is_in_files_until_import() {
    let mut app = booted();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.sheet, Some(Sheet::Import));
    assert!(app.inbox.is_empty(), "{:?}", inbox_names(&app));
    let c = app.import_count().unwrap();
    assert_eq!((c.copied, c.not_read, c.vaults), (7, 0, 1));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Later)));
    assert!(app.offers(Action::Import(I::Go)));
    assert!(app.session.keys.is_empty() && app.session.wallets.is_empty());
}

#[test]
fn with_the_stick_in_the_sheet_lists_every_file_ticked_a_seeds_too() {
    let mut app = booted();
    let texts = drawn(&mut app);
    assert!(texts.contains("Read from Boot stick"), "{texts}");
    assert!(
        texts.contains("1 vault · 1 PSBT · 5 other files"),
        "{texts}"
    );
    let view = app.import_view().unwrap();
    for (name, _) in stick_files() {
        assert!(texts.contains(&name), "{name} is not listed: {texts}");
        let f = file(&view, &name);
        assert!(f.chosen && f.enabled, "{name} is not ticked");
    }
    assert!(!app.offers(Action::Import(I::Submit)));
}

#[test]
fn pulling_the_stick_before_choosing_keeps_the_sheet_and_brings_in_nothing() {
    let mut app = pulled();
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert!(
        app.import_field().is_none(),
        "a passphrase field before Import"
    );
    assert!(app.inbox.is_empty() && app.session.keys.is_empty());
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Go)));
    assert!(app.offers(Action::Import(I::Later)));
}

#[test]
fn import_brings_in_every_ticked_file_then_the_vaults_unlock_loads_its_wallets() {
    let mut app = pulled();
    app.press(Action::Import(I::Go));
    // The files From the stick, the picture as what its code held, and
    // the vault, locked.
    assert_eq!(
        inbox_names(&app),
        [
            "notes.txt",
            "photo.jpg",
            "savings-unsigned.psbt",
            "summer-words.txt",
            "vault.ofv",
            "zebra-seedqr-words.txt",
            "zebra-wallet.txt"
        ]
    );
    assert!(app.vaults.open.is_empty());
    // Their wallet and keys: Zebra, test keys 2 and 3.
    assert!(app.session.wallets.iter().any(|w| w.name == "Zebra"));
    let mut want = vec![fp(1), fp(2)];
    want.sort();
    assert_eq!(key_fps(&app), want);
    // The vault's Unlock, on the sheet, typing going to it.
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert_eq!(app.import_field().map(|v| v.name), Some("vault.ofv".into()));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Submit)));
    assert!(app.offers(Action::Import(I::Cancel)));
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    // Its Load sheet (F62), everything ticked; Load ticked ends the
    // import and lands on Wallets.
    assert_eq!(
        (app.screen, app.sheet),
        (Screen::Home, Some(Sheet::UnlockLoad))
    );
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.import.is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Wallets, None));
    for name in ["Savings", "Spending", "Zebra"] {
        assert!(
            app.session.wallets.iter().any(|w| w.name == name),
            "{name} is not loaded"
        );
    }
    let mut want = vec![fp(0), fp(1), fp(2)];
    want.sort();
    assert_eq!(key_fps(&app), want);
}

#[test]
fn a_seed_imported_from_its_file_and_held_by_the_vault_loads_once() {
    let mut files = stick_files();
    files.push((
        "bacon-words.txt".into(),
        testkit::test_words(testkit::TEST_SEEDS[0].0).into_bytes(),
    ));
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Go));
    assert!(key_fps(&app).contains(&fp(0)), "the words file's key");
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    assert_eq!(app.vaults.unlock_error, None);
    // The vault's copy of the key is Loaded on its Load sheet.
    app.press(Action::Vault(V::LoadTicked));
    let mut want = vec![fp(0), fp(1), fp(2)];
    want.sort();
    // Once each: what Wallets and Sign list.
    assert_eq!(key_fps(&app), want, "a key loaded twice");
}

#[test]
fn a_wrong_passphrase_stays_on_the_sheet_and_says_so() {
    let mut app = pulled();
    app.press(Action::Import(I::Go));
    type_in(&mut app, "not the passphrase");
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert!(app.vaults.open.is_empty());
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert!(app.import.is_some());
    let error = app.vaults.unlock_error.clone().expect("no error said");
    assert!(
        drawn(&mut app).contains(&error),
        "the error is not on the sheet"
    );
    // And the right one, typed again, unlocks: its Load sheet, then the
    // import is over.
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Vault(V::LoadNotNow));
    assert!(app.import.is_none() && app.vaults.open.len() == 1);
}

#[test]
fn import_with_the_stick_in_waits_for_the_pull_to_unlock() {
    let mut app = booted();
    app.press(Action::Import(I::Go));
    assert!(app.inbox.iter().any(|i| i.name == "notes.txt"));
    assert!(
        app.session.keys.is_empty(),
        "a key loaded with the stick in"
    );
    let texts = drawn(&mut app);
    assert!(texts.contains("Pull the stick to continue"), "{texts}");
    assert!(!app.offers(Action::Import(I::Submit)));
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    // The words file's key loads as the stick goes; the field is ready.
    assert!(key_fps(&app).contains(&fp(2)));
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
}

#[test]
fn the_reminders_before_the_passphrase_leave_the_sheet_where_it_was() {
    let files = stick_files();
    let mut app = testkit::started();
    app.devices =
        faraday_core::devices::DeviceCheck::machine("flags\t: fpu vme", "LENOVO", "20XW", "10");
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app.press(Action::Import(I::Go));
    if app.sheet == Some(Sheet::Devices) {
        app.press(Action::DevicesUnderstood);
    }
    // The first secret typed: the passphrase field. The reminders come
    // first if they have not yet.
    app.press(Action::Vault(V::FocusPassphrase));
    if app.sheet == Some(Sheet::Devices) {
        app.press(Action::DevicesUnderstood);
    }
    assert_eq!(app.sheet, Some(Sheet::Import));
    assert!(app.import_field().is_some());
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.import.is_none() && app.vaults.open.len() == 1);
}

#[test]
fn later_brings_in_nothing_and_home_leads_with_the_import() {
    let mut app = pulled();
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import.is_some());
    assert!(app.inbox.is_empty() && app.session.keys.is_empty());
    let texts = drawn(&mut app);
    assert!(texts.contains("Import from Boot stick"), "{texts}");
    assert!(app.offers(boot_import::OPEN));
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Go)));
}

#[test]
fn later_with_the_stick_in_leaves_the_files_to_the_stick_visit() {
    let files = stick_files();
    let mut app = booted();
    app.press(Action::Import(I::Later));
    assert!(app.inbox.is_empty());
    app.press(Action::Nav(Screen::Visit));
    assert!(app.visit.inn.contains("notes.txt"), "{:?}", app.visit.inn);
    app.press(Action::VisitCopy);
    pump(&mut app, &files);
    assert!(app.inbox.iter().any(|i| i.name == "notes.txt"));
    assert!(app.inbox.iter().any(|i| i.name == "vault.ofv"));
}

#[test]
fn put_off_after_import_home_leads_with_the_vaults_unlock() {
    // No PSBT, whose Sign would lead Home first.
    let files: Vec<(String, Vec<u8>)> = stick_files()
        .into_iter()
        .filter(|(n, _)| !n.ends_with(".psbt"))
        .collect();
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Go));
    type_in(&mut app, "half typ");
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(
        app.vaults.passphrase.text.is_empty(),
        "what was typed stays"
    );
    let texts = drawn(&mut app);
    assert!(texts.contains("Unlock vault.ofv"), "{texts}");
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Submit)));
}

#[test]
fn with_two_vaults_each_is_offered_in_turn_and_cancel_leaves_it_locked() {
    let mut files = stick_files();
    files.push(("spare.ofv".into(), spare_vault()));
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Go));
    assert_eq!(app.import_field().map(|v| v.name), Some("vault.ofv".into()));
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    // Its Load sheet, then the second vault.
    assert_eq!(
        (app.screen, app.sheet),
        (Screen::Home, Some(Sheet::UnlockLoad))
    );
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    // The second, on the sheet in its turn; Cancel shows no Load sheet.
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert_eq!(app.import_field().map(|v| v.name), Some("spare.ofv".into()));
    app.press(Action::Import(I::Cancel));
    assert!(app.import.is_none());
    assert_eq!(app.sheet, None);
    let spare = app
        .vault_files()
        .into_iter()
        .find(|f| f.name == "spare.ofv")
        .expect("the cancelled vault is not in Files");
    assert!(spare.open.is_none());
}

#[test]
fn cancel_leaves_the_only_vault_locked_in_files() {
    let mut app = pulled();
    app.press(Action::Import(I::Go));
    app.press(Action::Import(I::Cancel));
    assert!(app.import.is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    let files = app.vault_files();
    assert_eq!(files.len(), 1);
    assert!(files[0].open.is_none());
}

#[test]
fn with_no_vault_import_brings_the_files_in_and_home_leads_with_the_psbt() {
    let files: Vec<(String, Vec<u8>)> = stick_files()
        .into_iter()
        .filter(|(n, _)| n != "vault.ofv")
        .collect();
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(app.import.is_some() && app.inbox.is_empty());
    app.press(Action::Import(I::Go));
    assert!(app.import.is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    let psbt = app
        .inbox
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .expect("the PSBT did not come in");
    assert!(app.inbox.iter().any(|i| i.name == "notes.txt"));
    assert!(app.session.wallets.iter().any(|w| w.name == "Zebra"));
    // Home leads with signing it.
    let _ = app.frame();
    assert!(app.offers(Action::StartSpend(psbt)));
}

#[test]
fn with_only_the_settings_file_no_sheet_comes_up() {
    let files = vec![(
        faraday_core::stick_settings::FILE.to_string(),
        b"faraday-settings 1\n".to_vec(),
    )];
    let app = booted_with(&files);
    assert!(app.import.is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
}

#[test]
fn wallet_files_with_no_key_here_still_load_as_wallets() {
    let kit = testkit::files().unwrap();
    let files: Vec<(String, Vec<u8>)> = kit
        .into_iter()
        .filter(|(n, _)| n == "savings-wallet.txt" || n == "spending-wallet.txt")
        .collect();
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Go));
    for name in ["Savings", "Spending"] {
        assert!(
            app.session.wallets.iter().any(|w| w.name == name),
            "{name} is not loaded"
        );
    }
    assert!(app.session.keys.is_empty());
}

#[test]
fn an_unticked_file_is_not_imported_and_its_key_does_not_load() {
    let mut app = pulled();
    for name in ["notes.txt", "summer-words.txt"] {
        let k = row(&app, name);
        app.press(Action::Import(I::File(k)));
        assert!(!file(&app.import_view().unwrap(), name).chosen);
    }
    app.press(Action::Import(I::Go));
    assert!(!app.inbox.iter().any(|i| i.name == "notes.txt"));
    assert!(!app.inbox.iter().any(|i| i.name == "summer-words.txt"));
    assert!(app.inbox.iter().any(|i| i.name == "photo.jpg"));
    assert_eq!(key_fps(&app), [fp(1)]);
    assert!(
        !app.inbox
            .iter()
            .any(|i| i.name == "summer-words.txt" && i.kind == FileKind::Words)
    );
}

#[test]
fn an_unticked_vault_stays_out_of_files_and_nothing_asks_to_unlock_it() {
    let mut app = pulled();
    let k = row(&app, "vault.ofv");
    app.press(Action::Import(I::File(k)));
    app.press(Action::Import(I::Go));
    assert!(!app.inbox.iter().any(|i| i.name == "vault.ofv"));
    assert!(app.import.is_none());
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.keys.len(), 2);
}

#[test]
fn a_tap_beside_the_sheet_is_later() {
    let mut app = pulled();
    let _ = app.frame();
    app.event(Event::Touch {
        x: 4,
        y: 4,
        phase: TouchPhase::Down,
    });
    app.event(Event::Touch {
        x: 4,
        y: 4,
        phase: TouchPhase::Up,
    });
    assert_eq!(app.sheet, None);
    assert!(app.import.is_some() && app.inbox.is_empty());
}

#[test]
fn a_lock_drops_the_import_and_the_next_process_visits_the_stick() {
    let mut app = unlocked_elsewhere();
    app.press(Action::Lock);
    assert!(app.import.is_none());
    assert!(app.restart_requested());
    let mut kept = Vec::new();
    let mut inbox = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            kept: k, inbox: i, ..
        } = c
        {
            kept = k;
            inbox = i;
        }
    }
    let mut next = testkit::started();
    next.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept,
    });
    next.storage(StorageEvent::Sticks(vec![boot_stick(&stick_files())]));
    pump(&mut next, &stick_files());
    assert!(next.import.is_none());
    assert_eq!((next.screen, next.sheet), (Screen::Visit, None));
}

#[test]
fn a_picture_on_a_visit_comes_in_ticked_and_has_its_codes_read_into_the_inbox() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "S".into(),
        label: "TESTSTICK".into(),
        boot: false,
        files: vec![("wallet-qr.png".into(), 9000)],
    }]));
    assert_eq!(app.screen, Screen::Visit);
    let _ = app.frame();
    assert!(
        app.offers(Action::VisitIn(0)),
        "the picture cannot be ticked"
    );
    assert!(app.visit.inn.contains("wallet-qr.png"), "it is not ticked");
    app.press(Action::VisitCopy);
    let mut asked = false;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::ReadQr { name, .. } = c {
            asked = true;
            app.storage(StorageEvent::QrRead {
                name,
                bytes: Vec::new(),
                payloads: vec![zebra_wallet().into_bytes()],
            });
        }
    }
    assert!(asked, "its codes were not asked for");
    let item = app
        .inbox
        .iter()
        .find(|i| i.name.starts_with("wallet-qr"))
        .expect("what the picture holds is not in the Inbox");
    assert_eq!(item.kind, FileKind::Wallet);
}

#[test]
fn a_seed_on_the_stick_as_words_and_a_picture_is_labelled_from_the_words_file() {
    let stem = "seed-1-9a6a2580";
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    // The stick's sorted listing puts the picture before the words file.
    let files: Vec<(String, Vec<u8>)> = vec![
        (format!("{stem}-compactseedqr.png"), b"a picture".to_vec()),
        (format!("{stem}-words.txt"), words.into_bytes()),
    ];
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
                app.storage(StorageEvent::Read { stick, name, bytes });
            }
            StorageCommand::ReadQr { name, .. } => {
                app.storage(StorageEvent::QrRead {
                    name,
                    bytes: Vec::new(),
                    payloads: vec![seed_seedqr(0)],
                });
            }
            _ => {}
        }
    }
    // No vault: Import brings it in.
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Go));
    assert!(app.import.is_none());
    let key = app
        .session
        .keys
        .iter()
        .find(|k| faraday_core::wallet::fp_text(k.fingerprint()) == fp(0))
        .expect("the seed did not load");
    assert_eq!(
        key.label,
        format!("{stem}-words"),
        "the picture's label won over the words file's"
    );
}

#[test]
fn a_vault_imported_then_unlocked_on_vaults_loads_its_own_wallets() {
    let mut app = unlocked_elsewhere();
    // Unlocked through the list, not the sheet (`docs/NEW-WALLET.md`
    // §11.1a): its Load sheet; Load ticked lands on Wallets, with the
    // vault's own wallets and keys loaded; the import, its one vault
    // open, is over.
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!((app.screen, app.sheet), (Screen::Wallets, None));
    assert!(app.import.is_none());
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    assert_eq!(app.vaults.back_to, None);
}

/// A USB mouse the shell holds back until a person says so.
fn mouse(app: &mut Faraday) {
    app.storage(StorageEvent::NewInput {
        id: 9,
        name: "USB Optical Mouse".into(),
        keyboard: false,
        pointer: true,
    });
}

#[test]
fn a_mouse_plugged_in_at_boot_is_asked_about_before_the_import() {
    let files = stick_files();
    let mut app = testkit::started();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    mouse(&mut app);
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputUse(9));
    assert_eq!(app.poll_input_decision(), Some((9, true)));
    assert_eq!(app.sheet, Some(Sheet::Import));
}

#[test]
fn a_mouse_seen_after_the_stick_is_asked_about_first_too() {
    let mut app = booted();
    assert_eq!(app.sheet, Some(Sheet::Import));
    mouse(&mut app);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputIgnore(9));
    assert_eq!(app.sheet, Some(Sheet::Import));
}

#[test]
fn a_mouse_waiting_comes_up_when_the_import_is_put_off() {
    let mut app = pulled();
    app.press(Action::Import(I::Later));
    assert_eq!(app.sheet, None);
    mouse(&mut app);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputUse(9));
    // The import was put off, so it stays behind Sticks.
    assert_eq!(app.sheet, None);
}

#[test]
fn a_codex32_seed_imported_with_the_stick_in_loads_when_it_is_pulled() {
    let files: Vec<(String, Vec<u8>)> = vec![
        ("notes.txt".into(), b"Shopping list\n".to_vec()),
        (
            "seed-codex32.txt".into(),
            b"ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw\n".to_vec(),
        ),
    ];
    let mut app = booted_with(&files);
    app.press(Action::Import(I::Go));
    assert!(app.inbox.iter().any(|i| i.name == "seed-codex32.txt"));
    assert!(
        app.session.keys.is_empty(),
        "a key loaded with the stick in"
    );
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.session.keys.len(), 1, "the codex32 seed did not load");
}

#[test]
fn unselect_all_unticks_every_file_and_import_then_waits_for_a_choice() {
    let mut app = pulled();
    let _ = app.frame();
    assert!(drawn(&mut app).contains("Unselect all"));
    app.press(Action::Import(I::All));
    let view = app.import_view().unwrap();
    assert!(view.files.iter().all(|f| !f.chosen));
    let texts = drawn(&mut app);
    assert!(texts.contains("Select all"), "{texts}");
    // Import is disabled with nothing ticked: pressed, it does nothing.
    assert!(!app.offers(Action::Import(I::Go)));
    app.press(Action::Import(I::Go));
    assert_eq!(app.sheet, Some(Sheet::Import));
    assert!(app.import.is_some() && app.inbox.is_empty());
    // Select all ticks them again.
    app.press(Action::Import(I::All));
    let view = app.import_view().unwrap();
    assert!(view.files.iter().filter(|f| f.enabled).all(|f| f.chosen));
}

/// SLIP-39 vector 4, "Basic sharing 2-of-3 (128 bits)": two shares.
const SLIP39_SHARES: [&str; 2] = [
    "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
    "shadow pistol academic acid actress prayer class unknown daughter sweater depict flip twice unkind craft early superior advocate guest smoking",
];

/// BIP 93 vector 1: a codex32 secret, its master seed `318c6318…`.
const CODEX32_SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";

/// BIP 93 vector 2: two shares of a codex32 secret.
const CODEX32_SHARES: [&str; 2] = [
    "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM",
    "MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN",
];

/// The fingerprint test seed 3's words make with `passphrase`.
fn fp_with(passphrase: &str) -> String {
    let mut probe = faraday_core::wallet::Session::default();
    let fp = probe
        .add_words_with(
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            passphrase,
            "",
            None,
        )
        .unwrap();
    faraday_core::wallet::fp_text(fp)
}

/// Every kind of seed file Faraday reads, imported from the boot sheet
/// with the stick in: none loads while it is in, and each loads, once,
/// as the stick is pulled, the moment a BIP-39 words file's does (F60).
#[test]
fn every_kind_of_seed_file_imported_with_the_stick_in_loads_at_the_pull() {
    let words = || testkit::test_words(testkit::TEST_SEEDS[2].0).into_bytes();
    let line = |s: &str| format!("{s}\n").into_bytes();
    type Case = (&'static str, Vec<(String, Vec<u8>)>, Option<String>);
    let cases: Vec<Case> = vec![
        (
            "BIP-39 words",
            vec![("summer-words.txt".into(), words())],
            Some(fp(2)),
        ),
        (
            "BIP-39 words with a passphrase file",
            vec![
                ("summer-words.txt".into(), words()),
                (
                    format!("passphrase-{}.txt", fp_with("orchard")),
                    b"orchard".to_vec(),
                ),
            ],
            Some(fp_with("orchard")),
        ),
        (
            "SeedQR picture",
            vec![("zebra-seedqr.png".into(), b"a picture".to_vec())],
            Some(fp(1)),
        ),
        (
            "SLIP-39 shares",
            vec![
                ("slip39-share-1.txt".into(), line(SLIP39_SHARES[0])),
                ("slip39-share-2.txt".into(), line(SLIP39_SHARES[1])),
            ],
            None,
        ),
        (
            "codex32 secret",
            vec![("codex32-secret.txt".into(), line(CODEX32_SECRET))],
            None,
        ),
        (
            "codex32 shares",
            vec![
                ("codex32-share-a.txt".into(), line(CODEX32_SHARES[0])),
                ("codex32-share-c.txt".into(), line(CODEX32_SHARES[1])),
            ],
            None,
        ),
    ];
    for (kind, files, want) in cases {
        let mut app = booted_with(&files);
        app.press(Action::Import(I::Go));
        for (name, _) in &files {
            assert!(
                app.inbox
                    .iter()
                    .any(|i| i.name.starts_with(&name[..name.len() - 4])),
                "{kind}: {name} is not in Files"
            );
        }
        assert!(
            app.session.keys.is_empty(),
            "{kind}: loaded with the stick in"
        );
        app.storage(StorageEvent::Sticks(Vec::new()));
        assert_eq!(app.session.keys.len(), 1, "{kind}: not loaded at the pull");
        if let Some(want) = want {
            assert_eq!(key_fps(&app), [want], "{kind}");
        }
    }
}

/// A vault holding a master seed, as a SLIP-39 or codex32 key is kept
/// (BIP 93 vector 1's master seed), its key loading at unlock.
fn seed_vault() -> Vec<u8> {
    use faraday_vault::records::{field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let seed = osk_bip::codex32::Codex32::parse(CODEX32_SECRET)
        .unwrap()
        .to_seed()
        .unwrap();
    let seed = seed.expose().bytes();
    let mut payload = vec![0u8; 65];
    payload[0] = seed.len() as u8;
    payload[1..1 + seed.len()].copy_from_slice(seed);
    let slot = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Seeds"),
            Record::new(kind::KEY)
                .with(field::KEY, &payload)
                .with(field::KEY_LABEL, b"codex32"),
        ],
    };
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 1,
        lanes: 1,
    };
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[b"seeds".as_slice()],
        &[slot],
        &[0x5c; 32],
    )
    .unwrap()
}

/// The key a codex32 secret file gives, imported from the boot sheet.
fn codex32_fp() -> String {
    let files = vec![(
        "codex32-secret.txt".to_string(),
        format!("{CODEX32_SECRET}\n").into_bytes(),
    )];
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Go));
    key_fps(&app)
        .pop()
        .expect("the codex32 secret did not load")
}

/// A vault's keys load from its Load sheet whatever they are: BIP-39
/// words (the test vault's test key 1) or a master seed, as a SLIP-39 or
/// codex32 key is kept, which gives the same key as its codex32 file. On
/// the boot sheet and from the Vaults list alike.
#[test]
fn a_vaults_words_and_master_seed_keys_load_on_unlock() {
    let codex32 = codex32_fp();
    let cases: Vec<(&str, Vec<u8>, &str, String)> = vec![
        (
            "BIP-39 words",
            testkit::test_vault().unwrap(),
            testkit::VAULT_PASSPHRASES[0],
            fp(0),
        ),
        ("master seed", seed_vault(), "seeds", codex32),
    ];
    for (kind, vault, phrase, want) in cases {
        let files = vec![("vault.ofv".to_string(), vault)];
        // On the boot sheet.
        let mut app = booted_with(&files);
        app.storage(StorageEvent::Sticks(Vec::new()));
        app.press(Action::Import(I::Go));
        type_in(&mut app, phrase);
        app.press(Action::Import(I::Submit));
        run_unlock(&mut app);
        assert_eq!(
            app.vaults.open.len(),
            1,
            "{kind}: {:?}",
            app.vaults.unlock_error
        );
        app.press(Action::Vault(V::LoadTicked));
        assert_eq!(
            key_fps(&app),
            std::slice::from_ref(&want),
            "{kind}, boot sheet"
        );
        // From the Vaults list.
        let mut app = booted_with(&files);
        app.press(Action::Import(I::Go));
        app.press(Action::Import(I::Later));
        app.storage(StorageEvent::Sticks(Vec::new()));
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(0)));
        type_in(&mut app, phrase);
        app.press(Action::Vault(V::Unlock));
        run_unlock(&mut app);
        assert_eq!(
            app.vaults.open.len(),
            1,
            "{kind}: {:?}",
            app.vaults.unlock_error
        );
        app.press(Action::Vault(V::LoadTicked));
        assert_eq!(key_fps(&app), [want], "{kind}, Vaults");
    }
}
