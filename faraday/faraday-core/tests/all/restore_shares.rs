//! Restoring a wallet from split sheets never combines sheets of two
//! wallets: a key whose xpub or derivation differs between sheets, sheets
//! of different names, and a vault's sheet of another wallet are refused,
//! so the keys in hand are counted and no wallet loads.

use crate::common::{device, type_text};
use faraday_core::restore::{is_share, merge};
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::{Origin, Session};
use faraday_core::wallet_backup::{split_plan, split_share};
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

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
        .add_wallet_confirmed(&c, name, descriptor, "Test wallet", Origin::Named)
        .expect("reads");
    let w = &s.wallets[i];
    let (m, n) = Session::quorum(w);
    let out = split_plan(n, m, 1)
        .iter()
        .map(|row| String::from(&*split_share(w, row).expect("a sheet")))
        .collect();
    (out, w.policy().to_descriptor_checksummed().as_str().into())
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

/// Sheets with no `Format:` line are P2SH, as a whole setup file without
/// one is (F263): merged, they rebuild the bare `sh(sortedmulti(...))`.
#[test]
fn sheets_without_a_format_line_merge_into_a_p2sh_wallet() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let bare: Vec<String> = a[..2]
        .iter()
        .map(|s| {
            let t: String = s
                .lines()
                .filter(|l| !l.starts_with("Format:"))
                .collect::<Vec<_>>()
                .join("\n");
            assert_ne!(&t, s);
            t
        })
        .collect();
    let whole = merge(&bare)
        .expect("merges")
        .whole
        .expect("every key in hand");
    let c = testkit::compute();
    let policy = faraday_bip::multisig_config::parse(&c, &whole).expect("reads");
    assert!(
        policy.to_descriptor().starts_with("sh(sortedmulti(2,"),
        "{}",
        &*policy.to_descriptor()
    );
}

/// A vault file opened by `phrase`, holding `records`.
fn vault_of(phrase: &str, records: Vec<Record>, seed: u8) -> Vec<u8> {
    testkit::kdbx_vault("Vault", phrase, seed, records).unwrap()
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
        ("home.kdbx".into(), vault_of("home phrase", vec![share], 21)),
    ]);
    unlock_on_vaults(&mut app, "home.kdbx", "home phrase");
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
    let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
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
        ("home.kdbx".into(), vault_of("home phrase", vec![share], 22)),
    ]);
    unlock_on_vaults(&mut app, "home.kdbx", "home phrase");
    let wallets = app.session.wallets.len();
    app.press(Action::RestoreWallet);
    let i = app
        .inbox()
        .iter()
        .position(|f| f.name == "savings-share-3-of-3.txt")
        .expect("in the Inbox");
    app.press(Action::RShare(i));
    let c = testkit::compute();
    let texts: Vec<String> = app
        .restore_share_texts(&c)
        .iter()
        .map(|t| t.to_string())
        .collect();
    assert_eq!(texts, vec![a[2].clone()]);
    app.press(Action::RRebuild);
    assert!(app.restore().and_then(|r| r.wallet).is_none(), "a wallet");
    assert_eq!(app.session.wallets.len(), wallets);
}

/// A Taproot multisig's descriptor from the test kits, without its checksum.
fn taproot_descriptor() -> String {
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "taproot-multisig")
        .expect("kit");
    k.descriptor.split('#').next().unwrap().to_string()
}

