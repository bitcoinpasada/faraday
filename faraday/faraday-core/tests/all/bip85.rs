//! BIP-85 from a loaded key: a child seed loads as a key of its own, a
//! password goes into the open vault as an entry, nothing leaves
//! unprotected without the secret sheet, and a password out (for KeePass
//! or as a file) names neither the key it came from nor its index.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::RecordsOf as _;
use crate::common::type_text;
use faraday_bip::bip39::Language;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};
use faraday_vault::records::{field, kind};

fn with_key(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    with_key_on(inbox, false)
}

/// The same, on Testnet when `testnet`, where a public sample is kept for
/// testing and nothing stands over the page.
fn with_key_on(inbox: Vec<(String, Vec<u8>)>, testnet: bool) -> Faraday {
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
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(inbox),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    if testnet {
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
    }
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1);
    if testnet {
        testkit::keep_published(&mut app);
    }
    app
}

#[test]
fn a_child_seed_loads_as_a_key_of_its_own() {
    let c = testkit::compute();
    let mut app = with_key(Vec::new());
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85);
    // Words, 12 of them, at index 7 typed on the keyboard.
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PLoad);
    app.press(Action::PRecord(false));
    assert_eq!(app.session.keys.len(), 2);
    let master = &app.session.keys.master_for_tests(0);
    let child = faraday_bip::bip85::child_mnemonic(&c, master, Language::English, 12, 7).unwrap();
    let words: Vec<&str> = child
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let mut probe = faraday_core::wallet::Session::default();
    let fp = probe
        .keys
        .add_words_with(&c, &words.join(" "), "", "", None)
        .unwrap();
    assert_eq!(app.session.keys[1].fingerprint, fp);
}

#[test]
fn a_value_goes_out_only_through_the_secret_sheet() {
    let mut app = with_key(Vec::new());
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PVault);
    assert!(app.outbox.is_empty(), "no vault is open");
    app.press(Action::POut);
    assert_eq!(app.sheet, Some(Sheet::SecretOut));
    assert!(app.outbox.is_empty());
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    assert_eq!(app.outbox.len(), 1);
}

#[test]
fn a_password_goes_into_the_vault_as_an_entry() {
    let c = testkit::compute();
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    let mut app = with_key(vec![vault]);
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PVault);
    let want =
        faraday_bip::bip85::child_password_base64(&c, app.session.keys.master_for_tests(0), 21, 0)
            .unwrap();
    let v = &app.vaults().open[0];
    let found = app
        .vaults()
        .store()
        .contents_for_tests(v.id)
        .of(kind::ENTRY)
        .any(|(_, r)| r.field(field::PASSWORD) == Some(want.as_str().as_bytes()));
    assert!(found, "the password is not an entry in the vault");
    assert!(app.outbox.is_empty());
}

/// Child words saved to the vault twice: the second press adds no record and
/// says the vault holds the key already.
#[test]
fn child_words_saved_to_the_vault_twice_are_added_once_and_the_person_is_told() {
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    let mut app = with_key(vec![vault]);
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    let keys = |app: &Faraday| {
        let v = &app.vaults().open[0];
        app.vaults()
            .store()
            .contents_for_tests(v.id)
            .of(kind::KEY)
            .count()
    };
    let before = keys(&app);
    app.press(Action::Bip85);
    app.press(Action::PApp(0));
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PVault);
    app.press(Action::PRecord(false));
    assert_eq!(keys(&app), before + 1, "the child key is added");
    app.press(Action::PVault);
    app.press(Action::PRecord(false));
    assert_eq!(keys(&app), before + 1, "a second press adds nothing");
    assert!(
        app.toast_text()
            .is_some_and(|t| t.contains("already holds this key")),
        "the person is told"
    );
}

