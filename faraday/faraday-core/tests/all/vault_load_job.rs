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

// ---------------------------------------------------------------------
// The other loads from a vault, a press run again after its load, and
// what a load still refuses.
// ---------------------------------------------------------------------

fn key_record(n: usize) -> faraday_vault::Record {
    let c = testkit::compute();
    let words = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[n].0),
    )
    .unwrap();
    faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&words))
}

fn wallet_record(descriptor: &str, name: &[u8]) -> faraday_vault::Record {
    faraday_vault::Record::new(kind::WALLET)
        .with(field::WALLET, descriptor.as_bytes())
        .with(field::WALLET_NAME, name)
}

/// An app on Testnet with a vault of `records`, unlocked, nothing loaded.
fn app_with(records: Vec<faraday_vault::Record>) -> Faraday {
    let files = vec![(
        "main.kdbx".to_string(),
        vault_of("Main", PHRASE, 1, records),
    )];
    let mut app = testkit::started();
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
    app
}

/// The press on the worker held there, then each way of leaving before
/// the answer adds no key; let finish, the keys `want` are in.
fn loads_then_leaves(
    make: &dyn Fn() -> Faraday,
    press: &dyn Fn(&mut Faraday),
    held: usize,
    want: usize,
    what: &str,
) {
    for (name, action) in [
        ("Back", Action::Back),
        ("Cancel", Action::Cancel),
        ("Lock", Action::Lock),
    ] {
        let mut app = make();
        testkit::hold_worker();
        press(&mut app);
        assert!(app.busy().is_some(), "{what}: the seeds are loading");
        assert_eq!(app.session.keys.len(), held, "{what}: nothing before");
        app.press(action);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        let _ = app.frame();
        assert_eq!(app.session.keys.len(), held, "{what}, {name}");
    }
    let mut app = make();
    press(&mut app);
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), want, "{what}: let finish");
}

#[test]
fn a_seed_taken_into_a_flow_from_a_vault_loads_in_the_job() {
    let make = || app_with(vec![key_record(0)]);
    let press = |app: &mut Faraday| {
        let r = app.vault_selected_index().unwrap_or(0);
        testkit::press_on_worker(
            app,
            Action::Seeds(faraday_core::seeds::SeedsAction::FromVault(0, r)),
        );
    };
    loads_then_leaves(&make, &press, 0, 1, "a seed taken from a vault");
}

#[test]
fn convert_from_a_vault_key_loads_it_in_the_job_and_goes_on() {
    let make = || {
        let mut app = app_with(vec![key_record(0)]);
        app.press(Action::Convert(faraday_core::convert::ConvertAction::Open));
        let _ = app.frame();
        app
    };
    let press = |app: &mut Faraday| {
        let id = app.vaults().open[0].id;
        let r = app
            .vaults()
            .records(id)
            .of(kind::KEY)
            .next()
            .map(|(i, _)| i)
            .expect("the vault keeps a key");
        testkit::press_on_worker(
            app,
            Action::Convert(faraday_core::convert::ConvertAction::VaultKey(0, r)),
        );
    };
    loads_then_leaves(&make, &press, 0, 1, "Convert's From a vault");
}

#[test]
fn a_slot_of_create_filled_from_a_vault_loads_in_the_job_and_is_filled() {
    let make = || {
        let mut app = app_with(vec![key_record(0)]);
        use faraday_core::{create::NewKind, cstep};
        let i = NewKind::ALL
            .iter()
            .position(|k| *k == NewKind::Multi)
            .unwrap() as u8;
        app.press(Action::CreateWallet);
        crate::common::ckind(&mut app, i);
        app.press(Action::CNext(cstep::KIND));
        app.press(Action::CVaults(false));
        app.press(Action::CNext(cstep::WHERE));
        app.press(Action::CNext(cstep::QUORUM));
        let _ = app.frame();
        assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
        app
    };
    let press = |app: &mut Faraday| {
        let id = app.vaults().open[0].id;
        let r = app
            .vaults()
            .records(id)
            .of(kind::KEY)
            .next()
            .map(|(i, _)| i)
            .expect("the vault keeps a key");
        testkit::press_on_worker(app, Action::CSlotVault(0, 0, r));
    };
    loads_then_leaves(&make, &press, 0, 1, "Create's slot from a vault");
}

