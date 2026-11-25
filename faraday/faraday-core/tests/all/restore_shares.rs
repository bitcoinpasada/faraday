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

/// A wallet named with a `#` is split into sheets and restored: the name
/// that comes back is the one on the sheets, read as the setup file the
/// merged sheets make is read, and it is not cut at the `#` nor lost.
#[test]
fn a_wallet_name_with_a_hash_survives_split_sheets() {
    let c = testkit::compute();
    for name in ["Fund #2", "#2", "A#B"] {
        let (a, _) = sheets(name, &two_of_three(0));
        let whole = merge(&a[..2]).expect("merges").whole.expect("whole");
        let (_, back) = faraday_bip::multisig_config::parse_named(&c, &whole).expect("reads");
        let back = back.expect("the name is there");
        assert!(
            !back.contains('#') && back.trim() == back.as_str().trim(),
            "{name}: {}",
            &*back
        );
        assert_eq!(
            &*back,
            name.replace('#', "").as_str(),
            "{name}: the name with the `#` left out"
        );
    }
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

/// A signed message whose body is a split sheet stays a signed message:
/// a sheet holds public keys only, and the file is not taken for a sheet
/// to restore from.
#[test]
fn a_signed_message_holding_a_sheet_stays_a_message() {
    const SIGNATURE: &str =
        "H9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=";
    let c = testkit::compute();
    let (good, _) = sheets("Savings", &two_of_three(0));
    assert!(is_share(&good[0]));
    let signed = format!(
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu\n{SIGNATURE}\n{}\n",
        good[0]
    );
    assert_eq!(
        faraday_core::wallet::classify(&c, "message.txt", signed.as_bytes()),
        faraday_core::wallet::FileKind::Message
    );
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

/// Files' Restore button with the sheets of two wallets, or one sheet of
/// three, opens Restore on its Description card instead of refusing or
/// toasting: the sheets of one wallet are ticked and the wallet waits for
/// the rest.
#[test]
fn files_restore_button_opens_restore_with_one_wallets_sheets() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let (b, _) = sheets("Holiday", &two_of_three(1));
    let mut app = device(vec![
        ("savings-share-1.txt".to_string(), a[0].clone().into_bytes()),
        ("holiday-share-1.txt".to_string(), b[0].clone().into_bytes()),
        ("holiday-share-2.txt".to_string(), b[1].clone().into_bytes()),
    ]);
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    assert!(app.offers(Action::RestoreShares));
    app.press(Action::RestoreShares);
    assert_eq!(app.screen, Screen::Restore);
    assert!(app.toast_text().is_none(), "{:?}", app.toast_text());
}

/// A vault record of `sheet`, kept for the wallet `desc` describes.
fn share_record(sheet: &str, desc: &str, number: u8) -> Record {
    Record::new(kind::WALLET_SHARE)
        .with(field::SHARE, sheet.as_bytes())
        .with(field::SHARE_WALLET, desc.as_bytes())
        .with(field::SHARE_WALLET_NAME, b"Twins")
        .with(field::SHARE_NUMBER, &[number])
        .with(field::SHARE_OF, &[3])
}

/// Restore open on a device whose open vault holds `records` and whose
/// Inbox holds the sheets `files`, each ticked; what the pool adds up to.
fn pool_of(
    records: Vec<Record>,
    files: &[&str],
) -> (Faraday, Result<faraday_core::restore::Merged, String>) {
    let mut inbox: Vec<(String, Vec<u8>)> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (format!("twins-share-{i}.txt"), f.as_bytes().to_vec()))
        .collect();
    inbox.push(("home.kdbx".into(), vault_of("home phrase", records, 22)));
    let mut app = device(inbox);
    unlock_on_vaults(&mut app, "home.kdbx", "home phrase");
    app.press(Action::RestoreWallet);
    for i in 0..files.len() {
        let at = app
            .inbox()
            .iter()
            .position(|f| f.name == format!("twins-share-{i}.txt").as_str())
            .expect("in the Inbox");
        app.press(Action::RShare(at));
    }
    let c = testkit::compute();
    let merged = app.restore_merged(&c).map_err(|e| e.to_string());
    (app, merged)
}

/// A twin's record of a text the first wallet's keys hold comes first in
/// the vault, the first's own Share 2 after it, nothing in hand: the
/// first wallet's three keys are in hand, as before the twin's record.
#[test]
fn a_twins_record_first_in_the_vault_does_not_hide_the_wallets_own_shares() {
    let (first, first_d) = sheets("Twins", testkit::WALLET_ID_FIRST);
    let (_, alias_d) = sheets("Twins", testkit::WALLET_ID_ALIAS);
    let (_, m) = pool_of(
        vec![
            share_record(&first[0], &alias_d, 1),
            share_record(&first[1], &first_d, 2),
        ],
        &[],
    );
    let m = m.expect("merges");
    assert_eq!((m.have.len(), m.n), (3, 3));
    assert!(m.whole.is_some());
}

