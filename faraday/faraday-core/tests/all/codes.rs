//! Every public file a flow makes is offered as a code and as a labelled
//! picture of it, beside the file itself: the multisig config, the BIP
//! 129 descriptor record, a cosigner's key and its key record, the silent
//! payments record and address, a signed message, and a GPG public key,
//! revocation and signature. What the QR sheet shows and what the PNG
//! holds read back, through the decoder the disk process and the camera
//! use, as the file's own text. A secret, and a file shown from Files,
//! offer no picture.

use crate::common::type_text;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Code, Faraday, Screen, StickInfo, StorageEvent};
use faraday_draw::canvas::InkKind;
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

/// The one code in a PNG, as text.
fn read_png(png: &[u8]) -> String {
    let c = faraday_core::testkit::compute();
    let payloads = faraday_files::qr_in_png(&c, png).expect("a PNG");
    assert_eq!(payloads.len(), 1, "one code");
    String::from_utf8(payloads[0].clone()).expect("text")
}

/// The code the QR sheet shows, as a camera reads it.
fn shown(app: &Faraday) -> String {
    let q = app.qr().expect("no QR sheet");
    assert_eq!(q.frames.len(), 1, "{}: one code", q.title.as_str());
    read_png(&faraday_codec::png::qr_png(&q.frames[0], 4))
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
        .to_vec()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("text")
}

/// The text the picture of the sheet's code is labelled with, its
/// wrapped lines joined.
fn label(app: &Faraday) -> String {
    let q = app.qr().expect("no QR sheet");
    faraday_core::picture::draw(&q.frames[0], &q.title, &q.label)
        .ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, .. } => Some(text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `code` shown as a QR and put in the Outbox as `png`, from the sheet
/// and from the flow's own button: each reads back as `want`.
fn reads_back(app: &mut Faraday, code: Code, png: &str, want: &str) {
    app.press(Action::ShowCode(code));
    assert_eq!(shown(app).trim(), want.trim(), "{code:?} shown");
    assert!(app.qr().unwrap().offers_png(), "{code:?}");
    app.press(Action::QrPng);
    assert_eq!(read_png(&outbox(app, png)).trim(), want.trim(), "{png}");
    app.press(Action::Cancel);
    app.outbox.retain(|i| i.name != png);
    app.press(Action::CodePng(code));
    assert_eq!(read_png(&outbox(app, png)).trim(), want.trim(), "{png}");
}

fn holding(name: &str, descriptor: &str) -> (Faraday, usize) {
    let c = faraday_core::testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    let w = app
        .session
        .add_wallet_confirmed(
            &c,
            name,
            descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
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
    making_at(false)
}

/// [`making`], with BIP-87's path chosen on Quorum when `bip87`.
fn making_at(bip87: bool) -> (Faraday, String) {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 4);
    app.press(Action::CNext(0));
    if bip87 {
        let _ = app.frame();
        app.press(Action::CMultiPathChange);
        app.press(Action::CMultiPath(true));
    }
    app.press(Action::CNext(1));
    let fp = app.session.keys[0].fingerprint;
    app.press(Action::CSlotHere(0, fp.0));
    (app, String::from(&*faraday_core::wallet::fp_text(fp)))
}

#[test]
fn a_bip_87_cosigners_key_code_and_signed_record_name_bip_87() {
    let (mut app, fp) = making_at(true);
    app.press(Action::CKeyOut(0));
    let want = plain(&text(&outbox(&app, &format!("xpub-{fp}.txt"))));
    assert!(want.contains("/87h/1h/0h]"), "{want}");
    // The code is ur:crypto-account and reads back as the key at m/87'.
    app.press(Action::CKeyQr(0));
    let q = app.qr().unwrap();
    let sheet = faraday_codec::png::qr_png(&q.frames[0], 4);
    assert_eq!(key_scanned("sheet.png", &sheet), want);
    assert!(label(&app).contains("m/87h/"), "{}", label(&app));
    app.press(Action::Cancel);
    // The record is signed by the key and names BIP-87's origin.
    app.press(Action::CKeyBsms(0));
    let record = text(&outbox(&app, &format!("xpub-{fp}-bsms.txt")));
    let r = faraday_bip::bsms::SignerRecord::parse(&record).expect("a BIP 129 record");
    r.verify().expect("signed by its key");
    assert_eq!(r.key.key_text().replace('\'', "h"), want);
}

/// An app holding test seeds 0 and 1, with an imported 2-of-3 multisig
/// wallet over test seeds 0, 1 and 2 whose seed-0 key is at `p0` and
/// seed-1 key at `p1` (seed 2's is at BIP-48).
fn holding_mixed(p0: &str, p1: &str) -> (Faraday, usize) {
    let c = faraday_core::testkit::compute();
    let mut app = testkit::started();
    for n in [0, 1] {
        app.press(Action::Entry(None));
        for ch in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
            app.event(Event::Key(Key::Char(ch)));
        }
        app.press(Action::EntryAdd);
    }
    testkit::move_to(&mut app.session, testkit::net()).expect("the test network");
    let d = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, p0),
        testkit::key(1, p1),
        testkit::key(2, "m/48'/1'/0'/2'")
    );
    let w = app
        .session
        .add_wallet_confirmed(&c, "Mixed", &d, "test", faraday_core::wallet::Origin::Named)
        .expect("a wallet");
    (app, w)
}

