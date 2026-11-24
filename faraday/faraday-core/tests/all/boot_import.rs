//! The boot import (F60, F130): the first look at the boot stick lists
//! its files by name and size, nothing of them read and nothing in Files,
//! and the sheet lists every one ticked, a seed's too. Pulling the stick
//! changes nothing but asks for it back. Import reads the ticked files
//! alone and brings in what they held, its wallets and keys with it, and
//! the vaults into Files, locked, each then offered its Unlock with
//! Cancel, an unlock followed by its Load sheet (F62); Later reads and
//! brings in nothing, and Home leads with the import. A lock drops it.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use faraday_core::boot_import::{self, ImportAction as I};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
use faraday_shell_api::{App, Event, Key, TouchPhase};

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
    let c = testkit::compute();
    let m = faraday_bip::bip39::Mnemonic::parse(
        &c,
        faraday_bip::bip39::Language::English,
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
            testkit::test_words(testkit::TEST_SEEDS[2].0)
                .as_bytes()
                .to_vec(),
        ),
        ("vault.kdbx".into(), testkit::test_vault().unwrap()),
        ("zebra-seedqr.png".into(), b"a picture".to_vec()),
        (
            "zebra-wallet.txt".into(),
            format!("{}\n", zebra_wallet()).into_bytes(),
        ),
    ]
}

fn boot_stick(files: &[(String, Vec<u8>)]) -> StickInfo {
    StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of(BOOT),
        label: faraday_text::Text::of("FARADAY"),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (faraday_text::Text::of(n), b.len() as u64))
            .collect(),
        // What the shell's listing finds: a vault by its first four bytes
        // and its length, whatever its name.
        vaults: files
            .iter()
            .filter(|(_, b)| faraday_codec::vault_file::is_vault_file(b))
            .map(|(n, _)| faraday_text::Text::of(n))
            .collect(),
    }
}

/// Answers the app's reads from the stick's files, and its picture's
/// codes with test key 2's SeedQR.
fn pump(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let bytes = files.iter().find(|(n, _)| *n == *name).unwrap().1.clone();
                app.storage(StorageEvent::Read {
                    stick,
                    name,
                    bytes: bytes.into(),
                });
            }
            StorageCommand::ReadQr { stick, name } => {
                app.storage(StorageEvent::QrRead {
                    stick,
                    name,
                    bytes: (Vec::new()).into(),
                    payloads: (vec![zebra_seedqr()]).into(),
                });
            }
            _ => {}
        }
    }
}

/// A fresh process with a boot stick holding `files` in.
fn booted_with(files: &[(String, Vec<u8>)]) -> Faraday {
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![boot_stick(files)]));
    pump(&mut app, files);
    app
}

/// A fresh process with the boot stick in.
fn booted() -> Faraday {
    booted_with(&stick_files())
}

/// Booted, and the stick pulled before Import.
fn pulled() -> Faraday {
    let mut app = booted();
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    app
}

/// Import pressed, and the files it asks for read from `files`.
fn go(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    app.press(Action::Import(I::Go));
    pump(app, files);
}

/// The stick pulled.
fn pull(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
}

/// Booted with `files`, Import pressed and its reads answered, and the
/// stick pulled.
fn imported_with(files: &[(String, Vec<u8>)]) -> Faraday {
    let mut app = booted_with(files);
    go(&mut app, files);
    pull(&mut app);
    app
}

/// Booted, Import pressed and its reads answered, and the stick pulled.
pub(crate) fn imported() -> Faraday {
    imported_with(&stick_files())
}

/// Types `passphrase` into the field typing goes to.
pub(crate) fn type_in(app: &mut Faraday, passphrase: &str) {
    for c in passphrase.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Lets the unlock asked for run: a frame, then the tick after it.
pub(crate) fn run_unlock(app: &mut Faraday) {
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
    let _ = app.frame();
}

/// Imported, put off with Later, and the vault unlocked on Vaults.
fn unlocked_elsewhere() -> Faraday {
    let mut app = imported();
    app.press(Action::Import(I::Later));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    app
}

/// A second vault, of its own salt: one slot with one entry.
fn spare_vault() -> Vec<u8> {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let records = vec![
        Record::new(kind::ENTRY)
            .with(field::TITLE, b"Locker")
            .with(field::PASSWORD, b"4321"),
    ];
    crate::common::vault_of("Spare", "spare", 0x5c, records)
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
    let mut v: Vec<String> = app.inbox().iter().map(|i| String::from(&*i.name)).collect();
    v.sort();
    v
}

fn key_fps(app: &Faraday) -> Vec<String> {
    let mut v: Vec<String> = app
        .session
        .keys
        .all()
        .map(|k| String::from(&*faraday_core::wallet::fp_text(k.fingerprint)))
        .collect();
    v.sort();
    v
}

/// A fresh process with a boot stick holding `files` just in: nothing
/// asked of it answered yet.
fn boot_only(files: &[(String, Vec<u8>)]) -> Faraday {
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![boot_stick(files)]));
    app
}

/// Answers what the app asks of the stick as [`pump`] does, and returns
/// the names of the files whose contents it asked for, sorted.
fn reads(app: &mut Faraday, files: &[(String, Vec<u8>)]) -> Vec<String> {
    let mut asked = Vec::new();
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                asked.push(String::from(&*name));
                let bytes = files.iter().find(|(n, _)| *n == *name).unwrap().1.clone();
                app.storage(StorageEvent::Read {
                    stick,
                    name,
                    bytes: bytes.into(),
                });
            }
            StorageCommand::ReadQr { stick, name } => {
                asked.push(String::from(&*name));
                app.storage(StorageEvent::QrRead {
                    stick,
                    name,
                    bytes: (Vec::new()).into(),
                    payloads: (vec![zebra_seedqr()]).into(),
                });
            }
            _ => {}
        }
    }
    asked.sort();
    asked
}

#[test]
fn before_import_the_sheet_lists_names_and_sizes_and_nothing_is_read() {
    let files = stick_files();
    let mut app = boot_only(&files);
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
    assert!(app.import_on_show());
    let texts = drawn(&mut app);
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    // Every file by its name and size, what it is guessed from its name
    // alone, ticked.
    let view = app.import_view().unwrap();
    for (name, bytes) in &files {
        assert!(
            texts.contains(name.as_str()),
            "{name} is not listed: {texts}"
        );
        let f = file(&view, name);
        assert!(f.chosen && f.enabled, "{name} is not ticked");
        assert!(f.line.contains("by its name"), "{name}: {}", &*f.line);
        if bytes.len() < 1000 {
            let size = format!("{}\u{a0}B", bytes.len());
            assert!(f.line.contains(&size), "{name}: {}", &*f.line);
        }
    }
    assert!(file(&view, "vault.kdbx").line.starts_with("Vault"));
    assert!(
        file(&view, "savings-unsigned.psbt")
            .line
            .starts_with("PSBT")
    );
    // Nothing comes in, and pulling the stick reads nothing either.
    assert!(app.inbox().is_empty() && app.session.keys.is_empty());
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    assert!(app.inbox().is_empty() && app.session.keys.is_empty());
}

#[test]
fn declining_the_import_reads_nothing() {
    let files = stick_files();
    let mut app = boot_only(&files);
    app.press(Action::Import(I::Later));
    let _ = app.frame();
    app.press(boot_import::OPEN);
    let _ = app.frame();
    app.press(Action::Import(I::Later));
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    assert!(app.inbox().is_empty() && app.session.keys.is_empty());
    assert!(app.vault_files().is_empty());
}

#[test]
fn import_reads_exactly_the_ticked_files() {
    let files = stick_files();
    let mut app = boot_only(&files);
    for name in ["notes.txt", "vault.kdbx"] {
        let k = row(&app, name);
        app.press(Action::Import(I::File(k)));
    }
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    app.press(Action::Import(I::Go));
    // While they are read, the sheet says so and Import is not offered.
    let texts = drawn(&mut app);
    assert!(texts.contains("0 of 5 files read"), "{texts}");
    assert!(!app.offers(Action::Import(I::Go)));
    let ticked: Vec<String> = files
        .iter()
        .map(|(n, _)| n.clone())
        .filter(|n| n != "notes.txt" && n != "vault.kdbx")
        .collect();
    assert_eq!(reads(&mut app, &files), ticked);
    // What they held comes in; the unticked ones stay on the stick.
    assert!(app.inbox().iter().any(|i| i.name == "zebra-wallet.txt"));
    assert!(!app.inbox().iter().any(|i| i.name == "notes.txt"));
    assert!(app.vault_files().is_empty());
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    assert!(app.session.wallets.iter().any(|w| w.name == "Zebra"));
}

#[test]
fn pulled_while_import_reads_what_was_read_comes_in_and_the_rest_is_said() {
    let files: Vec<(String, Vec<u8>)> = stick_files()
        .into_iter()
        .filter(|(n, _)| n == "notes.txt" || n == "zebra-wallet.txt")
        .collect();
    let mut app = boot_only(&files);
    app.press(Action::Import(I::Go));
    // The note is answered; the stick goes before the wallet's is.
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c
            && name == "notes.txt"
        {
            let bytes = files[0].1.clone();
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.into(),
            });
        }
    }
    assert!(app.inbox().is_empty(), "something came in before the reads");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(app.import().is_none());
    assert_eq!(inbox_names(&app), ["notes.txt"]);
    let texts = drawn(&mut app);
    assert!(texts.contains("1 not read"), "{texts}");
}

