//! Loading a key from an open vault: Load all and Load on one record work
//! the seed out on the worker. Back, Cancel or Lock pressed before the
//! answer leaves the key out; let to finish, the key is in as before.

use crate::common::*;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, StorageEvent, testkit};
use faraday_shell_api::{App, Event, Key};
use faraday_vault::records::{field, kind};

const PHRASE: &str = "load job phrase";

/// An app with a vault of one key record, unlocked.
fn unlocked() -> Faraday {
    let c = testkit::compute();
    let words = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    let key = faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&words));
    let files = vec![(
        "main.kdbx".to_string(),
        vault_of("Main", PHRASE, 1, vec![key]),
    )];
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Vault(V::Open(0)));
    for c in PHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    app
}

/// A press, named.
type Press = (&'static str, Box<dyn Fn(&mut Faraday)>);

/// The presses that load the one key, each ready to press.
fn loads() -> Vec<Press> {
    vec![
        (
            "Load all",
            Box::new(|app| {
                testkit::press_on_worker(app, Action::Vault(V::LoadAll(0)));
            }),
        ),
        (
            "Load on a record",
            Box::new(|app| {
                app.press(Action::Vault(V::Item(0)));
                testkit::press_on_worker(app, Action::Vault(V::Load));
            }),
        ),
    ]
}

/// The press done, its job out on the worker and held there.
fn pressed(press: &dyn Fn(&mut Faraday)) -> Faraday {
    let mut app = unlocked();
    assert!(app.session.keys.is_empty());
    testkit::hold_worker();
    press(&mut app);
    assert!(app.busy().is_some(), "the seed is loading");
    assert!(
        app.session.keys.is_empty(),
        "nothing is added before the answer"
    );
    app
}

#[test]
fn a_load_left_before_its_answer_adds_no_key() {
    for (what, press) in loads() {
        for (name, action) in [
            ("Back", Action::Back),
            ("Cancel", Action::Cancel),
            ("Lock", Action::Lock),
        ] {
            let mut app = pressed(&*press);
            app.press(action);
            testkit::release_worker();
            testkit::settle_stale(&mut app);
            let _ = app.frame();
            assert!(app.session.keys.is_empty(), "{what}, {name}");
        }
    }
}

#[test]
fn a_load_let_finish_adds_the_key() {
    for (what, press) in loads() {
        let mut app = pressed(&*press);
        testkit::release_worker();
        testkit::settle(&mut app);
        assert_eq!(app.session.keys.len(), 1, "{what}");
    }
}

/// A vault of one key and the one wallet that spends from it, unlocked,
/// then Load a wallet with its keys pressed so that its job is out on the
/// worker, held there.
fn wallets_pressed() -> Faraday {
    let c = testkit::compute();
    let words = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    let key = faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&words));
    let wallet = faraday_vault::Record::new(kind::WALLET)
        .with(field::WALLET, testkit::kits()[1].descriptor.as_bytes())
        .with(field::WALLET_NAME, b"Spending");
    let files = vec![(
        "main.kdbx".to_string(),
        vault_of("Main", PHRASE, 1, vec![key, wallet]),
    )];
    let mut app = testkit::started();
    // The test wallet is a testnet one.
    testkit::move_to(&mut app.session, faraday_bip::keys::Network::Testnet).unwrap();
    app.storage(StorageEvent::Restored {
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Vault(V::Open(0)));
    for c in PHRASE.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.session.keys.is_empty());
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadWithKeys(0)));
    assert!(app.busy().is_some(), "the seeds are loading");
    app
}

#[test]
fn a_wallet_loaded_with_its_keys_and_left_before_the_answer_adds_nothing() {
    for (name, action) in [
        ("Back", Action::Back),
        ("Cancel", Action::Cancel),
        ("Lock", Action::Lock),
    ] {
        let mut app = wallets_pressed();
        app.press(action);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        let _ = app.frame();
        assert!(app.session.keys.is_empty(), "{name}");
        assert!(app.session.wallets.is_empty(), "{name}");
    }
}

#[test]
fn a_wallet_loaded_with_its_keys_arrives_with_them_when_the_answer_is_taken() {
    let mut app = wallets_pressed();
    assert!(app.session.wallets.is_empty());
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(!app.session.keys.is_empty());
    assert!(!app.session.wallets.is_empty());
}
