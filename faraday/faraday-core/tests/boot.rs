//! A session at boot: the boot stick's vault waits on Home, pulling the
//! stick opens its passphrase prompt, and unlocking shows the vault's
//! wallets and keys on Files without loading them until asked.

use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// Answers the app's reads from the test vault, the one file on the
/// boot stick.
fn pump(app: &mut Faraday, vault: &[u8]) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: vault.to_vec(),
            });
        }
    }
}

fn booted() -> (Faraday, Vec<u8>) {
    let vault = testkit::test_vault().unwrap();
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: vec![("vault.ofv".into(), vault.len() as u64)],
    }]));
    pump(&mut app, &vault);
    (app, vault)
}

fn unlock(app: &mut Faraday) {
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1000 });
}

#[test]
fn the_boot_sticks_vault_waits_on_home_and_pulling_the_stick_asks_for_its_passphrase() {
    let (mut app, _) = booted();
    assert_eq!(app.screen, Screen::Home);
    assert!(app.inbox.iter().any(|i| i.name == "vault.ofv"));
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Unlock);
}

#[test]
fn unlocking_loads_nothing_until_the_person_chooses() {
    let (mut app, _) = booted();
    app.storage(StorageEvent::Sticks(Vec::new()));
    unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1);
    assert_eq!(app.screen, Screen::Files);
    assert!(app.session.keys.is_empty() && app.session.wallets.is_empty());

    // Drop Savings, then load the chosen: every other wallet and the key.
    let savings = app.vaults.open[0]
        .contents
        .records
        .iter()
        .position(|r| r.text(faraday_vault::records::field::WALLET_NAME) == Some("Savings"))
        .unwrap();
    app.press(Action::Vault(V::Choose(0, savings)));
    app.press(Action::Vault(V::LoadChosen(0)));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.wallets.len(), 11);
    assert!(!app.session.wallets.iter().any(|w| w.name == "Savings"));

    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.wallets.len(), 12);
}

#[test]
fn restore_starts_with_the_transaction_to_sign() {
    let mut app = Faraday::new();
    app.press(Action::RestoreWallet);
    assert_eq!(app.restore.as_ref().unwrap().open, Some(0));
}

#[test]
fn select_all_and_load_counts_what_it_loads_and_files_stays_on_screen() {
    let (mut app, _) = booted();
    app.storage(StorageEvent::Sticks(Vec::new()));
    unlock(&mut app);
    let rows = app.vault_rows(0);
    // Select all when some are chosen chooses all; again, none.
    if rows.iter().all(|r| r.chosen) {
        app.press(Action::Vault(V::ChooseAll(0)));
    }
    app.press(Action::Vault(V::ChooseAll(0)));
    assert!(app.vault_rows(0).iter().all(|r| r.chosen));
    app.press(Action::Vault(V::ChooseAll(0)));
    assert!(app.vault_rows(0).iter().all(|r| !r.chosen));
    app.press(Action::Vault(V::LoadChosen(0)));
    assert!(app.session.keys.is_empty() && app.session.wallets.is_empty());
    // One seed chosen: it alone loads, and Files stays with it marked.
    let seed = app.vault_rows(0).into_iter().find(|r| r.seed).unwrap();
    app.press(Action::Vault(V::Choose(0, seed.record)));
    app.press(Action::Vault(V::LoadChosen(0)));
    assert_eq!(app.session.keys.len(), 1);
    assert!(app.session.wallets.is_empty());
    assert_eq!(app.screen, Screen::Files, "loading left Files");
    assert!(
        app.vault_rows(0)
            .iter()
            .any(|r| r.seed && r.loaded && r.record == seed.record)
    );
    let _ = app.frame();
    assert!(
        app.offers(Action::Nav(Screen::Start)),
        "no way on to Wallets"
    );
}

#[test]
fn a_vault_passphrase_is_typed_only_once_the_stick_is_pulled() {
    let (mut app, vault) = booted();
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    app.press(Action::Vault(V::FocusPassphrase));
    unlock(&mut app);
    assert!(
        app.vaults.passphrase.text.is_empty(),
        "nothing is typed into the passphrase while the stick is attached"
    );
    assert!(app.vaults.open.is_empty());
    assert!(
        app.clean(),
        "the stick is still usable: no secret was typed"
    );

    app.storage(StorageEvent::Sticks(Vec::new()));
    pump(&mut app, &vault);
    unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1, "pulling the stick lets it unlock");
}
