//! The boot import: the first look at the boot stick copies its files
//! into memory with only its vault From the stick, and the sheet says what
//! was read and to pull the stick. Once it is out, the vault's passphrase
//! is typed on the sheet, and the unlock imports everything; a wrong one
//! stays on the sheet. Choose what to import lists the wallets its text
//! files and pictures hold with whether they can sign, and Import loads
//! what is ticked, moves the ticked files From the stick and wipes the
//! rest. With no vault everything comes in as the stick is pulled; with
//! only the settings file there is no sheet. Not now leaves Home leading
//! with the vault's Unlock; a lock drops the import.

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

/// Pulled, put off with Not now, and the vault unlocked on Vaults: the
/// import still waits.
fn unlocked_elsewhere() -> Faraday {
    let mut app = pulled();
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

fn wallet<'a>(
    view: &'a boot_import::ImportView,
    name: &str,
) -> Option<&'a boot_import::ImportWallet> {
    view.wallets.iter().find(|w| w.name == name)
}

fn file<'a>(view: &'a boot_import::ImportView, name: &str) -> &'a boot_import::ImportFile {
    view.files.iter().find(|f| f.name == name).unwrap()
}

#[test]
fn the_boot_stick_is_copied_into_memory_with_only_its_vault_in_the_inbox() {
    let mut app = booted();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let inbox: Vec<&str> = app.inbox.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(inbox, ["vault.ofv"]);
    let c = app.import_count().unwrap();
    assert_eq!((c.copied, c.not_read, c.vaults), (7, 0, 1));
    // Nothing loads, and nothing is imported, while it is in.
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Later)));
    assert!(!app.offers(Action::Import(I::Go)));
    app.press(Action::Import(I::Go));
    assert!(app.session.keys.is_empty() && app.import.is_some());
}

#[test]
fn with_the_stick_in_the_sheet_says_what_was_read_and_to_pull_it() {
    let mut app = booted();
    let texts = drawn(&mut app);
    assert!(texts.contains("Read from Boot stick"), "{texts}");
    assert!(
        texts.contains("1 vault · 1 PSBT · 5 other files"),
        "{texts}"
    );
    assert!(texts.contains("Pull the stick to continue"), "{texts}");
    // No lists, no passphrase and no import while it is in.
    assert!(!texts.contains("zebra-wallet.txt"), "{texts}");
    assert!(!app.offers(Action::Import(I::Choose)));
    assert!(!app.offers(Action::Import(I::Submit)));
    assert!(!app.offers(Action::Import(I::Unlock(0))));
    assert!(app.offers(Action::Import(I::Later)));
}

#[test]
fn after_the_pull_with_one_vault_the_sheet_takes_its_passphrase() {
    let mut app = pulled();
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    let texts = drawn(&mut app);
    assert!(texts.contains("Read from Boot stick"), "{texts}");
    assert!(texts.contains("vault.ofv"), "{texts}");
    assert!(texts.contains("Choose what to import"), "{texts}");
    assert!(
        app.offers(Action::Import(I::Submit)),
        "no Unlock on the sheet"
    );
    assert!(app.offers(Action::Import(I::Later)), "no Not now");
    assert!(app.offers(Action::Import(I::Choose)));
    // Typing goes to the field at once.
    type_in(&mut app, "abc");
    assert_eq!(app.vaults.passphrase.text.len(), 3);
}

#[test]
fn a_right_passphrase_on_the_sheet_imports_everything_and_closes_it() {
    let mut app = pulled();
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    assert!(app.import.is_none());
    // Wallets loaded: Wallets (`docs/NEW-WALLET.md` §11.1).
    assert_eq!((app.screen, app.sheet), (Screen::Wallets, None));
    // The vault's wallets, the stick's own, and test keys 1, 2 and 3:
    // the vault's, the picture's and the words'.
    for name in ["Savings", "Spending", "Zebra"] {
        assert!(
            app.session.wallets.iter().any(|w| w.name == name),
            "{name} is not loaded"
        );
    }
    let mut keys: Vec<String> = app
        .session
        .keys
        .iter()
        .map(|k| faraday_core::wallet::fp_text(k.master.fingerprint()))
        .collect();
    keys.sort();
    let mut want = vec![fp(0), fp(1), fp(2)];
    want.sort();
    assert_eq!(keys, want);
    // Every file but the seeds' comes in.
    let mut inbox: Vec<&str> = app.inbox.iter().map(|i| i.name.as_str()).collect();
    inbox.sort();
    assert_eq!(
        inbox,
        [
            "notes.txt",
            "photo.jpg",
            "savings-unsigned.psbt",
            "vault.ofv",
            "zebra-wallet.txt"
        ]
    );
}

