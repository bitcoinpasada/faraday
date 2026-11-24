//! A lock drops the camera's scan whole: its picture (which may show a
//! SeedQR), the code read, the sheet over the page and the camera.

use faraday_core::{Action, Faraday, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, Command, Event};

fn frame_event() -> Event {
    Event::CameraFrame {
        width: 4,
        height: 4,
        luma: vec![7; 16],
        chroma: None,
    }
}

fn commands(app: &mut Faraday) -> Vec<Command> {
    std::iter::from_fn(|| app.poll_command()).collect()
}

/// Locks as the person does, and as the shell's next tick settles it.
fn locked(app: &mut Faraday) {
    app.press(Action::LockUnsealed);
    testkit::settle(app);
    let _ = app.frame();
}

fn nothing_of_the_scan(app: &mut Faraday) {
    nothing_of_the_scan_held(app);
    assert!(commands(app).contains(&Command::CameraOff));
}

fn nothing_of_the_scan_held(app: &mut Faraday) {
    assert!(
        app.scan().is_none(),
        "the scan is still held after the lock"
    );
    assert_ne!(app.sheet, Some(Sheet::Scan));
}

fn with_picture(open: Action) -> Faraday {
    let mut app = testkit::started();
    app.press(open);
    app.event(frame_event());
    let _ = app.frame();
    assert!(app.scan().is_some_and(|s| s.frame.is_some()));
    let _ = commands(&mut app);
    app
}

#[test]
fn lock_leaves_no_scan_when_the_sheet_stood() {
    let mut app = with_picture(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    locked(&mut app);
    nothing_of_the_scan(&mut app);
}

#[test]
fn lock_leaves_no_scan_when_a_step_s_scan_stood() {
    // The scan opened from a key step, over the step.
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::ScanSeed);
    app.event(frame_event());
    let _ = app.frame();
    assert!(app.scan().is_some_and(|s| s.frame.is_some()));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    let _ = commands(&mut app);
    locked(&mut app);
    nothing_of_the_scan(&mut app);
}

#[test]
fn a_power_off_leaves_no_scan() {
    let mut app = with_picture(Action::Scan);
    app.medium = faraday_core::Medium::Stick;
    app.press(Action::PowerAsk);
    app.press(Action::PowerOff);
    testkit::settle(&mut app);
    let _ = app.frame();
    nothing_of_the_scan_held(&mut app);
}

#[test]
fn a_lock_waiting_on_a_seal_drops_the_scan_at_once() {
    let mut app =
        crate::vault_seal::device(crate::common::vault("Test", testkit::VAULT_PASSPHRASE, 7));
    crate::vault_seal::open_and_change(&mut app);
    app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
        [0x21; 32],
    )));
    crate::common::top_up(&mut app);
    app.press(Action::Scan);
    app.event(frame_event());
    let _ = app.frame();
    assert!(app.scan().is_some_and(|s| s.frame.is_some()));
    let _ = commands(&mut app);
    testkit::hold_worker();
    assert!(
        !testkit::vault_lock_one(&mut app, 0),
        "the seal is under way"
    );
    testkit::press_on_worker(&mut app, Action::Lock);
    // The seal still runs: the device has not locked, the scan is gone.
    assert!(!app.restart_requested(), "the lock did not wait");
    nothing_of_the_scan(&mut app);
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.restart_requested(), "the device did not lock");
}

#[test]
fn lock_leaves_no_read_card_scan() {
    // A step's scanner for one kind shows the card for text it does not take.
    let mut app = with_picture(Action::ScanSeed);
    app.event(Event::Scanned {
        bytes: b"some plain text read".to_vec(),
    });
    let _ = app.frame();
    assert!(app.scan().is_some_and(|s| s.card.is_some()));
    let _ = commands(&mut app);
    locked(&mut app);
    // The camera went off when the card showed.
    nothing_of_the_scan_held(&mut app);
}

#[test]
fn the_idle_lock_leaves_no_scan() {
    const MIN: u64 = 60_000;
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
    app.press(Action::Scan);
    app.event(frame_event());
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 0 });
    // Inside the warning period: the warning sheet is over the scan.
    app.event(Event::Tick {
        now_ms: 29 * MIN + 30_000,
    });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.event(Event::Tick { now_ms: 30 * MIN });
    let _ = app.frame();
    nothing_of_the_scan(&mut app);
}

#[test]
fn nothing_of_the_old_scan_returns_after_back_or_a_press() {
    // The first part of a transfer in parts, read on the scan sheet.
    let text = "A message long enough to come in several parts, one code after another.";
    let parts = faraday_qr::bbqr::encode('U', text.as_bytes(), 20).expect("parts");
    assert!(parts.len() > 1);
    let mut app = testkit::started();
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: parts[0].as_bytes().to_vec(),
    });
    let _ = app.frame();
    assert!(
        app.scan()
            .is_some_and(|s| s.reads > 0 && s.parts_read().is_some())
    );
    locked(&mut app);
    // A late picture and a late code find no scan and file nothing.
    app.event(frame_event());
    app.event(Event::Scanned {
        bytes: parts[1].as_bytes().to_vec(),
    });
    let _ = app.frame();
    assert!(app.scan().is_none());
    assert_ne!(app.sheet, Some(Sheet::Scan));
    app.press(Action::Cancel);
    app.press(Action::Scan);
    let _ = app.frame();
    assert!(app.scan().is_some_and(|s| {
        s.frame.is_none() && s.reads == 0 && s.note.is_none() && s.parts_read().is_none()
    }));
}
