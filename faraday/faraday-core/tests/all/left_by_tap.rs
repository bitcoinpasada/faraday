//! A screen that keeps nothing when it is left (New key, BIP-85, Explore,
//! the calculators, Vanity, Lightning, Silent payments, Upgrade) forgets
//! what it held however it is left: a tap on the sidebar or on Back, the
//! mouse's back button, or a scripted press. Opened again it is fresh,
//! and a secret it showed is not drawn.

use faraday_core::keygen::Start;
use faraday_core::tools::Tool;
use faraday_core::vanity::{VanityAction as V, vstep};
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, Event, Key, TouchPhase};

/// A press where the last frame drew `action`, as a finger makes it.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

/// What `show` puts on screen that was not drawn before it.
fn revealed(app: &mut Faraday, show: Action) -> Vec<String> {
    let before = texts(app);
    app.press(show);
    let after = texts(app);
    let new: Vec<String> = after.into_iter().filter(|t| !before.contains(t)).collect();
    assert!(!new.is_empty(), "{show:?} shows something");
    new
}

/// None of `secret` is drawn.
fn not_drawn(app: &mut Faraday, secret: &[String]) {
    let now = texts(app);
    for s in secret {
        assert!(!now.contains(s), "{s:?} is still drawn");
    }
}

/// The app with the test seed loaded, on Wallets.
fn with_key() -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    app.press(Action::Entry(None));
    typed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::Nav(Screen::WalletsStart));
    app
}

/// BIP-85 with a child seed's words on screen; returns the words.
fn bip85_shown(app: &mut Faraday) -> Vec<String> {
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85);
    app.press(Action::PApp(0));
    app.press(Action::PNext);
    app.press(Action::PNext);
    revealed(app, Action::PShow)
}

#[test]
fn new_key_left_by_a_tap_on_the_sidebar_forgets_the_words_typed() {
    let words = testkit::test_words("bacon");
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::Words.index()));
    app.press(Action::KNext);
    let _ = app.frame();
    typed(&mut app, &words);
    app.press(Action::KNext);
    let shown = revealed(&mut app, Action::KShow);
    tap(&mut app, Action::Nav(Screen::Settings));
    assert_eq!(app.screen, Screen::Settings);
    assert!(app.keygen.is_none(), "New key keeps nothing when left");
    assert!(
        app.entry.typed.as_str().is_empty(),
        "nor do its words stay typed"
    );
    not_drawn(&mut app, &shown);
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    assert_eq!(
        app.keygen.as_ref().unwrap().open,
        Some(faraday_core::keygen::kstep::START)
    );
    not_drawn(&mut app, &shown);
}

#[test]
fn bip85_left_by_a_tap_forgets_the_child_seed_shown() {
    let mut app = with_key();
    let shown = bip85_shown(&mut app);
    tap(&mut app, Action::Nav(Screen::Settings));
    assert!(app.bip85().is_none(), "BIP-85 keeps nothing when left");
    not_drawn(&mut app, &shown);
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    app.press(Action::Bip85);
    assert!(!app.bip85().unwrap().shown);
    not_drawn(&mut app, &shown);
}

#[test]
fn bip85_left_by_the_mouses_back_button_is_not_brought_back_by_forward() {
    let mut app = with_key();
    let shown = bip85_shown(&mut app);
    app.event(Event::Back);
    assert_ne!(app.screen, Screen::Bip85);
    assert!(app.bip85().is_none(), "Back leaves it as any leaving does");
    let left_to = app.screen;
    app.event(Event::Forward);
    assert_eq!(app.screen, left_to, "nothing kept to come back to");
    not_drawn(&mut app, &shown);
}

#[test]
fn lightning_left_by_a_tap_forgets_the_node_key_shown() {
    const CIPHER_SEED: &str = "above judge emerge veteran reform crunch system all snap please \
        shoulder vault hurt city quarter cover enlist swear success suggest drink wagon enrich body";
    let seed = CIPHER_SEED.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut app = testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    typed(&mut app, &seed);
    app.event(Event::Key(Key::Enter));
    // Drawn first, the aezeed stretched on the tick after (`docs/SPEED.md`
    // §9).
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    assert!(app.lightning().unwrap().node.is_some());
    let shown = revealed(&mut app, Action::LShow);
    tap(&mut app, Action::Nav(Screen::Settings));
    assert!(
        app.lightning().is_none(),
        "Lightning keeps nothing when left"
    );
    not_drawn(&mut app, &shown);
    app.press(Action::Lightning);
    assert!(app.lightning().unwrap().node.is_none());
    not_drawn(&mut app, &shown);
}

