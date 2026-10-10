//! A session at boot: the boot stick's vault comes into the Inbox with
//! the import over Home, pulling the stick leaves Home with the import,
//! not the passphrase prompt, and a vault unlocked on its own
//! (`docs/NEW-WALLET.md` §11.1a) loads the wallets and keys it holds and
//! lands on Wallets. Opened from inside a flow, it still shows what it
//! holds without loading it until asked.

use faraday_core::boot_import::ImportAction as I;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
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

/// Booted, with the import put off for later: the vault waits in Files.
fn later() -> (Faraday, Vec<u8>) {
    let (mut app, vault) = booted();
    app.press(Action::Import(I::Later));
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
fn the_boot_sticks_vault_waits_on_home_and_pulling_the_stick_leaves_the_import_up() {
    let (mut app, _) = booted();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.sheet, Some(Sheet::Import));
    assert!(app.inbox.iter().any(|i| i.name == "vault.ofv"));
    app.storage(StorageEvent::Sticks(Vec::new()));
    assert_eq!(app.screen, Screen::Home, "pulling the stick left Home");
    assert_eq!(app.sheet, Some(Sheet::Import));
    assert!(app.vaults.passphrase.text.is_empty());
}

#[test]
fn unlocking_loads_the_vaults_wallets_and_keys_and_lands_on_wallets() {
    let (mut app, _) = later();
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.press(Action::Vault(V::Open(0)));
    unlock(&mut app);
    assert_eq!(app.vaults.open.len(), 1);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.session.keys.len(), 1);
    let n = app.session.wallets.len();
    assert!(n > 1, "the test vault holds every test wallet");
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    assert_eq!(
        app.toast_text(),
        Some(format!("{n} wallets loaded from vault.ofv").as_str())
    );
    // The wallet shown is the one with a key here (Savings' seed loaded).
    let slots = app.session.slots(&app.session.wallets[app.wallet]);
    assert!(slots.iter().any(|s| s.held_by.is_some()));
}

#[test]
fn restore_starts_with_the_kind_of_wallet() {
    let mut app = Faraday::new();
    app.press(Action::RestoreWallet);
    assert_eq!(
        app.restore.as_ref().unwrap().open,
        Some(faraday_core::rstep::KIND)
    );
}

#[test]
fn select_all_and_load_counts_what_it_loads_and_files_stays_on_screen() {
    let (mut app, _) = later();
    app.storage(StorageEvent::Sticks(Vec::new()));
    // Opened from inside a flow (the vault way), not the Vaults list's
    // own Unlock: it still shows what it holds without loading it until
    // asked (`docs/NEW-WALLET.md` §11.1a).
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
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
    let (mut app, vault) = later();
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
