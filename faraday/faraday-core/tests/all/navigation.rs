//! Navigation by window class (`docs/LAYOUT.md` §3): the Pi's panel
//! keeps Home's menu; a compact window has the navigation bar at its
//! foot; a medium, an expanded or a low window the rail at its left,
//! with the guide and Settings; a large or extra-large one the
//! expanded rail, which lists the flows in progress. Where there is no
//! expanded rail, a flow in progress shows its chip on every
//! destination; a stick being written hides the navigation.

use faraday_core::in_progress::Going;
use faraday_core::nav::{BarTool, MenuPress, Nav, Overflow};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent};
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

/// The five destinations and the screen each opens. Vaults opens New
/// vault while Files holds no vault file.
const FIVE: [(Screen, Screen); 5] = [
    (Screen::WalletsStart, Screen::WalletsStart),
    (Screen::Files, Screen::Files),
    (Screen::Vaults, Screen::CreateVault),
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
    faraday_core::testkit::keep_published(app);
    app.press(Action::Nav(Screen::Home));
    assert!(app.holds_secret(), "the key did not load");
}

/// Make a new wallet, its first step done: a flow in progress.
fn flow_begun(app: &mut Faraday) {
    app.press(Action::CreateWallet);
    crate::common::ckind(app, 0);
    app.press(Action::Nav(Screen::Home));
}

/// The ⋮ on the page shown, whatever it holds.
fn more_menu(app: &mut Faraday) -> Action {
    let _ = app.frame();
    (0u8..64)
        .map(|bits| {
            Action::NavMenu(MenuPress::Open(Some(Overflow {
                own: bits & 1 != 0,
                choices: bits & 16 != 0,
                help: bits & 2 != 0,
                keyboard: bits & 4 != 0,
                keyboard_up: bits & 32 != 0,
                eye: bits & 8 != 0,
            })))
        })
        .find(|a| app.offers(*a))
        .expect("the bar's ⋮")
}

#[test]
fn the_pi_panels_menu_locks_from_a_flows_page_and_always_offers_power_off() {
    let mut app = on(480, 640, 286);
    app.press(Action::CreateWallet);
    let open = more_menu(&mut app);
    app.press(open);
    let _ = app.frame();
    assert!(app.offers(Action::NavMenu(MenuPress::Tool(BarTool::Power))));
    assert!(!app.offers(Action::NavMenu(MenuPress::Tool(BarTool::Lock))));
    app.press(Action::Cancel);
    app.press(Action::CancelFlow);
    app.press(Action::Nav(Screen::Home));
    key_in(&mut app);
    app.press(Action::CreateWallet);
    let open = more_menu(&mut app);
    app.press(open);
    let _ = app.frame();
    assert!(app.offers(Action::NavMenu(MenuPress::Tool(BarTool::Lock))));
    app.press(Action::NavMenu(MenuPress::Tool(BarTool::Lock)));
    assert!(!app.holds_secret(), "the menu's Lock did not lock");
}

#[test]
fn the_pi_panel_shows_homes_menu_and_no_bar() {
    // 480 × 640 at 286 dpi: 268 × 358 dp.
    let mut app = on(480, 640, 286);
    assert_eq!(app.nav(), Nav::Menu);
    // Wallets first, then Check and the guide: buttons on the menu itself,
    // the guide further down it.
    assert!(at(&mut app, Action::Nav(Screen::Check)).is_some());
    assert!(at(&mut app, Action::Nav(Screen::WalletsStart)).is_some());
    for _ in 0..20 {
        if at(&mut app, Action::Guide).is_some() {
            break;
        }
        app.event(Event::Scroll {
            x: 134,
            y: 300,
            dy: 120,
        });
    }
    assert!(at(&mut app, Action::Guide).is_some());
    assert!(
        !app.offers(Action::NavMenu(MenuPress::Open(None))),
        "no top app bar"
    );
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
        if screen == Screen::CreateVault {
            // New vault is a step of the flow container: no bar, Back leaves.
            assert!(!bar_shown(&mut app), "the bar on New vault");
            app.press(Action::Nav(Screen::Home));
        }
        // The bar stays on every other destination.
        let (_, y) = at(&mut app, Action::Nav(Screen::Home)).expect("the bar");
        assert!(y >= 800 - 80);
    }
    assert!(
        at(&mut app, Action::Nav(Screen::Check)).is_none_or(|p| p.1 < 800 - 80),
        "Check is not on the bar"
    );
    // Settings is in the top app bar's ⋮ menu.
    let more = at(&mut app, Action::NavMenu(MenuPress::Open(None))).expect("the ⋮ menu");
    assert!(more.1 < 64);
    tap(&mut app, more);
    assert_eq!(app.sheet, Some(Sheet::NavMenu(None)));
    let settings =
        at(&mut app, Action::NavMenu(MenuPress::Go(Screen::Settings))).expect("Settings");
    tap(&mut app, settings);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Settings);
}

