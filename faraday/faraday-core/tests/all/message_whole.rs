//! Sign a message shows the whole message: the field is as high as the
//! text, the Sign step shows it too, and what cannot be seen in a text
//! is drawn as marks. A message that ends in a newline is signed and
//! checked as the same message from the file Faraday makes for it.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, testkit};
use faraday_shell_api::{App, Event};

fn app() -> Faraday {
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
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.session
        .add_wallet(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app
}

/// A message whose last line comes after ten blank ones.
fn long() -> String {
    format!("first line{}the last line", "\n".repeat(11))
}

fn scan(app: &mut Faraday, text: &str) {
    app.press(Action::SignMessage);
    app.press(Action::MScan);
    app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    let _ = app.frame();
}

/// Opens a step and lets it finish opening, as a person sees it after a
/// moment.
fn open_step(app: &mut Faraday, step: u8) {
    app.press(Action::MStep(step));
    let _ = app.frame();
    for t in 1..=20u64 {
        app.event(Event::Tick {
            now_ms: 60_000 + t * 100,
        });
        let _ = app.frame();
    }
}

fn box_of(app: &mut Faraday, label: &str) -> (i32, i32, i32, i32) {
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == label)
        .unwrap_or_else(|| panic!("{label:?} is drawn"))
        .1
}

#[test]
fn a_scanned_message_is_all_in_its_field_before_signing() {
    let mut app = app();
    scan(&mut app, &long());
    open_step(&mut app, faraday_core::mstep::TEXT);
    let last = box_of(&mut app, "the last line");
    let below = box_of(&mut app, EN.message_scan);
    assert!(
        last.1 + last.3 <= below.1,
        "the last line {last:?} ends above the buttons under the field {below:?}"
    );
}

#[test]
fn the_sign_step_shows_the_whole_message() {
    let mut app = app();
    scan(&mut app, &long());
    open_step(&mut app, faraday_core::mstep::SIGN);
    assert!(app.drawn_texts().iter().any(|t| t == "the last line"));
    app.press(Action::MSign);
    let _ = app.frame();
    assert!(app.drawn_texts().iter().any(|t| t == "the last line"));
}

#[test]
fn control_and_zero_width_characters_are_drawn_as_marks() {
    let mut app = app();
    // Typed or read from a file: a scanner takes plain text only.
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "pay\u{200B}me\u{1B}[0m now\r\nok");
    open_step(&mut app, faraday_core::mstep::SIGN);
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "pay\u{25CC}me\u{25CC}[0m now\u{00B6}"),
        "{texts:?}"
    );
}

#[test]
fn a_message_ending_in_a_newline_is_checked_valid_from_its_file() {
    let mut app = app();
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello\n");
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.error, None);
    let sig = m.signed.as_ref().expect("signed");
    let file = faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
    let check = app
        .session
        .check_message(&testkit::compute(), &file)
        .expect("the file reads");
    assert_eq!(check.signed.message.as_str(), "hello\n");
    assert!(check.verdict.is_ok(), "{:?}", check.verdict);
}

/// Whether a person sees `c` drawn as itself in every text face: its own
/// glyph, not the `?` a character without one is drawn as.
fn has_own_glyph(c: char) -> bool {
    use faraday_draw::fonts::{Family, outlines};
    [Family::Regular, Family::SemiBold, Family::Mono]
        .into_iter()
        .all(|f| outlines(f).glyph(c).is_some())
}

#[test]
fn a_zero_width_space_is_drawn_as_a_mark_that_has_its_own_glyph() {
    let mut app = app();
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "pay\u{200B}me");
    open_step(&mut app, faraday_core::mstep::SIGN);
    let texts = app.drawn_texts();
    let line = texts
        .iter()
        .find(|t| t.starts_with("pay") && t.ends_with("me"))
        .unwrap_or_else(|| panic!("{texts:?}"));
    assert_ne!(&**line, "payme");
    for c in line.chars() {
        assert!(has_own_glyph(c), "{c:?} is drawn as ?");
    }
}

#[test]
fn every_character_that_shows_nothing_is_drawn_as_a_mark() {
    let mut app = app();
    app.press(Action::SignMessage);
    let hidden =
        "\u{61C}\u{115F}\u{3164}\u{FFA0}\u{FE0F}\u{E0100}\u{E0041}\u{180B}\u{FFFA}\u{1D173}\u{600}";
    testkit::message_text(&mut app, &format!("a{hidden}b"));
    open_step(&mut app, faraday_core::mstep::SIGN);
    let texts = app.drawn_texts();
    let marks = format!("a{}b", "\u{25CC}".repeat(hidden.chars().count()));
    assert!(texts.iter().any(|t| **t == *marks), "{texts:?}");
}