/// The first wallet's Share 1 in hand too: the twin's record is then a
/// duplicate of it, and the first still rebuilds.
#[test]
fn a_twins_record_of_a_share_in_hand_does_not_hide_the_wallets_own_shares() {
    let (first, first_d) = sheets("Twins", testkit::WALLET_ID_FIRST);
    let (_, alias_d) = sheets("Twins", testkit::WALLET_ID_ALIAS);
    let (_, m) = pool_of(
        vec![
            share_record(&first[0], &alias_d, 1),
            share_record(&first[1], &first_d, 2),
        ],
        &[&first[0]],
    );
    let m = m.expect("merges");
    assert_eq!((m.have.len(), m.n), (3, 3));
    assert!(m.whole.is_some());
}

/// A crafted twin whose own Share 2 completes it beside the first's
/// shares, which complete the first: neither loads, and the screen says
/// two wallets rebuild.
#[test]
fn two_wallets_of_one_checksum_that_both_complete_load_neither() {
    let (first, first_d) = sheets("Twins", testkit::WALLET_ID_FIRST);
    let (alias, alias_d) = sheets("Twins", testkit::WALLET_ID_ALIAS);
    let (mut app, m) = pool_of(
        vec![
            share_record(&first[0], &first_d, 1),
            share_record(&first[1], &first_d, 2),
            share_record(&alias[1], &alias_d, 2),
        ],
        &[],
    );
    let wallets = app.session.wallets.len();
    let said = m.err().expect("refused");
    assert!(said.contains("Twins (") && said.contains("neither is rebuilt"));
    assert!(!said.contains("delete") && !said.contains("out of the vault"));
    app.press(Action::RRebuild);
    testkit::confirm_wallets(&mut app);
    assert!(app.restore().and_then(|r| r.wallet).is_none());
    assert_eq!(app.session.wallets.len(), wallets);
}

/// `desc` with its third key's origin path changed (the fingerprint and
/// the xpub kept): the same wallet but for one derivation.
fn third_key_elsewhere(desc: &str) -> String {
    let at = desc.rfind('[').expect("a third key");
    let (head, tail) = desc.split_at(at);
    let tail = tail
        .replacen("/0'/2']", "/7'/2']", 1)
        .replacen("/0h/2h]", "/7h/2h]", 1);
    assert_ne!(tail, &desc[at..]);
    format!("{head}{tail}")
}

/// The same text as `sheet` with its last key's derivation changed.
fn last_derivation_elsewhere(sheet: &str) -> String {
    let at = sheet.rfind("Derivation: ").expect("a key");
    let end = at + sheet[at..].find('\n').expect("a line");
    format!(
        "{}Derivation: m/48'/1'/7'/2'{}",
        &sheet[..at],
        &sheet[end..]
    )
}

/// A wallet's description only the 2-of-3 over account 0.
fn account_zero() -> (Vec<String>, String) {
    sheets("Twins", &two_of_three(0))
}

/// Whether the pool's whole is the account-0 wallet's: key 3 at its own
/// derivation.
fn is_account_zero(m: &faraday_core::restore::Merged) -> bool {
    let whole = m.whole.as_deref().expect("whole");
    !whole.contains("/7'/2'") && whole.contains("/0'/2'")
}

/// A twin that differs from the wallet in one key's derivation holds two
/// of its three keys only: its record of the wallet's Share 1 does not
/// complete it with the wallet's Share 2.
#[test]
fn a_derivation_twins_record_does_not_borrow_the_wallets_shares() {
    let (w, w_d) = account_zero();
    let (_, t_d) = sheets("Twins", &third_key_elsewhere(&two_of_three(0)));
    let (_, m) = pool_of(
        vec![
            share_record(&w[0], &w_d, 1),
            share_record(&w[1], &w_d, 2),
            share_record(&w[0], &t_d, 1),
        ],
        &[],
    );
    let m = m.expect("one wallet");
    assert_eq!((m.have.len(), m.n), (3, 3));
    assert!(is_account_zero(&m));
}

/// One wallet written in two key orders is one wallet: a record of each
/// and a sheet in hand still rebuild it.
#[test]
fn one_wallet_in_two_key_orders_is_not_two_wallets() {
    let (w, w_d) = account_zero();
    let p = "m/48'/1'/0'/2'";
    let reordered = format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(1, p),
        testkit::key(2, p),
        testkit::key(0, p)
    );
    let (_, r_d) = sheets("Twins", &reordered);
    let (_, m) = pool_of(
        vec![share_record(&w[0], &w_d, 1), share_record(&w[1], &r_d, 2)],
        &[&w[2]],
    );
    let m = m.expect("one wallet");
    assert!(is_account_zero(&m));
}

