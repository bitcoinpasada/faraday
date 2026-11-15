//! An address the address check finds in a single-key wallet whose key is
//! loaded offers **Sign a message with this address**: Sign a message
//! opens on that address, and what it signs checks in Check a message.
//! The same holds for a find in a loaded key's own account, with or without
//! a wallet. A multisig find, a watch-only find and a find whose key is not
//! loaded offer nothing.

use faraday_bip::policy::WalletPolicy;
use faraday_core::address_check::AddressPress;
use faraday_core::scan_read::Way;
use faraday_core::wallet::Origin;
use faraday_core::{Action, Ask, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event, Key};

/// The best-known public test mnemonic, whose addresses the BIPs' vectors
/// give.
const WORDS: &str = "abandon abandon abandon abandon abandon abandon \
                     abandon abandon abandon abandon abandon about";

/// Faraday with `WORDS` loaded where `key`, each of `descriptors` a wallet,
/// and the address check's scanner open.
fn checking(descriptors: &[String], key: bool) -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    if key {
        app.session
            .keys
            .add_words(&c, &faraday_text::Text::of(WORDS), "Abandon", None)
            .unwrap();
    }
    for (i, d) in descriptors.iter().enumerate() {
        app.session
            .add_wallet(&c, &format!("Wallet {i}"), d, "test", Origin::Named)
            .unwrap();
    }
    app.press(Action::ScanFor(Ask::Address));
    app
}

/// The single-key wallet over `WORDS`' account 0 of `kind`.
fn account(kind: faraday_core::create::NewKind) -> String {
    let c = testkit::compute();
    let mut keys = testkit::session();
    keys.keys
        .add_words(&c, &faraday_text::Text::of(WORDS), "Abandon", None)
        .unwrap();
    let spec = kind.spec(keys.network(), 0).unwrap();
    let key = keys.keys.key_text(&c, 0, &spec).unwrap().unwrap();
    String::from(&*kind.descriptor(1, &[key.as_str()]))
}

fn address(descriptor: &str, change: bool, index: u32) -> String {
    let c = testkit::compute();
    WalletPolicy::parse_any(&c, descriptor)
        .unwrap()
        .address_at(&c, testkit::net(), change, index)
        .unwrap()
        .to_string()
}

