//! A row of the session pane on Tools opens its details in the page's
//! second pane (`docs/WORKSPACE.md` §7 point 1): the page stays lit, Back,
//! Escape and the page's tab return, a key's, a wallet's and a file's
//! details offer what they offer, a row's button stays its own press and a
//! flow begun from the details returns to them.

use faraday_core::testkit::KindText;
use faraday_core::{Action, Faraday, Row, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon about";

fn desktop() -> Faraday {
    desktop_of(768)
}

fn desktop_of(height: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
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

fn add_key(app: &mut Faraday, words: &str) -> [u8; 4] {
    let before: Vec<[u8; 4]> = app.session.keys.all().map(|k| k.fingerprint.0).collect();
    let c = faraday_core::testkit::compute();
    app.session.keys.add_words(&c, words, "Seed", None).unwrap();
    app.session
        .keys
        .all()
        .map(|k| k.fingerprint.0)
        .find(|f| !before.contains(f))
        .expect("no key added")
}

/// Tools with a key held, shown.
fn tools_with_key() -> (Faraday, [u8; 4]) {
    let mut app = desktop();
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    (app, fp)
}

fn shown(app: &mut Faraday, action: Action) -> bool {
    let _ = app.frame();
    app.offers(action)
}

/// A key's details are on show: they offer Make a wallet from the key.
fn details_of(app: &mut Faraday, fp: [u8; 4]) -> bool {
    shown(app, Action::KeyWallet(fp, 1))
}

fn touch(app: &mut Faraday, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

#[test]
fn a_key_row_opens_its_details_and_tools_stays() {
    let (mut app, fp) = tools_with_key();
    assert!(shown(&mut app, Action::RowOpen(Row::Key(fp))));
    assert!(!details_of(&mut app, fp));
    app.press(Action::RowOpen(Row::Key(fp)));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(details_of(&mut app, fp));
    assert!(
        app.offers(Action::ExploreKey(fp)),
        "Explore, the first offer"
    );
    assert!(app.offers(Action::KeyWallet(fp, 2)), "Add another key");
    assert!(app.offers(Action::RestoreWallet), "Restore");
    assert!(app.offers(Action::RowClose), "the way back");
}

#[test]
fn a_press_on_another_row_swaps_the_details() {
    let (mut app, a) = tools_with_key();
    let b = add_key(&mut app, &faraday_core::testkit::test_words("zebra"));
    app.press(Action::RowOpen(Row::Key(a)));
    assert!(details_of(&mut app, a));
    app.press(Action::RowOpen(Row::Key(b)));
    assert!(details_of(&mut app, b));
    assert!(!app.offers(Action::KeyWallet(a, 1)));
}

#[test]
fn back_the_tab_and_escape_return_the_page() {
    let (mut app, fp) = tools_with_key();
    for how in 0..2 {
        app.press(Action::RowOpen(Row::Key(fp)));
        assert!(details_of(&mut app, fp));
        match how {
            0 => {
                let at = app.where_offered(Action::RowClose).expect("the arrow");
                touch(&mut app, at);
            }
            _ => app.press(Action::Nav(Screen::Catalog)),
        }
        assert!(!details_of(&mut app, fp), "way {how}");
        assert_eq!(app.screen, Screen::Catalog);
        assert!(app.offers(Action::RowOpen(Row::Key(fp))), "way {how}");
    }
}

#[test]
fn the_key_going_away_returns_the_page() {
    let (mut app, fp) = tools_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    assert!(details_of(&mut app, fp));
    app.press(Action::RemoveKeysHold);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Catalog);
    assert!(!app.offers(Action::RowClose));
    // The same key held again is not opened by what was left.
    add_key(&mut app, WORDS);
    assert!(!details_of(&mut app, fp));
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn lock_clears_the_row_open() {
    let (mut app, fp) = tools_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    app.press(Action::Lock);
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Catalog));
    assert!(!details_of(&mut app, fp));
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn leaving_the_page_by_another_tab_clears_it() {
    let (mut app, fp) = tools_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    app.press(Action::Nav(Screen::Check));
    app.press(Action::Nav(Screen::Catalog));
    assert!(!details_of(&mut app, fp));
    // And coming back to the page by Back, not its tab.
    app.press(Action::RowOpen(Row::Key(fp)));
    app.press(Action::Nav(Screen::Check));
    app.event(Event::Back);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Catalog);
    assert!(!details_of(&mut app, fp));
}

#[test]
fn a_wallets_details_are_its_page() {
    let c = faraday_core::testkit::compute();
    let mut app = desktop();
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    faraday_core::testkit::keep_published(&mut app);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let d = format!("wpkh({}/<0;1>/*)", &*text);
    app.session
        .add_wallet_confirmed(
            &c,
            "Wallet",
            &d,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    assert!(!app.offers(Action::Backup(0)));
    app.press(Action::RowOpen(Row::Wallet(0)));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(shown(&mut app, Action::Backup(0)), "its What you can do");
}

#[test]
fn a_files_details_stand_beside_the_list() {
    let mut app = desktop();
    faraday_core::testkit::inbox_file(&mut app, "notes.bin", vec![0u8; 300]);
    app.press(Action::Nav(Screen::Catalog));
    assert!(shown(&mut app, Action::RowOpen(Row::File(0))));
    app.press(Action::RowOpen(Row::File(0)));
    let _ = app.frame();
    assert!(app.offers(Action::RowClose));
    assert_eq!(app.screen, Screen::Catalog);
}

#[test]
fn wallets_key_row_lists_add_another_key_and_restore() {
    let mut app = desktop();
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Wallets));
    let _ = app.frame();
    app.press(Action::RowOpen(Row::Key(fp)));
    assert_eq!(app.screen, Screen::Wallets);
    assert!(shown(&mut app, Action::KeyWallet(fp, 2)));
    assert!(app.offers(Action::RestoreWallet));
}

