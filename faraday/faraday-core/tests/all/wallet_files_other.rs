//! Wallet files from Caravan's, Coldcard's and Liana's own test suites
//! (`tools/vectors/wallet-files/`), read as Files reads them: each is
//! the wallet its name and contents state, and the files one wallet
//! comes in read as that one wallet.

use faraday_bip::keys::Network;
use faraday_bip::policy::WalletPolicy;
use faraday_core::wallet::{FileKind, classify, read_wallet, same_wallet};

fn file(path: &str) -> String {
    let full = format!(
        "{}/../../tools/vectors/wallet-files/{path}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

fn wallet(path: &str) -> WalletPolicy {
    let c = faraday_core::testkit::compute();
    let text = file(path);
    assert_eq!(
        classify(&c, path, text.as_bytes()),
        FileKind::Wallet,
        "{path} is not a wallet in Files"
    );
    read_wallet(&c, &text).unwrap_or_else(|e| panic!("{path}: {e:?}"))
}

fn same(a: &WalletPolicy, b: &WalletPolicy) -> bool {
    same_wallet(a) == same_wallet(b)
}

/// Each script type: Caravan's name, Coldcard's name, the descriptor's start.
const FORMATS: [(&str, &str, &str); 3] = [
    ("P2SH", "p2sh", "sh(sortedmulti(2,"),
    ("P2SH-P2WSH", "p2sh-p2wsh", "sh(wsh(sortedmulti(2,"),
    ("P2WSH", "p2wsh", "wsh(sortedmulti(2,"),
];

#[test]
fn caravans_wallet_and_its_coldcard_setup_file_read_as_one_two_of_two_wallet() {
    for (caravan, coldcard, start) in FORMATS {
        for (net, m) in [("MAINNET", "m"), ("TESTNET", "t")] {
            let c = faraday_core::testkit::compute();
            let network = if net == "MAINNET" {
                Network::Mainnet
            } else {
                Network::Testnet
            };
            let a = wallet(&format!(
                "caravan/fixtures/caravan/bitcoin-2-of-2-{caravan}_{net}.json"
            ));
            let b = wallet(&format!("caravan/fixtures/coldcard/wc-{coldcard}_{m}.txt"));
            assert!(a.to_descriptor().starts_with(start), "{caravan} {net}");
            assert_eq!(a.keys().len(), 2, "{caravan} {net}");
            // The two files list the keys in a different order: the same
            // wallet is the one whose addresses are the same.
            for change in [false, true] {
                assert_eq!(
                    a.address_at(&c, network, change, 0).unwrap().to_string(),
                    b.address_at(&c, network, change, 0).unwrap().to_string(),
                    "{caravan} {net}"
                );
            }
        }
    }
}

#[test]
fn coldcards_two_of_four_wallet_reads_the_same_from_its_three_files() {
    let dir = "coldcard/testing/data/multisig";
    for (_, format, start) in FORMATS {
        // The P2SH setup file has no `Format:` line, which Files refuses.
        let setup = if format == "p2sh" {
            wallet(&format!("{dir}/desc-{format}-myself.txt"))
        } else {
            wallet(&format!("{dir}/export-{format}-myself.txt"))
        };
        assert!(setup.to_descriptor().starts_with(start), "{format}");
        assert_eq!(setup.keys().len(), 4, "{format}");
        let descriptor = wallet(&format!("{dir}/desc-{format}-myself.txt"));
        let electrum = wallet(&format!("{dir}/el-{format}-myself.json"));
        assert!(same(&setup, &descriptor), "{format}: descriptor");
        assert!(same(&setup, &electrum), "{format}: Electrum");
    }
}

#[test]
fn electrums_wallet_files_with_coldcard_keys_read_as_their_wallets() {
    let dir = "coldcard/docs/sample-electrum-wallets";
    let multi = wallet(&format!("{dir}/electrum-2-of-4.json"));
    assert!(multi.to_descriptor().contains("sortedmulti(2,"));
    assert_eq!(multi.keys().len(), 4);
    let single = wallet(&format!("{dir}/electrum-single.json"));
    assert_eq!(single.keys().len(), 1);
}

#[test]
fn coldcards_generic_export_loads_as_its_native_segwit_wallet_with_the_address_it_shows() {
    let c = faraday_core::testkit::compute();
    for path in [
        "coldcard/testing/data/coldcard-export.json",
        "liana/liana-gui/test_assets/coldcard-export.json",
    ] {
        let w = wallet(path);
        assert!(w.to_descriptor().starts_with("wpkh(["), "{path}");
        let stated: serde_json::Value = serde_json::from_str(&file(path)).unwrap();
        assert_eq!(
            w.address_at(&c, Network::Testnet, false, 0)
                .unwrap()
                .to_string(),
            stated["bip84"]["first"].as_str().unwrap(),
            "{path}"
        );
    }
}

#[test]
fn a_cosigner_key_file_and_a_stored_setting_are_not_taken_for_wallets() {
    let c = faraday_core::testkit::compute();
    let dir = "coldcard/testing/data/multisig";
    let mut paths = vec![];
    for x in ["0F056943", "6BA6CFD0", "747B698E", "7BB026BE"] {
        paths.push(format!("{dir}/ccxp-{x}.json"));
    }
    for f in ["p2sh", "p2sh-p2wsh", "p2wsh"] {
        paths.push(format!("{dir}/setting-{f}-myself.json"));
    }
    for path in paths {
        let text = file(&path);
        assert_ne!(
            classify(&c, &path, text.as_bytes()),
            FileKind::Wallet,
            "{path}"
        );
        assert!(read_wallet(&c, &text).is_err(), "{path}");
    }
}