/// Each key of a wallet whose keys are at BIP-48 and BIP-87 gets the
/// account code and the signed BIP 129 record of its own path.
fn each_key_names_its_own_path(p0: &str, p1: &str) {
    let (mut app, w) = holding_mixed(p0, p1);
    for (slot, path) in [(0u8, p0), (1u8, p1)] {
        let purpose = path.split('/').nth(1).unwrap().replace('\'', "h");
        let key = app.session.wallets[w].policy().keys()[usize::from(slot)].clone();
        let fp = String::from(&*faraday_core::wallet::fp_text(key.fingerprint().unwrap()));
        app.press(Action::ShowCode(Code::WalletKey(w, slot)));
        assert!(
            shown(&app).to_lowercase().starts_with("ur:crypto-account"),
            "slot {slot} has a crypto-account code"
        );
        let sheet = faraday_codec::png::qr_png(&app.qr().unwrap().frames[0], 4);
        assert_eq!(
            key_scanned("sheet.png", &sheet),
            key.key_text().replace('\'', "h"),
            "slot {slot}"
        );
        assert!(
            label(&app).contains(&format!("m/{purpose}/")),
            "{}",
            label(&app)
        );
        app.press(Action::Cancel);
        app.press(Action::WalletKeyBsms(w, slot));
        let record = text(&outbox(&app, &format!("xpub-{fp}-bsms.txt")));
        let r = faraday_bip::bsms::SignerRecord::parse(&record).expect("a BIP 129 record");
        r.verify().expect("signed by its key");
        assert_eq!(r.key.key_text(), key.key_text(), "slot {slot}");
    }
}

#[test]
fn a_wallet_with_a_bip_48_key_and_a_bip_87_key_gives_each_its_own_code_and_record() {
    each_key_names_its_own_path("m/48'/1'/0'/2'", "m/87'/1'/0'");
}

#[test]
fn a_wallet_with_a_bip_87_key_and_a_bip_48_key_gives_each_its_own_code_and_record() {
    each_key_names_its_own_path("m/87'/1'/0'", "m/48'/1'/0'/2'");
}

/// A picture read on a stick visit, as the cosigner's Faraday reads it:
/// the key it holds.
fn key_scanned(name: &str, png: &[u8]) -> String {
    let c = faraday_core::testkit::compute();
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S1"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
    app.storage(StorageEvent::QrRead {
        stick: faraday_text::Text::of("S1"),
        name: faraday_text::Text::of(name),
        bytes: (png.to_vec()).into(),
        payloads: (faraday_files::qr_in_png(&c, png).expect("a PNG").to_vec()).into(),
    });
    let item = app
        .inbox()
        .iter()
        .find(|i| i.kind == FileKind::Key)
        .unwrap_or_else(|| panic!("no key read: {:?}", app.visit().log));
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
    let q = app.qr().unwrap();
    assert!(q.offers_png());
    let sheet = faraday_codec::png::qr_png(&q.frames[0], 4);
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
    let c = testkit::compute();
    let mut app = with_key();
    let fp = String::from(&*faraday_core::wallet::fp_text(
        app.session.keys[0].fingerprint,
    ));
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
    let address = app.silent_address(&c).unwrap();
    app.press(Action::SQr(false));
    assert_eq!(shown(&app), address.as_str());
    let l = label(&app);
    assert!(l.contains(&format!("Silent payment address · {fp}")), "{l}");
    app.press(Action::QrPng);
    assert_eq!(
        read_png(&outbox(&app, &format!("silent-{fp}.png"))),
        address.as_str()
    );
}

