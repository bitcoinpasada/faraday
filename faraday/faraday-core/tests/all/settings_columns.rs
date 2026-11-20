//! Settings' cards in the menu page's columns: as the window is resized
//! a card stays in its column, Mode to Randomness in the first and
//! Appearance on in the second.

use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Settings on a `width` × `height` window at 160 dpi.
fn settings_at(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
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
    app.press(Action::Nav(Screen::Settings));
    app
}

/// The left edge of the card titled `title`, pixels.
fn card_x(app: &mut Faraday, title: &str) -> i32 {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == title)
        .map(|(_, b)| b.0)
        .unwrap_or_else(|| panic!("{title:?} drawn"))
}

#[test]
fn a_settings_card_stays_in_its_column_as_the_window_is_resized() {
    for (w, h) in [(1280, 800), (1440, 900), (1920, 1080), (2560, 1440)] {
        let mut app = settings_at(w, h);
        let first = card_x(&mut app, EN.settings_time);
        let second = card_x(&mut app, EN.settings_appearance);
        assert!(first < second, "{w}x{h}: {first} {second}");
        assert!(
            card_x(&mut app, EN.settings_lock_and_power) < second,
            "{w}x{h}: Lock and power in the first column"
        );
    }
}
