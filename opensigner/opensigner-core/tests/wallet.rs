//! A multisig wallet registered for the session (`docs/PLANNING.md` §15
//! item 8): what the review of a scanned BIP-388 policy says, what using
//! it changes on Keys and in the Sign flow, and what forgetting it or
//! wiping the session leaves.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::sign::{Stage, Step};
use opensigner_core::verify::AddressResult;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_psbt::{OutputKind, Psbt};
use osk_shell_api::{Event, FileKind};
use osk_ui::widgets::Icon;

/// The 2-of-3 wallet of the loaded key and two fixed cosigners, and a
/// transaction of it with change of its own (`just psbt-fixtures`).
const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");
const PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.psbt");
/// The same wallet as the text file a coordinator exports for signers.
const CONFIG: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.txt");

fn policy() -> WalletPolicy {
    WalletPolicy::parse(POLICY).expect("the committed policy")
}

fn psbt_bytes() -> Vec<u8> {
    Psbt::parse_base64(PSBT.trim())
        .expect("the committed PSBT")
        .to_bytes()
}

/// The loaded test key on the wallet's chain.
fn with_key(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    assert_eq!(h.app.network(), Network::Regtest);
}

/// Home › Scan, the policy read from a file, and the review it opens.
fn read_policy(h: &mut Harness) {
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: POLICY.as_bytes().to_vec(),
    });
}

/// Add › "Load a wallet", and `bytes` read from a file there.
fn load_wallet_from_wallets(h: &mut Harness, bytes: &[u8]) {
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: bytes.to_vec(),
    });
}

/// Whether `address` is one of the strings on screen. A long string is
/// drawn elided and chunked in fours (§4.5), so the row is matched by
/// the head and tail it shows.
fn shows_address(h: &Harness, address: &str) -> bool {
    h.app.texts().iter().any(|t| {
        let flat = t.replace(' ', "");
        match flat.split_once('\u{2026}') {
            Some((head, tail)) => {
                !head.is_empty() && address.starts_with(head) && address.ends_with(tail)
            }
            None => flat == address,
        }
    })
}

/// The label the wallet's row and its review carry: "2 of 3".
fn quorum() -> String {
    strings::fill(strings::EN.wallet_quorum, &["2", "3"])
}

#[test]
fn a_scanned_policy_is_reviewed_by_its_quorum_keys_and_checksum() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    read_policy(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    assert!(says(&quorum()), "the quorum: {texts:?}");
    assert!(says(strings::EN.inspect_checksum_ok), "{texts:?}");
    for key in policy().keys() {
        // A BIP-388 policy writes an origin on every key, so every
        // cosigner has a master to state.
        let fingerprint = key.fingerprint().expect("a cosigner has an origin");
        assert!(
            texts.iter().any(|t| *t == fingerprint.to_string()),
            "cosigner {fingerprint} is on the review: {texts:?}"
        );
    }
    // §4.4's glyph marks the cosigner this device holds the key for,
    // and the eye the two it only watches.
    let glyphs: Vec<_> = h
        .app
        .row_glyphs()
        .into_iter()
        .filter(|(label, _)| label == strings::EN.sign_key_row)
        .map(|(_, icon)| icon)
        .collect();
    assert_eq!(
        glyphs.iter().filter(|i| **i == Icon::Wallet).count(),
        1,
        "the loaded key is marked as one of the cosigners: {glyphs:?}"
    );
    assert_eq!(glyphs.iter().filter(|i| **i == Icon::Eye).count(), 2);
    assert!(says(strings::EN.wallet_use), "the way to use it: {texts:?}");
}