#[test]
fn a_signed_message_reads_back_from_its_code_and_its_picture() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
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
        .add_wallet_confirmed(
            &c,
            spending.name,
            &spending.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "Faraday signs this");
    app.press(Action::MSign);
    let address = testkit::message(&app)
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

/// A second message from one address goes beside the first, and the
/// toast names the new file.
#[test]
fn a_second_message_from_one_address_goes_beside_the_first() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
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
        .add_wallet_confirmed(
            &c,
            spending.name,
            &spending.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app.press(Action::SignMessage);
    for text in ["First", "Second"] {
        testkit::message_text(&mut app, text);
        // Signing puts the message For the stick.
        app.press(Action::MSign);
    }
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    let _ = app.frame();
    let own = app
        .outbox
        .iter()
        .position(|i| i.name.ends_with("-2.txt"))
        .unwrap();
    assert!(app.offers(Action::OutboxRemove(own)));
    assert!(!app.offers(Action::OutboxRemove(0)), "the first's file");
    let names: Vec<&str> = app.outbox.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names[1].ends_with("-2.txt"), "{names:?}");
    assert!(app.toast_text().unwrap().contains(names[1]));
}

/// An app with a new GPG key in the open test vault, its certificate
/// and revocation in the Outbox, and a file in the Inbox to sign.
fn with_gpg_key() -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            vault,
            (
                faraday_text::Text::of("SHA256SUMS").to_string(),
                b"3d6f  faraday-0.1.0-linux-x86_64\n".to_vec(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_482_652,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..21 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    // The clock checked, as on the stick before making a key.
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    crate::common::make_released(
        &mut app,
        faraday_core::backup_flow::Kind::Gpg,
        &["Release", ""],
    );
    app
}

#[test]
fn a_gpg_public_key_reads_back_from_its_code_and_its_picture() {
    let mut app = with_gpg_key();
    let asc = app
        .outbox
        .iter()
        .map(|i| String::from(&*i.name))
        .find(|n| n.ends_with(".asc"))
        .expect("the public key");
    let stem = asc.trim_end_matches(".asc").to_string();
    let want = text(&outbox(&app, &asc));
    reads_back(&mut app, Code::GpgKey, &format!("{stem}.png"), &want);
}

#[test]
fn a_gpg_signature_reads_back_from_its_code_and_its_picture() {
    let c = faraday_core::testkit::compute();
    let mut app = with_gpg_key();
    app.press(Action::Vault(V::GpgSignPick));
    let k = app
        .inbox()
        .iter()
        .filter(|i| !i.name.ends_with(".asc"))
        .position(|i| i.name == "SHA256SUMS")
        .unwrap();
    app.press(Action::Vault(V::GpgTick(k)));
    // Sign, then the hold on its sheet (F63).
    app.press(Action::Vault(V::GpgSign));
    app.press(Action::Vault(V::GpgHoldSign));
    let want = text(&outbox(&app, "SHA256SUMS.asc"));
    let code = app.signature_code(&c, "SHA256SUMS").unwrap();
    reads_back(&mut app, code, "SHA256SUMS.asc.png", &want);
}

#[test]
fn a_secret_shown_as_a_code_offers_no_picture() {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("key-words.txt"),
            format!("{}\n", &*testkit::test_words(testkit::TEST_SEEDS[0].0)).into_bytes(),
        )]),
        kept: Vec::new(),
    });
    app.press(Action::QrOutbox(0));
    let q = app.qr().expect("no QR");
    assert!(q.secret);
    assert!(!q.offers_png());
    app.press(Action::QrPng);
    assert_eq!(app.outbox.len(), 1);
}