/// Shares ticked in Files that rebuild a wallet alone are taken whatever
/// the vault holds, a 3-of-3 over the same keys included.
#[test]
fn shares_in_hand_that_complete_alone_ignore_the_vaults_wallets() {
    let (w, w_d) = account_zero();
    let p = "m/48'/1'/0'/2'";
    let three = format!(
        "wsh(sortedmulti(3,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
        testkit::key(0, p),
        testkit::key(1, p),
        testkit::key(2, p)
    );
    let (_, three_d) = sheets("Twins", &three);
    let (_, m) = pool_of(
        vec![
            share_record(&w[2], &w_d, 3),
            share_record(&w[2], &three_d, 3),
        ],
        &[&w[0], &w[1]],
    );
    let m = m.expect("the hand");
    assert!(is_account_zero(&m));
}

/// A person with two wallets of different checksums in the vaults is
/// never refused: the first wallet's shares rebuild it.
#[test]
fn two_wallets_of_different_checksums_rebuild_the_first() {
    let (w, w_d) = account_zero();
    let (v, v_d) = sheets("Other", &two_of_three(1));
    let (_, m) = pool_of(
        vec![
            share_record(&w[0], &w_d, 1),
            share_record(&w[1], &w_d, 2),
            share_record(&v[0], &v_d, 1),
            share_record(&v[1], &v_d, 2),
        ],
        &[],
    );
    let m = m.expect("not refused");
    assert!(is_account_zero(&m));
}

/// A record whose wallet does not read is not taken: its changed share
/// does not stand in for the honest one.
#[test]
fn a_record_whose_wallet_does_not_read_is_not_counted() {
    let (w, w_d) = account_zero();
    let bare = Record::new(kind::WALLET_SHARE)
        .with(field::SHARE, last_derivation_elsewhere(&w[1]).as_bytes())
        .with(field::SHARE_NUMBER, &[2])
        .with(field::SHARE_OF, &[3]);
    let (_, m) = pool_of(
        vec![
            share_record(&w[0], &w_d, 1),
            bare,
            share_record(&w[1], &w_d, 2),
        ],
        &[],
    );
    let m = m.expect("merges");
    assert_eq!((m.have.len(), m.n), (3, 3));
    assert!(is_account_zero(&m));
}

/// A twin's record of the wallet's Share 1 under another name comes
/// first: the records that name the wallet are taken before it.
#[test]
fn records_naming_the_wallet_come_before_a_twins() {
    let (first, first_d) = sheets("Twins", testkit::WALLET_ID_FIRST);
    let (_, alias_d) = sheets("Twins", testkit::WALLET_ID_ALIAS);
    let renamed = first[0].replacen("Name: Twins", "Name: x", 1);
    assert_ne!(renamed, first[0]);
    let (_, m) = pool_of(
        vec![
            share_record(&renamed, &alias_d, 1),
            share_record(&first[0], &first_d, 1),
            share_record(&first[1], &first_d, 2),
        ],
        &[],
    );
    let m = m.expect("merges");
    assert_eq!((m.have.len(), m.n), (3, 3));
    assert!(m.whole.is_some());
}

