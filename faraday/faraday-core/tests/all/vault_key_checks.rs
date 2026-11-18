//! A wallet of several keys saved into a vault keeps the keys the person
//! ticked as checked, and comes back from the vault with them; a wallet a
//! vault holds with no ticks comes back with every key not its own not
//! checked.

use crate::common::{Shell, process, type_text};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::KeyCheck;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
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
