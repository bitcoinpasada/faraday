//! Tools and Check on the wide layout: the page's tile under This session
//! where the window has the height, else the pane at the left of the tile,
//! level, in the page's own mode.

use super::common;
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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

/// Where the line `text` starts: its x and y.
fn at(app: &mut Faraday, text: &str) -> (i32, i32) {
    let _ = app.frame();
    let found = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("{text} not drawn"));
    (found.1.0, found.1.1)
}

#[test]
fn check_on_a_tall_window_stands_under_this_session() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Check));
    let (card_x, card_y) = at(&mut app, EN.home_box_heading);
    let (group_x, group_y) = at(&mut app, EN.check_group_wallet);
    assert!(group_y > card_y, "the tile is under the card");
    assert!(
        group_x < card_x + 360,
        "and as wide as the card, not beside it"
    );
}

#[test]
fn check_on_a_short_window_stands_beside_this_session() {
    let mut app = sized(1366, 768);
    app.press(Action::Nav(Screen::Check));
    let (card_x, _) = at(&mut app, EN.home_box_heading);
    let (group_x, group_y) = at(&mut app, EN.check_group_wallet);
    assert!(group_x > card_x + 360, "the tile is at the pane's right");
    assert!(group_y < 200, "level with the pane's top");
}

#[test]
fn tools_with_a_key_lists_the_keys_in_its_pane() {
    let mut app = sized(1366, 768);
    common::add_key(&mut app, faraday_core::testkit::TEST_SEEDS[0].0);
    app.press(Action::Nav(Screen::Catalog));
    let texts = common::texts(&mut app);
    assert!(texts.iter().any(|t| t == EN.session_group_keys));
}

fn with_psbt(width: u16, height: u16) -> Faraday {
    use faraday_core::StorageEvent;
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = sized(width, height);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("spend.psbt".to_string(), psbt)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app
}

#[test]
fn tools_with_a_psbt_waiting_has_it_in_the_band_under_the_keys() {
    let mut app = with_psbt(1366, 768);
    common::add_key(&mut app, faraday_core::testkit::TEST_SEEDS[0].0);
    app.press(Action::Nav(Screen::Catalog));
    let (_, waiting_y) = at(&mut app, EN.session_group_waiting);
    let (_, keys_y) = at(&mut app, EN.session_group_keys);
    assert!(keys_y < waiting_y, "Waiting is the band at the pane's foot");
}

#[test]
fn tools_with_no_psbt_lists_keys_first() {
    let mut app = sized(1366, 768);
    common::add_key(&mut app, faraday_core::testkit::TEST_SEEDS[0].0);
    app.press(Action::Nav(Screen::Catalog));
    let (_, keys_y) = at(&mut app, EN.session_group_keys);
    assert!(keys_y < 200, "Keys is the pane's first group");
}

#[test]
fn a_flow_and_the_page_pair_stand_at_the_same_x() {
    let mut app = with_psbt(1366, 768);
    app.press(Action::Nav(Screen::Check));
    let (page_x, _) = at(&mut app, EN.home_box_heading);
    app.press(Action::StartSpend(0));
    let _ = app.frame();
    let flow_pane_x = app
        .drawn_text_boxes()
        .into_iter()
        .map(|(_, b)| b.0)
        .filter(|x| *x > 250)
        .min()
        .expect("the flow draws its pane");
    assert!(
        (page_x - flow_pane_x).abs() <= 8,
        "the pane's lines start where the flow's pane lines do"
    );
}

#[test]
fn tools_on_a_wide_window_fills_the_room_in_columns() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Catalog));
    let (pane_x, _) = at(&mut app, EN.home_box_heading);
    let (first_x, _) = at(&mut app, EN.catalog_common);
    let (last_x, _) = at(&mut app, EN.catalog_vaults_group);
    assert!(
        pane_x < 240 + 24 + 40,
        "the pane stands a page margin from the navigation"
    );
    assert!(
        last_x - first_x >= 2 * 360,
        "the groups stand in at least two columns, here three"
    );
    assert!(last_x < 1920, "and none runs off the window");
}

#[test]
fn tools_on_a_narrow_window_stays_in_one_column_where_the_flow_pair_is() {
    let mut app = sized(1366, 768);
    app.press(Action::Nav(Screen::Catalog));
    let (first_x, _) = at(&mut app, EN.catalog_common);
    assert!(
        (640..720).contains(&first_x),
        "one column, at the flow pair's card: {first_x}"
    );
}

