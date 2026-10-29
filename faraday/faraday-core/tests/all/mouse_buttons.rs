//! The mouse's back and forward buttons (`docs/history/NEW-WALLET.md` §13.1):
//! back does what the screen's back link does, or Escape where there is
//! none; forward opens again the screen back left, as it was left, until
//! something else is opened. A screen that keeps nothing when it is left
//! (New key) has nothing for forward to open.

use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Event};

/// New key opened from Add a key, on its first card.
fn new_key() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    let _ = app.frame();
    app
}

fn button(app: &mut Faraday, e: Event) {
    app.event(e);
    let _ = app.frame();
}

#[test]
fn back_goes_where_the_back_link_goes_and_forward_comes_back_as_it_was() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CKind(1));
    let kind = app.create().unwrap().kind;
    let _ = app.frame();
    button(&mut app, Event::Back);
    assert_ne!(app.screen, Screen::Create);
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().kind, kind, "kept as it was left");
}

#[test]
fn new_key_left_by_back_keeps_nothing_for_forward_to_open() {
    let mut app = new_key();
    app.press(Action::KWords(24));
    let _ = app.frame();
    // Back steps within New key first (F104): Length, then Start, each
    // as it was chosen; from Start it leaves.
    let open = |app: &Faraday| app.keygen().and_then(|k| k.open);
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(faraday_core::keygen::kstep::LENGTH));
    assert_eq!(app.keygen().unwrap().words, 24, "as it was chosen");
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(faraday_core::keygen::kstep::START));
    button(&mut app, Event::Back);
    assert_eq!(
        app.screen,
        Screen::Entry,
        "from Start, New key's back link is Add a key"
    );
    assert!(app.keygen().is_none(), "New key keeps nothing when left");
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn forward_does_nothing_once_another_screen_is_opened() {
    let mut app = new_key();
    // Length, then Start: Back from Start leaves.
    button(&mut app, Event::Back);
    button(&mut app, Event::Back);
    assert_eq!(app.screen, Screen::Entry);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn back_closes_a_sheet_as_escape_does_and_leaves_the_screen() {
    let mut app = new_key();
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    let _ = app.frame();
    button(&mut app, Event::Back);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
}