#[test]
fn with_the_stick_pulled_before_import_it_is_asked_back_and_its_return_brings_the_sheet() {
    let files = stick_files();
    let mut app = boot_only(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let texts = drawn(&mut app);
    assert!(texts.contains("Put the stick back in to import"), "{texts}");
    assert!(!app.offers(Action::Import(I::Go)));
    app.press(Action::Import(I::Go));
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    // Put back, it brings the sheet up again over Home, Import ready.
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
    assert!(app.import_on_show());
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Go)));
    app.press(Action::Import(I::Go));
    assert_eq!(reads(&mut app, &files).len(), files.len());
    assert!(app.inbox().iter().any(|i| i.name == "notes.txt"));
}

#[test]
fn the_boot_stick_is_listed_and_nothing_is_in_files_until_import() {
    let mut app = booted();
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
    assert!(app.import_on_show());
    assert!(app.inbox().is_empty(), "{:?}", inbox_names(&app));
    let c = app.import_count().unwrap();
    assert_eq!((c.vaults, c.psbts, c.other, c.asked), (1, 1, 5, 0));
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
        texts.contains("1 vault · 1 PSBT · 5 other files, by their names"),
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
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
    assert!(app.import_on_show());
    assert!(
        app.import_field().is_none(),
        "a passphrase field before Import"
    );
    assert!(app.inbox().is_empty() && app.session.keys.is_empty());
    let _ = app.frame();
    // Import reads the stick: with it out, Import waits for it.
    assert!(!app.offers(Action::Import(I::Go)));
    assert!(app.offers(Action::Import(I::Later)));
}

#[test]
fn import_brings_in_every_ticked_file_then_the_vaults_unlock_loads_its_wallets() {
    let mut app = imported();
    // The files From the stick, the picture as what its code held, and
    // the vault, locked.
    assert_eq!(
        inbox_names(&app),
        [
            "notes.txt",
            "photo.jpg",
            "savings-unsigned.psbt",
            "summer-words.txt",
            "vault.kdbx",
            "zebra-seedqr-words.txt",
            "zebra-wallet.txt"
        ]
    );
    assert!(app.vaults().open.is_empty());
    // Their wallet and keys: Zebra, test keys 2 and 3.
    assert!(app.session.wallets.iter().any(|w| w.name == "Zebra"));
    let mut want = vec![fp(1), fp(2)];
    want.sort();
    assert_eq!(key_fps(&app), want);
    // The vault's Unlock, on the sheet, typing going to it.
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert_eq!(
        app.import_field().map(|v| v.name.as_str().to_owned()),
        Some("vault.kdbx".to_owned())
    );
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Submit)));
    assert!(app.offers(Action::Import(I::Cancel)));
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    // Its Load sheet (F62), everything ticked; Load ticked ends the
    // import and lands on Wallets.
    assert_eq!(
        (app.screen, app.sheet),
        (Screen::Home, Some(Sheet::UnlockLoad))
    );
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.import().is_none());
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
        testkit::test_words(testkit::TEST_SEEDS[0].0)
            .as_bytes()
            .to_vec(),
    ));
    let mut app = imported_with(&files);
    assert!(key_fps(&app).contains(&fp(0)), "the words file's key");
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    assert_eq!(app.vaults().unlock_error, None);
    // The vault's copy of the key is Loaded on its Load sheet.
    app.press(Action::Vault(V::LoadTicked));
    let mut want = vec![fp(0), fp(1), fp(2)];
    want.sort();
    // Once each: what Wallets and Sign list.
    assert_eq!(key_fps(&app), want, "a key loaded twice");
}

#[test]
fn a_wrong_passphrase_stays_on_the_sheet_and_says_so() {
    let mut app = imported();
    type_in(&mut app, "not the passphrase");
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert!(app.vaults().open.is_empty());
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert!(app.import().is_some());
    let error = app.vaults().unlock_error.clone().expect("no error said");
    assert!(
        drawn(&mut app).contains(error.as_str()),
        "the error is not on the sheet"
    );
    // And the right one, typed again, unlocks: its Load sheet, then the
    // import is over.
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Vault(V::LoadNotNow));
    assert!(app.import().is_none() && app.vaults().open.len() == 1);
}

#[test]
fn import_with_the_stick_in_waits_for_the_pull_to_unlock() {
    let mut app = booted();
    go(&mut app, &stick_files());
    assert!(app.inbox().iter().any(|i| i.name == "notes.txt"));
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
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
}

#[test]
fn the_reminders_before_the_passphrase_leave_the_sheet_where_it_was() {
    let files = stick_files();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.devices =
        faraday_core::devices::DeviceCheck::machine("flags\t: fpu vme", "LENOVO", "20XW", "10");
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    go(&mut app, &files);
    pull(&mut app);
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
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.import().is_none() && app.vaults().open.len() == 1);
}

#[test]
fn later_brings_in_nothing_and_home_leads_with_the_import() {
    let mut app = booted();
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import().is_some());
    assert!(app.inbox().is_empty() && app.session.keys.is_empty());
    let texts = drawn(&mut app);
    assert!(texts.contains("Import from Boot stick"), "{texts}");
    assert!(app.offers(boot_import::OPEN));
    app.press(boot_import::OPEN);
    assert!(app.import_on_show());
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Go)));
}

#[test]
fn later_with_the_stick_in_leaves_the_files_to_the_stick_visit() {
    let files = stick_files();
    let mut app = booted();
    app.press(Action::Import(I::Later));
    assert!(app.inbox().is_empty());
    app.press(Action::Nav(Screen::Files));
    assert!(
        app.visit().inn.contains("notes.txt"),
        "{:?}",
        app.visit().inn
    );
    app.press(Action::VisitCopy);
    pump(&mut app, &files);
    assert!(app.inbox().iter().any(|i| i.name == "notes.txt"));
    assert!(app.inbox().iter().any(|i| i.name == "vault.kdbx"));
}

#[test]
fn put_off_after_import_home_leads_with_the_vaults_unlock() {
    // No PSBT, whose Sign would lead Home first.
    let files: Vec<(String, Vec<u8>)> = stick_files()
        .into_iter()
        .filter(|(n, _)| !n.ends_with(".psbt"))
        .collect();
    let mut app = imported_with(&files);
    type_in(&mut app, "half typ");
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(
        app.vaults().unlock_text().is_empty(),
        "what was typed stays"
    );
    let texts = drawn(&mut app);
    assert!(texts.contains("vault.kdbx"), "{texts}");
    assert!(texts.contains("Unlock"), "{texts}");
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Submit)));
}

#[test]
fn with_two_vaults_each_is_offered_in_turn_and_cancel_leaves_it_locked() {
    let mut files = stick_files();
    files.push(("spare.kdbx".into(), spare_vault()));
    let mut app = imported_with(&files);
    assert_eq!(
        app.import_field().map(|v| v.name.as_str().to_owned()),
        Some("vault.kdbx".to_owned())
    );
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    // Its Load sheet, then the second vault.
    assert_eq!(
        (app.screen, app.sheet),
        (Screen::Home, Some(Sheet::UnlockLoad))
    );
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    // The second, on the sheet in its turn; Cancel shows no Load sheet.
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert_eq!(
        app.import_field().map(|v| v.name.as_str().to_owned()),
        Some("spare.kdbx".to_owned())
    );
    app.press(Action::Import(I::Cancel));
    assert!(app.import().is_none());
    assert_eq!(app.sheet, None);
    let spare = app
        .vault_files()
        .into_iter()
        .find(|f| f.name == "spare.kdbx")
        .expect("the cancelled vault is not in Files");
    assert!(spare.open.is_none());
}

#[test]
fn cancel_leaves_the_only_vault_locked_in_files() {
    let mut app = imported();
    app.press(Action::Import(I::Cancel));
    assert!(app.import().is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    let files = app.vault_files();
    assert_eq!(files.len(), 1);
    assert!(files[0].open.is_none());
}

#[test]
fn with_no_vault_import_brings_the_files_in_and_home_leads_with_the_psbt() {
    let files: Vec<(String, Vec<u8>)> = stick_files()
        .into_iter()
        .filter(|(n, _)| n != "vault.kdbx")
        .collect();
    let mut app = booted_with(&files);
    assert!(app.import().is_some() && app.inbox().is_empty());
    go(&mut app, &files);
    pull(&mut app);
    assert!(app.import().is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    let psbt = app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .expect("the PSBT did not come in");
    assert!(app.inbox().iter().any(|i| i.name == "notes.txt"));
    assert!(app.session.wallets.iter().any(|w| w.name == "Zebra"));
    // Home leads with signing it.
    let _ = app.frame();
    assert!(app.offers(Action::StartSpend(psbt)));
}

#[test]
fn with_only_the_settings_file_no_sheet_comes_up() {
    let files = vec![(
        faraday_core::stick::settings::FILE.to_string(),
        b"faraday-settings 1\n".to_vec(),
    )];
    let app = booted_with(&files);
    assert!(app.import().is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
}

fn two_wallet_files() -> Vec<(String, Vec<u8>)> {
    testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n == "savings-wallet.txt" || n == "spending-wallet.txt")
        .collect()
}

#[test]
fn the_imports_multisig_wallet_is_asked_and_home_is_back_after_the_hold() {
    let mut app = imported_with(&two_wallet_files());
    // A wallet that came in by an event asks on the next tick.
    app.event(Event::Tick { now_ms: 2000 });
    // Spending is one key and loads at once; Savings asks.
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WalletConfirm));
    assert_eq!(app.session.wallets.len(), 1);
    app.press(Action::WalletHold);
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import().is_none());
    assert_eq!(app.session.wallets.len(), 2);
}

