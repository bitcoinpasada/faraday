//! On a one-column layout (a phone, the 7-inch panel) a press on a bar
//! or field that takes typing brings the on-screen keyboard up.

use faraday_core::edit::Here;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, DisplayInfo, Event, TouchPhase};

/// A phone and the 7-inch panel.
const WINDOWS: [(u16, u16, u16); 2] = [(1081, 2400, 420), (1024, 600, 170)];

fn window((width, height, dpi): (u16, u16, u16)) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
    let _ = app.frame();
}

#[test]
fn a_press_on_a_bar_or_field_that_takes_typing_brings_the_keyboard_up_and_again_once_put_away() {
    use crate::common::type_text;
    for w in WINDOWS {
        // Tools' Find a tool, a bar with no field to focus.
        let mut app = window(w);
        app.press(Action::Nav(Screen::Catalog));
        let _ = app.frame();
        assert!(app.keyboard_top().is_none(), "{w:?}: down first");
        tap(&mut app, Action::TypeHere(Here::Find));
        assert!(app.keyboard_top().is_some(), "{w:?}: Find a tool");

        // BIP-85's index, a field already typed into: put away, a press
        // brings the keyboard back.
        let mut app = window(w);
        app.press(Action::Entry(None));
        type_text(
            &mut app,
            &faraday_core::testkit::test_words(faraday_core::testkit::TEST_SEEDS[0].0),
        );
        app.press(Action::EntryAdd);
        app.press(Action::Bip85);
        app.press(Action::PApp(0));
        app.press(Action::PNext);
        let index = Action::TypeHere(Here::Index);
        tap(&mut app, index);
        assert!(app.keyboard_top().is_some(), "{w:?}: the index");
        tap(&mut app, Action::Osk(faraday_ui::keyboard::OskPress::Hide));
        assert!(app.keyboard_top().is_none(), "{w:?}: put away");
        tap(&mut app, index);
        assert!(app.keyboard_top().is_some(), "{w:?}: the index again");

        // Lightning's aezeed.
        let mut app = window(w);
        app.press(Action::Lightning);
        app.press(Action::LAezeed);
        tap(&mut app, Action::TypeHere(Here::Aezeed));
        assert!(app.keyboard_top().is_some(), "{w:?}: the aezeed");
    }
}