/// The rail at a window's left: the five destinations, Settings
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
    let home = in_rail(&mut app, Action::Nav(Screen::Home));
    let check = in_rail(&mut app, Action::Nav(Screen::Check));
    let wallets = in_rail(&mut app, Action::Nav(Screen::WalletsStart));
    assert!(home.1 < check.1 && check.1 < wallets.1, "Check under Home");
    tap(&mut app, check);
    assert_eq!(app.screen, Screen::Check);
    // The guide is the top app bar's ?, not a destination of the rail.
    let p = at(&mut app, Action::Guide).expect("the ? in the top app bar");
    assert!(
        p.1 < 64 && p.0 > 80,
        "the ? is not in the top app bar: {p:?}"
    );
    tap(&mut app, p);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    app.press(Action::Cancel);
    let p = in_rail(&mut app, Action::Nav(Screen::Settings));
    tap(&mut app, p);
    assert_eq!(app.screen, Screen::Settings);
    // Lock and Power off are the top app bar's power button's, not the rail's.
    let p = at(&mut app, Action::NavMenu(MenuPress::Power)).expect("the power button");
    assert!(
        p.1 < 64,
        "the power button is not in the top app bar: {p:?}"
    );
    tap(&mut app, p);
    let p =
        at(&mut app, Action::NavMenu(MenuPress::Tool(BarTool::Lock))).expect("Lock in the menu");
    tap(&mut app, p);
    assert!(
        app.sheet == Some(Sheet::LockAsk) || !app.holds_secret(),
        "Lock did nothing"
    );
}

#[test]
fn a_half_screen_window_shows_the_rail_with_settings_and_lock() {
    the_rail_offers_and_opens(760, 1000);
}

#[test]
fn a_seven_inch_panel_sized_window_shows_the_rail_with_settings_and_lock() {
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
        // A vault file in Files: Vaults shows its page, not New vault.
        app.storage(StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![(
                "vault.kdbx".to_string(),
                testkit::test_vault().unwrap(),
            )]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
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

/// Whether the bar's Files button is drawn at the foot of a 400 × 800 window.
fn bar_shown(app: &mut Faraday) -> bool {
    at(app, Action::Nav(Screen::Files)).is_some_and(|p| p.1 >= 800 - 80)
}

#[test]
fn a_tools_step_hides_the_bar_and_its_back_returns_it_on_the_page_it_began_from() {
    let mut app = on(400, 800, 160);
    key_in(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    assert!(bar_shown(&mut app), "the bar on Tools");
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85);
    assert!(!bar_shown(&mut app), "the bar on a step of Child seeds");
    // The step's top bar keeps its ⋮ menu.
    let more = more_menu(&mut app);
    let p = at(&mut app, more).expect("the ⋮ on the step");
    assert!(p.1 < 64);
    tap(&mut app, p);
    assert!(app.sheet.is_some(), "the step's ⋮ opens its menu");
    app.event(Event::Key(Key::Escape));
    assert_eq!(app.sheet, None, "Escape closes the menu");
    let _ = at(&mut app, Action::Nav(Screen::Catalog)).expect("the step's Back");
    app.press(Action::Nav(Screen::Catalog));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(bar_shown(&mut app), "the bar back on Tools");
}

#[test]
fn a_flows_step_hides_the_bar_and_home_has_it() {
    let mut app = on(400, 800, 160);
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    assert!(!bar_shown(&mut app), "the bar on a flow's step");
    app.press(Action::Nav(Screen::Home));
    assert!(bar_shown(&mut app), "the bar on Home");
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
            inbox: faraday_core::files_from(files),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::Nav(Screen::Catalog));
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
fn files_carries_a_badge_until_a_boot_stick_holding_a_file_is_visited() {
    let mut app = on(760, 1000, 160);
    // The boot stick, at its first look, leaves Home on screen: it waits.
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("sda1@sda#1"),
        label: faraday_text::Text::of("FARADAY"),
        boot: true,
        files: vec![(faraday_text::Text::of("inheritance.psbt"), 100)],
        vaults: Vec::new(),
    }]));
    // The file on it is offered for import on Files; Later leaves for Home
    // with Files' badge still showing.
    assert_eq!(app.screen, Screen::Files);
    assert!(app.import_on_show());
    app.press(Action::Import(
        faraday_core::boot_import::ImportAction::Later,
    ));
    let _ = app.frame();
    let drawn = dots(&mut app);
    let shown = match app.nav() {
        // The expanded rail: on its Files row, in the rail's width.
        Nav::ExpandedRail => drawn.iter().any(|d| d.0 + d.2 < 240),
        _ => {
            let files = app.hit_box(Action::Nav(Screen::Files)).expect("Files");
            drawn.iter().any(|d| within(*d, files))
        }
    };
    assert!(shown, "{drawn:?}");
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Nav(Screen::Home));
    assert!(dots(&mut app).is_empty());
}

