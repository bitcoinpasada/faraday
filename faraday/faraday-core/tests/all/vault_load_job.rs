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
        vault_of("Main", PHRASE, 1, vec![key]).into(),
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
        vault_of("Main", PHRASE, 1, vec![key, wallet]).into(),
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
        vault_of("Main", PHRASE, 1, records).into(),
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
        // A lock wipes the keys in; the answer arriving after it adds none.
        let kept = if name == "Lock" { 0 } else { held };
        assert_eq!(app.session.keys.len(), kept, "{what}, {name}");
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

/// Create on a multisig's keys step over a vault of `records`, unlocked.
fn create_over(records: Vec<faraday_vault::Record>) -> Faraday {
    let mut app = app_with(records);
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
}

/// The first key record's place in the open vault.
fn first_key(app: &Faraday) -> usize {
    let id = app.vaults().open[0].id;
    app.vaults()
        .records(id)
        .of(kind::KEY)
        .next()
        .map(|(i, _)| i)
        .expect("the vault keeps a key")
}

/// The words of test seed 0 and a passphrase kept alone in the vault, for
/// the key they make together.
fn key_and_kept_passphrase() -> Vec<faraday_vault::Record> {
    use faraday_keys::identity::{Material, fingerprint};
    let c = testkit::compute();
    let words = Mnemonic::parse(
        &c,
        Language::English,
        &testkit::test_words(testkit::TEST_SEEDS[0].0),
    )
    .unwrap();
    let fp = fingerprint(&c, Material::Words(&words), "kept alone", testkit::net()).unwrap();
    vec![
        key_record(0),
        faraday_vault::Record::new(kind::PASSPHRASE)
            .with(field::PASSPHRASE_KEY, &fp.0)
            .with(field::PASSPHRASE, b"kept alone"),
    ]
}

#[test]
fn a_slot_of_create_filled_from_a_vault_loads_in_the_job_and_is_filled() {
    use faraday_core::create::Source;
    for records in [vec![key_record(0)], key_and_kept_passphrase()] {
        let kept_alone = records.len() > 1;
        let make = || {
            create_over(
                records
                    .iter()
                    .map(faraday_vault::Record::duplicate)
                    .collect(),
            )
        };
        let press = |app: &mut Faraday| {
            let r = first_key(app);
            testkit::press_on_worker(app, Action::CSlotVault(0, 0, r));
        };
        loads_then_leaves(&make, &press, 0, 1, "Create's slot from a vault");
        let mut app = make();
        press(&mut app);
        testkit::settle(&mut app);
        let loaded = app.session.keys.all().last().unwrap().fingerprint.0;
        assert_eq!(
            app.create().unwrap().slots[0],
            Source::Here(loaded),
            "the slot holds the key loaded (a passphrase kept alone: {kept_alone})"
        );
    }
}

