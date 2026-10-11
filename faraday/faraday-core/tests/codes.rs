//! Every public file a flow makes is offered as a code and as a labelled
//! picture of it, beside the file itself: the multisig config, the BIP
//! 129 descriptor record, a cosigner's key and its key record, the silent
//! payments record and address, a signed message, and a GPG public key,
//! revocation and signature. What the QR sheet shows and what the PNG
//! holds read back, through the decoder the disk process and the camera
//! use, as the file's own text. A secret, and a file shown from Files,
//! offer no picture.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Code, Faraday, Screen, StickInfo, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};
use osk_ui::canvas::InkKind;

/// The one code in a PNG, as text.
fn read_png(png: &[u8]) -> String {
    let payloads = faraday_files::qr_in_png(png).expect("a PNG");
    assert_eq!(payloads.len(), 1, "one code");
    String::from_utf8(payloads[0].clone()).expect("text")
}

/// The code the QR sheet shows, as a camera reads it.
fn shown(app: &Faraday) -> String {
    let q = app.qr.as_ref().expect("no QR sheet");
    assert_eq!(q.frames.len(), 1, "{}: one code", q.title);
    read_png(&osk_codec::png::qr_png(&q.frames[0], 4))
}

/// The bytes of Outbox file `name`.
fn outbox(app: &Faraday, name: &str) -> Vec<u8> {
    app.outbox
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| {
            panic!(
                "no {name} in {:?}",
                app.outbox.iter().map(|i| &i.name).collect::<Vec<_>>()
            )
        })
        .bytes
        .clone()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("text")
}

/// The text the picture of the sheet's code is labelled with, its
/// wrapped lines joined.
fn label(app: &Faraday) -> String {
    let q = app.qr.as_ref().expect("no QR sheet");
    faraday_core::picture::draw(&q.frames[0], &q.title, &q.label)
        .ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, .. } => Some(text.clone()),
            InkKind::Icon(_) => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `code` shown as a QR and put in the Outbox as `png`, from the sheet
/// and from the flow's own button: each reads back as `want`.
fn reads_back(app: &mut Faraday, code: Code, png: &str, want: &str) {
    app.press(Action::ShowCode(code));
    assert_eq!(shown(app).trim(), want.trim(), "{code:?} shown");
    assert!(app.qr.as_ref().unwrap().offers_png(), "{code:?}");
    app.press(Action::QrPng);
    assert_eq!(read_png(&outbox(app, png)).trim(), want.trim(), "{png}");
    app.press(Action::Cancel);
    app.outbox.retain(|i| i.name != png);
    app.press(Action::CodePng(code));
    assert_eq!(read_png(&outbox(app, png)).trim(), want.trim(), "{png}");
}

fn holding(name: &str, descriptor: &str) -> (Faraday, usize) {
    let mut app = testkit::started();
    app.session = testkit::session();
    let w = app
        .session
        .add_wallet(name, descriptor, "test")
        .expect("a wallet");
    (app, w)
}

fn file(files: &[(String, Vec<u8>)], name: &str) -> String {
    text(&files.iter().find(|(n, _)| n == name).expect(name).1)
}

#[test]
fn a_multisig_config_reads_back_from_its_code_and_its_picture() {
    let files = testkit::public_files("Savings", &testkit::savings()).unwrap();
    let (mut app, w) = holding("Savings", &testkit::savings());
    reads_back(
        &mut app,
        Code::MultisigConfig(w),
        "savings-multisig-config.png",
        &file(&files, "savings-multisig-config.txt"),
    );
}

#[test]
fn a_bsms_descriptor_record_reads_back_from_its_code_and_its_picture() {
    let files = testkit::public_files("Savings", &testkit::savings()).unwrap();
    let (mut app, w) = holding("Savings", &testkit::savings());
    reads_back(
        &mut app,
        Code::Bsms(w),
        "savings-bsms.png",
        &file(&files, "savings-bsms.txt"),
    );
}

