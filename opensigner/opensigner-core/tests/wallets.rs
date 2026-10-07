//! Wallets: the list a person reads to find what turns keys into
//! addresses, the glyph on each row that says whether this device can
//! sign for it, and the menu a row opens.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::verify::AddressResult;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::ScriptType;
use osk_shell_api::{Event, FileKind, Key};
use osk_ui::widgets::Icon;

/// The 2-of-3 wallet of the test key and two fixed cosigners
/// (`just psbt-fixtures`).
const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");

/// BIP-84's test vector: the `abandon … about` account key at
/// `m/84'/0'/0'` as SLIP-132 spells it, and the first receive and change
/// addresses the vector lists for it.
const ZPUB: &str = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
const RECEIVE_0: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";
const CHANGE_0: &str = "bc1q8c6fshw2dlwun7ekn9qwf37cu2rn755upcp6el";

/// The same account key as a plain `xpub`.
fn xpub() -> osk_bip::keys::Xpub {
    osk_bip::slip132::decode_xpub(ZPUB)
        .expect("a SLIP-132 key")
        .0
}

/// That account as a coordinator hands it over: one key, with its
/// origin, and nothing of this device in it.
fn descriptor() -> String {
    format!("wpkh([73c5da0a/84'/0'/0']{}/<0;1>/*)", xpub())
}

/// Wallets › "Scan a wallet", and `text` read from a file there.
fn load_wallet(h: &mut Harness, text: &str) {
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: text.as_bytes().to_vec(),
    });
}

/// The Wallets list, whatever screen the harness is on.
fn open_wallets(h: &mut Harness) {
    h.open_wallets();
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
}

/// Wallets › "Scan a wallet", and the policy read from a file there.
fn load_policy(h: &mut Harness) {
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: POLICY.as_bytes().to_vec(),
    });
    h.tap(ids::INSPECT_USE_WALLET);
}

/// With nothing registered Wallets is the two rows that start a flow
/// (§4.14). There is no sentence about emptiness.
#[test]
fn an_empty_list_is_the_start_rows_alone() {
    let mut h = Harness::new(PANEL);
    open_wallets(&mut h);
    for row in [ids::WALLETS_ADD, ids::WALLETS_LOAD] {
        assert!(h.app.rect_of(row).is_some(), "row {} starts a flow", row.0);
    }
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none()
    );
}

/// A wallet none of whose keys is loaded here carries the eye: its
/// addresses can be derived and compared, never spent.
#[test]
fn a_wallet_with_no_key_of_this_device_carries_the_eye() {
    let mut h = Harness::new(PANEL);
    load_policy(&mut h);
    open_wallets(&mut h);
    let glyphs = h.app.row_glyphs();
    let policy = glyphs
        .iter()
        .find(|(label, _)| label.starts_with(strings::EN.wallet_kind_multisig))
        .expect("the policy in use");
    assert_eq!(policy.1, Icon::Eye, "public keys only");
}

/// A loaded key is no wallet: Wallets stays empty until one is added by
/// hand, and the single-sig wallet added then carries the key glyph,
/// what kind of wallet it is as the label, and the fingerprint and the
/// script type as the line a person reads it by.
#[test]
fn a_single_sig_wallet_added_by_hand_is_a_row_with_the_key_glyph() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    open_wallets(&mut h);
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "a key alone is no wallet: {:?}",
        h.app.texts()
    );

    h.add_single_sig(0, 2);
    open_wallets(&mut h);
    let glyphs = h.app.row_glyphs();
    let single = glyphs
        .iter()
        .find(|(label, _)| label.starts_with(strings::EN.wallet_single))
        .expect("the loaded key's own wallet");
    assert_eq!(single.1, Icon::Wallet, "this device holds the key");
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("73c5da0a") && t.contains(strings::EN.script_segwit)),
        "the row is read by the fingerprint and the script: {texts:?}"
    );
}