#[test]
fn convert_from_a_vault_key_with_a_kept_passphrase_chooses_the_key_loaded() {
    let mut app = app_with(key_and_kept_passphrase());
    app.press(Action::Convert(faraday_core::convert::ConvertAction::Open));
    let _ = app.frame();
    let r = first_key(&app);
    testkit::press_on_worker(
        &mut app,
        Action::Convert(faraday_core::convert::ConvertAction::VaultKey(0, r)),
    );
    testkit::settle(&mut app);
    let loaded = app.session.keys.all().last().unwrap().fingerprint.0;
    assert_eq!(
        faraday_core::convert::state(&app).and_then(|s| s.key),
        Some(loaded)
    );
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

/// What the first wallet's backup plan holds at each place, as the chart
/// shows it.
fn plan_holds(app: &Faraday) -> Vec<Vec<String>> {
    use faraday_core::glance::Backup;
    let c = testkit::compute();
    let g = faraday_core::glance::of_now(&c, app, 0).expect("the wallet's chart");
    let Backup::Plan { nodes, .. } = &g.backup else {
        panic!("no plan");
    };
    nodes
        .iter()
        .map(|n| n.holds.iter().map(|h| String::from(&**h)).collect())
        .collect()
}

#[test]
fn a_chart_change_confirmed_before_its_seeds_are_loaded_is_made_when_they_are() {
    use crate::chart_edit::{clock_kept, single_key_with_vault};
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    use faraday_core::plan::{At, Edit, What};
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let before = plan_holds(&app);
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
    assert_eq!(app.chart(), None, "the change was confirmed");
    assert_ne!(plan_holds(&app), before, "the change was made");
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

// ---------------------------------------------------------------------
// The boot import's load, a backup's load and Restore from a vault.
// ---------------------------------------------------------------------

const LEAVES: [(&str, Action); 3] = [
    ("Back", Action::Back),
    ("Cancel", Action::Cancel),
    ("Lock", Action::Lock),
];

/// A wallet record and the key record of the seed it spends from.
fn wallet_and_key() -> Vec<faraday_vault::Record> {
    vec![
        wallet_record(&testkit::kits()[1].descriptor, b"Spending"),
        key_record(0),
    ]
}

/// The vault's wallet record's place.
fn wallet_index(app: &Faraday) -> usize {
    let id = app.vaults().open[0].id;
    app.vaults()
        .records(id)
        .of(kind::WALLET)
        .next()
        .map(|(i, _)| i)
        .expect("the vault keeps a wallet")
}

/// Restore from a vault pressed with its job held on the worker.
fn restore_pressed() -> Faraday {
    let mut app = app_with(wallet_and_key());
    assert!(app.session.keys.is_empty());
    let r = wallet_index(&app);
    app.press(Action::RestoreWallet);
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::RFromVault(0, r));
    assert!(app.busy().is_some(), "the seeds are loading");
    assert!(app.session.keys.is_empty(), "no key before the answer");
    app
}

#[test]
fn restore_from_a_vault_left_before_its_answer_adds_no_key() {
    for (name, action) in LEAVES {
        let mut app = restore_pressed();
        app.press(action);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        let _ = app.frame();
        assert!(app.session.keys.is_empty(), "{name}");
        if name != "Lock" {
            assert_ne!(app.screen, faraday_core::Screen::Restore, "{name}");
        }
    }
}

#[test]
fn restore_from_a_vault_let_finish_brings_in_the_keys_and_shows_restore() {
    let mut app = restore_pressed();
    testkit::release_worker();
    testkit::settle(&mut app);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.screen, faraday_core::Screen::Restore);
}

/// The boot import's vault unlocked, its Load sheet up and Load ticked
/// pressed so that its job is out on the worker, held there.
fn import_pressed() -> (Faraday, usize) {
    use crate::boot_import::{imported, run_unlock, type_in};
    let mut app = imported();
    type_in(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Import(
        faraday_core::boot_import::ImportAction::Submit,
    ));
    run_unlock(&mut app);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    let before = app.session.keys.len();
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Vault(V::LoadTicked));
    assert!(app.busy().is_some(), "the seeds are loading");
    assert_eq!(app.session.keys.len(), before, "no key before the answer");
    (app, before)
}

#[test]
fn the_boot_imports_load_left_before_its_answer_adds_no_key() {
    for (name, action) in LEAVES {
        let (mut app, before) = import_pressed();
        app.press(action);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        let _ = app.frame();
        // A lock wipes the keys already in; the others leave them.
        let left = app.session.keys.len();
        assert!(left <= before, "{name}");
        if name != "Lock" {
            assert_eq!(left, before, "{name}");
        }
    }
}

#[test]
fn the_boot_imports_load_let_finish_brings_in_the_vaults_keys_and_ends_the_import() {
    let (mut app, before) = import_pressed();
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(app.session.keys.len() > before);
    assert!(app.import().is_none());
    assert_eq!(app.screen, faraday_core::Screen::Wallets);
}

