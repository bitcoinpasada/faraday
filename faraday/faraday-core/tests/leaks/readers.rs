//! The readers of what a vault or a file holds: a vault's entries
//! listed, opened and copied, a Krux encrypted code opened, a GPG key
//! restored from its paperkey, and a Bitcoin Core wallet read. Each plants
//! the secret the reader takes in and ends by Lock where the flow has one.

use crate::common::{device, type_text};
use crate::forms::{
    ENTRY_NOTES, ENTRY_PASSWORD, PHRASE, TOTP_URI, VAULT_PASSPHRASE, entry_forms, seed_forms,
    vault_passphrase_forms,
};
use crate::{arm, finish};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_leak_scan::Form;
use faraday_shell_api::{App, EntropyBytes, Event, Key};
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

/// A vault of the test seed as a key and one entry holding a password,
/// notes and a TOTP secret.
fn vault(label: &str, id: u8) -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    let key = Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_LABEL, b"Leak key");
    let entry = Record::new(kind::ENTRY)
        .with(field::TITLE, b"Mail")
        .with(field::PASSWORD, ENTRY_PASSWORD.as_bytes())
        .with(field::NOTES, ENTRY_NOTES.as_bytes())
        .with(field::TOTP, TOTP_URI.as_bytes());
    crate::common::vault_of(label, VAULT_PASSPHRASE, id, vec![key, entry])
}

/// Every form the vault holds.
fn vault_forms() -> Vec<Form> {
    let mut forms = seed_forms("");
    forms.extend(vault_passphrase_forms());
    forms.extend(entry_forms());
    forms
}

/// Unlocks place `at` of the Vaults list, the Load sheet answered Not now.
fn unlock(app: &mut Faraday, at: usize) {
    let before = app.vaults().open.len();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(at)));
    type_text(app, VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "the vault did not open"
    );
    if app.sheet == Some(Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
}

/// The page of the vault's first item of `category`, drawn.
fn open_item(app: &mut Faraday, category: usize) {
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(category)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
}

#[test]
fn a_vaults_entries_listed_and_opened_leave_no_copy_after_lock() {
    let file = vault("Cellar", 0x5e);
    let forms = vault_forms();
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app, 0);
    app.press(Action::Nav(Screen::VaultContents));
    for category in [0, 2] {
        app.press(Action::Vault(V::Category(category)));
        let _ = app.frame();
    }
    open_item(&mut app, 0);
    open_item(&mut app, 2);
    crate::seed_load::lock(&mut app);
    finish("vault entries opened", armed, true);
}

#[test]
fn a_vaults_entry_copied_to_another_vault_leaves_no_copy_after_lock() {
    let (a, b) = (
        vault("Alpha", 0x61),
        crate::common::vault_of("Beta", VAULT_PASSPHRASE, 0x62, Vec::new()),
    );
    let forms = vault_forms();
    let armed = arm(&forms);
    let mut app = device(vec![("a.kdbx".into(), a), ("b.kdbx".into(), b)]);
    unlock(&mut app, 0);
    unlock(&mut app, 1);
    testkit::vault_current(&mut app, 0);
    open_item(&mut app, 2);
    app.press(Action::Vault(V::CopyTo));
    assert_eq!(app.sheet, Some(Sheet::CopyTo));
    app.press(Action::Vault(V::CopyPick(0)));
    assert_eq!(app.sheet, None, "the copy was not made");
    crate::seed_load::lock(&mut app);
    finish("vault entry copied", armed, true);
}

/// The long passphrase a Krux code is sealed under.
const KEF_PASSPHRASE: &str = "Cobalt-Meridian-Thistle-6290-Juniper-Lantern-Opal-185";

/// A code Faraday seals around the test seed's words.
fn kef_code() -> Vec<u8> {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.kef_seal(
        zeroize::Zeroizing::new(KEF_PASSPHRASE.as_bytes().to_vec()),
        zeroize::Zeroizing::new(PHRASE.as_bytes().to_vec()),
        20,
        faraday_core::worker::kef::KefId::Random(8),
        10_000,
        "Working",
    )
    .expect("fresh bytes");
    testkit::settle(&mut app);
    let mut sealed = app.kef_take_sealed().unwrap().unwrap();
    std::mem::take(&mut *sealed)
}