#[test]
fn using_a_wallet_lists_it_on_wallets_and_forgetting_it_removes_it() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    read_policy(&mut h);
    // Using a wallet read at the scanner lands on its page, with
    // Wallets behind it.
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    let row = ids::at(ids::WALLETS_POLICY_ROW_BASE, 0);
    assert!(
        h.app.rect_of(row).is_some(),
        "the wallet is a row on Wallets"
    );
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains(&quorum())),
        "the row names the quorum: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| *t == policy().checksum()),
        "the checksum is on the wallet's page, not on Wallets: {texts:?}"
    );

    // The row opens the wallet's menu, whose Keys row is the Keys list
    // filtered to its members, and Forget on the menu is the way off
    // the list.
    h.tap(row);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    let texts = h.app.texts();
    for key in policy().keys() {
        let fingerprint = key.fingerprint().expect("a cosigner has an origin");
        assert!(
            texts.iter().any(|t| *t == fingerprint.to_string()),
            "cosigner {fingerprint} is a row: {texts:?}"
        );
    }
    assert!(
        !texts.iter().any(|t| t == strings::EN.wallet_forget),
        "Forget is a row of the menu, not of the review: {texts:?}"
    );
    h.tap(ids::BACK);
    h.tap(ids::WALLET_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    assert!(
        h.app.rect_of(row).is_none(),
        "a wallet forgotten is off the list"
    );
}

#[test]
fn a_wallet_in_use_verifies_the_change_of_its_own_transaction() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);

    // Without the wallet, the change output is a claim the device cannot
    // check, and the summary says so. A PSBT signs with the loaded key
    // and needs no wallet (§16.104 rule 5).
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(Step::Summary)));
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(matches!(
        insp.outputs[1].kind,
        OutputKind::UnverifiedChange { .. }
    ));
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t == strings::EN.sign_change_unverified),
        "{texts:?}"
    );
    h.tap(ids::BACK);

    // With it, the same output is change of that wallet and the input is
    // one the loaded key signs.
    read_policy(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.open_policy(0);
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(),
    });
    let insp = h.app.sign_inspection().expect("inspected");
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0
        }
    );
    assert!(!insp.has_danger(), "{:?}", insp.warnings);
    assert_eq!(insp.change_total.to_sat(), 39_000);
    assert!(insp.inputs[0].is_ours);
    assert_eq!(insp.participating_keys.len(), 1);
    let texts = h.app.texts();
    assert!(
        !texts
            .iter()
            .any(|t| t == strings::EN.sign_change_unverified),
        "{texts:?}"
    );
}

#[test]
fn verify_answers_for_an_address_of_a_wallet_in_use() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    read_policy(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    let address = policy()
        .address_at(Network::Regtest, false, 2)
        .unwrap()
        .to_string();
    h.open_policy(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_TYPE);
    h.type_text(&address);
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Wallet {
            wallet: 0,
            change: false,
            index: 2
        })
    );
}

#[test]
fn a_policy_that_does_not_check_out_is_refused_where_it_was_read() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    // The same wallet as a descriptor, with one character of its
    // checksum changed.
    let mut descriptor = policy().to_descriptor_checksummed();
    descriptor.pop();
    descriptor.push('q');
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: descriptor.into_bytes(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Scan, "still at the scanner");
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.scan_reason_policy),
        "{texts:?}"
    );
}

#[test]
fn wipe_forgets_the_wallets_with_the_keys() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    read_policy(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.open_settings();
    h.tap(ids::SETTINGS_WIPE_ROW);
    h.hold(ids::SETTINGS_WIPE);
    assert!(h.app.fingerprints().is_empty(), "the keys are gone");
    if h.app.rect_of(ids::SETTINGS_WIPED_DONE).is_some() {
        h.tap(ids::SETTINGS_WIPED_DONE);
    }
    h.go_home();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "no wallet survives the wipe"
    );
}

/// "Load a wallet" is offered with and without a key: as an empty-state
/// row on Wallets, and as a row of the Add a wallet menu once something
/// is registered.
#[test]
fn a_wallet_can_be_loaded_with_and_without_a_key() {
    let mut h = Harness::new(PANEL);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.open_wallets();
    assert!(
        h.app.reveal(ids::WALLETS_LOAD).is_some(),
        "the row is there before any key is loaded"
    );
    assert!(
        h.app.texts().iter().any(|t| t == strings::EN.wallet_load),
        "{:?}",
        h.app.texts()
    );

    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    h.go_home();
    h.open_wallets();
    h.tap(ids::WALLETS_ADD);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
    assert!(
        h.app.reveal(ids::WALLETS_LOAD).is_some(),
        "and behind Add a wallet once a wallet is listed"
    );
}

