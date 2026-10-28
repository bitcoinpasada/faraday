//! The top app bar's actions (the mode pill, the ?) stand on the bar on
//! every wide page, so a page scrolled under them shows nothing there
//! (Material's top app bar; `docs/LAYOUT.md` §3, §15).

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// The bar's right end in the frame drawn now: the right 140 px and the
/// top 36, the bar being 38 px tall at 96 dpi.
fn corner(app: &mut Faraday, w: usize) -> Vec<u8> {
    let f = app.frame();
    let width = usize::from(f.width);
    let mut out = Vec::new();
    for y in 0..36 {
        let from = (y * width + w - 140) * 4;
        out.extend_from_slice(&f.rgba[from..from + 140 * 4]);
    }
    out
}

#[test]
fn nothing_but_the_bar_is_drawn_under_the_actions_of_a_long_wide_page() {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 420,
        dpi: 96,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::WalletsStart));
    let mut now = 1_000;
    let mut tick = |app: &mut Faraday| {
        now += 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    };
    tick(&mut app);
    let first = app.frame().rgba.to_vec();
    let mut under: Option<Vec<u8>> = None;
    let mut seen = 0;
    for _ in 0..40 {
        app.event(Event::Scroll {
            x: 450,
            y: 250,
            dy: 8,
        });
        app.event(Event::ScrollEnd { x: 450, y: 250 });
        for _ in 0..12 {
            tick(&mut app);
        }
        // At every stop of the scroll the corner is what it was at the
        // first: the bar in its scrolled-under colour and its actions.
        let now = corner(&mut app, 1280);
        match &under {
            None => under = Some(now),
            Some(c) if *c != now => seen += 1,
            Some(_) => {}
        }
    }
    assert_eq!(
        seen, 0,
        "page content under the actions at {seen} of 40 stops"
    );
    let scrolled = app.frame().rgba.to_vec();
    assert_ne!(first, scrolled, "the page scrolled");
}