/// Specter's numbered parts, read one code after another as the scanner
/// reads them, put together.
fn scanned_parts(codes: &[Vec<u8>]) -> String {
    let mut a = faraday_qr::Assembler::default();
    let mut last = None;
    for png in codes {
        last = Some(a.feed(&read_png(png)));
    }
    match last {
        Some(faraday_qr::Step::Done(faraday_qr::Arrived::Text(t))) => t,
        other => panic!("not put together: {other:?}"),
    }
}

#[test]
fn specter_diys_addwallet_reads_back_from_its_numbered_parts_shown_and_as_pictures() {
    let (mut app, w) = holding("Savings", &testkit::savings());
    app.press(Action::PublicOut(w, 12));
    let want = text(&outbox(&app, "savings-specter-diy.txt"));
    assert!(
        want.starts_with("addwallet Savings&wsh(sortedmulti(2,"),
        "{want}"
    );
    app.press(Action::ShowCode(Code::SpecterDiy(w)));
    let q = app.qr().expect("no QR sheet");
    assert!(q.frames.len() > 1, "one code");
    let shown: Vec<Vec<u8>> = q
        .frames
        .iter()
        .map(|f| faraday_codec::png::qr_png(f, 4))
        .collect();
    assert_eq!(scanned_parts(&shown), want);
    app.press(Action::Cancel);
    app.press(Action::PublicOut(w, 14));
    let mut pictures: Vec<(String, Vec<u8>)> = app
        .outbox
        .iter()
        .filter(|i| i.name.starts_with("savings-specter-diy-part-"))
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .collect();
    assert_eq!(pictures.len(), q_frames(&mut app, w));
    pictures.sort();
    let pictures: Vec<Vec<u8>> = pictures.into_iter().map(|(_, b)| b).collect();
    assert_eq!(scanned_parts(&pictures), want);
    // Read back, the command is the wallet, under its name.
    let c = testkit::compute();
    let back = faraday_core::wallet::read_wallet(&c, &want).unwrap();
    assert_eq!(
        back.to_descriptor(),
        app.session.wallets[w].policy().to_descriptor()
    );
}

/// How many codes the sheet shows wallet `w`'s addwallet command in.
fn q_frames(app: &mut Faraday, w: usize) -> usize {
    app.press(Action::ShowCode(Code::SpecterDiy(w)));
    let n = app.qr().expect("no QR sheet").frames.len();
    app.press(Action::Cancel);
    n
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
        outbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            spending.descriptor.clone().into_bytes(),
        )]),
        kept: Vec::new(),
    });
    app.press(Action::QrOutbox(0));
    assert_eq!(shown(&app), spending.descriptor);
    assert!(!app.qr().unwrap().offers_png());
    app.press(Action::QrPng);
    assert_eq!(app.outbox.len(), 1);
}

/// A seed file titled with an address, its words six to a line, is not a signed message
/// (a signature is one base64 token, and a line of words is not): it is a plain text file,
/// as one with any other title is.
#[test]
fn an_address_titled_seed_file_is_text_not_a_message() {
    let c = testkit::compute();
    let file = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\nlimb sadness employ night rocket crime\ngarden timber title eight spy stay\ncompany embark secret face fork stable\nmosquito lock visual bamboo decide velvet\n";
    assert_eq!(
        faraday_core::wallet::classify(&c, "seed.txt", file.as_bytes()),
        FileKind::Text
    );
}

/// A seed written one word per line under an address title is a plain text file (as with any
/// other title), not a message, even when
/// its first word is valid base64 (`army`): it is not a signature.
#[test]
fn an_address_titled_seed_one_word_per_line_is_text_not_a_message() {
    let c = testkit::compute();
    let file = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\narmy\nvan\ndefense\ncarry\njealous\ntrue\ngarbage\nclaim\necho\nmedia\nmake\ncrunch\n";
    assert_eq!(
        faraday_core::wallet::classify(&c, "seed.txt", file.as_bytes()),
        FileKind::Text
    );
}

