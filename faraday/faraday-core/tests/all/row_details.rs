//! A row of the session pane on Tools opens its details in the page's
//! second pane (`docs/WORKSPACE.md` §7 point 1): the page stays lit, Back,
//! Escape and the page's tab return, a key's, a wallet's and a file's
//! details offer what they offer, a row's button stays its own press and a
//! flow begun from the details returns to them.

use faraday_core::convert::ConvertAction as C;
use faraday_core::lightning::NodeSource;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit::KindText;
use faraday_core::{Action, Faraday, KeyTool, Row, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const WORDS: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                      abandon abandon about";
const OTHER_WORDS: &str =
    "legal winner thank year wave sausage worth useful legal winner thank yellow";

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
    assert!(
        app.offers(Action::KeyWallet(fp, 2)),
        "Start a multisig wallet with this key"
    );
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

const MESSAGE: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq\nH9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=\nHello\n";

// ---- Files (WORKSPACE.md §7 point 1: a file of In Faraday opens its
// details in the stick's place)

fn a_stick() -> faraday_core::StorageEvent {
    faraday_core::StorageEvent::Sticks(vec![faraday_core::StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("spend.psbt"), 300)],
        vaults: Vec::new(),
    }])
}

/// Files with a stick in (or none) and two files held, `a.txt` and `b.txt`.
fn files_of(width: u16, stick: bool) -> Faraday {
    files_sized(width, 1080, stick)
}

fn files_sized(width: u16, height: u16, stick: bool) -> Faraday {
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
    if stick {
        app.storage(a_stick());
    }
    faraday_core::testkit::inbox_file(&mut app, "a.txt", MESSAGE.as_bytes().to_vec());
    faraday_core::testkit::inbox_file(&mut app, "b.txt", MESSAGE.as_bytes().to_vec());
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app
}

fn files_with_stick() -> Faraday {
    files_of(1366, true)
}

/// A press at the middle of the row's body, which opens its details.
fn press_body(app: &mut Faraday, k: usize) {
    let (x, y, _, _) = app.hit_box(Action::RowOpen(Row::File(k))).expect("the row");
    // The hit box found is the last one the row drew, its chevron's; the
    // body is left of it, clear of the row's main button (1366 wide).
    touch(app, (x - 365, y + 25));
}

/// The stick's card is on show: it offers Unselect all.
fn stick_pane(app: &mut Faraday) -> bool {
    shown(app, Action::VisitInAll)
}

fn count(app: &mut Faraday, name: &str) -> usize {
    app.drawn_texts().iter().filter(|t| *t == name).count()
}

#[test]
fn files_a_press_on_a_row_opens_its_details_in_the_sticks_place() {
    for stick in [true, false] {
        let mut app = files_of(1366, stick);
        assert!(!app.offers(Action::RowClose));
        press_body(&mut app, 0);
        assert!(app.offers(Action::RowClose), "stick {stick}: details open");
        assert_eq!(app.screen, Screen::Files);
        assert!(!stick_pane(&mut app), "stick {stick}: the stick's card");
        // The list stays at the left; the row's own press is the page's.
        assert!(app.offers(Action::RowOpen(Row::File(1))));
        assert!(!app.visit().from_inbox.contains("a.txt"), "a tick");
    }
}

#[test]
fn files_back_and_the_tab_return_with_the_ticks_and_the_stick_as_they_were() {
    let mut app = files_with_stick();
    let was = app.visit().from_inbox.clone();
    let (x, y, w, h) = app.hit_box(Action::VisitInbox(1)).expect("the tick");
    touch(&mut app, (x + w / 2, y + h / 2));
    let ticks = app.visit().from_inbox.clone();
    assert_ne!(ticks, was, "the box ticked");
    assert!(!app.offers(Action::RowClose), "the box opens nothing");
    for how in 0..3 {
        press_body(&mut app, 0);
        assert!(app.offers(Action::RowClose), "way {how}");
        match how {
            0 => {
                let at = app.where_offered(Action::RowClose).expect("the arrow");
                touch(&mut app, at);
            }
            1 => app.press(Action::Nav(Screen::Files)),
            _ => {
                app.event(Event::Back);
                let _ = app.frame();
            }
        }
        let _ = app.frame();
        assert!(!app.offers(Action::RowClose), "way {how}");
        assert_eq!(app.screen, Screen::Files);
        assert!(stick_pane(&mut app), "way {how}: the stick's card");
        assert_eq!(app.visit().from_inbox, ticks, "way {how}: the ticks");
    }
}

