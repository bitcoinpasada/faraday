//! The flows `reveals.rs` names for the reasons no other flow reaches: a
//! vault's GPG key shown by its eye and its revocation let out, a note
//! listed and opened, a Lightning key shown and put in a vault, a Silent
//! Payments scan key put in a vault and let out, and a key file made for
//! a vault. Each ends by Lock.

use crate::common::{device, type_text};
use crate::forms::{PHRASE, VAULT_PASSPHRASE, seed_forms, vault_passphrase_forms};
use crate::readers::{open_item, paper_seeds, unlock};
use crate::{arm, finish};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

/// The test seed typed into Add a key, on a device holding `vault`.
fn with_vault_and_key(vault: Vec<u8>) -> Faraday {
    let mut app = device(vec![("cellar.kdbx".into(), vault)]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(&mut app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    testkit::keep_published(&mut app);
    unlock(&mut app, 0);
    app
}

/// A GPG record of the paperkey vector's seeds.
fn gpg_record() -> (Record, Vec<Form>) {
    let (primary, subkey) = paper_seeds();
    let created = 1_780_000_000u32.to_le_bytes();
    let record = Record::new(kind::GPG)
        .with(1, &primary)
        .with(2, &created)
        .with(3, &subkey)
        .with(4, &created)
        .with(5, b"Test Person <test@example.com>")
        .with(6, &(2 * 31_536_000u32).to_le_bytes());
    let forms = vec![
        Form::new("GPG primary seed", primary.to_vec()),
        Form::new("GPG subkey seed", subkey.to_vec()),
    ];
    (record, forms)
}

/// The vault GPG flow; `probe` is whether it returns the revocation
/// certificate it let out, as forms to look for.
fn gpg_flow(record: Record, probe: bool) -> Vec<Form> {
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x71, vec![record]);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app, 0);
    app.storage(faraday_core::StorageEvent::Clock {
        unix_secs: 1_790_000_000,
    });
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    open_item(&mut app, 4);
    app.press(Action::Vault(V::Reveal(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::Reveal(0)));
    let _ = app.frame();
    app.press(Action::Vault(V::GpgExportRevocation));
    assert_eq!(
        app.sheet,
        Some(Sheet::SecretOut),
        "no revocation was offered"
    );
    let _ = app.frame();
    let found = if probe {
        let out = testkit::secret_out(&app).expect("no revocation was offered");
        vec![Form::new("revocation certificate", out.bytes.to_vec())]
    } else {
        Vec::new()
    };
    crate::seed_load::lock(&mut app);
    found
}

#[test]
fn a_vaults_gpg_key_shown_by_its_eye_and_its_revocation_let_out_leave_no_copy_after_lock() {
    let (record, mut forms) = gpg_record();
    // The seeds as the paperkey the eye opens holds them.
    forms.extend(crate::areas::paperkey_forms());
    forms.extend(vault_passphrase_forms());
    forms.extend(gpg_flow(gpg_record().0, true));
    let armed = arm(&forms);
    gpg_flow(record, false);
    finish("vault GPG eye and revocation", armed, true);
}

#[test]
fn a_vaults_note_listed_and_opened_leaves_no_copy_after_lock() {
    let note = Record::new(kind::NOTE).with(field::NOTE, crate::forms::ENTRY_NOTES.as_bytes());
    let mut forms = vault_passphrase_forms();
    forms.extend(crate::forms::entry_forms());
    let armed = arm(&forms);
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x72, vec![note]);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(3)));
    let _ = app.frame();
    open_item(&mut app, 3);
    crate::seed_load::lock(&mut app);
    finish("vault note opened", armed, true);
}

/// The bytes of a hex text, upper or lower case.
fn unhex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