#[test]
fn a_wallet_loaded_from_keys_is_reviewed_and_used_from_its_own_menu() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    load_wallet_from_wallets(&mut h, POLICY.as_bytes());
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    assert!(
        h.app.texts().iter().any(|t| t == strings::EN.wallet_use),
        "{:?}",
        h.app.texts()
    );

    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains(&quorum())),
        "the menu is titled with the wallet: {texts:?}"
    );
    for row in [
        ids::WALLET_ADDRESSES,
        ids::WALLET_EXPORT,
        ids::WALLET_KEYS,
        ids::WALLET_FORGET,
    ] {
        assert!(h.app.reveal(row).is_some(), "row {}", row.0);
    }
    h.tap(ids::BACK);
    assert_eq!(
        h.app.screen(),
        ScreenKind::Wallets,
        "the way back is Wallets"
    );

    // The row on Wallets opens the same page.
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

#[test]
fn the_wallet_menu_lists_the_addresses_of_the_wallet() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    load_wallet_from_wallets(&mut h, POLICY.as_bytes());
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);

    let receive = policy()
        .address_at(Network::Regtest, false, 0)
        .unwrap()
        .to_string();
    assert!(
        shows_address(&h, &receive),
        "the first receive address: {:?}",
        h.app.texts()
    );

    // The change chain is the one the wallet's own transaction pays its
    // change to, which the PSBT fixture states independently.
    let psbt = Psbt::parse_base64(PSBT.trim()).expect("the committed PSBT");
    let change = psbt.unsigned_tx().output[1].script_pubkey.clone();
    let expected = osk_bip::bitcoin::Address::from_script(
        &change,
        osk_bip::bitcoin::Network::from(Network::Regtest),
    )
    .expect("an address")
    .to_string();
    h.tap(ids::ADDR_CHANGE);
    assert!(
        shows_address(&h, &expected),
        "the change address the transaction pays to: {:?}",
        h.app.texts()
    );

    // One address opens on a screen of its own, with its code.
    h.tap(ids::at(ids::ADDR_ROW_BASE, 0));
    assert_eq!(h.app.qr_visible(), Some(true));
    assert!(shows_address(&h, &expected), "{:?}", h.app.texts());
}

#[test]
fn the_wallet_export_offers_the_descriptor_and_the_policy() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    load_wallet_from_wallets(&mut h, POLICY.as_bytes());
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains(&policy().checksum())),
        "the descriptor is shown with its checksum: {:?}",
        h.app.texts()
    );

    // The policy is the other format, and it is the two-part text.
    h.tap(ids::EXPORT_FORMAT);
    let row = String::from(strings::EN.export_policy);
    assert!(h.app.texts().contains(&row), "{:?}", h.app.texts());
    h.tap(ids::at(ids::PICK_BASE, 3));
    h.tap(ids::PICK_CONTINUE);
    h.tap(ids::EXPORT_TEXT);
    let whole: String = h.app.texts().join(" ");
    assert!(
        whole.contains("sortedmulti"),
        "the policy template, whole: {whole}"
    );
    h.tap(ids::COMPARE_DONE);

    // The QR screen opens over the menu.
    h.tap(ids::EXPORT_SHOW);
    assert_eq!(h.app.qr_visible(), Some(true));
}

#[test]
fn a_coordinators_text_config_loads_the_same_wallet_as_the_policy() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    load_wallet_from_wallets(&mut h, CONFIG.as_bytes());
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    let texts = h.app.texts();
    assert!(texts.iter().any(|t| t == &quorum()), "{texts:?}");
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_EXPORT);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains(&policy().checksum())),
        "the same wallet as the policy fixture: {:?}",
        h.app.texts()
    );
}

