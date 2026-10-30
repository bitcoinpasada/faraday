//! BIP-85 from a loaded key: a child seed loads as a key of its own, a
//! password goes into the open vault as an entry, nothing leaves
//! unprotected without the secret sheet, and a password out (for KeePass
//! or as a file) names neither the key it came from nor its index.

use crate::common::type_text;
use faraday_bip::bip39::Language;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};
use faraday_vault::records::{field, kind};

fn with_key(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
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
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1);
    app
}

#[test]
fn a_child_seed_loads_as_a_key_of_its_own() {
    let c = testkit::compute();
    let mut app = with_key(Vec::new());
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85);
    // Words, 12 of them, at index 7 typed on the keyboard.
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PLoad);
    app.press(Action::PRecord(false));
    assert_eq!(app.session.keys.len(), 2);
    let master = &app.session.keys.master_for_tests(0);
    let child = faraday_bip::bip85::child_mnemonic(&c, master, Language::English, 12, 7).unwrap();
    let words: Vec<&str> = child
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let mut probe = faraday_core::wallet::Session::default();
    let fp = probe
        .keys
        .add_words_with(&c, &words.join(" "), "", "", None)
        .unwrap();
    assert_eq!(app.session.keys[1].fingerprint, fp);
}

#[test]
fn a_value_goes_out_only_through_the_secret_sheet() {
    let mut app = with_key(Vec::new());
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PVault);
    assert!(app.outbox.is_empty(), "no vault is open");
    app.press(Action::POut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    assert_eq!(app.outbox.len(), 1);
}

#[test]
fn a_password_goes_into_the_vault_as_an_entry() {
    let c = testkit::compute();
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
        .unwrap();
    let mut app = with_key(vec![vault]);
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PVault);
    let want =
        faraday_bip::bip85::child_password_base64(&c, app.session.keys.master_for_tests(0), 21, 0)
            .unwrap();
    let v = &app.vaults().open[0];
    let found = app
        .vaults()
        .store()
        .contents_for_tests(v.id)
        .of(kind::ENTRY)
        .any(|(_, r)| r.field(field::PASSWORD) == Some(want.as_str().as_bytes()));
    assert!(found, "the password is not an entry in the vault");
    assert!(app.outbox.is_empty());
}

/// What `bytes` hold of the key a password came from: its fingerprint,
/// in either case, BIP-85's name, the index's word or the path's purpose.
fn names_the_parent(app: &Faraday, bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let fp = faraday_keys::identity::fp_text(app.session.keys[0].fingerprint);
    [
        fp.to_uppercase(),
        fp.to_lowercase(),
        "BIP-85".into(),
        "index".into(),
        "83696968".into(),
    ]
    .into_iter()
    .find(|w| text.contains(w.as_str()))
}

#[test]
fn an_exported_password_names_neither_its_parent_nor_its_index() {
    let c = testkit::compute();
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
        .unwrap();
    let mut app = with_key(vec![vault]);
    for i in 0..12u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut app, 1);
    assert_eq!(app.vaults().open.len(), 1);
    // A base64 password at index 7, into the vault.
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PVault);
    let want =
        faraday_bip::bip85::child_password_base64(&c, app.session.keys.master_for_tests(0), 21, 7)
            .unwrap();
    // Exported for KeePass.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(2)));
    let id = app.vaults().open[0].id;
    let at = app
        .vaults()
        .store()
        .contents_for_tests(id)
        .of(kind::ENTRY)
        .position(|(_, r)| r.field(field::PASSWORD) == Some(want.as_str().as_bytes()))
        .expect("the password's entry");
    app.press(Action::Vault(V::Item(at)));
    app.press(Action::Vault(V::ExportKdbx));
    type_text(&mut app, "keepass passphrase");
    app.press(Action::Vault(V::PromptGo));
    crate::common::settle(&mut app, 30);
    let file = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".kdbx"))
        .expect("no KeePass file");
    assert_eq!(file.name, "password.kdbx");
    let body = faraday_backup::kdbx::open_for_tests(&c, &file.bytes, b"keepass passphrase")
        .expect("the file opens under its passphrase");
    let xml = String::from_utf8_lossy(&body);
    assert!(
        xml.contains("<Key>Title</Key><Value>Password</Value>"),
        "{xml}"
    );
    assert_eq!(names_the_parent(&app, &body), None, "{xml}");
    assert_eq!(names_the_parent(&app, file.name.as_bytes()), None);
    // The same password out as a file, unprotected: itself alone.
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::POut);
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    let out = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".txt"))
        .expect("no password file");
    assert_eq!(out.name, "password.txt");
    assert_eq!(out.bytes, format!("{}\n", want.as_str()).into_bytes());
}
