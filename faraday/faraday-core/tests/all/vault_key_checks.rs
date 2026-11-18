//! A wallet of several keys saved into a vault keeps the keys the person
//! ticked as checked, and comes back from the vault with them; a wallet a
//! vault holds with no ticks comes back with every key not its own not
//! checked.

use crate::common::{Shell, process, type_text};
use faraday_core::multisig_check::MsAction;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::KeyCheck;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_shell_api::{App, Event};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

const PHRASE: &str = "ticked vault phrase";

fn multisig() -> String {
    let p = "m/48'/1'/0'/0'";
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, p),
        testkit::key(1, p),
        testkit::key(2, p)
    )
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
}

fn wait(app: &mut Faraday, what: &str, done: impl Fn(&Faraday) -> bool) {
    for t in 1..120u64 {
        if done(app) {
            return;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert!(
        done(app),
        "{what}: {:?} {:?}",
        app.screen,
        app.vaults().unlock_error
    );
}

/// A vault made and unlocked here, the wallet saved into it with the keys
/// `ticked` ticked; its file as the lock sealed it.
fn vault_holding_wallet(ticked: &[usize]) -> Vec<(String, Vec<u8>)> {
    vault_holding_wallet_then(ticked, |_, _| {})
}

/// As [`vault_holding_wallet`], with `after` run on the wallet (its place)
/// once it is saved, before the lock.
fn vault_holding_wallet_then(
    ticked: &[usize],
    after: impl FnOnce(&mut Faraday, usize),
) -> Vec<(String, Vec<u8>)> {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut app, Some(180));
    add_key(&mut app, 0);
    let w = app
        .session
        .add_wallet_confirmed(
            &c,
            "Ticked",
            &multisig(),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    for &k in ticked {
        app.session.tick_key(w, k);
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        type_text(&mut app, PHRASE);
    }
    app.press(Action::Vault(V::CGo));
    wait(&mut app, "made", |a| {
        a.screen == Screen::Unlock || !a.vaults().open.is_empty()
    });
    if app.vaults().open.is_empty() {
        app.press(Action::Vault(V::FocusPassphrase));
        type_text(&mut app, PHRASE);
        app.press(Action::Vault(V::Unlock));
        wait(&mut app, "unlocked", |a| !a.vaults().open.is_empty());
    }
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::AddKind(1)));
    app.press(Action::Vault(V::SaveWallet(w)));
    after(&mut app, w);
    let app = shell.lock(&mut app);
    app.outbox
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .collect()
}

/// Another power-on with the vault `files` unlocked; its wallets are
/// loaded at the unlock.
fn unlocked(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = process(files, Vec::new(), Vec::new());
    testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    app.press(Action::Vault(V::FocusPassphrase));
    type_text(&mut app, PHRASE);
    app.press(Action::Vault(V::Unlock));
    wait(&mut app, "unlocked", |a| !a.vaults().open.is_empty());
    testkit::settle(&mut app);
    // A vault of several loadable items asks what to load first.
    if app.sheet.is_some() {
        app.press(Action::Vault(V::LoadTicked));
        testkit::settle(&mut app);
    }
    app
}

#[test]
fn a_ticked_key_comes_back_checked_and_the_others_not() {
    let mut app = unlocked(vault_holding_wallet(&[1]));
    assert_eq!(app.session.wallets.len(), 1, "the wallet loaded");
    let w = &app.session.wallets[0];
    assert_eq!(
        w.key_checks(),
        [KeyCheck::Unchecked, KeyCheck::Checked, KeyCheck::Unchecked]
    );
    assert_eq!(w.keys_unchecked(), 2);
    // A key loaded now is its own, whatever the vault held.
    add_key(&mut app, 0);
    assert_eq!(
        app.session.wallets[0].key_checks(),
        [KeyCheck::Yours, KeyCheck::Checked, KeyCheck::Unchecked]
    );
}

