//! Read a Bitcoin Core wallet in Tools (`docs/decisions/F229.md`): a
//! wallet.dat from Files is read whole, its keys checked or, for an
//! encrypted wallet with no passphrase, said not to be; a wrong
//! passphrase, a Berkeley DB file and a journal beside the file are named;
//! the descriptors go to Wallets as Core lists them.

use crate::common::{has, has_part, offered, texts, type_text};
use faraday_bip::keys::Network;
use faraday_core::core_wallet::tool::CoreWalletAction as W;
use faraday_core::wallet::Session;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, Event};

const V: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tools/vectors/core-wallet"
);

fn vector(name: &str) -> Vec<u8> {
    std::fs::read(format!("{V}/{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The tool open with these files in Files and the first one chosen.
fn opened(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.session = Session::on(Network::Testnet);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Read a Bitcoin Core wallet")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::CoreWallet);
    app
}

/// Whether a line of the screen says this many private keys were checked.
fn keys_checked(app: &mut Faraday) -> bool {
    texts(app).iter().any(|t| {
        t.strip_suffix(" checked")
            .is_some_and(|n| n.parse::<u32>().is_ok_and(|n| n > 0))
    })
}

fn read_with(app: &mut Faraday, passphrase: &str) {
    if !passphrase.is_empty() {
        app.press(Action::CoreWallet(W::Focus));
        type_text(app, passphrase);
    }
    app.press(Action::CoreWallet(W::Read));
    app.event(Event::Tick { now_ms: 1 });
}

#[test]
fn a_plain_wallet_is_read_whole_with_its_keys_checked() {
    let mut app = opened(vec![("wallet.dat", vector("plain.dat"))]);
    read_with(&mut app, "");
    assert!(has(&mut app, "The wallet is whole"));
    assert!(keys_checked(&mut app));
    assert!(has(&mut app, "None"));
}

#[test]
fn an_encrypted_wallet_without_its_passphrase_is_whole_and_says_its_keys_were_not_checked() {
    let mut app = opened(vec![("wallet.dat", vector("encrypted.dat"))]);
    read_with(&mut app, "");
    assert!(has(&mut app, "The wallet is whole"));
    assert!(has_part(&mut app, "not checked, no passphrase given"));
    assert!(!keys_checked(&mut app));
}

#[test]
fn an_encrypted_wallet_with_its_passphrase_has_its_keys_checked_and_the_passphrase_is_gone() {
    let mut app = opened(vec![("wallet.dat", vector("encrypted.dat"))]);
    read_with(&mut app, "faraday");
    assert!(has(&mut app, "The wallet is whole"));
    assert!(keys_checked(&mut app));
    assert!(!has_part(&mut app, "no passphrase given"));
    // A second read has no passphrase left to use.
    app.press(Action::CoreWallet(W::Step(0)));
    read_with(&mut app, "");
    assert!(has_part(&mut app, "not checked, no passphrase given"));
}

#[test]
fn an_encrypted_wallet_is_read_on_the_worker_and_the_result_shows_when_it_ends() {
    let mut app = opened(vec![("wallet.dat", vector("encrypted.dat"))]);
    app.press(Action::CoreWallet(W::Focus));
    type_text(&mut app, "faraday");
    testkit::press_on_worker(&mut app, Action::CoreWallet(W::Read));
    assert!(app.busy().is_some());
    assert!(!has(&mut app, "The wallet is whole"));
    testkit::settle(&mut app);
    assert!(app.busy().is_none());
    assert!(has(&mut app, "The wallet is whole"));
    assert!(keys_checked(&mut app));
}

#[test]
fn a_wrong_passphrase_is_said_so() {
    let mut app = opened(vec![("wallet.dat", vector("encrypted.dat"))]);
    read_with(&mut app, "not it");
    assert!(has(&mut app, "The passphrase does not open the wallet"));
    assert!(has_part(&mut app, "not checked, the passphrase is wrong"));
    assert!(!has(&mut app, "The wallet is whole"));
}

#[test]
fn a_berkeley_db_wallet_is_refused_with_the_migratewallet_line() {
    let mut legacy = vec![0u8; 4096];
    legacy[12..16].copy_from_slice(&[0x62, 0x31, 0x05, 0x00]);
    let mut app = opened(vec![("wallet.dat", legacy)]);
    read_with(&mut app, "");
    assert!(has_part(&mut app, "legacy Berkeley DB wallet"));
    assert!(has_part(&mut app, "migratewallet"));
    assert!(!has(&mut app, "The wallet is whole"));
}

#[test]
fn a_journal_beside_the_file_refuses_it() {
    let mut app = opened(vec![
        ("wallet.dat", vector("plain.dat")),
        ("wallet.dat-journal", vec![0; 512]),
    ]);
    read_with(&mut app, "");
    assert!(has_part(
        &mut app,
        "wallet.dat-journal is beside this wallet"
    ));
    assert!(!has(&mut app, "The wallet is whole"));
}

#[test]
fn the_descriptors_with_their_origins_go_to_wallets() {
    let mut app = opened(vec![("watch-only-wallet.dat", vector("watchonly.dat"))]);
    read_with(&mut app, "");
    assert!(has(&mut app, "The wallet is whole"));
    app.press(Action::CoreWallet(W::Take(0)));
    let seen = texts(&mut app);
    assert_eq!(app.screen, Screen::Wallets, "{seen:?}");
    assert!(has_part(&mut app, "Watch Only"));
}

/// The receive and change descriptors of this kind Core lists for a
/// sample, as the one wallet text they make: `listdescriptors` with the
/// origins and account keys, checksums cut, the two chains as `<0;1>`.
fn core_lists(json: &str, kind: &str) -> String {
    let v: serde_json::Value = serde_json::from_slice(&vector(json)).unwrap();
    let list = &v["listdescriptors_public"]["descriptors"];
    let desc = |internal: bool| {
        list.as_array()
            .unwrap()
            .iter()
            .filter(|d| d["internal"] == internal)
            .map(|d| d["desc"].as_str().unwrap())
            .find(|d| d.starts_with(kind))
            .unwrap()
            .split('#')
            .next()
            .unwrap()
            .to_string()
    };
    let (receive, change) = (desc(false), desc(true));
    assert_eq!(receive.replace("/0/*", "/1/*"), change);
    receive.replace("/0/*", "/<0;1>/*")
}

/// Where Native SegWit and Taproot stand among the pairs offered, which go
/// Legacy, Nested, Native, Taproot.
const NATIVE: usize = 2;
const TAPROOT: usize = 3;

#[test]
fn taking_a_pair_of_a_wallet_bitcoin_core_made_adds_that_pair_with_the_origins_core_lists() {
    for (file, json, passphrase) in [
        ("plain.dat", "plain.json", ""),
        ("encrypted.dat", "encrypted.json", ""),
        ("encrypted.dat", "encrypted.json", "faraday"),
    ] {
        for (place, kind) in [(NATIVE, "wpkh("), (TAPROOT, "tr(")] {
            let mut app = opened(vec![("wallet.dat", vector(file))]);
            read_with(&mut app, passphrase);
            assert!(has(&mut app, "The wallet is whole"), "{file}");
            app.press(Action::CoreWallet(W::Take(place)));
            let seen = texts(&mut app);
            assert_eq!(app.screen, Screen::Wallets, "{file} {passphrase}: {seen:?}");
            assert_eq!(app.session.wallets.len(), 1, "{file} {passphrase}");
            let added = app.session.wallets[0].same();
            assert_eq!(
                added.split('#').next().unwrap(),
                core_lists(json, kind),
                "{file} {passphrase} {kind}"
            );
        }
    }
}

type Records = Vec<(Vec<u8>, Vec<u8>)>;

/// The application id and the records of a sample.
fn load(n: &str) -> (u32, Records) {
    let v: serde_json::Value = serde_json::from_slice(&vector(&format!("{n}.json"))).unwrap();
    let hex = |x: &serde_json::Value| {
        let s = x.as_str().unwrap();
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect::<Vec<u8>>()
    };
    let records = v["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (hex(&r[0]), hex(&r[1])))
        .collect();
    (v["application_id"].as_u64().unwrap() as u32, records)
}

/// The descriptor records (not their keys or caches) of a sample.
fn descriptor_records(other: &str) -> Records {
    load(other)
        .1
        .into_iter()
        .filter(|(k, _)| {
            // Each record is `<len><name><data>`: take the descriptor ones.
            k.get(1..).is_some_and(|n| {
                n.starts_with(b"walletdescriptor")
                    && !n.starts_with(b"walletdescriptorkey")
                    && !n.starts_with(b"walletdescriptorckey")
            })
        })
        .collect()
}

fn read_records(id: u32, records: &Records) -> faraday_core::core_wallet::Wallet {
    let c = faraday_core::testkit::compute();
    faraday_core::core_wallet::read(&c, id, records, Network::Regtest, None).unwrap()
}

/// A wallet's records with the descriptor records of another wallet added:
/// the other wallet's descriptors are in the file and active in no slot.
fn with_imported(name: &str, other: &str) -> faraday_core::core_wallet::Wallet {
    let (id, mut records) = load(name);
    records.extend(descriptor_records(other));
    read_records(id, &records)
}

#[test]
fn a_wallet_with_an_imported_multisig_offers_its_own_active_pairs_and_not_the_multisig() {
    let w = with_imported("plain", "multisig");
    assert!(
        w.descriptors.iter().any(|d| d.listed.starts_with("wsh(")),
        "the import is in the wallet"
    );
    let pairs = faraday_core::core_wallet::tool::active_pairs(&w);
    let labels: Vec<&str> = pairs.iter().map(|p| p.label).collect();
    assert_eq!(
        labels,
        [
            "Legacy (pkh)",
            "Nested SegWit (sh-wpkh)",
            "Native SegWit (wpkh)",
            "Taproot (tr)"
        ]
    );
    // Each pair sends its receive and its change descriptor: intake makes
    // `<0;1>` from a receive one alone, so only the pair's texts show a
    // dropped change.
    assert!(pairs.iter().all(|p| p.texts.len() == 2));
    let native = &pairs[NATIVE];
    assert_eq!(native.texts.len(), 2);
    assert!(native.texts.iter().all(|t| t.starts_with("wpkh(")));
    assert!(
        pairs
            .iter()
            .flat_map(|p| &p.texts)
            .all(|t| !t.starts_with("wsh("))
    );
}

#[test]
fn a_person_reading_a_plain_wallet_sees_each_pair_with_its_own_take() {
    let mut app = opened(vec![("plain.dat", vector("plain.dat"))]);
    read_with(&mut app, "");
    for _ in 0..40 {
        if offered(&mut app, Action::CoreWallet(W::Take(3))) {
            break;
        }
        app.event(Event::Scroll {
            x: 240,
            y: 400,
            dy: 120,
        });
    }
    let seen = texts(&mut app);
    for label in [
        "Legacy (pkh)",
        "Nested SegWit (sh-wpkh)",
        "Native SegWit (wpkh)",
        "Taproot (tr)",
    ] {
        assert!(seen.iter().any(|t| t == label), "{label}: {seen:?}");
    }
    let takes = seen.iter().filter(|t| *t == "Take to Wallets").count();
    assert_eq!(takes, 4, "{seen:?}");
}

#[test]
fn a_wallet_not_whole_is_not_taken() {
    let mut app = opened(vec![("wallet.dat", vector("encrypted.dat"))]);
    read_with(&mut app, "not the passphrase");
    assert!(!offered(&mut app, Action::CoreWallet(W::Take(0))));
    assert!(!has(&mut app, "Take to Wallets"));
}

#[test]
fn a_whole_wallet_with_no_active_descriptor_says_so_and_offers_no_take() {
    let mut app = opened(vec![("multisig.dat", vector("multisig.dat"))]);
    read_with(&mut app, "");
    assert!(has(&mut app, "The wallet is whole"));
    assert!(has(&mut app, "No active descriptors"));
    assert!(!offered(&mut app, Action::CoreWallet(W::Take(0))));
    assert!(!has(&mut app, "Take to Wallets"));
}

#[test]
fn a_wallet_with_a_plain_active_pair_does_not_say_there_is_none() {
    let mut app = opened(vec![("plain.dat", vector("plain.dat"))]);
    read_with(&mut app, "");
    assert!(!has(&mut app, "No active descriptors"));
}

/// A wallet whose active Native SegWit receive slot holds the multisig
/// sample's `wsh(multi(...))`, as Bitcoin Core marks a `wsh` one, and which
/// has no active change descriptor there.
fn multisig_active_in_the_segwit_slot() -> faraday_core::core_wallet::Wallet {
    let (multisig_id, multisig) = load("multisig");
    let alone = read_records(multisig_id, &multisig);
    let receive = alone
        .descriptors
        .iter()
        .find(|d| d.listed.starts_with("wsh(multi("))
        .unwrap()
        .id;
    let (id, mut records) = load("plain");
    // Its descriptors with their keys and caches, so that it is whole.
    records.extend(load("multisig").1.into_iter().filter(|(k, _)| {
        k.get(1..)
            .is_some_and(|n| n.starts_with(b"walletdescriptor"))
    }));
    let mut swapped = 0;
    records.retain_mut(|(k, v)| {
        let name = k.get(1..).unwrap_or_default();
        if k.last() != Some(&2) {
            return true;
        }
        if name.starts_with(b"activeexternalspk") {
            *v = receive.to_vec();
            swapped += 1;
        }
        !name.starts_with(b"activeinternalspk")
    });
    assert_eq!(swapped, 1);
    read_records(id, &records)
}

#[test]
fn a_multisig_active_in_the_segwit_slot_is_named_a_script_and_taken_as_the_multisig() {
    let w = multisig_active_in_the_segwit_slot();
    assert!(w.is_whole(), "{:?}", w.failures);
    let pairs = faraday_core::core_wallet::tool::active_pairs(&w);
    let labels: Vec<&str> = pairs.iter().map(|p| p.label).collect();
    assert_eq!(
        labels,
        [
            "Legacy (pkh)",
            "Nested SegWit (sh-wpkh)",
            "Script",
            "Taproot (tr)"
        ]
    );
    let multisig = &pairs[NATIVE];
    assert_eq!(multisig.texts.len(), 1);
    assert!(multisig.texts[0].starts_with("wsh(multi("));
    let mut session = Session::on(Network::Regtest);
    let c = testkit::compute();
    session
        .add_wallet(
            &c,
            "multisig",
            &multisig.texts.join("\n"),
            "multisig.dat",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    assert!(session.wallets[0].same().starts_with("wsh(multi("));
}

#[test]
fn a_migrated_wallet_lists_its_combo_descriptors_as_kept_and_takes_an_active_pair() {
    let mut app = opened(vec![("wallet.dat", vector("migrated.dat"))]);
    read_with(&mut app, "");
    assert!(has(&mut app, "The wallet is whole"));
    // The kept descriptors are drawn below the others: scrolled down to.
    let _ = app.frame();
    for _ in 0..40 {
        if app
            .drawn_texts()
            .iter()
            .any(|t| t.starts_with("Kept by Bitcoin Core, not taken ("))
        {
            break;
        }
        let (w, h) = app.size();
        app.event(Event::Scroll {
            x: (w * 0.75) as u16,
            y: (h / 2.0) as u16,
            dy: 120,
        });
        let _ = app.frame();
    }
    assert!(has_part(&mut app, "Kept by Bitcoin Core, not taken ("));
    assert!(has_part(&mut app, "combo("));
    app.press(Action::CoreWallet(W::Take(0)));
    let seen = texts(&mut app);
    assert_eq!(app.screen, Screen::Wallets, "{seen:?}");
}