#[test]
fn a_krux_code_opened_leaves_no_copy_after_lock() {
    let code = kef_code();
    let mut forms = seed_forms("");
    forms.push(Form::new(
        "code passphrase",
        KEF_PASSPHRASE.as_bytes().to_vec(),
    ));
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.event(Event::Scanned { bytes: code });
    let _ = app.frame();
    let open = app
        .scan()
        .and_then(|s| s.card.as_ref())
        .and_then(|c| c.ways.iter().position(|w| w.label() == "Open"))
        .expect("the card offers Open");
    app.press(Action::ReadGo(open as u8));
    assert_eq!(app.sheet, Some(Sheet::KefRead));
    type_text(&mut app, KEF_PASSPHRASE);
    app.event(Event::Key(Key::Enter));
    for _ in 0..6 {
        testkit::settle(&mut app);
        if app.busy().is_none() {
            break;
        }
    }
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::SeedRead), "the code did not open");
    app.press(Action::Cancel);
    crate::seed_load::lock(&mut app);
    finish("krux code opened", armed, true);
}

/// The key of `tools/vectors/paperkey/faraday-key.txt`: the seeds it was
/// made from.
fn paper_seeds() -> ([u8; 32], [u8; 32]) {
    let (mut primary, mut subkey) = ([0x11; 32], [0x22; 32]);
    primary[0] = 0;
    subkey[..2].fill(0);
    (primary, subkey)
}

#[test]
fn a_gpg_key_restored_from_its_paperkey_leaves_no_copy() {
    use faraday_pgp::{Armor, armor};
    const PRINTED: &str = include_str!("../../../../tools/vectors/paperkey/faraday-key.txt");
    let (primary, subkey) = paper_seeds();
    let made = faraday_keys::gpg::create(
        &zeroize::Zeroizing::new(primary),
        &zeroize::Zeroizing::new(subkey),
        1_780_000_000,
        "Test Person <test@example.com>".to_string(),
        2 * 31_536_000,
    );
    let text = armor(Armor::PublicKey, &made.certificate);
    let certificate = faraday_pgp::Certificate::read(text.as_bytes()).expect("its certificate");
    let lines: Vec<&str> = PRINTED
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();
    let forms = vec![
        Form::new("GPG primary seed", primary.to_vec()),
        Form::new("GPG subkey seed", subkey.to_vec()),
    ];
    let c = testkit::compute();
    let armed = arm(&forms);
    let mut restore = faraday_keys::gpg::Restore::new(certificate);
    for (i, l) in lines.iter().enumerate() {
        restore
            .line(i as u32 + 1, l)
            .unwrap_or_else(|e| panic!("line {}: {e:?}", i + 1));
    }
    assert!(restore.complete(), "the paperkey did not complete");
    let restored = restore.finish(&c).expect("the key restores");
    drop(restored);
    drop(restore);
    finish("gpg paperkey restored", armed, false);
}

#[test]
fn a_bitcoin_core_wallet_read_leaves_no_copy() {
    use faraday_bip::bitcoin::bip32::Xpriv;
    use faraday_bip::keys::Network;
    use faraday_core::core_wallet::read;
    use faraday_intake::secret_buf::SecretText;
    use std::str::FromStr;
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/vectors/core-wallet/encrypted.json"
    );
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("dump")).expect("json");
    let hex = |v: &serde_json::Value| {
        let s = v.as_str().expect("hex");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("hex"))
            .collect::<Vec<u8>>()
    };
    let records: Vec<(Vec<u8>, Vec<u8>)> = json["records"]
        .as_array()
        .expect("records")
        .iter()
        .map(|r| (hex(&r[0]), hex(&r[1])))
        .collect();
    let id = u32::try_from(json["application_id"].as_u64().expect("id")).expect("id");
    let desc = json["listdescriptors"]["descriptors"][0]["desc"]
        .as_str()
        .expect("a private descriptor");
    let start = desc.find("tprv").expect("its master key");
    let tprv = &desc[start..start + desc[start..].find('/').expect("its path")];
    let key = Xpriv::from_str(tprv).expect("a tprv");
    let forms = vec![
        Form::new("core master tprv text", tprv.as_bytes().to_vec()),
        Form::new("core master tprv", key.encode().to_vec()),
        Form::new(
            "core master private key",
            key.private_key.secret_bytes().to_vec(),
        ),
        Form::new("core master chain code", key.chain_code.as_bytes().to_vec()),
    ];
    let c = testkit::compute();
    let pass = SecretText::of("faraday");
    let armed = arm(&forms);
    let wallet = read(&c, id, &records, Network::Regtest, Some(&pass)).expect("the wallet reads");
    assert!(wallet.is_whole(), "{:?}", wallet.failures);
    drop(wallet);
    finish("core wallet read", armed, false);
}