#[test]
fn files_a_press_on_another_row_swaps_the_details() {
    let mut app = files_with_stick();
    press_body(&mut app, 0);
    let _ = app.frame();
    assert_eq!(
        count(&mut app, "a.txt"),
        3,
        "the list, the bar and the facts"
    );
    assert_eq!(count(&mut app, "b.txt"), 1);
    press_body(&mut app, 1);
    let _ = app.frame();
    assert_eq!(count(&mut app, "b.txt"), 3);
    assert_eq!(count(&mut app, "a.txt"), 1);
    app.press(Action::RowClose);
    let _ = app.frame();
    assert!(stick_pane(&mut app));
}

#[test]
fn files_a_rows_tick_and_button_are_their_own_presses() {
    let mut app = files_of(1366, true);
    // Four wallets, the one pressed last: the summary card loads the first
    // three or is folded, so its row carries its own Load.
    let (last, others): (Vec<_>, Vec<_>) = faraday_core::testkit::kits()
        .into_iter()
        .partition(|k| k.id == "spending");
    for (n, kit) in others.into_iter().take(3).chain(last).enumerate() {
        let name = format!("wallet-{n}.txt");
        faraday_core::testkit::inbox_file(&mut app, &name, kit.descriptor.into_bytes());
    }
    let _ = app.frame();
    let k = app
        .inbox()
        .iter()
        .position(|i| i.name == "wallet-3.txt")
        .unwrap();
    // A public file's tick is a plain press; a secret's asks an acknowledgement.
    let was = app.visit().from_inbox.contains("a.txt");
    let (x, y, w, h) = app.hit_box(Action::VisitInbox(0)).expect("the tick");
    touch(&mut app, (x + w / 2, y + h / 2));
    assert!(!app.offers(Action::RowClose), "the tick opens nothing");
    assert_ne!(
        app.visit().from_inbox.contains("a.txt"),
        was,
        "the tick changed"
    );
    let at = app
        .where_offered(Action::LoadWallet(k))
        .expect("the button");
    touch(&mut app, at);
    assert!(!app.offers(Action::RowClose), "the button opens nothing");
    assert!(!app.session.wallets.is_empty(), "Load loaded the wallet");
}

#[test]
fn files_the_file_going_away_returns_the_page() {
    let mut app = files_with_stick();
    press_body(&mut app, 0);
    assert!(app.offers(Action::RowClose));
    faraday_core::testkit::inbox_retain(&mut app, |it| it.name != "a.txt");
    // A person's next press, as the real path to it is a press.
    let (x, y, w, h) = app.hit_box(Action::VisitInbox(0)).expect("a tick");
    touch(&mut app, (x + w / 2, y + h / 2));
    assert!(!app.offers(Action::RowClose), "the details are gone");
    assert!(stick_pane(&mut app));
    assert!(stick_pane(&mut app));
}

