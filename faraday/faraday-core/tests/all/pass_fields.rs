//! A passphrase field takes what its standard allows, shows and hides with
//! its eye, takes no typing while a stick is attached, and is empty after
//! its sheet or screen is left and after a lock. Begun with the Inbox's
//! possible wallet and Lightning's field.

use crate::common::type_text;
use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::App;

const REFUSED: &str = "A passphrase here is printable ASCII only";
const STICK_LINE: &str = "Keys load only with no stick attached";

fn drawn(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().plain()
}

fn shows(app: &mut Faraday, text: &str) -> bool {
    drawn(app).iter().any(|t| t == text)
}

/// The Inbox's possible wallet sheet, open on a seed's words.
fn potential_sheet() -> Faraday {
    let mut app = testkit::started();
    let words = testkit::test_words("summer");
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            faraday_text::Text::of("summer-words.txt"),
            words.as_bytes().to_vec().into(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let c = testkit::compute();
    let fp = app.inbox_found_now(&c).potential()[0].fingerprint;
    app.press(Action::PotentialOpen(fp.0));
    assert_eq!(app.sheet, Some(Sheet::Potential));
    app.press(Action::PotentialType);
    app
}

/// Lightning's passphrase field, focused.
fn lightning_field() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Lightning);
    assert_eq!(app.screen, Screen::Lightning);
    app.press(Action::LAezeed);
    app.press(Action::LPassphrase);
    app
}

fn attach(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    if app.sheet == Some(Sheet::Lock) {
        app.press(Action::NotNow);
    }
}

#[test]
fn the_inbox_passphrase_refuses_a_letter_outside_ascii_and_says_so() {
    let mut app = potential_sheet();
    type_text(&mut app, "café");
    assert!(shows(&mut app, REFUSED));
    app.press(Action::PassEye);
    assert!(shows(&mut app, "caf"), "the letter before it was taken");
    // The next letter taken clears the line.
    type_text(&mut app, "e");
    assert!(shows(&mut app, "cafe"));
    assert!(!shows(&mut app, REFUSED));
}

#[test]
fn lightning_s_passphrase_has_an_eye_that_shows_and_hides_it() {
    let mut app = lightning_field();
    type_text(&mut app, "hunter2");
    assert!(!shows(&mut app, "hunter2"), "dots, not the letters");
    app.press(Action::PassEye);
    assert!(shows(&mut app, "hunter2"));
    app.press(Action::PassEye);
    assert!(!shows(&mut app, "hunter2"));
}

#[test]
fn lightning_s_passphrase_takes_no_typing_with_a_stick_attached() {
    let mut app = testkit::started();
    attach(&mut app);
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    app.press(Action::LPassphrase);
    type_text(&mut app, "hunter2");
    assert!(shows(&mut app, STICK_LINE), "the screen says why");
    app.press(Action::PassEye);
    assert!(!shows(&mut app, "hunter2"));
}

fn detach(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(Vec::new()));
}

#[test]
fn a_stick_arriving_with_the_possible_wallet_sheet_up_empties_the_passphrase() {
    let mut app = potential_sheet();
    type_text(&mut app, "hunter2");
    app.press(Action::PotentialTypeAgain);
    type_text(&mut app, "hunter2");
    assert!(app.potential().unwrap().typed_len() > 0);
    // The stick's arrival puts its own sheet over this one.
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("STICK"),
        boot: false,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    assert_ne!(app.sheet, Some(Sheet::Potential));
    assert!(app.potential().is_none_or(|p| p.typed_len() == 0));
}

#[test]
fn a_stick_arriving_with_lightning_up_empties_its_passphrase() {
    let mut app = lightning_field();
    type_text(&mut app, "hunter2");
    app.press(Action::PassEye);
    assert!(shows(&mut app, "hunter2"));
    attach(&mut app);
    assert_eq!(app.screen, Screen::Lightning);
    detach(&mut app);
    assert_eq!(app.screen, Screen::Lightning);
    app.press(Action::LPassphrase);
    assert!(!shows(&mut app, "hunter2"));
    assert_eq!(app.lightning().unwrap().passphrase(), "");
}

#[test]
fn the_possible_wallet_sheet_waits_for_the_eye_and_says_so() {
    const US_LINE: &str = "Keys are read as a US keyboard. Show the passphrase once to check it.";
    let mut app = potential_sheet();
    type_text(&mut app, "ab");
    app.press(Action::PotentialTypeAgain);
    type_text(&mut app, "ab");
    assert!(shows(&mut app, US_LINE));
    app.press(Action::PotentialMake);
    assert_eq!(app.sheet, Some(Sheet::Potential), "Make waits for the eye");
    app.press(Action::PassEye);
    app.press(Action::PotentialMake);
    assert_eq!(app.sheet, None);
}

/// Add a key with its words typed.
fn add_a_key() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    type_text(&mut app, &testkit::test_words("bacon"));
    app
}

#[test]
fn add_a_keys_passphrase_takes_no_typing_with_a_stick_attached() {
    let mut app = add_a_key();
    attach(&mut app);
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "hunter2");
    assert!(shows(&mut app, STICK_LINE), "the screen says why");
    assert_eq!(testkit::entry(&app).passphrase(), "");
}

#[test]
fn shift_tab_does_not_move_in_add_a_keys_passphrase_with_a_stick_attached() {
    let mut app = add_a_key();
    attach(&mut app);
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "x");
    assert!(shows(&mut app, STICK_LINE));
    app.event(Event::Key(Key::BackTab));
    assert!(
        shows(&mut app, STICK_LINE),
        "Shift-Tab is refused as Tab is"
    );
    assert_eq!(testkit::entry(&app).passphrase(), "");
}

#[test]
fn a_stick_arriving_empties_add_a_keys_passphrase() {
    let mut app = add_a_key();
    app.press(Action::EntryPassphrase);
    type_text(&mut app, "hunter2");
    assert_eq!(testkit::entry(&app).passphrase(), "hunter2");
    attach(&mut app);
    assert_eq!(testkit::entry(&app).passphrase(), "");
    detach(&mut app);
    assert_eq!(testkit::entry(&app).passphrase(), "");
}

#[test]
fn add_a_keys_passphrase_stops_at_256_bytes_and_says_so() {
    const TOO_LONG: &str = "That passphrase is as long as one can be";
    let mut app = add_a_key();
    app.press(Action::EntryPassphrase);
    type_text(&mut app, &"a".repeat(300));
    assert_eq!(testkit::entry(&app).passphrase().len(), 256);
    assert!(shows(&mut app, TOO_LONG));
}
