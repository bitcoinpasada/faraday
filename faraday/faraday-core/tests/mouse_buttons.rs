//! The mouse's back and forward buttons (`docs/NEW-WALLET.md` §13.1):
//! back does what the screen's back link does, or Escape where there is
//! none; forward opens again the screen back left, as it was left, until
//! something else is opened.

use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use osk_shell_api::{App, Event};

/// New key opened from Add a key, on its first card.
fn new_key() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    let _ = app.frame();
    app
}

fn button(app: &mut Faraday, e: Event) {
    app.event(e);
    let _ = app.frame();
}

#[test]
fn back_goes_where_the_back_link_goes_and_forward_comes_back_as_it_was() {
    let mut app = new_key();
    app.press(Action::KWords(24));
    let _ = app.frame();
    button(&mut app, Event::Back);
    assert_eq!(
        app.screen,
        Screen::Entry,
        "New key's back link is Add a key"
    );
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(
        app.keygen.as_ref().unwrap().words,
        24,
        "kept as it was left"
    );
}

#[test]
fn forward_does_nothing_once_another_screen_is_opened() {
    let mut app = new_key();
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
