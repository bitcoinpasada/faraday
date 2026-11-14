//! A power-off or a restart leaves nothing in the RAM directory the Inbox
//! and Outbox wait in: a restart keeps the memory powered, and a seed file
//! copied in from a stick or a secret put For the stick would be in the
//! clear in it. A lock keeps what is not secret for the next process.

use std::fs;
use std::path::{Path, PathBuf};

use faraday_core::testkit::{self, Plain};
use faraday_core::{Action, Faraday, Medium, StorageEvent, leaks};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_storage::{Boxes, Dirs, serve_with};

const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("faraday-exit-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn files_under(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for sub in ["inbox", "outbox", "kept"] {
        if let Ok(d) = fs::read_dir(dir.join(sub)) {
            found.extend(
                d.flatten()
                    .map(|e| format!("{sub}/{}", e.file_name().display())),
            );
        }
    }
    found
}

/// An app on the stick with a seed-words file in its Inbox and a secret
/// put For the stick in its Outbox, both written to `dir`.
fn with_files(dir: &Path) -> (Faraday, Boxes) {
    with_boxes(dir, true, WORDS.as_bytes())
}

/// The same, with or without the Outbox's secret (an Outbox with a file
/// waiting keeps the idle power-off from going ahead).
fn with_boxes(dir: &Path, outbox_secret: bool, inbox_bytes: &[u8]) -> (Faraday, Boxes) {
    let mut app = testkit::started();
    app.medium = Medium::Stick;
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("seed.txt"),
            inbox_bytes.to_vec(),
        )]),
        outbox: faraday_core::files_from(if outbox_secret {
            vec![(faraday_text::Text::of("key.txt"), WORDS.as_bytes().to_vec())]
        } else {
            Vec::new()
        }),
        kept: Vec::new(),
    });
    let mut boxes = Boxes::Dir(dir.to_path_buf());
    // Any change the app keeps writes the boxes.
    app.press(Action::IdleLock(5));
    serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
    assert!(
        files_under(dir).contains(&"inbox/seed.txt".to_string()),
        "the seed file waits: {:?}",
        files_under(dir)
    );
    (app, boxes)
}

fn last_frame(app: &mut Faraday) -> Vec<String> {
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
    let _ = app.frame();
    app.drawn_texts().plain()
}

#[test]
fn restart_and_power_off_leave_no_file_in_the_ram_directory() {
    for (name, action) in [
        ("restart", Action::Restart),
        ("off", Action::PowerOff),
        ("hold-off", Action::HoldPowerOff),
        ("hold-restart", Action::HoldRestart),
    ] {
        let dir = scratch(name);
        let (mut app, mut boxes) = with_files(&dir);
        leaks::plant(&[("seed words", WORDS)]);
        app.press(Action::PowerAsk);
        app.press(action);
        serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
        assert!(
            files_under(&dir).is_empty(),
            "{name}: {:?}",
            files_under(&dir)
        );
        let shown = last_frame(&mut app);
        assert!(leaks::find_forms(&shown).is_empty(), "{name}: {shown:?}");
        let _ = fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_power_sheet_leaves_no_file_when_it_is_not_asked_first() {
    // Nothing waiting in the Outbox: the Power button's Power off ends
    // the app at once.
    let dir = scratch("direct");
    let (mut app, mut boxes) = with_boxes(&dir, false, WORDS.as_bytes());
    app.press(Action::PowerAsk);
    app.press(Action::PowerOff);
    serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
    assert!(files_under(&dir).is_empty(), "{:?}", files_under(&dir));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_idle_power_off_leaves_no_file_in_the_ram_directory() {
    let dir = scratch("idle");
    let (mut app, mut boxes) = with_boxes(&dir, false, b"meeting notes");
    app.press(Action::IdleOff(1));
    serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
    let start = app.now_ms();
    // The first tick starts the idle clock.
    app.event(Event::Tick { now_ms: start });
    app.event(Event::Tick {
        now_ms: start + 61_000,
    });
    let exited = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, faraday_shell_api::Command::Exit));
    assert!(exited, "the idle power-off did not end the app");
    serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
    assert!(files_under(&dir).is_empty(), "{:?}", files_under(&dir));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_lock_keeps_the_outbox_secret_for_the_next_process() {
    let dir = scratch("lock-keeps");
    let (mut app, mut boxes) = with_files(&dir);
    app.press(Action::LockUnsealed);
    serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
    assert!(
        files_under(&dir).contains(&"outbox/key.txt".to_string()),
        "{:?}",
        files_under(&dir)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_lock_drops_the_seed_file() {
    let dir = scratch("lock");
    let (mut app, mut boxes) = with_files(&dir);
    app.press(Action::LockUnsealed);
    serve_with(&mut app, &mut boxes, None, &mut Dirs::default());
    let left = files_under(&dir);
    assert!(!left.contains(&"inbox/seed.txt".to_string()), "{left:?}");
    let _ = fs::remove_dir_all(&dir);
}
