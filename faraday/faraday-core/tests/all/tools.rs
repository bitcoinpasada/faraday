//! The calculators on Faraday's Tools screen: what is typed
//! reaches the calculator, and the answers are the published ones.

use crate::common::{has, through_fade};
use faraday_bip::hashes::hashes;
use faraday_bip::slip132::{KeyReading, key_facts};
use faraday_codec::encodings::read_input;
use faraday_core::strings::EN;
use faraday_core::testkit::Plain;
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
    sized(1280, 800)
}

/// The app on a `width` × `height` window.
fn sized(width: u16, height: u16) -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
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
        .plain()
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

const KEY_A: &str = "tpubDFZk17LmJpm9L6gT3oK6r3TWNni6fVLrkZLCsH2psqckyuuZ5239eePW97oLgikCUJ5zQX4MVVKDp9Yj9DC9wtYyGXkPwLuGeGN78U3ykR2";
const KEY_B: &str = "tpubDFcsPThm5auRE692Mr7j53V9NY9Lt3XKuUP6fgJjCbDmECdXthX6jv7Yfy5qZAAxFKndTSS22t6RrxaDTvKTV9beG354SxmaXrvEQVJeBsB";

#[test]
fn the_descriptor_tool_says_which_key_is_used_twice() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Descriptor);
    typed(
        &mut app,
        &format!(
            "wsh(sortedmulti(1,[9a6a2580/48h/1h/0h/3h]{KEY_A}/<0;1>/*,[5d388376/48h/1h/0h/3h]{KEY_B}/<0;1>/*,[00000000/84h/1h/0h]{KEY_A}/<0;1>/*))"
        ),
    );
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Keys"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == "Key 3 is the same as key 1"),
        "{texts:?}"
    );
    app.press(Action::TClear);
    typed(
        &mut app,
        &format!(
            "wsh(sortedmulti(1,[9a6a2580/48h/1h/0h/3h]{KEY_A}/<0;1>/*,[5d388376/48h/1h/0h/3h]{KEY_B}/<0;1>/*))"
        ),
    );
    assert!(!app.drawn_texts().iter().any(|t| t == "Keys"));
}

#[test]
fn a_descriptor_this_device_loads_is_taken_to_wallets_after_confirmation() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Descriptor);
    typed(
        &mut app,
        &format!(
            "wsh(sortedmulti(1,[9a6a2580/48h/1h/0h/3h]{KEY_A}/<0;1>/*,[5d388376/48h/1h/0h/3h]{KEY_B}/<0;1>/*))"
        ),
    );
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Take to Wallets"), "{texts:?}");
    assert!(app.offers(Action::TTake));
    app.press(Action::TTake);
    assert_eq!(app.screen, Screen::Tools, "the sheet asks first");
    testkit::confirm_wallets(&mut app);
    assert_eq!(app.screen, Screen::Wallets);
    assert!(app.drawn_texts().iter().any(|t| t == "Wallet"), "named");
    // What is not loadable offers no way on.
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Descriptor);
    typed(&mut app, "wpkh(02)");
    assert!(!app.drawn_texts().iter().any(|t| t == "Take to Wallets"));
}

const XPUB: &str = "xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V";
const DESCRIPTOR: &str = "wpkh(02c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5)";
const SEED: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn scan_note(app: &Faraday) -> String {
    app.scan()
        .and_then(|s| s.note.as_ref().map(|n| String::from(n.as_str())))
        .unwrap_or_default()
}

fn scanned(app: &mut Faraday, text: &str) {
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

#[test]
fn a_scanned_xpub_fills_the_field_and_is_converted() {
    let mut app = Faraday::new();
    app.press(Action::Tools);
    tool(&mut app, Tool::ConvertKey);
    app.press(Action::TScan);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));
    scanned(&mut app, XPUB);
    assert_eq!(app.sheet, None, "the scanner closes");
    let t = testkit::tools(&app).unwrap();
    assert_eq!(t.typed, XPUB);
    let KeyReading::Public(f) = key_facts(&t.typed, app.session.network()) else {
        panic!("not read as a public key");
    };
    assert!(!f.slip132.is_empty());
}

#[test]
fn a_scanned_descriptor_and_policy_fill_their_fields() {
    for t in [Tool::Descriptor, Tool::Miniscript] {
        let mut app = Faraday::new();
        app.press(Action::Tools);
        tool(&mut app, t);
        app.press(Action::TScan);
        scanned(&mut app, DESCRIPTOR);
        assert_eq!(app.sheet, None);
        assert_eq!(testkit::tools(&app).unwrap().typed, DESCRIPTOR);
    }
}