fn with_files(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    use faraday_core::StorageEvent;
    let mut app = sized(1366, 768);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app
}

fn spending_descriptor() -> Vec<u8> {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    kit.descriptor.into_bytes()
}

#[test]
fn a_wallet_file_row_on_tools_carries_the_buttons_its_menu_leads_with() {
    use faraday_core::file_menu::{FilePress, Row};
    let mut app = with_files(vec![("spending-wallet.txt", spending_descriptor())]);
    app.press(Action::Nav(Screen::Check));
    let _ = app.frame();
    assert!(
        common::texts(&mut app)
            .iter()
            .any(|t| t == EN.inbox_actions_load_wallet),
        "the row says Load"
    );
    assert!(app.offers(Action::LoadWallet(0)), "and the button is drawn");
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::File(FilePress::Menu(Row::In(0))));
    let menu = app.files_menu().menu.as_ref().expect("its menu");
    assert_eq!(
        menu.lines.first().map(|(l, _)| l.as_str()),
        Some(EN.inbox_actions_load_wallet)
    );
}

#[test]
fn a_file_with_no_action_offered_draws_no_button_on_the_pane() {
    let mut app = with_files(vec![("notes.txt", b"just some words".to_vec())]);
    app.press(Action::Nav(Screen::Check));
    let texts = common::texts(&mut app);
    assert!(texts.iter().any(|t| t == "notes.txt"), "the row is there");
    assert!(
        !app.offers(Action::LoadKey(0)) && !app.offers(Action::LoadWallet(0)),
        "and carries no button"
    );
    assert!(
        app.offers(Action::Nav(Screen::Files)),
        "the row itself opens Files"
    );
}

#[test]
fn a_loaded_wallets_file_row_draws_no_button() {
    let mut app = with_files(vec![("spending-wallet.txt", spending_descriptor())]);
    app.press(Action::LoadWallet(0));
    app.press(Action::Nav(Screen::Check));
    let _ = app.frame();
    assert!(!app.offers(Action::LoadWallet(0)));
}

fn type_on_home(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    let _ = app.frame();
    let _ = app.frame();
}

#[test]
fn home_search_results_stand_under_this_session_where_the_window_is_tall() {
    for (w, h) in [(1920, 1080), (1366, 1000)] {
        let mut app = sized(w, h);
        type_on_home(&mut app, "send");
        let (card_x, card_y) = at(&mut app, EN.home_box_heading);
        let (tile_x, tile_y) = at(&mut app, "Sign a transaction");
        assert!(tile_y > card_y, "{w}x{h}: the results are under the card");
        assert!(tile_x < card_x + 360, "{w}x{h}: and as wide as the card");
    }
}

#[test]
fn home_search_results_stand_beside_this_session_where_the_window_is_short() {
    let mut app = sized(1366, 768);
    type_on_home(&mut app, "e");
    let (card_x, _) = at(&mut app, EN.home_box_heading);
    let (tile_x, tile_y) = at(&mut app, "Sign a transaction");
    assert!(tile_x > card_x + 360, "the results are at the pane's right");
    assert!(tile_y < 200, "level with the pane's top");
}

#[test]
fn leaving_the_search_puts_the_page_back() {
    let mut app = sized(1920, 1080);
    type_on_home(&mut app, "send");
    assert!(
        common::texts(&mut app)
            .iter()
            .any(|t| t == "Sign a transaction")
    );
    for _ in 0..4 {
        app.event(Event::Key(faraday_shell_api::Key::Backspace));
    }
    let _ = app.frame();
    let _ = app.frame();
    let texts = common::texts(&mut app);
    assert!(!texts.iter().any(|t| t == "Sign a transaction"));
    assert!(texts.iter().any(|t| t == EN.home_box_heading));
}

/// Where the top bar's (64 px tall) search hint stands, and on Home the first pill.
fn search_places(app: &mut Faraday) -> Vec<(i32, i32)> {
    let _ = app.frame();
    let boxes = app.drawn_text_boxes();
    [EN.home_search_hint, EN.home_ways_new_short]
        .iter()
        .filter_map(|want| boxes.iter().find(|(t, b)| t == want && b.1 < 64))
        .map(|(_, b)| (b.0, b.1))
        .collect()
}

