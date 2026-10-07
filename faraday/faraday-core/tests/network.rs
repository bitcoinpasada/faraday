//! The network follows what comes in: a testnet wallet or PSBT moves the
//! session to testnet, and a wallet or PSBT from another network than the
//! loaded wallets is refused. Every key and wallet form on the test stick
//! loads as the wallet or key it stands for.

use faraday_core::testkit;
use faraday_core::wallet::{FileKind, Session, classify};
use faraday_core::{Action, Faraday, StorageEvent};
use osk_bip::keys::Network;

/// BIP-84's account 0 for the "abandon … about" seed: a mainnet wallet.
const MAINNET_WALLET: &str = "wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_inbox(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

#[test]
fn a_testnet_wallet_moves_the_session_to_testnet_and_a_mainnet_one_is_then_refused() {
    let mut app = with_inbox(vec![
        (
            "spending-wallet.txt",
            kit("spending").descriptor.into_bytes(),
        ),
        ("main-wallet.txt", MAINNET_WALLET.as_bytes().to_vec()),
    ]);
    assert_eq!(app.session.network(), Network::Mainnet);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.network(), Network::Testnet);
    assert_eq!(app.session.wallets.len(), 1);
    let first = app.session.address(&app.session.wallets[0], false, 0);
    assert!(first.starts_with("tb1q"), "{first}");

    app.press(Action::LoadWallet(1));
    assert_eq!(
        app.session.wallets.len(),
        1,
        "the mainnet wallet was loaded"
    );
    assert_eq!(app.session.network(), Network::Testnet);
    assert!(app.toast_text().is_some_and(|t| t.contains("Mainnet")));
}

#[test]
fn moving_to_mainnet_is_refused_while_testnet_wallets_are_loaded() {
    let mut app = with_inbox(vec![(
        "spending-wallet.txt",
        kit("spending").descriptor.into_bytes(),
    )]);
    app.press(Action::LoadWallet(0));
    app.press(Action::Network(Network::Mainnet));
    assert_eq!(app.session.network(), Network::Testnet);
    // Signet writes keys as testnet does: the same wallets stay.
    app.press(Action::Network(Network::Signet));
    assert_eq!(app.session.network(), Network::Signet);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_testnet_psbt_moves_an_empty_session_to_testnet() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = with_inbox(vec![("spend.psbt", psbt.to_bytes())]);
    app.press(Action::StartSpend(0));
    assert!(app.spend.is_some());
    assert_eq!(app.session.network(), Network::Testnet);
}

#[test]
fn a_mainnet_psbt_is_refused_while_testnet_wallets_are_loaded() {
    // The Spending spend with its key paths at coin type 0, as a mainnet
    // coordinator writes them.
    let mut inner = testkit::unsigned(&kit("spending")).unwrap().into_inner();
    for (_, path) in inner.inputs[0].bip32_derivation.values_mut() {
        let mut steps: Vec<_> = path.into_iter().copied().collect();
        steps[1] = osk_bip::bitcoin::bip32::ChildNumber::from_hardened_idx(0).unwrap();
        *path = steps.into();
    }
    let mainnet = osk_psbt::Psbt::from(inner);
    let mut app = with_inbox(vec![
        ("spend.psbt", mainnet.to_bytes()),
        (
            "spending-wallet.txt",
            kit("spending").descriptor.into_bytes(),
        ),
    ]);
    app.press(Action::LoadWallet(1));
    app.press(Action::StartSpend(0));
    assert!(app.spend.is_none(), "a mainnet PSBT was opened");
    assert_eq!(app.session.network(), Network::Testnet);
}

#[test]
fn every_wallet_form_on_the_test_stick_loads_its_wallet() {
    let files = testkit::files().unwrap();
    let mut wallets: Vec<(String, String)> = testkit::kits()
        .into_iter()
        .map(|k| (k.id.to_string(), k.descriptor))
        .collect();
    wallets.push((
        "musig".into(),
        testkit::musig_policy().unwrap().to_descriptor_checksummed(),
    ));
    wallets.push((
        "threshold".into(),
        testkit::threshold_record().unwrap().to_text(),
    ));
    let mut seen = 0;
    for (id, descriptor) in &wallets {
        let mut want = Session::default();
        want.add_wallet(id, descriptor, "kit").unwrap();
        // The same wallet: the same first receive and change addresses.
        let addresses = |s: &Session| {
            (
                s.address(&s.wallets[0], false, 0),
                s.address(&s.wallets[0], true, 0),
            )
        };
        let want = addresses(&want);
        for suffix in ["-wallet.txt", "-wallet.json", "-multisig-setup.txt"] {
            let name = format!("{id}{suffix}");
            let Some((_, bytes)) = files.iter().find(|(n, _)| *n == name) else {
                continue;
            };
            assert_eq!(classify(&name, bytes), FileKind::Wallet, "{name}");
            let mut s = Session::default();
            s.add_wallet(id, &String::from_utf8_lossy(bytes), &name)
                .unwrap_or_else(|e| panic!("{name}: {}", e.text()));
            assert_eq!(addresses(&s), want, "{name}");
            assert_eq!(s.network(), Network::Testnet, "{name}");
            seen += 1;
        }
    }
    assert!(seen >= 12 * 2, "only {seen} wallet files");
}

#[test]
fn every_seed_form_and_xpub_on_the_test_stick_is_its_key() {
    let files = testkit::files().unwrap();
    for (n, (word, _)) in testkit::TEST_SEEDS.iter().enumerate() {
        let mut s = testkit::session();
        let fp = s.add_words(&testkit::test_words(word), "", None).unwrap();
        let stem = format!("seed-{}-{}", n + 1, faraday_core::wallet::fp_text(fp));
        let xpub_stem = format!("xpub-{}-{}", n + 1, faraday_core::wallet::fp_text(fp));
        let get = |suffix: &str| {
            files
                .iter()
                .find(|(name, _)| *name == format!("{stem}{suffix}"))
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| panic!("no {stem}{suffix}"))
        };
        // The words file holds the words.
        let words = String::from_utf8(get("-words.txt")).unwrap();
        assert!(words.contains(&testkit::test_words(word)));
        // The backup is an OpenSigner backup.
        assert_eq!(
            classify(&format!("{stem}.oskb"), &get(".oskb")),
            FileKind::Backup
        );
        // Each account xpub is a cosigner's with this key's fingerprint,
        // on testnet.
        for what in ["multisig", "nested-multisig", "taproot-multisig", "single"] {
            let name = format!("{xpub_stem}-{what}.txt");
            let bytes = files
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, b)| b.clone())
                .unwrap_or_else(|| panic!("no {name}"));
            assert_eq!(classify(&name, &bytes), FileKind::Key, "{name}");
            let text = String::from_utf8(bytes).unwrap();
            assert!(text.contains(&format!("[{}/", faraday_core::wallet::fp_text(fp))));
            assert!(text.contains("]tpub"), "{name}");
        }
    }
}