/// What `bytes` hold of the key a password came from: its fingerprint,
/// in either case, BIP-85's name, the index's word or the path's purpose.
fn names_the_parent(app: &Faraday, bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let fp = faraday_keys::identity::fp_text(app.session.keys[0].fingerprint);
    [
        fp.to_uppercase(),
        fp.to_lowercase(),
        "BIP-85".into(),
        "index".into(),
        "83696968".into(),
    ]
    .into_iter()
    .find(|w| text.contains(w.as_str()))
}

#[test]
fn an_exported_password_names_neither_its_parent_nor_its_index() {
    let c = testkit::compute();
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    let mut app = with_key(vec![vault]);
    for i in 0..12u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut app, 1);
    assert_eq!(app.vaults().open.len(), 1);
    // A base64 password at index 7, into the vault.
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PVault);
    let want =
        faraday_bip::bip85::child_password_base64(&c, app.session.keys.master_for_tests(0), 21, 7)
            .unwrap();
    // Exported for KeePass.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(2)));
    let id = app.vaults().open[0].id;
    let at = app
        .vaults()
        .store()
        .contents_for_tests(id)
        .of(kind::ENTRY)
        .position(|(_, r)| r.field(field::PASSWORD) == Some(want.as_str().as_bytes()))
        .expect("the password's entry");
    app.press(Action::Vault(V::Item(at)));
    app.press(Action::Vault(V::ExportKdbx));
    type_text(&mut app, "keepass passphrase");
    app.press(Action::Vault(V::FocusPromptAgain));
    type_text(&mut app, "keepass passphrase");
    app.press(Action::Vault(V::PromptGo));
    crate::common::settle(&mut app, 30);
    let file = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".kdbx"))
        .expect("no KeePass file");
    assert_eq!(file.name, "keepass-export.kdbx");
    let opened = faraday_vault::file::open(&c, &file.bytes, "keepass passphrase")
        .expect("the file opens under its passphrase");
    let read = opened.read();
    assert_eq!(read.items.len(), 1);
    let faraday_vault::kdbx::items::Body::Record(r) = &read.items[0].body else {
        panic!("a person's entry");
    };
    assert_eq!(r.kind, kind::ENTRY);
    assert_eq!(r.text(field::TITLE), Some("Password"));
    assert_eq!(r.field(field::PASSWORD), Some(want.as_str().as_bytes()));
    assert!(read.items[0].path.is_empty(), "in the root group");
    let all = format!(
        "{} {:?} {}",
        &*read.label,
        read.items[0].path,
        [field::TITLE, field::USERNAME, field::URL, field::NOTES]
            .iter()
            .filter_map(|f| r.text(*f))
            .collect::<Vec<_>>()
            .join(" ")
    );
    assert_eq!(names_the_parent(&app, all.as_bytes()), None, "{all}");
    assert_eq!(names_the_parent(&app, file.name.as_bytes()), None);
    // The same password out as a file, unprotected: itself alone.
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::POut);
    app.press(Action::SecretUnprotected);
    app.press(Action::SecretHold);
    let out = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".txt"))
        .expect("no password file");
    assert_eq!(out.name, "password.txt");
    assert_eq!(*out.bytes, format!("{}\n", want.as_str()).into_bytes());
}

/// BIP-85's value shows no code of any kind (`docs/QR.md` §7.3).
#[test]
fn a_value_has_no_code() {
    let mut app = with_key(Vec::new());
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    assert_eq!(crate::common::codes_offered(&mut app), Vec::new());
}

#[test]
fn bip85_with_no_key_adds_one_on_the_add_a_key_page_and_returns() {
    let mut app = testkit::started();
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85);
    let _ = app.frame();
    assert!(app.offers(Action::Entry(None)), "Add a key is offered");
    assert!(!app.offers(Action::KeyGen(None)));
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.screen, Screen::Bip85, "back in the tool");
    let added = app.session.keys[0].fingerprint.0;
    assert!(
        app.bip85().is_some_and(|b| b.key == Some(added)),
        "the new key is ticked in the pick"
    );
}

fn drawn_has(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_texts().join(" ").contains(text)
}

