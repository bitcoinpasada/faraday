//! The idle timers (F58): a session left alone is warned
//! at twenty-five minutes, with a countdown, and locks at thirty; a
//! machine left alone with nothing held powers off at sixty, unless the
//! Outbox still holds what would be lost. Make a new wallet holds the
//! idle lock until its keys are kept somewhere.

use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};

const MIN: u64 = 60_000;

fn with_wallet() -> Faraday {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    app
}

fn tick(app: &mut Faraday, ms: u64) -> Vec<Command> {
    app.event(Event::Tick { now_ms: ms });
    std::iter::from_fn(|| app.poll_command()).collect()
}

/// What the app asked the shell to keep, last.
fn kept(app: &mut Faraday) -> Vec<(String, Vec<u8>)> {
    let mut last = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept, .. } = c {
            last = faraday_core::testkit::plain_kept(kept);
        }
    }
    last
}

fn after_idle_lock(outbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = with_wallet();
    tick(&mut app, 0);
    tick(&mut app, 30 * MIN);
    assert!(app.restart_requested());
    let kept = kept(&mut app);
    let mut next = Faraday::new();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(outbox),
        kept: faraday_core::kept_from(kept),
    });
    next
}

#[test]
fn a_session_left_alone_is_warned_at_twenty_five_minutes_and_locks_at_thirty() {
    let mut app = with_wallet();
    tick(&mut app, 0);
    tick(&mut app, 24 * MIN);
    assert_eq!(app.sheet, None);
    tick(&mut app, 25 * MIN);
    assert_eq!(
        app.sheet,
        Some(Sheet::IdleWarn),
        "the warning comes at twenty-five"
    );
    assert!(!tick(&mut app, 29 * MIN).contains(&Command::Exit));
    assert!(tick(&mut app, 30 * MIN).contains(&Command::Exit));
    assert!(
        app.restart_requested(),
        "an idle lock restarts, it does not power off"
    );
}