#[test]
fn cancelling_the_imports_multisig_wallet_adds_it_not_and_comes_back_to_home() {
    let mut app = imported_with(&two_wallet_files());
    app.event(Event::Tick { now_ms: 2000 });
    app.press(Action::WalletCancel);
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import().is_none());
    // Only the one-key wallet, which asked nothing.
    assert_eq!(app.session.wallets.len(), 1);
    assert!(app.session.wallets.iter().all(|w| w.name != "Savings"));
}

#[test]
fn wallet_files_with_no_key_here_still_load_as_wallets() {
    let kit = testkit::files().unwrap();
    let files: Vec<(String, Vec<u8>)> = kit
        .into_iter()
        .filter(|(n, _)| n == "savings-wallet.txt" || n == "spending-wallet.txt")
        .collect();
    let mut app = imported_with(&files);
    faraday_core::testkit::confirm_wallets(&mut app);
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
    let mut app = booted();
    for name in ["notes.txt", "summer-words.txt"] {
        let k = row(&app, name);
        app.press(Action::Import(I::File(k)));
        assert!(!file(&app.import_view().unwrap(), name).chosen);
    }
    go(&mut app, &stick_files());
    pull(&mut app);
    assert!(!app.inbox().iter().any(|i| i.name == "notes.txt"));
    assert!(!app.inbox().iter().any(|i| i.name == "summer-words.txt"));
    assert!(app.inbox().iter().any(|i| i.name == "photo.jpg"));
    assert_eq!(key_fps(&app), [fp(1)]);
    assert!(
        !app.inbox()
            .iter()
            .any(|i| i.name == "summer-words.txt" && i.kind == FileKind::Words)
    );
}

#[test]
fn an_unticked_vault_stays_out_of_files_and_nothing_asks_to_unlock_it() {
    let mut app = booted();
    let k = row(&app, "vault.kdbx");
    app.press(Action::Import(I::File(k)));
    go(&mut app, &stick_files());
    pull(&mut app);
    assert!(!app.inbox().iter().any(|i| i.name == "vault.kdbx"));
    assert!(app.import().is_none());
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.keys.len(), 2);
}

#[test]
fn a_tap_beside_the_passphrase_sheet_is_later() {
    let mut app = imported();
    assert_eq!(app.sheet, Some(Sheet::Import));
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
    assert!(app.import().is_some(), "the vault still waits for Home");
}

/// A fresh process with the boot stick's list up, a file For the stick and
/// a setting changed: what In Faraday's controls and the stick's card act on.
fn boot_with_work(files: &[(String, Vec<u8>)]) -> Faraday {
    use faraday_core::ui::Mode;
    let mut app = boot_only(files);
    testkit::outbox_file_at(&mut app, 0, "outgoing.txt", b"going out".to_vec());
    let mut theme = app.theme();
    theme.mode = Mode::Dark;
    app.press(Action::Theme(theme));
    let _ = app.frame();
    assert!(app.settings_changed(), "a changed setting to write");
    assert!(app.import_on_show());
    app
}

/// What the list's page refuses: the stick's card, Delete files and the
/// stick's own file menu. Whatever the page draws live must not be one.
fn refused_with_the_list(a: Action) -> bool {
    use faraday_core::file_menu::{FilePress as P, Row};
    match a {
        Action::VisitFrom(..) | Action::VisitTab(..) | Action::VisitBar(..) => false,
        Action::NameTakenBeside | Action::NameTakenOverwrite => false,
        a if format!("{a:?}").starts_with("Visit") => true,
        Action::DeleteFiles
        | Action::DeleteUnprotected
        | Action::DeleteWhat(..)
        | Action::DeletePick(..)
        | Action::DeletePasses(..)
        | Action::DeleteGo => true,
        Action::File(P::Menu(r) | P::Rename(r) | P::Details(r) | P::Decode(r)) => {
            matches!(r, Row::Stick(_))
        }
        Action::File(P::StickDelete(_) | P::NameStick) => true,
        _ => false,
    }
}

/// Nothing the list's page offers is refused: In Faraday's ticks, Select
/// all, Write and the end row are not drawn live while the list stands.
#[test]
fn nothing_the_list_page_offers_is_refused() {
    let files = stick_files();
    let mut app = boot_with_work(&files);
    // A secret file written to the stick: its notice and Delete are not
    // drawn on the list's page.
    app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("backup-words.txt"),
        bytes: b"words".to_vec().into(),
        kind: FileKind::Words,
        secret: true,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    app.storage(StorageEvent::Written {
        stick: faraday_text::Text::of(BOOT),
        name: faraday_text::Text::of("backup-words.txt"),
        wrote_as: faraday_text::Text::of("backup-words.txt"),
    });
    assert!(app.import_on_show());
    let _ = app.frame();
    let offered = app.offered_actions();
    assert!(!offered.is_empty());
    let refused: Vec<_> = offered
        .into_iter()
        .filter(|a| refused_with_the_list(*a))
        .collect();
    assert!(refused.is_empty(), "offered and refused: {refused:?}");
}

/// A file menu of an In Faraday file opened while the reads run and closed
/// after them: the vault step still comes up, and does not wait to jump in
/// over a later Cancel.
#[test]
fn closing_a_file_menu_over_the_reads_brings_the_vault_step_up() {
    use faraday_core::file_menu::{FilePress as P, Row};
    let files = stick_files();
    let mut app = boot_only(&files);
    testkit::outbox_file_at(&mut app, 0, "outgoing.txt", b"going out".to_vec());
    let _ = app.frame();
    reads_ended_under(&mut app, &files, |a| {
        a.press(Action::File(P::Menu(Row::Out(0))));
        assert_eq!(a.sheet, Some(Sheet::FileMenu));
    });
    assert_eq!(app.sheet, Some(Sheet::FileMenu));
    app.press(Action::File(P::Close));
    assert_eq!(app.sheet, Some(Sheet::Import), "the vault step is lost");
    assert!(app.import_field().is_some());
}

/// A stick arriving while a backup flow is on screen takes the list down
/// all the same: Files does not bring the old list back over the new stick.
#[test]
fn a_stick_arriving_in_a_backup_flow_takes_the_list_down() {
    use faraday_core::backup_flow::Kind;
    let files = stick_files();
    let mut app = boot_only(&files);
    assert!(app.import_on_show());
    crate::common::start(&mut app, Kind::SecureBoot);
    assert_eq!(app.screen, Screen::BackupFlow);
    let other = StickInfo {
        id: faraday_text::Text::of("other"),
        boot: false,
        ..boot_stick(&files)
    };
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files), other]));
    let _ = app.frame();
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(!app.import_tile_up(), "the old list came back over Files");
}

/// Home's row after another stick came in: the import is of the stick it
/// was read from, and its list does not rise over the other's card.
#[test]
fn homes_row_does_not_raise_the_list_over_another_stick() {
    let files = stick_files();
    let mut app = boot_only(&files);
    app.press(Action::Import(I::Later));
    pull(&mut app);
    let mut changed = files.clone();
    changed.pop();
    let other = StickInfo {
        id: faraday_text::Text::of("other"),
        boot: false,
        ..boot_stick(&changed)
    };
    app.storage(StorageEvent::Sticks(vec![other]));
    let _ = app.frame();
    app.press(boot_import::OPEN);
    let _ = app.frame();
    assert!(!app.import_tile_up(), "another stick's card shows the list");
    assert!(app.import().is_none(), "the import is of another stick");
}

/// What the stick's files are hashed for: Faraday holds a file of the first
/// name and size.
fn boot_holding_a_listed_file(files: &[(String, Vec<u8>)]) -> Faraday {
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![files[0].clone()]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Sticks(vec![boot_stick(files)]));
    let _ = app.frame();
    app
}

fn hashes(app: &mut Faraday) -> usize {
    let mut n = 0;
    while let Some(c) = app.poll_storage() {
        n += usize::from(matches!(c, StorageCommand::Hash { .. }));
    }
    n
}

/// The settings read, a second listing of the stick comes while its list
/// stands: still nothing of it is hashed. After Later the stick's card is
/// the person's to visit, and it is.
#[test]
fn a_second_listing_hashes_nothing_while_the_list_stands_and_does_after_later() {
    let files = stick_files();
    let mut app = boot_holding_a_listed_file(&files);
    assert_eq!(hashes(&mut app), 0);
    assert!(app.import_tile_up());
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    assert_eq!(hashes(&mut app), 0, "hashed under the list");
    app.press(Action::Import(I::Later));
    app.press(Action::Nav(Screen::Files));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    assert!(!app.import_tile_up());
    assert!(hashes(&mut app) > 0, "never hashed once the list was down");
}

