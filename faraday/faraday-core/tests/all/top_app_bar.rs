//! The top app bar over a wide page beside the expanded rail
//! (`docs/LAYOUT.md` §3, DESIGN §4.1): it names the page, its back arrow
//! leaves the page where the page has a way back, and it takes the
//! scrolled-under colour while the page is scrolled under it.

use faraday_core::strings::EN;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// The app on a window `width` × `height` pixels at `dpi`, drawn once.
fn on(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
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

/// Time moves on a frame, and the frame is drawn.
fn tick(app: &mut Faraday) {
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
}

/// The bar's height in pixels at `dpi`, a pixel short of its foot.
fn bar_px(dpi: u16) -> i32 {
    64 * i32::from(dpi) / 160 - 1
}

fn names_the_page_and_goes_back(width: u16, height: u16) {
    let mut app = on(width, height, 160);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::Nav(Screen::CheckMessage));
    let boxes = app.drawn_text_boxes();
    let title = boxes
        .iter()
        .find(|(t, _)| t == EN.check_check_a_signed)
        .unwrap_or_else(|| panic!("no page name drawn at {width} × {height}"));
    assert!(
        title.1.1 + title.1.3 <= bar_px(160),
        "the page's name at {:?} is not in the bar at {width} × {height}",
        title.1
    );
    // The way back the bar offers is above the page, beside the rail.
    let (x, y) = app
        .where_offered(Action::Nav(Screen::WalletsStart))
        .expect("the way back is offered");
    assert!(
        i32::from(y) <= bar_px(160) && x > 240,
        "the way back at {x}, {y} is not the bar's at {width} × {height}"
    );
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    tick(&mut app);
    assert_eq!(app.screen, Screen::WalletsStart, "the bar's back arrow");
}

#[test]
fn a_wide_page_is_named_by_its_top_app_bar_whose_back_arrow_leaves_it() {
    names_the_page_and_goes_back(1280, 800);
    names_the_page_and_goes_back(1920, 1080);
}

/// The colour of the bar's empty middle, at its top.
fn bar_colour(app: &mut Faraday, x: usize) -> [u8; 4] {
    let f = app.frame();
    let i = (2 * usize::from(f.width) + x) * 4;
    [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2], f.rgba[i + 3]]
}

/// Forty notches of the wheel over the page, by `dy` each.
fn scroll(app: &mut Faraday, dy: i16) {
    for _ in 0..40 {
        app.event(Event::Scroll { x: 450, y: 250, dy });
        app.event(Event::ScrollEnd { x: 450, y: 250 });
        for _ in 0..12 {
            tick(app);
        }
    }
}

#[test]
fn the_top_app_bar_takes_the_scrolled_under_colour_while_the_page_is_scrolled() {
    let mut app = on(1280, 420, 96);
    app.press(Action::Nav(Screen::WalletsStart));
    tick(&mut app);
    let top = bar_colour(&mut app, 800);
    scroll(&mut app, 8);
    assert_ne!(bar_colour(&mut app, 800), top, "scrolled, the bar's colour");
    scroll(&mut app, -8);
    assert_eq!(
        bar_colour(&mut app, 800),
        top,
        "back at the top, the bar's colour"
    );
}

fn a_flow_is_named_by_the_bar(width: u16, height: u16) {
    let mut app = on(width, height, 160);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::RestoreWallet);
    tick(&mut app);
    assert_eq!(app.screen, Screen::Restore, "Restore is open");
    let boxes = app.drawn_text_boxes();
    let title = boxes
        .iter()
        .find(|(t, _)| t == EN.restore_title)
        .unwrap_or_else(|| panic!("no flow name drawn at {width} × {height}"));
    assert!(
        title.1.1 + title.1.3 <= bar_px(160) && title.1.0 > 240,
        "the flow's name at {:?} is not in the bar at {width} × {height}",
        title.1
    );
    // The back arrow at the bar's start, beside the rail.
    let (x, y) = (240 + 4 + 24, 32);
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    tick(&mut app);
    assert_eq!(
        app.screen,
        Screen::WalletsStart,
        "the bar's back arrow at {width} × {height}"
    );
}

#[test]
fn a_wide_step_flow_is_named_by_its_top_app_bar_whose_back_arrow_leaves_it() {
    a_flow_is_named_by_the_bar(1280, 800);
    a_flow_is_named_by_the_bar(1920, 1080);
}

#[test]
fn a_wide_flow_in_progress_has_cancel_on_its_top_app_bar() {
    let mut app = on(1280, 800, 160);
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    tick(&mut app);
    assert_eq!(app.screen, Screen::Create, "Make a new wallet is open");
    let (x, y) = app
        .where_offered(Action::CancelFlow)
        .expect("Cancel is offered");
    assert!(
        i32::from(y) <= bar_px(160) && x > 240,
        "Cancel at {x}, {y} is not on the bar"
    );
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    tick(&mut app);
    assert!(
        app.screen != Screen::Create || app.sheet.is_some(),
        "Cancel on the bar leaves the flow or asks first"
    );
}

#[test]
fn wide_home_is_named_by_its_top_app_bar() {
    for (width, height) in [(1280, 800), (1920, 1080)] {
        let mut app = on(width, height, 160);
        assert_eq!(app.screen, Screen::Home);
        let boxes = app.drawn_text_boxes();
        let names: Vec<_> = boxes
            .iter()
            .filter(|(t, r)| t == EN.home_home && r.0 > 240)
            .collect();
        assert_eq!(names.len(), 1, "Home is named once at {width} × {height}");
        let r = names[0].1;
        assert!(
            r.1 + r.3 <= bar_px(160),
            "Home's name at {r:?} is not in the bar at {width} × {height}"
        );
    }
}