fn read(app: &mut Faraday, text: &str) {
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

/// The result's ways over.
fn ways(app: &Faraday) -> Vec<Way> {
    app.address_check_state()
        .map(|p| p.ways().to_vec())
        .unwrap_or_default()
}

fn said(app: &Faraday) -> String {
    app.address_check_state()
        .map(|p| {
            p.result_lines()
                .iter()
                .map(|f| String::from(f.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn sign_at(app: &Faraday) -> Option<usize> {
    ways(app).iter().position(|w| matches!(w, Way::SignAddress))
}

#[test]
fn a_found_address_signs_a_message_that_checks() {
    use faraday_core::create::NewKind;
    for kind in [NewKind::NativeSegwit, NewKind::Taproot] {
        let d = account(kind);
        let mut app = checking(std::slice::from_ref(&d), true);
        let found = address(&d, false, 3);
        read(&mut app, &found);
        let i = sign_at(&app).expect("the result offers Sign a message");
        assert_eq!(
            ways(&app)[i].label().to_string(),
            "Sign a message with this address"
        );
        app.press(Action::AddressCheck(AddressPress::Way(i as u8)));
        assert_eq!(app.screen, Screen::Message);
        let m = testkit::message(&app).unwrap();
        assert_eq!((m.wallet, m.index), (Some(0), 3));
        testkit::message_text(&mut app, "Pay the bearer of this message");
        app.press(Action::MSign);
        let m = testkit::message(&app).unwrap();
        assert_eq!(m.error, None);
        let signed = m.signed.as_ref().expect("signed");
        assert_eq!(signed.address.to_string(), found);
    }
}

#[test]
fn the_signature_of_a_found_address_checks_in_check_a_message() {
    let d = account(faraday_core::create::NewKind::NativeSegwit);
    let mut app = checking(std::slice::from_ref(&d), true);
    read(&mut app, &address(&d, false, 0));
    let i = sign_at(&app).expect("offered");
    app.press(Action::AddressCheck(AddressPress::Way(i as u8)));
    testkit::message_text(&mut app, "Hello");
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    let signed = m.signed.as_ref().expect("signed");
    // Checked against Faraday's own verifier: no published vector is
    // asserted here.
    let text = format!("{}\n{}\nHello", &*signed.address, &*signed.signature);
    let c = testkit::compute();
    let check = app.session.check_message(&c, &text).expect("a signed text");
    assert!(check.verdict.is_ok(), "{:?}", check.verdict);
    assert_eq!(check.wallet, Some((0, false, 0)));
}

#[test]
fn a_multisig_find_offers_nothing() {
    let d = testkit::savings();
    let c = testkit::compute();
    let mut app = checking(std::slice::from_ref(&d), false);
    let (words, name) = testkit::TEST_SEEDS[0];
    app.session
        .keys
        .add_words(&c, &testkit::test_words(words), name, None)
        .unwrap();
    read(&mut app, &address(&d, false, 1));
    assert_eq!(sign_at(&app), None);
}

#[test]
fn a_find_whose_key_is_not_loaded_offers_nothing() {
    let d = account(faraday_core::create::NewKind::NativeSegwit);
    let mut app = checking(std::slice::from_ref(&d), false);
    read(&mut app, &address(&d, false, 1));
    assert!(said(&app).contains("Found in Wallet 0"), "{}", said(&app));
    assert_eq!(sign_at(&app), None);
}

/// The single-key wallet over `words`' account 0 of native SegWit.
fn account_of(words: &str) -> String {
    let c = testkit::compute();
    let mut keys = testkit::session();
    keys.keys
        .add_words(&c, &faraday_text::Text::of(words), "Other", None)
        .unwrap();
    let kind = faraday_core::create::NewKind::NativeSegwit;
    let spec = kind.spec(keys.network(), 0).unwrap();
    let key = keys.keys.key_text(&c, 0, &spec).unwrap().unwrap();
    String::from(&*kind.descriptor(1, &[key.as_str()]))
}

/// No wallet can hold the loaded key's fingerprint with another seed's
/// key (adding one is refused), so the refusal is reached from the session
/// with an address that is another seed's.
#[test]
fn a_key_that_gives_another_address_at_the_path_refuses_to_sign() {
    use faraday_bip::keys::{Fingerprint, ScriptType};
    let other = account_of("all all all all all all all all all all all all");
    let fingerprint = Fingerprint([0x73, 0xc5, 0xda, 0x0a]);
    let c = testkit::compute();
    let mut session = testkit::session();
    session
        .keys
        .add_words(&c, &faraday_text::Text::of(WORDS), "Abandon", None)
        .unwrap();
    let at = faraday_core::wallet::MessageAt {
        fingerprint,
        path: "m/84'/1'/0'/0/0".parse().unwrap(),
        script: ScriptType::NativeSegwit,
        change: false,
        index: 0,
        address: faraday_text::Text::of(&address(&other, false, 0)),
        line: faraday_text::Text::of("73c5da0a · m/84h/1h/0h/0/0"),
    };
    let err = session
        .sign_message_at(&c, &at, "Hello", faraday_psbt::message::Format::Bip322)
        .expect_err("refused");
    assert_eq!(&*err, "Key 73c5da0a does not give this address at its path");
    let long = faraday_core::wallet::MessageAt {
        path: format!("m{}", "/0".repeat(17)).parse().unwrap(),
        ..at
    };
    let err = session
        .sign_message_at(&c, &long, "Hello", faraday_psbt::message::Format::Bip322)
        .expect_err("refused");
    assert_eq!(&*err, "Key 73c5da0a cannot derive this path");
}

#[test]
fn a_find_in_the_second_wallet_opens_on_it_with_a_format_it_can_sign() {
    use faraday_core::create::NewKind::{NativeSegwit, NestedSegwit, Taproot};
    // BIP-322 is chosen for the first wallet and BIP-137 only fits the
    // second, and the other way round.
    for (first, second, bip322) in [
        (NativeSegwit, NestedSegwit, false),
        (NestedSegwit, Taproot, true),
    ] {
        let ds = [account(first), account(second)];
        let mut app = checking(&ds, true);
        read(&mut app, &address(&ds[1], false, 2));
        let i = sign_at(&app).expect("offered");
        app.press(Action::AddressCheck(AddressPress::Way(i as u8)));
        let m = testkit::message(&app).unwrap();
        assert_eq!((m.wallet, m.index), (Some(1), 2));
        assert_eq!(m.bip322, bip322);
        testkit::message_text(&mut app, "Hello");
        app.press(Action::MSign);
        let m = testkit::message(&app).unwrap();
        assert_eq!(m.error, None);
        assert!(m.signed.is_some());
    }
}

#[test]
fn keys_typed_after_opening_on_a_found_address_reach_the_message() {
    let d = account(faraday_core::create::NewKind::NativeSegwit);
    let mut app = checking(std::slice::from_ref(&d), true);
    read(&mut app, &address(&d, false, 0));
    let i = sign_at(&app).expect("offered");
    app.press(Action::AddressCheck(AddressPress::Way(i as u8)));
    for c in "Hi".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert_eq!(&*testkit::message(&app).unwrap().text, "Hi");
}

const MAIN: faraday_bip::keys::Network = faraday_bip::keys::Network::Mainnet;

/// Faraday on Mainnet with `words` loaded, `descriptor` a wallet when
/// given, and the address check's scanner open.
fn main_checking(words: &str, descriptor: Option<&str>) -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session
        .keys
        .add_words(&c, &faraday_text::Text::of(words), "Key", None)
        .unwrap();
    if let Some(d) = descriptor {
        app.session
            .add_wallet(&c, "Wallet", d, "test", Origin::Named)
            .unwrap();
    }
    app.press(Action::ScanFor(Ask::Address));
    app
}

/// `WORDS`' account 0 of `kind` on Mainnet, as a single-key wallet.
fn main_account(kind: faraday_core::create::NewKind) -> String {
    let c = testkit::compute();
    let mut keys = faraday_core::wallet::Session::on(MAIN);
    keys.keys
        .add_words(&c, &faraday_text::Text::of(WORDS), "Abandon", None)
        .unwrap();
    let spec = kind.spec(MAIN, 0).unwrap();
    let key = keys.keys.key_text(&c, 0, &spec).unwrap().unwrap();
    String::from(&*kind.descriptor(1, &[key.as_str()]))
}

/// The scan's Sign way pressed on a find.
fn press_sign(app: &mut Faraday) {
    let i = sign_at(app).expect("the result offers Sign a message");
    app.press(Action::AddressCheck(AddressPress::Way(i as u8)));
    assert_eq!(app.screen, Screen::Message);
}

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for k in 0..4 {
            if k <= chunk.len() {
                out.push(A[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn signs_and_checks(app: &mut Faraday, expect: &str, bip322: bool) {
    testkit::message_text(app, "Hello");
    app.press(Action::MFormat(bip322));
    app.press(Action::MSign);
    let m = testkit::message(app).unwrap();
    assert_eq!(m.error, None);
    let signed = m.signed.as_ref().expect("signed");
    assert_eq!(&*signed.address, expect);
    let text = format!("{}\n{}\nHello", &*signed.address, &*signed.signature);
    let c = testkit::compute();
    let check = app.session.check_message(&c, &text).expect("a signed text");
    assert!(check.verdict.is_ok(), "{:?}", check.verdict);
}

#[test]
fn a_key_with_no_wallet_signs_what_trezor_signs() {
    // Trezor's published p2sh-p2wpkh signature at m/49h/0h/0h/0/0. Trezor
    // signs with the first RFC 6979 nonce and Faraday grinds for a low r:
    // they agree here because r begins 0x74, under 2^255, so the first
    // nonce is already low. Trezor's p2wpkh and p2pkh cases have a high r
    // and would differ.
    let words = "all all all all all all all all all all all all";
    let mut app = main_checking(words, None);
    read(&mut app, "3L6TyTisPBmrDAj6RoKmDzNnj4eQi54gD2");
    assert!(said(&app).contains("Found in key "), "{}", said(&app));
    let labels: Vec<String> = ways(&app).iter().map(|w| w.label().to_string()).collect();
    assert_eq!(labels[0], "Sign a message with this address");
    assert_eq!(labels.len(), 2);
    press_sign(&mut app);
    let m = testkit::message(&app).unwrap();
    assert!(m.done[0] && !m.bip322);
    testkit::message_text(&mut app, "This is an example of a signed message.");
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.error, None);
    let signed = m.signed.as_ref().expect("signed");
    let trezor = "23744de4516fac5c140808015664516a32fead94de89775cec7e24dbc24fe133\
                  075ac09301c4cc8e197bea4b6481661d5b8e9bf19d8b7b8a382ecdb53c2ee0750d";
    let bytes: Vec<u8> = (0..trezor.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&trezor[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(&*signed.signature, base64(&bytes));
}

#[test]
fn a_change_address_in_a_wallet_signs_and_checks() {
    let d = main_account(faraday_core::create::NewKind::NativeSegwit);
    let found = "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el";
    for bip322 in [true, false] {
        let mut app = main_checking(WORDS, Some(&d));
        read(&mut app, found);
        press_sign(&mut app);
        assert_eq!(testkit::message(&app).unwrap().wallet, None);
        signs_and_checks(&mut app, found, bip322);
        let c = testkit::compute();
        let m = testkit::message(&app).unwrap();
        let text = format!(
            "{}\n{}\nHello",
            &*m.signed.as_ref().unwrap().address,
            &*m.signed.as_ref().unwrap().signature
        );
        let check = app.session.check_message(&c, &text).unwrap();
        assert_eq!(check.wallet, Some((0, true, 0)));
    }
}

#[test]
fn a_change_address_of_a_key_with_no_wallet_signs_in_its_script() {
    // BIP-86's first change address, signed in BIP-322, the only format
    // for Taproot.
    let found = "bc1p3qkhfews2uk44qtvauqyr2ttdsw7svhkl9nkm9s9c3x4ax5h60wqwruhk7";
    let mut app = main_checking(WORDS, None);
    read(&mut app, found);
    assert!(said(&app).contains("Found in key "), "{}", said(&app));
    press_sign(&mut app);
    assert!(testkit::message(&app).unwrap().bip322);
    signs_and_checks(&mut app, found, true);
}

#[test]
fn a_key_taken_away_before_signing_refuses() {
    let mut app = main_checking(WORDS, None);
    read(&mut app, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    press_sign(&mut app);
    app.session.keys.clear();
    testkit::message_text(&mut app, "Hello");
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert!(m.signed.is_none());
    assert_eq!(m.error.as_deref(), Some("Key 73c5da0a is not loaded here"));
}
