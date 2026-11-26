//! The Waiting band at the session pane's foot, and the groups' order
//! (`docs/WORKSPACE.md` §1, §2): Keys first, Waiting across the pane's foot
//! outside the part that scrolls, In progress a group of its own on Tools
//! and Check.

use faraday_core::in_progress::Going;
use faraday_core::strings::EN;
use faraday_core::testkit::TEST_SEEDS;
use faraday_core::{Action, Faraday, Row, Screen, rstep};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

fn sized(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
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

fn psbt() -> Vec<u8> {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    faraday_core::testkit::unsigned(&kit).unwrap().to_bytes()
}

/// A window with a key held and a PSBT waiting (the inbox's first file),
/// with `files` more files held after it.
fn with_psbt(width: u16, height: u16, files: usize) -> Faraday {
    let mut app = sized(width, height);
    super::common::add_key(&mut app, TEST_SEEDS[0].0);
    faraday_core::testkit::inbox_file(&mut app, "spend.psbt", psbt());
    for n in 0..files {
        faraday_core::testkit::inbox_file_at(
            &mut app,
            1 + n,
            &format!("note-{n}.txt"),
            b"a note".to_vec(),
        );
    }
    let _ = app.frame();
    app
}

fn go(app: &mut Faraday, page: Screen) {
    app.press(Action::Nav(page));
    let _ = app.frame();
}

/// Where the line `text` starts, if drawn: (x, y).
fn at(app: &mut Faraday, text: &str) -> Option<(i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, b)| t == text && b.0 > 240)
        .map(|(_, b)| (b.0, b.1))
}

const PAGES: [Screen; 4] = [
    Screen::Home,
    Screen::Catalog,
    Screen::Check,
    Screen::Wallets,
];

#[test]
fn the_band_stands_under_the_groups_on_the_four_pages() {
    let mut app = with_psbt(1366, 768, 0);
    for page in PAGES {
        go(&mut app, page);
        assert!(app.offers(Action::StartSpend(0)), "{page:?}: Sign");
        let (_, waiting) = at(&mut app, EN.session_group_waiting).expect("Waiting label");
        let (_, keys) = at(&mut app, EN.session_group_keys).expect("Keys label");
        assert!(keys < waiting, "{page:?}: Waiting under Keys");
    }
}

#[test]
fn the_band_is_not_drawn_with_nothing_waiting() {
    let mut app = sized(1366, 768);
    super::common::add_key(&mut app, TEST_SEEDS[0].0);
    for page in PAGES {
        go(&mut app, page);
        assert!(
            at(&mut app, EN.session_group_waiting).is_none(),
            "{page:?}: no band"
        );
    }
}

#[test]
fn the_list_scrolls_under_a_band_that_stays() {
    let mut app = with_psbt(1366, 600, 40);
    go(&mut app, Screen::Catalog);
    let band = app
        .hit_box(Action::StartSpend(0))
        .expect("Sign in the band");
    let row = app
        .hit_box(Action::RowOpen(Row::File(1)))
        .expect("a file row");
    for _ in 0..3 {
        app.event(Event::Scroll {
            x: row.0 + 40,
            y: row.1 + 10,
            dy: 200,
        });
        app.event(Event::ScrollEnd {
            x: row.0 + 40,
            y: row.1 + 10,
        });
        let _ = app.frame();
    }
    assert_ne!(
        app.hit_box(Action::RowOpen(Row::File(1))).map(|b| b.1),
        Some(row.1),
        "the list moved"
    );
    assert_eq!(
        app.hit_box(Action::StartSpend(0)),
        Some(band),
        "the band stood still"
    );
}

#[test]
fn a_flow_in_progress_is_a_group_of_its_own_on_tools_and_check() {
    let mut app = with_psbt(1366, 768, 0);
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    for page in [Screen::Catalog, Screen::Check] {
        go(&mut app, page);
        let (_, progress) = at(&mut app, EN.session_group_progress).expect("In progress");
        let (_, waiting) = at(&mut app, EN.session_group_waiting).expect("Waiting");
        let (_, keys) = at(&mut app, EN.session_group_keys).expect("Keys");
        assert!(keys < progress && progress < waiting, "{page:?}");
        assert!(app.offers(Action::Resume(Going::Restore)), "{page:?}");
    }
}

#[test]
fn keys_stand_above_wallets_on_tools_and_home() {
    let mut app = with_psbt(1366, 768, 0);
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    faraday_core::testkit::inbox_file_at(&mut app, 1, "wallet.txt", kit.descriptor.into_bytes());
    faraday_core::testkit::load_wallet(&mut app, 1);
    go(&mut app, Screen::Catalog);
    let (_, keys) = at(&mut app, EN.session_group_keys).expect("Keys");
    let (_, wallets) = at(&mut app, EN.session_group_wallets).expect("Wallets");
    assert!(keys < wallets, "Tools: Keys above Wallets");
    go(&mut app, Screen::Home);
    let (kx, ky) = at(&mut app, EN.session_group_keys).expect("Keys");
    let (wx, wy) = at(&mut app, EN.session_group_wallets).expect("Wallets");
    assert!((kx, ky) < (wx, wy), "Home: Keys first");
}

#[test]
fn the_phones_home_keeps_the_lead_at_the_top() {
    let mut app = with_psbt(480, 640, 0);
    go(&mut app, Screen::Home);
    let sign = app.where_offered(Action::StartSpend(0)).expect("the lead");
    assert!(sign.1 < 320, "the lead stands in the top half: {sign:?}");
    let _ = app.frame();
    assert!(
        !app.drawn_text_boxes()
            .iter()
            .any(|(t, _)| t == EN.session_group_waiting),
        "the phone draws no band"
    );
}

#[test]
fn a_press_on_a_band_row_opens_its_details_on_tools() {
    let mut app = with_psbt(1366, 768, 0);
    go(&mut app, Screen::Catalog);
    let (x, y, _, h) = app.hit_box(Action::RowOpen(Row::File(0))).expect("the row");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch {
            x: x + 24,
            y: y + h / 2,
            phase,
        });
    }
    let _ = app.frame();
    assert!(app.offers(Action::RowClose), "the details are open");
    assert_eq!(app.screen, Screen::Catalog);
}

#[test]
fn homes_card_shows_every_waiting_row_where_the_window_has_the_room() {
    let mut app = with_psbt(1366, 768, 0);
    for n in 1..4 {
        faraday_core::testkit::inbox_file_at(&mut app, n, &format!("spend-{n}.psbt"), psbt());
    }
    app.press(Action::RestoreWallet);
    app.press(Action::RNext(rstep::KIND));
    go(&mut app, Screen::Home);
    for n in 0..4 {
        let (_, y, _, h) = app
            .hit_box(Action::StartSpend(n))
            .unwrap_or_else(|| panic!("PSBT {n}'s Sign is on screen"));
        assert!(y + h <= 768, "PSBT {n}");
    }
    let (_, y, _, h) = app
        .hit_box(Action::Resume(Going::Restore))
        .expect("the flow's Resume is on screen");
    assert!(y + h <= 768);
}
