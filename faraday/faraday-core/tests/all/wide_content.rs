//! Wide content on large and extra-large windows (`docs/LAYOUT.md` §14,
//! §17): a wallet's buttons stand under its chart, and Explore keeps
//! its reading column, its long values wrapping between groups.

use crate::common::process;
use faraday_core::testkit::Plain;
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
            .position(|it| it.name.as_str() == n.as_str())
            .expect("the wallet's file");
        testkit::load_wallet(&mut app, i);
    }
    assert!(!app.session.wallets.is_empty(), "a wallet loaded");
    // Every key checked: the wallet node has no key-check line, so the
    // card's address stays in view at this window size.
    for w in 0..app.session.wallets.len() {
        for k in 0..app.session.wallets[w].key_checks().len() {
            app.session.tick_key(w, k);
        }
    }
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

/// On a 1280 × 800 window a wallet card's first receive address is drawn
/// whole on one line, never split where its first line could be read as
/// the whole address.
#[test]
fn a_wallet_cards_first_receive_address_is_drawn_on_one_line() {
    let mut app = with_wallets(1280, 800);
    app.press(Action::Nav(Screen::Wallets));
    for i in 0..app.session.wallets.len() {
        app.press(Action::PickWallet(i));
        let _ = app.frame();
        let texts: Vec<String> = app
            .drawn_text_boxes()
            .plain()
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        let at = texts
            .iter()
            .position(|t| t == "First receive address · 0/0")
            .expect("the first receive address's label");
        // The first line after the label that starts as an address: a
        // caution over it, where keys are not checked, is not it.
        // The search stops at the next label (a text with " · " in it) or
        // at the end of the page's texts after the label's own.
        let end = texts[at + 1..]
            .iter()
            .position(|t| t.contains(" · "))
            .map_or(texts.len(), |n| at + 1 + n);
        let line: String = texts[at + 1..end]
            .iter()
            .map(|t| t.split_whitespace().collect::<String>())
            .find(|t| t.starts_with("bc1") || t.starts_with("tb1"))
            .expect("the address after its label");
        assert!(line.starts_with("bc1") || line.starts_with("tb1"), "{line}");
        assert!(
            line.len() == 42 || line.len() == 62,
            "wallet {i}: {line} is the whole address"
        );
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
    faraday_core::testkit::keep_published(&mut app);
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
        let grouped = "tb1q 6rz2 8mcf axtm d6v7 89l9 rrlr usdp rr9p qcpv kl";
        let line = boxes(&mut app, grouped);
        assert_eq!(line.len(), 1, "{w}: the address in groups, whole");
        // The extended key wraps between groups: every line of it ends
        // with a whole group, and none runs past the column.
        let _ = app.frame();
        let xpub_lines: Vec<(String, (i32, i32, i32, i32))> = app
            .drawn_text_boxes()
            .plain()
            .into_iter()
            .filter(|(t, _)| {
                t.starts_with("xpub")
                    || t.starts_with("zpub")
                    || t.starts_with("tpub")
                    || t.starts_with("vpub")
            })
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

/// The room a page is centred in: beside the rail, or the whole window
/// for a held app (`docs/LAYOUT.md` §15), as (start, width).
fn room(w: u16, held: bool) -> (i32, i32) {
    if held {
        (0, i32::from(w))
    } else {
        (RAIL, i32::from(w) - RAIL)
    }
}

/// Where Wallets with none loaded starts its column at the window
/// sizes of a big monitor: the column's middle is the room's,
/// held and not held.
#[test]
fn a_page_meant_to_be_centred_is_centred_in_its_room_held_or_not() {
    for held in [false, true] {
        for (w, h) in [
            (1366u16, 768u16),
            (1600, 900),
            (1920, 1080),
            (2000, 1090),
            (2560, 1440),
        ] {
            let mut app = testkit::started();
            app.online = true;
            app.held = held;
            window(&mut app, w, h);
            app.press(Action::Nav(Screen::WalletsStart));
            let at = boxes(&mut app, "Your wallets")
                .into_iter()
                .filter(|b| b.0 > 0)
                .min_by_key(|b| b.1)
                .expect("the section's label");
            let (start, width) = room(w, held);
            // The page is the menu page's width, or the room less a page
            // inset at each side where that is narrower, and starts at its
            // label.
            let page = 1112.min(width - 96);
            let middle = at.0 + page / 2;
            let want = start + width / 2;
            assert!(
                (middle - want).abs() <= 4,
                "{w}x{h} held {held}: middle {middle}, room's {want} ({at:?})"
            );
        }
    }
}
