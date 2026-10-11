//! A threshold wallet, loaded from its group record the way any other
//! wallet is loaded (`docs/PLANNING.md` §16.103): what its review says,
//! what its menu is called, the addresses it lists, what it exports, and
//! what a restart and Forget leave.
//!
//! The wallet is the committed 2-of-3 regtest group,
//! `tools/vectors/psbt/wallet-threshold-regtest.record`. This device
//! holds no share of it, so every row that names a participant carries
//! the eye.

mod common;

use std::str::FromStr;

use common::{ABANDON, Element, Harness, PANEL, PIN, SECURE_PHONE};
use opensigner_core::verify::AddressResult;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::Network;
use osk_bip::miniscript::Descriptor;
use osk_bip::miniscript::descriptor::DescriptorPublicKey;
use osk_bip::policy::WalletPolicy;
use osk_bip::threshold::ThresholdRecord;
use osk_shell_api::{Event, FileKind};

const RECORD: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest.record");

fn record() -> ThresholdRecord {
    ThresholdRecord::parse(RECORD).expect("the committed record")
}

/// "2 of 3", the wallet's own threshold.
fn quorum() -> String {
    strings::fill(strings::EN.wallet_quorum, &["2", "3"])
}

/// "Key 1 of 3", as the review of a scanned record counts its members.
fn share_label(i: usize) -> String {
    strings::fill(strings::EN.wallet_member, &[&format!("{}", i + 1), "3"])
}

/// The fingerprint of member `i`'s public share, which is what the
/// wallet's Keys review names it by.
fn member_print(i: usize) -> String {
    record()
        .share_fingerprint(i)
        .expect("a present member")
        .to_string()
}

/// The wallet's first three receive addresses, derived here from the
/// record's own descriptor line through `miniscript`.
fn receive() -> Vec<String> {
    let descriptor = Descriptor::<DescriptorPublicKey>::from_str(&record().descriptor())
        .expect("the record's descriptor");
    let chain = descriptor
        .into_single_descriptors()
        .expect("both chains")
        .swap_remove(0);
    let secp = osk_psbt::bitcoin::secp256k1::Secp256k1::verification_only();
    (0..3u32)
        .map(|index| {
            chain
                .clone()
                .at_derivation_index(index)
                .expect("index")
                .derived_descriptor(&secp)
                .expect("derivation")
                .address(osk_psbt::bitcoin::Network::Regtest)
                .expect("an address")
                .to_string()
        })
        .collect()
}

/// A device on regtest with the record read at the wallet scanner.
fn load_wallet(h: &mut Harness) {
    h.set_network(3);
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: RECORD.as_bytes().to_vec(),
    });
}

/// Whether `address` is one of the strings on screen, which a long one
/// reaches elided and chunked in fours (§4.5).
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

#[test]
fn a_threshold_wallet_is_reviewed_as_its_threshold_and_its_shares() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Inspect);

    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    assert!(says(strings::EN.wallet_threshold), "Threshold: {texts:?}");
    assert!(says(&quorum()), "how many of how many: {texts:?}");
    assert!(says(strings::EN.script_taproot), "the script: {texts:?}");
    assert!(says(strings::EN.inspect_checksum_ok), "{texts:?}");
    for i in 0..3 {
        assert!(says(&share_label(i)), "share {i}'s row: {texts:?}");
        let fingerprint = record().share_fingerprint(i).expect("a present share");
        assert!(
            says(&fingerprint.to_string()),
            "share {i}'s fingerprint: {texts:?}"
        );
    }
    assert!(says(strings::EN.wallet_use), "the way to use it: {texts:?}");
}