#[test]
fn a_calculator_left_by_a_tap_forgets_what_was_typed() {
    let mut app = testkit::started();
    app.press(Action::Tools);
    let i = Tool::ALL.iter().position(|t| *t == Tool::Hashes).unwrap();
    app.press(Action::TTool(i as u8));
    let before = texts(&mut app);
    typed(&mut app, "correct horse battery staple");
    let shown: Vec<String> = texts(&mut app)
        .into_iter()
        .filter(|t| !before.contains(t))
        .collect();
    assert!(!shown.is_empty());
    tap(&mut app, Action::Nav(Screen::Settings));
    assert!(
        app.tools.is_none(),
        "the calculators keep nothing when left"
    );
    app.press(Action::Tools);
    app.press(Action::TTool(i as u8));
    not_drawn(&mut app, &shown);
}

#[test]
fn explore_left_by_a_tap_opens_again_on_its_first_path() {
    let mut app = with_key();
    app.press(Action::Explore);
    let first = app.explore().unwrap().path.clone();
    app.press(Action::XPreset(1));
    assert_ne!(app.explore().unwrap().path, first);
    tap(&mut app, Action::Nav(Screen::Settings));
    assert!(app.explore().is_none(), "Explore keeps nothing when left");
    app.press(Action::Explore);
    assert_eq!(app.explore().unwrap().path, first);
}

#[test]
fn vanity_left_by_a_tap_stops_its_search() {
    let mut app = with_key();
    app.press(Action::Vanity(V::Open));
    app.press(Action::Vanity(V::Next(vstep::DIAL)));
    app.press(Action::Vanity(V::Next(vstep::SCRIPT)));
    // Long enough that no find comes while the test runs.
    typed(&mut app, "qqqqqqqq");
    app.press(Action::Vanity(V::Next(vstep::PREFIX)));
    app.press(Action::Vanity(V::Start));
    app.event(Event::Tick { now_ms: 100 });
    assert!(app.vanity().unwrap().running);
    tap(&mut app, Action::Nav(Screen::Settings));
    assert!(
        app.vanity().is_none(),
        "the search ends when Vanity is left"
    );
    app.event(Event::Tick { now_ms: 200 });
    app.press(Action::Vanity(V::Open));
    assert!(!app.vanity().unwrap().running);
}

#[test]
fn silent_payments_left_by_a_tap_opens_again_fresh() {
    let mut app = with_key();
    app.press(Action::Silent);
    assert_eq!(app.screen, Screen::Silent);
    let fresh = app.silent().unwrap().open;
    app.press(Action::SLabel(1));
    tap(&mut app, Action::Nav(Screen::Settings));
    assert!(
        app.silent().is_none(),
        "Silent payments keeps nothing when left"
    );
    app.press(Action::Silent);
    assert_eq!(app.silent().unwrap().open, fresh);
}

#[test]
fn upgrade_left_by_a_tap_lets_the_boot_partitions_go() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    app.press(Action::UpgradeOpen);
    assert_eq!(app.screen, Screen::Upgrade);
    while app.poll_storage().is_some() {}
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    assert!(!app.upgrading(), "Upgrade ends when it is left");
    let asked: Vec<StorageCommand> = std::iter::from_fn(|| app.poll_storage()).collect();
    assert!(
        asked.contains(&StorageCommand::BootForget),
        "the copier drops the source: {asked:?}"
    );
    // A boot partition arriving now is not taken for an upgrade.
    app.storage(StorageEvent::Boots(Vec::new()));
    assert!(!app.upgrading());
}

#[test]
fn a_tap_and_a_scripted_press_leave_bip85_the_same_way() {
    let mut tapped = with_key();
    let mut pressed = with_key();
    let shown = bip85_shown(&mut tapped);
    assert_eq!(bip85_shown(&mut pressed), shown);
    tap(&mut tapped, Action::Nav(Screen::Settings));
    pressed.press(Action::Nav(Screen::Settings));
    assert_eq!(tapped.screen, pressed.screen);
    assert_eq!(tapped.bip85().is_none(), pressed.bip85().is_none());
    assert!(tapped.bip85().is_none());
    assert_eq!(texts(&mut tapped), texts(&mut pressed));
}

#[test]
fn new_key_left_by_a_tap_on_its_back_link_forgets_its_entries() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    let words = app.keygen.as_ref().unwrap().words as u8;
    app.press(Action::KWords(words));
    app.press(Action::KNext);
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::DiceFlips.index()));
    app.press(Action::KNext);
    app.press(Action::KRoll(4));
    // Its back link, to Add a key it was opened from.
    tap(&mut app, Action::Nav(Screen::Entry));
    assert_eq!(app.screen, Screen::Entry);
    assert!(app.keygen.is_none(), "New key keeps nothing when left");
    app.press(Action::KeyGen(None));
    assert_eq!(app.keygen.as_ref().unwrap().dice.len(), 0);
}
