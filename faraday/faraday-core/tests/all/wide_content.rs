//! Wide content on large and extra-large windows (`docs/LAYOUT.md` §14,
//! §17): Home's tiles start beside the navigation and run across the
//! window, a wallet's buttons stand under its chart, and Explore keeps
//! its reading column, its long values wrapping between groups.

use crate::common::process;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// The expanded rail's width beside every wide page.
const RAIL: i32 = 240;

fn window(app: &mut Faraday, width: u16, height: u16) {
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
}

/// Where `text` is drawn, each place: (x, y, w, h).
fn boxes(app: &mut Faraday, text: &str) -> Vec<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == text)
        .map(|(_, r)| r)
        .collect()
}

/// The rectangle of what `a` presses: (x, y, w, h).
fn hit(app: &mut Faraday, a: Action) -> (i32, i32, i32, i32) {
    let _ = app.frame();
    let (x, y, w, h) = app.hit_box(a).expect("drawn");
    (i32::from(x), i32::from(y), i32::from(w), i32::from(h))
}

#[test]
fn homes_tiles_start_beside_the_navigation_and_stand_in_one_row() {
    let tiles = [
        Action::CreateWallet,
        Action::RestoreWallet,
        Action::Vault(faraday_core::vault::VaultAction::Create),
    ];
    for (w, h) in [(1920u16, 1080u16), (2560, 1440)] {
        let mut app = Faraday::new();
        window(&mut app, w, h);
        app.press(Action::Nav(Screen::Home));
        let placed: Vec<_> = tiles.iter().map(|a| hit(&mut app, *a)).collect();
        let first = placed[0];
        // The first tile stands beside the rail, not in a page centred in
        // the window.
        assert!(first.0 - RAIL < 80, "{w}: the first tile at {first:?}");
        // The ways in side by side, each as wide as the next, none
        // past the window's right margin.
        assert!(
            placed.iter().all(|t| t.1 == first.1 && t.2 == first.2),
            "{w}: one row of equal tiles: {placed:?}"
        );
        let end = placed.iter().map(|t| t.0 + t.2).max().unwrap_or(0);
        assert!(end <= i32::from(w) - 24, "{w}: the row ends at {end}");
    }
}

/// The test stick's wallets loaded, at `width` × `height`.
fn with_wallets(width: u16, height: u16) -> Faraday {
    let files: Vec<(String, Vec<u8>)> = testkit::files()
        .expect("the test stick's files")
        .into_iter()
        .filter(|(n, _)| n.ends_with("-wallet.txt"))
        .collect();
    let names: Vec<String> = files.iter().map(|(n, _)| n.clone()).collect();
    let mut app = process(files, Vec::new(), Vec::new());
    window(&mut app, width, height);
    for n in names.iter().take(3) {
        let i = app
            .inbox()
            .iter()
            .position(|it| &it.name == n)
            .expect("the wallet's file");
        app.press(Action::LoadWallet(i));
    }
    assert!(!app.session.wallets.is_empty(), "a wallet loaded");
    app
}

#[test]
fn a_wallets_buttons_stand_under_its_chart_on_a_big_window() {
    for (w, h) in [(1920u16, 1080u16), (2560, 1440)] {
        let mut app = with_wallets(w, h);
        app.press(Action::Nav(Screen::Wallets));
        for i in 0..app.session.wallets.len() {
            app.press(Action::PickWallet(i));
            let shown = hit(&mut app, Action::QrWallet(i));
            let address = boxes(&mut app, "First receive address · 0/0")
                .into_iter()
                .map(|b| b.1)
                .max()
                .expect("the first receive address's label");
            // Under the chart and the address it ends with, not at the
            // card's foot: the window has room to spare below them.
            assert!(
                shown.1 > address,
                "{w}: wallet {i}'s buttons at {shown:?} under the address at {address}"
            );
            assert!(
                shown.1 + shown.3 < i32::from(h) - 160,
                "{w}: wallet {i}'s buttons at {shown:?} near its content, not the foot"
            );
            // The chart stands over the reading column: the wallet's node,
            // the lowest place its name is drawn, starts within it.
            let name = app.session.wallets[i].name.clone();
            let node = boxes(&mut app, &name)
                .into_iter()
                .max_by_key(|b| b.1)
                .expect("the chart's wallet node");
            assert!(
                node.0 < shown.0 + 720,
                "{w}: wallet {i}'s chart node at {node:?}, the column from {}",
                shown.0
            );
        }
    }
}

fn exploring(w: u16, h: u16) -> Faraday {
    let mut app = testkit::started();
    window(&mut app, w, h);
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
fn explore_keeps_its_column_centred_and_wraps_long_values_between_groups() {
    for (w, h) in [(1920u16, 1080u16), (2560, 1440)] {
        let mut app = exploring(w, h);
        let _ = app.frame();
        // The top app bar names the page; the column starts at its labels.
        let label = boxes(&mut app, "Path")
            .into_iter()
            .filter(|b| b.0 > RAIL)
            .min_by_key(|b| b.1)
            .expect("the path's label");
        // The 720 dp column's middle is the room's beside the rail.
        let middle = label.0 + 720 / 2;
        let room = RAIL + (i32::from(w) - RAIL) / 2;
        assert!(
            (middle - room).abs() <= 2,
            "{w}: the column's middle {middle}, the room's {room}"
        );
        // The address in groups of four, on one line in the column.
        let grouped = "bc1q cr8t e4kr 609g cawu tmrz a0j4 xv80 jy8z 306f yu";
        let line = boxes(&mut app, grouped);
        assert_eq!(line.len(), 1, "{w}: the address in groups, whole");
        // The extended key wraps between groups: every line of it ends
        // with a whole group, and none runs past the column.
        let _ = app.frame();
        let xpub_lines: Vec<(String, (i32, i32, i32, i32))> = app
            .drawn_text_boxes()
            .into_iter()
            .filter(|(t, _)| t.starts_with("xpub") || t.starts_with("zpub"))
            .collect();
        assert!(!xpub_lines.is_empty(), "{w}: the extended key drawn");
        for (t, b) in &xpub_lines {
            assert!(
                t.split(' ').all(|g| g.chars().count() <= 4),
                "{w}: {t:?} in groups of four"
            );
            assert!(b.0 + b.2 <= label.0 + 720, "{w}: {t:?} at {b:?}");
        }
    }
}