#[test]
fn the_threshold_wallet_menu_is_named_by_its_threshold_and_lists_its_addresses() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| {
            t.contains(strings::EN.wallet_threshold)
                && t.contains(&quorum())
                && t.contains(strings::EN.script_taproot)
        }),
        "the menu is titled with the wallet: {texts:?}"
    );
    // This device holds no key of the group, so there is no way to sign
    // here, and its members are the Keys row like any other wallet's.
    assert!(h.app.rect_of(ids::WALLET_SIGN).is_none(), "no Sign row");
    assert!(
        texts.iter().any(|t| t == strings::EN.wallet_keys),
        "the Keys row: {texts:?}"
    );
    for id in [
        ids::WALLET_ADDRESSES,
        ids::WALLET_CHECK,
        ids::WALLET_EXPORT,
        ids::WALLET_KEYS,
        ids::WALLET_FORGET,
    ] {
        assert!(h.app.rect_of(id).is_some(), "the page's rows");
    }

    // Keys is the Keys screen filtered to the record's members, named
    // by the fingerprint of each public share (§16.104 rule 3).
    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    let texts = h.app.texts();
    for i in 0..3 {
        assert!(texts.contains(&member_print(i)), "{texts:?}");
    }
    h.tap(ids::BACK);

    h.tap(ids::WALLET_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.script_taproot),
        "the script type is a fact of the list: {:?}",
        h.app.texts()
    );
    for address in receive() {
        assert!(
            shows_address(&h, &address),
            "{address} is on the list: {:?}",
            h.app.texts()
        );
    }

    // The change chain is the wallet's other chain, and not the receive
    // one.
    h.tap(ids::ADDR_CHANGE);
    let policy = WalletPolicy::parse_any(RECORD).expect("the record");
    let change = policy
        .address_at(Network::Regtest, true, 0)
        .unwrap()
        .to_string();
    assert!(change != receive()[0], "the chains differ");
    assert!(shows_address(&h, &change), "{:?}", h.app.texts());
}

#[test]
fn verify_answers_for_an_address_of_a_threshold_wallet_in_use() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.open_policy(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_TYPE);
    h.type_text(&receive()[2]);
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Wallet {
            wallet: 0,
            change: false,
            index: 2
        })
    );

    // An address of another wallet is not one of this wallet's.
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.open_policy(0);
    h.tap(ids::WALLET_CHECK);
    h.tap(ids::SCAN_TYPE);
    h.type_text("bcrt1qcn77knrrdsk3wpjmndhe4xuntfqq3ft7xtr0yu23grfeqgsxkn2qjk0fz5");
    h.key(osk_shell_api::Key::Enter);
    assert!(
        !matches!(h.app.verify_result(), Some(AddressResult::Wallet { .. })),
        "{:?}",
        h.app.verify_result()
    );
}

#[test]
fn the_threshold_wallet_exports_its_descriptor_and_its_record() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    let policy = WalletPolicy::parse_any(RECORD).expect("the record");
    assert!(
        h.app.texts().iter().any(|t| t.contains(&policy.checksum())),
        "the descriptor row names its checksum: {:?}",
        h.app.texts()
    );

    h.tap(ids::EXPORT_FORMAT);
    h.tap(ids::at(ids::PICK_BASE, 3));
    h.tap(ids::PICK_CONTINUE);
    h.tap(ids::EXPORT_TEXT);
    let whole: String = h.app.texts().join(" ").replace(' ', "");
    for line in RECORD.trim().lines() {
        assert!(
            whole.contains(&line.replace(' ', "")),
            "the record, whole: {line}"
        );
    }
}

/// A device that keeps keys keeps the threshold wallet in use with them:
/// it comes back after a restart, and Forget takes it away for good.
#[test]
fn a_threshold_wallet_in_use_comes_back_after_a_restart() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "the shell is holding the blob");
    h.go_home();
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);

    let row = ids::at(ids::WALLETS_POLICY_ROW_BASE, 0);
    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    h.open_wallets();
    assert!(
        h.app.rect_of(row).is_some(),
        "the wallet came back with the key: {:?}",
        h.app.texts()
    );
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with(strings::EN.wallet_kind_threshold)),
        "its row says what it is: {texts:?}"
    );

    h.tap(row);
    h.tap(ids::WALLET_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    assert!(h.app.rect_of(row).is_none(), "the row is gone");
    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    h.open_wallets();
    assert!(h.app.rect_of(row).is_none(), "and forgotten on the device");
}
