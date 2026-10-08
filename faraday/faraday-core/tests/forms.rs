//! Add a key in the other forms OpenSigner reads: SLIP-39 shares, codex32
//! strings and Seed XOR parts, each typed one at a time, against the
//! published vectors (SLIP-39's own, BIP 93's) and the repository's Seed
//! XOR vector.

use std::str::FromStr;

use faraday_core::{Action, Faraday, Screen};
use osk_bip::bitcoin::bip32::Xpriv;
use osk_bip::bitcoin::secp256k1::Secp256k1;
use osk_shell_api::{App, Event, Key};

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn enter(app: &mut Faraday) {
    app.event(Event::Key(Key::Enter));
}

fn adding(form: u8) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(form));
    app
}

fn fingerprint_of(xprv: &str) -> [u8; 4] {
    let x = Xpriv::from_str(xprv).unwrap();
    x.fingerprint(&Secp256k1::new()).to_bytes()
}

fn only_key(app: &Faraday) -> [u8; 4] {
    assert_eq!(app.session.keys.len(), 1, "{:?}", app.entry.error);
    app.session.keys[0].master.fingerprint().0
}

#[test]
fn two_slip39_shares_and_the_passphrase_make_the_vectors_key() {
    // SLIP-39 vector 4, "Basic sharing 2-of-3 (128 bits)", passphrase TREZOR.
    let mut app = adding(1);
    typed(
        &mut app,
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
    );
    enter(&mut app);
    // The same share twice is refused.
    typed(
        &mut app,
        "shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed",
    );
    enter(&mut app);
    assert!(app.entry.error.is_some());
    app.press(Action::EntryClear);
    typed(
        &mut app,
        "shadow pistol academic acid actress prayer class unknown daughter sweater depict flip twice unkind craft early superior advocate guest smoking",
    );
    enter(&mut app);
    app.press(Action::EntryPassphrase);
    typed(&mut app, "TREZOR");
    enter(&mut app);
    assert_eq!(
        only_key(&app),
        fingerprint_of(
            "xprv9s21ZrQH143K2nNuAbfWPHBtfiSCS14XQgb3otW4pX655q58EEZeC8zmjEUwucBu9dPnxdpbZLCn57yx45RBkwJHnwHFjZK4XPJ8SyeYjYg"
        )
    );
    assert_eq!(app.screen, Screen::Wallets);
}

#[test]
fn a_codex32_secret_is_the_key_at_once() {
    // BIP 93 vector 1.
    let mut app = adding(2);
    typed(&mut app, "ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxx4nzvca9cmczlw");
    enter(&mut app);
    assert_eq!(
        only_key(&app),
        fingerprint_of(
            "xprv9s21ZrQH143K3taPNekMd9oV5K6szJ8ND7vVh6fxicRUMDcChr3bFFzuxY8qP3xFFBL6DWc2uEYCfBFZ2nFWbAqKPhtCLRjgv78EZJDEfpL"
        )
    );
}

#[test]
fn two_codex32_shares_make_the_key_typed_in_capitals() {
    // BIP 93 vector 2.
    let mut app = adding(2);
    typed(&mut app, "MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM");
    enter(&mut app);
    assert!(app.session.keys.is_empty(), "one share of two is not a key");
    typed(&mut app, "MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN");
    enter(&mut app);
    enter(&mut app);
    assert_eq!(
        only_key(&app),
        fingerprint_of(
            "xprv9s21ZrQH143K2NkobdHxXeyFDqE44nJYvzLFtsriatJNWMNKznGoGgW5UMTL4fyWtajnMYb5gEc2CgaKhmsKeskoi9eTimpRv2N11THhPTU"
        )
    );
}

#[test]
fn seed_xor_parts_combine_into_the_seeds_words() {
    // tools/vectors/xor/12-words-2-parts.txt.
    let mut app = adding(3);
    typed(
        &mut app,
        "float vendor rabbit upgrade alter globe right toddler window roof black rule",
    );
    enter(&mut app);
    enter(&mut app);
    assert!(app.session.keys.is_empty(), "one part is not a split");
    typed(
        &mut app,
        "icon weasel expose grace stand spike plastic high runway betray prevent axis",
    );
    enter(&mut app);
    enter(&mut app);
    let mut probe = faraday_core::wallet::Session::default();
    let want = probe
        .add_words_with(
            "city apple they mesh stadium raw cabin material ecology put royal public",
            "",
            "",
            None,
        )
        .unwrap();
    assert_eq!(only_key(&app), want.0);
}

#[test]
fn a_codex32_string_and_a_slip39_share_can_be_scanned_instead_of_typed() {
    // BIP 93 vector 1, as a QR code holds it.
    let mut app = adding(2);
    app.press(Action::ScanPart);
    assert!(app.scan.as_ref().is_some_and(|s| s.part));
    app.event(Event::Scanned {
        bytes: b"MS10TESTSXXXXXXXXXXXXXXXXXXXXXXXXXX4NZVCA9CMCZLW".to_vec(),
    });
    assert!(app.scan.is_none(), "the camera stayed open");
    assert_eq!(app.session.keys.len(), 1, "{:?}", app.entry.error);

    // One SLIP-39 share scanned: in, waiting for the second.
    let mut app = adding(1);
    app.press(Action::ScanPart);
    app.event(Event::Scanned {
        bytes: b"shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed".to_vec(),
    });
    assert_eq!(
        app.entry
            .parts
            .lines(faraday_core::forms::Form::Slip39)
            .len(),
        1
    );
    // Something that is no share says so, and the camera stays open.
    app.press(Action::ScanPart);
    app.event(Event::Scanned {
        bytes: b"hello".to_vec(),
    });
    assert!(app.scan.as_ref().is_some_and(|s| s.note.is_some()));
}