#[test]
fn a_file_fills_the_field_as_typed_text_does() {
    let mut app = Faraday::new();
    app.press(Action::Tools);
    tool(&mut app, Tool::ConvertKey);
    testkit::inbox_file(&mut app, "key.txt", format!("{XPUB}\n").into_bytes());
    let _ = app.frame();
    app.press(Action::TFile(0));
    let t = testkit::tools(&app).unwrap();
    assert_eq!(t.typed, XPUB);
    assert!(matches!(
        key_facts(&t.typed, app.session.network()),
        KeyReading::Public(_)
    ));
}

#[test]
fn a_seed_or_another_code_does_not_fill_the_field() {
    for text in [
        SEED,
        "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn",
        "otpauth://totp/Scanned:me?secret=JBSWY3DPEHPK3PXP",
    ] {
        for t in [Tool::ConvertKey, Tool::Descriptor, Tool::Miniscript] {
            let mut app = Faraday::new();
            app.press(Action::Tools);
            tool(&mut app, t);
            app.press(Action::TScan);
            scanned(&mut app, text);
            assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan), "{text}");
            assert!(
                scan_note(&app).contains("not used here"),
                "{}",
                scan_note(&app)
            );
            assert!(testkit::tools(&app).unwrap().typed.is_empty(), "{text}");
        }
    }
}

#[test]
fn a_file_with_a_seed_does_not_fill_the_field() {
    let mut app = Faraday::new();
    app.press(Action::Tools);
    tool(&mut app, Tool::ConvertKey);
    testkit::inbox_file(&mut app, "words.txt", SEED.as_bytes().to_vec());
    let _ = app.frame();
    app.press(Action::TFile(0));
    assert!(testkit::tools(&app).unwrap().typed.is_empty());
}

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

/// Hashes on a wide window: the field, Next, then the same hashes the
/// old page showed.
#[test]
fn hashes_on_a_wide_window_gives_the_same_hashes_on_its_result_card() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Hashes);
    typed(&mut app, "abc");
    assert!(has(&mut app, EN.tools_next));
    app.press(Action::TStep(1));
    through_fade(&mut app);
    assert!(has(&mut app, ABC_SHA256));
}

/// Your choices names what was typed, and Back opens the field again with
/// the text kept.
#[test]
fn hashes_card_names_what_was_typed_and_back_keeps_it() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Hashes);
    typed(&mut app, "abc");
    app.press(Action::TStep(1));
    through_fade(&mut app);
    assert!(has(&mut app, "abc"));
    app.press(Action::Back);
    through_fade(&mut app);
    assert_eq!(app.screen, Screen::Tools);
    assert_eq!(testkit::tools(&app).unwrap().typed, "abc");
    // The result's card takes no typing; the field does.
    app.press(Action::TStep(1));
    through_fade(&mut app);
    typed(&mut app, "d");
    assert_eq!(testkit::tools(&app).unwrap().typed, "abc");
    app.press(Action::TStep(0));
    through_fade(&mut app);
    typed(&mut app, "d");
    assert_eq!(testkit::tools(&app).unwrap().typed, "abcd");
}

/// Nothing typed: Next opens nothing.
#[test]
fn hashes_next_waits_for_something_typed() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Hashes);
    app.press(Action::TStep(1));
    through_fade(&mut app);
    assert!(!has(&mut app, "SHA-256"));
}

/// A phone's window draws the page as before: no card, no Next.
#[test]
fn hashes_on_a_phone_is_still_the_page() {
    let mut app = sized(480, 640);
    app.press(Action::Tools);
    tool(&mut app, Tool::Hashes);
    typed(&mut app, "abc");
    assert!(!has(&mut app, EN.tools_next));
    assert!(has(&mut app, ABC_SHA256));
}

/// Your choices lists what was typed and not the result as a choice not
/// made.
#[test]
fn hashes_choices_has_no_result_row_that_reads_not_chosen() {
    let mut app = wide();
    app.press(Action::Tools);
    tool(&mut app, Tool::Hashes);
    typed(&mut app, "abc");
    app.press(Action::TStep(1));
    through_fade(&mut app);
    assert!(has(&mut app, EN.tools_card_input));
    assert!(has(&mut app, "abc"));
    assert!(!has(&mut app, EN.flow_not_chosen_yet));
}