/// An app making a wsh multisig with test seed 0 in its first slot.
fn making() -> (Faraday, String) {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::CNext(0));
    app.press(Action::CNext(1));
    let fp = app.session.keys[0].master.fingerprint();
    app.press(Action::CSlotHere(0, fp.0));
    (app, faraday_core::wallet::fp_text(fp))
}

/// A picture read on a stick visit, as the cosigner's Faraday reads it:
/// the key it holds.
fn key_scanned(name: &str, png: &[u8]) -> String {
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "S1".to_string(),
        label: "TESTSTICK".to_string(),
        boot: false,
        files: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Visit));
    app.storage(StorageEvent::QrRead {
        name: name.to_string(),
        bytes: png.to_vec(),
        payloads: faraday_files::qr_in_png(png).expect("a PNG"),
    });
    let item = app
        .inbox
        .iter()
        .find(|i| i.kind == FileKind::Key)
        .unwrap_or_else(|| panic!("no key read: {:?}", app.visit.log));
    plain(&text(&item.bytes))
}

fn plain(key_file: &str) -> String {
    faraday_core::create::read_key(key_file)
        .expect("a key")
        .replace('\'', "h")
}

#[test]
fn a_cosigners_key_reads_back_from_its_code_and_its_picture() {
    let (mut app, fp) = making();
    app.press(Action::CKeyOut(0));
    let want = plain(&text(&outbox(&app, &format!("xpub-{fp}.txt"))));
    // Shown as Sparrow and SeedSigner read a wsh key: ur:crypto-account.
    app.press(Action::CKeyQr(0));
    let q = app.qr.as_ref().unwrap();
    assert!(q.offers_png());
    let sheet = osk_codec::png::qr_png(&q.frames[0], 4);
    assert_eq!(key_scanned("sheet.png", &sheet), want);
    let l = label(&app);
    assert!(
        l.contains(&format!("Key {fp} · ")) && l.contains("m/48h/"),
        "{l}"
    );
    app.press(Action::Cancel);
    app.press(Action::CodePng(Code::Key(0)));
    let png = outbox(&app, &format!("xpub-{fp}.png"));
    assert_eq!(key_scanned("xpub.png", &png), want);
}

#[test]
fn a_cosigners_key_record_reads_back_from_its_code_and_its_picture() {
    let (mut app, fp) = making();
    app.press(Action::CKeyBsms(0));
    let want = text(&outbox(&app, &format!("xpub-{fp}-bsms.txt")));
    reads_back(
        &mut app,
        Code::KeyBsms(0),
        &format!("xpub-{fp}-bsms.png"),
        &want,
    );
}

fn with_key() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app
}

#[test]
fn the_silent_payments_record_and_address_read_back_from_code_and_picture() {
    let mut app = with_key();
    let fp = faraday_core::wallet::fp_text(app.session.keys[0].master.fingerprint());
    app.press(Action::Silent);
    app.press(Action::SRecord);
    let record = text(&outbox(&app, &format!("silent-{fp}.txt")));
    reads_back(
        &mut app,
        Code::SilentRecord,
        &format!("silent-{fp}-record.png"),
        &record,
    );
    // The address on show, from its own Show as QR.
    let address = app.silent_address().unwrap();
    app.press(Action::SQr(false));
    assert_eq!(shown(&app), address);
    let l = label(&app);
    assert!(l.contains(&format!("Silent payment address · {fp}")), "{l}");
    app.press(Action::QrPng);
    assert_eq!(
        read_png(&outbox(&app, &format!("silent-{fp}.png"))),
        address
    );
}

