//! The search and the pills in a flow on a wide window: they stay in the top
//! bar and work, a search typed over a step shows its results alone and
//! keeps the step, and only a press on a result leaves the flow, kept in
//! progress (`docs/WORKSPACE.md` §4, §16; F331).

use faraday_core::create::Source;
use faraday_core::edit::Here;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, DisplayInfo, Event, Key};
use std::collections::BTreeSet;

fn wide() -> Faraday {
    at(1366, 768)
}

fn at(width: u16, height: u16) -> Faraday {
    let mut app = testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
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

fn making() -> Faraday {
    let mut app = wide();
    app.press(Action::CreateWallet);
    let _ = app.frame();
    app
}

fn texts(app: &mut Faraday) -> BTreeSet<String> {
    let _ = app.frame();
    app.drawn_texts()
        .into_iter()
        .map(|t| t.as_str().to_string())
        .collect()
}

fn type_in(app: &mut Faraday, s: &str) {
    app.press(Action::TypeHere(Here::HomeFind));
    for c in s.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn in_a_flow_the_search_and_the_pills_stand_in_the_bar_and_work() {
    let mut app = making();
    assert_eq!(app.screen, Screen::Create);
    let _ = app.frame();
    // The field stands in the bar, and the pills or, where the flow's
    // heading needs the room, the one Start button.
    let ways = app
        .hit_box(Action::RestoreWallet)
        .or_else(|| app.hit_box(Action::NavMenu(faraday_core::nav::MenuPress::Start)));
    for (a, b) in [
        (
            Action::TypeHere(Here::HomeFind),
            app.hit_box(Action::TypeHere(Here::HomeFind)),
        ),
        (Action::RestoreWallet, ways),
    ] {
        let b = b.unwrap_or_else(|| panic!("{a:?} not in the flow"));
        assert!(b.1 + b.3 <= 64, "{a:?} stands in the bar: {b:?}");
    }
    app.press(Action::RestoreWallet);
    assert_eq!(app.screen, Screen::Restore);
}

#[test]
fn new_wallet_goes_back_to_the_wallet_being_made() {
    let mut app = making();
    testkit::create_set_slot(&mut app, 0, Source::Here([1, 2, 3, 4]));
    app.press(Action::RestoreWallet);
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    let slot = app.create().map(|c| c.slots[0].clone());
    assert_eq!(slot, Some(Source::Here([1, 2, 3, 4])));
}

#[test]
fn a_search_in_a_flow_keeps_the_step_and_shows_only_its_results() {
    let mut app = making();
    let step = texts(&mut app);
    type_in(&mut app, "s");
    let at_once = texts(&mut app);
    assert_eq!(app.screen, Screen::Create);
    let gone: Vec<_> = step.difference(&at_once).collect();
    assert!(
        gone.len() >= 3,
        "the step is not drawn under the results: {gone:?}"
    );
    type_in(&mut app, "ign");
    assert!(texts(&mut app).contains("Sign a transaction"));
    assert_eq!(testkit::home_find(&app), "sign");
}

#[test]
fn escape_brings_the_step_back_and_a_result_leaves_it_in_progress() {
    let mut app = making();
    let step = texts(&mut app);
    type_in(&mut app, "sign");
    let _ = app.frame();
    app.event(Event::Key(Key::Escape));
    let back = texts(&mut app);
    assert_eq!(app.screen, Screen::Create);
    assert!(testkit::home_find(&app).is_empty());
    assert!(step.is_subset(&back) || step.intersection(&back).count() * 2 > step.len());
    type_in(&mut app, "explore");
    let _ = app.frame();
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name.starts_with("Explore"))
        .expect("the tile") as u8;
    app.press(Action::Catalog(tile));
    assert_ne!(app.screen, Screen::Create);
    assert!(app.create().is_some(), "the wallet being made is kept");
}

/// The search and the pills are a constant on the screen: one rectangle each
/// on every menu page, and a menu change leaves them where they were.
#[test]
fn the_search_and_the_pills_stand_still_on_every_menu_page() {
    let pages = [
        Screen::Home,
        Screen::Catalog,
        Screen::Check,
        Screen::WalletsStart,
        Screen::Files,
        Screen::Vaults,
        Screen::Settings,
    ];
    let parts = [
        Action::TypeHere(Here::HomeFind),
        Action::CreateWallet,
        Action::RestoreWallet,
        Action::Nav(Screen::SpendBackup),
    ];
    for (w, h) in [(1366, 768), (1920, 1080)] {
        let mut app = at(w, h);
        let mut first: Option<Vec<_>> = None;
        for page in pages {
            app.press(Action::Nav(page));
            let _ = app.frame();
            // Vaults with no vault opens the flow that makes one.
            if app.screen != page {
                assert_eq!(page, Screen::Vaults, "{w}x{h}: {page:?} not reached");
                continue;
            }
            let boxes: Vec<_> = parts.iter().map(|a| app.hit_box(*a)).collect();
            assert!(
                boxes.iter().all(Option::is_some),
                "{w}x{h} {page:?}: {boxes:?}"
            );
            match &first {
                None => first = Some(boxes),
                Some(f) => assert_eq!(f, &boxes, "{w}x{h} {page:?} moves them"),
            }
        }
    }
}

#[test]
fn one_search_serves_every_menu_page_and_a_page_press_clears_it() {
    let mut app = wide();
    app.press(Action::Nav(Screen::Settings));
    type_in(&mut app, "send");
    assert_eq!(testkit::home_find(&app), "send");
    app.press(Action::Nav(Screen::Check));
    assert!(testkit::home_find(&app).is_empty());
}