#[test]
fn leaving_a_flow_begun_from_the_details_returns_to_them() {
    let (mut app, fp) = tools_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    let _ = app.frame();
    app.press(Action::ExploreKey(fp));
    assert_eq!(app.screen, Screen::Explore);
    let _ = app.frame();
    let (x, y) = app.back_offered().expect("a way back");
    touch(&mut app, (x, y));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(details_of(&mut app, fp), "the details, as they were");
}

/// A wallet file held, on Tools: its row carries Load.
fn tools_with_wallet_file() -> Faraday {
    let mut app = desktop();
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    faraday_core::testkit::inbox_file(&mut app, "spending-wallet.txt", kit.descriptor.into_bytes());
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    app
}

#[test]
fn a_press_beside_a_rows_button_opens_the_details_and_the_button_loads() {
    let mut app = tools_with_wallet_file();
    assert!(app.offers(Action::LoadWallet(0)), "the row's button");
    let (x, y, _, h) = app.hit_box(Action::RowOpen(Row::File(0))).expect("the row");
    touch(&mut app, (x + 24, y + h / 2));
    assert!(app.offers(Action::RowClose), "the details are open");
    assert_eq!(app.screen, Screen::Catalog);
    assert!(app.session.wallets.is_empty(), "the button was not pressed");
    app.press(Action::RowClose);
    let _ = app.frame();
    let at = app
        .where_offered(Action::LoadWallet(0))
        .expect("the button");
    touch(&mut app, at);
    assert!(!app.offers(Action::RowClose), "the button is its own press");
    assert!(!app.session.wallets.is_empty(), "Load loaded the wallet");
}

#[test]
fn the_platforms_back_returns_the_page() {
    // The shell maps Escape to the platform's Back.
    let (mut app, fp) = tools_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    assert!(details_of(&mut app, fp));
    app.event(Event::Back);
    let _ = app.frame();
    assert!(!details_of(&mut app, fp));
    assert_eq!(app.screen, Screen::Catalog);
    assert!(app.offers(Action::RowOpen(Row::Key(fp))));
}

#[test]
fn leaving_a_flow_begun_from_the_details_returns_at_their_scroll() {
    let mut app = desktop_of(768);
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::RowOpen(Row::Key(fp)));
    let _ = app.frame();
    app.list_offset = 60.0;
    let _ = app.frame();
    let at = app.list_offset;
    assert!(at > 0.0, "the details scroll: {at}");
    app.press(Action::ExploreKey(fp));
    let _ = app.frame();
    let (x, y) = app.back_offered().expect("a way back");
    touch(&mut app, (x, y));
    let _ = app.frame();
    assert!(app.offers(Action::RowClose), "the details, as they were");
    assert_eq!(app.list_offset, at, "the scroll they had");
}

// ---- Home (WORKSPACE.md §7 point 1: a row of Home's card opens its details)

