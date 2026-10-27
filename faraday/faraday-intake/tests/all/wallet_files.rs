//! Other wallets' files, read by the description reader and by the
//! readers it calls (`caravan`, `electrum`, `single_key`). The files are
//! the ones those wallets' own test suites carry
//! (`faraday/faraday-core/tests/vectors/interop/README.md`); each reads as
//! the wallet it holds, with the first address it shows, and a malformed
//! or wrong one is refused.

use crate::common::{interop, parse};
use faraday_bip::keys::Network;
use faraday_bip::policy::{Template, WalletPolicy, Wrapper};
use faraday_intake::wallet::{Refusal, first_address, holds_secret, json_label, read_wallet};
use faraday_intake::{caravan, electrum, single_key};

fn wallet(path: &str) -> WalletPolicy {
    let c = crate::common::compute();
    read_wallet(&c, &interop(path)).unwrap_or_else(|e| panic!("{path}: {}", e.text()))
}

fn refusal(path: &str) -> String {
    let c = crate::common::compute();
    match read_wallet(&c, &interop(path)) {
        Err(Refusal::NotAWallet(why)) => why,
        Err(other) => panic!("{path}: {}", other.text()),
        Ok(_) => panic!("{path} was read as a wallet"),
    }
}

fn first(policy: &WalletPolicy) -> (Network, String) {
    first_address(&crate::common::compute(), policy).unwrap()
}

/// The quorum and the number of keys of a multisig wallet.
fn quorum(policy: &WalletPolicy) -> (usize, usize, Wrapper) {
    match policy.template() {
        Template::Multi {
            threshold, wrapper, ..
        } => (threshold, policy.keys().len(), wrapper),
        other => panic!("not a multisig: {other:?}"),
    }
}

#[test]
fn a_coldcard_multisig_setup_file_reads_as_its_2_of_4_with_every_key() {
    let w = wallet("sparrow/cc-multisig-export-1.txt");
    let (m, n, wrapper) = quorum(&w);
    assert_eq!((m, n), (2, 4));
    assert!(matches!(wrapper, Wrapper::Wsh));
    let fingerprints: Vec<String> = w
        .keys()
        .iter()
        .map(|k| k.fingerprint().unwrap().to_string())
        .collect();
    assert_eq!(
        fingerprints,
        ["0f056943", "6ba6cfd0", "747b698e", "7bb026be"]
    );
    assert_eq!(
        first(&w),
        (
            Network::Mainnet,
            "bc1qafdvz3k4sjwn5lmq44yk5tzag4tyfq72mcqq5nzrsqepmsynrt9st3z2k9".to_string()
        )
    );
}

#[test]
fn a_coldcard_nested_testnet_setup_file_reads_on_testnet_at_its_first_address() {
    let w = wallet("sparrow/cc-multisig-export-2.txt");
    let (m, n, wrapper) = quorum(&w);
    assert_eq!((m, n), (2, 4));
    assert!(matches!(wrapper, Wrapper::ShWsh));
    assert_eq!(
        first(&w),
        (
            Network::Testnet,
            "2MzrH47gyB7r4a9MUvEGq4qSM5amPbe6DMS".to_string()
        )
    );
}

#[test]
fn a_coldcard_setup_file_whose_cosigners_derive_differently_reads_with_each_ones_own_path() {
    let w = wallet("sparrow/cc-multisig-export-multideriv.txt");
    assert_eq!(quorum(&w).0, 3);
    let d = w.to_descriptor();
    assert!(d.contains("/47h/0h/0h/1h]"), "{d}");
    assert_eq!(
        first(&w).1,
        "bc1qhkcnw08rcwhwr9lq6tfczd9jz5zrp7fcatxfq0wwfqafzhcfer3qm59y4m"
    );
}

#[test]
fn a_coldcard_setup_file_with_a_cosigner_missing_or_no_policy_is_refused() {
    for path in [
        "sparrow/cc-multisig-export-missingkey.txt",
        "sparrow/cc-multisig-export-nopolicy.txt",
    ] {
        let c = crate::common::compute();
        assert!(
            matches!(read_wallet(&c, &interop(path)), Err(Refusal::NotAWallet(_))),
            "{path}"
        );
    }
}

#[test]
fn a_coldcard_account_export_reads_as_the_native_segwit_wallet_it_states_the_first_address_of() {
    let text = interop("bluewallet/coldcardmk4-generic.json");
    let w = wallet("bluewallet/coldcardmk4-generic.json");
    assert!(
        w.to_descriptor()
            .starts_with("wpkh([086ee178/84h/0h/0h]xpub")
    );
    let stated = parse(&text)
        .get("bip84")
        .and_then(|b| b.get("first"))
        .and_then(|f| f.as_str().map(str::to_string))
        .unwrap();
    assert_eq!(first(&w), (Network::Mainnet, stated));
}

