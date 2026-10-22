//! A mode change empties Files, From the stick and For the stick, with
//! the keys and wallets, and carries nothing but the mode; a lock keeps
//! the files (F81, `docs/MODES.md` §2).

use faraday_bip::keys::Network;
use faraday_core::wallet::Session;
use faraday_core::{
    Action, DESKTOP_MODES, Faraday, Mode, STICK_MODES, Screen, StorageCommand, StorageEvent,
    testkit,
};

fn with_files(mut app: Faraday) -> Faraday {
    let kit = testkit::kits().into_iter().next().unwrap();
    let psbt = testkit::unsigned(&kit).unwrap().to_bytes();
    app.storage(StorageEvent::Restored {
        inbox: vec![("scanned-1.psbt".to_string(), psbt)],
        outbox: vec![("note.txt".to_string(), b"a note".to_vec())],
        kept: Vec::new(),
    });
    assert_eq!(app.inbox.len(), 1);
    assert_eq!(app.outbox.len(), 1);
    app
}

fn demo_app() -> Faraday {
    let mut app = testkit::started();
    app.online = true;
    app.session = Session::on(Network::Testnet);
    app.session.set_demo(testkit::demo_samples());
    app.set_modes(DESKTOP_MODES);
    with_files(app)
}

/// The boxes the app last asked the shell to keep.
fn saved(app: &mut Faraday) -> Option<(usize, usize)> {
    let mut last = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { inbox, outbox, .. } = c {
            last = Some((inbox.len(), outbox.len()));
        }
    }
    last
}

#[test]
fn a_mode_change_empties_both_boxes_and_the_session() {
    let mut app = demo_app();
    let _ = saved(&mut app);
    app.press(Action::Mode(Mode::Testnet));
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Testnet);
    assert!(app.inbox.is_empty(), "From the stick");
    assert!(app.outbox.is_empty(), "For the stick");
    assert!(app.session.keys.is_empty());
    assert!(app.session.wallets.is_empty());
    assert_eq!(saved(&mut app), Some((0, 0)), "nothing is carried over");
}

#[test]
fn a_plain_lock_in_demo_keeps_both_boxes() {
    let mut app = demo_app();
    let _ = saved(&mut app);
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(saved(&mut app), Some((1, 1)));
}

#[test]
fn a_mode_change_on_the_stick_carries_no_boxes_either() {
    let app = testkit::started();
    assert_eq!(app.modes(), STICK_MODES);
    let mut app = with_files(app);
    let _ = saved(&mut app);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Mode(Mode::Signet));
    assert!(app.restart_requested());
    assert_eq!(saved(&mut app), Some((0, 0)));
}
