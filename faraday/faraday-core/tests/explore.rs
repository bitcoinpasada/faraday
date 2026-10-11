//! Explore shows the public side of a loaded key at a path: the
//! addresses BIP-84 and BIP-86 publish for "abandon … about", the script
//! a path's purpose names, and a path that is not one named as such.

use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, Event, Key};

fn exploring() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    for c in "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::Explore);
    assert_eq!(app.screen, Screen::Explore);
    app
}

#[test]
fn bip84_and_bip86s_first_addresses() {
    let mut app = exploring();
    let r = app.explore_reading().unwrap();
    assert_eq!(r.address, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
    app.press(Action::XPreset(1));
    let r = app.explore_reading().unwrap();
    assert_eq!(
        r.address,
        "bc1p5cyxnuxmeuwuvkwfem96lqzszd02n6xdcjrs20cac6yqjjwudpxqkedrcr"
    );
    // The next index along.
    app.press(Action::XPreset(0));
    app.press(Action::XIndex(1));
    let r = app.explore_reading().unwrap();
    assert_eq!(r.address, "bc1qnjg0jd8228aq7egyzacy8cys3knf9xvrerkf9g");
}

#[test]
fn a_path_typed_wrongly_is_named() {
    let mut app = exploring();
    for _ in 0..20 {
        app.event(Event::Key(Key::Backspace));
    }
    for c in "84h//0".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(app.explore_path().is_err());
    assert!(app.explore_reading().is_none());
}