#[test]
fn forgetting_a_wallet_from_its_menu_removes_its_row() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    load_wallet_from_wallets(&mut h, POLICY.as_bytes());
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "the row is gone"
    );
}

#[test]
fn a_wallet_works_with_no_key_loaded() {
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    load_wallet_from_wallets(&mut h, POLICY.as_bytes());
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    assert!(
        !h.app
            .texts()
            .iter()
            .any(|t| t == osk_ui::components::OutputBadge::Mine.label()),
        "no cosigner is this device's: {:?}",
        h.app.texts()
    );
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_ADDRESSES);
    let receive = policy()
        .address_at(Network::Regtest, false, 0)
        .unwrap()
        .to_string();
    assert!(
        shows_address(&h, &receive),
        "the wallet still derives its addresses: {:?}",
        h.app.texts()
    );
}

/// The row of "Which format?" that is Bitcoin Core's import file, which
/// is its place in `ExportFormat::ALL`.
const CORE_IMPORT_ROW: usize = 6;

/// A wallet's Export, at the Core import format, with `rescan` chosen on
/// the Choice that format opens (0 is the start, 1 is now).
fn core_import(h: &mut Harness, rescan: usize) {
    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_FORMAT);
    h.tap(ids::at(ids::PICK_BASE, CORE_IMPORT_ROW));
    h.tap(ids::PICK_CONTINUE);
    h.tap(ids::at(ids::PICK_BASE, rescan));
    h.tap(ids::PICK_CONTINUE);
}

/// The bytes the last "Save to file" handed the shell, and the name it
/// was offered under.
fn written(h: &Harness) -> (String, Vec<u8>) {
    let mut writes = h.seen.iter().filter_map(|c| match c {
        osk_shell_api::Command::WriteFile {
            name_hint, bytes, ..
        } => Some((name_hint.clone(), bytes.clone())),
        _ => None,
    });
    let one = writes.next().expect("a write");
    assert!(writes.next().is_none(), "one WriteFile");
    one
}

/// The wallet as a file Bitcoin Core takes: `importdescriptors` over the
/// multipath descriptor, active, the first thousand addresses of each
/// chain, and the rescan point the Choice asked for
/// (`docs/PLANNING.md` §16.114).
#[test]
fn the_core_import_file_is_the_wallet_as_importdescriptors_json() {
    for (rescan, timestamp) in [
        (0usize, serde_json::json!(0)),
        (1, serde_json::json!("now")),
    ] {
        let mut h = Harness::new(PANEL);
        with_key(&mut h);
        load_wallet_from_wallets(&mut h, POLICY.as_bytes());
        h.tap(ids::INSPECT_USE_WALLET);
        core_import(&mut h, rescan);
        h.tap(ids::EXPORT_SAVE);
        let (name, bytes) = written(&h);
        assert_eq!(name, format!("{}-core-import.json", policy().checksum()));

        let value: serde_json::Value =
            serde_json::from_slice(&bytes).expect("the file is JSON Core can read");
        let requests = value.as_array().expect("an array of requests");
        assert_eq!(requests.len(), 1, "one multipath descriptor, one request");
        let request = &requests[0];
        assert_eq!(
            request["desc"].as_str(),
            Some(policy().to_descriptor_checksummed().as_str())
        );
        assert_eq!(request["active"], serde_json::json!(true));
        assert_eq!(request["range"], serde_json::json!([0, 999]));
        assert_eq!(request["timestamp"], timestamp);
        // One multipath descriptor covers both chains, so Core is not
        // told which of them this is.
        assert!(request.get("internal").is_none(), "an internal flag");
    }
}

/// The file the import format writes is offered by the wallet's name
/// where the wallet has one.
#[test]
fn a_named_wallets_import_file_carries_its_name() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    load_wallet_from_wallets(&mut h, POLICY.as_bytes());
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_NAME);
    h.type_text("Cold store");
    h.key(osk_shell_api::Key::Enter);
    core_import(&mut h, 1);
    h.tap(ids::EXPORT_SAVE);
    let (name, _) = written(&h);
    assert_eq!(name, "Cold store-core-import.json");
}
