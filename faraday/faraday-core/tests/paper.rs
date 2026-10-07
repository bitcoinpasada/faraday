//! A key's other paper forms: a Seed XOR split and codex32 shares made
//! in the backup, typed back into Add a key, give the same key.

use faraday_core::paper::PaperForm;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Test key 1 and the Spending wallet over it, with the system's
/// randomness answered.
fn backing_up() -> Faraday {
    let mut app = Faraday::new();
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
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x70 + i; 32])));
    }
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Entry(None));
    typed(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    app.press(Action::Backup(0));
    app
}

fn added_from(form: u8, parts: &[&str]) -> [u8; 4] {
    let mut app = Faraday::new();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(form));
    for p in parts {
        typed(&mut app, p);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.session.keys.len(), 1, "{:?}", app.entry.error);
    app.session.keys[0].master.fingerprint().0
}

#[test]
fn a_seed_xor_split_typed_back_is_the_same_key() {
    let mut app = backing_up();
    let want = app.session.keys[0].master.fingerprint().0;
    app.press(Action::BXor(3));
    let Some(PaperForm::Xor(parts)) = &app.backup.as_ref().unwrap().paper else {
        panic!("no split");
    };
    assert_eq!(parts.len(), 3);
    let parts: Vec<&str> = parts.iter().map(|p| p.as_str()).collect();
    assert_eq!(added_from(3, &parts), want);
}

#[test]
fn two_of_three_codex32_shares_typed_back_are_the_same_key() {
    let mut app = backing_up();
    let want = app.session.keys[0].master.fingerprint().0;
    app.press(Action::BCodex32(2, 3));
    let Some(PaperForm::Codex32 { k, strings }) = &app.backup.as_ref().unwrap().paper else {
        panic!("no shares");
    };
    assert_eq!((*k, strings.len()), (2, 3));
    assert_eq!(added_from(2, &[&strings[0], &strings[2]]), want);
}