/// A signed message goes as its own text, public; one whose body is a
/// seed's words is not a signed message to Faraday and does not go so.
#[test]
fn a_signed_message_holding_seed_words_does_not_go_as_the_message() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    let signed =
        |body: &str| format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{body}\n");
    let ordinary = signed("Pay the invoice by Friday.");
    let seed = signed(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    assert_eq!(
        faraday_core::wallet::classify(&c, "message.txt", ordinary.as_bytes()),
        FileKind::Message
    );
    assert_ne!(
        faraday_core::wallet::classify(&c, "message.txt", seed.as_bytes()),
        FileKind::Message
    );
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("message.txt"),
            ordinary.clone().into_bytes(),
        )]),
        kept: Vec::new(),
    });
    app.press(Action::QrOutbox(0));
    assert_eq!(shown(&app), ordinary);
}

/// A signed message holding a seed is a secret file as the same words in a
/// plain text file are: its code is marked secret and offers no picture.
#[test]
fn a_signed_message_holding_a_seed_is_a_secret_file() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    let words12 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let words24 = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art";
    for words in [words12, words24] {
        let signed = format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{words}\n");
        let plain = format!("{words}\n");
        let kind = faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes());
        assert_eq!(
            kind,
            faraday_core::wallet::classify(&c, "words.txt", plain.as_bytes())
        );
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: Vec::new(),
            outbox: faraday_core::files_from(vec![(
                faraday_text::Text::of("message.txt"),
                signed.into_bytes(),
            )]),
            kept: Vec::new(),
        });
        app.press(Action::QrOutbox(0));
        let q = app.qr().expect("no QR");
        assert!(q.secret);
        assert!(!q.offers_png());
    }
}

/// A signed message whose body holds a seed beside other text, an
/// authenticator's secret or an extended private key on a line of its own
/// is a secret file too, never a message.
#[test]
fn a_signed_message_holding_a_secret_beside_other_text_is_a_secret_file() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let xprv = "xprv9s21ZrQH143K2nNuAbfWPHBtfiSCS14XQgb3otW4pX655q58EEZeC8zmjEUwucBu9dPnxdpbZLCn57yx45RBkwJHnwHFjZK4XPJ8SyeYjYg";
    for body in [
        format!("My seed, keep it safe:\n{words}\nThank you."),
        "Scan this:\notpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example"
            .to_string(),
        format!("The key is\n{xprv}\nfor the old wallet."),
        format!("Name: Savings\n# {words}\nPolicy: 2 of 3"),
        format!("The words are\n{words}."),
    ] {
        let signed = format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{body}\n");
        let mut app = testkit::started();
        app.storage(StorageEvent::Restored {
            inbox: Vec::new(),
            outbox: faraday_core::files_from(vec![(
                faraday_text::Text::of("message.txt"),
                signed.clone().into_bytes(),
            )]),
            kept: Vec::new(),
        });
        assert_ne!(
            faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes()),
            FileKind::Message,
            "{body}"
        );
        app.press(Action::QrOutbox(0));
        assert!(app.qr().expect("no QR").secret, "{body}");
    }
}

/// A valid French seed of one word with two accents, in the forms a person
/// may write it with only some of its accents: the first accent left off a
/// composed word (`bénefice`), and off a decomposed one.
fn half_accented_french_seeds() -> Vec<String> {
    use faraday_bip::bip39::Language;
    let c = testkit::compute();
    let l = Language::French;
    let word = (0..2048u16)
        .find(|&i| l.word_nfkd(i).chars().filter(|c| !c.is_ascii()).count() >= 2)
        .expect("a two-accent word");
    let (shown, nfkd) = (l.word_display(word), l.word_nfkd(word));
    let last = (0..2048u16)
        .map(|i| l.word_display(i))
        .find(|w| {
            faraday_keys::words::parse(&c, l, &format!("{} {w}", [shown; 11].join(" "))).is_ok()
        })
        .expect("a closing word");
    let first_accent = shown.chars().position(|c| !c.is_ascii()).unwrap();
    let composed: String = shown
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let plain = match c {
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'à' | 'â' => 'a',
                'î' | 'ï' => 'i',
                'ô' => 'o',
                'ù' | 'û' | 'ü' => 'u',
                'ç' => 'c',
                other => other,
            };
            if i == first_accent { plain } else { c }
        })
        .collect();
    let mut dropped = false;
    let decomposed: String = nfkd
        .chars()
        .filter(|c| {
            let mark = ('\u{0300}'..='\u{036F}').contains(c);
            !(mark && !std::mem::replace(&mut dropped, true))
        })
        .collect();
    [composed, decomposed]
        .into_iter()
        .map(|w| format!("{} {last}", [w.as_str(); 11].join(" ")))
        .collect()
}

