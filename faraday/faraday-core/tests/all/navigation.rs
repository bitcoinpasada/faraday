//! Navigation by window class (`docs/LAYOUT.md` §3): the Pi's panel
//! keeps Home's menu; a compact window has the navigation bar at its
//! foot; a medium, an expanded or a low window the rail at its left,
//! with Learn, Settings and Lock; a large or extra-large one the
//! expanded rail, which lists the flows in progress. Where there is no
//! expanded rail, a flow in progress shows its chip on every
//! destination; a stick being written hides the navigation.

use faraday_core::in_progress::Going;
use faraday_core::nav::Nav;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, cstep};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

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

/// Where `action` is pressed on the frame drawn now, pixels: the last
/// region drawn for it, the one on top.
fn at(app: &mut Faraday, action: Action) -> Option<(u16, u16)> {
    let _ = app.frame();
    app.where_offered(action)
}

/// A finger on (x, y).
fn tap(app: &mut Faraday, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
}

/// The five destinations and the screen each opens.
const FIVE: [(Screen, Screen); 5] = [
    (Screen::WalletsStart, Screen::WalletsStart),
    (Screen::Files, Screen::Files),
    (Screen::Vaults, Screen::Vaults),
    (Screen::Catalog, Screen::Catalog),
    (Screen::Home, Screen::Home),
];

/// A key typed in: something held, so Lock is offered.
fn key_in(app: &mut Faraday) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::Nav(Screen::Home));
    assert!(app.holds_secret(), "the key did not load");
}

/// Make a new wallet, its first step done: a flow in progress.
fn flow_begun(app: &mut Faraday) {
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::Nav(Screen::Home));
}

#[test]
fn the_pi_panel_shows_homes_menu_and_no_bar() {
    // 480 × 640 at 286 dpi: 268 × 358 dp.
    let mut app = on(480, 640, 286);
    assert_eq!(app.nav(), Nav::Menu);
    // Wallets and Learn are buttons on the menu itself.
    assert!(at(&mut app, Action::Nav(Screen::WalletsStart)).is_some());
    assert!(at(&mut app, Action::Learn).is_some());
    assert!(!app.offers(Action::NavMenu(None)), "no top app bar");
    // Another page has the bar back to Home, and no rail or bar.
    app.press(Action::Nav(Screen::Settings));
    let (_, y) = at(&mut app, Action::Nav(Screen::Home)).expect("the way back");
    assert!(y < 44 * 286 / 160, "the way back is not at the top");
    assert!(
        !app.offers(Action::Nav(Screen::Files)),
        "no navigation on Settings"
    );
}

#[test]
fn a_compact_window_shows_the_bars_five_destinations_and_each_opens() {
    let mut app = on(400, 800, 160);
    assert_eq!(app.nav(), Nav::Bar);
    for (dest, screen) in FIVE {
        let (x, y) =
            at(&mut app, Action::Nav(dest)).unwrap_or_else(|| panic!("{dest:?} not on the bar"));
        assert!(y >= 800 - 80, "{dest:?} not in the bar at the foot: {y}");
        tap(&mut app, (x, y));
        assert_eq!(app.screen, screen, "{dest:?}");
        // The bar stays on every destination.
        let (_, y) = at(&mut app, Action::Nav(Screen::Home)).expect("the bar");
        assert!(y >= 800 - 80);
    }
    // Settings is in the top app bar's ⋮ menu.
    let more = at(&mut app, Action::NavMenu(None)).expect("the ⋮ menu");
    assert!(more.1 < 64);
    tap(&mut app, more);
    assert_eq!(app.sheet, Some(Sheet::NavMenu));
    let settings = at(&mut app, Action::NavMenu(Some(Screen::Settings))).expect("Settings");
    tap(&mut app, settings);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Settings);
}

/// The rail at a window's left: the five destinations, Learn, Settings
/// and (with a key held) Lock, each opening what it names.
fn the_rail_offers_and_opens(width: u16, height: u16) {
    let mut app = on(width, height, 160);
    assert_eq!(app.nav(), Nav::Rail, "{width} × {height}");
    key_in(&mut app);
    let in_rail = |app: &mut Faraday, a: Action| {
        let p = at(app, a).unwrap_or_else(|| panic!("{a:?} not on the rail"));
        assert!(p.0 < 80, "{a:?} not in the rail: {p:?}");
        p
    };
    for (dest, screen) in FIVE {
        let p = in_rail(&mut app, Action::Nav(dest));
        tap(&mut app, p);
        assert_eq!(app.screen, screen, "{dest:?}");
    }
    let p = in_rail(&mut app, Action::Learn);
    tap(&mut app, p);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    app.press(Action::Cancel);
    let p = in_rail(&mut app, Action::Nav(Screen::Settings));
    tap(&mut app, p);
    assert_eq!(app.screen, Screen::Settings);
    let p = in_rail(&mut app, Action::LockAsk);
    tap(&mut app, p);
    assert!(
        app.sheet == Some(Sheet::LockAsk) || !app.holds_secret(),
        "Lock did nothing"
    );
}

#[test]
fn a_half_screen_window_shows_the_rail_with_learn_settings_and_lock() {
    the_rail_offers_and_opens(760, 1000);
}

#[test]
fn a_seven_inch_panel_sized_window_shows_the_rail_with_learn_settings_and_lock() {
    the_rail_offers_and_opens(960, 580);
}