#[test]
fn a_wallet_saved_with_no_key_ticked_comes_back_with_none() {
    let app = unlocked(vault_holding_wallet(&[]));
    let w = &app.session.wallets[0];
    assert_eq!(w.key_ticks(), [false, false, false]);
    assert_eq!(w.keys_unchecked(), 3);
}

#[test]
fn a_wallet_a_vault_holds_with_no_ticks_loads_unchecked() {
    let file = testkit::kdbx_vault(
        "Plain vault",
        PHRASE,
        0x5c,
        vec![
            Record::new(kind::WALLET)
                .with(field::WALLET, multisig().as_bytes())
                .with(field::WALLET_NAME, b"Plain"),
        ],
    )
    .unwrap();
    let app = unlocked(vec![("plain.kdbx".to_string(), file)]);
    let w = &app.session.wallets[0];
    assert_eq!(w.keys_unchecked(), 3);
    assert!(w.key_ticks().iter().all(|t| !t));
}

/// The same wallet with its keys in the order `order` names.
fn multisig_in(order: [usize; 3]) -> String {
    let p = "m/48'/1'/0'/0'";
    let k: Vec<String> = order
        .iter()
        .map(|&n| format!("{}/<0;1>/*", testkit::key(n, p)))
        .collect();
    format!("wsh(sortedmulti(2,{},{},{}))", k[0], k[1], k[2])
}

fn record_of(order: [usize; 3], ticked: &[usize], loads: bool) -> Record {
    let mut r = Record::new(kind::WALLET)
        .with(field::WALLET, multisig_in(order).as_bytes())
        .with(field::WALLET_NAME, b"Twice");
    faraday_vault::records::set_keys_checked(&mut r, ticked);
    faraday_vault::records::set_loads_at_unlock(&mut r, loads);
    r
}

/// The keys of the wallet that read as checked, as the numbers `testkit::key`
/// takes, found by master fingerprint.
fn checked_keys(app: &Faraday) -> Vec<usize> {
    let w = &app.session.wallets[0];
    let fp_of = |n: usize| {
        let key = testkit::key(n, "m/48'/1'/0'/0'");
        key.trim_start_matches('[')[..8].to_ascii_lowercase()
    };
    (0..3)
        .filter(|&n| {
            w.policy().keys().iter().zip(w.key_checks()).any(|(k, c)| {
                *c == KeyCheck::Checked
                    && k.fingerprint().is_some_and(|f| {
                        faraday_core::wallet::fp_text(f).to_ascii_lowercase() == fp_of(n)
                    })
            })
        })
        .collect()
}

#[test]
fn two_records_of_one_wallet_in_other_key_orders_tick_exactly_their_keys() {
    // The first record ticks its place 0 (key 0); the second, listing the
    // keys the other way round, ticks its place 0 (key 2).
    let file = testkit::kdbx_vault(
        "Twice",
        PHRASE,
        0x5d,
        vec![
            record_of([0, 1, 2], &[0], true),
            record_of([2, 1, 0], &[0], true),
        ],
    )
    .unwrap();
    let app = unlocked(vec![("twice.kdbx".to_string(), file)]);
    assert_eq!(
        app.session.wallets.len(),
        1,
        "{:?} {:?}",
        app.toast_text(),
        app.screen
    );
    assert_eq!(
        checked_keys(&app),
        [0, 2],
        "{:?}",
        app.session.wallets[0].key_ticks()
    );
}

#[test]
fn a_key_unticked_after_the_save_reads_not_checked_at_the_next_unlock() {
    let files = vault_holding_wallet_then(&[2], |app, w| {
        app.press(Action::KeyTick(
            faraday_core::wallet::KeyBlockOf::Wallet(w as u16),
            2,
        ));
    });
    let app = unlocked(files);
    assert_eq!(app.session.wallets[0].key_ticks(), [false, false, false]);
    assert_eq!(app.session.wallets[0].key_checks()[2], KeyCheck::Unchecked);
}