/// `sheet` with the value of its first key line made by `key`.
fn with_key(sheet: &str, key: impl Fn(&str) -> String) -> String {
    let mut done = false;
    let mut out = String::new();
    for line in sheet.lines() {
        match line.split_once(": ") {
            Some((label, value)) if label.len() == 8 && !done => {
                done = true;
                out.push_str(&format!("{label}: {}\n", key(value)));
            }
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

/// `sheet` with its first `Derivation:` value replaced by `path`.
fn with_path(sheet: &str, path: &str) -> String {
    let mut done = false;
    let mut out = String::new();
    for line in sheet.lines() {
        if line.starts_with("Derivation: ") && !done {
            done = true;
            out.push_str(&format!("Derivation: {path}\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// A key value that is more than one key, or a key with descriptor text
/// after it, and a derivation that is more than a path, make a sheet
/// unreadable: no wallet is built from it, whether the sheets are of a
/// Taproot multisig or of a Coldcard-format one.
#[test]
fn a_sheet_whose_key_or_derivation_holds_more_than_one_is_refused() {
    for descriptor in [taproot_descriptor(), two_of_three(0)] {
        let (good, _) = sheets("Savings", &descriptor);
        assert!(merge(&good[..2]).expect("merges").whole.is_some());
        let other = good[1]
            .lines()
            .find_map(|l| l.split_once(": ").filter(|(k, _)| k.len() == 8))
            .map(|(_, v)| v.to_string())
            .expect("a key line");
        for i in 0..3 {
            let sheet = with_key(&good[0], |v| match i {
                0 => format!("{v}/<0;1>/*,[11223344/48h/0h/0h/2h]{other}"),
                1 => format!("{v}/<0;1>/*"),
                _ => format!("{v},{other}"),
            });
            assert!(merge(&[sheet, good[1].clone()]).is_err());
        }
        for path in [
            "m/48'/1'/0']xpub,[11223344/48'",
            "m/48'/1'/0',[11223344/48'",
            "m/48'/1'/0'/2']",
            other.as_str(),
        ] {
            let sheet = with_path(&good[0], path);
            assert!(merge(&[sheet, good[1].clone()]).is_err(), "{path}");
        }
    }
}

/// A Derivation that is not a plain path (no `/` after `m`, a capital `M`,
/// a doubled `m`, a lone `/`, a sign or leading zeros on a step) makes a
/// sheet no share, so it is not offered for Restore.
#[test]
fn a_sheet_whose_derivation_is_not_a_plain_path_is_not_a_share() {
    for descriptor in [taproot_descriptor(), two_of_three(0)] {
        let (good, _) = sheets("Savings", &descriptor);
        assert!(is_share(&good[0]));
        for path in [
            "m44h/0h",
            "M/44h",
            "m/m/44h",
            "mm/44h",
            "/",
            "m/+48'/00'",
            "m/48'/01'",
            "m/48''",
            "m/48'/",
            "m//48'",
            "m/2147483648",
        ] {
            let sheet = with_path(&good[0], path);
            assert!(!is_share(&sheet), "{path}");
            assert!(merge(&[sheet, good[1].clone()]).is_err(), "{path}");
        }
    }
}

/// Sheets with `m`, `m/`, hardened and unhardened steps and `h` marks
/// still merge into a whole wallet.
#[test]
fn sheets_with_plain_paths_still_restore() {
    for descriptor in [taproot_descriptor(), two_of_three(0)] {
        let (good, _) = sheets("Savings", &descriptor);
        for path in [
            "m",
            "m/",
            "m/48'/1'/0'/2'",
            "m/48h/1h/0h/2h",
            "m/48/1/0/2",
            "48'/0",
        ] {
            let all: Vec<String> = good[..2]
                .iter()
                .map(|s| {
                    s.lines()
                        .map(|l| match l.starts_with("Derivation: ") {
                            true => format!("Derivation: {path}\n"),
                            false => format!("{l}\n"),
                        })
                        .collect()
                })
                .collect();
            assert!(all.iter().all(|s| is_share(s)), "{path}");
            let whole = merge(&all).map(|m| m.whole.is_some());
            assert_eq!(whole.ok(), Some(true), "{path}");
        }
    }
}

/// A named Taproot multisig rebuilt from two of its sheets keeps its name,
/// whether the sheets are restored from Files or picked in Restore.
#[test]
fn a_taproot_multisig_rebuilt_from_its_sheets_keeps_its_name() {
    let (sheets, _) = sheets("Family trust", &taproot_descriptor());
    let files = || {
        vec![
            (
                "trust-share-1.txt".to_string(),
                sheets[0].clone().into_bytes(),
            ),
            (
                "trust-share-2.txt".to_string(),
                sheets[1].clone().into_bytes(),
            ),
        ]
    };

    let mut app = device(files());
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    app.press(Action::RestoreShares);
    testkit::confirm_wallets(&mut app);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    assert_eq!(&*app.session.wallets[0].name, "Family trust");

    let mut app = device(files());
    app.press(Action::RestoreWallet);
    for name in ["trust-share-1.txt", "trust-share-2.txt"] {
        let i = app
            .inbox()
            .iter()
            .position(|f| f.name == name)
            .expect("in the Inbox");
        app.press(Action::RShare(i));
    }
    app.press(Action::RRebuild);
    testkit::confirm_wallets(&mut app);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(&*app.session.wallets[0].name, "Family trust");
}

/// A named Taproot multisig's sheets listed in the Inbox show the wallet
/// under the sheets' name.
#[test]
fn a_taproot_multisig_from_its_sheets_is_listed_in_the_inbox_by_name() {
    let (sheets, _) = sheets("Fund", &taproot_descriptor());
    let app = device(vec![
        ("fund-1.txt".to_string(), sheets[0].clone().into_bytes()),
        ("fund-2.txt".to_string(), sheets[1].clone().into_bytes()),
    ]);
    let c = testkit::compute();
    let found = app.inbox_found_now(&c);
    let rebuilt: Vec<_> = found.wallets.iter().filter(|w| w.from_shares).collect();
    assert_eq!(rebuilt.len(), 1);
    assert_eq!(&*rebuilt[0].name, "Fund");
}

/// The same sheets brought into Spend from a backup open the wallet as
/// "Fund".
#[test]
fn a_taproot_multisig_from_its_sheets_keeps_its_name_in_spend_from_a_backup() {
    use faraday_core::spend_backup::{SpendBackupAction as F, page};
    let (sheets, _) = sheets("Fund", &taproot_descriptor());
    let mut app = device(Vec::new());
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::Next(page::MAP)));
    for s in &sheets[..2] {
        if app.scan().is_none() {
            app.press(Action::SpendBackup(F::Scan));
        }
        app.event(Event::Scanned {
            bytes: s.clone().into_bytes(),
        });
        testkit::settle(&mut app);
        let _ = app.frame();
    }
    testkit::confirm_wallets(&mut app);
    assert_eq!(app.session.wallets.len(), 1, "{:?}", app.toast_text());
    assert_eq!(&*app.session.wallets[0].name, "Fund");
}
