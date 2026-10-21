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
