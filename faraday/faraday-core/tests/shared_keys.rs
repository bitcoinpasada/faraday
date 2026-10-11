//! One seed in several wallets: test key 1 is Spending's only key and one
//! of Savings' three. Loaded once, it fills its place in both; removing
//! one wallet leaves it for the other; it signs for whichever wallet the
//! transaction spends from; and a vault holding it with both wallets loads
//! it once.

use faraday_core::testkit::{self, Kit};
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, StorageEvent};
use osk_shell_api::{App, Event, Key};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

fn add_key(app: &mut Faraday, seed: &str) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(seed).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn both() -> Faraday {
    let mut app = with_inbox(vec![
        (
            "spending-wallet.txt".to_string(),
            kit("spending").descriptor.into_bytes(),
        ),
        (
            "savings-wallet.txt".to_string(),
            kit("savings").descriptor.into_bytes(),
        ),
        (
            "savings-unsigned.psbt".to_string(),
            testkit::spend_of(&kit("savings")).unwrap().0.to_bytes(),
        ),
    ]);
    app.press(Action::LoadWallet(at(&app, "spending-wallet.txt")));
    app.press(Action::LoadWallet(at(&app, "savings-wallet.txt")));
    add_key(&mut app, "bacon");
    app
}

fn held(app: &Faraday, wallet: &str) -> usize {
    let w = app
        .session
        .wallets
        .iter()
        .find(|w| w.name == wallet)
        .unwrap();
    app.session
        .slots(w)
        .iter()
        .filter(|s| s.held_by.is_some())
        .count()
}

#[test]
fn one_seed_loaded_once_fills_its_place_in_both_wallets() {
    let app = both();
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(held(&app, "Spending"), 1);
    assert_eq!(held(&app, "Savings"), 1);
}

#[test]
fn the_same_seed_typed_again_is_not_loaded_twice() {
    let mut app = both();
    add_key(&mut app, "bacon");
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn removing_one_wallet_leaves_the_seed_for_the_other() {
    let mut app = both();
    let i = app
        .session
        .wallets
        .iter()
        .position(|w| w.name == "Spending")
        .unwrap();
    app.press(Action::RemoveWallet(i));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(held(&app, "Savings"), 1);
}

#[test]
fn the_shared_seed_signs_for_the_wallet_the_transaction_spends_from() {
    let mut app = both();
    app.press(Action::StartSpend(at(&app, "savings-unsigned.psbt")));
    let s = app.spend.as_ref().unwrap();
    let w = s.wallet.map(|i| app.session.wallets[i].name.clone());
    assert_eq!(w.as_deref(), Some("Savings"));
    app.press(Action::SignHere);
    assert_eq!(app.spend.as_ref().unwrap().spend.signed_here.len(), 1);
}

#[test]
fn a_vault_with_both_wallets_loads_the_shared_seed_once() {
    let mut app = faraday_core::testkit::started();
    // Unlocking runs on the tick after a drawn frame: a display to draw on.
    app.event(Event::Display(osk_shell_api::DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".to_string(), testkit::test_vault().unwrap())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Vault(V::Open(0)));
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
    let together = app.vault_wallets_with_keys(0);
    let sharing: Vec<_> = together
        .iter()
        .filter(|w| w.name == "Spending" || w.name == "Savings")
        .collect();
    assert_eq!(sharing.len(), 2);
    assert_eq!(
        sharing[0].keys, sharing[1].keys,
        "one seed record, named by both"
    );
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(held(&app, "Spending"), 1);
    assert_eq!(held(&app, "Savings"), 1);
}