#[test]
fn a_wrong_passphrase_stays_on_the_sheet_and_says_so() {
    let mut app = pulled();
    type_in(&mut app, "not the passphrase");
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert!(app.vaults.open.is_empty());
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
    assert!(app.import.is_some() && app.session.keys.is_empty());
    let error = app.vaults.unlock_error.clone().expect("no error said");
    assert!(
        drawn(&mut app).contains(&error),
        "the error is not on the sheet"
    );
    // And the right one, typed again, imports.
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert!(app.import.is_none() && !app.session.keys.is_empty());
}

#[test]
fn not_now_leaves_home_leading_with_the_vaults_unlock() {
    let mut app = pulled();
    type_in(&mut app, "half typ");
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import.is_some());
    assert!(
        app.vaults.passphrase.text.is_empty(),
        "what was typed stays"
    );
    let texts = drawn(&mut app);
    assert!(texts.contains("Unlock vault.ofv"), "{texts}");
    assert!(app.offers(boot_import::OPEN));
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Submit)));
}

#[test]
fn with_two_vaults_each_is_unlocked_from_its_row_and_one_imports_everything() {
    let mut files = stick_files();
    files.push(("spare.ofv".into(), spare_vault()));
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    let view = app.import_view().unwrap();
    assert_eq!(view.vaults.len(), 2);
    let test = view
        .vaults
        .iter()
        .position(|v| v.name == "vault.ofv")
        .unwrap();
    // No field until a row's Unlock is pressed.
    assert!(app.offers(Action::Import(I::Unlock(0))));
    assert!(app.offers(Action::Import(I::Unlock(1))));
    assert!(!app.offers(Action::Import(I::Submit)));
    app.press(Action::Import(I::Unlock(test)));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Submit)));
    assert!(!app.offers(Action::Import(I::Unlock(test))));
    type_in(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Import(I::Submit));
    run_unlock(&mut app);
    assert!(app.import.is_none());
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    // The other stays in Files, locked.
    let spare = app
        .vault_files()
        .into_iter()
        .find(|f| f.name == "spare.ofv")
        .expect("the other vault is gone");
    assert!(spare.open.is_none());
}

#[test]
fn with_no_vault_the_files_come_in_as_the_stick_is_pulled() {
    let files: Vec<(String, Vec<u8>)> = stick_files()
        .into_iter()
        .filter(|(n, _)| n != "vault.ofv")
        .collect();
    let mut app = booted_with(&files);
    assert_eq!(app.sheet, Some(Sheet::Import));
    app.storage(StorageEvent::Sticks(Vec::new()));
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
fn choose_what_to_import_lists_a_wallet_with_its_key_from_a_picture() {
    let mut app = pulled();
    app.press(Action::Import(I::Choose));
    let _ = app.frame();
    assert!(app.offers(Action::Import(I::Go)));
    let texts = drawn(&mut app);
    assert!(texts.contains("Zebra"), "{texts}");
    let view = app.import_view().unwrap();
    let zebra = wallet(&view, "Zebra").expect("the wallet file is not listed");
    assert!(zebra.chosen);
    assert_eq!(zebra.status(), "Can sign");
    assert!(zebra.files.contains(&"zebra-wallet.txt".to_string()));
    assert!(
        zebra.files.contains(&"zebra-seedqr.png".to_string()),
        "the picture that holds its key is not named: {:?}",
        zebra.files
    );
    // Test key 3: no wallet on the stick uses it.
    assert!(
        view.keys
            .iter()
            .any(|k| k.fingerprint == fp(2) && k.chosen && k.files == ["summer-words.txt"])
    );
    // Every file is ticked but the seeds': the words file and the
    // picture of a SeedQR, whose keys come in without them.
    for name in [
        "savings-unsigned.psbt",
        "notes.txt",
        "photo.jpg",
        "vault.ofv",
        "zebra-wallet.txt",
    ] {
        assert!(file(&view, name).chosen, "{name} is not ticked");
    }
    assert!(!file(&view, "summer-words.txt").chosen);
    assert!(!file(&view, "zebra-seedqr.png").chosen);
}

#[test]
fn a_vault_unlocked_elsewhere_has_already_loaded_its_wallets_off_the_sheet() {
    let mut app = unlocked_elsewhere();
    // Savings loaded already with the rest of the vault's wallets
    // (`docs/NEW-WALLET.md` §11.1a), so the sheet leaves it out: only
    // Zebra, from the stick's own file, still needs choosing.
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let view = app.import_view().unwrap();
    assert!(
        wallet(&view, "Savings").is_none(),
        "a wallet loaded already is not offered again"
    );
    let zebra = wallet(&view, "Zebra").expect("the stick's own wallet is not listed");
    assert_eq!(zebra.status(), "Can sign");
    // Test key 1, the vault's own, loaded with Savings; it is not
    // offered again either. Test key 3 (the words file) is not used by
    // any wallet left on the sheet, so it stands on its own now.
    assert!(!view.keys.iter().any(|k| k.fingerprint == fp(0)));
    assert!(view.keys.iter().any(|k| k.fingerprint == fp(2)));
    // An open vault's file stays From the stick, and has no field.
    let v = file(&view, "vault.ofv");
    assert!(v.chosen && !v.enabled);
    assert_eq!(app.import_field(), None);
}

#[test]
fn a_wallet_short_of_keys_says_how_many_more_it_needs() {
    // Savings, 2 of 3, with test key 2 in a picture; Spending with no key.
    let kit = testkit::files().unwrap();
    let mut files: Vec<(String, Vec<u8>)> = kit
        .into_iter()
        .filter(|(n, _)| n == "savings-wallet.txt" || n == "spending-wallet.txt")
        .collect();
    files.push(("zebra-seedqr.png".into(), b"a picture".to_vec()));
    files.push(("vault.ofv".into(), testkit::test_vault().unwrap()));
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Import(I::Choose));
    let view = app.import_view().unwrap();
    assert_eq!(
        wallet(&view, "Savings").unwrap().status(),
        "1 of 3 keys here · 1 more needed"
    );
    assert_eq!(wallet(&view, "Spending").unwrap().status(), "Watch-only");
}