/// The stick's own actions do nothing while the import's list stands where
/// the stick's card does: nothing is read or copied before Import.
#[test]
fn the_stick_visit_is_refused_while_the_import_list_stands() {
    let files = stick_files();
    for action in [
        Action::VisitIn(0),
        Action::VisitInAll,
        Action::VisitCopy,
        Action::VisitCopyAndLoad,
        Action::VisitCopyAndSign,
    ] {
        // Each in an app of its own, with the default ticks: one action
        // changes what the next would have to read.
        let mut app = boot_only(&files);
        assert!(app.import_on_show());
        let ticked = app.visit().inn.clone();
        assert!(!ticked.is_empty());
        app.press(action);
        let _ = app.frame();
        assert_eq!(reads(&mut app, &files), Vec::<String>::new(), "{action:?}");
        assert!(app.inbox().is_empty() && app.session.keys.is_empty());
        assert!(app.import_on_show(), "{action:?} took the list down");
        assert_eq!(app.visit().inn, ticked, "{action:?} changed the ticks");
    }
}

/// Every other action of the stick's own card is refused too, by the one
/// rule, and the stick's own file actions: nothing is read, hashed, renamed
/// or deleted before Import.
#[test]
fn every_action_of_the_sticks_own_card_is_refused_while_the_import_list_stands() {
    use faraday_core::file_menu::{FilePress as P, Row};
    let files = stick_files();
    for action in [
        Action::DeleteFiles,
        Action::DeleteUnprotected,
        Action::VisitStick(0),
        Action::VisitOut(0),
        Action::VisitOutAll,
        Action::VisitInbox(0),
        Action::VisitSettings,
        Action::VisitWrite,
        Action::VisitReplace,
        Action::VisitKeepBoth,
        Action::File(P::Details(Row::Stick(0))),
        Action::File(P::Menu(Row::Stick(0))),
        Action::File(P::Rename(Row::Stick(0))),
        Action::File(P::NameStick),
        Action::File(P::StickDelete(0)),
        Action::File(P::RenameGo),
    ] {
        // A file For the stick and a setting changed: something to act on.
        let mut app = boot_with_work(&files);
        assert!(app.import_on_show());
        let ticked = app.visit().inn.clone();
        let out = app.visit().out.clone();
        let settings = app.visit_settings_on();
        while app.poll_storage().is_some() {}
        app.press(action);
        let _ = app.frame();
        let mut asked = 0;
        while app.poll_storage().is_some() {
            asked += 1;
        }
        assert_eq!(asked, 0, "{action:?} asked the stick");
        assert_eq!(app.visit().out, out, "{action:?} changed the ticks");
        assert_eq!(app.visit_settings_on(), settings, "{action:?}");
        assert!(app.import_on_show(), "{action:?} took the list down");
        assert_eq!(app.visit().inn, ticked, "{action:?} changed the ticks");
        assert_eq!(app.sheet, None, "{action:?} opened a sheet");
    }
}

/// The stick's files are not read for a name Faraday holds while its
/// import waits: its listing is all that is read before Import (F130).
#[test]
fn a_file_of_a_held_name_on_the_stick_is_not_hashed_while_the_import_waits() {
    let files = stick_files();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    // Faraday holds a file with the stick's first name and size.
    let (name, bytes) = files[0].clone();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(name, bytes)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    let mut hashed = 0;
    while let Some(c) = app.poll_storage() {
        hashed += usize::from(matches!(c, StorageCommand::Hash { .. }));
    }
    assert_eq!(
        hashed, 0,
        "the import waiting, a file of the stick was hashed"
    );
}

/// Another stick put back where the boot stick was, or this one with a
/// listing that changed, is not the list's: the list is taken down, the
/// import ends and nothing of the old list is read from it.
#[test]
fn a_swapped_stick_takes_the_list_down_and_is_not_read_by_it() {
    let files = stick_files();
    let mut other = files.clone();
    other[0].1.push(b'x');
    let mut fewer = files.clone();
    fewer.pop();
    for changed in [other, fewer] {
        let mut app = boot_only(&files);
        assert!(app.import_on_show());
        pull(&mut app);
        app.storage(StorageEvent::Sticks(vec![boot_stick(&changed)]));
        let _ = app.frame();
        assert!(
            !app.import_on_show(),
            "the old list stands for another listing"
        );
        assert!(!app.offers(Action::Import(I::Go)));
        app.press(Action::Import(I::Go));
        assert_eq!(reads(&mut app, &changed), Vec::<String>::new());
        assert!(app.inbox().is_empty());
    }
}

/// A lock with the import read: nothing it brought is kept, and the stick
/// seen again in that clean process is not a first look.
#[test]
fn a_lock_drops_the_import_and_the_first_look_is_not_repeated() {
    let files = stick_files();
    let mut app = booted_with(&files);
    go(&mut app, &files);
    assert!(app.session.keys.is_empty());
    // The sheet stands over the Lock button: Later first.
    app.press(Action::Import(I::Later));
    app.press(Action::Lock);
    // What came in is not kept: Lock asks, and the lock without sealing
    // goes ahead.
    if app.sheet == Some(Sheet::LockAsk) {
        app.press(Action::LockUnsealed);
    }
    testkit::settle(&mut app);
    assert!(
        app.import().is_none() && !app.import_on_show(),
        "{:?}",
        app.sheet
    );
    pull(&mut app);
    assert!(key_fps(&app).is_empty(), "{:?}", key_fps(&app));
    // The stick back in the same process: no list, no read. A process
    // that held a secret would lock instead of looking; this one is clean.
    assert!(app.clean());
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    assert!(app.import().is_none() && !app.import_on_show());
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    assert!(key_fps(&app).is_empty());
}

/// A lock with the list still standing (nothing read) drops the list, and
/// the stick seen again in that process is not listed again.
#[test]
fn a_lock_over_the_list_drops_it_and_the_stick_is_not_listed_again() {
    let files = stick_files();
    let mut app = boot_only(&files);
    assert!(app.import_on_show());
    app.press(Action::Lock);
    assert!(app.import().is_none() && !app.import_on_show());
    assert!(app.clean());
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    assert!(app.import().is_none() && !app.import_on_show());
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
}

#[test]
fn a_lock_drops_the_import_and_the_next_process_visits_the_stick() {
    let mut app = unlocked_elsewhere();
    app.press(Action::Lock);
    assert!(app.import().is_none());
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
    faraday_core::testkit::move_to(&mut next.session, faraday_core::testkit::NET).unwrap();
    next.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept,
    });
    next.storage(StorageEvent::Sticks(vec![boot_stick(&stick_files())]));
    pump(&mut next, &stick_files());
    assert!(next.import().is_none());
    assert_eq!((next.screen, next.sheet), (Screen::Files, None));
}

#[test]
fn a_picture_on_a_visit_comes_in_ticked_and_has_its_codes_read_into_the_inbox() {
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("wallet-qr.png"), 9000)],
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Files);
    let _ = app.frame();
    assert!(
        app.offers(Action::VisitIn(0)),
        "the picture cannot be ticked"
    );
    assert!(
        app.visit().inn.contains("wallet-qr.png"),
        "it is not ticked"
    );
    app.press(Action::VisitCopy);
    let mut asked = false;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::ReadQr { stick, name } = c {
            asked = true;
            app.storage(StorageEvent::QrRead {
                stick,
                name,
                bytes: (Vec::new()).into(),
                payloads: (vec![zebra_wallet().into_bytes()]).into(),
            });
        }
    }
    assert!(asked, "its codes were not asked for");
    let item = app
        .inbox()
        .iter()
        .find(|i| i.name.starts_with("wallet-qr"))
        .expect("what the picture holds is not in the Inbox");
    assert_eq!(item.kind, FileKind::Wallet);
}

/// A stick at a visit (not the boot stick) with one picture, `name`.
fn visit_with_picture(name: &str) -> Faraday {
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of(name), 9000)],
        vaults: Vec::new(),
    }]));
    // The visit page ticks its pictures when it is first drawn.
    let _ = app.frame();
    app
}

/// Presses Copy at the visit and answers every code read with `payload`.
fn visit_copy_answered(app: &mut Faraday, payload: Vec<u8>) {
    app.press(Action::VisitCopy);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::ReadQr { stick, name } = c {
            app.storage(StorageEvent::QrRead {
                stick,
                name,
                bytes: (Vec::new()).into(),
                payloads: (vec![payload.clone()]).into(),
            });
        }
    }
}

#[test]
fn a_picture_of_a_codex32_share_at_a_visit_comes_in_only_when_ticked() {
    let share = CODEX32_SHARES[0].as_bytes().to_vec();
    let mut app = visit_with_picture("share.png");
    // The picture is ticked by default; the person unticks it.
    app.press(Action::VisitIn(0));
    assert!(!app.visit().inn.contains("share.png"), "it is still ticked");
    visit_copy_answered(&mut app, share.clone());
    assert!(
        app.inbox().iter().all(|i| !i.name.starts_with("share")),
        "an unticked picture of a share came into Files"
    );

    // Left at the visit's own tick, the picture is not the person's choice;
    // unticked and ticked again by them, it is.
    let mut app = visit_with_picture("share.png");
    app.press(Action::VisitIn(0));
    app.press(Action::VisitIn(0));
    assert!(app.visit().inn.contains("share.png"), "it is not ticked");
    visit_copy_answered(&mut app, share);
    assert!(
        app.inbox().iter().any(|i| i.name.starts_with("share")),
        "a ticked picture of a share did not come in"
    );
}

