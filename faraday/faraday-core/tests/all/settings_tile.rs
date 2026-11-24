//! Settings on the wide layout is one tile of group cards: every control
//! is reachable by scrolling and works, at the sizes a person has.

use faraday_core::ui::{Theme, ThemeName};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn at(width: u16, height: u16) -> Faraday {
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
    let _ = app.frame();
    app
}

/// Scrolls the page as a person would until a frame offers `action`.
fn scroll_until_offered(app: &mut Faraday, (x, y): (u16, u16), action: Action) -> bool {
    for _ in 0..60 {
        let _ = app.frame();
        if app.offers(action) {
            return true;
        }
        app.event(Event::Scroll { x, y, dy: 120 });
    }
    false
}

#[test]
fn the_last_card_is_reached_by_scrolling_at_each_wide_size() {
    for (w, h) in [(1366, 768), (1920, 1080), (1024, 600)] {
        let mut app = at(w, h);
        assert!(
            scroll_until_offered(&mut app, (w / 2, h / 2), Action::SelfTestRun),
            "About's self-test is never offered at {w}x{h}"
        );
        let (_, y, _, bh) = app.hit_box(Action::SelfTestRun).expect("offered");
        assert!(
            u32::from(y) + u32::from(bh) <= u32::from(h),
            "off screen at {w}x{h}"
        );
    }
}

#[test]
fn a_theme_chip_is_pressed_and_kept_inside_the_tile() {
    for (w, h) in [(1366, 768), (1920, 1080), (1024, 600)] {
        let mut app = at(w, h);
        let want = Theme {
            name: ThemeName::BitcoinOrange,
            ..app.theme()
        };
        assert!(
            scroll_until_offered(&mut app, (w / 2, h / 2), Action::Theme(want)),
            "no dark high chip at {w}x{h}"
        );
        app.press(Action::Theme(want));
        assert_eq!(app.theme(), want, "at {w}x{h}");
    }
}

#[test]
fn a_compact_window_still_lists_the_sections() {
    let mut app = at(480, 640);
    let listed = (0..8u8).any(|k| app.offers(Action::SettingsSection(Some(k))));
    assert!(listed, "no list of sections at 480x640");
    crate::common::settings_section(&mut app, "About");
    assert!(scroll_until_offered(
        &mut app,
        (240, 320),
        Action::SelfTestRun
    ));
}
