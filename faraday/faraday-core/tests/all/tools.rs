//! The calculators on Faraday's Tools screen: what is typed
//! reaches the calculator, and the answers are the published ones.

use faraday_bip::hashes::hashes;
use faraday_bip::slip132::{KeyReading, key_facts};
use faraday_codec::encodings::read_input;
use faraday_core::strings::EN;
use faraday_core::tools::Tool;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

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
    let t = testkit::tools(&app).unwrap();
    let h = hashes(&read_input(&t.typed, t.read_as).unwrap());
    let hex: String = h.sha256.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // A new tool starts with an empty field.
    tool(&mut app, Tool::ConvertKey);
    assert!(testkit::tools(&app).unwrap().typed.is_empty());
    typed(
        &mut app,
        "xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V",
    );
    let t = testkit::tools(&app).unwrap();
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
    typed(&mut app, "12a3.4");
    assert_eq!(
        testkit::tools(&app).unwrap().typed,
        "1234",
        "sat has no point"
    );
}

/// The app on a 1280 × 800 window.
fn wide() -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

/// Whether the drawn page shows `label` with `value` right after it.
fn row(app: &mut Faraday, label: &str, value: &str) -> bool {
    app.drawn_texts()
        .windows(2)
        .any(|w| w[0] == label && w[1] == value)
}

#[test]
fn an_amount_in_btc_or_mbtc_takes_a_point_down_to_one_satoshi() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Units);
    // BTC: eight places, one point.
    app.press(Action::TMode(1));
    typed(&mut app, "0.5.00000019");
    assert_eq!(testkit::tools(&app).unwrap().typed, "0.50000001");
    assert!(
        row(&mut app, "sat", "50,000,001"),
        "{:?}",
        app.drawn_texts()
    );
    assert!(row(&mut app, "mBTC", "500.00001"));
    // mBTC: five places; the places past them are dropped.
    app.press(Action::TMode(2));
    assert_eq!(testkit::tools(&app).unwrap().typed, "0.50000");
    assert!(row(&mut app, "sat", "50,000"));
    // sat: no point, so the places go.
    app.press(Action::TMode(0));
    assert_eq!(testkit::tools(&app).unwrap().typed, "0");
}

#[test]
fn the_miniscript_compiler_says_what_it_refuses_in_the_apps_words() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Miniscript);
    typed(&mut app, "or(pk(A)");
    let prefix = EN.tools_miniscript_not_policy.trim_end_matches("{}");
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(prefix), "{shown}");
}

#[test]
fn encodings_names_the_encoding() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Encodings);
    // BIP-173's P2WPKH example.
    typed(&mut app, "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4");
    assert!(
        row(&mut app, "Encoding", "bech32"),
        "{:?}",
        app.drawn_texts()
    );
}

/// The top app bar's way back names the page it returns to: Tools for a
/// calculator opened from Tools.
#[test]
fn a_calculator_opened_from_tools_names_tools_as_its_way_back() {
    let mut app = wide();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Tools);
    assert_eq!(app.screen, Screen::Tools);
    // The bar's texts, beside the rail: the page's name and, under it,
    // where its way back goes.
    let bar: Vec<String> = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(_, (x, y, _, h))| *x > 240 && y + h <= 64)
        .map(|(t, _)| t)
        .collect();
    assert_eq!(bar.iter().filter(|t| *t == "Tools").count(), 1, "{bar:?}");
    assert_eq!(
        bar.iter().filter(|t| *t == "Back to Tools").count(),
        1,
        "{bar:?}"
    );
    assert!(!bar.iter().any(|t| t == "Wallets"), "{bar:?}");
    assert!(app.back_link() == Some(Action::Nav(Screen::Catalog)));
}
