//! The on-screen keyboard in the wide layout (the stick on a normal
//! screen, the desktop app), for a person with only a wireless keyboard:
//! a keyboard button beside the ? where typing goes somewhere, which
//! docks the keyboard at the foot of the page area and puts it away. It
//! never rises by itself, and its keys type what physical keys would.

use faraday_core::{Action, Faraday, OskPress, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

const W: u16 = 1280;
const H: u16 = 800;

const SHOW: Action = Action::Osk(OskPress::Show);
const HIDE: Action = Action::Osk(OskPress::Hide);

fn wide() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: W,
        height: H,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Where `action` can be pressed in the window as drawn now.
fn offered(app: &mut Faraday, action: Action) -> Option<(u16, u16)> {
    let _ = app.frame();
    app.where_offered(action).filter(|&(x, y)| x < W && y < H)
}

fn click(app: &mut Faraday, action: Action) {
    let (x, y) = offered(app, action).unwrap_or_else(|| panic!("{action:?} is not offered"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

/// Add a key, its BIP-39 passphrase field focused.
fn on_passphrase() -> Faraday {
    let mut app = wide();
    app.press(Action::Entry(None));
    app.press(Action::EntryPassphrase);
    assert!(faraday_core::testkit::entry(&app).on_passphrase);
    app
}

#[test]
fn the_button_is_offered_where_typing_goes_somewhere_and_nowhere_else() {
    let mut app = on_passphrase();
    assert!(offered(&mut app, SHOW).is_some(), "a focused field");
    app.press(Action::Nav(Screen::Catalog));
    assert!(
        offered(&mut app, SHOW).is_some(),
        "Tools, which types with no field"
    );
    for s in [Screen::Home, Screen::Settings, Screen::Vaults] {
        app.press(Action::Nav(s));
        assert!(offered(&mut app, SHOW).is_none(), "{s:?}");
    }
}

#[test]
fn it_never_rises_by_itself() {
    let mut app = on_passphrase();
    assert!(offered(&mut app, Action::Osk(OskPress::Char('a'))).is_none());
    app.event(Event::Key(Key::Char('x')));
    assert!(offered(&mut app, Action::Osk(OskPress::Char('a'))).is_none());
}

#[test]
fn its_keys_type_into_a_passphrase_field_as_physical_keys_do() {
    let mut by_hand = on_passphrase();
    for c in ['a', 'b'] {
        by_hand.event(Event::Key(Key::Char(c)));
    }
    by_hand.event(Event::Key(Key::Backspace));
    by_hand.event(Event::Key(Key::Char('c')));

    let mut app = on_passphrase();
    click(&mut app, SHOW);
    for c in ['a', 'b'] {
        click(&mut app, Action::Osk(OskPress::Char(c)));
    }
    click(&mut app, Action::Osk(OskPress::Back));
    click(&mut app, Action::Osk(OskPress::Char('c')));
    assert_eq!(faraday_core::testkit::entry(&app).passphrase.as_str(), "ac");
    assert_eq!(
        faraday_core::testkit::entry(&app).passphrase.as_str(),
        faraday_core::testkit::entry(&by_hand).passphrase.as_str()
    );
    // The field kept its focus through every key.
    assert!(faraday_core::testkit::entry(&app).on_passphrase);
}

#[test]
fn it_docks_beside_the_sidebar_and_goes_on_a_second_press() {
    let mut app = on_passphrase();
    click(&mut app, SHOW);
    let (x, y) = offered(&mut app, Action::Osk(OskPress::Char('q'))).expect("the keyboard is up");
    assert!(y > H / 2, "at the foot of the window");
    // The passphrase field stays in view above it.
    let (_, field_y) = offered(&mut app, Action::EntryPassphrase).expect("the field");
    let (_, top_row) = offered(&mut app, Action::Osk(OskPress::Char('1'))).expect("the digits");
    assert!(field_y < top_row, "the field is above the keyboard");
    // Not over the sidebar: the sidebar's places are still offered.
    let (hx, _) = offered(&mut app, Action::Nav(Screen::Home)).expect("the sidebar");
    assert!(x > hx);
    assert!(offered(&mut app, Action::Nav(Screen::Settings)).is_some());
    click(&mut app, HIDE);
    assert!(offered(&mut app, Action::Osk(OskPress::Char('q'))).is_none());
    assert!(offered(&mut app, SHOW).is_some());
}

#[test]
fn done_puts_it_away_and_leaving_the_page_does_too() {
    let mut app = on_passphrase();
    click(&mut app, SHOW);
    click(&mut app, Action::Osk(OskPress::Done));
    assert!(offered(&mut app, Action::Osk(OskPress::Char('q'))).is_none());
    app.press(Action::Nav(Screen::Catalog));
    click(&mut app, SHOW);
    assert!(offered(&mut app, Action::Osk(OskPress::Char('q'))).is_some());
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::Catalog));
    assert!(offered(&mut app, Action::Osk(OskPress::Char('q'))).is_none());
}