#[test]
fn a_signed_message_reads_back_from_its_code_and_its_picture() {
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .add_words(
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    let spending = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.session
        .add_wallet(spending.name, &spending.descriptor, "test")
        .expect("the wallet");
    app.press(Action::SignMessage);
    app.message.as_mut().unwrap().text = "Faraday signs this".to_string();
    app.press(Action::MSign);
    let address = app
        .message
        .as_ref()
        .unwrap()
        .signed
        .as_ref()
        .expect("signed")
        .address
        .clone();
    app.press(Action::MOut);
    let name = format!("message-{}", &address[address.len() - 6..]);
    let want = text(&outbox(&app, &format!("{name}.txt")));
    app.press(Action::MQr);
    assert_eq!(shown(&app).trim(), want.trim());
    app.press(Action::QrPng);
    assert_eq!(
        read_png(&outbox(&app, &format!("{name}.png"))).trim(),
        want.trim()
    );
    app.press(Action::Cancel);
    app.outbox.retain(|i| !i.name.ends_with(".png"));
    reads_back(&mut app, Code::Message, &format!("{name}.png"), &want);
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// An app with a new GPG key in the open test vault, its certificate
/// and revocation in the Outbox, and a file in the Inbox to sign.
fn with_gpg_key() -> Faraday {
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
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            vault,
            (
                "SHA256SUMS".into(),
                b"3d6f  faraday-0.1.0-linux-x86_64\n".to_vec(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_482_652,
    });
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..21 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "{:?}", app.vaults.unlock_error);
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    app.press(Action::Vault(V::Add));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Release");
    app.press(Action::Vault(V::FormSave));
    app
}

/// The name of the Outbox file ending `end`.
fn named(app: &Faraday, end: &str) -> String {
    app.outbox
        .iter()
        .map(|i| i.name.clone())
        .find(|n| n.ends_with(end))
        .unwrap_or_else(|| panic!("no *{end}"))
}

#[test]
fn a_gpg_public_key_reads_back_from_its_code_and_its_picture() {
    let mut app = with_gpg_key();
    let asc = app
        .outbox
        .iter()
        .map(|i| i.name.clone())
        .find(|n| n.ends_with(".asc") && !n.ends_with("-revocation.asc"))
        .expect("the public key");
    let stem = asc.trim_end_matches(".asc").to_string();
    let want = text(&outbox(&app, &asc));
    reads_back(&mut app, Code::GpgKey, &format!("{stem}.png"), &want);
}

#[test]
fn a_gpg_revocation_reads_back_and_says_what_it_revokes() {
    let mut app = with_gpg_key();
    let asc = named(&app, "-revocation.asc");
    let stem = asc.trim_end_matches("-revocation.asc").to_string();
    let want = text(&outbox(&app, &asc));
    reads_back(
        &mut app,
        Code::GpgRevocation,
        &format!("{stem}-revocation.png"),
        &want,
    );
    app.press(Action::ShowCode(Code::GpgRevocation));
    let l = label(&app);
    assert!(l.contains("Revokes Release"), "{l}");
}

#[test]
fn a_gpg_signature_reads_back_from_its_code_and_its_picture() {
    let mut app = with_gpg_key();
    let k = app
        .inbox
        .iter()
        .position(|i| i.name == "SHA256SUMS")
        .unwrap();
    app.press(Action::Vault(V::GpgSignPick));
    app.press(Action::Vault(V::GpgSign(k)));
    let want = text(&outbox(&app, "SHA256SUMS.asc"));
    reads_back(
        &mut app,
        Code::GpgSignature(k),
        "SHA256SUMS-signature.png",
        &want,
    );
}

#[test]
fn a_secret_shown_as_a_code_offers_no_picture() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: vec![(
            "threshold-partly-signed.osk".to_string(),
            testkit::threshold_carry().unwrap(),
        )],
        kept: Vec::new(),
    });
    app.press(Action::QrOutbox(0));
    let q = app.qr.as_ref().expect("no QR");
    assert!(q.secret);
    assert!(!q.offers_png());
    app.press(Action::QrPng);
    assert_eq!(app.outbox.len(), 1);
}

#[test]
fn files_show_as_qr_is_the_file_and_offers_no_picture() {
    let spending = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: vec![(
            "spending-wallet.txt".to_string(),
            spending.descriptor.clone().into_bytes(),
        )],
        kept: Vec::new(),
    });
    app.press(Action::QrOutbox(0));
    assert_eq!(shown(&app), spending.descriptor);
    assert!(!app.qr.as_ref().unwrap().offers_png());
    app.press(Action::QrPng);
    assert_eq!(app.outbox.len(), 1);
}
