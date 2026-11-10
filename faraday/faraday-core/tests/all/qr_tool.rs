//! The QR vault tool in Tools (`docs/VAULT.md` §13): Make seals a typed
//! note or a loaded key's words, the code goes to the Outbox as the
//! `.pgp` file, and leaving the tool or a lock wipes what was typed.

use faraday_core::qr_tool::{self, Pick, QrToolAction as Q};
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_keys::{Keys, QrHolds, Reveal};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

const WORDS12: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn started() -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app
}

fn open(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "QR vault")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Types the passphrase twice and the name.
fn passphrase(app: &mut Faraday, pass: &str, name: &str) {
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    type_in(app, pass);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Again)));
    type_in(app, pass);
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Name)));
    type_in(app, name);
}

/// A note typed and sealed under `pass`, named `name`.
fn make_note(app: &mut Faraday, note: &str, pass: &str, name: &str) {
    open(app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(app, note);
    app.press(Action::QrTool(Q::Next));
    passphrase(app, pass, name);
    app.press(Action::QrTool(Q::Make));
    testkit::settle(app);
}

#[test]
fn the_tile_opens_the_tool() {
    let mut app = started();
    open(&mut app);
    assert_eq!(app.screen, Screen::QrTool);
    assert!(qr_tool::state(&app).is_some());
}

#[test]
fn a_note_typed_and_sealed_opens_back_with_the_same_text() {
    let mut app = started();
    make_note(&mut app, "meet at the old oak", "correct horse", "");
    assert!(qr_tool::state(&app).unwrap().is_made());
    app.press(Action::QrTool(Q::File));
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .expect("the code is in the Outbox");
    let c = testkit::compute();
    let mut keys = Keys::default();
    let holds = keys.qr_open(&c, "correct horse", &item.bytes).unwrap();
    assert_eq!(holds, QrHolds::Note);
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "meet at the old oak");
}

#[test]
fn a_named_code_goes_to_the_outbox_under_its_name() {
    let mut app = started();
    make_note(&mut app, "a note", "pass", "Spare");
    app.press(Action::QrTool(Q::File));
    let stem = qr_tool::state(&app).unwrap().stem();
    assert_ne!(stem, "qr-vault");
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == format!("{stem}.pgp"))
        .unwrap();
    assert_eq!(item.bytes.len(), 396);
}

#[test]
fn the_room_left_is_counted_as_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    let empty = qr_tool::state(&app).unwrap().room_left();
    type_in(&mut app, "hello");
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.note_chars(), 5);
    assert_eq!(s.room_left(), empty - 5);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(qr_tool::state(&app).unwrap().room_left(), empty - 4);
}

#[test]
fn typing_stops_at_the_limit() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    let room = qr_tool::state(&app).unwrap().room_left();
    type_in(&mut app, &"a".repeat(room + 20));
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.room_left(), 0);
    assert_eq!(s.note_chars(), room);
}

#[test]
fn a_key_not_of_12_or_24_words_cannot_be_picked() {
    let mut app = started();
    let c = testkit::compute();
    let fifteen = format!("{} address", vec!["abandon"; 14].join(" "));
    app.session
        .keys
        .add_words(&c, &fifteen, "Odd", None)
        .unwrap();
    app.session
        .keys
        .add_words(&c, WORDS12, "Twelve", None)
        .unwrap();
    assert!(!app.session.keys.qr_key_offered(0));
    assert!(app.session.keys.qr_key_offered(1));
    open(&mut app);
    app.press(Action::QrTool(Q::Key(0)));
    let s = qr_tool::state(&app).unwrap();
    assert_eq!(s.key(), None);
    app.press(Action::QrTool(Q::Key(1)));
    assert_eq!(qr_tool::state(&app).unwrap().key(), Some(1));
}

#[test]
fn a_loaded_key_is_sealed_and_opens_back_as_a_seed() {
    let mut app = started();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(&c, WORDS12, "Twelve", None)
        .unwrap();
    open(&mut app);
    app.press(Action::QrTool(Q::Key(0)));
    passphrase(&mut app, "a phrase", "");
    app.press(Action::QrTool(Q::Make));
    testkit::settle(&mut app);
    app.press(Action::QrTool(Q::File));
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .unwrap();
    let mut keys = Keys::default();
    let holds = keys.qr_open(&c, "a phrase", &item.bytes).unwrap();
    assert_eq!(holds, QrHolds::Seed { words: 12 });
}