/// Published BIP-39 vectors in two other lists (`tools/vectors/bip39`):
/// Spanish (entropy of zeros) and Japanese (`test_JP_BIP39.json`, whose
/// words are joined by the ideographic space).
const SEED_SPANISH: &str =
    "ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco ábaco abierto";
/// The Spanish vector as it is typed or scanned without its accents.
const SEED_SPANISH_PLAIN: &str =
    "abaco abaco abaco abaco abaco abaco abaco abaco abaco abaco abaco abierto";
const SEED_JAPANESE: &str = "あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あいこくしん\u{3000}あおぞら";

/// The layouts of `hidden_secret_comments` for a seed in another list,
/// where they apply: beside a label, one word to a line with its place,
/// wrapped over lines and in capitals.
fn hidden_seed_comments(seed: &str) -> Vec<String> {
    let words: Vec<&str> = seed.split_whitespace().collect();
    let wrapped: String = words
        .chunks(5)
        .map(|c| format!("# {}\n", c.join(" ")))
        .collect();
    let numbered: String = words
        .iter()
        .enumerate()
        .map(|(i, w)| format!("# {}. {w}\n", i + 1))
        .collect();
    vec![
        format!("# seed: {seed} here\n"),
        format!("# {}\n", words.join(" ")),
        wrapped,
        numbered,
        format!("# Seed: {} here\n", seed.to_uppercase()),
    ]
}

/// A seed in Spanish or Japanese beside other text is a secret file, as
/// one in English is: in a share sheet's comment, in a signed message and
/// alone in a text file.
#[test]
fn a_seed_in_another_language_beside_other_text_is_a_secret_file() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    for seed in [SEED_SPANISH, SEED_SPANISH_PLAIN, SEED_JAPANESE] {
        // Alone in a file the accentless seed is a text file, not a seed
        // file (the seed reader does not fold); a text file is secret too.
        if seed != SEED_SPANISH_PLAIN {
            assert_eq!(
                faraday_core::wallet::classify(&c, "seed.txt", seed.as_bytes()),
                FileKind::Words,
                "{seed}"
            );
        }
        for extra in hidden_seed_comments(seed) {
            assert_ne!(
                faraday_core::wallet::classify(&c, "share.txt", share_sheet(&extra).as_bytes()),
                FileKind::Share,
                "{extra}"
            );
            let signed =
                format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{extra}");
            assert_ne!(
                faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes()),
                FileKind::Message,
                "{extra}"
            );
        }
    }
    // A French seed written with only some of its accents is a secret too.
    for seed in half_accented_french_seeds() {
        for extra in hidden_seed_comments(&seed) {
            assert_ne!(
                faraday_core::wallet::classify(&c, "share.txt", share_sheet(&extra).as_bytes()),
                FileKind::Share,
                "{extra}"
            );
            let signed =
                format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{extra}");
            assert_ne!(
                faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes()),
                FileKind::Message,
                "{extra}"
            );
        }
    }
    // Ordinary Spanish words in a comment stay public.
    assert_eq!(
        faraday_core::wallet::classify(
            &c,
            "share.txt",
            share_sheet("# ábaco abierto abogado abono aborto abrazo abrir abuelo\n").as_bytes()
        ),
        FileKind::Share
    );
}