/// A backup of a wallet, written to its sticks after the lock and at Done
/// with its vault open and its keys not loaded; Unlock from Done pressed
/// so that the vault's load is out on the worker, held there.
fn backup_pressed() -> Faraday {
    use crate::backup_flow_wallet::{PHRASE, restart, single_key, step, write_sticks};
    use faraday_core::backup_flow::Step;
    use faraday_core::backup_flow::steps::FlowAction as F;
    let mut shell = crate::common::Shell::new();
    let (mut app, _, _) = single_key(&mut shell, true);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    // The vault is opened on Vaults, its Load sheet left unpressed.
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(faraday_core::Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    crate::common::type_text(&mut app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut app, 100);
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    assert!(app.session.keys.is_empty());
    testkit::hold_worker();
    testkit::press_on_worker(&mut app, Action::Flow(F::UnlockDone(0)));
    assert!(app.busy().is_some(), "the seeds are loading");
    assert!(app.session.keys.is_empty(), "no key before the answer");
    app
}

#[test]
fn a_backups_load_left_before_its_answer_adds_no_key() {
    for (name, action) in LEAVES {
        let mut app = backup_pressed();
        app.press(action);
        testkit::release_worker();
        testkit::settle_stale(&mut app);
        let _ = app.frame();
        assert!(app.session.keys.is_empty(), "{name}");
        if name != "Lock" {
            assert_ne!(app.screen, faraday_core::Screen::Backup, "{name}");
        }
    }
}

#[test]
fn a_backups_load_let_finish_brings_in_the_keys_and_shows_the_checklist() {
    let mut app = backup_pressed();
    testkit::release_worker();
    testkit::settle(&mut app);
    assert!(!app.session.keys.is_empty());
    assert!(!app.session.wallets.is_empty());
    assert_eq!(app.screen, faraday_core::Screen::Backup);
}

/// A passphrase typed on a wallet's chart and checked with its seeds not
/// loaded: loaded first, then compared, by the button or by Enter.
/// The passphrase check open on a chart, the seeds not loaded, `typed` in it.
fn pass_check_typed(typed: &str) -> (Faraday, faraday_core::glance::Press) {
    use crate::chart_edit::single_key_with_vault_pass;
    use faraday_core::glance::Press;
    use faraday_core::glance::sheet::{ChartAction as C, Target};
    use faraday_core::plan::{At, Edit, What};
    const PASS: &str = "lantern-orchid-41";
    let (mut app, _) = single_key_with_vault_pass(PASS);
    crate::chart_edit::clock_kept(&mut app);
    app.press(Action::Nav(faraday_core::Screen::VaultContents));
    app.press(Action::Vault(V::Category(1)));
    app.press(Action::Vault(V::Item(0)));
    let _ = app.frame();
    let r = app.vault_selected_index().expect("a wallet is chosen");
    let press = Press::Vault(0, r);
    app.press(Action::Chart(C::Open(
        press,
        Target::CopyTo(What::Passphrase(0)),
    )));
    let _ = app.frame();
    app.press(Action::Chart(C::Edit(
        press,
        Edit::Add(At::Place(2), What::Passphrase(0)),
    )));
    app.press(Action::Chart(C::Confirm(press)));
    let Some((_, Target::PassShow(k, n, j))) = app.chart() else {
        panic!("the passphrase is not shown: {:?}", app.chart());
    };
    app.press(Action::Chart(C::Open(press, Target::PassCheck(k, n, j))));
    while !app.session.keys.is_empty() {
        app.session.keys.remove(0);
    }
    crate::common::type_text(&mut app, typed);
    (app, press)
}

fn pass_checked(enter: bool, typed: &str) -> Option<bool> {
    use faraday_core::glance::sheet::ChartAction as C;
    let (mut app, press) = pass_check_typed(typed);
    if enter {
        app.event(Event::Key(Key::Enter));
    } else {
        testkit::press_on_worker(&mut app, Action::Chart(C::PassTry(press)));
    }
    assert!(app.busy().is_some(), "the seeds are loading");
    testkit::settle(&mut app);
    assert!(!app.session.keys.is_empty(), "the seeds are in");
    app.chart_work().result
}

#[test]
fn a_passphrase_checked_before_its_seeds_are_loaded_is_compared_when_they_are() {
    for enter in [false, true] {
        assert_eq!(
            pass_checked(enter, "lantern-orchid-41"),
            Some(true),
            "{enter}"
        );
        assert_eq!(pass_checked(enter, "not it"), Some(false), "{enter}");
    }
}

#[test]
fn escape_on_a_charts_sheet_while_its_seeds_load_leaves_none_of_them_in() {
    let (mut app, _) = pass_check_typed("lantern-orchid-41");
    app.event(Event::Key(Key::Enter));
    assert!(app.busy().is_some(), "the seeds are loading");
    app.event(Event::Key(Key::Escape));
    testkit::settle(&mut app);
    assert!(app.session.keys.is_empty(), "the seeds were not kept");
    assert_eq!(app.chart(), None, "the sheet is closed");
}