#[test]
fn one_step_is_drawn_at_a_time_and_your_choices_lists_the_answers() {
    let mut app = with_key_on(Vec::new(), true);
    app.press(Action::Bip85);
    assert!(
        drawn_has(&mut app, "Step 2 of 4"),
        "{}",
        app.drawn_texts().join(" ")
    );
    assert!(
        !drawn_has(&mut app, "Step 1 of 4"),
        "the earlier step is not drawn"
    );
    assert!(
        drawn_has(&mut app, "Your choices"),
        "the pane lists the steps"
    );
    assert!(app.offers(Action::PStep(0)), "a row opens the earlier step");
    app.press(Action::PStep(0));
    assert!(drawn_has(&mut app, "Step 1 of 4"));
    assert!(!drawn_has(&mut app, "Step 2 of 4"));
}

#[test]
fn leaving_a_shown_value_draws_the_next_step_at_once() {
    let mut app = with_key(Vec::new());
    app.press(Action::Bip85);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    app.press(Action::PShow);
    let _ = app.frame();
    let _ = app.take_fade_counts();
    app.press(Action::PStep(2));
    let _ = app.frame();
    assert_eq!(app.take_fade_counts().started, 0, "no fade from the value");
}

/// A second key beside the first. The flow opens on its key step with
/// neither picked, as there are two keys.
fn with_two_keys() -> Faraday {
    // On Testnet, where no notice about a public sample stands over the page.
    let mut app = with_key_on(Vec::new(), true);
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 2);
    app
}

#[test]
fn a_press_on_a_key_picks_it_and_next_moves_on_with_that_key() {
    let c = testkit::compute();
    let mut app = with_two_keys();
    app.press(Action::Bip85);
    assert_eq!(app.screen, Screen::Bip85);
    let second = app.session.keys[1].fingerprint.0;
    app.press(Action::PKey(second));
    // The pick alone does not move on.
    let on_key_step = |app: &Faraday| app.bip85().is_some_and(|b| b.open == Some(0));
    assert!(on_key_step(&app));
    assert!(app.bip85().is_some_and(|b| b.key == Some(second)));
    app.press(Action::PNext);
    assert!(!on_key_step(&app));
    // Words, 12 of them, at index 7: the child of the second key.
    app.press(Action::PApp(0));
    for _ in 0..4 {
        app.press(Action::PLength(-1));
    }
    app.press(Action::PNext);
    app.event(Event::Key(Key::Char('7')));
    app.press(Action::PNext);
    app.press(Action::PLoad);
    app.press(Action::PRecord(false));
    assert_eq!(app.session.keys.len(), 3);
    let master = &app.session.keys.master_for_tests(1);
    let child = faraday_bip::bip85::child_mnemonic(&c, master, Language::English, 12, 7).unwrap();
    let words: Vec<&str> = child
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let mut probe = faraday_core::wallet::Session::default();
    let fp = probe
        .keys
        .add_words_with(&c, &words.join(" "), "", "", None)
        .unwrap();
    assert_eq!(app.session.keys[2].fingerprint, fp);
}

#[test]
fn next_on_the_key_step_does_nothing_until_a_key_is_picked() {
    let mut app = testkit::started();
    app.press(Action::Bip85);
    app.press(Action::PNext);
    assert!(
        app.bip85()
            .is_some_and(|b| b.key.is_none() && b.open == Some(0))
    );
}

/// The texts drawn now that `before` did not hold: the value on show.
fn drawn_beyond(app: &mut Faraday, before: &[String]) -> Vec<String> {
    crate::common::settle(app, 30);
    let _ = app.frame();
    app.drawn_texts()
        .iter()
        .map(|t| t.as_str().to_string())
        .filter(|t| !before.contains(t))
        .collect()
}