#[test]
fn a_code_scanned_at_a_visit_leaves_a_text_question_waiting_asked() {
    let mut app = visit_with_picture("note.png");
    faraday_core::testkit::open_scan(&mut app, faraday_core::ScanPurpose::Files);
    visit_copy_answered(&mut app, b"a short note 123".to_vec());
    assert_eq!(app.sheet, Some(Sheet::TextSeed), "the question is not up");
    app.event(Event::Scanned {
        bytes: zebra_wallet().into_bytes(),
    });
    assert_eq!(app.sheet, Some(Sheet::TextSeed), "the question was dropped");
}

#[test]
fn a_seed_on_the_stick_as_words_and_a_picture_is_labelled_from_the_words_file() {
    let stem = "seed-1-9a6a2580";
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    // The stick's sorted listing puts the picture before the words file.
    let files: Vec<(String, Vec<u8>)> = vec![
        (format!("{stem}-compactseedqr.png"), b"a picture".to_vec()),
        (format!("{stem}-words.txt"), words.as_bytes().to_vec()),
    ];
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let bytes = files.iter().find(|(n, _)| *n == *name).unwrap().1.clone();
                app.storage(StorageEvent::Read {
                    stick,
                    name,
                    bytes: bytes.into(),
                });
            }
            StorageCommand::ReadQr { stick, name } => {
                app.storage(StorageEvent::QrRead {
                    stick,
                    name,
                    bytes: (Vec::new()).into(),
                    payloads: (vec![seed_seedqr(0)]).into(),
                });
            }
            _ => {}
        }
    }
    // No vault: Import brings it in.
    app.press(Action::Import(I::Go));
    while let Some(c) = app.poll_storage() {
        match c {
            StorageCommand::Read { stick, name } => {
                let bytes = files.iter().find(|(n, _)| *n == *name).unwrap().1.clone();
                app.storage(StorageEvent::Read {
                    stick,
                    name,
                    bytes: bytes.into(),
                });
            }
            StorageCommand::ReadQr { stick, name } => {
                app.storage(StorageEvent::QrRead {
                    stick,
                    name,
                    bytes: (Vec::new()).into(),
                    payloads: (vec![seed_seedqr(0)]).into(),
                });
            }
            _ => {}
        }
    }
    pull(&mut app);
    assert!(app.import().is_none());
    let key = app
        .session
        .keys
        .all()
        .find(|k| *faraday_core::wallet::fp_text(k.fingerprint) == *fp(0))
        .expect("the seed did not load");
    assert_eq!(
        key.label,
        format!("{stem}-words").as_str(),
        "the picture's label won over the words file's"
    );
}

#[test]
fn a_vault_imported_then_unlocked_on_vaults_loads_its_own_wallets() {
    let mut app = unlocked_elsewhere();
    // Unlocked through the list, not the sheet:
    // its Load sheet over the vault's contents; Load ticked stays there,
    // with the vault's own wallets and keys loaded; the import, its one
    // vault open, is over.
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!((app.screen, app.sheet), (Screen::VaultContents, None));
    assert!(app.import().is_none());
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    assert_eq!(app.vaults().back_to, None);
}

/// A USB mouse the shell holds back until a person says so.
fn mouse(app: &mut Faraday) {
    app.storage(StorageEvent::NewInput {
        id: 9,
        name: faraday_text::Text::of("USB Optical Mouse"),
        keyboard: false,
        pointer: true,
        touch: false,
    });
    app.storage(StorageEvent::InputUsed { id: 9 });
}

/// The same mouse, plugged in and never moved.
fn silent_mouse(app: &mut Faraday) {
    app.storage(StorageEvent::NewInput {
        id: 9,
        name: faraday_text::Text::of("USB Optical Mouse"),
        keyboard: false,
        pointer: true,
        touch: false,
    });
}

#[test]
fn a_mouse_plugged_in_at_boot_is_asked_about_before_the_import() {
    let files = stick_files();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    mouse(&mut app);
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputUse(9));
    assert_eq!(app.poll_input_decision(), Some((9, true)));
    assert!(app.import_on_show());
}

#[test]
fn a_mouse_seen_after_the_stick_is_asked_about_first_too() {
    let mut app = booted();
    assert!(app.import_on_show());
    mouse(&mut app);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputIgnore(9));
    assert!(app.import_on_show());
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
    go(&mut app, &files);
    assert!(app.inbox().iter().any(|i| i.name == "seed-codex32.txt"));
    assert!(
        app.session.keys.is_empty(),
        "a key loaded with the stick in"
    );
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.session.keys.len(), 1, "the codex32 seed did not load");
}

#[test]
fn a_hex_or_xprv_file_imports_as_a_file_and_loads_no_key() {
    let files: Vec<(String, Vec<u8>)> = vec![
        (
            "hash.txt".into(),
            b"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\n".to_vec(),
        ),
        (
            "master.txt".into(),
            b"xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi\n".to_vec(),
        ),
        ("notes.txt".into(), b"Shopping list\n".to_vec()),
    ];
    let mut app = booted_with(&files);
    go(&mut app, &files);
    for name in ["hash.txt", "master.txt"] {
        assert!(
            app.inbox()
                .iter()
                .any(|i| i.name == name && i.kind == FileKind::KeyLine),
            "{name}"
        );
    }
    assert!(app.session.keys.is_empty(), "a hex file loaded by itself");
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(
        app.session.keys.is_empty(),
        "loaded when the stick came out"
    );
}

#[test]
fn unselect_all_unticks_every_file_and_import_then_waits_for_a_choice() {
    let mut app = booted();
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
    assert!(app.import_on_show());
    assert!(app.import().is_some() && app.inbox().is_empty());
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
    let c = testkit::compute();
    let mut probe = faraday_core::wallet::Session::default();
    let fp = probe
        .keys
        .add_words_with(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[2].0),
            passphrase,
            "",
            None,
        )
        .unwrap();
    String::from(&*faraday_core::wallet::fp_text(fp))
}

/// Every kind of seed file Faraday reads, imported from the boot sheet
/// with the stick in: none loads while it is in, and each loads, once,
/// as the stick is pulled, the moment a BIP-39 words file's does (F60).
#[test]
fn every_kind_of_seed_file_imported_with_the_stick_in_loads_at_the_pull() {
    let words = || {
        testkit::test_words(testkit::TEST_SEEDS[2].0)
            .as_bytes()
            .to_vec()
    };
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
        go(&mut app, &files);
        for (name, _) in &files {
            assert!(
                app.inbox()
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
        // A key of codex32 strings keeps its set's facts: the strings in
        // Files (`docs/CODEX32.md` §9.4).
        if kind == "codex32 shares" {
            let set = app.session.keys[0].codex32.clone().expect("the set");
            assert_eq!(set.threshold, 2);
            assert_eq!(set.indices().len(), 2, "{kind}");
        }
    }
}

/// A vault holding a master seed, as a SLIP-39 or codex32 key is kept
/// (BIP 93 vector 1's master seed), its key loading at unlock.
fn seed_vault() -> Vec<u8> {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let seed = faraday_bip::codex32::Codex32::parse(CODEX32_SECRET)
        .unwrap()
        .to_seed()
        .unwrap();
    let seed = seed.expose().bytes();
    let mut payload = vec![0u8; 65];
    payload[0] = seed.len() as u8;
    payload[1..1 + seed.len()].copy_from_slice(seed);
    let records = vec![
        Record::new(kind::KEY)
            .with(field::KEY, &payload)
            .with(field::KEY_LABEL, b"codex32"),
    ];
    crate::common::vault_of("Seeds", "seeds", 0x5d, records)
}

/// The key a codex32 secret file gives, imported from the boot sheet.
fn codex32_fp() -> String {
    let files = vec![(
        "codex32-secret.txt".to_string(),
        format!("{CODEX32_SECRET}\n").into_bytes(),
    )];
    let app = imported_with(&files);
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
            testkit::VAULT_PASSPHRASE,
            fp(0),
        ),
        ("master seed", seed_vault(), "seeds", codex32),
    ];
    for (kind, vault, phrase, want) in cases {
        let files = vec![("vault.kdbx".to_string(), vault)];
        // On the boot sheet.
        let mut app = imported_with(&files);
        type_in(&mut app, phrase);
        app.press(Action::Import(I::Submit));
        run_unlock(&mut app);
        assert_eq!(
            app.vaults().open.len(),
            1,
            "{kind}: {:?}",
            app.vaults().unlock_error
        );
        app.press(Action::Vault(V::LoadTicked));
        assert_eq!(
            key_fps(&app),
            std::slice::from_ref(&want),
            "{kind}, boot sheet"
        );
        // From the Vaults list.
        let mut app = booted_with(&files);
        go(&mut app, &files);
        app.press(Action::Import(I::Later));
        app.storage(StorageEvent::Sticks(Vec::new()));
        app.press(Action::Nav(Screen::Vaults));
        app.press(Action::Vault(V::Open(0)));
        type_in(&mut app, phrase);
        app.press(Action::Vault(V::Unlock));
        run_unlock(&mut app);
        assert_eq!(
            app.vaults().open.len(),
            1,
            "{kind}: {:?}",
            app.vaults().unlock_error
        );
        app.press(Action::Vault(V::LoadTicked));
        assert_eq!(key_fps(&app), [want], "{kind}, Vaults");
    }
}

