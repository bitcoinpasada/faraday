//! Home's Scan is the one press in for everything a scanner takes (F167)
//! (`docs/QR.md` §2; owner, 2026-10-29): a codex32 string (a Codex32QR
//! code's) and a SLIP-39 share go to Add a key's form for them, a TOTP
//! setup code to a new entry in the open vault, or the read card asks
//! for a vault first; none goes into Files, from Home's Scan
//! or any other scanner, and a WIF or Nostr secret key and words that
//! fail a BIP-39 checksum are named, never filed; a WIF read on Home's
//! Scan opens the Sweep sheet (F171). On a shell with a
//! clipboard (Faraday Demo) each is taken pasted as it is scanned (F75).

use crate::common::type_text;
use faraday_core::forms::Form;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Command, EntropyBytes, Event, FileKind};

/// BIP 93's vector 2: two of a 2-of-n set, in the Codex32QR profile's
/// upper case.
const SHARE_A: &str = "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM";
/// One share of a 2-of-n SLIP-39 set (SLIP-39's test vectors).
const SLIP39: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed";
const TOTP: &str = "otpauth://totp/Scanned:me?secret=JBSWY3DPEHPK3PXP";
const WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

/// Home's Scan, open.
fn home_scan(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan), "Home's Scan did not open");
}

/// One code read by the camera.
fn read(app: &mut Faraday, text: &str) {
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

/// Paste pressed, and the clipboard holding `text`.
fn paste(app: &mut Faraday, text: &str) {
    app.press(Action::ScanPaste);
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, Command::RequestClipboard { .. }));
    assert!(asked, "Paste did not ask the shell for the clipboard");
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: text.to_string(),
    });
    let _ = app.frame();
}

/// The desktop app, started, with a clipboard.
fn desktop() -> Faraday {
    let mut app = testkit::started();
    app.clipboard = true;
    app
}

/// The test vault unlocked.
fn with_vault(clipboard: bool) -> Faraday {
    let mut app = testkit::started();
    app.clipboard = clipboard;
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.fvlt".into(), testkit::test_vault().unwrap())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.event(Event::Entropy(EntropyBytes::new([7; 32])));
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
    assert_eq!(app.vaults().open.len(), 1, "the vault did not open");
    app
}

fn in_codex32_form(app: &Faraday) {
    assert!(
        app.scan().is_none(),
        "the camera stayed open: {}",
        note(app)
    );
    assert_eq!(app.screen, Screen::Entry);
    let e = testkit::entry(app);
    assert_eq!(e.form, Form::Codex32);
    assert_eq!(e.parts.codex32.len(), 1);
    assert!(app.session.keys.is_empty(), "nothing loads on one share");
    assert!(app.inbox().is_empty(), "the share was filed");
}

fn in_slip39_form(app: &Faraday) {
    assert!(
        app.scan().is_none(),
        "the camera stayed open: {}",
        note(app)
    );
    assert_eq!(app.screen, Screen::Entry);
    let e = testkit::entry(app);
    assert_eq!(e.form, Form::Slip39);
    assert_eq!(e.parts.lines(Form::Slip39).len(), 1);
    assert!(app.inbox().is_empty(), "the share was filed");
}

fn in_new_entry(app: &Faraday) {
    assert!(
        app.scan().is_none(),
        "the camera stayed open: {}",
        note(app)
    );
    assert_eq!(app.screen, Screen::VaultContents);
    assert!(app.vaults().form.is_some(), "no entry form opened");
    assert!(app.inbox().iter().all(|i| i.name == "vault.fvlt"));
}

#[test]
fn a_codex32qr_code_scanned_on_home_opens_the_codex32_form_with_it_in() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, SHARE_A);
    in_codex32_form(&app);
}

#[test]
fn a_slip39_share_scanned_on_home_opens_the_slip39_form_with_it_in() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, SLIP39);
    in_slip39_form(&app);
}

#[test]
fn a_totp_setup_code_scanned_on_home_opens_a_new_entry_in_the_open_vault() {
    let mut app = with_vault(false);
    home_scan(&mut app);
    read(&mut app, TOTP);
    in_new_entry(&app);
    app.press(Action::Vault(V::FormSave));
    assert!(app.vaults().form.is_none(), "the entry was not saved");
}

#[test]
fn a_totp_setup_code_scanned_on_home_with_no_vault_open_asks_for_one() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, TOTP);
    assert_eq!(make_a_vault(&app), Some(true), "{}", note(&app));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner closed");
    assert!(app.inbox().is_empty(), "the setup code was filed");
}

/// Whether the read card asks for a vault to be made, the code not kept
/// (`docs/SCANNING.md` §4.5).
fn make_a_vault(app: &Faraday) -> Option<bool> {
    let card = app.scan()?.card.as_ref()?;
    Some(
        card.not_kept
            && card
                .ways
                .iter()
                .map(|w| w.label())
                .eq(["Make a vault".to_string()]),
    )
}

#[test]
fn a_wif_private_key_scanned_on_home_opens_the_sweep_sheet_and_is_not_filed() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, WIF);
    assert_eq!(app.sheet, Some(Sheet::Sweep), "{}", note(&app));
    assert!(app.inbox().is_empty(), "the private key was filed");
    assert!(app.session.keys.is_empty(), "the private key was loaded");
}