#[test]
fn a_pick_changed_after_the_value_was_shown_hides_it() {
    let mut app = with_two_keys();
    app.press(Action::Bip85);
    app.press(Action::PKey(app.session.keys[0].fingerprint.0));
    app.press(Action::PNext);
    app.press(Action::PApp(4));
    app.press(Action::PNext);
    app.press(Action::PNext);
    crate::common::settle(&mut app, 30);
    let _ = app.frame();
    let hidden: Vec<String> = app
        .drawn_texts()
        .iter()
        .map(|t| t.as_str().to_string())
        .collect();
    app.press(Action::PShow);
    crate::common::settle(&mut app, 30);
    let first = drawn_beyond(&mut app, &hidden);
    assert!(!first.is_empty(), "the first key's value is drawn");
    // A pick on the key step, with no step opened between.
    app.press(Action::PKey(app.session.keys[1].fingerprint.0));
    assert!(app.bip85().is_some_and(|b| !b.shown));
    let after = drawn_beyond(&mut app, &[]);
    assert!(
        first.iter().all(|t| !after.contains(t)),
        "the first key's value is not drawn after the pick"
    );
    app.press(Action::PShow);
    let second = drawn_beyond(&mut app, &after);
    assert!(
        !second.is_empty() && first.iter().all(|t| !second.contains(t)),
        "the second key's value is drawn, not the first's"
    );
}

#[test]
fn enter_on_a_focused_row_picks_it_and_the_key_step_stays_open() {
    // Tab to each control in turn, in fresh flows, until Enter lands on
    // the second key's row.
    let mut found = false;
    for tabs in 1..40 {
        let mut app = with_two_keys();
        app.press(Action::Bip85);
        let second = app.session.keys[1].fingerprint.0;
        app.press(Action::PKey(app.session.keys[0].fingerprint.0));
        crate::common::settle(&mut app, 30);
        let _ = app.frame();
        for _ in 0..tabs {
            app.event(Event::Key(Key::Tab));
            let _ = app.frame();
        }
        app.event(Event::Key(Key::Enter));
        if app.screen == Screen::Bip85
            && app
                .bip85()
                .is_some_and(|b| b.key == Some(second) && b.open == Some(0))
        {
            // Next then goes on with the second key.
            app.press(Action::PNext);
            assert!(
                app.bip85()
                    .is_some_and(|b| b.key == Some(second) && b.open != Some(0))
            );
            found = true;
            break;
        }
    }
    assert!(found, "Enter on the second key's row picked it");
}

#[test]
fn enter_with_two_keys_and_none_picked_goes_nowhere() {
    let mut app = with_two_keys();
    app.press(Action::Bip85);
    let _ = app.frame();
    app.event(Event::Key(Key::Enter));
    assert!(
        app.bip85()
            .is_some_and(|b| b.key.is_none() && b.open == Some(0))
    );
}

/// Adds the third test key through the flow's way in (Add a key), from
/// whatever step is open, and returns to the tool.
fn add_a_key_from_the_way_in(app: &mut Faraday) -> [u8; 4] {
    let before = app.session.keys.len();
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[2].0));
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), before + 1);
    assert_eq!(app.screen, Screen::Bip85, "back in the tool");
    app.session.keys[before].fingerprint.0
}

/// One key, the flow open on its Value card.
fn on_the_value_card() -> Faraday {
    let mut app = with_key_on(Vec::new(), true);
    app.press(Action::Bip85);
    app.press(Action::PNext);
    app.press(Action::PNext);
    assert!(
        app.bip85()
            .is_some_and(|b| b.open == Some(faraday_core::bip85::pstep::RESULT))
    );
    app
}

#[test]
fn a_key_added_on_the_value_card_leaves_the_flow_on_the_key_picked() {
    let mut app = on_the_value_card();
    let first = app.session.keys[0].fingerprint.0;
    app.press(Action::PLoad);
    app.press(Action::PRecord(false));
    let loaded = app.bip85().and_then(|b| b.loaded);
    assert!(loaded.is_some(), "the child was loaded as a key");
    let added = add_a_key_from_the_way_in(&mut app);
    assert_ne!(added, first);
    let b = app.bip85().expect("the flow is kept");
    assert_eq!(b.key, Some(first), "Your choices still names the first key");
    assert_eq!(b.loaded, loaded, "what was loaded stays with its key");
    assert_eq!(b.open, Some(faraday_core::bip85::pstep::RESULT));
}