#[test]
fn single_account_exports_read_as_the_wallet_and_first_address_their_makers_show() {
    for (path, start, address) in [
        (
            "sparrow/jade-keystore.txt",
            "wpkh([73c5da0a/84h/0h/0h]",
            "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu",
        ),
        (
            "sparrow/keystone-singlesig-keystore-1.txt",
            "wpkh([5271C071/84h/0h/0h]",
            "bc1q9unqc738dxjg5mk8zqtz33zg59cahrj29s24lp",
        ),
        (
            "sparrow/specter-diy-keystore.txt",
            "wpkh([b317ec86/84h/1h/0h]",
            "tb1q685z47r72vmqxskvrgc6y6vln7rgwald3463el",
        ),
        (
            "sparrow/cc-singlesig-keystore-1.json",
            "wpkh([0f056943/84h/1h/123h]",
            "tb1qc58ys2dphtphg6yuugdf3d0kufmk0tye044g3l",
        ),
        (
            "bluewallet/coldcardQ.json",
            "wpkh([b68af6e4/84h/0h/0h]",
            "bc1q8tart7tv6da6ylqauf2fde23vwgpsakxznkf7c",
        ),
    ] {
        let w = wallet(path);
        assert!(w.to_descriptor().starts_with(start), "{path}");
        assert_eq!(first(&w).1, address, "{path}");
    }
}

#[test]
fn a_file_of_a_kind_that_is_not_a_wallet_is_refused_with_a_reason() {
    for path in [
        "bluewallet/cobo-crypto-account.txt",
        "bluewallet/sparrow-crypto-output.txt",
        "sparrow/specter-diy-export.txt",
        "sparrow/keystone-singlesig-sp-keystore-1.txt",
        "bluewallet/quicklook-preview-sample.txn",
    ] {
        assert!(!refusal(path).is_empty(), "{path}");
    }
}

#[test]
fn a_caravan_multisig_config_reads_as_its_2_of_3_with_every_key() {
    let text = interop("sparrow/caravan-multisig-export-1.json");
    let v = parse(&text);
    assert!(caravan::looks_like(&v));
    let d = caravan::descriptor(&v).unwrap();
    assert!(d.starts_with("wsh(sortedmulti(2,"), "{d}");
    for fp in ["8188029f", "a15e8133", "d31cf7f9"] {
        assert!(d.contains(&format!("[{fp}/48'/0'/0'/2']")), "{fp}: {d}");
    }
    let w = wallet("sparrow/caravan-multisig-export-1.json");
    assert_eq!(quorum(&w).0, 2);
    assert_eq!(w.keys().len(), 3);
    assert_eq!(
        first(&w).1,
        "bc1qycv3tcefpavtj0nqrwgunel3e4rjquxz9wdgtm8xtmr9m80r2ylsw24vzc"
    );
}

#[test]
fn a_caravan_config_whose_quorum_does_not_match_its_keys_is_refused() {
    for path in [
        "sparrow/caravan-multisig-export-2.json",
        "sparrow/caravan-multisig-export-3.json",
    ] {
        let v = parse(&interop(path));
        assert!(caravan::looks_like(&v));
        assert!(caravan::descriptor(&v).is_err(), "{path}");
        assert!(refusal(path).contains("quorum"), "{path}");
    }
}

#[test]
fn a_caravan_config_with_a_key_that_has_no_origin_is_refused() {
    let v = parse(&interop("bluewallet/caravan-multisig.json"));
    assert!(caravan::looks_like(&v));
    let why = caravan::descriptor(&v).unwrap_err();
    assert!(why.contains("bip32Path"), "{why}");
}

#[test]
fn json_that_is_not_a_caravan_config_is_not_taken_for_one() {
    for path in [
        "bluewallet/coldcardQ.json",
        "sparrow/electrum-singlesig-wallet.json",
    ] {
        assert!(!caravan::looks_like(&parse(&interop(path))), "{path}");
    }
}

#[test]
fn an_electrum_single_key_file_reads_as_its_nested_segwit_wallet() {
    let text = interop("sparrow/electrum-singlesig-wallet.json");
    let v = parse(&text);
    assert!(electrum::looks_like(&v));
    assert!(!electrum::holds_secret(&v));
    assert!(!holds_secret(&text));
    let d = electrum::descriptor(&v).unwrap();
    assert!(d.starts_with("sh(wpkh([ab543c67/49h/0h/0h]xpub"), "{d}");
    let w = wallet("sparrow/electrum-singlesig-wallet.json");
    assert_eq!(
        first(&w),
        (
            Network::Mainnet,
            "3C36uuKHxSGtPbJ4xYvPXdqWVKpWvCuGWJ".to_string()
        )
    );
}