const SEED12: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A multisig share sheet of one key of three, with `extra` lines after it.
fn share_sheet(extra: &str) -> String {
    let k = testkit::key(0, "m/48'/1'/0'/2'");
    let (origin, xpub) = k.trim_start_matches('[').split_once(']').unwrap();
    let (fp, path) = origin.split_once('/').unwrap();
    format!(
        "Name: Split\nPolicy: 2 of 3\nFormat: P2WSH\nDerivation: m/{path}\n{}: {xpub}\n{extra}",
        fp.to_uppercase()
    )
}

/// A share sheet is public; one with a seed in a comment or in a line the
/// reader does not know is a secret file, dropped at a lock.
#[test]
fn a_share_sheet_with_a_seed_in_a_comment_or_unknown_line_is_a_secret_file() {
    let c = testkit::compute();
    let kind = |t: &str| faraday_core::wallet::classify(&c, "share.txt", t.as_bytes());
    assert_eq!(kind(&share_sheet("")), FileKind::Share);
    assert_eq!(
        kind(&share_sheet(
            "# Printed for the safe at the bank, second floor, keep it dry.\n"
        )),
        FileKind::Share
    );
    for extra in [
        format!("# {SEED12}\n"),
        format!("seed: {SEED12}\n"),
        format!("Note: \"{SEED12}\"\n"),
        format!("# my seed is {SEED12}. keep it safe\n"),
        "# xprv9s21ZrQH143K2nNuAbfWPHBtfiSCS14XQgb3otW4pX655q58EEZeC8zmjEUwucBu9dPnxdpbZLCn57yx45RBkwJHnwHFjZK4XPJ8SyeYjYg\n"
            .to_string(),
    ] {
        let t = share_sheet(&extra);
        assert_ne!(kind(&t), FileKind::Share, "{extra}");
        assert_ne!(kind(&t), FileKind::Message, "{extra}");
    }
}

const SLIP20: &str = "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed";
const SLIP33: &str = "wildlife deal beard romp alcohol space mild usual clothes union nuclear testify course research heat listen task location thank hospital slice smell failure fawn helpful priest ambition average recover lecture process dough stadium";
const WIF: &str = "L1aW4aubDFB7yfras2S1mN3bqg9nwySY8nkoLmJebSLD5BWv3ENZ";

/// Comment lines carrying a secret in a layout that is easy to miss:
/// capitals, a share over many or numbered lines, a seed numbered one
/// word to a line, a WIF key.
fn hidden_secret_comments() -> Vec<String> {
    let wrapped = |words: &str, per: usize| {
        words
            .split(' ')
            .collect::<Vec<_>>()
            .chunks(per)
            .map(|c| format!("# {}\n", c.join(" ")))
            .collect::<String>()
    };
    let numbered = |words: &str| {
        words
            .split(' ')
            .enumerate()
            .map(|(i, w)| format!("# {}. {w}\n", i + 1))
            .collect::<String>()
    };
    vec![
        format!("# Seed: {} here\n", SEED12.to_uppercase()),
        numbered(SEED12),
        wrapped(SLIP20, 5),
        wrapped(SLIP33, 8),
        numbered(SLIP20),
        numbered(SLIP33),
        format!("# Share: {}\n", SLIP33.to_uppercase()),
        format!("# key: {WIF}\n"),
    ]
}

/// A share file with a secret in a comment, in any of these layouts, is
/// a secret file; a comment of ordinary words stays public.
#[test]
fn a_share_sheet_with_a_hidden_secret_in_a_comment_is_a_secret_file() {
    let c = testkit::compute();
    let kind = |t: &str| faraday_core::wallet::classify(&c, "share.txt", t.as_bytes());
    for extra in hidden_secret_comments() {
        let t = share_sheet(&extra);
        assert_ne!(kind(&t), FileKind::Share, "{extra}");
    }
    assert_eq!(
        kind(&share_sheet(
            "# academic always wildlife fancy gross oasis cylinder\n"
        )),
        FileKind::Share
    );
}

/// The same layouts in a signed message's body make a secret file.
#[test]
fn a_signed_message_with_a_hidden_secret_is_a_secret_file() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    for body in hidden_secret_comments() {
        let signed = format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{body}");
        assert_ne!(
            faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes()),
            FileKind::Message,
            "{body}"
        );
    }
}

