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
fn tools_with_a_psbt_waiting_lists_files_before_keys() {
    let mut app = with_psbt(1366, 768);
    common::add_key(&mut app, faraday_core::testkit::TEST_SEEDS[0].0);
    app.press(Action::Nav(Screen::Catalog));
    let (_, files_y) = at(&mut app, EN.session_group_files);
    let (_, keys_y) = at(&mut app, EN.session_group_keys);
    assert!(files_y < keys_y, "the waiting PSBT's group comes first");
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