/// The app on a window with the desktop app's or the stick's power words.
fn on_app(online: bool, width: u16, height: u16) -> Faraday {
    let mut app = testkit::started();
    app.online = online;
    app.shell_sticks = online;
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
    let _ = app.frame();
    app
}

/// The power button pressed where it stands.
fn open_power_menu(app: &mut Faraday) {
    let p = at(app, Action::NavMenu(MenuPress::Power)).expect("the power button");
    assert!(
        p.1 < 64,
        "the power button is not in the top app bar: {p:?}"
    );
    tap(app, p);
    assert!(matches!(app.sheet, Some(Sheet::PowerMenu(_))));
}

#[test]
fn the_power_button_opens_a_menu_with_lock_restart_and_power_off() {
    for (w, h) in [(1280, 800), (1024, 600), (760, 1000), (400, 800)] {
        let mut app = on_app(false, w, h);
        key_in(&mut app);
        assert!(
            !app.offers(Action::LockAsk) && !app.offers(Action::PowerAsk),
            "{w}x{h}"
        );
        open_power_menu(&mut app);
        let y = |app: &mut Faraday, t| at(app, Action::NavMenu(MenuPress::Tool(t))).map(|p| p.1);
        let lock = y(&mut app, BarTool::Lock).expect("Lock");
        let restart = y(&mut app, BarTool::Restart).expect("Restart");
        let off = y(&mut app, BarTool::Power).expect("Power off");
        assert!(
            lock < restart && restart < off,
            "{w}x{h}: Lock, Restart, Power off in order"
        );
    }
}

#[test]
fn the_power_menu_has_lock_only_while_something_is_held() {
    let mut app = on_app(false, 1280, 800);
    open_power_menu(&mut app);
    let _ = app.frame();
    assert!(!app.offers(Action::NavMenu(MenuPress::Tool(BarTool::Lock))));
    assert!(app.offers(Action::NavMenu(MenuPress::Tool(BarTool::Power))));
}

#[test]
fn lock_from_the_power_menu_locks() {
    let mut app = on_app(false, 1280, 800);
    key_in(&mut app);
    open_power_menu(&mut app);
    let p = at(&mut app, Action::NavMenu(MenuPress::Tool(BarTool::Lock))).expect("Lock");
    tap(&mut app, p);
    assert!(!app.holds_secret(), "the menu's Lock did not lock");
}

#[test]
fn power_off_and_restart_from_the_power_menu_open_the_power_sheet() {
    for tool in [BarTool::Power, BarTool::Restart] {
        let mut app = on_app(false, 1280, 800);
        key_in(&mut app);
        open_power_menu(&mut app);
        let p = at(&mut app, Action::NavMenu(MenuPress::Tool(tool))).expect("the item");
        tap(&mut app, p);
        assert_eq!(app.sheet, Some(Sheet::Power), "{tool:?}");
    }
}

#[test]
fn the_desktop_apps_power_menu_has_lock_and_quit_and_no_restart() {
    let mut app = on_app(true, 1280, 800);
    key_in(&mut app);
    open_power_menu(&mut app);
    let _ = app.frame();
    assert!(crate::common::has(&mut app, "Quit"));
    assert!(crate::common::has(&mut app, "Lock"));
    assert!(!crate::common::has(&mut app, "Restart"));
    assert!(!crate::common::has(&mut app, "Power off"));
}

#[test]
fn the_sidebar_has_no_lock_or_power_off_button() {
    let mut app = on_app(false, 1280, 800);
    key_in(&mut app);
    let _ = app.frame();
    assert_eq!(app.nav(), Nav::ExpandedRail);
    assert!(!app.offers(Action::LockAsk) && !app.offers(Action::PowerAsk));
}

#[test]
fn the_pi_panels_menu_keeps_its_lock_and_power_off_rows() {
    let mut app = on(480, 640, 286);
    key_in(&mut app);
    assert_eq!(app.nav(), Nav::Menu);
    let mut seen = (false, false);
    for _ in 0..30 {
        let _ = app.frame();
        seen.0 |= app.offers(Action::LockAsk);
        seen.1 |= app.offers(Action::PowerAsk);
        app.event(Event::Scroll {
            x: 134,
            y: 300,
            dy: 120,
        });
    }
    assert_eq!(seen, (true, true), "Lock and Power off on Home's menu");
}