fn home_of(width: u16, height: u16) -> Faraday {
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

fn home_with_key() -> (Faraday, [u8; 4]) {
    let mut app = home_of(1366, 768);
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    (app, fp)
}

#[test]
fn home_a_key_row_opens_its_details_and_home_stays() {
    let (mut app, fp) = home_with_key();
    assert!(shown(&mut app, Action::RowOpen(Row::Key(fp))));
    assert!(!details_of(&mut app, fp));
    app.press(Action::RowOpen(Row::Key(fp)));
    assert_eq!(app.screen, Screen::Home);
    assert!(details_of(&mut app, fp));
    assert!(
        app.offers(Action::ExploreKey(fp)),
        "Explore, the first offer"
    );
    assert!(
        app.offers(Action::RowClose),
        "the way back {:?}",
        app.offered_actions()
    );
}

#[test]
fn home_the_details_fit_under_the_card_or_beside_it() {
    // Tall: under the card; short: beside it. Either way they are shown.
    for height in [1080, 600] {
        let mut app = home_of(1920, height);
        let fp = add_key(&mut app, WORDS);
        app.press(Action::Nav(Screen::Home));
        let _ = app.frame();
        app.press(Action::RowOpen(Row::Key(fp)));
        assert!(details_of(&mut app, fp), "at {height}");
    }
}

#[test]
fn home_back_escape_and_the_tab_return_home() {
    let (mut app, fp) = home_with_key();
    for how in 0..3 {
        app.press(Action::RowOpen(Row::Key(fp)));
        assert!(details_of(&mut app, fp), "way {how}");
        match how {
            0 => {
                let at = app.where_offered(Action::RowClose).expect("the arrow");
                touch(&mut app, at);
            }
            1 => {
                app.event(Event::Back);
                let _ = app.frame();
            }
            _ => app.press(Action::Nav(Screen::Home)),
        }
        assert!(!details_of(&mut app, fp), "way {how}");
        assert_eq!(app.screen, Screen::Home);
    }
}

#[test]
fn home_a_press_on_another_row_swaps_the_details() {
    let (mut app, a) = home_with_key();
    let b = add_key(&mut app, &faraday_core::testkit::test_words("zebra"));
    let _ = app.frame();
    app.press(Action::RowOpen(Row::Key(a)));
    assert!(details_of(&mut app, a));
    app.press(Action::RowOpen(Row::Key(b)));
    assert!(details_of(&mut app, b));
    assert!(!app.offers(Action::KeyWallet(a, 1)));
}

#[test]
fn home_the_key_going_away_returns_home() {
    let (mut app, fp) = home_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    assert!(details_of(&mut app, fp));
    app.press(Action::RemoveKeysHold);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Home);
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn home_lock_clears_the_row_open() {
    let (mut app, fp) = home_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    app.press(Action::Lock);
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Home));
    assert!(!details_of(&mut app, fp));
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn home_leaving_a_flow_begun_from_the_details_returns_to_them() {
    let (mut app, fp) = home_with_key();
    app.press(Action::RowOpen(Row::Key(fp)));
    let _ = app.frame();
    app.press(Action::ExploreKey(fp));
    assert_eq!(app.screen, Screen::Explore);
    let _ = app.frame();
    let (x, y) = app.back_offered().expect("a way back");
    touch(&mut app, (x, y));
    assert_eq!(app.screen, Screen::Home);
    assert!(details_of(&mut app, fp), "the details, as they were");
}

#[test]
fn home_a_compact_window_opens_nothing() {
    let mut app = home_of(480, 640);
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(!app.offers(Action::RowOpen(Row::Key(fp))));
    app.press(Action::RowOpen(Row::Key(fp)));
    let _ = app.frame();
    assert!(!app.offers(Action::RowClose));
    assert!(!details_of(&mut app, fp));
}

fn home_with_wallet() -> Faraday {
    home_with_wallet_of(1366, 768)
}

