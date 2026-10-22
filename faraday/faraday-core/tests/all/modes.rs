//! Modes (F81, `docs/MODES.md`): Demo, Mainnet, Testnet, Signet and Regtest.
//! A mode change is a lock that leaves a fresh session in the mode
//! chosen, and a lock keeps the mode it was in. Demo is the public seeds
//! on testnet: its addresses are real testnet addresses, and its dimmed
//! actions are live on a test network.

use faraday_bip::keys::Network;
use faraday_core::seeds::SeedsAction as S;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::Session;
use faraday_core::{
    Action, DESKTOP_MODES, Faraday, Mode, STICK_MODES, Screen, Sheet, StorageEvent, demo, testkit,
};
use faraday_shell_api::{App, Event};

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                       abandon abandon about";

/// The desktop's app: online, in Demo.
fn demo_app() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.online = true;
    app.session = Session::on(Network::Testnet);
    app.session.set_demo(&c);
    app.set_modes(DESKTOP_MODES);
    app
}

#[test]
fn choosing_a_mode_with_keys_and_wallets_loaded_leaves_a_fresh_session_in_it() {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    let kit = testkit::kits().into_iter().next().unwrap();
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.session
        .add_wallet(
            &c,
            "w",
            &kit.descriptor,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Mode(Mode::Signet));
    assert!(app.restart_requested());
    assert!(app.session.keys.is_empty(), "the keys are gone");
    assert!(app.session.wallets.is_empty(), "the wallets are gone");
    assert_eq!(app.mode_after_restart(), Mode::Signet);
    assert_eq!(app.sheet, None);
}

#[test]
fn a_lock_keeps_the_mode() {
    for mode in [Mode::Mainnet, Mode::Testnet, Mode::Regtest] {
        let mut app = testkit::started();
        testkit::move_to(&mut app.session, mode.network()).unwrap();
        app.press(Action::Lock);
        assert!(app.restart_requested(), "{mode:?}");
        assert_eq!(app.mode_after_restart(), mode);
    }
    let mut app = demo_app();
    app.press(Action::Lock);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Demo);
}

#[test]
fn the_mode_in_use_is_read_from_the_session() {
    let mut app = testkit::started();
    assert_eq!(app.mode(), Mode::Mainnet);
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    assert_eq!(app.mode(), Mode::Testnet);
    assert_eq!(app.modes(), STICK_MODES);
    assert_eq!(demo_app().mode(), Mode::Demo);
    assert_eq!(demo_app().mode().network(), Network::Testnet);
}

#[test]
fn choosing_the_mode_in_use_or_one_not_offered_does_nothing() {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
    app.press(Action::Mode(Mode::Mainnet));
    assert!(!app.restart_requested());
    // The stick has no Demo.
    app.press(Action::Mode(Mode::Demo));
    assert!(!app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
    // The desktop has no Mainnet.
    let mut app = demo_app();
    app.press(Action::Mode(Mode::Mainnet));
    assert!(!app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Demo);
}

/// A device with the test vault open and a change in it that has not
/// been sealed: a lock asks first.
fn with_unsealed_change() -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Mainnet).unwrap();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(100);
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".into(), testkit::test_vault().unwrap())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Vault(V::Open(0)));
    crate::common::type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the vault did not open");
    app.press(Action::Entry(None));
    crate::common::type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let k = app.session.keys.len() - 1;
    app.press(Action::Vault(V::SaveKey(k)));
    assert!(app.vaults.open[0].changes > 0, "the vault was not changed");
    app.event(Event::Tick { now_ms: 120_000 });
    app
}

#[test]
fn choosing_a_mode_where_a_lock_asks_shows_the_lock_sheet_and_not_now_keeps_the_mode() {
    let mut app = with_unsealed_change();
    app.press(Action::Mode(Mode::Testnet));
    assert_eq!(app.sheet, Some(Sheet::LockAsk));
    assert!(!app.restart_requested(), "nothing is wiped yet");
    assert_eq!(app.vaults.open.len(), 1);
    assert_eq!(app.mode_after_restart(), Mode::Testnet);
    // Not now: the person stays where they were, in the mode in use.
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.mode_after_restart(), Mode::Mainnet);
    // Locking anyway ends in the mode chosen.
    app.press(Action::Mode(Mode::Testnet));
    app.press(Action::LockUnsealed);
    assert!(app.restart_requested());
    assert_eq!(app.mode_after_restart(), Mode::Testnet);
}

#[test]
fn demo_addresses_are_testnet_addresses_a_testnet_wallet_sits_beside() {
    let c = testkit::compute();
    let mut app = demo_app();
    app.press(Action::Entry(None));
    crate::common::type_text(&mut app, &demo::words("bacon"));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.first().unwrap().fingerprint.0;
    app.press(Action::KeyWallet(fp, 1));
    app.press(Action::Seeds(S::Make));
    let ours = &app.session.wallets[app.session.wallets.len() - 1];
    let address = app.session.address(&c, ours, false, 0);
    assert!(address.starts_with("tb1q"), "{address}");
    // A real testnet wallet from elsewhere loads in the same session.
    let kit = testkit::kits().into_iter().next().unwrap();
    app.session
        .add_wallet(
            &c,
            "t",
            &kit.descriptor,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_eq!(app.mode(), Mode::Demo);
    assert_eq!(app.session.wallets.len(), 2);
}

#[test]
fn demo_refuses_a_key_that_is_not_public_and_takes_a_public_one() {
    let c = testkit::compute();
    let mut app = demo_app();
    assert!(app.session.keys.add_words(&c, ABANDON, "", None).is_err());
    assert!(
        app.session
            .keys
            .add_words(&c, &demo::words("gas"), "", None)
            .is_ok()
    );
    // A passphrase that does not make a public key: refused.
    assert!(
        app.session
            .keys
            .add_words_with(&c, &demo::words("trap"), "not public", "", None)
            .is_err()
    );
}

#[test]
fn a_testnet_wallet_moves_an_empty_stick_session_to_testnet_and_a_loaded_key_stops_it() {
    let c = testkit::compute();
    let kit = testkit::kits().into_iter().next().unwrap();
    let mut empty = testkit::started();
    testkit::move_to(&mut empty.session, Network::Mainnet).unwrap();
    empty
        .session
        .add_wallet(
            &c,
            "t",
            &kit.descriptor,
            "t",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert_eq!(empty.mode(), Mode::Testnet);

    let mut keyed = testkit::started();
    testkit::move_to(&mut keyed.session, Network::Mainnet).unwrap();
    keyed.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    let refused = keyed.session.add_wallet(
        &c,
        "t",
        &kit.descriptor,
        "t",
        faraday_core::wallet::Origin::Named,
    );
    assert!(refused.is_err());
    assert_eq!(keyed.mode(), Mode::Mainnet);
}

#[test]
fn the_actions_the_demo_dims_are_live_on_testnet() {
    let demo = demo_app();
    let testnet = testkit::started();
    for a in [
        Action::BXor(2),
        Action::PShow,
        Action::LShow,
        Action::LAezeed,
        Action::SScanVault,
    ] {
        assert!(demo.demo_refuses(a), "{a:?} is open in Demo");
        assert!(!testnet.demo_refuses(a), "{a:?} is dimmed on Testnet");
    }
    assert!(!demo.makes_secrets());
    assert!(testnet.makes_secrets());
}

#[test]
fn demo_hands_out_a_public_seed_for_new_key_and_testnet_does_not() {
    let demo = demo_app();
    assert!(demo.session.keys.next_handout().is_some());
    let testnet = testkit::started();
    assert!(testnet.session.keys.next_handout().is_none());
}