#[test]
fn a_key_ticked_after_the_save_reads_checked_at_the_next_unlock() {
    let files = vault_holding_wallet_then(&[], |app, w| {
        app.press(Action::KeyTick(
            faraday_core::wallet::KeyBlockOf::Wallet(w as u16),
            1,
        ));
    });
    let app = unlocked(files);
    assert_eq!(app.session.wallets[0].key_checks()[1], KeyCheck::Checked);
}

#[test]
fn the_vault_items_load_brings_the_records_ticks() {
    let file = testkit::kdbx_vault(
        "Load",
        PHRASE,
        0x5e,
        vec![record_of([2, 1, 0], &[0], false)],
    )
    .unwrap();
    let mut app = unlocked(vec![("load.kdbx".to_string(), file)]);
    assert!(app.session.wallets.is_empty(), "not loaded at the unlock");
    app.press(Action::Vault(V::Item(0)));
    app.press(Action::Vault(V::Load));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(checked_keys(&app), [2]);
}

#[test]
fn restore_from_a_vault_brings_the_records_ticks() {
    let file = testkit::kdbx_vault(
        "Restore",
        PHRASE,
        0x5f,
        vec![record_of([0, 1, 2], &[1], false)],
    )
    .unwrap();
    let mut app = unlocked(vec![("restore.kdbx".to_string(), file)]);
    app.press(Action::RestoreWallet);
    let record = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .iter()
        .position(|r| r.text(field::WALLET_NAME) == Some("Twice"))
        .expect("the vault's wallet");
    app.press(Action::RFromVault(0, record));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(checked_keys(&app), [1]);
}

#[test]
fn a_key_ticked_on_the_sheet_for_a_wallet_a_vault_holds_reads_checked_at_the_next_unlock() {
    let mut app = unlocked(vault_holding_wallet(&[]));
    assert_eq!(app.session.wallets.len(), 1);
    // The same wallet met again in Check a multisig setup, a key ticked
    // there and the wallet added.
    app.press(Action::MultisigCheck(MsAction::Open));
    app.press(Action::MultisigCheck(MsAction::Scan));
    app.event(Event::Scanned {
        bytes: multisig().into_bytes(),
    });
    let _ = app.frame();
    app.press(Action::KeyTick(faraday_core::wallet::KeyBlockOf::Check, 1));
    app.press(Action::MultisigCheck(MsAction::Add));
    if app.sheet == Some(Sheet::WalletConfirm) {
        app.press(Action::WalletHold);
    }
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.wallets[0].key_checks()[1], KeyCheck::Checked);
    let mut shell = Shell::new();
    let app = shell.lock(&mut app);
    let files: Vec<_> = app
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .collect();
    let app = unlocked(files);
    assert_eq!(app.session.wallets[0].key_checks()[1], KeyCheck::Checked);
}

fn held_file() -> Vec<u8> {
    testkit::kdbx_vault(
        "Held",
        PHRASE,
        0x60,
        vec![record_of([0, 1, 2], &[1, 2], false)],
    )
    .unwrap()
}

/// The vault holds a record of the wallet with keys 1 and 2 ticked, and the
/// wallet is loaded from a file, not from the record, through its sheet
/// with nothing ticked.
fn held_with_two_ticked_and_loaded_from_a_file() -> Faraday {
    let mut app = unlocked(vec![
        ("held.kdbx".to_string(), held_file()),
        (
            "zz-wallet.txt".to_string(),
            multisig_in([0, 1, 2]).into_bytes(),
        ),
    ]);
    assert!(app.session.wallets.is_empty());
    testkit::load_wallet(&mut app, 1);
    assert_eq!(app.session.wallets.len(), 1);
    app
}

/// Locked and unlocked again, the wallet loaded from the vault's record.
fn reloaded_from_the_record(mut app: Faraday) -> Faraday {
    let mut shell = Shell::new();
    let mut files: Vec<_> = shell
        .lock(&mut app)
        .outbox
        .iter()
        .filter(|i| i.name.ends_with(".kdbx"))
        .map(|i| (String::from(&*i.name), i.bytes.to_vec()))
        .collect();
    // A vault nothing changed in is not written again.
    if files.is_empty() {
        files.push(("held.kdbx".to_string(), held_file()));
    }
    let mut app = unlocked(files);
    assert!(app.session.wallets.is_empty());
    app.press(Action::Vault(V::Item(0)));
    app.press(Action::Vault(V::Load));
    assert_eq!(app.session.wallets.len(), 1);
    app
}

