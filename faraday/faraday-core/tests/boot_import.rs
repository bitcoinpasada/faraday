//! The boot import: the first look at the boot stick copies its files
//! into memory with only its vault in the Inbox; once the stick is out,
//! the wallets its text files and pictures hold are listed with whether
//! they can sign, a vault unlocked from the sheet adds its own, and
//! Import loads what is chosen, moves the chosen files into the Inbox and
//! wipes the rest. Import later leaves it waiting behind Sticks; a lock
//! drops it. A picture ticked on a stick visit has its codes read.

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

/// Pulled, and the vault unlocked from the sheet.
fn unlocked() -> Faraday {
    let mut app = pulled();
    app.press(Action::Import(I::Unlock(0)));
    assert_eq!(app.screen, Screen::Unlock);
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    let _ = app.frame();
    app
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
fn pulling_the_stick_lists_a_wallet_with_its_key_from_a_picture() {
    let app = pulled();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.sheet, Some(Sheet::Import));
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
    // The PSBT is chosen for the Inbox; the note is not, and nor is the
    // photo, which is read as a File and can still be chosen.
    assert!(file(&view, "savings-unsigned.psbt").chosen);
    assert!(!file(&view, "notes.txt").chosen);
    let photo = file(&view, "photo.jpg");
    assert!(photo.enabled && !photo.chosen);
    assert!(file(&view, "vault.ofv").chosen);
    assert!(app.offers(Action::Import(I::Go)));
    assert!(app.offers(Action::Import(I::Unlock(0))));
}

#[test]
fn a_vault_unlocked_from_the_sheet_comes_back_to_it_with_its_wallets() {
    let app = unlocked();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let view = app.import_view().unwrap();
    // Savings is 2 of 3 over test keys 1 (in the vault), 2 (the picture)
    // and 3 (the words file).
    let savings = wallet(&view, "Savings").expect("the vault's wallets are not listed");
    assert_eq!(savings.status(), "Can sign");
    assert!(savings.files.contains(&"vault.ofv".to_string()));
    assert!(
        !view.keys.iter().any(|k| k.fingerprint == fp(2)),
        "test key 3 is used by Savings now"
    );
    // An open vault's file stays in the Inbox.
    let v = file(&view, "vault.ofv");
    assert!(v.chosen && !v.enabled);
    // Back from Unlock comes back to the sheet too.
    let mut app = pulled();
    app.press(Action::Import(I::Unlock(0)));
    let _ = app.frame();
    assert!(app.offers(boot_import::OPEN), "no way back to the import");
    app.press(boot_import::OPEN);
    assert_eq!((app.screen, app.sheet), (Screen::Home, Some(Sheet::Import)));
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
    let mut app = booted_with(&files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let view = app.import_view().unwrap();
    assert_eq!(
        wallet(&view, "Savings").unwrap().status(),
        "1 of 3 keys here · 1 more needed"
    );
    assert_eq!(wallet(&view, "Spending").unwrap().status(), "Watch-only");
}

#[test]
fn import_loads_what_is_chosen_and_wipes_the_rest() {
    let mut app = unlocked();
    let view = app.import_view().unwrap();
    let spending = view
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .unwrap();
    let notes = view
        .files
        .iter()
        .position(|f| f.name == "notes.txt")
        .unwrap();
    let wanted = view.wallets.len() - 1;
    app.press(Action::Import(I::Wallet(spending)));
    app.press(Action::Import(I::File(notes)));
    app.press(Action::Import(I::Go));
    assert!(app.import.is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert_eq!(app.session.wallets.len(), wanted);
    assert!(!app.session.wallets.iter().any(|w| w.name == "Spending"));
    assert!(app.session.wallets.iter().any(|w| w.name == "Zebra"));
    // Test keys 1, 2 and 3: the vault's, the picture's and the words'.
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
    // The chosen files are in the Inbox; the rest are gone.
    let mut inbox: Vec<&str> = app.inbox.iter().map(|i| i.name.as_str()).collect();
    inbox.sort();
    assert_eq!(inbox, ["notes.txt", "savings-unsigned.psbt", "vault.ofv"]);
    assert!(!app.inbox.iter().any(|i| i.kind == FileKind::Words));
    // And Sticks has nothing left to open.
    let _ = app.frame();
    assert!(!app.offers(boot_import::OPEN));
}

#[test]
fn a_locked_vault_left_out_leaves_the_inbox_and_its_wallets_are_not_listed() {
    let mut app = pulled();
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
fn import_later_goes_home_and_sticks_opens_it_again() {
    let mut app = pulled();
    app.press(Action::Import(I::Later));
    assert_eq!((app.screen, app.sheet), (Screen::Home, None));
    assert!(app.import.is_some());
    let _ = app.frame();
    assert!(app.offers(boot_import::OPEN));
    app.press(boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    // A tap beside the sheet is Import later.
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
    let mut app = unlocked();
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
fn a_picture_ticked_on_a_visit_has_its_codes_read_into_the_inbox() {
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
    app.press(Action::VisitIn(0));
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
    app.storage(StorageEvent::Sticks(Vec::new()));
    let view = app.import_view().unwrap();
    assert!(
        view.keys.iter().any(|k| k.fingerprint == fp(0) && k.chosen),
        "the seed is not offered"
    );
    app.press(Action::Import(I::Go));
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
fn leaving_unlock_by_nav_clears_the_way_back_to_the_import_sheet() {
    let mut app = pulled();
    app.press(Action::Import(I::Unlock(0)));
    assert_eq!(app.screen, Screen::Unlock);
    assert_eq!(app.vaults.back_to, Some(Screen::Home));
    // Left some other way than Back or a successful unlock: sidebar nav.
    app.press(Action::Nav(Screen::Vaults));
    assert_eq!(app.screen, Screen::Vaults);
    // Unlock begun again, from the Vaults list rather than the sheet.
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    assert_eq!(
        app.vaults.back_to, None,
        "a stale way back to the import sheet lingers"
    );
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    let _ = app.frame();
    // Back to the vault list, not the import sheet: the stale way back
    // would otherwise reopen it.
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
    assert!(app.import.is_some());
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