/// What the Restore screen says now, drawn.
fn screen_says(app: &mut Faraday) -> String {
    app.drawn_texts()
        .iter()
        .map(|t| t.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Restore open on a device whose Inbox holds `files` (name, text), the
/// vaults `vaults` (file name, phrase, seed, records) open.
fn restore_with(files: &[(&str, &str)], vaults: Vec<(&str, &str, u8, Vec<Record>)>) -> Faraday {
    let mut inbox: Vec<(String, Vec<u8>)> = files
        .iter()
        .map(|(n, t)| (n.to_string(), t.as_bytes().to_vec()))
        .collect();
    let mut opens = Vec::new();
    for (n, phrase, seed, records) in vaults {
        inbox.push((n.to_string(), vault_of(phrase, records, seed)));
        opens.push((n, phrase));
    }
    let mut app = device(inbox);
    for (n, phrase) in opens {
        unlock_on_vaults(&mut app, n, phrase);
    }
    app.press(Action::RestoreWallet);
    app.press(Action::RStep(faraday_core::rstep::DESCRIPTION));
    app
}

fn tick(app: &mut Faraday, name: &str) {
    let at = app
        .inbox()
        .iter()
        .position(|f| f.name == name)
        .expect("in the Inbox");
    app.press(Action::RShare(at));
}

/// A ticked share file replaced by one of the same name and length: the
/// answer is the new file's.
#[test]
fn a_ticked_share_file_replaced_by_one_of_the_same_size_changes_the_answer() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let (b, _) = sheets("Savings", &two_of_three(1));
    assert_eq!(a[0].len(), b[0].len());
    let mut app = restore_with(&[("s-share.txt", &a[0])], vec![]);
    tick(&mut app, "s-share.txt");
    let c = testkit::compute();
    let before = app.restore_share_texts(&c);
    assert_eq!(before.len(), 1);
    testkit::inbox_overwrite_bytes(&mut app, 0, b[0].as_bytes());
    let after = app.restore_share_texts(&c);
    assert_eq!(after.len(), 1);
    assert_ne!(before[0].as_str(), after[0].as_str());
    assert_eq!(after[0].as_str(), b[0].as_str());
}

/// A lock empties what Restore kept.
#[test]
fn a_lock_drops_the_pool_restore_kept() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let mut app = restore_with(&[("s-share.txt", &a[0])], vec![]);
    tick(&mut app, "s-share.txt");
    let c = testkit::compute();
    let _ = app.restore_share_texts(&c);
    assert!(testkit::restore_pool_kept(&app));
    app.press(Action::Lock);
    assert!(!testkit::restore_pool_kept(&app));
}

/// A file of the name of a ticked share, now holding a seed beside the
/// share, is not read: the pool holds none of it.
#[test]
fn a_secret_file_under_a_ticked_name_is_not_read() {
    let (a, _) = sheets("Savings", &two_of_three(0));
    let mut app = restore_with(&[("s-share.txt", &a[0])], vec![]);
    tick(&mut app, "s-share.txt");
    let c = testkit::compute();
    assert_eq!(app.restore_share_texts(&c).len(), 1);
    testkit::inbox_retain(&mut app, |i| i.name != "s-share.txt");
    let secret = format!(
        "{}\n# abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n",
        a[0]
    );
    testkit::inbox_file(&mut app, "s-share.txt", secret.into_bytes());
    assert_eq!(
        app.inbox().last().map(|i| i.kind),
        Some(faraday_core::wallet::FileKind::SecretWallet)
    );
    assert!(app.restore_share_texts(&c).is_empty());
}

/// Two vaults, each alone rebuilding one of two wallets of one name and
/// checksum: the screen says so, and that one vault ends it, with no
/// "Or" before it.
#[test]
fn the_vault_sentence_is_said_alone_on_the_drawn_screen() {
    let (first, first_d) = sheets("Twins", testkit::WALLET_ID_FIRST);
    let (alias, alias_d) = sheets("Twins", testkit::WALLET_ID_ALIAS);
    let mut app = restore_with(
        &[("x-share.txt", &first[0])],
        vec![
            (
                "home.kdbx",
                "home phrase",
                22,
                vec![
                    share_record(&first[0], &first_d, 1),
                    share_record(&first[1], &first_d, 2),
                ],
            ),
            (
                "work.kdbx",
                "work phrase",
                23,
                vec![share_record(&alias[1], &alias_d, 2)],
            ),
        ],
    );
    let _ = app.frame();
    let said = screen_says(&mut app);
    assert!(said.contains("rebuild both Twins ("), "{said}");
    assert!(
        said.contains("Open only the vault of one of them."),
        "{said}"
    );
    assert!(!said.contains("Or open only"), "{said}");
}

/// A share file only one of the two holds, ticked: the tick sentence is
/// said, and the vault sentence after it begins with "Or" where it is
/// said too. The files tried are the share files listed.
#[test]
fn the_tick_sentence_is_said_on_the_drawn_screen_for_a_listed_share_file() {
    let (first, first_d) = sheets("Twins", testkit::WALLET_ID_FIRST);
    let (alias, alias_d) = sheets("Twins", testkit::WALLET_ID_ALIAS);
    let mut seen = Vec::new();
    let mut any = false;
    for (k, extra) in first.iter().chain(alias.iter()).enumerate() {
        let mut app = restore_with(
            &[("x-share.txt", extra)],
            vec![(
                "home.kdbx",
                "home phrase",
                22,
                vec![
                    share_record(&first[0], &first_d, 1),
                    share_record(&first[1], &first_d, 2),
                    share_record(&alias[1], &alias_d, 2),
                ],
            )],
        );
        let _ = app.frame();
        let said = screen_says(&mut app);
        any |= said.contains("Tick a share file that only one of them holds.");
        seen.push(format!("{k}: {said}"));
    }
    assert!(any, "{seen:#?}");
}
