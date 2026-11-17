//! Loading a seed: typed into Add a key, pasted into the scanner and
//! loaded with a passphrase, and loaded from a vault by the unlock's Load
//! sheet (the unlock's passphrase typed on the way), each ended by Lock.

#![expect(
    clippy::disallowed_methods,
    reason = "a test or example builds keys and seeds directly and is not shipped (docs/KEYS.md)"
)]

use crate::common::{device, type_text};
use crate::forms::{PASSPHRASE, PHRASE, VAULT_PASSPHRASE, seed_forms, vault_passphrase_forms};
use crate::{arm, finish, finish_left};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::seeds::read::Choice;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::Session;
use faraday_core::{Action, DESKTOP_MODES, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Command, Event, FileKind};
use faraday_vault::Record;
use faraday_vault::records::{self, field, kind};

/// Faraday Demo on Testnet, its clipboard read by Paste, Add a key open.
fn demo_on_testnet() -> Faraday {
    let mut app = testkit::started();
    app.clipboard = true;
    app.online = true;
    app.session = Session::on(testkit::NET);
    app.set_modes(DESKTOP_MODES);
    app.press(Action::Entry(None));
    let _ = app.frame();
    app
}

/// Paste pressed and the clipboard holding `text`.
pub fn paste(app: &mut Faraday, text: faraday_text::Text) {
    app.press(Action::ScanPaste);
    let asked = std::iter::from_fn(|| app.poll_command())
        .any(|c| matches!(c, Command::RequestClipboard { .. }));
    assert!(asked, "Paste did not ask for the clipboard");
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text,
    });
    let _ = app.frame();
}

/// A lock as a person's: the pool filled for a seal, Lock pressed, the
/// worker's answers taken.
pub fn lock(app: &mut Faraday) {
    crate::common::lock(app);
    assert!(app.session.keys.is_empty(), "Lock left a key");
}

#[test]
fn a_seed_typed_into_add_a_key_leaves_no_copy_after_lock() {
    let forms = seed_forms("");
    let armed = arm(&forms);
    let mut app = demo_on_testnet();
    type_text(&mut app, PHRASE);
    app.press(Action::EntryAdd);
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    lock(&mut app);
    finish("seed typed", armed, true);
}

#[test]
fn a_seed_pasted_and_loaded_with_a_passphrase_leaves_no_copy_after_lock() {
    let forms = seed_forms(PASSPHRASE);
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.clipboard = true;
    app.press(Action::ScanSeed);
    paste(&mut app, faraday_text::Text::of(PHRASE));
    assert_eq!(app.sheet, Some(Sheet::SeedRead));
    app.press(Action::SeedRead(Choice::AddPass));
    app.press(Action::SeedRead(Choice::Field(0)));
    type_text(&mut app, PASSPHRASE);
    app.press(Action::SeedRead(Choice::Field(1)));
    type_text(&mut app, PASSPHRASE);
    testkit::pause(&mut app);
    app.press(Action::SeedRead(Choice::WithPass));
    assert_eq!(app.session.keys.len(), 1, "no key loaded");
    lock(&mut app);
    finish("seed pasted with a passphrase", armed, true);
}

/// A vault of one key, its words and its own passphrase kept, sealed
/// under `VAULT_PASSPHRASE`.
fn vault() -> Vec<u8> {
    let c = testkit::compute();
    let m = Mnemonic::parse(&c, Language::English, PHRASE).expect("the vector's words");
    let key = Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&m))
        .with(field::KEY_LABEL, b"Leak key")
        .with(field::KEY_PASSPHRASE, PASSPHRASE.as_bytes());
    crate::common::vault_of("Cellar", VAULT_PASSPHRASE, 0x5d, vec![key])
}