#[test]
fn the_passphrase_must_match_and_not_be_empty() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(&mut app, "note");
    app.press(Action::QrTool(Q::Next));
    assert!(!qr_tool::state(&app).unwrap().passphrase_ready());
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Pass)));
    type_in(&mut app, "one");
    app.press(Action::QrTool(Q::Focus(qr_tool::Field::Again)));
    type_in(&mut app, "two");
    assert!(!qr_tool::state(&app).unwrap().passphrase_ready());
    app.press(Action::QrTool(Q::Make));
    testkit::settle(&mut app);
    assert!(!qr_tool::state(&app).unwrap().is_made());
    for _ in 0..3 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(&mut app, "one");
    assert!(qr_tool::state(&app).unwrap().passphrase_ready());
}

#[test]
fn the_files_put_in_the_outbox_are_public() {
    let mut app = started();
    make_note(&mut app, "a note", "pass", "");
    // A secret reaching the Outbox trips put_outbox's debug assert.
    app.press(Action::QrTool(Q::File));
    app.press(Action::QrTool(Q::Png));
    assert!(
        app.outbox
            .iter()
            .any(|i| i.name == "qr-vault.pgp" && !i.secret)
    );
    assert!(app.outbox.iter().any(|i| i.name.ends_with(".png")));
}

#[test]
fn make_another_empties_the_flow() {
    let mut app = started();
    make_note(&mut app, "a note", "pass", "Name");
    app.press(Action::QrTool(Q::Another));
    let s = qr_tool::state(&app).unwrap();
    assert!(!s.is_made());
    assert_eq!(s.pick(), None);
    assert_eq!(s.note_chars(), 0);
    assert!(!s.passphrase_ready());
    assert_eq!(s.stem(), "qr-vault");
}

#[test]
fn leaving_wipes_what_was_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(&mut app, "a note");
    app.press(Action::Nav(Screen::Home));
    assert!(qr_tool::state(&app).is_none());
    open(&mut app);
    assert_eq!(qr_tool::state(&app).unwrap().note_chars(), 0);
}

#[test]
fn a_lock_wipes_what_was_typed() {
    let mut app = started();
    open(&mut app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(&mut app, "a note");
    app.press(Action::Lock);
    assert!(qr_tool::state(&app).is_none());
}

/// A note typed and its passphrase typed, the seal not yet pressed.
fn ready_to_make(app: &mut Faraday) {
    open(app);
    app.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_in(app, "meet at the old oak");
    app.press(Action::QrTool(Q::Next));
    passphrase(app, "correct horse", "");
}

#[test]
fn a_note_sealed_on_the_worker_shows_progress_then_opens_back() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    assert!(app.busy().is_some(), "the screen shows Progress");
    assert!(!qr_tool::state(&app).unwrap().is_made());
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert!(qr_tool::state(&app).unwrap().is_made());
    app.press(Action::QrTool(Q::File));
    let item = app
        .outbox
        .iter()
        .find(|i| i.name == "qr-vault.pgp")
        .expect("the code is in the Outbox");
    let c = testkit::compute();
    let mut keys = Keys::default();
    assert_eq!(
        keys.qr_open(&c, "correct horse", &item.bytes),
        Ok(QrHolds::Note)
    );
    assert_eq!(&*keys.qr_note(Reveal::QrVault), "meet at the old oak");
}

#[test]
fn back_while_the_code_is_sealed_makes_no_code_and_keeps_nothing_typed() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Back);
    assert!(app.busy().is_none());
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    if let Some(s) = qr_tool::state(&app) {
        assert!(!s.is_made(), "no code was made");
        assert!(!s.passphrase_ready());
        assert_eq!(s.note_chars(), 0);
    }
    assert!(app.outbox.is_empty());
}

#[test]
fn a_lock_while_the_code_is_sealed_makes_nothing_and_keeps_nothing_typed() {
    let mut app = started();
    ready_to_make(&mut app);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::QrTool(Q::Make));
    app.press(Action::Lock);
    assert!(app.busy().is_none());
    assert!(qr_tool::state(&app).is_none(), "nothing typed is kept");
    testkit::release_worker();
    testkit::settle_stale(&mut app);
    assert!(qr_tool::state(&app).is_none());
    assert!(app.outbox.is_empty());
}