/// A boot stick file whose name Files holds with other bytes comes in
/// beside it under a numbered name, and the toast says so; one Files
/// holds with the same bytes is not kept twice.
#[test]
fn import_keeps_a_file_of_a_held_name_beside_it_and_a_same_one_once() {
    let files = stick_files();
    let wallet = files
        .iter()
        .find(|(n, _)| n == "zebra-wallet.txt")
        .unwrap()
        .clone();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("notes.txt"),
                b"Other notes\n".to_vec(),
            ),
            (faraday_text::Text::of(&wallet.0), wallet.1),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    go(&mut app, &files);
    let names = inbox_names(&app);
    let count = |n: &str| names.iter().filter(|x| x.as_str() == n).count();
    assert_eq!(count("notes.txt"), 1, "{names:?}");
    assert_eq!(count("notes-2.txt"), 1, "{names:?}");
    assert_eq!(count("zebra-wallet.txt"), 1, "{names:?}");
    assert!(
        !names.iter().any(|n| n == "zebra-wallet-2.txt"),
        "{names:?}"
    );
    let held = app.inbox().iter().find(|i| i.name == "notes.txt").unwrap();
    assert_eq!(*held.bytes, b"Other notes\n");
    assert!(
        app.toast_text()
            .is_some_and(|t| t.ends_with("already in Files, kept as notes-2.txt")),
        "{:?}",
        app.toast_text()
    );
}

#[test]
fn a_vault_under_a_short_name_is_counted_and_imported_as_a_vault() {
    let mut files = stick_files();
    for f in &mut files {
        if f.0 == "vault.kdbx" {
            f.0 = "VAULT~1.FVL".into();
        }
    }
    let mut app = booted_with(&files);
    let texts = drawn(&mut app);
    assert!(
        texts.contains("1 vault · 1 PSBT · 5 other files, by their names"),
        "{texts}"
    );
    go(&mut app, &files);
    pull(&mut app);
    assert!(inbox_names(&app).contains(&"VAULT~1.FVL".to_string()));
    assert_eq!(
        app.import_field().map(|v| v.name.as_str().to_owned()),
        Some("VAULT~1.FVL".to_owned())
    );
}

#[test]
fn a_file_named_kdbx_that_holds_other_bytes_is_not_counted_as_a_vault() {
    let mut files = stick_files();
    for f in &mut files {
        if f.0 == "vault.kdbx" {
            f.1 = b"not a vault".to_vec();
        }
    }
    let mut app = booted_with(&files);
    go(&mut app, &files);
    pull(&mut app);
    assert!(app.import_field().is_none());
}

#[test]
fn a_held_device_that_never_acts_does_not_stop_the_import_and_one_used_takes_the_screen() {
    let mut app = booted();
    silent_mouse(&mut app);
    assert!(app.import_on_show());
    app.storage(StorageEvent::InputUsed { id: 9 });
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputIgnore(9));
    assert!(app.import_on_show());
}

/// The boot stick found while the mouse's sheet is up over the Devices
/// sheet: the import still comes up, after both.
#[test]
fn an_import_that_arrives_under_a_mouse_over_the_devices_sheet_comes_up_after_both() {
    use faraday_core::devices::{DeviceCheck, Interface, UsbDevice};
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.devices = DeviceCheck::machine("flags\t: fpu vme", "LENOVO", "20XW", "10");
    let hid = |protocol| Interface {
        class: 3,
        subclass: 1,
        protocol,
    };
    let kvm = UsbDevice {
        port: "1-2".into(),
        vendor: 0x1d6b,
        product: 0x0104,
        manufacturer: "PiKVM".into(),
        name: "Composite KVM Device".into(),
        interfaces: vec![hid(1), hid(2)],
    };
    app.usb_devices(&[kvm]);
    assert_eq!(app.sheet, Some(Sheet::Devices));
    mouse(&mut app);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    let files = stick_files();
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputUse(9));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    app.usb_devices(&[]);
    app.press(Action::DevicesCheck);
    if app.sheet == Some(Sheet::Devices) {
        app.press(Action::DevicesUnderstood);
    }
    assert!(app.import_on_show());
}

/// The boot import under the Devices sheet under the mouse's: each comes
/// back in turn.
#[test]
fn a_mouse_over_the_devices_sheet_over_the_import_returns_each_in_turn() {
    use faraday_core::devices::{DeviceCheck, Interface, UsbDevice};
    // The vault step is the sheet the hold covers: the list step is a page.
    let mut app = imported();
    assert_eq!(app.sheet, Some(Sheet::Import));
    app.devices = DeviceCheck::machine("flags\t: fpu vme", "LENOVO", "20XW", "10");
    let hid = |protocol| Interface {
        class: 3,
        subclass: 1,
        protocol,
    };
    let kvm = UsbDevice {
        port: "1-2".into(),
        vendor: 0x1d6b,
        product: 0x0104,
        manufacturer: "PiKVM".into(),
        name: "Composite KVM Device".into(),
        interfaces: vec![
            hid(1),
            hid(2),
            Interface {
                class: 8,
                subclass: 6,
                protocol: 0x50,
            },
        ],
    };
    app.usb_devices(&[kvm]);
    app.press(Action::Vault(V::FocusPassphrase));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    mouse(&mut app);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputUse(9));
    assert_eq!(app.sheet, Some(Sheet::Devices));
    app.usb_devices(&[]);
    app.press(Action::DevicesCheck);
    // The reminders may come first, as in the test above.
    if app.sheet == Some(Sheet::Devices) {
        app.press(Action::DevicesUnderstood);
    }
    assert!(app.import_on_show());
}

/// Booted with one text picture, Import pressed and its codes read: the ask is up.
fn text_picture_asked() -> faraday_core::Faraday {
    let files: Vec<(String, Vec<u8>)> = vec![("note.png".to_string(), b"a picture".to_vec())];
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let answer = |app: &mut faraday_core::Faraday| {
        while let Some(c) = app.poll_storage() {
            match c {
                StorageCommand::Read { stick, name } => {
                    let bytes = files.iter().find(|(n, _)| *n == *name).unwrap().1.clone();
                    app.storage(StorageEvent::Read {
                        stick,
                        name,
                        bytes: bytes.into(),
                    });
                }
                StorageCommand::ReadQr { stick, name } => {
                    app.storage(StorageEvent::QrRead {
                        stick,
                        name,
                        bytes: (Vec::new()).into(),
                        payloads: (vec![b"a short note 123".to_vec()]).into(),
                    });
                }
                _ => {}
            }
        }
    };
    answer(&mut app);
    app.press(Action::Import(I::Go));
    answer(&mut app);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::TextSeed));
    app
}

#[test]
fn a_text_picture_on_the_boot_stick_loads_only_once_it_is_confirmed() {
    let mut app = text_picture_asked();
    app.press(Action::TextSeed(boot_import::TextSeedAnswer::Load));
    pull(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the confirmed key did not load");
}

#[test]
fn leaving_a_text_picture_at_the_boot_import_loads_nothing() {
    let mut app = text_picture_asked();
    app.press(Action::TextSeed(boot_import::TextSeedAnswer::Leave));
    pull(&mut app);
    assert!(app.session.keys.is_empty());
}

#[test]
fn an_ask_whose_sheet_was_replaced_does_not_hold_the_import() {
    let mut app = text_picture_asked();
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Power));
    app.press(Action::Cancel);
    pull(&mut app);
    assert!(app.import().is_none(), "the import is still held");
    assert!(app.session.keys.is_empty());
}

/// The boot sheet's vault unlocked and its Load sheet up, Load pressed so
/// that its job is out on the worker, held there.
fn load_pressed_on_worker() -> Faraday {
    let files = vec![("vault.kdbx".to_string(), testkit::test_vault().unwrap())];
    let mut app = imported_with(&files);
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.vaults().open.len(), 1);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadTicked));
    for t in 30..40 {
        if app.busy().is_some() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(app.busy().is_some(), "the seeds are loading");
    app
}

/// Back, Cancel or Lock pressed while the seeds load: the answer, when it
/// comes, adds no key.
#[test]
fn seeds_loading_when_back_cancel_or_lock_is_pressed_add_no_key() {
    for (name, action) in [
        ("Back", Action::Back),
        ("Cancel", Action::Cancel),
        ("Lock", Action::Lock),
    ] {
        let mut app = load_pressed_on_worker();
        app.press(action);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        let _ = app.frame();
        assert!(key_fps(&app).is_empty(), "{name}: {:?}", key_fps(&app));
    }
}

/// The answer, taken as it comes, adds the vault's key.
#[test]
fn seeds_loaded_on_the_worker_are_added_when_the_answer_is_taken() {
    let mut app = load_pressed_on_worker();
    assert!(
        key_fps(&app).is_empty(),
        "nothing is added before the answer"
    );
    testkit::release_worker();
    testkit::settle(&mut app);
    assert_eq!(key_fps(&app), [fp(0)]);
}

#[test]
fn a_picture_of_the_same_name_on_another_stick_is_not_taken_for_the_boot_sticks() {
    let files: Vec<(String, Vec<u8>)> = vec![("zebra-seedqr.png".into(), b"a picture".to_vec())];
    let mut app = booted_with(&files);
    app.press(Action::Import(I::Go));
    let mut asked = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::ReadQr { stick, name } = c {
            asked.push((stick, name));
        }
    }
    assert!(!asked.is_empty(), "the picture was not asked for");
    for (stick, name) in asked {
        // The other stick's picture, of the same name, answers first.
        app.storage(StorageEvent::QrRead {
            stick: faraday_text::Text::of("other"),
            name: name.clone(),
            bytes: (Vec::new()).into(),
            payloads: (vec![seed_seedqr(0)]).into(),
        });
        assert!(
            inbox_names(&app).is_empty(),
            "the other stick's code came in: {:?}",
            inbox_names(&app)
        );
        app.storage(StorageEvent::QrRead {
            stick,
            name,
            bytes: (Vec::new()).into(),
            payloads: (vec![zebra_seedqr()]).into(),
        });
    }
    assert_eq!(inbox_names(&app), ["zebra-seedqr-words.txt"]);
}

