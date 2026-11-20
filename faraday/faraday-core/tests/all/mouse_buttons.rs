//! The mouse's back and forward buttons:
//! back does what the screen's back link does, or Escape where there is
//! none; forward opens again the screen back left, as it was left, until
//! something else is opened. A screen that keeps nothing when it is left
//! (New key) has nothing for forward to open.

use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App, Event, Key, TouchPhase};

/// New key opened from Add a key, on its first card.
fn new_key() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    app.press(Action::KStart(
        faraday_core::keygen::Start::Make(faraday_core::forms::Form::Words).index(),
    ));
    let _ = app.frame();
    app
}

fn button(app: &mut Faraday, e: Event) {
    app.event(e);
    let _ = app.frame();
}

#[test]
fn back_opens_the_step_before_in_make_a_new_wallet_and_forward_opens_it_again() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    app.press(Action::CKind(1));
    let kind = app.create().unwrap().kind;
    let open = app.create().unwrap().open;
    assert_ne!(open, Some(faraday_core::cstep::KIND));
    let _ = app.frame();
    // From a later step Back opens Kind, its choice as it was.
    button(&mut app, Event::Back);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().open, Some(faraday_core::cstep::KIND));
    assert_eq!(app.create().unwrap().kind, kind);
    button(&mut app, Event::Forward);
    assert_eq!(app.create().unwrap().open, open, "Forward opens it again");
    button(&mut app, Event::Back);
    // From the first step it leaves, and Forward comes back as it was.
    button(&mut app, Event::Back);
    assert_ne!(app.screen, Screen::Create);
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().kind, kind, "kept as it was left");
}

#[test]
fn the_back_arrow_opens_the_step_before_in_make_a_new_wallet() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(1));
    let _ = app.frame();
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().unwrap().open, Some(faraday_core::cstep::KIND));
    app.press(Action::Back);
    assert_ne!(app.screen, Screen::Create);
}

#[test]
fn new_key_left_by_back_keeps_nothing_for_forward_to_open() {
    let mut app = new_key();
    app.press(Action::KWords(24));
    let _ = app.frame();
    // Back steps within New key first (F104): Length, then Start, each
    // as it was chosen; from Start it leaves.
    let open = |app: &Faraday| app.keygen().and_then(|k| k.open);
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(faraday_core::keygen::kstep::LENGTH));
    assert_eq!(app.keygen().unwrap().words, 24, "as it was chosen");
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(faraday_core::keygen::kstep::START));
    button(&mut app, Event::Back);
    assert_eq!(
        app.screen,
        Screen::Entry,
        "from Start, New key's back link is Add a key"
    );
    assert!(app.keygen().is_none(), "New key keeps nothing when left");
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn forward_does_nothing_once_another_screen_is_opened() {
    let mut app = new_key();
    // Length, then Start: Back from Start leaves.
    button(&mut app, Event::Back);
    button(&mut app, Event::Back);
    assert_eq!(app.screen, Screen::Entry);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    button(&mut app, Event::Forward);
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn back_closes_a_sheet_as_escape_does_and_leaves_the_screen() {
    let mut app = new_key();
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    let _ = app.frame();
    button(&mut app, Event::Back);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
}

#[test]
fn escape_leaves_a_page_as_the_back_button_does() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Settings);
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    let _ = app.frame();
    assert_ne!(app.screen, Screen::Settings);
}

#[test]
fn escape_in_new_key_steps_back_and_then_leaves_as_the_back_button_does() {
    let mut app = new_key();
    let open = |app: &Faraday| app.keygen().and_then(|k| k.open);
    let first = open(&app);
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    let _ = app.frame();
    assert_ne!(open(&app), first, "the step before");
    for _ in 0..4 {
        if app.screen != Screen::KeyGen {
            break;
        }
        app.event(Event::Key(faraday_shell_api::Key::Escape));
        let _ = app.frame();
    }
    assert_eq!(app.screen, Screen::Entry);
}

/// The three ways out of a screen: the arrow, Escape, the mouse's back.
#[derive(Clone, Copy, Debug)]
enum Way {
    Arrow,
    Escape,
    Mouse,
}

const WAYS: [Way; 3] = [Way::Arrow, Way::Escape, Way::Mouse];

fn leave(app: &mut Faraday, way: Way) {
    let _ = app.frame();
    match way {
        Way::Arrow => app.press(Action::Back),
        Way::Escape => app.event(Event::Key(Key::Escape)),
        Way::Mouse => app.event(Event::Back),
    }
    let _ = app.frame();
}

