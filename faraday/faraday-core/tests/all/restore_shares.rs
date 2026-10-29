//! Restoring a wallet from split sheets never combines sheets of two
//! wallets: a key whose xpub or derivation differs between sheets, sheets
//! of different names, and a vault's sheet of another wallet are refused,
//! so the keys in hand are counted and no wallet loads.

use crate::common::{device, type_text};
use faraday_core::restore::merge;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::{Origin, Session};
use faraday_core::wallet_backup::{split_plan, split_share};
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};

/// The 2-of-3 over the three test seeds at BIP-48 account `account`.
fn two_of_three(account: u32) -> String {
    let p = format!("m/48'/1'/{account}'/2'");
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, &p),
        testkit::key(1, &p),
        testkit::key(2, &p)
    )
}

/// The split sheets of the wallet `descriptor` describes, named `name`,
/// one key left off each, with its description as Faraday keeps it.
fn sheets(name: &str, descriptor: &str) -> (Vec<String>, String) {
    let c = testkit::compute();
    let mut s = testkit::session();
    let i = s
        .add_wallet(&c, name, descriptor, "Test wallet", Origin::Named)
        .expect("reads");
    let w = &s.wallets[i];
    let (m, n) = Session::quorum(w);
    let out = split_plan(n, m, 1)
        .iter()
        .map(|row| split_share(w, row).expect("a sheet"))
        .collect();
    (out, w.policy().to_descriptor_checksummed())
}

/// Two wallets over the same seeds at two accounts: a sheet of each does
/// not merge, though their fingerprints are the same.
#[test]
fn sheets_whose_keys_differ_under_one_fingerprint_do_not_merge() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let (b, _) = sheets("Savings", &two_of_three(1));
    assert!(merge(&[a[0].clone(), a[1].clone()]).is_ok());
    assert!(
        merge(&[a[0].clone(), b[1].clone()]).is_err(),
        "a wallet of both"
    );
}

/// Sheets of one wallet that carry different names do not merge.
#[test]
fn sheets_of_different_names_do_not_merge() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let renamed = a[1].replace("Name: Savings", "Name: Holiday");
    assert_ne!(renamed, a[1]);
    assert!(merge(&[a[0].clone(), renamed]).is_err());
}

/// A vault file whose one slot, opened by `phrase`, holds `records`.
fn vault_of(phrase: &str, records: Vec<Record>, seed: u8) -> Vec<u8> {
    let c = testkit::compute();
    let mut all = vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Vault")];
    all.extend(records);
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        Cost {
            memory_kib: faraday_vault::MIN_MEMORY_KIB,
            passes: 1,
            lanes: 1,
        },
        &[phrase.as_bytes()],
        &[Contents { records: all }],
        &[seed; 32],
    )
    .unwrap()
}

/// Unlocks vault file `name` from the Vaults tab with `phrase`, loading
/// nothing on its Load sheet.
fn unlock_on_vaults(app: &mut Faraday, name: &str, phrase: &str) {
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("the vault is listed");
    let before = app.vaults().open.len();
    app.press(Action::Vault(V::Open(n)));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), before + 1);
    if app.sheet == Some(faraday_core::Sheet::UnlockLoad) {
        app.press(Action::Vault(V::LoadNotNow));
    }
}

/// One sheet of wallet A in hand, and in an open vault a sheet of wallet
/// B (the same seeds at another account, the same name and quorum): the
/// vault's sheet is not taken, and no wallet loads.
#[test]
fn a_sheet_in_hand_and_a_vaults_sheet_of_another_wallet_load_no_wallet() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let (b, b_desc) = sheets("Savings", &two_of_three(1));
    let share = Record::new(kind::WALLET_SHARE)
        .with(field::SHARE, b[1].as_bytes())
        .with(field::SHARE_WALLET, b_desc.as_bytes())
        .with(field::SHARE_WALLET_NAME, b"Savings")
        .with(field::SHARE_NUMBER, &[2])
        .with(field::SHARE_OF, &[3]);
    let mut app = device(vec![
        ("savings-share-1-of-3.txt".into(), a[0].clone().into_bytes()),
        ("home.fvlt".into(), vault_of("home phrase", vec![share], 21)),
    ]);
    unlock_on_vaults(&mut app, "home.fvlt", "home phrase");
    let wallets = app.session.wallets.len();
    app.press(Action::RestoreWallet);
    let i = app
        .inbox()
        .iter()
        .position(|f| f.name == "savings-share-1-of-3.txt")
        .expect("in the Inbox");
    app.press(Action::RShare(i));
    let c = testkit::compute();
    let texts = app.restore_share_texts(&c);
    assert_eq!(texts, vec![a[0].clone()], "the vault's sheet taken");
    app.press(Action::RRebuild);
    assert!(app.restore().and_then(|r| r.wallet).is_none(), "a wallet");
    assert_eq!(app.session.wallets.len(), wallets);
}

/// One sheet of wallet A in hand (keys 1 and 3), and in an open vault a
/// sheet of wallet B (keys 1 and 2 of A's, and another seed's), of the
/// same name and quorum: the vault's sheet is of another wallet than the
/// one in hand describes, so it is not taken, and no wallet loads.
#[test]
fn a_vaults_sheet_of_a_wallet_without_the_keys_in_hand_is_not_taken() {
    let a_desc = two_of_three(0);
    let p = "m/48'/1'/0'/2'";
    let other = testkit::three_of_five();
    let other = other
        .split_once("sortedmulti(3,")
        .and_then(|(_, rest)| rest.split(',').next())
        .expect("a key")
        .to_string();
    let b_text = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{other}))",
        testkit::key(0, p),
        testkit::key(1, p),
    );
    let (a, _) = sheets("Savings", &a_desc);
    let (b, b_desc) = sheets("Savings", &b_text);
    let share = Record::new(kind::WALLET_SHARE)
        .with(field::SHARE, b[0].as_bytes())
        .with(field::SHARE_WALLET, b_desc.as_bytes())
        .with(field::SHARE_WALLET_NAME, b"Savings")
        .with(field::SHARE_NUMBER, &[1])
        .with(field::SHARE_OF, &[3]);
    let mut app = device(vec![
        ("savings-share-3-of-3.txt".into(), a[2].clone().into_bytes()),
        ("home.fvlt".into(), vault_of("home phrase", vec![share], 22)),
    ]);
    unlock_on_vaults(&mut app, "home.fvlt", "home phrase");
    let wallets = app.session.wallets.len();
    app.press(Action::RestoreWallet);
    let i = app
        .inbox()
        .iter()
        .position(|f| f.name == "savings-share-3-of-3.txt")
        .expect("in the Inbox");
    app.press(Action::RShare(i));
    let c = testkit::compute();
    assert_eq!(app.restore_share_texts(&c), vec![a[2].clone()]);
    app.press(Action::RRebuild);
    assert!(app.restore().and_then(|r| r.wallet).is_none(), "a wallet");
    assert_eq!(app.session.wallets.len(), wallets);
}