/// Only a clean process takes a stick: with a key held the stick's return
/// shows the Lock card, never the list, and Import reads nothing.
#[test]
fn a_key_held_keeps_the_list_down_and_import_asks_nothing_of_the_stick() {
    let files = stick_files();
    let mut app = pulled();
    assert!(app.import_tile_up());
    crate::common::add_key(&mut app, testkit::TEST_SEEDS[1].0);
    assert!(!app.session.keys.is_empty());
    assert!(!app.import_tile_up(), "the list stands with a key held");
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Lock));
    assert!(!app.import_tile_up(), "the list stands over the Lock card");
    app.press(Action::NotNow);
    assert_eq!(app.screen, Screen::Files);
    assert!(!app.import_tile_up());
    app.press(Action::Import(I::Go));
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    assert!(app.import_view().unwrap().files.iter().all(|f| f.enabled));
}

/// The stick arriving in a process that holds a key takes the list down,
/// not only hides it: with the key gone, it does not come back.
#[test]
fn a_stick_arriving_over_a_held_key_takes_the_list_down_for_good() {
    let files = stick_files();
    let mut app = pulled();
    crate::common::add_key(&mut app, testkit::TEST_SEEDS[1].0);
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    app.press(Action::NotNow);
    app.session.keys.remove_all();
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Files);
    assert!(
        !app.import_tile_up(),
        "the list came back with the key gone"
    );
}

/// From the stick in any other flow takes the list down as well.
#[test]
fn from_the_stick_in_a_flow_takes_the_list_down() {
    let mut app = booted_with(&stick_files());
    assert!(app.import_tile_up());
    app.press(Action::VisitFrom(Screen::SignerCheck));
    assert_eq!(app.screen, Screen::Files);
    assert!(!app.import_tile_up(), "the list stands in the flow's way");
    assert!(app.import().is_some(), "the import waits");
}

/// The reads end under a sheet that is not the import's: the person is on
/// Home, the list is gone, and the vault step comes back as the sheet closes.
fn reads_ended_under(app: &mut Faraday, files: &[(String, Vec<u8>)], cover: impl Fn(&mut Faraday)) {
    app.press(Action::Import(I::Go));
    cover(app);
    pump(app, files);
    assert!(!app.import_tile_up());
    assert_eq!(app.screen, Screen::Home, "the list is not left on Files");
}

fn unlocked_by_typing(app: &mut Faraday) {
    pull(app);
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert!(app.import_field().is_some());
    type_in(app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
}

#[test]
fn a_held_mouse_acting_over_the_reads_leaves_the_vault_step_to_come_back() {
    let files = stick_files();
    let mut app = booted_with(&files);
    reads_ended_under(&mut app, &files, mouse);
    assert_eq!(app.sheet, Some(Sheet::NewInput));
    app.press(Action::InputIgnore(9));
    unlocked_by_typing(&mut app);
}

#[test]
fn the_idle_warning_over_the_reads_leaves_the_vault_step_to_come_back() {
    let files = stick_files();
    let mut app = booted_with(&files);
    reads_ended_under(&mut app, &files, |app| {
        app.event(Event::Tick { now_ms: 0 });
        app.event(Event::Tick {
            now_ms: 26 * 60_000,
        });
        assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    });
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.event(Event::Key(Key::Char('a')));
    unlocked_by_typing(&mut app);
}

#[test]
fn the_power_sheet_over_the_reads_leaves_the_vault_step_to_come_back() {
    let files = stick_files();
    let mut app = booted_with(&files);
    reads_ended_under(&mut app, &files, |app| {
        app.press(Action::PowerAsk);
        assert_eq!(app.sheet, Some(Sheet::Power));
    });
    app.press(Action::NotNow);
    unlocked_by_typing(&mut app);
}

#[test]
fn a_sheet_closing_after_the_person_left_home_does_not_bring_the_vault_step_back() {
    let files = stick_files();
    let mut app = booted_with(&files);
    reads_ended_under(&mut app, &files, |app| {
        app.press(Action::PowerAsk);
        assert_eq!(app.sheet, Some(Sheet::Power));
    });
    app.screen = Screen::Settings;
    app.press(Action::NotNow);
    assert_eq!(app.sheet, None);
    assert_eq!(
        app.screen,
        Screen::Settings,
        "the vault step pulled the person back"
    );
}

#[test]
fn a_mouse_over_the_reads_of_an_import_with_no_vault_leaves_home() {
    let files: Vec<_> = stick_files()
        .into_iter()
        .filter(|(n, _)| n != "vault.kdbx")
        .collect();
    let mut app = booted_with(&files);
    reads_ended_under(&mut app, &files, mouse);
    app.press(Action::InputIgnore(9));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import().is_none());
}

/// The wallet of an import that asks after the list was covered: its close
/// brings the vault step back with the field typing reaches.
#[test]
fn a_wallet_asking_after_the_list_was_covered_leaves_the_field_taking_typing() {
    let mut files = two_wallet_files();
    files.push(("vault.kdbx".into(), testkit::test_vault().unwrap()));
    let mut app = booted_with(&files);
    reads_ended_under(&mut app, &files, mouse);
    app.press(Action::InputIgnore(9));
    pull(&mut app);
    app.event(Event::Tick { now_ms: 2000 });
    assert_eq!(app.sheet, Some(Sheet::WalletConfirm));
    app.press(Action::WalletCancel);
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
}

/// From the stick in another flow leaves the list: its own card draws, the
/// import waits as after Later, and the pull goes back to the flow.
#[test]
fn from_the_stick_in_another_flow_takes_the_list_down_and_the_pull_goes_back() {
    let files = stick_files();
    let mut app = booted_with(&files);
    app.press(Action::Nav(Screen::Entry));
    app.press(Action::VisitFrom(Screen::Entry));
    assert_eq!(app.screen, Screen::Files);
    assert!(!app.import_tile_up(), "the list stands in the flow's way");
    assert!(app.import().is_some(), "the import waits");
    pull(&mut app);
    assert_ne!(app.screen, Screen::Files);
}

/// The listing of the boot stick changed on Files, with the same id: what
/// a write to it from Files leaves.
fn rewritten(files: &[(String, Vec<u8>)]) -> Vec<(String, Vec<u8>)> {
    let mut changed = files.to_vec();
    changed.push(("written.txt".into(), b"new".to_vec()));
    changed
}

/// Import pressed and put off with Later: the stick coming back with
/// another listing does not end the import, whose files were read already;
/// Home's row still brings the vault step up.
#[test]
fn an_import_pressed_survives_a_changed_listing_and_homes_row_brings_the_step() {
    let files = stick_files();
    let mut app = imported_with(&files);
    app.press(Action::Import(I::Later));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&rewritten(&files))]));
    let _ = app.frame();
    app.press(boot_import::OPEN);
    assert!(app.import().is_some(), "the pressed import was ended");
    assert_eq!(app.sheet, Some(Sheet::Import));
}

/// An import not yet pressed whose stick's listing changed: Home's row
/// ends it and Files opens, where the stick's card is.
#[test]
fn homes_row_over_a_changed_listing_ends_the_import_and_opens_files() {
    let files = stick_files();
    let mut app = boot_only(&files);
    app.press(Action::Import(I::Later));
    app.press(Action::Nav(Screen::Files));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&rewritten(&files))]));
    let _ = app.frame();
    app.press(Action::Nav(Screen::Home));
    app.press(boot_import::OPEN);
    let _ = app.frame();
    assert!(app.import().is_none());
    assert_eq!(app.screen, Screen::Files, "the row ended in nothing");
    assert!(!app.import_tile_up());
}

/// The list left by the nav bar is not up: a second listing of its stick
/// still hashes nothing for a name Faraday holds.
#[test]
fn a_listing_after_the_nav_bar_left_the_list_hashes_nothing() {
    let files = stick_files();
    let mut app = boot_holding_a_listed_file(&files);
    assert_eq!(hashes(&mut app), 0);
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Nav(Screen::Home));
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    assert_eq!(hashes(&mut app), 0, "hashed before any answer");
}

/// A secret held once leaves the process unclean: Home's row
/// raises the list all the same, with Import off and the reason said, and
/// pressing it asks nothing of the stick.
#[test]
fn a_process_that_held_a_secret_raises_the_list_with_import_off_and_says_why() {
    let files = stick_files();
    let mut app = pulled();
    crate::common::add_key(&mut app, testkit::TEST_SEEDS[1].0);
    assert!(!app.clean(), "a held key leaves the process unclean");
    app.session.keys.remove_all();
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    let _ = app.frame();
    if app.sheet == Some(Sheet::Lock) {
        app.press(Action::NotNow);
    }
    assert!(app.import().is_some(), "the import ended at the arrival");
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app.press(boot_import::OPEN);
    let _ = app.frame();
    assert!(app.import_tile_up(), "Home's row raised no list");
    while app.poll_storage().is_some() {}
    app.press(Action::Import(I::Go));
    assert_eq!(reads(&mut app, &files), Vec::<String>::new());
    assert!(app.import_view().unwrap().files.iter().all(|f| f.enabled));
    let texts = drawn(&mut app);
    assert!(texts.contains("Import is off"), "{texts}");
}