/// A vault of a wallet over the three test seeds and the seeds of `n` of
/// them, its wallet in the session with no key loaded.
fn wallet_without_keys(n: usize) -> Faraday {
    let mut records = vec![wallet_record(&testkit::kits()[0].descriptor, b"Savings")];
    records.extend((0..n).map(key_record));
    let mut app = app_with(records);
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadWithKeys(0)));
    testkit::settle(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    while !app.session.keys.is_empty() {
        app.session.keys.remove(0);
    }
    app
}

#[test]
fn a_new_wallet_from_this_one_loads_every_seed_the_vault_keeps_of_it() {
    let make = || wallet_without_keys(2);
    let press = |app: &mut Faraday| {
        testkit::press_on_worker(app, Action::NewWalletFrom(0));
    };
    loads_then_leaves(&make, &press, 0, 2, "New wallet from this one");
    let mut app = make();
    press(&mut app);
    testkit::settle(&mut app);
    assert_eq!(app.screen, faraday_core::Screen::Create, "the flow went on");
}

#[test]
fn a_chart_change_confirmed_before_its_seeds_are_loaded_is_made_when_they_are() {
    use crate::chart_edit::{clock_kept, single_key_with_vault};
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    use faraday_core::plan::{At, Edit, What};
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    app.press(Action::Nav(faraday_core::Screen::VaultContents));
    app.press(Action::Vault(V::Category(1)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    let r = app.vault_selected_index().expect("a wallet is chosen");
    let press = Press::Vault(0, r);
    let edit = Edit::Move(At::Place(0), What::Words(0), At::Place(2));
    let forget = |app: &mut Faraday| {
        while !app.session.keys.is_empty() {
            app.session.keys.remove(0);
        }
    };
    // Asked with the seeds not loaded: the ask is made once they are.
    forget(&mut app);
    testkit::press_on_worker(&mut app, Action::Chart(C::Edit(press, edit)));
    assert!(app.busy().is_some(), "the seeds are loading");
    testkit::settle(&mut app);
    assert_eq!(app.chart(), Some((press, Target::Confirm)));
    // Confirmed with them not loaded: the ask waits for the answer.
    forget(&mut app);
    testkit::press_on_worker(&mut app, Action::Chart(C::Confirm(press)));
    assert!(app.busy().is_some(), "the seeds are loading");
    assert!(
        app.chart().is_some(),
        "the change asked for waits for the answer"
    );
    testkit::settle(&mut app);
    assert!(!app.session.keys.is_empty());
    assert_eq!(app.chart(), None, "the change was confirmed and made");
}

#[test]
fn a_wallet_with_a_loaded_keys_fingerprint_and_another_xpub_is_still_refused() {
    // Seed 1's account key, named as seed 0's.
    let path = "m/84'/1'/0'";
    let other = testkit::key(1, path);
    let named = other.replace(
        &testkit::seed_fingerprint(1).to_string(),
        &testkit::seed_fingerprint(0).to_string(),
    );
    let forged = format!("wpkh({named}/<0;1>/*)");
    let mut app = app_with(vec![key_record(0), wallet_record(&forged, b"Forged")]);
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadWithKeys(0)));
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1, "the seed loads");
    assert!(app.session.wallets.is_empty(), "the forged wallet does not");
    let said = app.toast_text().unwrap_or_default().to_string();
    assert!(said.contains("Forged"), "{said}");
}

#[test]
fn a_key_that_is_not_a_sample_says_why_it_is_not_loaded_in_practice() {
    // Words that are not one of the samples Practice takes.
    let c = testkit::compute();
    let words = Mnemonic::parse(
        &c,
        Language::English,
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    )
    .unwrap();
    let record = faraday_vault::Record::new(kind::KEY)
        .with(field::KEY, &faraday_vault::records::words_payload(&words));
    let mut app = app_with(vec![record]);
    testkit::into_demo(&mut app);
    app.press(Action::Vault(V::Item(0)));
    testkit::press_on_worker(&mut app, Action::Vault(V::Load));
    testkit::settle(&mut app);
    assert!(app.session.keys.is_empty());
    let said = app.toast_text().unwrap_or_default().to_string();
    assert!(said.contains("not a sample"), "{said}");
}