/// The Lightning flow; `probe` is whether it returns the node key it drew.
fn lightning_flow(probe: bool) -> Vec<Form> {
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x73, Vec::new());
    let mut app = with_vault_and_key(file);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::LKey(fp));
    app.press(Action::LShow);
    let _ = app.frame();
    let found = if probe {
        let hex = app
            .drawn_texts()
            .into_iter()
            .find(|t| t.len() == 64 && t.bytes().all(|b| b.is_ascii_hexdigit()))
            .expect("the node key was not drawn");
        vec![
            Form::new("node private key text", hex.as_bytes().to_vec()),
            Form::new("node private key", unhex(hex.as_str())),
        ]
    } else {
        Vec::new()
    };
    app.press(Action::LShow);
    let _ = app.frame();
    app.press(Action::LVault);
    while app.poll_storage().is_some() {}
    crate::seed_load::lock(&mut app);
    while app.poll_storage().is_some() {}
    found
}

#[test]
fn a_lightning_key_shown_and_put_in_a_vault_leaves_no_copy_after_lock() {
    let mut forms = seed_forms("");
    forms.extend(vault_passphrase_forms());
    forms.extend(lightning_flow(true));
    let armed = arm(&forms);
    lightning_flow(false);
    finish("lightning key shown, put in a vault", armed, true);
}

/// The scan key flow; `probe` is whether it returns the key it let out.
fn scan_key_flow(probe: bool) -> Vec<Form> {
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x74, Vec::new());
    let mut app = with_vault_and_key(file);
    app.press(Action::Silent);
    app.press(Action::SStep(faraday_core::silent::sstep::SCAN));
    app.press(Action::SScanVault);
    while app.poll_storage().is_some() {}
    app.press(Action::SScanOut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut), "no scan key was offered");
    let _ = app.frame();
    let found = if probe {
        let out = testkit::secret_out(&app).expect("no scan key was offered");
        let key = std::str::from_utf8(&out.bytes).unwrap().trim().to_string();
        assert!(
            key.contains("spscan1"),
            "the file is not a scan key: {key:?} {:?}",
            out.name
        );
        vec![Form::new("scan private key text", key.into_bytes())]
    } else {
        Vec::new()
    };
    crate::seed_load::lock(&mut app);
    while app.poll_storage().is_some() {}
    found
}

#[test]
fn a_scan_key_put_in_a_vault_and_let_out_leaves_no_copy_after_lock() {
    let mut forms = seed_forms("");
    forms.extend(vault_passphrase_forms());
    forms.extend(scan_key_flow(true));
    let armed = arm(&forms);
    scan_key_flow(false);
    finish("scan key in a vault and out", armed, true);
}

/// The key file flow; `probe` is whether it returns the key file made and
/// the key in it.
fn key_file_flow(probe: bool) -> Vec<Form> {
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x75, Vec::new());
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    unlock(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    app.press(Action::Vault(V::MoreAsk));
    app.press(Action::Vault(V::VKeyMake));
    assert!(!app.outbox.is_empty(), "no key file was made");
    let found = if probe {
        let made = app.outbox.last().expect("no key file was made");
        let text = String::from_utf8_lossy(&made.bytes).into_owned();
        let data = text
            .split_once("<Data")
            .and_then(|(_, r)| r.split_once('>'))
            .and_then(|(_, r)| r.split_once("</Data>"))
            .expect("the key file holds no key")
            .0;
        let key: String = data.split_whitespace().collect();
        vec![
            Form::new("key file", made.bytes.to_vec()),
            Form::new("key file key text", key.into_bytes()),
            Form::new("key file key", unhex(data)),
        ]
    } else {
        Vec::new()
    };
    crate::seed_load::lock(&mut app);
    while app.poll_storage().is_some() {}
    found
}

#[test]
fn a_key_file_made_for_a_vault_leaves_no_copy_after_lock() {
    let mut forms = vault_passphrase_forms();
    forms.extend(key_file_flow(true));
    let armed = arm(&forms);
    key_file_flow(false);
    finish("vault key file made", armed, true);
}
