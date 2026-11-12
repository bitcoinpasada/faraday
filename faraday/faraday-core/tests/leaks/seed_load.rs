//! Loading a seed: typed into Add a key, pasted into the scanner and
//! loaded with a passphrase, and loaded from a vault by the unlock's Load
//! sheet (the unlock's passphrase typed on the way), each ended by Lock.

use crate::common::{device, type_text};
use crate::forms::{PASSPHRASE, PHRASE, VAULT_PASSPHRASE, seed_forms, vault_passphrase_forms};
use crate::{arm, finish};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::seeds::read::Choice;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::Session;
use faraday_core::{Action, DESKTOP_MODES, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Command, Event, FileKind};
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

/// Faraday Demo on Testnet, its clipboard read by Paste, Add a key open.
fn demo_on_testnet() -> Faraday {
    let mut app = testkit::started();
    app.clipboard = true;
    app.online = true;
    app.session = Session::on(testkit::NET);
    app.set_modes(DESKTOP_MODES);
    app.press(Action::Entry(None));
    let _ = app.frame();
    app
}

/// Paste pressed and the clipboard holding `text`.
fn paste(app: &mut Faraday, text: String) {
    app.press(Action::ScanPaste);
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, Command::RequestClipboard { .. }));
    assert!(asked, "Paste did not ask for the clipboard");
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text,
    });
    let _ = app.frame();
}

/// A lock as a person's: the pool filled for a seal, Lock pressed, the
/// worker's answers taken.
pub fn lock(app: &mut Faraday) {
    crate::common::lock(app);
    assert!(app.session.keys.is_empty(), "Lock left a key");
}

#[test]
fn a_seed_typed_into_add_a_key_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = demo_on_testnet();
    type_text(&mut app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    lock(&mut app);
    finish("seed typed", armed, true);
}

#[test]
fn a_seed_pasted_and_loaded_with_a_passphrase_leaves_no_copy_after_lock() {
    let forms = seed_forms(PASSPHRASE);
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.clipboard = true;
    app.press(Action::ScanSeed);
    paste(&mut app, PHRASE.to_string());
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(Choice::AddPass));
    app.press(Action::SeedRead(Choice::Field(0)));
    type_text(&mut app, PASSPHRASE);
    app.press(Action::SeedRead(Choice::Field(1)));
    type_text(&mut app, PASSPHRASE);
    testkit::pause(&mut app);
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    lock(&mut app);
    finish("seed pasted with a passphrase", armed, true);
}

/// A vault of one key, its words and its own passphrase kept, sealed
/// under `VAULT_PASSPHRASE`.
fn vault() -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    let key = Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_LABEL, b"Leak key")
        .with(field::KEY_PASSPHRASE, PASSPHRASE.as_bytes());
    crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x5d, vec![key])
}

/// Unlocks the vault from the Vaults tab, its passphrase typed.
pub fn unlock(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
}

#[test]
fn a_vaults_seed_loaded_from_the_load_sheet_leaves_no_copy_after_lock() {
    load_flow();
}

fn load_flow() {
    let file = vault();
    let mut forms = seed_forms(PASSPHRASE);
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad), "the Load sheet asks");
    // The load runs on the worker, as a person's press has it.
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadTicked));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the vault's key did not load");
    lock(&mut app);
    finish("vault load", armed, true);
}

#[test]
fn a_vault_unlocked_and_locked_without_loading_leaves_no_copy() {
    let file = vault();
    let forms = vault_passphrase_forms();
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    app.press(Action::Vault(V::LoadNotNow));
    lock(&mut app);
    finish("vault unlock", armed, true);
}

/// A vault with a seed loaded from it and a change not sealed, then Lock:
/// the seal runs on the worker while the lock closes the session.
#[test]
fn a_lock_pressed_while_the_vault_seals_leaves_no_copy() {
    let file = vault();
    let mut forms = seed_forms(PASSPHRASE);
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(app.session.keys.len(), 1, "the vault's key did not load");
    testkit::vault_bump(&mut app, 0);
    crate::common::top_up(&mut app);
    testkit::press_on_worker(&mut app, Action::Lock);
    testkit::settle(&mut app);
    assert!(app.session.keys.is_empty(), "Lock left a key");
    finish("vault seal at lock", armed, true);
}
