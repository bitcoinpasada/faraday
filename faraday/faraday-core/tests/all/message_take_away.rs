//! The end of Sign a message, where the pair stands: the take-away is the
//! pane at the left in place of Your choices, a tick row for the file and
//! one button at its foot (off with nothing ticked, "Waiting for a stick"
//! with no stick, "Written and checked" after the write). Other sizes keep
//! the card as it was.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn signed_at(width: u16, height: u16) -> Faraday {
    let c = testkit::compute();
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
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    app.press(Action::SignMessage);
    testkit::message_text(&mut app, "hello");
    app.press(Action::MSign);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    let _ = app.frame();
    app
}

fn stick_in(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("one"),
        label: faraday_text::Text::of("FIRST"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    // Keys are in memory after the signing: the stick's arrival asks to
    // lock first, and the person leaves that for now.
    app.press(Action::NotNow);
    let _ = app.frame();
}

fn shown(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_text_boxes().iter().any(|(t, _)| t == text)
}

fn file(app: &Faraday) -> String {
    app.outbox.first().expect("the file waits").name.to_string()
}

fn write_button() -> String {
    format!("Write to {}", EN.medium_stick_inline)
}

#[test]
fn the_pane_holds_the_take_away_at_the_last_step_on_both_wide_sizes() {
    for (w, h) in [(1366, 768), (1920, 1080)] {
        let mut app = signed_at(w, h);
        let name = file(&app);
        assert!(shown(&mut app, EN.take_away_head), "{w}x{h}");
        assert!(shown(&mut app, &name), "{w}x{h}");
        assert!(!shown(&mut app, EN.flow_your_choices), "{w}x{h}");
        assert!(app.offers(Action::MQr));
        // The card holds neither the file nor the code buttons: Show as QR
        // is offered once, and the PNG not at all.
        assert_eq!(app.offered_times(Action::MQr), 1, "{w}x{h}");
        assert!(!app.offers(Action::CodePng(faraday_core::Code::Message)));
        assert_eq!(
            app.drawn_text_boxes()
                .iter()
                .filter(|(t, _)| **t == *name)
                .count(),
            1,
            "the file is drawn once, in the pane"
        );
    }
}

#[test]
fn no_sheet_opens_at_the_end() {
    let app = signed_at(1366, 768);
    assert_eq!(app.sheet, None::<Sheet>);
}

#[test]
fn a_tick_is_the_visit_chosen_set_and_the_button_follows_it() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    let name = file(&app);
    assert!(app.visit().out.contains(&*name), "ticked when made");
    assert!(app.offers(Action::VisitWrite));
    assert!(shown(&mut app, &write_button()));
    app.press(Action::VisitOut(0));
    let _ = app.frame();
    assert!(!app.visit().out.contains(&*name));
    assert!(!app.offers(Action::VisitWrite), "off with nothing ticked");
    assert!(shown(&mut app, &write_button()));
    app.press(Action::VisitOut(0));
    let _ = app.frame();
    assert!(app.offers(Action::VisitWrite));
}

#[test]
fn with_no_stick_the_button_reads_waiting_for_a_stick() {
    let mut app = signed_at(1366, 768);
    let waiting = format!("Waiting for {}", "a stick");
    assert!(shown(&mut app, &waiting));
    let n = app
        .drawn_text_boxes()
        .iter()
        .filter(|(t, _)| **t == *waiting)
        .count();
    assert_eq!(n, 1, "said once, on the button");
    assert!(!app.offers(Action::VisitWrite));
    assert!(app.outbox.len() == 1, "the file waits in the outbox");
}

#[test]
fn with_keys_in_memory_the_button_asks_to_lock_before_it_writes() {
    let mut app = signed_at(1366, 768);
    stick_in(&mut app);
    assert!(app.offers(Action::VisitWrite));
    app.press(Action::VisitWrite);
    assert_eq!(app.sheet, Some(Sheet::WriteOut));
    assert!(app.outbox.len() == 1, "the file still waits");
}

#[test]
fn back_to_the_step_before_brings_your_choices_back() {
    let mut app = signed_at(1366, 768);
    assert!(shown(&mut app, EN.take_away_head));
    app.press(Action::MStep(faraday_core::mstep::FORMAT));
    assert!(shown(&mut app, EN.flow_your_choices));
    assert!(!shown(&mut app, EN.take_away_head));
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.take_away_head));
}

fn sha_of(app: &Faraday) -> [u8; 32] {
    faraday_crypto::sha256_whole(&testkit::compute(), &app.outbox[0].bytes)
}

#[test]
fn written_and_checked_holds_for_the_same_name_and_the_same_bytes_only() {
    // Another message under the same name: a receipt of the same name and
    // other bytes does not make this file written.
    let mut app = signed_at(1366, 768);
    let name = file(&app);
    app.outbox.clear();
    testkit::receipt_written(&mut app, &name, [7; 32]);
    assert!(!shown(&mut app, EN.pull_line_written_checked));
    assert!(shown(&mut app, EN.message_make_the_file) || shown(&mut app, EN.made_state_make_again));
    // The right bytes: written and checked.
    let mut app = signed_at(1366, 768);
    let (name, sha) = (file(&app), sha_of(&app));
    app.outbox.clear();
    testkit::receipt_written(&mut app, &name, sha);
    assert!(shown(&mut app, EN.pull_line_written_checked));
    assert!(!shown(&mut app, EN.message_make_the_file));
    assert!(!shown(&mut app, EN.made_state_make_again));
}

