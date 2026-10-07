//! BIP-85 from a loaded key: a child seed loads as a key of its own, a
//! password goes into the open vault as an entry, and nothing leaves
//! unprotected without the secret sheet.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_vault::records::{field, kind};
use osk_bip::bip39::Language;
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

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
    assert_eq!(app.session.keys.len(), 2);
    let master = &app.session.keys[0].master;
    let child = osk_bip::bip85::child_mnemonic(master, Language::English, 12, 7).unwrap();
    let words: Vec<&str> = child
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let mut probe = faraday_core::wallet::Session::default();
    let fp = probe
        .add_words_with(&words.join(" "), "", "", None)
        .unwrap();
    assert_eq!(app.session.keys[1].master.fingerprint(), fp);
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
    app.press(Action::SecretAck);
    app.press(Action::SecretUnprotected);
    assert_eq!(app.outbox.len(), 1);
}

#[test]
fn a_password_goes_into_the_vault_as_an_entry() {
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap();
    let mut app = with_key(vec![vault]);
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1);
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PVault);
    let want = osk_bip::bip85::child_password_base64(&app.session.keys[0].master, 21, 0).unwrap();
    let v = &app.vaults.open[0];
    let found = v
        .contents
        .of(kind::ENTRY)
        .any(|(_, r)| r.field(field::PASSWORD) == Some(want.as_str().as_bytes()));
    assert!(found, "the password is not an entry in the vault");
    assert!(app.outbox.is_empty());
}