#[test]
fn input_puts_the_lock_off() {
    let mut app = with_wallet();
    tick(&mut app, 0);
    tick(&mut app, 26 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.event(Event::Key(Key::Char('a')));
    assert_eq!(app.sheet, None, "a key closes the warning");
    assert!(!tick(&mut app, 55 * MIN).contains(&Command::Exit));
    assert!(tick(&mut app, 56 * MIN).contains(&Command::Exit));
}

#[test]
fn the_key_that_closes_the_warning_types_nothing() {
    let mut app = with_wallet();
    assert_eq!(app.screen, Screen::Wallets);
    app.press(Action::Rename);
    app.event(Event::Key(Key::Char('a')));
    let before = app.renaming().cloned();
    assert!(
        before.as_ref().is_some_and(|n| n.ends_with('a')),
        "typing reaches the name"
    );
    tick(&mut app, 0);
    tick(&mut app, 26 * MIN);
    app.event(Event::Key(Key::Char('x')));
    assert_eq!(app.renaming().cloned(), before);
}

#[test]
fn the_idle_warning_carries_no_clock() {
    let mut app = with_wallet();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    tick(&mut app, 0);
    tick(&mut app, 25 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    // The line, read across the rows the dialog wraps it onto.
    let body = |texts: &[String]| -> String {
        let all = texts.join(" ");
        let from = all.find("No input for").unwrap_or(all.len());
        let rest = &all[from..];
        let end = rest
            .find("keeps it")
            .map_or(rest.len(), |e| e + "keeps it".len());
        rest[..end].to_string()
    };
    let first = app.drawn_texts().plain();
    assert!(
        first.iter().any(|t| t == "Locking soon"),
        "no countdown title: {first:?}"
    );
    let first_body = body(&first);
    // The minutes without input when it opened, and the lock setting: two
    // durations, never a clock time.
    let lock = app.idle_lock_min();
    assert_eq!(
        first_body,
        format!(
            "No input for 25 minutes · locks at {lock} minutes without input · any key or touch keeps it"
        ),
    );
    tick(&mut app, 25 * MIN + 1_000);
    let second = app.drawn_texts().plain();
    assert_eq!(
        first_body,
        body(&second),
        "the sheet's text does not change from one second to the next"
    );
}

#[test]
fn never_means_never() {
    let mut app = with_wallet();
    app.press(Action::IdleLock(0));
    tick(&mut app, 0);
    assert!(!tick(&mut app, 120 * MIN).contains(&Command::Exit));
}

/// A tool holding something typed that is secret, with no key or vault
/// loaded, is warned and locked like any other session.
#[test]
fn a_secret_typed_in_a_tool_is_locked_at_thirty_minutes_with_no_key_loaded() {
    use faraday_core::kef_tool::{KefToolAction as K, Pick};
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::KefTool(K::Pick(Pick::Type)));
    for c in "a note for later".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(testkit::secret_typed(&app));
    tick(&mut app, 0);
    tick(&mut app, 25 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    assert!(tick(&mut app, 30 * MIN).contains(&Command::Exit));
    assert!(app.restart_requested());
}

#[test]
fn after_an_idle_lock_the_machine_powers_off_at_sixty_minutes() {
    let mut app = after_idle_lock(Vec::new());
    assert_eq!(app.sheet, Some(Sheet::Locked));
    // Thirty minutes already passed before the lock.
    tick(&mut app, 0);
    assert!(!tick(&mut app, 29 * MIN).contains(&Command::Exit));
    assert!(tick(&mut app, 30 * MIN).contains(&Command::Exit));
    assert!(!app.restart_requested());
}

#[test]
fn a_full_outbox_holds_the_power_off_and_says_why() {
    let mut app = after_idle_lock(vec![("vault.kdbx".into(), vec![1, 2, 3])]);
    app.press(Action::Cancel);
    tick(&mut app, 0);
    assert!(!tick(&mut app, 120 * MIN).contains(&Command::Exit));
    assert_eq!(app.sheet, Some(Sheet::Locked));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn the_desktop_app_never_powers_itself_off() {
    let mut app = Faraday::new();
    app.online = true;
    tick(&mut app, 0);
    assert!(!tick(&mut app, 120 * MIN).contains(&Command::Exit));
}

#[test]
fn the_idle_power_off_shows_no_power_sheet_even_with_secure_boot_off() {
    let mut app = after_idle_lock(Vec::new());
    app.secure_boot = faraday_core::SecureBootAtStart::Off;
    tick(&mut app, 0);
    assert!(tick(&mut app, 30 * MIN).contains(&Command::Exit));
    assert_ne!(app.sheet, Some(Sheet::Power));
}

/// Make a new wallet begun with a key typed in, its Kind answered.
fn creating() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 0);
    app
}

#[test]
fn make_a_new_wallet_holds_the_idle_lock_and_says_so() {
    let c = testkit::compute();
    let mut app = creating();
    assert!(app.lock_held(&c));
    tick(&mut app, 0);
    tick(&mut app, 25 * MIN);
    assert_eq!(app.sheet, None, "no warning");
    assert!(!tick(&mut app, 120 * MIN).contains(&Command::Exit));
    assert!(!app.restart_requested());
    let t = app.drawn_texts();
    assert!(
        t.iter().any(|t| t == "Auto-lock off until backup complete"),
        "{t:?}"
    );
}

/// Every text drawn now.
fn texts(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts().plain()
}

#[test]
fn words_on_show_while_make_a_new_wallet_holds_the_lock_are_masked_at_the_warning_time() {
    let c = testkit::compute();
    let mut app = creating();
    crate::common::add_a_key_flow(&mut app);
    app.press(Action::KStart(faraday_core::keygen::Start::Words.index()));
    let _ = app.frame();
    for ch in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(ch)));
    }
    app.press(Action::KNext);
    let before = texts(&mut app);
    app.press(Action::KShow);
    let after = texts(&mut app);
    let words: Vec<String> = after.into_iter().filter(|t| !before.contains(t)).collect();
    assert!(!words.is_empty(), "Show words shows the words");
    assert!(app.lock_held(&c));
    tick(&mut app, 0);
    tick(&mut app, 24 * MIN);
    let now = texts(&mut app);
    assert!(words.iter().all(|w| now.contains(w)), "not before the time");
    tick(&mut app, 25 * MIN);
    let now = texts(&mut app);
    assert!(
        !words.iter().any(|w| now.contains(w)),
        "the words are masked"
    );
    assert_eq!(app.sheet, None, "no warning sheet");
    assert!(app.lock_held(&c), "the flow is still held");
    assert!(app.keygen().is_some(), "and the key is still there");
    assert!(!tick(&mut app, 120 * MIN).contains(&Command::Exit));
    assert!(!app.restart_requested());
    app.press(Action::KShow);
    let now = texts(&mut app);
    assert!(
        words.iter().all(|w| now.contains(w)),
        "Show words shows them again"
    );
}

