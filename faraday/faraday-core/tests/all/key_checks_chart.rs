//! A wallet of several keys with keys not checked says so on its chart
//! node and shows every key, tickable, on its own sheet.

use crate::common::texts;
use faraday_core::glance::Press;
use faraday_core::glance::sheet::{ChartAction as C, Target};
use faraday_core::testkit::KindText;
use faraday_core::wallet::KeyBlockOf;
use faraday_core::{Action, Faraday, Sheet, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn device() -> Faraday {
    let mut app = Faraday::new();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1440,
        height: 900,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

/// A 2-of-3 wallet with one key loaded: two keys are not checked.
fn two_unchecked(app: &mut Faraday) -> usize {
    let c = testkit::compute();
    add_key(app, 0);
    app.session
        .add_wallet_confirmed(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap()
}

fn open_wallet_sheet(app: &mut Faraday, w: usize, target: Target) {
    app.press(Action::OpenWallet(w));
    app.press(Action::Chart(C::Open(Press::Loaded(w), target)));
    crate::common::a_moment(app);
    assert_eq!(app.sheet, Some(Sheet::Chart));
}

fn all(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

/// The sheet scrolled down, as a person would, until a line holding
/// `part` is drawn (at most 12 wheel steps), then read as it stands. The
/// side sheet is taller than the window once its addresses are worked
/// out, and a line below the window is not drawn.
fn all_with(app: &mut Faraday, part: &str) -> String {
    for _ in 0..12 {
        let s = all(app);
        if s.contains(part) {
            return s;
        }
        let (w, h) = app.size();
        app.event(Event::Scroll {
            x: (w - 100.0) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
    }
    all(app)
}

#[test]
fn the_node_and_the_sheet_count_the_keys_not_checked_and_a_tick_there_updates_both() {
    for target in [Target::Wallet, Target::Addresses] {
        let mut app = device();
        let w = two_unchecked(&mut app);
        open_wallet_sheet(&mut app, w, target);
        let s = all(&mut app);
        assert!(s.contains("Yours, can sign"), "{s}");
        assert_eq!(s.matches("Not checked").count(), 2, "{s}");
        // The node's line and the sheet's line.
        assert!(s.contains("2 keys not checked"), "{s}");
        let s = all_with(&mut app, "compare each with its own device");
        assert!(s.contains("compare each with its own device"), "{s}");

        app.press(Action::KeyTick(KeyBlockOf::Wallet(w as u16), 1));
        let s = all_with(&mut app, "1 key not checked");
        assert!(s.contains("1 key not checked"), "{s}");
        assert!(!s.contains("2 keys not checked"), "{s}");
        app.press(Action::KeyTick(KeyBlockOf::Wallet(w as u16), 2));
        let s = all_with(&mut app, "Every key checked");
        assert!(s.contains("Every key checked"), "{s}");
        assert!(!s.contains("keys not checked"), "{s}");
        assert!(!s.contains("1 key not checked"), "{s}");
    }
}

#[test]
fn a_wallet_of_one_key_shows_no_key_checks() {
    let mut app = device();
    let c = testkit::compute();
    add_key(&mut app, 0);
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    let w = app
        .session
        .add_wallet_confirmed(
            &c,
            "Spending",
            &format!("wpkh({}/<0;1>/*)", &*text),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    open_wallet_sheet(&mut app, w, Target::Wallet);
    let s = all(&mut app);
    assert!(!s.contains("not checked"), "{s}");
    assert!(!s.contains("Every key checked"), "{s}");
}