fn home_with_wallet_of(width: u16, height: u16) -> Faraday {
    let c = faraday_core::testkit::compute();
    let mut app = home_of(width, height);
    app.session.keys.add_words(&c, WORDS, "Seed", None).unwrap();
    faraday_core::testkit::keep_published(&mut app);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let d = format!("wpkh({}/<0;1>/*)", &*text);
    app.session
        .add_wallet_confirmed(
            &c,
            "Wallet",
            &d,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

#[test]
fn home_a_wallets_details_are_its_page() {
    let mut app = home_with_wallet();
    assert!(!app.offers(Action::Backup(0)));
    app.press(Action::RowOpen(Row::Wallet(0)));
    assert_eq!(app.screen, Screen::Home);
    assert!(shown(&mut app, Action::Backup(0)), "its What you can do");
}

#[test]
fn home_a_press_beside_a_rows_button_opens_the_details_and_the_button_is_its_own() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = home_of(1366, 768);
    faraday_core::testkit::inbox_file(&mut app, "spend.psbt", psbt);
    faraday_core::testkit::inbox_file_at(&mut app, 1, "wallet.txt", kit.descriptor.into_bytes());
    faraday_core::testkit::load_wallet(&mut app, 1);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.offers(Action::StartSpend(0)), "the row's button");
    let (x, y, _, h) = app.hit_box(Action::RowOpen(Row::File(0))).expect("the row");
    touch(&mut app, (x + 24, y + h / 2));
    assert!(app.offers(Action::RowClose), "the details are open");
    assert_eq!(app.screen, Screen::Home);
    app.press(Action::RowClose);
    let _ = app.frame();
    let (bx, by, bw, bh) = app.hit_box(Action::StartSpend(0)).expect("the button");
    touch(&mut app, (bx + bw / 2, by + bh / 2));
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
    assert!(!app.offers(Action::RowClose), "the button is its own press");
    assert!(app.spend().is_some(), "Sign began the spend");
}

fn signs(app: &mut Faraday) -> usize {
    app.drawn_ink()
        .iter()
        .filter(|i| {
            matches!(
                i.kind,
                faraday_draw::canvas::InkKind::Icon(faraday_draw::widgets::Icon::Sign)
            )
        })
        .count()
}

#[test]
fn a_files_offers_carry_the_icon_their_action_has_elsewhere() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = home_of(1366, 768);
    faraday_core::testkit::inbox_file(&mut app, "spend.psbt", psbt);
    // The spending wallet is loaded, so the file offers Sign.
    faraday_core::testkit::inbox_file_at(&mut app, 1, "wallet.txt", kit.descriptor.into_bytes());
    faraday_core::testkit::load_wallet(&mut app, 1);
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    let before = signs(&mut app);
    app.press(Action::RowOpen(Row::File(0)));
    let _ = app.frame();
    assert!(app.offers(Action::StartSpend(0)), "Sign is offered");
    let after = signs(&mut app);
    assert!(
        after > before,
        "the Sign offer has its icon: {before} then {after}"
    );
}

#[test]
fn home_the_rails_windows_open_no_details() {
    // 800 x 600 dp is a rail window with the search in the top bar.
    let mut app = home_of(800, 600);
    let fp = add_key(&mut app, WORDS);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(!app.offers(Action::RowOpen(Row::Key(fp))));
    app.press(Action::RowOpen(Row::Key(fp)));
    let _ = app.frame();
    assert!(!app.offers(Action::RowClose));
    assert!(!app.offers(Action::RowOpen(Row::Key(fp))));
}

#[test]
fn home_a_wallets_details_stand_beside_the_card_at_1366() {
    let mut app = home_with_wallet();
    app.press(Action::RowOpen(Row::Wallet(0)));
    let _ = app.frame();
    let row = app
        .hit_box(Action::RowOpen(Row::Wallet(0)))
        .expect("the row stays in the card");
    let b = app.hit_box(Action::Backup(0)).expect("the details");
    assert!(
        b.0 >= row.0 + row.2,
        "beside the card, not under it: {b:?} vs {row:?}"
    );
}

/// Where the wallet's page fits whole under the card, the details stand
/// under it, as every row's do.
#[test]
fn home_a_wallets_details_stand_under_the_card_at_1920x1600() {
    let mut app = home_with_wallet_of(1920, 1600);
    app.press(Action::RowOpen(Row::Wallet(0)));
    let _ = app.frame();
    let row = app
        .hit_box(Action::RowOpen(Row::Wallet(0)))
        .expect("the row stays in the card");
    let b = app.hit_box(Action::Backup(0)).expect("the details");
    assert!(
        b.1 >= row.1 + row.3,
        "under the card, not beside it: {b:?} vs {row:?}"
    );
    assert!(
        b.0 < row.0 + row.2,
        "in the card's column, not beside it: {b:?} vs {row:?}"
    );
}
