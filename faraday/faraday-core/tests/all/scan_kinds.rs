//! What every scanner does with a kind of code it is not for
//! (`docs/SCANNING.md` §4): no scanner puts a secret in Files, by the
//! camera, by Paste or from a picture on a stick; a scanner names a kind
//! it does not take by what it is ("A transaction to sign · not used
//! here"), and an Add a key form's Scan sends the other share form's part
//! to that form and keeps its own parser's line for a part of its own.

use faraday_core::family::{FamilyAction as F, page};
use faraday_core::forms::Form;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Command, Event, FileKind};

/// BIP 93's vector 1: a codex32 secret.
const CODEX32_SECRET: &str = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
/// BIP 93's vector 2: one share of a 2-of-n set.
const CODEX32_SHARE: &str = "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM";
/// One share of a 2-of-n SLIP-39 set (SLIP-39's test vectors).
const SLIP39: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed";
/// BIP-39's first vector's words.
const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
/// The same seed as a SeedQR's digits (SeedSigner's SeedQR: each word's
/// index in four digits).
const SEEDQR: &str = "000000000000000000000000000000000000000000000003";
/// Twelve words of the English list whose checksum fails.
const UNCHECKED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
/// Bitcoin Core's key_io_valid.json: a compressed mainnet WIF.
const WIF: &str = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
/// BIP-32's test vector 1: the master private key.
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LH5LpRKD3pfgJwQbtSm9mSTtgoeQqk9ib";
/// NIP-19's example secret key.
const NSEC: &str = "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5";
const TOTP: &str = "otpauth://totp/Scanned:me?secret=JBSWY3DPEHPK3PXP";
const ENTRIES: &str = "title: Mail\npassword: correct horse";
/// paperkey 1.6's printed lines of one key (`tools/vectors/paperkey/`).
const PAPERKEY: &str = include_str!("../../../../tools/vectors/paperkey/faraday-key.txt");

/// Every secret kind of `docs/SCANNING.md` §2 a scanner can read as text.
const SECRETS: [&str; 12] = [
    WORDS,
    SEEDQR,
    CODEX32_SECRET,
    CODEX32_SHARE,
    SLIP39,
    UNCHECKED,
    WIF,
    XPRV,
    NSEC,
    TOTP,
    ENTRIES,
    PAPERKEY,
];

/// The scanners a person can open with no wallet or vault, by where
/// they are opened.
#[derive(Clone, Copy, Debug)]
enum Opened {
    Home,
    Files,
    Restore,
    AddAKey,
    Codex32Form,
    Intake,
}

const SCANNERS: [Opened; 6] = [
    Opened::Home,
    Opened::Files,
    Opened::Restore,
    Opened::AddAKey,
    Opened::Codex32Form,
    Opened::Intake,
];

fn open(app: &mut Faraday, at: Opened) {
    match at {
        Opened::Home => {
            app.press(Action::Nav(Screen::Home));
            app.press(Action::Scan);
        }
        Opened::Files => {
            app.press(Action::Nav(Screen::Files));
            app.press(Action::Scan);
        }
        Opened::Restore => {
            app.press(Action::RestoreWallet);
            app.press(Action::Scan);
        }
        Opened::AddAKey => {
            app.press(Action::Entry(None));
            app.press(Action::ScanSeed);
        }
        Opened::Codex32Form => {
            app.press(Action::Entry(None));
            let i = Form::ALL.iter().position(|f| *f == Form::Codex32).unwrap();
            app.press(Action::EntryForm(i as u8));
            app.press(Action::ScanPart);
        }
        Opened::Intake => {
            app.press(Action::Nav(Screen::Family));
            app.press(Action::Family(F::Next(page::MAP)));
            app.press(Action::Family(F::Scan));
        }
    }
    assert_eq!(
        app.sheet,
        Some(Sheet::Scan),
        "{at:?}'s scanner did not open"
    );
}

fn note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

fn read(app: &mut Faraday, code: &[u8]) {
    app.event(Event::Scanned {
        bytes: code.to_vec(),
    });
    let _ = app.frame();
}

/// Paste pressed, the clipboard holding `text`, where the scanner offers
/// Paste.
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

/// Whether anything of `secret` is in Files.
fn filed(app: &Faraday, secret: &str) -> bool {
    let t = secret.trim();
    app.inbox().iter().any(|i| {
        let held = String::from_utf8_lossy(&i.bytes);
        held.contains(t) || t.contains(held.trim()) && !held.trim().is_empty()
    })
}