#[test]
fn a_value_shown_when_a_key_is_added_is_not_drawn_under_the_other_key() {
    let mut app = on_the_value_card();
    crate::common::settle(&mut app, 30);
    let _ = app.frame();
    let hidden: Vec<String> = app
        .drawn_texts()
        .iter()
        .map(|t| t.as_str().to_string())
        .collect();
    app.press(Action::PShow);
    crate::common::settle(&mut app, 30);
    let first = drawn_beyond(&mut app, &hidden);
    assert!(!first.is_empty(), "the first key's value is drawn");
    add_a_key_from_the_way_in(&mut app);
    assert!(app.bip85().is_some_and(|b| !b.shown));
    let after = drawn_beyond(&mut app, &[]);
    assert!(
        first.iter().all(|t| !after.contains(t)),
        "nothing of the value is drawn on return"
    );
}

#[test]
fn a_key_added_on_the_key_step_is_ticked_and_next_goes_on_with_it() {
    let mut app = with_two_keys();
    app.press(Action::Bip85);
    app.press(Action::PKey(app.session.keys[0].fingerprint.0));
    let added = add_a_key_from_the_way_in(&mut app);
    let b = app.bip85().expect("the flow is kept");
    assert_eq!(b.key, Some(added), "the new key is ticked");
    assert!(!b.shown);
    assert_eq!(b.open, Some(faraday_core::bip85::pstep::KEY));
    app.press(Action::PNext);
    assert!(
        app.bip85().is_some_and(
            |b| b.key == Some(added) && b.open != Some(faraday_core::bip85::pstep::KEY)
        )
    );
}

#[test]
fn leaving_the_way_in_with_nothing_added_keeps_the_pick() {
    let mut app = with_two_keys();
    app.press(Action::Bip85);
    let first = app.session.keys[0].fingerprint.0;
    app.press(Action::PKey(first));
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Bip85);
    assert!(app.bip85().is_some_and(|b| b.key == Some(first)));
}

/// The index field is a Tab stop; Enter there goes on, as before.
fn tab_to_the_index_field(app: &mut Faraday) {
    for _ in 0..40 {
        app.event(Event::Key(Key::Tab));
        let _ = app.frame();
        if testkit::focused(app) == Some(Action::TypeHere(faraday_core::edit::Here::Index)) {
            return;
        }
    }
    panic!("the index field is a Tab stop");
}

#[test]
fn enter_in_the_index_field_goes_on_with_what_is_typed() {
    let mut app = with_key_on(Vec::new(), true);
    app.press(Action::Bip85);
    app.press(Action::PNext);
    let _ = app.frame();
    tab_to_the_index_field(&mut app);
    app.event(Event::Key(Key::Char('7')));
    app.event(Event::Key(Key::Enter));
    let b = app.bip85().expect("the flow is kept");
    assert_eq!(b.open, Some(faraday_core::bip85::pstep::RESULT));
    assert_eq!(b.index(), Some(7));
}

#[test]
fn enter_on_a_focused_plus_presses_it() {
    let mut app = with_key_on(Vec::new(), true);
    app.press(Action::Bip85);
    app.press(Action::PNext);
    let _ = app.frame();
    let before = app.bip85().and_then(|b| b.index());
    for _ in 0..40 {
        app.event(Event::Key(Key::Tab));
        let _ = app.frame();
        if testkit::focused(&app) == Some(Action::PIndex(1)) {
            break;
        }
    }
    assert_eq!(testkit::focused(&app), Some(Action::PIndex(1)));
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.bip85().and_then(|b| b.index()), Some(1));
    assert_eq!(before, Some(0));
}
