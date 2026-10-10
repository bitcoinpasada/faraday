//! The idle timers (`PLAN.md` §12.3): a session left alone is warned at
//! five minutes, with a countdown, and locks at ten; a machine left alone
//! with nothing held powers off at twenty, unless the Outbox still holds
//! what would be lost.

use faraday_core::{Action, Faraday, Screen, Sheet, StorageCommand, StorageEvent, testkit};
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, Key, SecureHardware};

const MIN: u64 = 60_000;

fn with_wallet() -> Faraday {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
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
            last = kept;
        }
    }
    last
}

fn after_idle_lock(outbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = with_wallet();
    tick(&mut app, 0);
    tick(&mut app, 10 * MIN);
    assert!(app.restart_requested());
    let kept = kept(&mut app);
    let mut next = Faraday::new();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox,
        kept,
    });
    next
}

#[test]
fn a_session_left_alone_is_warned_at_five_minutes_and_locks_at_ten() {
    let mut app = with_wallet();
    tick(&mut app, 0);
    tick(&mut app, 4 * MIN);
    assert_eq!(app.sheet, None);
    tick(&mut app, 5 * MIN);
    assert_eq!(
        app.sheet,
        Some(Sheet::IdleWarn),
        "the warning comes at five"
    );
    assert!(!tick(&mut app, 9 * MIN).contains(&Command::Exit));
    assert!(tick(&mut app, 10 * MIN).contains(&Command::Exit));
    assert!(
        app.restart_requested(),
        "an idle lock restarts, it does not power off"
    );
}

#[test]
fn input_puts_the_lock_off() {
    let mut app = with_wallet();
    tick(&mut app, 0);
    tick(&mut app, 6 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.event(Event::Key(Key::Char('a')));
    assert_eq!(app.sheet, None, "a key closes the warning");
    assert!(!tick(&mut app, 15 * MIN).contains(&Command::Exit));
    assert!(tick(&mut app, 16 * MIN).contains(&Command::Exit));
}

#[test]
fn the_key_that_closes_the_warning_types_nothing() {
    let mut app = with_wallet();
    assert_eq!(app.screen, Screen::Wallets);
    app.press(Action::Rename);
    app.event(Event::Key(Key::Char('a')));
    let before = app.renaming.clone();
    assert!(
        before.as_ref().is_some_and(|n| n.ends_with('a')),
        "typing reaches the name"
    );
    tick(&mut app, 0);
    tick(&mut app, 6 * MIN);
    app.event(Event::Key(Key::Char('x')));
    assert_eq!(app.renaming, before);
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
    tick(&mut app, 5 * MIN);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    let body = |texts: &[String]| -> String {
        texts
            .iter()
            .find(|t| t.contains("No input for"))
            .cloned()
            .unwrap_or_default()
    };
    let first = app.drawn_texts();
    assert!(
        first.iter().any(|t| t == "Locking soon"),
        "no countdown title: {first:?}"
    );
    let first_body = body(&first);
    // The minutes without input when it opened, and the lock setting: two
    // durations, never a clock time.
    let lock = app.idle_lock_min;
    assert_eq!(
        first_body,
        format!(
            "No input for 5 minutes · locks at {lock} minutes without input · any key or touch keeps it"
        ),
    );
    tick(&mut app, 5 * MIN + 1_000);
    let second = app.drawn_texts();
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

#[test]
fn after_an_idle_lock_the_machine_powers_off_at_twenty_minutes() {
    let mut app = after_idle_lock(Vec::new());
    assert_eq!(app.sheet, Some(Sheet::Locked));
    // Ten minutes already passed before the lock.
    tick(&mut app, 0);
    assert!(!tick(&mut app, 9 * MIN).contains(&Command::Exit));
    assert!(tick(&mut app, 10 * MIN).contains(&Command::Exit));
    assert!(!app.restart_requested());
}

#[test]
fn a_full_outbox_holds_the_power_off_and_says_why() {
    let mut app = after_idle_lock(vec![("vault.ofv".into(), vec![1, 2, 3])]);
    app.press(Action::Cancel);
    tick(&mut app, 0);
    assert!(!tick(&mut app, 30 * MIN).contains(&Command::Exit));
    assert_eq!(app.sheet, Some(Sheet::Locked));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn the_desktop_app_never_powers_itself_off() {
    let mut app = Faraday::new();
    app.online = true;
    tick(&mut app, 0);
    assert!(!tick(&mut app, 60 * MIN).contains(&Command::Exit));
}