#[test]
fn a_codex32_string_pasted_on_homes_scan_opens_the_codex32_form() {
    let mut app = desktop();
    home_scan(&mut app);
    paste(&mut app, &SHARE_A.to_lowercase());
    in_codex32_form(&app);
}

#[test]
fn a_slip39_share_pasted_on_homes_scan_opens_the_slip39_form() {
    let mut app = desktop();
    home_scan(&mut app);
    paste(&mut app, SLIP39);
    in_slip39_form(&app);
}

#[test]
fn a_totp_setup_code_pasted_on_homes_scan_opens_a_new_entry() {
    let mut app = with_vault(true);
    home_scan(&mut app);
    paste(&mut app, TOTP);
    in_new_entry(&app);
}

#[test]
fn a_codex32_string_pasted_on_files_own_scanner_is_still_refused() {
    let mut app = desktop();
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Scan);
    paste(&mut app, &SHARE_A.to_lowercase());
    assert!(note(&app).contains("share of a key"), "{}", note(&app));
    assert!(app.inbox().is_empty());
}

/// NIP-19's example secret key.
const NSEC: &str = "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5";
/// Twelve words of the English list whose BIP-39 checksum fails.
const UNCHECKED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";

/// A secret the scanner cannot take is named on the line and never
/// filed; the scanner stays open.
fn named_not_filed(app: &Faraday, said: &str) {
    assert!(note(app).contains(said), "{}", note(app));
    assert!(app.inbox().is_empty(), "the secret was filed");
    assert!(app.session.keys.is_empty());
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner closed");
}

#[test]
fn a_nostr_secret_key_scanned_on_home_is_named_and_not_filed() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, NSEC);
    named_not_filed(&app, "Nostr secret key");
}

#[test]
fn words_whose_checksum_fails_scanned_on_home_are_named_and_not_filed() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, UNCHECKED);
    named_not_filed(&app, "checksum fails");
}

/// Keystone's `sp()` descriptor (Sparrow's test files): BIP-392's scan
/// key with its origin.
const SP: &str = include_str!("../vectors/interop/sparrow/keystone-singlesig-sp-keystore-1.txt");

/// The scan key sees a wallet's payments and spends none: its `sp()`
/// scanned on Home is filed as a wallet file (owner, 2026-10-29). The
/// spend key in its place is a private key, named and never filed
/// (F167).
#[test]
fn an_sp_descriptor_scanned_on_home_is_filed_as_a_wallet_and_a_spend_key_is_not() {
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, SP);
    assert_eq!(app.inbox().len(), 1, "{}", note(&app));
    assert_eq!(app.inbox()[0].kind, faraday_core::wallet::FileKind::Wallet);
    let mut app = testkit::started();
    home_scan(&mut app);
    read(&mut app, &SP.replacen("spscan1", "spspend1", 1));
    named_not_filed(&app, "That is a private key");
}

/// paperkey 1.6's printed lines and its raw form of one key
/// (`tools/vectors/paperkey/`).
const PAPERKEY: &str = include_str!("../../../../tools/vectors/paperkey/faraday-key.txt");
const PAPERKEY_RAW: &[u8] = include_bytes!("../../../../tools/vectors/paperkey/faraday-key.raw");

/// A paperkey, a GPG key's secret, scanned on Home's Scan or Files' own
/// scanner, in either of paperkey's forms or with a later line mistyped,
/// is named with where it is taken and never filed (F167).
#[test]
fn a_paperkey_scanned_on_home_or_in_files_is_named_and_not_filed() {
    let mistyped = PAPERKEY.replacen("  3: 11", "  3: 12", 1);
    assert_ne!(mistyped, PAPERKEY, "the vector's line 3 changed");
    for screen in [Screen::Home, Screen::Files] {
        for code in [PAPERKEY.as_bytes(), PAPERKEY_RAW, mistyped.as_bytes()] {
            let mut app = testkit::started();
            app.press(Action::Nav(screen));
            app.press(Action::Scan);
            assert_eq!(
                app.sheet,
                Some(Sheet::Scan),
                "{screen:?}'s Scan did not open"
            );
            app.event(Event::Scanned {
                bytes: code.to_vec(),
            });
            let _ = app.frame();
            named_not_filed(&app, "That is a paperkey. Restore a GPG key takes it");
        }
    }
}

#[test]
fn files_own_scanner_routes_a_share_and_a_totp_code_as_homes_does() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Scan);
    read(&mut app, TOTP);
    assert_eq!(make_a_vault(&app), Some(true), "{}", note(&app));
    assert!(app.inbox().is_empty(), "the setup code was filed");
    app.press(Action::ReadAgain);
    read(&mut app, SHARE_A);
    in_codex32_form(&app);
}

#[test]
fn the_spend_tabs_intake_names_a_totp_code_and_files_no_secret() {
    use faraday_core::family::{FamilyAction as F, page};
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::Next(page::MAP)));
    app.press(Action::Family(F::Scan));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the intake scanner is not on");
    for (secret, said) in [
        (TOTP, "not used here"),
        (WIF, "private key"),
        (NSEC, "Nostr secret key"),
        (UNCHECKED, "fail their checksum"),
    ] {
        read(&mut app, secret);
        assert!(note(&app).contains(said), "{secret}: {}", note(&app));
        assert!(app.inbox().is_empty(), "{secret} was filed");
    }
}