#[test]
fn the_search_and_pills_stand_still_as_the_guide_opens() {
    for (w, h) in [(1366, 768), (1920, 1080)] {
        for screen in [Screen::Home, Screen::Catalog] {
            let mut app = sized(w, h);
            app.press(Action::Nav(screen));
            let closed = search_places(&mut app);
            app.press(Action::Guide);
            // Below 1600 the guide is a sheet over the page: the page is
            // compared once it is closed again.
            if w < 1600 {
                app.press(Action::GuideClose);
            }
            let open = search_places(&mut app);
            assert!(!closed.is_empty(), "{screen:?} at {w}x{h}: search drawn");
            assert_eq!(closed, open, "{screen:?} at {w}x{h}");
        }
    }
}

/// Whether the tile stands under This session's card, not beside it.
fn stacked(app: &mut Faraday) -> bool {
    let (card_x, card_y) = at(app, EN.home_box_heading);
    let (group_x, group_y) = at(app, EN.check_group_wallet);
    group_y > card_y && group_x < card_x + 360
}

#[test]
fn check_stays_beside_this_session_after_a_check_is_left_until_home() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Check));
    assert!(stacked(&mut app));
    app.press(Action::Nav(Screen::Tools));
    app.press(Action::Back);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Check);
    assert!(!stacked(&mut app), "Back lands on the pair");
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Nav(Screen::Check));
    assert!(!stacked(&mut app), "Files and back keeps the pair");
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::Check));
    assert!(stacked(&mut app), "Home stacks it again");
}

#[test]
fn tools_stand_beside_too_once_a_tool_was_left() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Nav(Screen::Tools));
    app.press(Action::Back);
    app.press(Action::Nav(Screen::Check));
    assert!(!stacked(&mut app), "one rule for both pages");
}

#[test]
fn a_flow_left_for_check_leaves_it_beside_even_from_home() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Tools));
    app.press(Action::Nav(Screen::Check));
    assert!(!stacked(&mut app), "a flow always stands side by side");
}

#[test]
fn wallets_left_for_tools_leaves_tools_beside() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::Nav(Screen::Check));
    assert!(
        !stacked(&mut app),
        "Wallets never stacks, and Check follows"
    );
}

#[test]
fn home_resets_what_stood_side_by_side() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Nav(Screen::Check));
    assert!(stacked(&mut app));
}

#[test]
fn settings_sets_nothing() {
    let mut app = sized(1920, 1080);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Nav(Screen::Check));
    assert!(stacked(&mut app));
}

#[test]
fn a_tools_page_that_did_not_fit_stacked_holds_the_pair_for_the_next_page() {
    let mut app = sized(1366, 768);
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    app.press(Action::Nav(Screen::Check));
    let (card_x, _) = at(&mut app, EN.home_box_heading);
    let (group_x, _) = at(&mut app, EN.check_group_wallet);
    assert!(group_x > card_x + 360, "beside");
}

#[test]
fn search_results_follow_a_page_that_stood_side_by_side() {
    let results_beside = |first: Screen| {
        let mut app = sized(1920, 1080);
        app.press(Action::Nav(first));
        app.press(Action::Nav(Screen::Catalog));
        type_on_home(&mut app, "send");
        let (card_x, _) = at(&mut app, EN.home_box_heading);
        let (tile_x, _) = at(&mut app, "Sign a transaction");
        tile_x > card_x + 360
    };
    assert!(!results_beside(Screen::Home), "from Home they stack");
    assert!(results_beside(Screen::Wallets), "after Wallets they do not");
}

#[test]
fn a_small_window_and_the_phone_are_as_they_were_after_a_tool_is_left() {
    let mut app = sized(1366, 768);
    app.press(Action::Nav(Screen::Check));
    app.press(Action::Nav(Screen::Tools));
    app.press(Action::Back);
    let (card_x, _) = at(&mut app, EN.home_box_heading);
    let (group_x, _) = at(&mut app, EN.check_group_wallet);
    assert!(group_x > card_x + 360, "beside, as it was");
}

#[test]
fn the_phone_page_is_the_same_after_a_tool_was_left() {
    let mut app = sized(390, 844);
    app.press(Action::Nav(Screen::Check));
    let first = at(&mut app, EN.check_group_wallet);
    app.press(Action::Nav(Screen::Tools));
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Check);
    assert_eq!(at(&mut app, EN.check_group_wallet), first);
}