#[test]
fn a_tick_keeps_the_keys_a_vault_remembers_of_a_wallet_loaded_from_a_file() {
    let mut app = held_with_two_ticked_and_loaded_from_a_file();
    app.press(Action::KeyTick(
        faraday_core::wallet::KeyBlockOf::Wallet(0),
        0,
    ));
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [0, 1, 2]);
}

#[test]
fn a_sheet_confirmed_with_nothing_ticked_keeps_what_the_vault_remembers() {
    let app = held_with_two_ticked_and_loaded_from_a_file();
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [1, 2]);
}

/// The wallet met in Check a multisig setup (its keys in `order`), key 0
/// ticked there and the wallet added.
fn added_through_check_with_key_0_ticked(app: &mut Faraday, order: [usize; 3]) {
    app.press(Action::MultisigCheck(MsAction::Open));
    app.press(Action::MultisigCheck(MsAction::Scan));
    app.event(Event::Scanned {
        bytes: multisig_in(order).into_bytes(),
    });
    let _ = app.frame();
    let place = order.iter().position(|&n| n == 0).unwrap() as u8;
    app.press(Action::KeyTick(
        faraday_core::wallet::KeyBlockOf::Check,
        place,
    ));
    app.press(Action::MultisigCheck(MsAction::Add));
    if app.sheet == Some(Sheet::WalletConfirm) {
        app.press(Action::WalletHold);
    }
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_sheet_only_adds_to_what_the_vault_remembers() {
    let mut app = unlocked(vec![("held.kdbx".to_string(), held_file())]);
    added_through_check_with_key_0_ticked(&mut app, [0, 1, 2]);
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [0, 1, 2]);
}

#[test]
fn a_sheet_with_the_keys_in_another_order_adds_the_key_by_its_fingerprint() {
    let mut app = unlocked(vec![("held.kdbx".to_string(), held_file())]);
    added_through_check_with_key_0_ticked(&mut app, [2, 1, 0]);
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [0, 1, 2]);
}

#[test]
fn an_untick_reaches_a_record_the_wallet_was_not_loaded_from() {
    let mut app = held_with_two_ticked_and_loaded_from_a_file();
    for _ in 0..2 {
        app.press(Action::KeyTick(
            faraday_core::wallet::KeyBlockOf::Wallet(0),
            1,
        ));
    }
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [2]);
    assert_eq!(app.session.wallets[0].key_checks()[1], KeyCheck::Unchecked);
}

#[test]
fn a_tick_in_another_key_order_reaches_the_record_by_its_key() {
    let mut app = unlocked(vec![
        ("held.kdbx".to_string(), held_file()),
        (
            "zz-wallet.txt".to_string(),
            multisig_in([2, 1, 0]).into_bytes(),
        ),
    ]);
    testkit::load_wallet(&mut app, 1);
    assert_eq!(app.session.wallets.len(), 1);
    // Key 0 is the file's place 2 and the record's place 0.
    app.press(Action::KeyTick(
        faraday_core::wallet::KeyBlockOf::Wallet(0),
        2,
    ));
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [0, 1, 2]);
}

#[test]
fn a_press_on_a_key_that_is_yours_changes_no_record() {
    let mut app = held_with_two_ticked_and_loaded_from_a_file();
    add_key(&mut app, 1);
    assert_eq!(app.session.wallets[0].key_checks()[1], KeyCheck::Yours);
    app.press(Action::KeyTick(
        faraday_core::wallet::KeyBlockOf::Wallet(0),
        1,
    ));
    let app = reloaded_from_the_record(app);
    assert_eq!(checked_keys(&app), [1, 2]);
}
