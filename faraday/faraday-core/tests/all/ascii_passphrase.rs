//! A BIP-39 passphrase here is printable ASCII (`docs/STANDARDS.md`): a
//! letter outside it typed into Add a key's passphrase field or a
//! SeedQR's is not taken, and the field says it is refused, naming no
//! letter of what is masked. Create's, New key's and Lightning's fields
//! are tested beside their flows.

use crate::common::type_text;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::seeds::read::Choice;
use faraday_core::{Action, Faraday, Sheet, testkit};
use faraday_shell_api::{App, Event};

pub const REFUSED: &str = "A passphrase here is printable ASCII only";

fn drawn(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

#[test]
fn add_a_key_refuses_a_letter_outside_ascii_and_says_so() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words("bacon"));
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "café");
    assert_eq!(&*testkit::entry(&app).passphrase, "caf");
    let texts = drawn(&mut app);
    assert!(texts.iter().any(|t| t == REFUSED), "{texts:?}");
    // The next letter taken clears it.
    type_text(&mut app, "e");
    assert_eq!(&*testkit::entry(&app).passphrase, "cafe");
    let texts = drawn(&mut app);
    assert!(!texts.iter().any(|t| t == REFUSED), "{texts:?}");
}

#[test]
fn a_seedqrs_passphrase_refuses_a_letter_outside_ascii_and_says_so() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.press(Action::ScanSeed);
    let m = Mnemonic::parse(&c, Language::English, &testkit::test_words("bacon")).unwrap();
    app.event(Event::Scanned {
        bytes: faraday_codec::seedqr::to_digits(&m)
            .expose()
            .as_bytes()
            .to_vec(),
    });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(Choice::AddPass));
    app.press(Action::SeedRead(Choice::Field(0)));
    type_text(&mut app, "café");
    let texts = drawn(&mut app);
    assert!(texts.iter().any(|t| t == REFUSED), "{texts:?}");
}