/// A signed message whose body holds a seed or a share in any shape is a
/// secret file; prose with many seed-list words and an ordinary message
/// stay a public message.
#[test]
fn a_signed_message_with_a_secret_in_any_shape_is_a_secret_file() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    let kind = |body: &str| {
        let signed = format!("bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{body}\n");
        faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes())
    };
    let commas = SEED12.replace(' ', ", ");
    let slip = [
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder",
        "mustang wrist rescue view short owner flip making coding armed",
    ];
    let slip_one = slip.join(" ");
    let codex32 = "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw";
    let migration = "otpauth-migration://offline?data=CjEKCkhlbGxvId6tvu8SGEV4YW1wbGU6YWxpY2VAZ29vZ2xlLmNvbRoHRXhhbXBsZSABKAEwAhABGAEgACjd";
    for body in [
        format!("My seed is {SEED12}. Keep it safe."),
        format!("My seed is \"{SEED12}\""),
        commas,
        format!("Share A: {codex32}"),
        format!("Scan this: {migration}"),
        format!("Backup:\n{}\n{}", slip[0], slip[1]),
        format!("# # {SEED12}"),
        format!("Notes\n{migration}"),
        format!("Notes, thanks\n{slip_one}"),
        format!("Notes, thanks\nShare 1 {codex32}"),
    ] {
        assert_ne!(kind(&body), FileKind::Message, "{body}");
    }
    for body in [
        "Pay the invoice by Friday.",
        "Please sign and send the letter about the army, the actor, the access and the actual account. Another address is above, again.",
        "Abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual adapt add",
    ] {
        assert_eq!(kind(body), FileKind::Message, "{body}");
    }
}

/// A wallet card's first receive address opens its code: the address as
/// the card prints it, whole under the code, with no picture
/// (`docs/QR.md` §7).
#[test]
fn the_wallet_cards_receive_address_opens_its_code() {
    let c = testkit::compute();
    let (mut app, w) = holding("Savings", &testkit::savings());
    // Every key checked: the wallet node has no key-check line and the
    // card no caution over the address.
    for k in 0..app.session.wallets[w].key_checks().len() {
        app.session.tick_key(w, k);
    }
    app.press(Action::OpenWallet(w));
    let want = app
        .session
        .address_shown(&c, &app.session.wallets[w], false, 0);
    crate::common::card_to_action(&mut app, Action::ShowCode(Code::Receive(w, 0)));
    crate::common::opens_its_code(&mut app, Code::Receive(w, 0), &want);
}

/// An app with a vault holding one recovery sheet of `descriptor`, named
/// `name`, unlocked and on the sheet's page.
fn on_a_sheet(name: &str, descriptor: &str) -> Faraday {
    use faraday_vault::Record;
    use faraday_vault::records::{field, kind};
    let payload =
        faraday_vault::layouts::sheet_payload(descriptor.as_bytes(), name.as_bytes(), b"")
            .expect("a sheet");
    let records = vec![Record::new(kind::SHEET).with(field::SHEET, &payload)];
    let vault = crate::common::vault_of("Sheets", "sheets", 0x5e, records);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("sheets.kdbx"), vault)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, "sheets");
    app.press(Action::Vault(V::Unlock));
    for t in 1..21 {
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
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(3)));
    app.press(Action::Vault(V::Item(0)));
    app
}

#[test]
fn a_recovery_sheets_descriptor_reads_back_from_its_code_and_its_picture() {
    let descriptor = testkit::savings();
    let mut app = on_a_sheet("Savings", &descriptor);
    assert!(crate::common::offered(
        &mut app,
        Action::ShowCode(Code::VaultSheet)
    ));
    assert!(crate::common::offered(
        &mut app,
        Action::CodePng(Code::VaultSheet)
    ));
    reads_back(
        &mut app,
        Code::VaultSheet,
        "savings-descriptor.png",
        &descriptor,
    );
    assert!(
        !app.outbox
            .iter()
            .find(|i| i.name == "savings-descriptor.png")
            .unwrap()
            .secret
    );
    app.press(Action::ShowCode(Code::VaultSheet));
    assert!(label(&app).contains("Public: watch only, spends nothing"));
}
