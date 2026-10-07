//! OpenSigner's calculators on Faraday's Tools screen: what is typed
//! reaches the calculator, and the answers are the published ones.

use faraday_core::{Action, Faraday, Screen};
use opensigner_core::tools::{KeyReading, Tool, hashes, key_facts, read_input};
use osk_shell_api::{App, Event, Key};

fn tool(app: &mut Faraday, t: Tool) {
    let i = Tool::ALL.iter().position(|x| *x == t).unwrap();
    app.press(Action::TTool(i as u8));
}

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn what_is_typed_reaches_the_calculator() {
    let mut app = Faraday::new();
    app.press(Action::Tools);
    assert_eq!(app.screen, Screen::Tools);
    tool(&mut app, Tool::Hashes);
    typed(&mut app, "abc");
    let t = app.tools.as_ref().unwrap();
    let h = hashes(&read_input(&t.typed, t.read_as));
    let hex: String = h.sha256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // A new tool starts with an empty field.
    tool(&mut app, Tool::ConvertKey);
    assert!(app.tools.as_ref().unwrap().typed.is_empty());
    typed(
        &mut app,
        "xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V",
    );
    let t = app.tools.as_ref().unwrap();
    let KeyReading::Public(f) = key_facts(&t.typed, app.session.network()) else {
        panic!("not read as a public key");
    };
    // BIP-84's zpub for "abandon … about".
    assert!(f.slip132.iter().any(|(_, k)| k
        == "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs"));
}

#[test]
fn the_units_field_takes_digits_only() {
    let mut app = Faraday::new();
    app.press(Action::Tools);
    tool(&mut app, Tool::Units);
    typed(&mut app, "12a3");
    assert_eq!(app.tools.as_ref().unwrap().typed, "123");
}