/// Unlocks the vault from the Vaults tab, its passphrase typed.
pub fn unlock(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(app, VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
}

#[test]
fn a_vaults_seed_loaded_from_the_load_sheet_leaves_no_copy_after_lock() {
    load_flow();
}

fn load_flow() {
    let file = vault();
    let mut forms = seed_forms(PASSPHRASE);
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad), "the Load sheet asks");
    // The load runs on the worker, as a person's press has it.
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadTicked));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the vault's key did not load");
    lock(&mut app);
    finish("vault load", armed, true);
}

#[test]
fn a_vault_unlocked_and_locked_without_loading_leaves_no_copy() {
    let file = vault();
    let forms = vault_passphrase_forms();
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    app.press(Action::Vault(V::LoadNotNow));
    lock(&mut app);
    finish("vault unlock", armed, true);
}

/// A vault with a seed loaded from it and a change not sealed, then Lock:
/// the seal runs on the worker while the lock closes the session.
#[test]
fn a_lock_pressed_while_the_vault_seals_leaves_no_copy() {
    let file = vault();
    let mut forms = seed_forms(PASSPHRASE);
    forms.extend(vault_passphrase_forms());
    let armed = arm(&forms);
    let mut app = device(vec![("cellar.kdbx".into(), file)]);
    unlock(&mut app);
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(app.session.keys.len(), 1, "the vault's key did not load");
    testkit::vault_bump(&mut app, 0);
    crate::common::top_up(&mut app);
    testkit::press_on_worker(&mut app, Action::Lock);
    testkit::settle(&mut app);
    assert!(app.session.keys.is_empty(), "Lock left a key");
    finish("vault seal at lock", armed, true);
}

/// Twelve words made from 128 coin flips: the live preview of the last
/// word works the entropy out on every frame, and frees no plain copy of
/// it.
#[test]
fn coin_flips_previewed_as_the_last_word_leave_no_copy_of_the_entropy() {
    let heads = |i: usize| (i * 5 + i / 3).is_multiple_of(2);
    let mut entropy = [0u8; 16];
    for i in 0..128 {
        if heads(i) {
            entropy[i / 8] |= 0x80 >> (i % 8);
        }
    }
    let forms = [faraday_leak_scan::Form::new(
        "coin entropy",
        entropy.to_vec(),
    )];
    let armed = arm(&forms);
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(heads(i)));
        let _ = app.frame();
    }
    assert!(
        app.keygen().unwrap().checksum_word().is_some(),
        "the last word was not previewed"
    );
    finish("coin preview", armed, false);
}

