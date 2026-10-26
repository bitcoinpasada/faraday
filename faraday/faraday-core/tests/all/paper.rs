//! A key's other paper form: a Seed XOR split made in the backup, typed
//! back into Add a key, gives the same key; codex32 strings are not
//! offered for a key of words.

use faraday_core::paper::PaperForm;
use faraday_core::testkit;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Test key 1 and the Spending wallet over it, with the system's
/// randomness answered.
fn backing_up() -> Faraday {
    let mut app = faraday_core::testkit::started();
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
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::EntryForm(form));
    for p in parts {
        typed(&mut app, p);
        app.event(Event::Key(Key::Enter));
    }
    app.event(Event::Key(Key::Enter));
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        testkit::entry(&app).error
    );
    app.session.keys[0].fingerprint.0
}

#[test]
fn a_seed_xor_split_typed_back_is_the_same_key() {
    let mut app = backing_up();
    let want = app.session.keys[0].fingerprint.0;
    app.press(Action::BXor(3));
    let Some(PaperForm::Xor(parts)) = &app.backup().unwrap().paper else {
        panic!("no split");
    };
    assert_eq!(parts.len(), 3);
    let parts: Vec<&str> = parts.iter().map(|p| p.as_str()).collect();
    assert_eq!(added_from(3, &parts), want);
}

/// A key of words is offered Seed XOR as another paper form and never
/// codex32 strings, which would hold its passphrase-merged seed and give
/// back neither the words nor the passphrase.
#[test]
fn a_key_of_words_is_not_offered_codex32_strings_by_hand() {
    let mut app = backing_up();
    app.press(Action::BChecklist);
    if app.backup().unwrap().open == Some(faraday_core::bstep::BLANK) {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    let copy = faraday_core::bstep::of(faraday_core::plan::Item::Copy(0));
    if app.backup().unwrap().open != Some(copy) {
        app.press(Action::BStep(copy));
    }
    app.press(Action::BReveal);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Seed XOR, 3 parts"), "{texts:?}");
    assert!(
        !texts.iter().any(|t| t.to_lowercase().contains("codex32")),
        "{texts:?}"
    );
}