#[test]
fn an_electrum_multisig_file_reads_as_its_2_of_4_on_testnet() {
    let w = wallet("sparrow/electrum-multisig-wallet.json");
    let (m, n, wrapper) = quorum(&w);
    assert_eq!((m, n), (2, 4));
    assert!(matches!(wrapper, Wrapper::ShWsh));
    assert_eq!(
        first(&w),
        (
            Network::Testnet,
            "2MzrH47gyB7r4a9MUvEGq4qSM5amPbe6DMS".to_string()
        )
    );
}

#[test]
fn an_electrum_file_holding_a_private_key_is_called_a_secret_and_not_read_as_a_wallet() {
    let path = "bluewallet/fromsparrow-electrum.json";
    let text = interop(path);
    assert!(electrum::holds_secret(&parse(&text)));
    assert!(holds_secret(&text));
    assert!(refusal(path).contains("Electrum"));
}

#[test]
fn a_text_that_is_not_an_electrum_file_is_not_called_a_secret_one() {
    assert!(!holds_secret(&interop(
        "bluewallet/coldcardmk4-generic.json"
    )));
    assert!(!holds_secret(&interop(
        "bluewallet/coldcardmk4-descriptor.txt"
    )));
    assert!(!electrum::encrypted_whole(
        "{\"wallet_type\": \"standard\"}"
    ));
    assert!(electrum::encrypted_whole("QklFMQM="));
}

#[test]
fn a_specter_wallet_file_reads_with_its_name_its_script_and_its_first_address() {
    let text = interop("sparrow/specter-wallet.json");
    assert_eq!(json_label(&text).as_deref(), Some("specter-wallet"));
    let w = wallet("sparrow/specter-wallet.json");
    assert!(
        w.to_descriptor()
            .starts_with("sh(wpkh([4df18faa/49h/0h/0h]xpub")
    );
    assert_eq!(first(&w).1, "33c64CnvBKZUu5VDqYANEjFFCzvcFJUs11");
}

#[test]
fn a_specter_multisig_file_reads_as_a_3_of_n_multi_with_its_first_address() {
    let text = interop("sparrow/specter-multisig-wallet.json");
    assert_eq!(json_label(&text).as_deref(), Some("specter-multisig"));
    let w = wallet("sparrow/specter-multisig-wallet.json");
    assert_eq!(quorum(&w).0, 3);
    assert!(w.to_descriptor().starts_with("wsh(multi(3,"));
    assert_eq!(
        first(&w).1,
        "bc1qp2z49y5m7dfca5944adfs9fydwawjeunnu8250h5t5jy4pyfma0qquw47e"
    );
}

#[test]
fn a_wallet_files_label_over_several_lines_is_one_line_of_text() {
    let text = "{\"label\": \" Savings\\ndescriptor: wpkh(x)\u{2028}line: y\\r\\u0085\\t \", \"descriptor\": \"\"}";
    assert_eq!(
        json_label(text).as_deref(),
        Some("Savingsdescriptor: wpkh(x)line: y")
    );
    // A label of control characters and spaces alone is no label.
    assert_eq!(json_label("{\"label\": \"\\n\\t\\u2029 \"}"), None);
}

#[test]
fn text_with_no_label_has_no_json_label() {
    assert_eq!(json_label(&interop("sparrow/jade-keystore.txt")), None);
    assert_eq!(json_label("{\"label\": \"  \"}"), None);
    assert_eq!(json_label("not json"), None);
}

#[test]
fn a_cobo_or_keystone_file_reads_as_its_account_key_with_the_path_it_states() {
    let v = parse(&interop("sparrow/cobo-singlesig-keystore-1.json"));
    let key = single_key::read(&v).unwrap().unwrap();
    assert_eq!(key.source, single_key::Source::Cobo);
    assert_eq!(key.script, faraday_bip::slip132::Script::NativeSegwit);
    assert!(
        key.key.starts_with("[73c5da0a/84h/0h/0h]xpub"),
        "{}",
        key.key
    );
    let d = key.descriptor().unwrap();
    assert_eq!(d, format!("wpkh({}/<0;1>/*)", key.key));
    let w = wallet("sparrow/cobo-singlesig-keystore-1.json");
    assert_eq!(first(&w).1, "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu");
}

