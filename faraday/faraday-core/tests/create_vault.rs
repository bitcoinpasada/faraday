//! Pausing a wallet's creation to make a vault for its keys: Make a vault
//! opens the vault wizard, and finishing it lands on Unlock with the new
//! vault picked and a way back; unlocking it, or going back, returns to
//! the wallet still being made.

use faraday_core::keygen::Way;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen};
use osk_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn settle(app: &mut Faraday, from: u64) {
    for t in from..from + 20 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A single-key wallet, made and added to the session from the Create
/// flow's own New key, stopping on the Vault card.
fn create_to_vault_step() -> Faraday {
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
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x20 + i; 32])));
    }
    app.press(Action::CreateWallet);
    app.press(Action::CNext(faraday_core::cstep::KIND));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(i % 3 != 1));
    }
    app.press(Action::KNext); // entries -> check
    app.press(Action::KNext); // check -> words
    app.press(Action::KNext); // words -> quiz
    for _ in 0..30 {
        let Some(q) = app.keygen.as_ref().and_then(|k| k.quiz.as_ref()) else {
            break;
        };
        if q.state() == opensigner_core::quiz::QuizState::Passed {
            break;
        }
        let slot = q.correct_slot() as u8;
        app.press(Action::KQuiz(slot));
    }
    app.press(Action::KAdd);
    assert_eq!(app.screen, Screen::Create, "New key returns to Create");
    app.press(Action::CNext(faraday_core::cstep::KEYS));
    app.press(Action::CMake);
    assert!(
        app.create.as_ref().unwrap().built.is_some(),
        "the wallet did not build"
    );
    app.press(Action::CNext(faraday_core::cstep::CHECK));
    app.press(Action::CNext(faraday_core::cstep::BACKUP));
    app.press(Action::CStep(faraday_core::cstep::VAULT));
    app
}

fn make_a_vault(app: &mut Faraday) {
    app.press(Action::Vault(V::CreateFrom(Screen::Create)));
    assert_eq!(app.screen, Screen::CreateVault);
    app.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::WHERE)));
    app.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::COST)));
    app.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::SIZE)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CFocus(0, true)));
    type_text(app, "test phrase");
    app.press(Action::Vault(V::CGo));
    settle(app, 1);
    assert_eq!(app.screen, Screen::Unlock);
    let name = app
        .vaults
        .just_made
        .clone()
        .expect("the vault was not made");
    assert_eq!(app.vault_files()[app.vaults.pick].name, name);
}

#[test]
fn going_back_from_unlock_leaves_the_new_vault_locked_and_returns_to_create() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    app.press(Action::Vault(V::Back));
    assert_eq!(
        app.screen,
        Screen::Create,
        "the way back should return to the wallet still being made"
    );
    assert!(app.vaults.open.is_empty(), "the new vault stays locked");
}

#[test]
fn opening_a_just_made_vault_needs_its_passphrase_typed_again() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    settle(&mut app, 30);
    assert_eq!(
        app.screen,
        Screen::Unlock,
        "nothing typed while making the vault should open it on its own"
    );
    assert!(
        app.vaults.passphrase.text.is_empty(),
        "the passphrase must not be filled in for the person"
    );
}

#[test]
fn opening_a_just_made_vault_with_its_passphrase_returns_to_create() {
    let mut app = create_to_vault_step();
    make_a_vault(&mut app);
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, 30);
    assert_eq!(
        app.screen,
        Screen::Create,
        "unlocking the vault just made should return to the wallet still being made"
    );
}

#[test]
fn a_new_vaults_passphrases_wait_until_the_stick_is_pulled() {
    let mut app = create_to_vault_step();
    app.press(Action::Vault(V::CreateFrom(Screen::Create)));
    app.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::WHERE)));
    app.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::COST)));
    app.press(Action::Vault(V::CNext(faraday_core::vaults::vstep::SIZE)));
    app.storage(faraday_core::StorageEvent::Sticks(vec![
        faraday_core::StickInfo {
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
        },
    ]));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "typed with a stick in");
    app.press(Action::Vault(V::Dice(0)));
    type_text(&mut app, "12345");
    let c = app.vaults.create.as_ref().unwrap();
    assert!(
        c.phrases
            .iter()
            .all(|(a, b)| a.text.is_empty() && b.text.is_empty()),
        "nothing secret is typed while a stick is attached"
    );
    assert!(
        app.vaults.dice.is_none(),
        "no dice are rolled with a stick in"
    );

    app.storage(faraday_core::StorageEvent::Sticks(Vec::new()));
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    assert_eq!(
        &*app.vaults.create.as_ref().unwrap().phrases[0].0.text,
        "test phrase"
    );
}

#[test]
fn a_finished_backup_offers_a_stick_and_asks_to_lock_first() {
    let mut app = create_to_vault_step();
    let wallet = app.create.as_ref().unwrap().built.unwrap();
    app.press(Action::Backup(wallet));
    for step in app.backup_steps() {
        app.press(Action::BNext(step));
    }
    app.press(Action::BSheets);
    let pdfs = app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".pdf"))
        .count();
    assert_eq!(pdfs, 2, "the template and the backup sheet go out");

    // A seed is held, so the stick comes after a lock.
    app.press(Action::WriteAsk);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WriteOut));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1, "Not now keeps the session");
}
