//! A key's other paper forms: a Seed XOR split and codex32 shares made
//! in the backup, typed back into Add a key, give the same key.

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

#[test]
fn two_of_three_codex32_shares_typed_back_are_the_same_key() {
    let mut app = backing_up();
    let want = app.session.keys[0].fingerprint.0;
    app.press(Action::BCodex32(2, 3));
    let Some(PaperForm::Codex32 { k, strings }) = &app.backup().unwrap().paper else {
        panic!("no shares");
    };
    assert_eq!((*k, strings.len()), (2, 3));
    assert_eq!(added_from(2, &[&strings[0], &strings[2]]), want);
}

/// The copies by hand's codex32 strings show one at a time
/// (`docs/CODEX32.md` §9.1): each typed back on the codex32 keyboard, a
/// box typed wrong named, Skip only after its caution, then the next;
/// past the last, every string has been written down.
#[test]
fn codex32_strings_show_one_at_a_time_each_typed_back() {
    use faraday_core::codex32_backup::{Found, Press};
    let mut app = backing_up();
    // The checklist's copy of the key, its words shown.
    app.press(Action::BChecklist);
    // The blank templates, printed from Faraday Starter, come first.
    if app.backup().unwrap().open == Some(faraday_core::bstep::BLANK) {
        app.press(Action::BNext(faraday_core::bstep::BLANK));
    }
    let copy = faraday_core::bstep::of(faraday_core::plan::Item::Copy(0));
    if app.backup().unwrap().open != Some(copy) {
        app.press(Action::BStep(copy));
    }
    app.press(Action::BReveal);
    app.press(Action::BCodex32(2, 3));
    let strings: Vec<String> = match &app.backup().unwrap().paper {
        Some(PaperForm::Codex32 { strings, .. }) => strings.iter().map(|s| s.to_string()).collect(),
        _ => panic!("no shares"),
    };
    let at = |app: &Faraday| app.backup().unwrap().paper_typed.at;
    let found = |app: &Faraday| app.backup().unwrap().paper_typed.found.clone();
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "String 1 of 3"), "{texts:?}");
    let shown = |s: &str| faraday_ui::grouped(s);
    let drawn = texts.join(" ");
    assert!(
        !drawn.contains(&shown(&strings[1])[..20]),
        "one string at a time"
    );
    // Box 17 typed wrong is named.
    let mut wrong = strings[0].clone();
    let other = if &wrong[16..17] == "q" { "p" } else { "q" };
    wrong.replace_range(16..17, other);
    typed(&mut app, &wrong[3..]);
    assert_eq!(found(&app), Some(Found::Differs("Box 17 differs".into())));
    app.press(Action::BC32(Press::PaperNext));
    assert_eq!(at(&app), 0, "not on before it is typed back");
    for _ in 3..wrong.len() {
        app.event(Event::Key(Key::Backspace));
    }
    typed(&mut app, &strings[0][3..]);
    assert_eq!(found(&app), Some(Found::Holds("String 1 matches".into())));
    app.press(Action::BC32(Press::PaperNext));
    assert_eq!(at(&app), 1);
    // Skip asks first.
    app.press(Action::BC32(Press::PaperSkip));
    assert_eq!(at(&app), 1);
    assert!(app.backup().unwrap().paper_typed.skip_ask);
    app.press(Action::BC32(Press::PaperSkip));
    assert_eq!(at(&app), 2);
    typed(&mut app, &strings[2][3..]);
    app.press(Action::BC32(Press::PaperNext));
    assert_eq!(at(&app), 3);
    let _ = app.frame();
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "Every string written down")
    );
}
