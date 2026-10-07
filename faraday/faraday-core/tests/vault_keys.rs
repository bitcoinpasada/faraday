//! Keys wherever they are: a wallet's missing key loads from an open vault
//! that holds it, a locked vault is offered for unlocking and the person
//! comes back to where they were, a key the quorum does not need is not
//! asked for, and a vault holding wallets with their seeds loads each
//! with its seeds in one press.

use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
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
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    app
}

fn find(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

/// Types the test vault's passphrase on the Unlock screen and waits.
fn type_and_unlock(app: &mut Faraday) {
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

#[test]
fn a_missing_key_in_an_open_vault_loads_from_the_wallet_card() {
    let mut app = device(vec![kit_file("spending-wallet.txt"), kit_file("vault.ofv")]);
    app.press(Action::Vault(V::Open(0)));
    type_and_unlock(&mut app);
    assert!(app.session.keys.is_empty(), "unlocking loads nothing");
    let w = find(&app, "spending-wallet.txt");
    app.press(Action::LoadWallet(w));
    assert_eq!(app.screen, Screen::Wallets);
    let _ = app.frame();
    let fp = app.session.slots(&app.session.wallets[0])[0]
        .fingerprint
        .unwrap();
    assert!(app.offers(Action::Vault(V::LoadKeyOf(fp.0))));
    app.press(Action::Vault(V::LoadKeyOf(fp.0)));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].master.fingerprint(), fp);
}

#[test]
fn a_locked_vault_is_offered_from_the_wallet_card_and_unlocking_comes_back() {
    let mut app = device(vec![kit_file("spending-wallet.txt"), kit_file("vault.ofv")]);
    let w = find(&app, "spending-wallet.txt");
    app.press(Action::LoadWallet(w));
    let _ = app.frame();
    let unlock = Action::Vault(V::OpenFrom(0, Screen::Wallets));
    assert!(app.offers(unlock), "the locked vault is not offered");
    app.press(unlock);
    assert_eq!(app.screen, Screen::Unlock);
    type_and_unlock(&mut app);
    assert_eq!(app.screen, Screen::Wallets, "unlocking does not come back");
    let _ = app.frame();
    let fp = app.session.slots(&app.session.wallets[0])[0]
        .fingerprint
        .unwrap();
    assert!(app.offers(Action::Vault(V::LoadKeyOf(fp.0))));
}

#[test]
fn a_key_the_quorum_does_not_need_is_not_asked_for() {
    let mut app = device(vec![kit_file("savings-wallet.txt")]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    let w = find(&app, "savings-wallet.txt");
    app.press(Action::LoadWallet(w));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let slots = app.session.slots(&app.session.wallets[0]);
    let missing: Vec<_> = slots.iter().filter(|s| s.held_by.is_none()).collect();
    assert_eq!(missing.len(), 1, "Savings is a 2 of 3 with two keys here");
    assert!(!app.offers(Action::Entry(missing[0].fingerprint.map(|f| f.0))));
}

#[test]
fn a_key_the_quorum_needs_is_asked_for() {
    let mut app = device(vec![kit_file("savings-wallet.txt")]);
    add_key(&mut app, 0);
    let w = find(&app, "savings-wallet.txt");
    app.press(Action::LoadWallet(w));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let slots = app.session.slots(&app.session.wallets[0]);
    let asked = slots
        .iter()
        .filter(|s| s.held_by.is_none())
        .filter(|s| app.offers(Action::Entry(s.fingerprint.map(|f| f.0))))
        .count();
    assert_eq!(asked, 2);
}

#[test]
fn a_vaults_wallets_load_with_their_seeds_in_one_press() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    app.press(Action::Vault(V::Open(0)));
    type_and_unlock(&mut app);
    let together = app.vault_wallets_with_keys(0);
    assert!(
        together.iter().any(|w| w.name == "Spending"),
        "Spending and test key 1 are in the test vault together"
    );
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::LoadWithKeys(0))));
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.keys.len(), 1, "test key 1 loads with them");
    assert_eq!(app.session.wallets.len(), together.len());
    assert!(app.vault_wallets_with_keys(0).iter().all(|w| w.done));
}

#[test]
fn only_the_chosen_wallets_load_with_their_seeds() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    app.press(Action::Vault(V::Open(0)));
    type_and_unlock(&mut app);
    let together = app.vault_wallets_with_keys(0);
    assert!(together.len() > 1);
    // Keep only the first.
    for w in &together[1..] {
        app.press(Action::Vault(V::Choose(0, w.record)));
    }
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.wallets[0].name, together[0].name);
}
