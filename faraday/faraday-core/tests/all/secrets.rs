//! A secret leaves this device sealed in a vault unless the person says
//! otherwise after a warning.

use crate::common::type_text;
use faraday_core::secrets::Exposure;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    // A round draws its nonces from the system's randomness, which the
    // shell answers at the first display.
    app.event(Event::Entropy(EntropyBytes::new([5u8; 32])));
    app
}

fn find(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
}

/// What the last box save before a lock handed the next process.
type Boxes = (
    Vec<(String, Vec<u8>)>,
    Vec<(String, Vec<u8>)>,
    Vec<(String, Vec<u8>)>,
);

fn locked(app: &mut Faraday) -> Boxes {
    while app.poll_storage().is_some() {}
    app.press(Action::Lock);
    let mut last = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            last = Some((inbox, outbox, kept));
        }
    }
    last.expect("the lock saved the boxes")
}

#[test]
fn a_secret_let_out_stays_a_secret_across_a_lock() {
    // A Lightning node key reads as plain text; it is still a secret.
    let mut app = device(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Lightning);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::LKey(fp));
    app.press(Action::LOut);
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    assert_eq!(app.outbox.len(), 1);
    assert_eq!(app.outbox[0].exposure(), Exposure::Secret);

    let (inbox, outbox, kept) = locked(&mut app);
    let mut next = device(Vec::new());
    next.storage(StorageEvent::Restored {
        inbox,
        outbox,
        kept,
    });
    assert_eq!(next.outbox.len(), 1);
    assert_eq!(
        next.outbox[0].exposure(),
        Exposure::Secret,
        "after the lock it would be written as a public file"
    );
}

#[test]
fn a_lock_leaves_no_secret_file_from_a_stick_in_the_inbox() {
    let words = format!("{}\n", testkit::test_words(testkit::TEST_SEEDS[0].0));
    let mut app = device(vec![
        ("seed-words.txt".to_string(), words.into_bytes()),
        (
            "recovery-codes.txt".to_string(),
            b"mail recovery\n1234 5678\n".to_vec(),
        ),
        kit_file("savings-wallet.txt"),
        kit_file("vault.fvlt"),
    ]);
    let k = find(&app, "seed-words.txt");
    app.press(Action::LoadKey(k));
    assert_eq!(app.session.keys.len(), 1);

    let (inbox, _, _) = locked(&mut app);
    let names: Vec<&str> = inbox.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        ["savings-wallet.txt", "vault.fvlt"],
        "the words and the text are wiped with the session; the public wallet and the sealed vault stay"
    );
}