#[test]
fn a_file_removed_in_files_is_offered_to_make_again_and_made() {
    let mut app = signed_at(1366, 768);
    let name = file(&app);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::OutboxRemove(0));
    assert!(app.outbox.is_empty());
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.take_away_head));
    assert!(shown(&mut app, EN.message_make_the_file));
    assert!(app.offers(Action::MOut));
    app.press(Action::MOut);
    assert_eq!(file(&app), name, "made again under its name");
    assert!(app.visit().out.contains(&*name), "ticked");
    assert!(!shown(&mut app, EN.message_make_the_file));
}

/// The first signature's file still waits: the second's, removed, is named
/// as it will be made, beside it.
#[test]
fn a_removed_file_is_offered_under_the_name_it_is_made_under() {
    let mut app = signed_at(1366, 768);
    let first = file(&app);
    testkit::message_text(&mut app, "second");
    app.press(Action::MSign);
    let second = app.outbox[1].name.to_string();
    assert!(second.ends_with("-2.txt"), "{second}");
    app.press(Action::Nav(Screen::Files));
    app.press(Action::OutboxRemove(1));
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.message_make_the_file));
    // The row may end the name in "…"; what it shows is the second's alone.
    let names: Vec<String> = app
        .drawn_text_boxes()
        .iter()
        .map(|(t, _)| t.trim_end_matches('…').to_string())
        .filter(|t| t.starts_with("message-") && !t.contains(' '))
        .collect();
    assert!(!names.is_empty());
    for name in names {
        assert!(
            second.starts_with(&name) && !first.starts_with(&name),
            "{name}"
        );
    }
    app.press(Action::MOut);
    assert_eq!(app.outbox[1].name.as_str(), second);
}

#[test]
fn a_file_renamed_in_files_is_offered_to_make_again() {
    let mut app = signed_at(1366, 768);
    let name = file(&app);
    // What Rename in Files leaves: the same file under another name.
    app.press(Action::Nav(Screen::Files));
    app.outbox[0].name = faraday_text::Text::of("mine.txt");
    app.press(Action::Back);
    app.press(Action::MStep(faraday_core::mstep::SIGN));
    assert!(shown(&mut app, EN.message_make_the_file));
    assert!(app.offers(Action::MOut));
    assert!(!shown(&mut app, EN.pull_line_written_checked));
    app.press(Action::MOut);
    assert_eq!(app.outbox.len(), 2);
    assert!(app.outbox.iter().any(|i| i.name == *name));
}

#[test]
fn the_smaller_sizes_keep_the_card_with_its_file() {
    // The card's own file row (name and its Remove) at the sizes where the
    // pane does not stand. Every size below the expanded rail is one column,
    // where the card is never given to the pane: none of the sizes tried (800x600 to 1920x1080) reaches the card's
    // `pair_left` gate in the wide layout (1200x800 and up have the pane,
    // 1100x700 and down are one column), so the gate itself is held by the
    // wide-size test above (the file drawn once, in the pane).
    for (w, h) in [(480, 640), (320, 480), (1024, 600)] {
        let mut app = signed_at(w, h);
        let name = file(&app);
        assert!(!shown(&mut app, EN.take_away_head), "{w}x{h}");
        assert!(app.offers(Action::MQr), "{w}x{h}: Show as QR in the card");
        if w >= 480 {
            // At 320 wide the name is cut by the card's row.
            assert!(shown(&mut app, &name), "{w}x{h}: the file in the card");
        }
        assert!(
            app.offers(Action::OutboxRemove(0)),
            "{w}x{h}: its Remove, as before"
        );
        assert!(!shown(&mut app, &write_button()), "{w}x{h}");
    }
}

const SEED_MESSAGE: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

/// A message that is itself a seed has no file: nothing waits, no toast
/// says so, no Make the file is offered; the code still is.
#[test]
fn a_message_that_is_a_seed_makes_no_file_and_offers_its_code_only() {
    for (w, h) in [(1366, 768), (480, 640)] {
        let mut app = signed_at(w, h);
        app.press(Action::OutboxRemove(0));
        assert!(app.outbox.is_empty());
        app.press(Action::MStep(faraday_core::mstep::TEXT));
        testkit::message_text(&mut app, SEED_MESSAGE);
        app.press(Action::MSign);
        assert!(testkit::message(&app).unwrap().signed.is_some());
        assert!(app.outbox.is_empty(), "nothing is written");
        let toast = app.toast_text().unwrap_or_default().to_string();
        assert!(!toast.contains("waits"), "{toast}");
        assert_eq!(toast, EN.secret_text_those_are);
        app.press(Action::MStep(faraday_core::mstep::SIGN));
        assert!(!shown(&mut app, EN.message_make_the_file));
        assert!(!app.offers(Action::MOut));
        assert!(app.offers(Action::MQr), "the code stays offered");
    }
}

/// An ordinary message is filed and says it waits, as before.
#[test]
fn an_ordinary_message_still_waits_as_a_file() {
    let app = signed_at(1366, 768);
    assert_eq!(app.outbox.len(), 1);
    assert!(app.toast_text().is_some_and(|t| t.contains("waits")));
}