#[test]
fn import_loads_what_is_ticked_and_wipes_the_rest() {
    let mut app = pulled();
    app.press(Action::Import(I::Choose));
    let view = app.import_view().unwrap();
    let notes = view
        .files
        .iter()
        .position(|f| f.name == "notes.txt")
        .unwrap();
    let summer = view
        .keys
        .iter()
        .position(|k| k.fingerprint == fp(2))
        .unwrap();
    app.press(Action::Import(I::File(notes)));
    app.press(Action::Import(I::Key(summer)));
    app.press(Action::Import(I::Go));
    assert!(app.import.is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.vaults.open.is_empty(), "Import unlocked the vault");
    // Zebra and its key from the picture; not test key 3.
    let names: Vec<&str> = app
        .session
        .wallets
        .iter()
        .map(|w| w.name.as_str())
        .collect();
    assert_eq!(names, ["Zebra"]);
    let keys: Vec<String> = app
        .session
        .keys
        .iter()
        .map(|k| faraday_core::wallet::fp_text(k.master.fingerprint()))
        .collect();
    assert_eq!(keys, [fp(1)]);
    // The ticked files are From the stick; the rest are gone.
    let mut inbox: Vec<&str> = app.inbox.iter().map(|i| i.name.as_str()).collect();
    inbox.sort();
    assert_eq!(
        inbox,
        [
            "photo.jpg",
            "savings-unsigned.psbt",
            "vault.ofv",
            "zebra-wallet.txt"
        ]
    );
    assert!(!app.inbox.iter().any(|i| i.kind == FileKind::Words));
    // And Home has nothing left to open.
    let _ = app.frame();
    assert!(!app.offers(boot_import::OPEN));
}

#[test]
fn a_locked_vault_left_out_leaves_the_inbox_and_its_wallets_are_not_listed() {
    let mut app = pulled();
    app.press(Action::Import(I::Choose));
    let view = app.import_view().unwrap();
    assert!(wallet(&view, "Savings").is_none());
    let k = view
        .files
        .iter()
        .position(|f| f.name == "vault.ofv")
        .unwrap();
    app.press(Action::Import(I::File(k)));
    app.press(Action::Import(I::Go));
    assert!(!app.inbox.iter().any(|i| i.name == "vault.ofv"));
    // Zebra and its key, and test key 3 on its own.
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.keys.len(), 2);
}

#[test]
fn not_now_goes_home_and_the_lead_opens_it_again() {
    let mut app = pulled();
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import.is_some());
    let _ = app.frame();
    assert!(app.offers(boot_import::OPEN));
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    // A tap beside the sheet is Not now.
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
    assert!(app.import.is_some());
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
    assert!(
        inbox.iter().all(|(n, _)| n == "vault.ofv"),
        "what was not imported was kept: {:?}",
        inbox.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
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
    // No vault: pulling the stick imports it.
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert!(app.import.is_none());
    let key = app
        .session
        .keys
        .iter()
        .find(|k| faraday_core::wallet::fp_text(k.master.fingerprint()) == fp(0))
        .expect("the seed did not load");
    assert_eq!(
        key.label,
        format!("{stem}-words"),
        "the picture's label won over the words file's"
    );
}

#[test]
fn a_vault_unlocked_on_vaults_while_the_import_waits_loads_its_own_wallets() {
    let app = unlocked_elsewhere();
    // Unlocked through the list, not the sheet (`docs/NEW-WALLET.md`
    // §11.1a): Wallets, with the vault's own wallets and keys loaded;
    // the stick's import still waits, untouched.
    assert_eq!((app.screen, app.sheet), (Screen::Wallets, None));
    assert!(app.import.is_some());
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