/// The expanded rail at the window's left lists a flow in progress.
fn the_expanded_rail_lists_a_flow(width: u16, height: u16) {
    let mut app = on(width, height, 160);
    assert_eq!(app.nav(), Nav::ExpandedRail);
    flow_begun(&mut app);
    app.press(Action::Nav(Screen::Settings));
    let (x, _) = at(&mut app, Action::Resume(Going::Create)).expect("the flow's row");
    assert!(x < 240, "not in the expanded rail");
}

#[test]
fn a_desktop_window_shows_the_expanded_rail_with_a_flow_in_progress() {
    the_expanded_rail_lists_a_flow(1280, 800);
}

#[test]
fn a_full_monitor_shows_the_expanded_rail_with_a_flow_in_progress() {
    the_expanded_rail_lists_a_flow(1920, 1080);
}

#[test]
fn a_flow_in_progress_shows_its_chip_on_every_destination() {
    for (w, h) in [(400, 800), (760, 1000), (960, 580)] {
        let mut app = on(w, h, 160);
        flow_begun(&mut app);
        for dest in [
            Screen::Home,
            Screen::WalletsStart,
            Screen::Files,
            Screen::Vaults,
            Screen::Catalog,
            Screen::Settings,
        ] {
            app.press(Action::Nav(dest));
            let p = at(&mut app, Action::Resume(Going::Create))
                .unwrap_or_else(|| panic!("no chip on {dest:?} at {w} × {h}"));
            tap(&mut app, p);
            assert_eq!(app.screen, Screen::Create, "the chip on {dest:?}");
        }
    }
}

/// The y of Home's Make a new wallet tile and of Restore a wallet's.
fn home_tile_rows(width: u16, height: u16, dpi: u16) -> (u16, u16) {
    let mut app = on(width, height, dpi);
    app.press(Action::Nav(Screen::Home));
    let a = at(&mut app, Action::CreateWallet).expect("Make a new wallet is not on Home");
    let b = at(&mut app, Action::RestoreWallet).expect("Restore a wallet is not on Home");
    (a.1, b.1)
}

#[test]
fn homes_tiles_take_as_many_columns_as_the_window_has_room_for() {
    for (w, h) in [(1280, 800), (1920, 1080)] {
        let (a, b) = home_tile_rows(w, h, 96);
        assert_eq!(a, b, "{w}x{h}: the tiles are not side by side");
    }
}

/// Whether box `inner` (x, y, w, h) has its middle inside `outer`.
fn within(inner: (i32, i32, i32, i32), outer: (u16, u16, u16, u16)) -> bool {
    let (cx, cy) = (inner.0 + inner.2 / 2, inner.1 + inner.3 / 2);
    let (x, y, w, h) = (
        i32::from(outer.0),
        i32::from(outer.1),
        i32::from(outer.2),
        i32::from(outer.3),
    );
    cx >= x && cx < x + w && cy >= y && cy < y + h
}

/// The boxes the frame draws the text `want` in.
fn drawn_at(app: &mut Faraday, want: &str) -> Vec<(i32, i32, i32, i32)> {
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == want)
        .map(|(_, b)| b)
        .collect()
}

#[test]
fn files_carries_the_inbox_count_on_the_bar_and_both_rails_until_it_empties() {
    for (w, h) in [(400, 800), (760, 1000), (1280, 800)] {
        let mut app = on(w, h, 160);
        let files = (1..=3)
            .map(|i| (format!("note-{i}.txt"), b"a note".to_vec()))
            .collect();
        app.storage(StorageEvent::Restored {
            inbox: files,
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::Nav(Screen::Vaults));
        let _ = app.frame();
        let badges = drawn_at(&mut app, "3");
        let shown = match app.nav() {
            // The expanded rail: on its Files row, in the rail's width.
            Nav::ExpandedRail => badges.iter().any(|b| b.0 + b.2 < 240),
            _ => {
                let files = app.hit_box(Action::Nav(Screen::Files)).unwrap();
                badges.iter().any(|b| within(*b, files))
            }
        };
        assert!(shown, "{w} × {h}: no count on Files, {badges:?}");
        for _ in 0..3 {
            app.press(Action::InboxRemove(0));
        }
        assert!(app.inbox().is_empty());
        assert!(drawn_at(&mut app, "3").is_empty(), "{w} × {h}");
    }
}

/// Where the frame draws a badge's dot.
fn dots(app: &mut Faraday) -> Vec<(i32, i32, i32, i32)> {
    app.drawn_ink()
        .into_iter()
        .filter(|i| i.kind == faraday_draw::canvas::InkKind::Dot)
        .map(|i| (i.rect.x, i.rect.y, i.rect.w, i.rect.h))
        .collect()
}

#[test]
fn the_stick_visit_carries_a_dot_until_the_stick_is_visited() {
    let mut app = on(760, 1000, 160);
    // The boot stick, at its first look, leaves Home on screen: it waits.
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "sda1@sda#1".into(),
        label: "FARADAY".into(),
        boot: true,
        files: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Home);
    let drawn = dots(&mut app);
    let visit = app.hit_box(Action::Nav(Screen::Visit)).expect("the visit");
    assert!(drawn.iter().any(|d| within(*d, visit)), "{drawn:?}");
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::Nav(Screen::Home));
    assert!(dots(&mut app).is_empty());
}
