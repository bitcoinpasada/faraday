//! Importing a key's words from a stick: once in the Inbox, a Words file
//! offers its own Load button (refused with the stick still attached);
//! "Import and load" loads it on its own the moment the stick is pulled.

use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageEvent};

const STICK: &str = "S1";
const FILE: &str = "key-1-words.txt";

fn words() -> String {
    faraday_core::testkit::test_words("bacon")
}

fn attach(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: STICK.to_string(),
        label: "TESTSTICK".to_string(),
        boot: false,
        files: vec![(FILE.to_string(), words().len() as u64)],
    }]));
}

fn detach(app: &mut Faraday) {
    app.storage(StorageEvent::Sticks(Vec::new()));
}

#[test]
fn a_words_file_copied_in_offers_to_load_the_key_it_spells() {
    let mut app = faraday_core::testkit::started();
    attach(&mut app);
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitIn(0));
    app.press(Action::VisitCopy);
    app.storage(StorageEvent::Read {
        stick: STICK.to_string(),
        name: FILE.to_string(),
        bytes: words().into_bytes(),
    });
    let item = app
        .inbox
        .iter()
        .find(|i| i.name == FILE)
        .expect("not copied in");
    assert_eq!(item.kind, FileKind::Words);
    let k = app.inbox.iter().position(|i| i.name == FILE).unwrap();
    // Refused with the stick still attached.
    app.press(Action::LoadKey(k));
    assert!(app.session.keys.is_empty(), "loaded with a stick attached");
    detach(&mut app);
    app.press(Action::LoadKey(k));
    assert_eq!(app.session.keys.len(), 1, "the key did not load");
}

#[test]
fn import_and_load_loads_the_key_once_the_stick_is_pulled() {
    let mut app = faraday_core::testkit::started();
    attach(&mut app);
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitIn(0));
    app.press(Action::VisitCopyAndLoad);
    assert!(app.visit.load_after.contains(FILE));
    assert_eq!(
        app.screen,
        Screen::Files,
        "Import and load goes to the Inbox"
    );
    app.storage(StorageEvent::Read {
        stick: STICK.to_string(),
        name: FILE.to_string(),
        bytes: words().into_bytes(),
    });
    assert!(app.session.keys.is_empty(), "loaded before the stick left");
    detach(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the key did not load on its own");
    assert!(app.visit.load_after.is_empty());
}

/// The test stick's own words file, as the backup writes it: a note line
/// above the words.
fn stick_words_file() -> (String, Vec<u8>) {
    faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n.ends_with("-words.txt"))
        .unwrap()
}

#[test]
fn a_words_file_with_a_note_line_is_copied_and_loads_on_pulling_the_stick() {
    let (name, bytes) = stick_words_file();
    assert!(String::from_utf8_lossy(&bytes).starts_with('#'));
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: STICK.to_string(),
        label: "TESTSTICK".to_string(),
        boot: false,
        files: vec![(name.clone(), bytes.len() as u64)],
    }]));
    app.press(Action::Nav(Screen::Visit));
    app.press(Action::VisitIn(0));
    app.press(Action::VisitCopyAndLoad);
    app.storage(StorageEvent::Read {
        stick: STICK.to_string(),
        name: name.clone(),
        bytes,
    });
    let item = app.inbox.iter().find(|i| i.name == name);
    assert_eq!(
        item.map(|i| i.kind),
        Some(FileKind::Words),
        "the words file was refused at the copy"
    );
    assert!(
        app.visit.log.iter().all(|(_, ok)| *ok),
        "a refusal was logged"
    );
    detach(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the key did not load");
}

#[test]
fn words_numbered_one_to_a_line_are_read_as_the_words() {
    let numbered: String = faraday_core::testkit::test_words("bacon")
        .split(' ')
        .enumerate()
        .map(|(i, w)| format!("{}. {w}\n", i + 1))
        .collect();
    assert_eq!(
        faraday_core::wallet::classify("words.txt", numbered.as_bytes()),
        FileKind::Words
    );
}

/// Test key 1's SeedQR digits: each word's index as four digits.
fn seedqr_digits() -> Vec<u8> {
    let m = osk_bip::bip39::Mnemonic::parse(
        osk_bip::bip39::Language::English,
        &faraday_core::testkit::test_words("bacon"),
    )
    .unwrap();
    m.indices()
        .iter()
        .map(|i| format!("{i:04}"))
        .collect::<String>()
        .into_bytes()
}

#[test]
fn a_seedqr_picture_read_on_a_visit_loads_its_key_when_the_stick_is_pulled() {
    let mut app = faraday_core::testkit::started();
    attach(&mut app);
    app.press(Action::Nav(Screen::Visit));
    app.storage(StorageEvent::QrRead {
        name: "key-1-seedqr.png".to_string(),
        payloads: vec![seedqr_digits()],
    });
    let item = app
        .inbox
        .iter()
        .find(|i| i.name == "key-1-seedqr-words.txt")
        .unwrap_or_else(|| {
            panic!(
                "not in Files: {:?} {:?}",
                app.inbox.iter().map(|i| &i.name).collect::<Vec<_>>(),
                app.visit.log
            )
        });
    assert_eq!(item.kind, FileKind::Words);
    assert!(
        app.session.keys.is_empty(),
        "loaded with the stick attached"
    );
    detach(&mut app);
    assert_eq!(
        app.session.keys.len(),
        1,
        "the key did not load on pulling the stick"
    );
}
