//! Files other wallets write, read by Faraday. Each loads as the wallet
//! the other wallet holds, with the address it shows; the files are the
//! ones those wallets' own test suites carry
//! (`tests/vectors/interop/README.md`). Faraday's own files against
//! Bitcoin Core are `faraday/tools/core-check.py`.

use faraday_core::wallet::{FileKind, classify, read_wallet};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;

fn file(path: &str) -> String {
    let full = format!(
        "{}/tests/vectors/interop/{path}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("{full}: {e}"))
}

/// The wallet Files loads from the file, which it must call a wallet.
fn wallet(path: &str) -> WalletPolicy {
    let text = file(path);
    assert_eq!(
        classify(path, text.as_bytes()),
        FileKind::Wallet,
        "{path} is not a wallet in Files"
    );
    read_wallet(&text).unwrap_or_else(|e| panic!("{path}: {e:?}"))
}

fn first(policy: &WalletPolicy, net: Network) -> String {
    policy.address_at(net, false, 0).unwrap().to_string()
}

/// The `first` address a Coldcard export states for its BIP-84 account.
fn stated_first(text: &str) -> String {
    let at = text.find("\"bip84\"").unwrap();
    let rest = &text[at..];
    let at = rest.find("\"first\"").unwrap() + "\"first\"".len();
    rest[at..].split('"').nth(1).unwrap().to_string()
}

#[test]
fn a_coldcard_account_export_loads_its_native_segwit_wallet_at_the_coldcards_first_address() {
    for (path, net, master) in [
        (
            "bluewallet/coldcardmk4-generic.json",
            Network::Mainnet,
            "086ee178",
        ),
        ("bluewallet/coldcardQ.json", Network::Mainnet, "b68af6e4"),
        (
            "bluewallet/nunchuk-export.json",
            Network::Mainnet,
            "b68af6e4",
        ),
        (
            "sparrow/cc-singlesig-keystore-1.json",
            Network::Testnet,
            "0f056943",
        ),
    ] {
        let w = wallet(path);
        let d = w.to_descriptor();
        assert!(d.starts_with("wpkh("), "{path}: {d}");
        assert!(
            d.contains(&format!("[{master}/84")),
            "{path} is not rooted at its master: {d}"
        );
        assert_eq!(first(&w, net), stated_first(&file(path)), "{path}");
    }
}

#[test]
fn a_coldcard_account_export_and_the_same_coldcards_descriptor_are_one_wallet() {
    let export = wallet("bluewallet/coldcardmk4-generic.json");
    let descriptor = wallet("bluewallet/coldcardmk4-descriptor.txt");
    assert_eq!(
        first(&export, Network::Mainnet),
        first(&descriptor, Network::Mainnet)
    );
    assert_eq!(
        faraday_core::wallet::same_wallet(&export),
        faraday_core::wallet::same_wallet(&descriptor)
    );
}

#[test]
fn coldcard_multisig_setup_files_load_with_their_quorum_and_every_key() {
    for (path, start, m, fingerprints) in [
        (
            "sparrow/cc-multisig-export-1.txt",
            "wsh(sortedmulti(2,",
            2,
            &["0f056943", "6ba6cfd0", "747b698e", "7bb026be"][..],
        ),
        (
            "sparrow/cc-multisig-export-2.txt",
            "sh(wsh(sortedmulti(2,",
            2,
            &["0f056943", "6ba6cfd0", "747b698e", "7bb026be"][..],
        ),
        (
            "bluewallet/fromsparrow-coldcard.txt",
            "wsh(sortedmulti(2,",
            2,
            &["a3909080", "7ab71df0", "f11b9ff2"][..],
        ),
    ] {
        let d = wallet(path).to_descriptor();
        assert!(d.starts_with(start), "{path}: {d}");
        assert_eq!(wallet(path).quorum().map(|(q, _)| q), Some(m), "{path}");
        for fp in fingerprints {
            assert!(d.contains(&format!("[{fp}/48")), "{path} lost {fp}: {d}");
        }
    }
}

#[test]
fn electrums_multisig_export_keeps_each_cosigners_own_derivation() {
    let d = wallet("sparrow/cc-multisig-export-multideriv.txt").to_descriptor();
    assert!(d.starts_with("wsh(sortedmulti(3,"), "{d}");
    assert!(
        d.contains("[ca9a2b19/47'/0'/0'/1']") || d.contains("[ca9a2b19/47h/0h/0h/1h]"),
        "{d}"
    );
    assert!(
        d.contains("[06b57041/48'/0'/0'/2']") || d.contains("[06b57041/48h/0h/0h/2h]"),
        "{d}"
    );
}

#[test]
fn bsms_records_from_nunchuk_and_sparrow_load_and_their_first_address_holds() {
    for (path, start) in [
        ("bluewallet/nunchuck-bsms.txt", "wsh(sortedmulti(2,"),
        ("sparrow/bsms/multisig-1.bsms", "wsh(sortedmulti(2,"),
        ("sparrow/bsms/multisig-2.bsms", "wsh(sortedmulti(2,"),
        ("sparrow/bsms/multisig-3.bsms", "sh(wsh(multi(2,"),
    ] {
        let w = wallet(path);
        assert!(w.to_descriptor().starts_with(start), "{path}");
        let stated = file(path).lines().nth(3).unwrap().trim().to_string();
        let net = if stated.starts_with("tb1") || stated.starts_with('2') {
            Network::Testnet
        } else {
            Network::Mainnet
        };
        assert_eq!(first(&w, net), stated, "{path}");
    }
}

#[test]
fn a_bsms_record_whose_address_is_not_its_wallets_is_refused() {
    let text = file("sparrow/bsms/multisig-1.bsms");
    let lines: Vec<&str> = text.lines().collect();
    let other = file("sparrow/bsms/multisig-2.bsms");
    let wrong = format!(
        "{}\n{}\n{}\n{}\n",
        lines[0],
        lines[1],
        lines[2],
        other.lines().nth(3).unwrap()
    );
    assert!(read_wallet(&wrong).is_err());
}

#[test]
fn multipath_descriptors_from_sparrow_load() {
    let w = wallet("sparrow/descriptor-multipath.txt");
    assert!(w.to_descriptor().starts_with("wpkh([a262308d/84"));
    assert!(w.address_at(Network::Mainnet, true, 0).is_ok());
}

#[test]
fn a_psbt_and_a_finished_transaction_from_bluewallet_are_named_as_such() {
    let root = format!("{}/tests/vectors/interop", env!("CARGO_MANIFEST_DIR"));
    let psbt = std::fs::read(format!("{root}/bluewallet/quicklook-preview-sample.psbt")).unwrap();
    assert_eq!(classify("sample.psbt", &psbt), FileKind::Psbt);
    let txn = std::fs::read(format!("{root}/bluewallet/quicklook-preview-sample.txn")).unwrap();
    assert_eq!(classify("sample.txn", &txn), FileKind::Transaction);
}

fn scanned(text: &str) -> faraday_core::Item {
    use osk_shell_api::{App, Event};
    let mut app = faraday_core::Faraday::new();
    app.press(faraday_core::Action::Scan);
    app.event(Event::Scanned {
        bytes: text.trim().as_bytes().to_vec(),
    });
    app.inbox
        .last()
        .cloned()
        .unwrap_or_else(|| panic!("nothing reached the Inbox from {text}"))
}

#[test]
fn sparrows_multisig_as_a_crypto_output_code_scans_in_as_its_wallet() {
    let item = scanned(&file("bluewallet/sparrow-crypto-output.txt"));
    assert_eq!(item.kind, FileKind::Wallet, "{}", item.name);
    let w = read_wallet(&String::from_utf8_lossy(&item.bytes)).unwrap();
    assert!(w.to_descriptor().starts_with("wsh(sortedmulti(2,"));
    assert_eq!(w.keys().len(), 3);
}

#[test]
fn a_keystone_cobo_account_code_scans_in_as_its_multisig_account_key() {
    let item = scanned(&file("bluewallet/cobo-crypto-account.txt"));
    assert_eq!(item.kind, FileKind::Key, "{}", item.name);
    let key = faraday_core::create::read_key(&String::from_utf8_lossy(&item.bytes)).unwrap();
    // BlueWallet reads this code as fingerprint 01EBDA7D at m/48'/0'/0'/2'.
    assert!(
        key.starts_with("[01ebda7d/48h/0h/0h/2h]") || key.starts_with("[01ebda7d/48'/0'/0'/2']"),
        "{key}"
    );
}

fn same(a: &WalletPolicy, b: &WalletPolicy) -> bool {
    faraday_core::wallet::same_wallet(a) == faraday_core::wallet::same_wallet(b)
}

#[test]
fn a_receive_and_a_change_descriptor_or_a_receive_one_alone_read_as_the_multipath_wallet() {
    let multipath = wallet("sparrow/descriptor-multipath.txt");
    for path in [
        "sparrow/descriptor-receive-change1.txt",
        "sparrow/descriptor-receive-change2.txt",
        "sparrow/descriptor-receive.txt",
        "sparrow/descriptor-labelled.txt",
    ] {
        assert!(same(&wallet(path), &multipath), "{path}");
    }
}

#[test]
fn jades_single_chain_export_reads_with_its_change_chain() {
    // Jade's test file is the "abandon … about" seed's BIP-84 account,
    // whose first address BIP-84 publishes.
    let w = wallet("sparrow/jade-keystore.txt");
    assert_eq!(
        first(&w, Network::Mainnet),
        "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu"
    );
    assert!(w.address_at(Network::Mainnet, true, 0).is_ok());
}

#[test]
fn specter_desktop_wallet_files_without_derivations_load() {
    let single = wallet("sparrow/specter-wallet.json");
    assert!(single.to_descriptor().starts_with("sh(wpkh([4df18faa/49"));
    let multi = wallet("sparrow/specter-multisig-wallet.json");
    assert!(multi.to_descriptor().starts_with("wsh(multi(3,"));
    assert_eq!(multi.keys().len(), 4);
}

#[test]
fn sparrows_specter_and_coldcard_exports_of_one_wallet_read_as_one_wallet() {
    assert!(same(
        &wallet("bluewallet/fromsparrow-specter.json"),
        &wallet("bluewallet/fromsparrow-coldcard.txt")
    ));
}

#[test]
fn bitcoin_cores_listdescriptors_reads_as_its_native_segwit_wallet() {
    let text = file("core/listdescriptors.json");
    let w = wallet("core/listdescriptors.json");
    let d = w.to_descriptor();
    assert!(d.starts_with("wpkh("), "{d}");
    // The key is the one in Core's own BIP-84 receive descriptor.
    let at = text.find("\"wpkh(").unwrap() + 2;
    let key = &text[at + 5..at + 5 + text[at + 5..].find("/0/*").unwrap()];
    assert!(d.contains(key), "{d} lacks {key}");
}

#[test]
fn a_descriptor_whose_checksum_is_wrong_is_refused_however_it_is_written() {
    let text = file("sparrow/descriptor-receive-change1.txt");
    let broken = text.replacen("#s5x06kda", "#s5x06kdb", 1);
    assert!(read_wallet(&broken).is_err());
}

// Cosigner keys for Create.

use faraday_core::create::{NewKind, key_for, read_keys};

#[test]
fn coldcards_multisig_key_file_gives_the_key_its_multisig_setup_file_lists() {
    // The same Coldcard, 0F056943, on testnet: its key file writes the
    // nested key as a Upub; its setup file lists it as a tpub.
    let key = key_for(
        NewKind::MultiNested,
        &file("sparrow/cc-multisig-keystore-1.json"),
    )
    .unwrap();
    assert!(key.starts_with("[0f056943/48"), "{key}");
    assert!(
        key.ends_with("tpubDF2rnouQaaYrUEy2JM1YD3RFzew4onawGM4X2Re67gguTf5CbHonBRiFGe3Xjz7DK88dxBFGf2i7K1hef3PM4cFKyUjcbJXddaY9F5tJBoP"),
        "{key}"
    );
    let wsh = key_for(NewKind::Multi, &file("sparrow/cc-multisig-keystore-1.json")).unwrap();
    assert!(
        wsh.contains("/48h/1h/0h/2h]") || wsh.contains("/48'/1'/0'/2']"),
        "{wsh}"
    );
    let legacy = key_for(
        NewKind::MultiLegacy,
        &file("sparrow/cc-multisig-keystore-1.json"),
    )
    .unwrap();
    assert!(
        legacy.contains("/45h]") || legacy.contains("/45']"),
        "{legacy}"
    );
}

#[test]
fn unchaineds_key_file_and_the_same_coldcards_generic_export_give_one_key() {
    // Both are Coldcard B68AF6E4: Unchained's file writes the key as a
    // Zpub, the generic export as an xpub in its bip48_2 account.
    let from_unchained = key_for(NewKind::Multi, &file("bluewallet/unchained.json")).unwrap();
    let from_export = key_for(NewKind::Multi, &file("bluewallet/coldcardQ.json")).unwrap();
    assert!(
        from_unchained.starts_with("[b68af6e4/48"),
        "{from_unchained}"
    );
    assert_eq!(from_unchained, from_export);
    let nested = key_for(NewKind::MultiNested, &file("bluewallet/unchained.json")).unwrap();
    assert_eq!(
        nested,
        key_for(NewKind::MultiNested, &file("bluewallet/coldcardQ.json")).unwrap()
    );
}

#[test]
fn a_coldcard_file_with_no_account_for_the_kind_gives_no_key() {
    assert!(key_for(NewKind::TapMulti, &file("bluewallet/coldcardQ.json")).is_none());
    assert!(read_keys(&file("bluewallet/coldcardQ.json")).len() >= 5);
}

#[test]
fn create_offers_a_coldcard_export_for_a_cosigners_slot_and_takes_its_multisig_key() {
    use osk_shell_api::{App, Event, Key};
    let mut app = faraday_core::testkit::started();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![
            (
                "coldcardQ.json".to_string(),
                file("bluewallet/coldcardQ.json").into_bytes(),
            ),
            (
                "ccxp-0F056943.json".to_string(),
                file("sparrow/cc-multisig-keystore-2.json").into_bytes(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(faraday_core::Action::Entry(None));
    for c in faraday_core::testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(faraday_core::Action::EntryAdd);
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(faraday_core::Action::CreateWallet);
    app.press(faraday_core::Action::CKind(multi));
    app.press(faraday_core::Action::CNext(faraday_core::cstep::KIND));
    app.press(faraday_core::Action::CVaults(false));
    app.press(faraday_core::Action::CNext(faraday_core::cstep::WHERE));
    app.press(faraday_core::Action::CNext(faraday_core::cstep::QUORUM));
    let _ = app.frame();
    let at = app
        .inbox
        .iter()
        .position(|i| i.name == "coldcardQ.json")
        .unwrap();
    assert!(app.offers(faraday_core::Action::CSlotFile(1, at)));
    app.press(faraday_core::Action::CSlotFile(1, at));
    let c = app.create.as_ref().unwrap();
    match &c.slots[1] {
        faraday_core::create::Source::Cosigner(k) => {
            assert!(
                k.starts_with("[b68af6e4/48h/0h/0h/2h]")
                    || k.starts_with("[b68af6e4/48'/0'/0'/2']"),
                "{k}"
            )
        }
        other => panic!("slot 2 is {other:?}"),
    }
}

// SLIP-132 keys (`docs/INTEROP.md` §5 I1).

/// BIP-84's published account key for the `abandon … about` seed, which
/// SLIP-0132 writes as a zpub; BIP-84's first receive address for it.
const BIP84_ZPUB: &str = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
const BIP84_FIRST: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";

/// The keys a descriptor lists, as written: what follows each `]`.
fn written_keys(text: &str) -> Vec<String> {
    text.split(']')
        .skip(1)
        .map(|k| k.split('/').next().unwrap().to_string())
        .collect()
}

#[test]
fn casas_descriptor_of_ypubs_loads_as_its_nested_segwit_multisig() {
    let w = wallet("bluewallet/casa-descriptor.txt");
    assert!(
        w.to_descriptor().starts_with("sh(wsh(sortedmulti(3,")
            || w.to_text().starts_with("sh(wsh(sortedmulti(3,"),
        "{}",
        w.to_descriptor()
    );
    assert_eq!(w.keys().len(), 5);
    assert_eq!(w.quorum().map(|(m, _)| m), Some(3));
    assert!(w.address_at(Network::Mainnet, false, 0).is_ok());
}

#[test]
fn specter_diys_keystore_fills_a_native_segwit_slot_and_no_multisig_one() {
    let text = file("sparrow/specter-diy-keystore.txt");
    let key = key_for(NewKind::NativeSegwit, &text).unwrap();
    assert!(
        key.starts_with("[b317ec86/84h/1h/0h]tpub") || key.starts_with("[b317ec86/84'/1'/0']tpub"),
        "{key}"
    );
    assert!(key_for(NewKind::Multi, &text).is_none());
    assert!(key_for(NewKind::NestedSegwit, &text).is_none());
    // On its own the key is the testnet BIP-84 wallet it names.
    let w = wallet("sparrow/specter-diy-keystore.txt");
    assert!(w.to_descriptor().starts_with("wpkh([b317ec86/84"));
    assert!(w.address_at(Network::Testnet, false, 0).is_ok());
}

#[test]
fn keystones_change_descriptor_with_an_upper_case_fingerprint_reads_as_its_wallet() {
    let w = wallet("sparrow/keystone-singlesig-keystore-1.txt");
    let d = w.to_descriptor().to_lowercase();
    assert!(d.starts_with("wpkh([5271c071/84"), "{d}");
    assert!(d.contains("/<0;1>/*"), "{d}");
    assert!(w.address_at(Network::Mainnet, false, 0).is_ok());
    assert!(w.address_at(Network::Mainnet, true, 0).is_ok());
}

#[test]
fn bip_84s_account_as_a_zpub_has_bip_84s_first_address() {
    let bare = read_wallet(BIP84_ZPUB).unwrap();
    assert_eq!(first(&bare, Network::Mainnet), BIP84_FIRST);
    let rooted = read_wallet(&format!("[73c5da0a/84h/0h/0h]{BIP84_ZPUB}")).unwrap();
    assert_eq!(first(&rooted, Network::Mainnet), BIP84_FIRST);
    // The same account as Jade writes it, in xpub form.
    assert!(same(&rooted, &wallet("sparrow/jade-keystore.txt")));
    let described =
        read_wallet(&format!("wpkh([73c5da0a/84h/0h/0h]{BIP84_ZPUB}/<0;1>/*)")).unwrap();
    assert!(same(&described, &rooted));
}

#[test]
fn a_slip132_key_whose_version_disagrees_with_its_script_or_path_is_refused() {
    let unchained = file("bluewallet/unchained.json");
    let zpub = unchained
        .split('"')
        .find(|t| t.starts_with("Zpub"))
        .unwrap();
    let casa = written_keys(&file("bluewallet/casa-descriptor.txt"));
    let (y1, y2) = (&casa[0], &casa[1]);
    for (text, key) in [
        (format!("wpkh([b68af6e4/48h/0h/0h/2h]{zpub}/<0;1>/*)"), zpub),
        (
            format!("wsh(multi(2,[35282ae3]{y1}/<0;1>/*,[1b373e31]{y2}/<0;1>/*))"),
            y1.as_str(),
        ),
        (format!("tr({BIP84_ZPUB}/<0;1>/*)"), BIP84_ZPUB),
        (
            format!("sh(wpkh([73c5da0a/84h/0h/0h]{BIP84_ZPUB}/<0;1>/*))"),
            BIP84_ZPUB,
        ),
        // A zpub on a BIP-49 path.
        (format!("[73c5da0a/49h/0h/0h]{BIP84_ZPUB}"), BIP84_ZPUB),
    ] {
        // Refused by name: the refusal names the key it will not take.
        let e = read_wallet(&text).expect_err(&text);
        assert!(e.text().contains(&key[..8]), "{text}: {}", e.text());
    }
    // A key file whose zpub sits on another purpose's path gives no key.
    let moved = file("sparrow/specter-diy-keystore.txt").replace("/84h/", "/49h/");
    assert!(read_keys(&moved).is_empty());
    assert!(key_for(NewKind::NestedSegwit, &moved).is_none());
}

// BIP 129 records Sparrow writes (`docs/INTEROP.md` §5 I3).

#[test]
fn sparrows_bsms_records_in_upper_case_or_without_path_restrictions_load_at_their_first_address() {
    for path in [
        "sparrow/bsms/multisig-4.bsms",
        "sparrow/bsms/multisig-6.bsms",
    ] {
        let w = wallet(path);
        assert!(
            w.to_descriptor().starts_with("wsh(sortedmulti(2,"),
            "{path}"
        );
        let stated = file(path).lines().nth(3).unwrap().trim().to_string();
        assert_eq!(first(&w, Network::Mainnet), stated.to_lowercase(), "{path}");
    }
}

#[test]
fn bsms_records_sparrow_refuses_stay_refused() {
    for n in [5, 7, 9, 11, 13, 14, 15, 17] {
        let path = format!("sparrow/bsms/multisig-{n}.bsms");
        assert!(read_wallet(&file(&path)).is_err(), "{path}");
    }
}

#[test]
fn a_bsms_records_base58_address_with_its_case_changed_is_refused() {
    let text = file("sparrow/bsms/multisig-3.bsms");
    let address = text.lines().nth(3).unwrap().trim();
    assert!(read_wallet(&text).is_ok());
    for changed in [address.to_uppercase(), address.to_lowercase()] {
        assert!(
            read_wallet(&text.replace(address, &changed)).is_err(),
            "{changed}"
        );
    }
}

// Single-key JSON files (`docs/INTEROP.md` §5 I2).

#[test]
fn cobos_single_key_file_for_bip_84s_seed_has_bip_84s_first_address() {
    // Sparrow's Cobo file: BIP-84's account for `abandon … about`.
    let w = wallet("sparrow/cobo-singlesig-keystore-1.json");
    assert!(w.to_descriptor().starts_with("wpkh([73c5da0a/84"));
    assert_eq!(first(&w, Network::Mainnet), BIP84_FIRST);
    assert!(same(&w, &wallet("sparrow/jade-keystore.txt")));
    // Create takes it for a native SegWit key slot, and for no multisig.
    let text = file("sparrow/cobo-singlesig-keystore-1.json");
    let key = key_for(NewKind::NativeSegwit, &text).unwrap();
    assert!(
        key.starts_with("[73c5da0a/84h/0h/0h]xpub") || key.starts_with("[73c5da0a/84'/0'/0']xpub"),
        "{key}"
    );
    assert!(key_for(NewKind::Multi, &text).is_none());
}

#[test]
fn blue_wallets_cobo_skeleton_is_the_wallet_keystone_exports_for_the_same_key() {
    // BlueWallet's file writes its path with `\/`; Sparrow's Keystone
    // file is the same key, 5271C071, as a change descriptor.
    let cobo = wallet("bluewallet/skeleton-cobo.txt");
    assert!(cobo.to_descriptor().starts_with("wpkh([5271c071/84"));
    // Keystone's keeps its fingerprint upper-case.
    assert_eq!(
        cobo.to_descriptor(),
        wallet("sparrow/keystone-singlesig-keystore-1.txt")
            .to_descriptor()
            .replace("5271C071", "5271c071")
    );
}

#[test]
fn coldcards_wasabi_file_is_the_native_segwit_account_of_its_generic_export() {
    let generic = file("bluewallet/coldcardmk4-generic.json");
    let wasabi = wallet("bluewallet/coldcardmk4-wasabi.json");
    assert!(same(&wasabi, &read_wallet(&generic).unwrap()));
    assert_eq!(first(&wasabi, Network::Mainnet), stated_first(&generic));
}

#[test]
fn a_single_key_file_whose_key_disagrees_with_its_path_is_refused() {
    // BIP-84's zpub on a BIP-49 path, and a Wasabi file (BIP-84) with a
    // ypub.
    let cobo = file("sparrow/cobo-singlesig-keystore-1.json").replace("84'", "49'");
    let e = read_wallet(&cobo).expect_err("zpub at 49'");
    assert!(e.text().contains(&BIP84_ZPUB[..8]), "{}", e.text());
    let generic = file("bluewallet/coldcardmk4-generic.json");
    let ypub = generic.split('"').find(|t| t.starts_with("ypub")).unwrap();
    let wasabi = format!(
        "{{\"ColdCardFirmwareVersion\": \"5.2.2\", \"MasterFingerprint\": \"086EE178\", \"ExtPubKey\": \"{ypub}\"}}"
    );
    assert!(read_wallet(&wasabi).is_err());
}

#[test]
fn json_nested_past_the_depth_limit_is_refused() {
    // The object is one level; the arrays inside it the rest.
    let depth = faraday_intake::json::MAX_DEPTH;
    let cobo = file("sparrow/cobo-singlesig-keystore-1.json");
    let body = cobo.trim().trim_start_matches('{').trim_end_matches('}');
    let deep = format!(
        "{{{body}, \"x\": {}{}}}",
        "[".repeat(depth),
        "]".repeat(depth)
    );
    assert!(read_wallet(&deep).is_err());
    // One level less reads as the file it is.
    let shallow = format!(
        "{{{body}, \"x\": {}{}}}",
        "[".repeat(depth - 1),
        "]".repeat(depth - 1)
    );
    assert_eq!(
        first(&read_wallet(&shallow).unwrap(), Network::Mainnet),
        BIP84_FIRST
    );
}

/// The files Public files' row `what` puts in the Outbox for a wallet
/// loaded from `descriptor`, as text.
fn public_out(descriptor: &str, what: u8) -> Vec<(String, String)> {
    let mut app = faraday_core::Faraday::new();
    app.session = faraday_core::testkit::session();
    let w = app
        .session
        .add_wallet(
            "Test",
            descriptor,
            "Test wallet",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap_or_else(|e| panic!("{descriptor}: {}", e.text()));
    app.press(faraday_core::Action::PublicOut(w, what));
    app.outbox
        .iter()
        .map(|i| (i.name.clone(), String::from_utf8(i.bytes.clone()).unwrap()))
        .collect()
}

/// Public files' Wasabi row (`Faraday::public_out`).
const WASABI_ROW: u8 = 13;
/// Public files' Caravan row.
const CARAVAN_ROW: u8 = 11;

fn kit(id: &str) -> String {
    faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == id)
        .unwrap()
        .descriptor
}

#[test]
fn faradays_wasabi_file_reads_back_as_the_same_wallet_and_only_bip_84s_account_0_has_one() {
    let spending = kit("spending");
    let out = public_out(&spending, WASABI_ROW);
    let [(name, text)] = &out[..] else {
        panic!("{out:?}")
    };
    assert_eq!(name, "test-wasabi.json");
    let back = read_wallet(text).unwrap();
    assert!(same(&back, &read_wallet(&spending).unwrap()));
    assert_eq!(
        first(&back, Network::Testnet),
        first(&read_wallet(&spending).unwrap(), Network::Testnet)
    );
    for other in ["nested", "taproot", "savings"] {
        assert!(public_out(&kit(other), WASABI_ROW).is_empty(), "{other}");
    }
    // Account 1 is not the account Wasabi assumes.
    let account_1 = format!(
        "wpkh({}/<0;1>/*)",
        faraday_core::testkit::key(0, "m/84'/1'/1'")
    );
    assert!(public_out(&account_1, WASABI_ROW).is_empty());
}

// Caravan and Unchained configs (`docs/INTEROP.md` §5 I5).

#[test]
fn sparrows_caravan_export_loads_as_the_descriptor_its_fields_spell() {
    let w = wallet("sparrow/caravan-multisig-export-1.json");
    let p = "48'/0'/0'/2'";
    let by_hand = format!(
        "wsh(sortedmulti(2,[8188029f/{p}]xpub6EMVvcTUbaABdaPLaVWE72CjcN72URa5pKK1knrKLz1hKaDwUkgddc3832a8MHEpLyuow7MfjMRomt2iMtwPH4pWrFLft4JsquHjeZfKsYp/<0;1>/*,[a15e8133/{p}]xpub6EbTHAgvzZtVppjLPXia2prt6T2w7DD2VQ8gysgV6ECzmKkA6zt4WR3hX4bgciTDrnaneZoJbA19tYKBMrWuA89SbgYAdbMNWmtzrgLhqco/<0;1>/*,[d31cf7f9/{p}]xpub6Ea6qWGr6BWpCSyLo8ggypvXg1MwadiAGZxAHvrBUwvT2ki6pNVG61bg3YEdxkj7FRJ1cLFK7Vvqd9rcDvwCLX7VfEMfJJfdbA1NGsFazvz/<0;1>/*))"
    );
    assert!(same(&w, &read_wallet(&by_hand).unwrap()));
}

#[test]
fn caravan_exports_sparrow_refuses_are_refused() {
    // Sparrow's own test refuses both: a quorum of 0, and a total of 3
    // over two keys.
    for path in [
        "sparrow/caravan-multisig-export-2.json",
        "sparrow/caravan-multisig-export-3.json",
    ] {
        assert!(read_wallet(&file(path)).is_err(), "{path}");
    }
}

#[test]
fn blue_wallets_caravan_file_with_its_third_keys_path_loads_at_blue_wallets_first_address() {
    // As written, its third key's bip32Path is "Unknown": a multisig
    // wallet here holds no key without its origin, and the key is named.
    let text = file("bluewallet/caravan-multisig.json");
    let e = read_wallet(&text).expect_err("a key with no origin");
    assert!(e.text().contains("xpub6ERj"), "{}", e.text());
    // Sparrow's Coldcard export of the same wallet states that key's
    // path; with it, the config is the wallet that export is.
    let given = text.replace(
        "\"bip32Path\": \"Unknown\"",
        "\"bip32Path\": \"m/48'/0'/0'/2'\"",
    );
    let w = read_wallet(&given).unwrap();
    assert!(same(&w, &wallet("bluewallet/fromsparrow-coldcard.txt")));
    // BlueWallet's `multisig-hd-wallet.test.js`, "can parse files from
    // sparrow wallet".
    assert_eq!(
        first(&w, Network::Mainnet),
        "bc1qtysquqsjqjfqvhd6l2h470hdgwhcahs4nq2ca49cyxftwjnjt9ssh8emel"
    );
}

#[test]
fn a_caravan_config_whose_keys_or_start_disagree_with_it_is_refused() {
    let text = file("sparrow/caravan-multisig-export-1.json");
    for changed in [
        text.replace("\"mainnet\"", "\"testnet\""),
        text.replace("\"startingAddressIndex\": 0", "\"startingAddressIndex\": 5"),
        text.replace("\"P2WSH\"", "\"P2TR\""),
    ] {
        assert!(read_wallet(&changed).is_err(), "{changed}");
    }
    // A key whose SLIP-132 version names nested multisig, in a P2WSH
    // config: Unchained's Ypub.
    let ypub = file("bluewallet/unchained.json")
        .split('"')
        .find(|t| t.starts_with("Ypub"))
        .unwrap()
        .to_string();
    let first_xpub = text
        .split('"')
        .find(|t| t.starts_with("xpub6"))
        .unwrap()
        .to_string();
    let e = read_wallet(&text.replace(&first_xpub, &ypub)).expect_err("Ypub in P2WSH");
    assert!(e.text().contains(&ypub[..8]), "{}", e.text());
}

#[test]
fn faradays_caravan_file_reads_back_with_the_same_first_address() {
    for id in ["savings", "vault-nested", "vault-legacy"] {
        let d = kit(id);
        let out = public_out(&d, CARAVAN_ROW);
        let [(name, text)] = &out[..] else {
            panic!("{id}: {out:?}")
        };
        assert_eq!(name, "test-caravan.json");
        let back = read_wallet(text).unwrap_or_else(|e| panic!("{id}: {}", e.text()));
        let own = read_wallet(&d).unwrap();
        assert!(same(&back, &own), "{id}");
        assert_eq!(
            first(&back, Network::Testnet),
            first(&own, Network::Testnet),
            "{id}"
        );
    }
    // Caravan builds sortedmulti only.
    for id in ["spending", "taproot-multisig", "inheritance"] {
        assert!(public_out(&kit(id), CARAVAN_ROW).is_empty(), "{id}");
    }
}

// Bitcoin Core before 29 (`docs/INTEROP.md` §5 I4).

/// Public files' row for Bitcoin Core before 29.
const CORE_BEFORE_29_ROW: u8 = 9;
/// Public files' Electrum row.
const ELECTRUM_ROW: u8 = 10;

/// The requests of an `importdescriptors` file: each one's `desc` and
/// `internal`, after checking the fields Core needs.
fn import_requests(text: &str) -> Vec<(String, bool)> {
    use faraday_intake::json::{self, Value};
    let v = json::parse(text).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            assert_eq!(r.get("active"), Some(&Value::Bool(true)), "{text}");
            assert_eq!(r.get("timestamp").and_then(Value::as_u64), Some(0));
            let range: Vec<u64> = r
                .get("range")
                .and_then(Value::as_array)
                .unwrap()
                .iter()
                .filter_map(Value::as_u64)
                .collect();
            assert_eq!(range, [0, 999]);
            let internal = match r.get("internal") {
                Some(Value::Bool(b)) => *b,
                other => panic!("internal: {other:?}"),
            };
            (
                r.get("desc").and_then(Value::as_str).unwrap().to_string(),
                internal,
            )
        })
        .collect()
}

#[test]
fn the_import_for_core_before_29_is_the_receive_and_change_descriptors_core_itself_lists() {
    // Core 31's `listdescriptors` reads as its BIP-84 wallet; the file for
    // Core before 29 holds the two descriptors Core listed for it,
    // checksums and all.
    let core = file("core/listdescriptors.json");
    let w = read_wallet(&core).unwrap();
    let out = public_out(&w.to_descriptor_checksummed(), CORE_BEFORE_29_ROW);
    let [(name, text)] = &out[..] else {
        panic!("{out:?}")
    };
    assert_eq!(name, "test-bitcoin-core-before-29.json");
    let requests = import_requests(text);
    let listed = |internal: bool| {
        let v = faraday_intake::json::parse(&core).unwrap();
        v.get("descriptors")
            .and_then(faraday_intake::json::Value::as_array)
            .unwrap()
            .iter()
            .filter(|d| d.get("internal") == Some(&faraday_intake::json::Value::Bool(internal)))
            .filter_map(|d| d.get("desc").and_then(faraday_intake::json::Value::as_str))
            .find(|d| d.starts_with("wpkh("))
            .unwrap()
            .to_string()
    };
    assert_eq!(
        requests,
        [(listed(false), false), (listed(true), true)],
        "{text}"
    );
}

#[test]
fn the_two_descriptors_for_core_before_29_read_back_as_the_wallet_each_with_a_valid_checksum() {
    for id in [
        "savings",
        "spending",
        "taproot",
        "nested",
        "legacy",
        "vault-nested",
        "vault-legacy",
        "taproot-multisig",
        "inheritance",
        "tree",
    ] {
        let d = kit(id);
        let out = public_out(&d, CORE_BEFORE_29_ROW);
        let [(_, text)] = &out[..] else {
            panic!("{id}: {out:?}")
        };
        let requests = import_requests(text);
        assert_eq!(
            requests.iter().map(|(_, i)| *i).collect::<Vec<_>>(),
            [false, true],
            "{id}"
        );
        for (desc, _) in &requests {
            assert!(!desc.contains('<'), "{id}: {desc}");
            assert!(osk_bip::descriptor::verify_checksum(desc), "{id}: {desc}");
        }
        let own = read_wallet(&d).unwrap().to_descriptor();
        for ((desc, _), chain) in requests.iter().zip(["0", "1"]) {
            assert_eq!(
                desc.split('#').next().unwrap(),
                own.replace("<0;1>", chain),
                "{id}"
            );
        }
        // Read back as a receive and a change descriptor: `miniscript`,
        // which reads such a pair here, has no `sortedmulti_a`.
        if id != "taproot-multisig" {
            let both = format!("{}\n{}", requests[0].0, requests[1].0);
            let back = read_wallet(&both).unwrap_or_else(|e| panic!("{id}: {}", e.text()));
            assert!(same(&back, &read_wallet(&d).unwrap()), "{id}");
        }
    }
}

// Electrum wallet files (`docs/INTEROP.md` §5 I6).

#[test]
fn sparrows_electrum_singlesig_file_loads_as_its_nested_segwit_key() {
    let w = wallet("sparrow/electrum-singlesig-wallet.json");
    let ypub = file("sparrow/electrum-singlesig-wallet.json")
        .split('"')
        .find(|t| t.starts_with("ypub"))
        .unwrap()
        .to_string();
    let plain = faraday_intake::slip132::decode(&ypub).unwrap().plain;
    assert_eq!(
        w.to_descriptor(),
        format!("sh(wpkh([ab543c67/49h/0h/0h]{plain}/<0;1>/*))")
    );
}

#[test]
fn sparrows_electrum_multisig_file_is_the_coldcards_own_setup_file_its_fingerprints_little_endian()
{
    // Four Coldcards, each named in the file by `ckcc_xfp`, a decimal
    // read little-endian: 1130956047 is 0F056943, the Coldcard that wrote
    // the setup file of the same wallet.
    let electrum = wallet("sparrow/electrum-multisig-wallet.json");
    let coldcard = wallet("sparrow/cc-multisig-export-2.txt");
    let d = electrum.to_descriptor();
    assert!(d.starts_with("sh(wsh(sortedmulti(2,"), "{d}");
    for fp in ["0f056943", "6ba6cfd0", "747b698e", "7bb026be"] {
        assert!(d.contains(&format!("[{fp}/48h/1h/0h/1h]")), "{fp}: {d}");
    }
    assert!(same(&electrum, &coldcard));
    assert_eq!(
        first(&electrum, Network::Testnet),
        first(&coldcard, Network::Testnet)
    );
}

#[test]
fn blue_wallets_electrum_file_loads_as_its_native_segwit_key_at_its_root_fingerprint() {
    let text = file("bluewallet/skeleton-electrum-hex-only.txt");
    let w = wallet("bluewallet/skeleton-electrum-hex-only.txt");
    let zpub = text.split('"').find(|t| t.starts_with("zpub")).unwrap();
    let plain = faraday_intake::slip132::decode(zpub).unwrap().plain;
    assert_eq!(
        w.to_descriptor(),
        format!("wpkh([b616be56/84h/0h/0h]{plain}/<0;1>/*)")
    );
}

/// An Electrum file around BIP-84's published account key, with `origin`
/// as the keystore's fingerprint fields.
fn electrum_bip84(origin: &str) -> String {
    format!(
        "{{\"keystore\": {{\"type\": \"bip32\", \"xpub\": \"{BIP84_ZPUB}\"{origin}}}, \"wallet_type\": \"standard\", \"use_encryption\": false, \"seed_version\": 17}}"
    )
}

#[test]
fn an_electrum_file_of_bip_84s_account_has_bip_84s_first_address_however_its_origin_is_written() {
    // 73c5da0a, BIP-84's master fingerprint, as `root_fingerprint` and
    // as Coldcard's little-endian `ckcc_xfp`.
    let derivation = ", \"derivation\": \"m/84'/0'/0'\"";
    let le = u32::from_le_bytes([0x73, 0xc5, 0xda, 0x0a]);
    for origin in [
        format!("{derivation}, \"root_fingerprint\": \"73c5da0a\""),
        format!("{derivation}, \"ckcc_xfp\": {le}"),
    ] {
        let w = read_wallet(&electrum_bip84(&origin)).unwrap();
        assert!(
            w.to_descriptor()
                .starts_with("wpkh([73c5da0a/84h/0h/0h]xpub")
        );
        assert_eq!(first(&w, Network::Mainnet), BIP84_FIRST);
    }
    // With neither, a single key loads with no origin, at the same
    // address: upstream holds a single key without one.
    let bare = read_wallet(&electrum_bip84("")).unwrap();
    assert!(bare.keys()[0].fingerprint().is_none());
    assert_eq!(first(&bare, Network::Mainnet), BIP84_FIRST);
}

#[test]
fn an_electrum_file_holding_a_private_key_is_a_secret_and_its_origin_less_multisig_is_refused() {
    use faraday_core::secrets::Exposure;
    let path = "bluewallet/fromsparrow-electrum.json";
    let text = file(path);
    let kind = classify(path, text.as_bytes());
    assert_eq!(kind, FileKind::SecretWallet);
    assert_eq!(kind.exposure(), Exposure::Secret);
    // Its keystores name no fingerprint or derivation: a multisig wallet
    // here holds no key without its origin, and the first is named.
    let e = read_wallet(&text).expect_err("no origins");
    assert!(e.text().contains("Zpub74sR"), "{}", e.text());
    assert!(!e.text().contains("prv"), "{}", e.text());
    // Its public keys are the keys of the same wallet's Specter file.
    let specter = wallet("bluewallet/fromsparrow-specter.json");
    let keys: Vec<String> = text
        .split('"')
        .filter(|t| t.starts_with("Zpub"))
        .map(|k| faraday_intake::slip132::decode(k).unwrap().plain)
        .collect();
    let mut theirs: Vec<String> = specter
        .keys()
        .iter()
        .map(|k| k.xpub().to_string())
        .collect();
    let mut ours = keys.clone();
    theirs.sort();
    ours.sort();
    assert_eq!(ours, theirs);
}

#[test]
fn an_encrypted_electrum_file_is_refused_as_encrypted() {
    let text = file("sparrow/electrum-singlesig-wallet.json");
    for encrypted in [
        text.replace("\"use_encryption\": false", "\"use_encryption\": true"),
        // Electrum's whole-file encryption: base64 of its `BIE1` envelope.
        "QklFMQOm0k8xWLe5KvvT1W9t3w3JY0oYxVqVZ6a0r7c9Qw==".to_string(),
    ] {
        let e = read_wallet(&encrypted).expect_err("encrypted");
        assert_eq!(e.text(), "Not a wallet: This Electrum file is encrypted");
    }
}

#[test]
fn faradays_electrum_file_reads_back_as_the_same_wallet_with_slip_132_keys_and_no_private_key() {
    for (id, prefix) in [
        ("spending", "vpub"),
        ("nested", "upub"),
        ("legacy", "tpub"),
        ("savings", "Vpub"),
        ("vault-nested", "Upub"),
        ("vault-legacy", "tpub"),
    ] {
        let d = kit(id);
        let out = public_out(&d, ELECTRUM_ROW);
        let [(name, text)] = &out[..] else {
            panic!("{id}: {out:?}")
        };
        assert_eq!(name, "test-electrum.json");
        assert!(
            text.contains(&format!("\"xpub\": \"{prefix}")),
            "{id}: {text}"
        );
        assert!(text.contains("\"xprv\": null"), "{id}");
        assert_eq!(classify(name, text.as_bytes()), FileKind::Wallet, "{id}");
        let back = read_wallet(text).unwrap_or_else(|e| panic!("{id}: {}", e.text()));
        let own = read_wallet(&d).unwrap();
        assert!(same(&back, &own), "{id}");
        assert_eq!(
            first(&back, Network::Testnet),
            first(&own, Network::Testnet),
            "{id}"
        );
    }
    // Not Taproot, miniscript or an unsorted multisig.
    for id in ["taproot", "taproot-multisig", "inheritance", "tree"] {
        assert!(public_out(&kit(id), ELECTRUM_ROW).is_empty(), "{id}");
    }
}

#[test]
fn a_psbt_electrum_builds_from_a_bare_zpub_is_not_signed_here() {
    // Electrum given an account key alone puts that key's own fingerprint
    // in the PSBT, where the seed's master fingerprint belongs: no key
    // here matches it, and nothing is signed. The same spend with the
    // key's origin signs.
    use faraday_core::testkit::{self, Kit};
    let rooted = testkit::key(0, "m/84'/1'/0'");
    let account = rooted.split(']').nth(1).unwrap().to_string();
    let own: osk_bip::bitcoin::bip32::Xpub = account.parse().unwrap();
    let kit_of = |descriptor: String| Kit {
        id: "spending",
        name: "Spending",
        descriptor,
        cosigner: None,
        fee: 1_410,
    };
    let sign = |psbt: &mut _| {
        let mut session = testkit::session();
        session
            .add_words(
                &testkit::test_words(testkit::TEST_SEEDS[0].0),
                "Test key 1",
                None,
            )
            .unwrap();
        session
            .add_wallet(
                "Spending",
                &format!("wpkh({rooted}/<0;1>/*)"),
                "Test wallet",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap();
        session.sign(psbt, [0; 32], &mut None).unwrap_or_default()
    };
    let mut bare = testkit::unsigned(&kit_of(format!(
        "wpkh([{}]{account}/<0;1>/*)",
        own.fingerprint()
    )))
    .unwrap();
    assert!(sign(&mut bare).is_empty());
    let mut with_origin = testkit::unsigned(&kit_of(format!("wpkh({rooted}/<0;1>/*)"))).unwrap();
    assert_eq!(sign(&mut with_origin).len(), 1);
}
