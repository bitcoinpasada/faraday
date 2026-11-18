//! A key's other paper form: a Seed XOR split made in the backup, typed
//! back into Add a key, gives the same key; codex32 strings are not
//! offered for a key of words.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use faraday_core::testkit::{self, Plain};
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn typed(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Test key 1 and the Spending wallet over it, with the system's
/// randomness answered.
pub(crate) fn backing_up() -> Faraday {
    backing_up_with(|app| typed(app, &testkit::test_words(testkit::TEST_SEEDS[0].0)))
}

fn backing_up_with(enter: impl FnOnce(&mut Faraday)) -> Faraday {
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
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Entry(None));
    enter(&mut app);
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

/// A key on a word list other than English is given no Seed XOR parts:
/// Add a key restores them on the English list only.
#[test]
fn a_spanish_key_is_given_no_seed_xor_paper() {
    use faraday_bip::bip39::{Language, Mnemonic};
    let entropy: Vec<u8> = (0u8..16)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();
    let es = Mnemonic::from_entropy(Language::Spanish, &entropy).unwrap();
    let mut app = backing_up_with(|app| {
        let at = faraday_core::forms::LANGUAGES
            .iter()
            .position(|l| *l == Language::Spanish)
            .unwrap();
        app.press(Action::EntryLanguage(at as u8));
        for (k, &i) in es.indices().iter().enumerate() {
            if k > 0 {
                app.event(Event::Key(Key::Char(' ')));
            }
            for c in Language::Spanish.typed(i).unwrap().as_chars() {
                app.event(Event::Key(Key::Char(*c)));
            }
        }
    });
    assert_eq!(app.session.keys[0].language, Language::Spanish);
    app.press(Action::BXor(3));
    assert!(app.backup().unwrap().paper.is_none());
    assert_eq!(
        app.toast_text(),
        Some(faraday_strings::EN.convert_not_given)
    );
}

#[test]
fn a_seed_xor_split_typed_back_is_the_same_key() {
    let mut app = backing_up();
    let want = app.session.keys[0].fingerprint.0;
    app.press(Action::BXor(3));
    let Some(parts) = &app.backup().unwrap().paper else {
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
    if app.backup().unwrap().open == Some(faraday_core::plan::Item::Templates) {
        app.press(Action::BNext(faraday_core::plan::Item::Templates));
    }
    let copy = faraday_core::plan::Item::Copy(0);
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

fn part_words(app: &Faraday, i: usize) -> String {
    app.backup().unwrap().paper.as_ref().unwrap()[i].to_string()
}

#[test]
fn seed_xor_parts_are_put_away_only_once_checked_or_on_a_second_press() {
    let mut app = backing_up();
    app.press(Action::BXor(2));
    app.press(Action::BPaperHide);
    assert!(app.backup().unwrap().paper.is_some(), "put away unchecked");
    app.press(Action::BPaperHide);
    assert!(app.backup().unwrap().paper.is_none());
}

#[test]
fn checked_seed_xor_parts_stay_when_put_away() {
    let mut app = backing_up();
    app.press(Action::BXor(2));
    for i in 0..2u8 {
        app.press(Action::BPartCheck(i));
        let words = part_words(&app, i.into());
        typed(&mut app, &format!("{words} "));
        assert_eq!(app.backup().unwrap().paper_ok & (1 << i), 1 << i);
    }
    app.press(Action::BPaperHide);
    assert!(app.backup().unwrap().paper.is_none(), "checked parts stay");
}

#[test]
fn seed_xor_parts_on_the_plan_are_an_item_done_once_every_part_is_checked() {
    use faraday_core::plan::Item;
    let mut app = backing_up();
    app.press(Action::BXor(2));
    let c = testkit::compute();
    assert!(app.backup_items_now(&c).contains(&Item::Parts(0)));
    assert!(!app.backup_item_done_now(&c, Item::Parts(0)));
    for i in 0..2u8 {
        app.press(Action::BPartCheck(i));
        let words = part_words(&app, i.into());
        typed(&mut app, &format!("{words} "));
        assert_eq!(
            app.backup_item_done_now(&c, Item::Parts(0)),
            i == 1,
            "after part {}",
            i + 1
        );
    }
    // Put away, the checks stay.
    app.press(Action::BPaperHide);
    assert!(app.backup_item_done_now(&c, Item::Parts(0)));
}

#[test]
fn a_seed_xor_part_typed_back_with_one_word_changed_is_named_as_differing() {
    let mut app = backing_up();
    app.press(Action::BXor(3));
    app.press(Action::BPartCheck(1));
    let words = part_words(&app, 1);
    let mut w: Vec<String> = words.split(' ').map(String::from).collect();
    let other = part_words(&app, 0).split(' ').next().unwrap().to_string();
    w[2] = if w[2] == other {
        "abandon".into()
    } else {
        other
    };
    typed(&mut app, &format!("{} ", w.join(" ")));
    let b = app.backup().unwrap();
    assert_eq!(b.paper_ok, 0);
    let error = b.check.as_ref().and_then(|c| c.error.as_deref());
    assert_eq!(error, Some("Word 3 differs from part 2"));
}

#[test]
fn back_ups_seed_xor_section_names_the_places_the_parts_are_on() {
    let mut app = backing_up();
    testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    app.press(Action::BChecklist);
    app.press(Action::BNext(faraday_core::plan::Item::Templates));
    app.press(Action::BReveal);
    app.press(Action::BXor(2));
    let want = "On the plan: Place 1, Place 2";
    let mut seen = false;
    for _ in 0..40 {
        let _ = app.frame();
        if app.drawn_texts().plain().iter().any(|t| t == want) {
            seen = true;
            break;
        }
        for x in [600, 1326] {
            app.event(Event::Scroll { x, y: 400, dy: 150 });
        }
    }
    let all: Vec<String> = app.drawn_texts().plain();
    assert!(seen, "the section says where the parts are: {all:?}");
}
