//! The flows `reveals.rs` names for the reasons no other flow reaches: a
//! vault's GPG key shown by its eye and its revocation let out, a note
//! listed and opened, a Lightning key shown and put in a vault, a Silent
//! Payments scan key put in a vault and let out, and a key file made for
//! a vault, a codex32 string and a passphrase kept in a vault by the
//! backup, a codex32 string's and a SeedQR's picture written for a stick,
//! a seed's SeedQR picture let out, and a key's Save to a stick. Each ends
//! by Lock.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::{device, type_text};
use crate::forms::{PHRASE, VAULT_PASSPHRASE, seed_forms, vault_passphrase_forms};
use crate::readers::{open_item, paper_seeds, unlock};
use crate::{arm, finish};
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::plan::{self, At, Item, Spot, What};
use faraday_core::seeds::read::Choice;
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
    // A new one, with a reason, through the same sheet.
    app.press(Action::Back);
    assert_eq!(app.sheet, None, "the revocation sheet stayed");
    app.press(Action::Vault(V::GpgRevocationPage));
    assert!(app.vaults().form.is_some(), "no reason was asked");
    app.press(Action::Vault(V::GpgReason(1)));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "My key was stolen");
    app.press(Action::Vault(V::FormSave));
    assert_eq!(
        app.sheet,
        Some(Sheet::SecretOut),
        "no new revocation was offered"
    );
    let _ = app.frame();
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
        let secret = spscan_secret(&key);
        vec![
            Form::new("scan private key text", key.into_bytes()),
            Form::new("scan private key", secret),
        ]
    } else {
        Vec::new()
    };
    crate::seed_load::lock(&mut app);
    while app.poll_storage().is_some() {}
    found
}

/// The scan key's 32 bytes in a BIP-392 `spscan1q…` text: the bech32m data
/// after the version character, 5 bits to a character, its first 32 bytes.
fn spscan_secret(key: &str) -> Vec<u8> {
    const ALPHABET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let data = key.split_once('1').expect("no separator").1;
    let groups: Vec<u32> = data[..data.len() - 6]
        .chars()
        .skip(1)
        .map(|c| ALPHABET.find(c).expect("not bech32") as u32)
        .collect();
    let (mut acc, mut bits, mut out) = (0u32, 0, Vec::new());
    for g in groups {
        acc = acc << 5 | g;
        bits += 5;
        while bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out.truncate(32);
    assert_eq!(out.len(), 32, "the scan key is short");
    out
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

/// The plan's map changed straight, as the board's presses would.
fn board_edit(app: &mut Faraday, f: impl FnOnce(Vec<Spot>) -> Vec<Spot>) {
    let c = testkit::compute();
    let shape = app.plan_shape_now(app.backup().unwrap().wallet);
    testkit::change_backup_answers(app, |a| {
        let m = plan::map(&c, &shape, a);
        a.set_map(f(m));
    });
}

/// The seeds of the test vault's backup, drained of what the shell takes.
fn settle_storage(app: &mut Faraday) {
    while app.poll_storage().is_some() {}
}

/// Vector 2's key in a one-wallet backup, on a device holding one vault.
fn codex32_with_vault(seed: u8) -> Faraday {
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, seed, Vec::new());
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    crate::seed_load::unlock(&mut app);
    crate::codex32::key_and_backup(&mut app);
    app
}

fn codex32_forms() -> Vec<Form> {
    let mut forms = crate::codex32::forms_of("share A text", crate::codex32::A).to_vec();
    forms.extend(crate::codex32::forms_of("share C text", crate::codex32::C));
    forms.extend(vault_passphrase_forms());
    forms
}

/// A codex32 string the plan keeps in the vault, saved there by the
/// backup as a share record.
#[test]
fn a_codex32_string_saved_into_a_vault_by_the_backup_leaves_no_copy_after_lock() {
    use faraday_core::board::Thing;
    let forms = codex32_forms();
    let armed = arm(&forms);
    let mut app = codex32_with_vault(0x74);
    let a = plan::codex32::index_of(b'a').unwrap();
    crate::common::vault_at(&mut app, 0, 0, &[]);
    crate::common::into_vault(&mut app, 0, Thing::String(0, a));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    settle_storage(&mut app);
    assert_eq!(
        crate::common::contents(&app, 0)
            .iter()
            .filter(|r| r.kind == kind::CODEX32_SHARE)
            .count(),
        1,
        "no share record was kept"
    );
    crate::seed_load::lock(&mut app);
    settle_storage(&mut app);
    finish("codex32 string saved into a vault", armed, true);
}

/// The backup's Lock and write on `app`, the plain files let out as the
/// sheet asks, then the restart: the app that locked and the fresh one.
fn lock_and_write(mut app: Faraday) -> (Faraday, Faraday) {
    let mut shell = crate::common::Shell::light();
    testkit::type_passphrases(&mut app);
    app.press(Action::BNext(Item::Envelopes));
    shell.flow(&mut app, F::WalletLock);
    crate::common::let_seed_files_out(&mut app);
    shell.pump(&mut app);
    assert!(app.restart_requested(), "locked: {:?}", app.toast_text());
    let fresh = shell.restart();
    assert!(!fresh.outbox.is_empty(), "no stick file was made");
    (app, fresh)
}

/// A codex32 string's Codex32QR picture written for a stick (its text
/// file would stay in the Outbox until sent, as a seed's words do).
#[test]
fn a_codex32_picture_made_for_a_stick_leaves_no_copy_after_lock() {
    let forms = codex32_forms();
    let armed = arm(&forms);
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    crate::codex32::key_and_backup(&mut app);
    let a = plan::codex32::index_of(b'a').unwrap();
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::Codex32QrFile(0, a))
    });
    app.press(Action::BChecklist);
    if app.backup().unwrap().open == Some(Item::Templates) {
        app.press(Action::BNext(Item::Templates));
    }
    app.press(Action::BStep(Item::Codex32(0)));
    for j in ["a", "c", "d"] {
        let j = plan::codex32::index_of(j.as_bytes()[0]).unwrap();
        app.press(Action::BC32(faraday_core::codex32_backup::Press::Open(
            j as u8,
        )));
        app.press(Action::BC32(faraday_core::codex32_backup::Press::Skip));
        app.press(Action::BC32(faraday_core::codex32_backup::Press::Skip));
    }
    let (old, fresh) = lock_and_write(app);
    finish("codex32 picture made for a stick", armed, true);
    drop((old, fresh));
}