/// Whether the one button reads `text` exactly.
fn button_reads(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_texts().iter().any(|t| t == text)
}

/// Unticks every file but `keep`.
fn tick_only(app: &mut Faraday, keep: &[&str]) {
    let names: Vec<String> = app
        .import_view()
        .unwrap()
        .files
        .iter()
        .map(|f| String::from(&*f.name))
        .collect();
    for n in names {
        if !keep.contains(&n.as_str()) {
            let k = row(app, &n);
            app.press(Action::Import(I::File(k)));
        }
    }
}

#[test]
fn the_button_says_it_loads_keys_while_a_ticked_name_may_hold_one() {
    let mut files = stick_files();
    files.push(("words.txt".into(), b"a note".to_vec()));
    let mut app = boot_only(&files);
    tick_only(&mut app, &["words.txt"]);
    assert!(button_reads(&mut app, "Import and load any keys"));
    // Unticked, with a PSBT ticked instead: the plain word.
    let k = row(&app, "words.txt");
    app.press(Action::Import(I::File(k)));
    let k = row(&app, "savings-unsigned.psbt");
    app.press(Action::Import(I::File(k)));
    assert!(button_reads(&mut app, "Import"));
    assert!(!button_reads(&mut app, "Import and load any keys"));
}

#[test]
fn the_button_says_import_for_a_psbt_and_a_vault() {
    let files = stick_files();
    let mut app = boot_only(&files);
    tick_only(&mut app, &["savings-unsigned.psbt", "vault.kdbx"]);
    assert!(button_reads(&mut app, "Import"));
    assert!(!button_reads(&mut app, "Import and load any keys"));
    // Pressing reads the two files ticked.
    assert_eq!(
        {
            app.press(Action::Import(I::Go));
            reads(&mut app, &files)
        },
        vec![
            "savings-unsigned.psbt".to_string(),
            "vault.kdbx".to_string()
        ]
    );
}

/// The one button's words for each name ticked alone.
#[test]
fn the_words_follow_the_extension_of_the_ticked_name_alone() {
    let names: [(&str, bool); 14] = [
        ("seed.png", true),
        ("SEED.PNG", true),
        ("readme", true),
        ("data.bin", true),
        ("sig", true),
        ("asc", true),
        ("descriptor", true),
        ("a.json", true),
        ("Note.TxT", true),
        ("wallet.descriptor", false),
        ("Wallet.DESCRIPTOR", false),
        ("key.asc", false),
        ("key.SIG", false),
        ("tx.txn", false),
    ];
    for (name, keys) in names {
        let mut files = stick_files();
        files.push((name.into(), b"x".to_vec()));
        let mut app = boot_only(&files);
        tick_only(&mut app, &[name]);
        assert_eq!(
            button_reads(&mut app, "Import and load any keys"),
            keys,
            "{name}"
        );
        assert_eq!(button_reads(&mut app, "Import"), !keys, "{name}");
    }
}

#[test]
fn a_vault_under_another_name_reads_import() {
    let mut files = stick_files();
    files.push(("backup.dat".into(), spare_vault()));
    let mut app = boot_only(&files);
    tick_only(&mut app, &["backup.dat"]);
    assert!(button_reads(&mut app, "Import"));
    assert!(!button_reads(&mut app, "Import and load any keys"));
}

#[test]
fn the_words_stay_as_they_were_at_the_press_while_the_files_are_read() {
    let files = stick_files();
    let mut app = boot_only(&files);
    tick_only(&mut app, &["notes.txt"]);
    assert!(button_reads(&mut app, "Import and load any keys"));
    app.press(Action::Import(I::Go));
    assert!(button_reads(&mut app, "Import and load any keys"));
    assert!(!button_reads(&mut app, "Import"));
    assert_eq!(reads(&mut app, &files), vec!["notes.txt".to_string()]);
}

#[test]
fn the_words_stay_when_one_file_is_answered_while_another_is_read() {
    let files = stick_files();
    // A key-named file read, one whose read fails and a picture's codes,
    // each beside a PSBT, which does not count.
    for (named, failed) in [
        ("notes.txt", false),
        ("notes.txt", true),
        ("zebra-seedqr.png", false),
    ] {
        let mut app = boot_only(&files);
        tick_only(&mut app, &[named, "savings-unsigned.psbt"]);
        assert!(button_reads(&mut app, "Import and load any keys"));
        app.press(Action::Import(I::Go));
        // Only the key-named file is answered, the PSBT still Reading.
        let mut answered = false;
        while let Some(c) = app.poll_storage() {
            match c {
                StorageCommand::Read { stick, name } if &*name == named => {
                    let bytes = files.iter().find(|(n, _)| n == named).unwrap().1.clone();
                    app.storage(if failed {
                        StorageEvent::ReadFailed {
                            stick,
                            name,
                            reason: faraday_text::Text::of("read error"),
                        }
                    } else {
                        StorageEvent::Read {
                            stick,
                            name,
                            bytes: bytes.into(),
                        }
                    });
                    answered = true;
                }
                StorageCommand::ReadQr { stick, name } if &*name == named => {
                    app.storage(StorageEvent::QrRead {
                        stick,
                        name,
                        bytes: (Vec::new()).into(),
                        payloads: (vec![zebra_seedqr()]).into(),
                    });
                    answered = true;
                }
                _ => {}
            }
        }
        assert!(answered, "{named}");
        assert!(
            button_reads(&mut app, "Import and load any keys"),
            "{named} {failed}"
        );
        assert!(!button_reads(&mut app, "Import"), "{named} {failed}");
    }
}

#[test]
fn the_words_stay_when_a_vault_the_listing_missed_is_read() {
    let mut files = stick_files();
    files.push(("backup.txt".into(), spare_vault()));
    let mut stick = boot_stick(&files);
    stick.vaults.clear();
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![stick]));
    // With a PSBT, which does not count, still being read when the vault
    // is answered.
    tick_only(&mut app, &["backup.txt", "savings-unsigned.psbt"]);
    assert!(button_reads(&mut app, "Import and load any keys"));
    app.press(Action::Import(I::Go));
    let mut asked = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            asked.push((stick, name));
        }
    }
    let (stick, name) = asked
        .into_iter()
        .find(|(_, n)| &**n == "backup.txt")
        .expect("the vault is read");
    app.storage(StorageEvent::Read {
        stick,
        name,
        bytes: spare_vault().into(),
    });
    assert!(button_reads(&mut app, "Import and load any keys"));
    assert!(!button_reads(&mut app, "Import"));
}

/// Shows the display at a size.
fn display(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.settle();
}

/// Later's and Import's boxes (y, bottom), pixels, each whole and
/// pressable.
fn foot_boxes(app: &mut Faraday, height: u16, dpi: u16) -> [(i32, i32); 2] {
    // A small panel's card is a page taller than the panel: turned to its
    // end, by the wheel at the page's edge.
    let _ = app.frame();
    for _ in 0..40 {
        app.event(Event::Wheel {
            x: 1,
            y: height / 2,
            dy: 120,
        });
        app.settle();
        let _ = app.frame();
    }
    let find = |app: &Faraday, a: Action| {
        let t = app
            .drawn_targets()
            .into_iter()
            .find(|t| t.0 == a)
            .unwrap_or_else(|| panic!("{a:?} not drawn"));
        assert!(!t.3, "{a:?} is cut");
        assert!(
            t.2.y >= 0 && t.2.y + t.2.h <= i32::from(height),
            "{a:?} off the panel"
        );
        (t.2.y, t.2.y + t.2.h)
    };
    let boxes = [
        find(app, Action::Import(I::Later)),
        find(app, Action::Import(I::Go)),
    ];
    // The card's own box is not drawn to a test. At the page's end the
    // card leaves at least the page inset (48 dp) under it, and the
    // buttons stand VISIT_PAD (20 dp) inside its foot: so what is left
    // under Import is at least their sum. A card counting one row of
    // buttons while they stack ends above Import's bottom, and leaves
    // less.
    let left = i32::from(height) - boxes[1].1;
    let least = i32::from(dpi) * (48 + 20) / 160;
    assert!(
        left >= least,
        "Import ends {left} px above the panel's foot, under the card's own {least}"
    );
    boxes
}

#[test]
fn at_small_sizes_both_buttons_stay_whole_and_the_same_ticked_or_not() {
    for (w, h, dpi) in [(320, 480, 160), (480, 640, 286), (240, 320, 160)] {
        let mut files = stick_files();
        files.push(("words.txt".into(), b"a note".to_vec()));
        let mut app = boot_only(&files);
        display(&mut app, w, h, dpi);
        // A key-named file ticked: the long words.
        tick_only(&mut app, &["words.txt"]);
        let long = foot_boxes(&mut app, h, dpi);
        // A PSBT ticked instead: the short words, the same boxes.
        let k = row(&app, "words.txt");
        app.press(Action::Import(I::File(k)));
        let k = row(&app, "savings-unsigned.psbt");
        app.press(Action::Import(I::File(k)));
        let short = foot_boxes(&mut app, h, dpi);
        assert_eq!(long, short, "{w}x{h} at {dpi}");
    }
}