#[test]
fn the_text_card_summary_shows_the_first_line_that_has_text() {
    let mut app = app();
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "\nhello");
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "hello"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == EN.message_not_written));
}

/// Goes back to Home, as a person leaves Sign a message.
fn home(app: &mut faraday_core::Faraday) {
    for _ in 0..6 {
        if app.screen == faraday_core::Screen::Home {
            return;
        }
        app.press(Action::Back);
    }
    assert_eq!(app.screen, faraday_core::Screen::Home);
}

/// The message `text` signed in Faraday, its signed form as a QR scanned
/// on Home: the Check a message screen's verdict.
fn scanned_back(text: &str) -> faraday_core::Faraday {
    let mut app = app();
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, text);
    app.press(Action::MSign);
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.error, None);
    let sig = m.signed.as_ref().expect("signed");
    let code = faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
    home(&mut app);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: code.as_bytes().to_vec(),
    });
    app.press(Action::ReadGo(0));
    assert_eq!(app.screen, faraday_core::Screen::CheckMessage);
    app
}

fn check_of(app: &faraday_core::Faraday, shows: &str) -> faraday_core::wallet::MessageCheck {
    let item = app
        .inbox()
        .iter()
        .find(|i| i.kind == faraday_core::wallet::FileKind::Message)
        .expect("the message is in Files");
    let check = app
        .session
        .check_message(&testkit::compute(), &String::from_utf8_lossy(&item.bytes))
        .expect("the file reads");
    assert_eq!(check.signed.message.as_str(), shows);
    check
}

#[test]
fn a_message_ending_in_a_newline_checks_valid_from_its_scanned_qr() {
    for text in ["hello\n", "hello ", "hello\n\n", "hello"] {
        let app = scanned_back(text);
        let check = check_of(&app, text);
        assert!(check.verdict.is_ok(), "{text:?}: {:?}", check.verdict);
    }
}

#[test]
fn a_different_message_checks_invalid_from_a_file_and_from_a_qr() {
    let mut app = app();
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello\n");
    app.press(Action::MSign);
    let sig = testkit::message(&app).unwrap().signed.clone().unwrap();
    // The signature of `hello\n`, said of `hello`.
    let other = faraday_keys::message::signed_text(&sig.address, &sig.signature, "hello");
    let check = app
        .session
        .check_message(&testkit::compute(), &other)
        .expect("the file reads");
    assert!(check.verdict.is_err());
    home(&mut app);
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: other.as_bytes().to_vec(),
    });
    app.press(Action::ReadGo(0));
    let check = check_of(&app, "hello");
    assert!(check.verdict.is_err());
}

#[test]
fn a_signed_message_pasted_on_homes_scan_checks_valid() {
    let mut app = app();
    app.clipboard = true;
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello\n");
    app.press(Action::MSign);
    let sig = testkit::message(&app).unwrap().signed.clone().unwrap();
    let text = faraday_keys::message::signed_text(&sig.address, &sig.signature, "hello\n");
    home(&mut app);
    app.press(Action::Scan);
    app.press(Action::ScanPaste);
    while app.poll_command().is_some() {}
    app.event(Event::Clipboard {
        kind: faraday_shell_api::FileKind::Text,
        text: faraday_text::Text::of(&text),
    });
    let _ = app.frame();
    app.press(Action::ReadGo(0));
    assert_eq!(app.screen, faraday_core::Screen::CheckMessage);
    let check = check_of(&app, "hello\n");
    assert!(check.verdict.is_ok(), "{:?}", check.verdict);
}

#[test]
fn a_changed_message_is_drawn_again_after_a_hidden_character() {
    let mut app = app();
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "pay\u{200B}me");
    open_step(&mut app, faraday_core::mstep::SIGN);
    assert!(app.drawn_texts().iter().any(|t| t == "pay\u{25CC}me"));
    testkit::message_text(&mut app, "first\u{200B}\nsecond\nthird");
    let _ = app.frame();
    let texts = app.drawn_texts();
    for line in ["first\u{25CC}", "second", "third"] {
        assert!(texts.iter().any(|t| t == line), "{line:?} in {texts:?}");
    }
    assert!(!texts.iter().any(|t| t == "pay\u{25CC}me"));
}