#[test]
fn no_scanner_files_a_secret_read_by_the_camera() {
    for at in SCANNERS {
        for secret in SECRETS {
            let mut app = testkit::started();
            open(&mut app, at);
            read(&mut app, secret.as_bytes());
            assert!(
                !filed(&app, secret),
                "{at:?} filed {}",
                &secret[..secret.len().min(24)]
            );
        }
    }
}

#[test]
fn no_scanner_files_a_secret_pasted() {
    for at in SCANNERS {
        for secret in SECRETS {
            let mut app = testkit::started();
            app.clipboard = true;
            open(&mut app, at);
            paste(&mut app, secret);
            assert!(
                !filed(&app, secret),
                "{at:?} filed {} pasted",
                &secret[..secret.len().min(24)]
            );
        }
    }
}

#[test]
fn a_picture_on_a_stick_holding_a_private_key_or_words_that_are_no_seed_files_none() {
    for secret in [UNCHECKED, WIF, XPRV, NSEC, PAPERKEY] {
        let mut app = testkit::started();
        app.storage(StorageEvent::QrRead {
            name: "code.png".into(),
            bytes: Vec::new(),
            payloads: vec![secret.as_bytes().to_vec()],
        });
        let _ = app.frame();
        assert!(
            !filed(&app, secret),
            "a picture filed {}",
            &secret[..secret.len().min(24)]
        );
    }
}

/// A public kind read where it is not taken is named by what it is, and
/// the scanner stays open on it.
#[test]
fn add_a_keys_scanner_names_what_it_read_instead_of_a_seed() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let unsigned = testkit::unsigned(&kit).unwrap();
    let psbt = unsigned.to_bytes();
    let base64 = unsigned.to_base64();
    for (code, said) in [
        (psbt.clone(), "A transaction to sign · not used here"),
        (base64.into_bytes(), "A transaction to sign · not used here"),
        (
            kit.descriptor.as_bytes().to_vec(),
            "A wallet description · not used here",
        ),
        (
            TOTP.as_bytes().to_vec(),
            "A TOTP setup code · not used here",
        ),
        (
            WIF.as_bytes().to_vec(),
            "That is a private key. Sweep a private key, in Tools, takes it",
        ),
        (b"\x01\x02\x03".to_vec(), "Nothing Faraday reads"),
    ] {
        let mut app = testkit::started();
        open(&mut app, Opened::AddAKey);
        read(&mut app, &code);
        assert_eq!(note(&app), said);
        assert_eq!(app.sheet, Some(Sheet::Scan));
        assert!(app.inbox().is_empty());
        assert!(app.session.keys.is_empty());
    }
}

#[test]
fn add_a_keys_scanner_sends_a_slip39_share_to_its_form() {
    let mut app = testkit::started();
    open(&mut app, Opened::AddAKey);
    read(&mut app, SLIP39.as_bytes());
    assert!(app.scan().is_none(), "the camera stayed on: {}", note(&app));
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(testkit::entry(&app).form, Form::Slip39);
    assert_eq!(testkit::entry(&app).parts.lines(Form::Slip39).len(), 1);
}

#[test]
fn the_codex32_forms_scanner_names_a_kind_that_is_not_its_forms() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = testkit::started();
    open(&mut app, Opened::Codex32Form);
    read(&mut app, kit.descriptor.as_bytes());
    assert_eq!(note(&app), "A wallet description · not used here");
    // A string that starts as a codex32 string does keeps the parser's
    // line.
    read(
        &mut app,
        b"ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrX",
    );
    assert!(
        note(&app).starts_with("Not a codex32 string"),
        "{}",
        note(&app)
    );
    assert!(testkit::entry(&app).parts.codex32.is_empty());
}

#[test]
fn the_codex32_forms_scanner_takes_a_slip39_share_onto_the_slip39_form() {
    let mut app = testkit::started();
    open(&mut app, Opened::Codex32Form);
    read(&mut app, SLIP39.as_bytes());
    assert!(app.scan().is_none(), "the camera stayed on: {}", note(&app));
    assert_eq!(testkit::entry(&app).form, Form::Slip39);
    assert_eq!(testkit::entry(&app).parts.lines(Form::Slip39).len(), 1);
}

#[test]
fn the_codex32_forms_scanner_sends_a_whole_seed_to_the_seed_sheet() {
    let mut app = testkit::started();
    open(&mut app, Opened::Codex32Form);
    read(&mut app, SEEDQR.as_bytes());
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    assert!(app.scan().is_none());
}