#[test]
fn a_coldcard_file_for_wasabi_reads_as_the_bip_84_account_in_its_master_key() {
    let text = interop("bluewallet/coldcardmk4-wasabi.json");
    let key = single_key::read(&parse(&text)).unwrap().unwrap();
    assert_eq!(key.source, single_key::Source::Wasabi);
    assert!(
        key.key.starts_with("[086ee178/84h/0h/0h]xpub"),
        "{}",
        key.key
    );
    // Its wallet is the Coldcard's own export of the same account.
    assert_eq!(
        wallet("bluewallet/coldcardmk4-wasabi.json").to_descriptor(),
        wallet("bluewallet/coldcardmk4-generic.json").to_descriptor()
    );
}

#[test]
fn json_that_is_not_a_single_key_file_is_not_read_as_one() {
    for path in [
        "bluewallet/coldcardQ.json",
        "sparrow/electrum-singlesig-wallet.json",
        "sparrow/caravan-multisig-export-1.json",
    ] {
        assert!(single_key::read(&parse(&interop(path))).is_none(), "{path}");
    }
}

#[test]
fn a_single_key_file_with_a_damaged_fingerprint_or_key_is_refused() {
    let text = interop("bluewallet/coldcardmk4-wasabi.json");
    let fp = text.split("\"MasterFingerprint\": \"").nth(1).unwrap()[..8].to_string();
    let bad_fp = text.replace(&fp, "ZZZZZZZZ");
    assert!(single_key::read(&parse(&bad_fp)).unwrap().is_err());
    let xpub = text.split("\"ExtPubKey\": \"").nth(1).unwrap();
    let xpub = &xpub[..xpub.find('"').unwrap()];
    let mut damaged = xpub.to_string();
    damaged.replace_range(20..21, if &damaged[20..21] == "A" { "B" } else { "A" });
    let bad_key = text.replace(xpub, &damaged);
    assert!(single_key::read(&parse(&bad_key)).unwrap().is_err());
}

#[test]
fn the_wasabi_file_faraday_writes_reads_back_as_the_same_key() {
    let w = wallet("bluewallet/coldcardmk4-generic.json");
    assert!(single_key::wasabi_writes(&w));
    let file = single_key::wasabi_file(&w).unwrap();
    let back = single_key::read(&parse(&file)).unwrap().unwrap();
    let original = single_key::read(&parse(&interop("bluewallet/coldcardmk4-wasabi.json")))
        .unwrap()
        .unwrap();
    assert_eq!(back, original);
}

#[test]
fn only_a_bip_84_account_0_wallet_has_a_wasabi_file() {
    let nested = wallet("sparrow/electrum-singlesig-wallet.json");
    assert!(!single_key::wasabi_writes(&nested));
    assert!(single_key::wasabi_file(&nested).is_none());
    let multi = wallet("sparrow/cc-multisig-export-1.txt");
    assert!(!single_key::wasabi_writes(&multi));
}

#[test]
fn the_files_faraday_writes_for_other_wallets_read_back_as_the_same_wallet() {
    let c = crate::common::compute();
    let multi = wallet("sparrow/caravan-multisig-export-1.json");
    assert!(caravan::writes(&multi));
    let file = caravan::file("Vault", &multi, Network::Mainnet).unwrap();
    let again = caravan::descriptor(&parse(&file)).unwrap();
    assert_eq!(
        read_wallet(&c, &again).unwrap().to_descriptor(),
        multi.to_descriptor()
    );
    let multi = wallet("sparrow/electrum-multisig-wallet.json");
    assert!(electrum::writes(&multi));
    let file = electrum::file(&multi).unwrap();
    assert_eq!(
        read_wallet(&c, &file).unwrap().to_descriptor(),
        multi.to_descriptor()
    );
}

#[test]
fn a_descriptor_in_a_receive_and_change_pair_or_alone_is_one_wallet_with_one_first_address() {
    let one = wallet("sparrow/descriptor-multipath.txt");
    let pair = wallet("sparrow/descriptor-receive-change1.txt");
    let labelled = wallet("sparrow/descriptor-labelled.txt");
    assert_eq!(first(&pair), first(&one));
    assert_eq!(first(&labelled).0, Network::Mainnet);
    let alone = wallet("sparrow/descriptor-receive.txt");
    assert_eq!(
        first(&alone).1,
        "bc1qg6stutzna28k472gr2hfnvvz63503qv8x2hsv9"
    );
}

#[test]
fn a_bsms_record_with_a_wrong_field_is_refused_and_a_sound_one_is_read() {
    assert!(wallet("sparrow/bsms/multisig-1.bsms").keys().len() > 1);
    for n in [5, 7, 14, 15, 16, 17] {
        let path = format!("sparrow/bsms/multisig-{n}.bsms");
        assert!(refusal(&path).contains("BSMS"), "{path}");
    }
}