/// A 2-of-3 whose keys include one of this device's carries the key
/// glyph, and its Keys review marks that key and no other.
#[test]
fn a_policy_with_a_key_of_this_device_carries_the_key_glyph() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    load_policy(&mut h);
    open_wallets(&mut h);
    let glyphs = h.app.row_glyphs();
    let policy = glyphs
        .iter()
        .find(|(label, _)| label.starts_with(strings::EN.wallet_kind_multisig))
        .expect("the policy in use");
    assert_eq!(policy.1, Icon::Wallet, "one of its keys is loaded here");

    // Its Keys review marks each cosigner the same way, and the loaded
    // key is the one with the key glyph.
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    let marks: Vec<Icon> = h
        .app
        .row_glyphs()
        .into_iter()
        .map(|(_, icon)| icon)
        .collect();
    assert_eq!(marks.len(), 3, "one row per key");
    assert_eq!(
        marks.iter().filter(|i| **i == Icon::Wallet).count(),
        1,
        "one of the three is this device's"
    );
    assert!(
        !h.app.texts().iter().any(|t| t == "MINE"),
        "the badge is for outputs and addresses, not keys"
    );
}

/// A key's addresses are its wallet's: the list the key menu used to
/// carry is now one tap inside Wallets, and derives the same run.
#[test]
fn a_single_sig_wallet_lists_the_addresses_of_its_key() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let expected = h
        .app
        .addresses(0, osk_bip::keys::ScriptType::NativeSegwit, false, 3)[0..3]
        .to_vec();
    h.open_single_sig(0);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.tap(ids::WALLET_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses);
    let texts = h.app.texts();
    for address in &expected {
        let head = &address[..8];
        assert!(
            texts.iter().any(|t| t.replace(' ', "").starts_with(head)),
            "{address} is in the list: {texts:?}"
        );
    }
}

/// Every wallet page is the same shape: the actions, the key or keys,
/// then Forget. A wallet over one key this device holds is the only one
/// that signs a message.
#[test]
fn a_wallet_page_carries_its_actions_its_keys_and_forget() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_single_sig(0);
    for row in [
        ids::WALLET_SIGN,
        ids::WALLET_SIGN_MESSAGE,
        ids::WALLET_ADDRESSES,
        ids::WALLET_CHECK,
        ids::WALLET_EXPORT,
        ids::WALLET_KEYS,
        ids::WALLET_FORGET,
    ] {
        assert!(h.app.reveal(row).is_some(), "row {}", row.0);
    }

    // A policy this device holds a key of signs transactions; a message
    // is signed by one key, so no policy carries that row.
    load_policy(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert!(h.app.reveal(ids::WALLET_SIGN).is_some());
    assert!(
        h.app.rect_of(ids::WALLET_SIGN_MESSAGE).is_none(),
        "a message is signed by one key"
    );
    for row in [
        ids::WALLET_ADDRESSES,
        ids::WALLET_CHECK,
        ids::WALLET_EXPORT,
        ids::WALLET_KEYS,
        ids::WALLET_FORGET,
    ] {
        assert!(h.app.reveal(row).is_some(), "row {}", row.0);
    }
}

/// A wallet config that arrives is reviewed, and the review leaves it
/// unused until "Add this wallet" is pressed.
#[test]
fn a_reviewed_wallet_is_not_a_wallet_in_use() {
    let mut h = Harness::new(PANEL);
    h.open_add_wallet(ids::WALLETS_LOAD);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: POLICY.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    let quorum = strings::fill(strings::EN.wallet_quorum, &["2", "3"]);
    let texts = h.app.texts();
    assert!(
        texts.contains(&quorum),
        "the review states the quorum: {texts:?}"
    );
    open_wallets(&mut h);
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "a review is not a use"
    );
}