#[test]
fn files_lock_clears_the_row_open() {
    let mut app = files_with_stick();
    press_body(&mut app, 0);
    app.press(Action::Lock);
    faraday_core::testkit::inbox_file(&mut app, "a.txt", MESSAGE.as_bytes().to_vec());
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn files_leaving_a_flow_begun_from_the_details_returns_to_them() {
    let mut app = files_with_stick();
    press_body(&mut app, 0);
    assert!(app.offers(Action::RowClose));
    // The details' button (the file's own Check) begins the flow.
    app.press(Action::CheckMessage(0));
    assert_ne!(app.screen, Screen::Files, "the details offer a flow");
    let _ = app.frame();
    let (x, y) = app.back_offered().expect("a way back");
    touch(&mut app, (x, y));
    assert_eq!(app.screen, Screen::Files);
    assert!(app.offers(Action::RowClose), "the details, as they were");
}

#[test]
fn files_the_rails_window_opens_none() {
    let mut app = files_of(800, true);
    assert!(!app.offers(Action::RowOpen(Row::File(0))));
    app.press(Action::RowOpen(Row::File(0)));
    let _ = app.frame();
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn files_on_a_small_panel_a_rows_press_opens_the_details_and_back_returns() {
    // Without a stick: the list has the room for the row.
    let mut app = files_of(400, false);
    assert!(app.offers(Action::RowOpen(Row::File(0))), "the row");
    // The row's chevron, at its end.
    let (x, y, w, h) = app.hit_box(Action::RowOpen(Row::File(0))).expect("the row");
    touch(&mut app, (x + w / 2, y + h / 2));
    assert!(app.offers(Action::RowClose), "the details are open");
    assert!(
        !app.offers(Action::RowOpen(Row::File(0))),
        "the details stand in the list's place"
    );
    assert_eq!(app.screen, Screen::Files);
    let (x, y) = app.back_offered().expect("a way back");
    touch(&mut app, (x, y));
    assert_eq!(app.screen, Screen::Files);
    assert!(app.offers(Action::RowOpen(Row::File(0))), "the list again");
    assert!(!app.offers(Action::RowClose));
}

#[test]
fn files_a_stick_put_in_or_pulled_with_details_open_keeps_the_stick_as_before() {
    let mut app = files_of(1366, false);
    press_body(&mut app, 0);
    app.storage(a_stick());
    let _ = app.frame();
    assert!(app.offers(Action::RowClose), "still open");
    assert!(!stick_pane(&mut app));
    app.press(Action::RowClose);
    let _ = app.frame();
    assert!(stick_pane(&mut app), "the stick put in");
    press_body(&mut app, 0);
    app.storage(faraday_core::StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert!(app.offers(Action::RowClose), "still open");
    app.press(Action::RowClose);
    let _ = app.frame();
    assert!(!stick_pane(&mut app), "the stick pulled");
    assert!(app.offers(Action::RowOpen(Row::File(0))));
}

/// Files with many files held, so that the page scrolls, scrolled down a
/// little (the rows still in view).
fn files_scrolled() -> (Faraday, f32, f32) {
    let mut app = files_with_stick();
    for n in 0..60 {
        faraday_core::testkit::inbox_file(
            &mut app,
            &format!("f{n:02}.txt"),
            MESSAGE.as_bytes().to_vec(),
        );
    }
    let _ = app.frame();
    for _ in 0..2 {
        app.event(Event::Scroll {
            x: 200,
            y: 400,
            dy: 300,
        });
        app.settle();
        let _ = app.frame();
    }
    let (page, list) = (app.list_offset, app.visit_in_offset());
    (app, page, list)
}

#[test]
fn files_back_keeps_the_lists_scroll() {
    for how in 0..3 {
        let (mut app, page, list) = files_scrolled();
        assert!(page > 0.0, "the page scrolled");
        let k = (0..62)
            .find(|k| app.offers(Action::RowOpen(Row::File(*k))))
            .expect("a row in view");
        press_body(&mut app, k);
        let _ = app.frame();
        assert!(app.offers(Action::RowClose));
        match how {
            0 => app.press(Action::RowClose),
            1 => app.press(Action::Nav(Screen::Files)),
            _ => {
                app.event(Event::Back);
            }
        }
        let _ = app.frame();
        assert!(!app.offers(Action::RowClose), "way {how}");
        assert_eq!(app.list_offset, page, "way {how}: the page's scroll");
        assert_eq!(app.visit_in_offset(), list, "way {how}: the list's scroll");
    }
}

#[test]
fn files_the_bar_names_the_file_only_where_the_details_stand() {
    // Where the room no longer holds the pair (the window narrowed, or the
    // guide pane beside the page), the arrow and the file's name go with
    // the details: both or neither.
    for width in [1366u16, 1280, 1200, 1100, 1000, 900] {
        let mut app = files_of(1366, true);
        press_body(&mut app, 0);
        let _ = app.frame();
        assert!(app.offers(Action::RowClose));
        app.event(Event::Display(DisplayInfo {
            width,
            height: 1080,
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
        let _ = app.frame();
        let named = count(&mut app, "a.txt");
        let details = named >= 3;
        let arrow = app.offers(Action::RowClose);
        assert_eq!(
            arrow, details,
            "{width}: the arrow with the details ({named})"
        );
        assert_eq!(
            named >= 2,
            details,
            "{width}: the name with the details ({named})"
        );
    }
}

/// The owner's rule (2026-11-25): the box alone ticks a file for Write; a
/// press anywhere else on the row opens its details. A file that asks
/// (seed words) has its box open the secret sheet, never the details.
#[test]
fn files_the_box_ticks_and_the_rest_of_the_row_opens_the_details() {
    for stick in [true, false] {
        let mut app = files_sized(1366, 768, stick);
        faraday_core::testkit::inbox_file(&mut app, "seed.txt", WORDS.as_bytes().to_vec());
        let _ = app.frame();
        let seed = app
            .inbox()
            .iter()
            .position(|i| i.name == "seed.txt")
            .unwrap();
        for (k, asks) in [(0usize, false), (seed, true)] {
            let name = app.inbox()[k].name.to_string();
            let ticked = |app: &Faraday| app.visit().from_inbox.contains(name.as_str());
            if stick {
                // The box: ticked (or asked), the details not opened.
                let at = app.where_offered(Action::VisitInbox(k)).expect("the box");
                touch(&mut app, at);
                assert!(
                    !app.offers(Action::RowClose),
                    "{name}: the box opened details"
                );
                if asks {
                    assert!(!ticked(&app), "{name}: ticked unasked");
                    assert!(app.sheet.is_some(), "{name}: no secret sheet");
                    app.press(Action::Cancel);
                    let _ = app.frame();
                } else {
                    assert!(ticked(&app), "{name}: not ticked");
                }
            } else {
                assert!(
                    !app.offers(Action::VisitInbox(k)),
                    "{name}: a box, no stick"
                );
            }
            let was = ticked(&app);
            // The hit box found is the last the row drew: its chevron's, 48
            // wide at the column's edge, the body's press ending before it.
            for what in ["name", "line", "chevron"] {
                let (x, y, w, h) = app.hit_box(Action::RowOpen(Row::File(k))).expect("the row");
                let at = match what {
                    "name" => (x - 365, y + 14),
                    "line" => (x - 365, y + 34),
                    _ => (x + w / 2, y + h / 2),
                };
                touch(&mut app, at);
                assert!(app.offers(Action::RowClose), "{name} {what}: no details");
                assert_eq!(ticked(&app), was, "{name} {what}: tick changed");
                assert!(app.sheet.is_none(), "{name} {what}: a sheet");
                // The open row pressed again, then back to the list.
                app.press(Action::RowClose);
                let _ = app.frame();
                assert!(!app.offers(Action::RowClose));
            }
            // The open row pressed again keeps its details; another swaps.
            press_body(&mut app, k);
            press_body(&mut app, k);
            assert!(app.offers(Action::RowClose), "{name}: again closed");
            let other = if k == 0 { 1 } else { 0 };
            press_body(&mut app, other);
            assert!(app.offers(Action::RowClose), "{name}: swap");
            app.press(Action::RowClose);
            let _ = app.frame();
        }
    }
}

/// The 2-of-2 or 2-of-3 over the first `n` test seeds.
fn multi_of(n: usize) -> String {
    let keys: Vec<String> = (0..n)
        .map(|k| faraday_core::testkit::key(k, "m/48'/1'/0'/2'") + "/<0;1>/*")
        .collect();
    format!("wsh(sortedmulti(2,{}))", keys.join(","))
}

fn single() -> String {
    format!(
        "wpkh({}/<0;1>/*)",
        faraday_core::testkit::key(0, "m/84'/1'/0'")
    )
}

fn add_wallet(app: &mut Faraday, descriptor: &str) {
    let c = faraday_core::testkit::compute();
    app.session
        .add_wallet_confirmed(
            &c,
            "Savings",
            descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("reads");
}

/// The fingerprint of slot `k` of the first wallet.
fn slot(app: &Faraday, k: usize) -> [u8; 4] {
    app.session.slots(&app.session.wallets[0])[k]
        .fingerprint
        .expect("a fingerprint")
        .0
}

/// The pages whose pane gives a wallet's row its button.
const ROW_PAGES: [Screen; 3] = [Screen::Home, Screen::Catalog, Screen::Check];

fn go(app: &mut Faraday, page: Screen) {
    app.press(Action::Nav(page));
    let _ = app.frame();
}

/// Whether `action`'s button is drawn inside the first wallet's row.
fn in_wallet_row(app: &Faraday, action: Action) -> bool {
    let row = app.hit_box(Action::RowOpen(Row::Wallet(0)));
    match (row, app.where_offered(action)) {
        (Some((x, y, w, h)), Some((px, py))) => px >= x && px < x + w && py >= y && py < y + h,
        _ => false,
    }
}

#[test]
fn a_wallet_with_one_empty_slot_has_add_a_key_for_that_slot() {
    // A single key's wallet, and a 2-of-2 with one of its keys here.
    for (descriptor, held) in [(single(), None), (multi_of(2), Some("zebra"))] {
        let mut app = desktop();
        if let Some(word) = held {
            add_key(&mut app, &faraday_core::testkit::test_words(word));
        }
        add_wallet(&mut app, &descriptor);
        let slots = app.session.slots(&app.session.wallets[0]);
        let k = slots
            .iter()
            .position(|s| s.held_by.is_none())
            .expect("an empty slot");
        let one = Action::Entry(Some(slot(&app, k)));
        for page in ROW_PAGES {
            go(&mut app, page);
            assert!(in_wallet_row(&app, one), "{page:?} {descriptor}");
            let at = app.where_offered(one).expect("the button");
            touch(&mut app, at);
            assert_eq!(app.screen, Screen::Entry, "{page:?}");
        }
    }
}

#[test]
fn a_wallet_with_several_empty_slots_has_the_general_add_a_key() {
    let mut app = desktop();
    add_wallet(&mut app, &multi_of(3));
    for page in ROW_PAGES {
        go(&mut app, page);
        for k in 0..3 {
            let one = Action::Entry(Some(slot(&app, k)));
            assert!(!app.offers(one), "{page:?}");
        }
        assert!(in_wallet_row(&app, Action::Entry(None)), "{page:?}");
    }
}

#[test]
fn a_press_beside_add_a_key_opens_the_wallets_details() {
    for page in ROW_PAGES {
        let mut app = desktop();
        add_wallet(&mut app, &single());
        go(&mut app, page);
        let details = Action::RowOpen(Row::Wallet(0));
        let (x, y, _, _) = app.hit_box(details).expect("the row");
        touch(&mut app, (x + 24, y + 12));
        assert_eq!(app.screen, page);
        assert!(shown(&mut app, Action::Backup(0)), "{page:?}: its page");
    }
}

#[test]
fn a_wallet_that_can_spend_has_no_row_button() {
    let mut app = desktop();
    add_key(&mut app, &faraday_core::testkit::test_words("bacon"));
    add_wallet(&mut app, &single());
    for page in ROW_PAGES {
        go(&mut app, page);
        assert!(app.offers(Action::RowOpen(Row::Wallet(0))), "{page:?}");
        let one = Action::Entry(Some(slot(&app, 0)));
        assert!(!app.offers(one), "{page:?}");
        assert!(!in_wallet_row(&app, Action::Entry(None)), "{page:?}");
    }
}

#[test]
fn a_silent_wallet_without_its_key_has_no_row_button() {
    let c = faraday_core::testkit::compute();
    let mut keyed = desktop();
    add_key(&mut keyed, &faraday_core::testkit::test_words("bacon"));
    keyed.press(Action::Silent);
    let record = keyed.silent_record(&c).expect("its record");
    let record = faraday_bip::silent_wallet::SilentWallet::parse(&record).unwrap();
    let fp = record.fingerprint.0;
    let mut app = desktop();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "silent-0.txt".to_string(),
            record.to_text().into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    for page in ROW_PAGES {
        go(&mut app, page);
        assert!(app.offers(Action::RowOpen(Row::Wallet(0))), "{page:?}");
        assert!(!app.offers(Action::Entry(Some(fp))), "{page:?}");
        assert!(!in_wallet_row(&app, Action::Entry(None)), "{page:?}");
    }
}

#[test]
fn with_a_stick_attached_the_rows_add_a_key_does_nothing() {
    let mut app = desktop();
    add_wallet(&mut app, &single());
    let one = Action::Entry(Some(slot(&app, 0)));
    go(&mut app, Screen::Check);
    let at = app.where_offered(one).expect("the button, no stick");
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("a"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        },
    ]));
    for page in ROW_PAGES {
        go(&mut app, page);
        assert!(!app.offers(one), "{page:?}: dimmed");
    }
    go(&mut app, Screen::Check);
    touch(&mut app, at);
    assert_ne!(app.screen, Screen::Entry);
}

#[test]
fn vaults_wallet_rows_have_no_add_a_key() {
    let mut app = desktop();
    // With no vault file, Vaults is its New vault flow.
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "cellar.kdbx".to_string(),
            crate::common::vault("Cellar", "correct horse battery", 1),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    add_wallet(&mut app, &single());
    go(&mut app, Screen::Vaults);
    assert!(app.offers(Action::OpenWallet(0)), "the wallet's row");
    assert!(!app.offers(Action::Entry(Some(slot(&app, 0)))));
}

/// Tools with a key a wallet uses, that key's details open.
fn tools_with_a_used_keys_details() -> (Faraday, [u8; 4]) {
    let mut app = desktop();
    let fp = add_key(&mut app, WORDS);
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    assert_eq!(app.session.wallets.len(), 1, "a wallet uses the key");
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    app.press(Action::RowOpen(Row::Key(fp)));
    let _ = app.frame();
    (app, fp)
}

/// Presses `action` on a wallet-used key's details, as they offer it, and
/// returns the app in the flow it opened.
fn used_key_press(action: impl Fn([u8; 4]) -> Action) -> (Faraday, [u8; 4]) {
    let (mut app, fp) = tools_with_a_used_keys_details();
    let action = action(fp);
    assert!(shown(&mut app, action), "{action:?} is not offered");
    app.press(action);
    let _ = app.frame();
    (app, fp)
}

#[test]
fn a_used_keys_details_offer_make_a_wallet_and_it_takes_the_key() {
    for n in [1u8, 2] {
        let (app, fp) = used_key_press(|fp| Action::KeyWallet(fp, n));
        assert_eq!(app.screen, Screen::Restore);
        let s = app
            .restore()
            .and_then(|r| r.seeds.as_ref())
            .expect("no seeds card");
        assert_eq!(s.keys, vec![fp]);
        assert_eq!(s.n, usize::from(n));
    }
}

#[test]
fn a_used_keys_details_offer_restore() {
    let (app, _) = used_key_press(|_| Action::RestoreWallet);
    assert_eq!(app.screen, Screen::Restore);
}

#[test]
fn a_used_keys_details_offer_another_form_with_the_key_picked() {
    let (mut app, fp) = used_key_press(|fp| Action::Convert(C::OpenOn(fp)));
    assert_eq!(app.screen, Screen::Convert);
    let key = faraday_core::convert::state(&app).and_then(|s| s.key);
    assert_eq!(key, Some(fp));
    // Back steps back through the flow's cards, then leaves it.
    for _ in 0..4 {
        if app.screen != Screen::Convert {
            break;
        }
        let _ = app.frame();
        let (x, y) = app.back_offered().expect("a way back");
        touch(&mut app, (x, y));
    }
    assert_eq!(app.screen, Screen::Catalog);
    assert!(details_of(&mut app, fp), "back in the key's details");
}

#[test]
fn a_used_keys_details_offer_explore() {
    let (app, _) = used_key_press(Action::ExploreKey);
    assert_eq!(app.screen, Screen::Explore);
}

#[test]
fn a_keys_details_offer_add_to_vault_and_it_saves_that_key() {
    use faraday_core::vault::VaultAction as V;
    // Tall enough for the whole group.
    let mut app = desktop_of(1400);
    let fp = add_key(&mut app, WORDS);
    let vault = faraday_core::testkit::test_vault().unwrap();
    faraday_core::testkit::inbox_file(&mut app, "vault.kdbx", vault);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::RowOpen(Row::Key(fp)));
    assert!(details_of(&mut app, fp));
    assert!(
        !app.offers(Action::Vault(V::SaveKey(0))),
        "no vault is open"
    );
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    // The unlock asks what to load from the vault: nothing.
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::RowOpen(Row::Key(fp)));
    let save = Action::Vault(V::SaveKey(0));
    assert!(shown(&mut app, save), "Add to vault");
    let records = |app: &Faraday| {
        let id = app.vaults().open[0].id;
        app.vaults().store().contents_for_tests(id).len()
    };
    let before = records(&app);
    app.press(save);
    let _ = app.frame();
    assert_eq!(records(&app), before + 1, "the vault has the key");
    assert!(!app.offers(save), "offered again");
}

/// Back until the flow on `flow` is left: the key's details are there again.
fn back_to_the_details(app: &mut Faraday, flow: Screen, fp: [u8; 4]) {
    // Back steps back through the flow's cards, then leaves it.
    for _ in 0..6 {
        if app.screen != flow {
            break;
        }
        let _ = app.frame();
        let (x, y) = app.back_offered().expect("a way back");
        touch(app, (x, y));
    }
    assert_eq!(app.screen, Screen::Catalog);
    assert!(details_of(app, fp), "back in the key's details");
}

#[test]
fn a_keys_details_offer_derive_with_the_key_picked() {
    let (mut app, fp) = used_key_press(|fp| Action::KeyTool(KeyTool::Derive, fp));
    assert_eq!(app.screen, Screen::Bip85);
    assert_eq!(app.bip85().and_then(|b| b.key), Some(fp));
    back_to_the_details(&mut app, Screen::Bip85, fp);
}

#[test]
fn a_keys_details_offer_silent_payments_with_the_key_picked() {
    let (mut app, fp) = used_key_press(|fp| Action::KeyTool(KeyTool::Silent, fp));
    assert_eq!(app.screen, Screen::Silent);
    assert_eq!(app.silent().and_then(|s| s.key), Some(fp));
    back_to_the_details(&mut app, Screen::Silent, fp);
}

#[test]
fn a_keys_details_offer_the_lightning_node_key_with_the_key_picked() {
    let (mut app, fp) = used_key_press(|fp| Action::KeyTool(KeyTool::Lightning, fp));
    assert_eq!(app.screen, Screen::Lightning);
    let source = app.lightning().and_then(|l| l.source);
    assert_eq!(source, Some(NodeSource::Key(fp)));
    back_to_the_details(&mut app, Screen::Lightning, fp);
}

#[test]
fn a_key_with_two_keys_loaded_opens_each_tool_on_the_one_whose_details_are_open() {
    let (mut app, fp) = tools_with_key();
    add_key(&mut app, OTHER_WORDS);
    app.press(Action::RowOpen(Row::Key(fp)));
    let tool = Action::KeyTool(KeyTool::Derive, fp);
    assert!(shown(&mut app, tool));
    app.press(tool);
    assert_eq!(app.bip85().and_then(|b| b.key), Some(fp));
}

#[test]
fn a_key_without_words_is_not_offered_the_lightning_node_key() {
    let mut app = desktop();
    let c = faraday_core::testkit::compute();
    app.session.keys.add_seed(&c, &[7u8; 32], "").unwrap();
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::RowOpen(Row::Key(fp)));
    assert!(shown(&mut app, Action::KeyTool(KeyTool::Derive, fp)));
    assert!(app.offers(Action::KeyTool(KeyTool::Silent, fp)));
    assert!(!app.offers(Action::KeyTool(KeyTool::Lightning, fp)));
}

#[test]
fn a_key_waiting_for_its_backup_is_not_offered_silent_payments() {
    let (mut app, fp) = tools_with_key();
    faraday_core::testkit::keep_published(&mut app);
    let made = app.session.keys[0].fingerprint;
    faraday_core::testkit::mark_made_here(&mut app, made);
    app.press(Action::RowOpen(Row::Key(fp)));
    assert!(shown(&mut app, Action::KeyTool(KeyTool::Derive, fp)));
    assert!(!app.offers(Action::KeyTool(KeyTool::Silent, fp)));
}