#[test]
fn once_make_a_new_wallet_ends_the_idle_lock_comes_back() {
    let c = testkit::compute();
    let mut app = creating();
    app.press(Action::CancelFlow);
    if app.sheet == Some(Sheet::CancelFlow(faraday_core::in_progress::Going::Create)) {
        app.press(Action::CancelFlowNow);
    }
    assert!(app.create().is_none());
    assert!(!app.lock_held(&c));
    tick(&mut app, 0);
    assert!(tick(&mut app, 30 * MIN).contains(&Command::Exit));
    let t = app.drawn_texts();
    assert!(!t.iter().any(|t| t == "Auto-lock off until backup complete"));
}

#[test]
fn a_lock_while_held_says_it_ends_make_a_new_wallet_and_ends_it() {
    let c = testkit::compute();
    let mut app = creating();
    app.press(Action::LockAsk);
    assert_eq!(app.sheet, Some(Sheet::LockAsk), "the lock asks first");
    let t = app.drawn_texts();
    let wiped = t.iter().position(|s| s == "Wiped from memory");
    let ends = t.iter().position(|s| s == "Make a new wallet · ends");
    assert!(wiped.is_some() && ends > wiped, "{t:?}");
    app.press(Action::Lock);
    assert!(app.restart_requested());
    let kept = kept(&mut app);
    let mut next = Faraday::new();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept: faraday_core::kept_from(kept),
    });
    assert!(next.create().is_none());
    assert!(
        next.flows_in_progress_now(&c).is_empty(),
        "nothing to come back to"
    );
}

#[test]
fn the_idle_warning_comes_over_the_code_sheet_and_takes_the_code_off_the_screen() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    let w = app
        .session
        .add_wallet_confirmed(
            &c,
            "Savings",
            &testkit::savings(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("a wallet");
    tick(&mut app, 0);
    app.press(Action::PublicOut(w, 12));
    app.press(Action::ShowCode(faraday_core::Code::SpecterDiy(w)));
    assert_eq!(app.sheet, Some(Sheet::Qr));
    assert!(app.qr().is_some());
    tick(&mut app, 25 * MIN);
    assert!(app.qr().is_none(), "the code is gone");
    assert_eq!(app.sheet, Some(Sheet::IdleWarn), "the warning counts down");
}

#[test]
fn the_sheet_under_the_idle_warning_is_back_once_the_warning_is_put_away() {
    let mut app = with_wallet();
    tick(&mut app, 0);
    app.press(Action::Guide);
    let under = app.sheet;
    assert_eq!(under, Some(Sheet::Guide));
    tick(&mut app, 26 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.event(Event::Key(Key::Char('a')));
    assert_eq!(app.sheet, under, "the guide is where it was");
}

const SEED_WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn locked_texts(app: &mut Faraday) -> String {
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.drawn_texts().plain().join(" ")
}

#[test]
fn a_lock_names_a_secret_in_the_clear_still_waiting() {
    let mut app = after_idle_lock(vec![("seed.txt".into(), SEED_WORDS.as_bytes().to_vec())]);
    assert_eq!(app.sheet, Some(Sheet::Locked));
    let text = locked_texts(&mut app);
    assert!(text.contains("Locked after a time without input"), "{text}");
    assert!(text.contains("with a secret in the clear"), "{text}");
}

#[test]
fn a_waiting_file_with_no_secret_is_not_named_as_one() {
    let mut app = after_idle_lock(vec![("vault.kdbx".into(), vec![1, 2, 3])]);
    let text = locked_texts(&mut app);
    assert!(!text.contains("secret in the clear"), "{text}");
}

#[test]
fn the_sheet_held_up_with_no_lock_does_not_say_keys_were_wiped() {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("seed.txt"),
            SEED_WORDS.as_bytes().to_vec(),
        )]),
        kept: Vec::new(),
    });
    tick(&mut app, 0);
    tick(&mut app, 120 * MIN);
    assert_eq!(app.sheet, Some(Sheet::Locked));
    let text = locked_texts(&mut app);
    assert!(!text.contains("wiped"), "{text}");
    assert!(!text.contains("Locked after"), "{text}");
}