/// A wallet is public data, so a descriptor over one key this device
/// does not hold is a wallet like any other: it is reviewed, used, and
/// derives the addresses the descriptor names.
#[test]
fn a_single_key_descriptor_is_a_wallet_this_device_watches() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h, &descriptor());
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);

    open_wallets(&mut h);
    let glyphs = h.app.row_glyphs();
    assert!(
        glyphs
            .iter()
            .any(|(label, icon)| label.starts_with(strings::EN.wallet_single)
                && *icon == Icon::Eye),
        "a single-sig wallet with the eye: {glyphs:?}"
    );
    assert!(
        h.app.texts().iter().any(|t| t.starts_with("73c5da0a")),
        "read by the master it comes from: {:?}",
        h.app.texts()
    );
    assert!(
        !h.app
            .texts()
            .iter()
            .any(|t| t.to_lowercase().contains("read-only") || t.to_lowercase().contains("watch")),
        "the glyph says it, the words do not: {:?}",
        h.app.texts()
    );

    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_ADDRESSES);
    let says = |h: &Harness, address: &str| {
        let texts = h.app.texts();
        assert!(
            texts
                .iter()
                .any(|t| t.replace(' ', "").starts_with(&address[..8])),
            "{address} is in the list: {texts:?}"
        );
    };
    says(&h, RECEIVE_0);
    h.tap(ids::ADDR_CHANGE);
    says(&h, CHANGE_0);
}

/// The same wallet on a device that holds its key carries the key glyph:
/// what the glyph says is whether this device can sign, and here it can.
#[test]
fn a_single_key_wallet_of_a_loaded_key_carries_the_key_glyph() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    load_wallet(&mut h, &descriptor());
    h.tap(ids::INSPECT_USE_WALLET);

    open_wallets(&mut h);
    let glyphs = h.app.row_glyphs();
    assert!(
        !glyphs.iter().any(|(_, icon)| *icon == Icon::Eye),
        "every wallet here is one this device holds a key of: {glyphs:?}"
    );
}

/// An extended public key on its own says nothing about what it pays to,
/// so the Choice asks. A SLIP-132 spelling answers it; an `xpub` does
/// not, and SegWit is the row the check starts on.
#[test]
fn a_bare_key_is_asked_what_it_pays_to() {
    let s = &strings::EN;
    for (key, want) in [
        (String::from(ZPUB), s.script_segwit),
        (
            osk_bip::slip132::encode_xpub(&xpub(), ScriptType::NestedSegwit),
            s.script_nested,
        ),
        (format!("{}", xpub()), s.script_segwit),
    ] {
        let mut h = Harness::new(PANEL);
        load_wallet(&mut h, &key);
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t == s.script_type_title),
            "the Choice asks for {key}: {texts:?}"
        );
        assert_eq!(
            h.app.checked_rows(),
            vec![String::from(want)],
            "what {key} pays to"
        );
    }
}

/// The wallet a bare key makes has no origin, so the review says the
/// master is unknown rather than inventing one, and the export has no
/// BIP-388 policy to offer, because BIP-388 writes an origin on every
/// key.
#[test]
fn a_wallet_with_no_origin_says_so_and_exports_only_a_descriptor() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h, &format!("{}", xpub()));
    h.tap(ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.wallet_origin_unknown),
        "the key row states what is missing: {texts:?}"
    );
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);

    h.tap(ids::WALLET_EXPORT);
    h.tap(ids::EXPORT_FORMAT);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.export_descriptor),
        "the descriptor is a form it has: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == s.export_policy),
        "the BIP-388 policy is not: {texts:?}"
    );
}

/// Verify answers for the addresses of every wallet in use, whether or
/// not this device could spend from it.
#[test]
fn verify_answers_for_an_address_of_a_wallet_with_no_key_here() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h, &descriptor());
    h.tap(ids::INSPECT_USE_WALLET);

    h.open_policy(0);
    h.tap(ids::WALLET_CHECK);
    h.tap(ids::SCAN_TYPE);
    h.type_text(CHANGE_0);
    h.key(Key::Enter);
    assert_eq!(
        h.app.verify_result(),
        Some(&AddressResult::Wallet {
            wallet: 0,
            change: true,
            index: 0,
        })
    );
}

/// A wallet this device holds no private key of is read-only: its page
/// has no Sign row at all.
#[test]
fn a_read_only_wallet_has_no_sign_rows() {
    let mut h = Harness::new(PANEL);
    load_policy(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    for row in [ids::WALLET_SIGN, ids::WALLET_SIGN_MESSAGE] {
        assert!(
            h.app.rect_of(row).is_none(),
            "row {} on a wallet with no key of this device",
            row.0
        );
    }
    assert!(h.app.reveal(ids::WALLET_ADDRESSES).is_some());
}