/// A way out of a screen: its name and the press.
pub type Way = (&'static str, fn(&mut Faraday));

/// The ways out of a screen, as a person takes them.
pub fn ways_out() -> [Way; 3] {
    [
        ("the arrow", |a| a.press(Action::Back)),
        ("Escape", |a| {
            a.event(Event::Key(faraday_shell_api::Key::Escape))
        }),
        ("the mouse's back", |a| a.event(Event::Back)),
    ]
}

#[test]
fn a_seed_typed_into_add_a_key_is_gone_once_the_screen_is_left() {
    for (way, leave) in ways_out() {
        let forms = seed_forms("");
        let armed = arm(&forms);
        let mut app = demo_on_testnet();
        type_text(&mut app, PHRASE);
        leave(&mut app);
        let _ = app.frame();
        assert_ne!(app.screen, Screen::Entry, "{way} did not leave Add a key");
        finish_left(&format!("seed typed, left by {way}"), armed, &[]);
    }
}

#[test]
fn a_seed_typed_into_add_a_key_and_paused_on_is_gone_once_the_screen_is_left() {
    // The pause: typing stops long enough for a frame to work out the
    // status line's key.
    for (way, leave) in ways_out() {
        let forms = seed_forms("");
        let armed = arm(&forms);
        let mut app = demo_on_testnet();
        type_text(&mut app, PHRASE);
        app.event(Event::Tick { now_ms: 100_000 });
        let _ = app.frame();
        leave(&mut app);
        let _ = app.frame();
        assert_ne!(app.screen, Screen::Entry, "{way} did not leave Add a key");
        finish_left(&format!("seed typed, paused, left by {way}"), armed, &[]);
    }
}

#[test]
fn a_seed_typed_into_my_seed_makes_my_wallet_is_gone_once_the_screen_is_left() {
    use faraday_core::seed_check::SeedAction as S;
    // Left with the words typed and not taken, and with the key made.
    for taken in [false, true] {
        for (way, leave) in ways_out() {
            let forms = seed_forms("");
            let armed = arm(&forms);
            let mut app = testkit::started();
            app.press(Action::SeedCheck(S::Open));
            app.press(Action::SeedCheck(S::Type));
            type_text(&mut app, PHRASE);
            if taken {
                app.press(Action::SeedCheck(S::Words));
                app.press(Action::SeedCheck(S::NoPass));
                testkit::settle(&mut app);
            }
            let _ = app.frame();
            // Each way out steps back through the flow's cards first.
            for _ in 0..12 {
                if app.screen != Screen::SeedCheck {
                    break;
                }
                leave(&mut app);
                let _ = app.frame();
            }
            // Escape has no way out on some cards: the arrow ends it.
            for _ in 0..12 {
                if app.screen != Screen::SeedCheck {
                    break;
                }
                app.press(Action::Back);
                let _ = app.frame();
            }
            assert_ne!(app.screen, Screen::SeedCheck, "{way} did not leave");
            let flow = format!("seed typed into My seed, taken {taken}, left by {way}");
            finish_left(&flow, armed, &[]);
        }
    }
}

/// New key opened from Add a key with the words already typed there
/// (`PHRASE`), the words typed again inside New key and taken as far as its
/// Key card.
fn new_key_over_add_a_key() -> Faraday {
    use faraday_core::keygen::Start;
    let mut app = demo_on_testnet();
    type_text(&mut app, PHRASE);
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen, "New key did not open");
    app.press(Action::KStart(Start::Words.index()));
    let _ = app.frame();
    type_text(&mut app, PHRASE);
    app.press(Action::KNext);
    let _ = app.frame();
    app
}

/// Leaves New key and then Add a key by `leave`, as far as neither is on
/// screen.
fn leave_both(app: &mut Faraday, leave: fn(&mut Faraday)) {
    for _ in 0..12 {
        if !matches!(app.screen, Screen::KeyGen | Screen::Entry) {
            break;
        }
        leave(app);
        let _ = app.frame();
    }
    for _ in 0..12 {
        if !matches!(app.screen, Screen::KeyGen | Screen::Entry) {
            break;
        }
        app.press(Action::Back);
        let _ = app.frame();
    }
    assert!(
        !matches!(app.screen, Screen::KeyGen | Screen::Entry),
        "New key and Add a key were not left"
    );
}

#[test]
fn words_typed_in_add_a_key_and_new_key_are_gone_once_new_key_is_finished_and_left() {
    for (way, leave) in ways_out() {
        let forms = seed_forms("");
        let armed = arm(&forms);
        let mut app = new_key_over_add_a_key();
        app.press(Action::KLock);
        assert_eq!(app.session.keys.len(), 1, "New key did not add the key");
        leave_both(&mut app, leave);
        let flow = format!("new key finished, Add a key left by {way}");
        finish_left(&flow, armed, crate::sign::KEY_FORMS);
    }
}

#[test]
fn words_typed_in_add_a_key_and_new_key_are_gone_once_new_key_is_left() {
    for (way, leave) in ways_out() {
        let forms = seed_forms("");
        let armed = arm(&forms);
        let mut app = new_key_over_add_a_key();
        leave_both(&mut app, leave);
        assert!(app.session.keys.is_empty(), "{way} added a key");
        assert!(app.keygen().is_none(), "{way} left New key's state");
        assert!(
            testkit::entry(&app).typed.as_str().is_empty(),
            "{way} left words on {:?}",
            app.screen
        );
        finish_left(&format!("new key left by {way}"), armed, &[]);
    }
}