#[test]
fn a_vault_page_is_left_by_escape_and_the_mouse_back_as_by_the_arrow() {
    let mut ends = Vec::new();
    for way in WAYS {
        let mut app = testkit::started();
        app.storage(faraday_core::StorageEvent::Restored {
            inbox: faraday_core::files_from(vec![(
                "vault.kdbx".to_string(),
                faraday_core::testkit::test_vault().unwrap(),
            )]),
            outbox: Vec::new(),
            kept: Vec::new(),
        });
        app.press(Action::Nav(Screen::Vaults));
        assert_eq!(app.screen, Screen::Vaults);
        leave(&mut app, way);
        assert_ne!(app.screen, Screen::Vaults, "{way:?}");
        ends.push(app.screen);
    }
    assert!(ends.iter().all(|s| *s == ends[0]), "{ends:?}");
}

/// A stick with a file, on Files, and its free space chosen to be zeroed.
fn zeroing_sheet() -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("S"),
        label: faraday_text::Text::of("TESTSTICK"),
        boot: false,
        files: vec![(faraday_text::Text::of("photo.png"), 9000)],
        vaults: Vec::new(),
    }]));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::DeleteFiles);
    app.press(Action::DeleteWhat(1));
    app.storage(StorageEvent::StickSpace {
        stick: faraday_text::Text::of("S"),
        free: 3 << 30,
        size: 7 << 30,
    });
    let _ = app.frame();
    let hold = Action::Vault(faraday_core::vault::VaultAction::HoldWipe);
    let (x, y) = app.where_offered(hold).expect("the hold is on the sheet");
    app.event(Event::Tick { now_ms: 1_000 });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    app.event(Event::Tick {
        now_ms: 1_100 + faraday_core::vault::HOLD_MS,
    });
    app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    assert_eq!(app.sheet, Some(Sheet::Zeroing));
    app
}

/// A sheet's own Cancel, Escape and the mouse's back: the same, and the
/// screen under it stays. `closes` is whether Cancel closes it (the
/// Published sheet's caution has no way past it).
fn sheet_closes_alike(open: impl Fn() -> Faraday, closes: bool) {
    for way in WAYS {
        let mut app = open();
        let screen = app.screen;
        match way {
            Way::Arrow => app.press(Action::Cancel),
            _ => leave(&mut app, way),
        }
        let after = if closes { None } else { Some(Sheet::Published) };
        assert_eq!(app.sheet, after, "{way:?}");
        assert_eq!(app.screen, screen, "{way:?}: the sheet only");
    }
}

#[test]
fn the_zeroing_sheet_closes_on_escape_and_the_mouse_back_as_on_its_cancel() {
    sheet_closes_alike(zeroing_sheet, true);
}

#[test]
fn the_published_sheet_answers_escape_and_the_mouse_back_as_its_cancel_does_by_staying() {
    // The caution has no way past it but Keep for testing or Remove.
    sheet_closes_alike(
        || {
            let mut app = testkit::started();
            testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
            app.press(Action::Entry(None));
            // BIP-39's second vector: published, not a sample Mainnet refuses.
            let words =
                "legal winner thank year wave sausage worth useful legal winner thank yellow";
            for c in words.chars() {
                app.event(Event::Key(Key::Char(c)));
            }
            app.press(Action::EntryAdd);
            assert_eq!(app.sheet, Some(Sheet::Published));
            app
        },
        false,
    );
}

#[test]
fn words_typed_in_add_a_key_are_gone_after_escape_and_the_mouse_back() {
    for way in [Way::Escape, Way::Mouse] {
        let mut app = testkit::started();
        app.press(Action::Entry(None));
        for c in "orange harbour".chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        assert_eq!(&*testkit::entry(&app).typed, "orange harbour");
        leave(&mut app, way);
        assert_ne!(app.screen, Screen::Entry, "{way:?}");
        assert_eq!(&*testkit::entry(&app).typed, "", "{way:?}: kept in state");
        app.press(Action::Entry(None));
        let _ = app.frame();
        let drawn = app.drawn_texts().join(" ");
        assert!(!drawn.contains("orange"), "{way:?}: drawn again");
        assert_eq!(&*testkit::entry(&app).typed, "", "{way:?}");
    }
}