/// A seed's SeedQR picture written for a stick (its words file would stay
/// in the Outbox until sent).
#[test]
fn a_seedqr_picture_made_for_a_stick_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    crate::backup::at_the_copy(&mut app, plan::Preset::Paper);
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, PHRASE);
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Stick(0, 0), What::SeedQrFile(0))
    });
    let (old, fresh) = lock_and_write(app);
    finish("SeedQR picture made for a stick", armed, true);
    drop((old, fresh));
}

/// A seed's SeedQR picture let out unprotected from the secret sheet,
/// which also made the words file the sheet offers.
#[test]
fn a_seedqr_picture_let_out_unprotected_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = testkit::started();
    crate::backup::at_the_copy(&mut app, plan::Preset::Paper);
    app.press(Action::BReveal);
    app.press(Action::BCheck);
    testkit::type_words(&mut app, PHRASE);
    board_edit(&mut app, |m| {
        plan::with_added(&m, At::Place(0), What::SeedQr(0))
    });
    app.press(Action::BFile);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(
        testkit::secret_out(&app).is_some_and(|o| o.forms.len() > 1),
        "no picture was offered"
    );
    app.press(Action::SecretForm(1));
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    assert_eq!(app.sheet, None);
    settle_storage(&mut app);
    crate::seed_load::lock(&mut app);
    settle_storage(&mut app);
    finish("SeedQR picture let out", armed, true);
}

/// A key kept in a vault, its Save to a stick opened: the words and the
/// SeedQR are offered by name.
#[test]
fn a_vault_keys_save_to_a_stick_opened_leaves_no_copy_after_lock() {
    let c = testkit::compute();
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, PHRASE)
        .expect("the vector's words");
    let key = Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&m))
        .with(field::KEY_LABEL, b"Leak key");
    let mut forms = seed_forms("");
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x75, vec![key]);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app, 0);
    open_item(&mut app, 0);
    app.press(Action::Vault(V::SaveToStick));
    let _ = app.frame();
    assert!(app.sheet.is_some(), "Save to a stick did not open");
    crate::seed_load::lock(&mut app);
    finish("vault key save to a stick", armed, true);
}

/// A passphrase the plan keeps in the vault, saved there by the backup
/// as a passphrase record.
#[test]
fn a_passphrase_saved_into_a_vault_by_the_backup_leaves_no_copy_after_lock() {
    use faraday_core::board::Thing;
    use faraday_core::seeds::SeedsAction as S;
    let forms = seed_forms(crate::forms::PASSPHRASE);
    let mut forms = forms;
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let file = crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x76, Vec::new());
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    crate::seed_load::unlock(&mut app);
    app.clipboard = true;
    app.press(Action::ScanSeed);
    crate::seed_load::paste(&mut app, faraday_text::Text::of(PHRASE));
    app.press(Action::SeedRead(Choice::AddPass));
    app.press(Action::SeedRead(Choice::Field(0)));
    type_text(&mut app, crate::forms::PASSPHRASE);
    app.press(Action::SeedRead(Choice::Field(1)));
    type_text(&mut app, crate::forms::PASSPHRASE);
    testkit::pause(&mut app);
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1, "no wallet was made");
    app.press(Action::Backup(0));
    testkit::backup_as(&mut app, plan::Preset::PaperVault);
    // The seed stays on paper; only its passphrase is kept in the vault.
    crate::common::vault_at(&mut app, 0, 0, &[]);
    crate::common::no_paper_passphrases(&mut app);
    crate::common::into_vault(&mut app, 0, Thing::Pass(0));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));
    settle_storage(&mut app);
    assert_eq!(
        crate::common::contents(&app, 0)
            .iter()
            .filter(|r| r.kind == kind::PASSPHRASE)
            .count(),
        1,
        "no passphrase record was kept: {:?} {:?} {:?}",
        crate::common::backup_held(&app, At::Vault(0)),
        app.toast_text(),
        app.sheet
    );
    crate::seed_load::lock(&mut app);
    settle_storage(&mut app);
    finish("passphrase saved into a vault", armed, true);
}
