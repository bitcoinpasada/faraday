//! Miniscript and taproot-tree wallets in the product: loading one,
//! what its review says, its row on Home, its addresses, its export,
//! forgetting it, and what the Sign review makes of a spend of it.
//!
//! The wallets are `tools/vectors/psbt/wallet-liana.policy` — one key
//! now, a recovery key after 52 560 blocks — and
//! `tools/vectors/psbt/wallet-tree.policy`, a key path and two leaves.
//! Both are regtest and both hold the test key.

mod common;

use common::{ABANDON, Element, Harness, PANEL, PIN, SECURE_PHONE};
use opensigner_core::{ScreenKind, ids, strings, tools};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_shell_api::{Event, FileKind};
use osk_ui::widgets::Icon;
use osk_ui::widgets::keyboard::KeyInput;

const LIANA: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.policy");
const TREE: &str = include_str!("../../../tools/vectors/psbt/wallet-tree.policy");
const LIANA_PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-liana.psbt");

fn policy(text: &str) -> WalletPolicy {
    WalletPolicy::parse_any(text).expect("a committed fixture")
}

/// A device on regtest with the test key loaded.
fn regtest_device() -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    h
}

/// Add › "Load a wallet", with `text` read from a file there.
fn load_wallet(h: &mut Harness, text: &str) {
    h.go_home();
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: text.as_bytes().to_vec(),
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

/// The review of the Liana-shaped wallet: what kind of wallet it is,
/// who spends now and after how long, what it pays to, and the two ways
/// it can be spent, in words.
#[test]
fn the_review_of_a_miniscript_wallet_names_its_spend_paths() {
    let mut h = regtest_device();
    load_wallet(&mut h, LIANA);
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    assert!(says(strings::EN.wallet_recovery), "the kind: {texts:?}");
    assert!(
        says(strings::EN.wallet_recovery_now),
        "who spends now: {texts:?}"
    );
    assert!(
        says(&strings::fill1(strings::EN.wallet_recovery_after, "365")),
        "the wait: {texts:?}"
    );
    assert!(says(strings::EN.script_segwit), "the script: {texts:?}");
    assert!(says(strings::EN.inspect_checksum_ok), "{texts:?}");

    // One row per spend path: key A alone, and key B after the wait.
    assert!(
        says(&strings::fill1(strings::EN.wallet_path_key, "A")),
        "the first path: {texts:?}"
    );
    let waited = strings::fill(
        strings::EN.wallet_path_after,
        &[
            &strings::fill1(strings::EN.wallet_path_key, "B"),
            &strings::fill(
                strings::EN.wallet_lock_blocks,
                &["52\u{00a0}560", strings::EN.wallet_about_year],
            ),
        ],
    );
    assert!(says(&waited), "the recovery path: {texts:?}");
    assert!(says(strings::EN.wallet_use), "the way to use it: {texts:?}");
}

/// A taproot tree is named as one, and its rows are the key path first
/// and then a leaf each, in the order `tr()` writes them.
#[test]
fn the_review_of_a_taproot_tree_lists_the_key_path_first() {
    let mut h = regtest_device();
    load_wallet(&mut h, TREE);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    // The fixture's second leaf opens after a wait, so the wallet is
    // read as a recovery wallet whatever else its tree holds.
    assert!(says(strings::EN.wallet_recovery), "the kind: {texts:?}");
    assert!(says(strings::EN.script_taproot), "the script: {texts:?}");

    let paths: Vec<&String> = texts
        .iter()
        .filter(|t| {
            t.starts_with("Key ")
                || t.starts_with(&strings::fill1(strings::EN.wallet_path_keys, ""))
        })
        .collect();
    assert_eq!(
        paths.first().map(|t| t.as_str()),
        Some(strings::fill1(strings::EN.wallet_path_key, "A").as_str()),
        "the key path comes first: {texts:?}"
    );
    assert!(
        says(&strings::fill(
            strings::EN.wallet_path_after,
            &[
                &strings::fill1(strings::EN.wallet_path_key, "B"),
                &strings::fill(
                    strings::EN.wallet_lock_blocks,
                    &[
                        "4\u{00a0}320",
                        &strings::fill1(strings::EN.wallet_about_days, "30")
                    ],
                ),
            ],
        )),
        "the waiting leaf: {texts:?}"
    );
    assert!(
        says(&strings::fill1(strings::EN.wallet_path_key, "C")),
        "the other leaf: {texts:?}"
    );
}

/// In use, each is a row on Wallets with the key glyph — the test key is in
/// both — its addresses are the wallet's own, its export names the
/// descriptor's checksum, and Forget takes the row away.
#[test]
fn a_miniscript_wallet_is_a_row_a_list_of_addresses_and_an_export() {
    for (text, label) in [
        (LIANA, strings::EN.wallet_kind_recovery),
        (TREE, strings::EN.wallet_kind_recovery),
    ] {
        let wallet = policy(text);
        let mut h = regtest_device();
        load_wallet(&mut h, text);
        h.tap(ids::INSPECT_USE_WALLET);
        assert_eq!(h.app.screen(), ScreenKind::Wallet);

        h.open_wallets();
        let row = ids::at(ids::WALLETS_POLICY_ROW_BASE, 0);
        let glyphs = h.app.row_glyphs();
        let found = glyphs
            .iter()
            .find(|(name, _)| name.starts_with(label))
            .unwrap_or_else(|| panic!("the wallet's row: {glyphs:?}"));
        assert_eq!(found.1, Icon::Wallet, "this device holds one of its keys");

        h.tap(row);
        h.tap(ids::WALLET_ADDRESSES);
        assert_eq!(h.app.screen(), ScreenKind::Addresses);
        for index in 0..2 {
            let address = wallet
                .address_at(Network::Regtest, false, index)
                .expect("an address")
                .to_string();
            assert!(
                shows_address(&h, &address),
                "{address} is on the list: {:?}",
                h.app.texts()
            );
        }
        h.tap(ids::ADDR_CHANGE);
        let change = wallet
            .address_at(Network::Regtest, true, 0)
            .expect("an address")
            .to_string();
        assert!(shows_address(&h, &change), "{:?}", h.app.texts());

        h.tap(ids::BACK);
        h.tap(ids::WALLET_EXPORT);
        assert_eq!(h.app.screen(), ScreenKind::Export);
        assert!(
            h.app.texts().iter().any(|t| t.contains(&wallet.checksum())),
            "the export names the checksum: {:?}",
            h.app.texts()
        );

        h.open_wallets();
        h.tap(row);
        h.tap(ids::WALLET_FORGET);
        assert_eq!(h.app.screen(), ScreenKind::Wallets);
        assert!(h.app.rect_of(row).is_none(), "the row is gone");
    }
}

/// A device that keeps keys keeps a miniscript wallet beside the key it
/// is in use with, so it is still a row after the app is closed and the
/// PIN opens it again.
#[test]
fn a_miniscript_wallet_in_use_survives_a_lock_and_an_unlock() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "the key is on the device");

    load_wallet(&mut h, LIANA);
    h.tap(ids::INSPECT_USE_WALLET);
    h.open_wallets();
    let row = ids::at(ids::WALLETS_POLICY_ROW_BASE, 0);
    assert!(h.app.rect_of(row).is_some(), "in use");

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    h.open_wallets();
    assert!(
        h.app.rect_of(row).is_some(),
        "the wallet came back with the key: {:?}",
        h.app.texts()
    );
    h.tap(row);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains(strings::EN.wallet_recovery)),
        "and it is the same wallet: {:?}",
        h.app.texts()
    );
}

/// A key written with one chain has no change chain to verify against,
/// so the wallet is refused where it was read and the reason is on
/// screen.
#[test]
fn a_miniscript_key_with_one_chain_is_refused_with_a_reason() {
    let one_chain = policy(LIANA).to_descriptor().replace("/<0;1>/*", "/0/*");
    let mut h = regtest_device();
    load_wallet(&mut h, &one_chain);
    assert_ne!(
        h.app.screen(),
        ScreenKind::Wallet,
        "not a wallet this device can use"
    );
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.scan_reason_policy),
        "the reason is on screen: {:?}",
        h.app.texts()
    );
}

/// Tools › Miniscript compiles a policy into the descriptor of the same
/// wallet, and says why one that does not compile does not.
#[test]
fn the_compiler_turns_a_policy_into_a_descriptor() {
    let mut h = regtest_device();
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    let row = ids::at(ids::TOOLS_CALC_BASE, tools::Tool::ALL.len() - 1);
    h.tap(row);
    h.tap(ids::SCAN_TYPE);
    assert_eq!(h.app.screen(), ScreenKind::Tool);

    const POLICY: &str = "or(pk(@0),and(pk(@1),older(52560)))";
    h.type_text(POLICY);
    h.pad(ids::TOOL_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.screen(), ScreenKind::ToolResult);

    // The compiled descriptor is the Liana form: one key that spends
    // now and one that spends after the wait. The compiler picks `pk`
    // over `pkh` for the second branch, which is the same spend a byte
    // cheaper.
    let facts = osk_bip::compile::compile(POLICY, osk_bip::compile::PolicyScript::Segwit, &[])
        .expect("this policy compiles");
    assert!(
        facts
            .descriptor
            .starts_with("wsh(or_d(pk(@0),and_v(v:pk(@1),older(52560))))#"),
        "the Liana form: {}",
        facts.descriptor
    );
    h.tap(ids::at(ids::TOOL_ROW_BASE, 0));
    let whole: String = h.app.texts().join(" ").replace(' ', "");
    assert!(
        whole.contains(&facts.descriptor),
        "the descriptor, whole: {whole}"
    );
    h.tap(ids::COMPARE_DONE);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| *t == strings::fill1(strings::EN.wallet_path_key, "A")),
        "and its spend paths: {:?}",
        h.app.texts()
    );
}

/// A policy the compiler refuses says why, in the compiler's own words,
/// and ✓ leads nowhere.
#[test]
fn a_policy_that_does_not_compile_says_why() {
    let mut h = regtest_device();
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::at(ids::TOOLS_CALC_BASE, tools::Tool::ALL.len() - 1));
    h.tap(ids::SCAN_TYPE);
    h.type_text("or(pk(@0),");
    assert_eq!(
        h.app.screen(),
        ScreenKind::Tool,
        "\u{2713} is dead, so there is nowhere to go"
    );
    let reason =
        osk_bip::compile::compile("or(pk(@0),", osk_bip::compile::PolicyScript::Segwit, &[])
            .expect_err("this policy does not compile");
    assert!(!reason.is_empty());
    assert!(
        h.app.texts().contains(&reason),
        "the compiler's own words: {:?}",
        h.app.texts()
    );
}

/// A spend of the Liana wallet: its change is verified while the wallet
/// is in use, and the review names the way through the script the
/// transaction takes instead of refusing the input.
#[test]
fn the_sign_review_verifies_the_change_and_names_the_spend_path() {
    let mut h = regtest_device();
    load_wallet(&mut h, LIANA);
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_SIGN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: osk_psbt::Psbt::parse_base64(LIANA_PSBT.trim())
            .expect("the committed PSBT")
            .to_bytes(),
    });
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        matches!(
            insp.outputs[1].kind,
            osk_psbt::OutputKind::WalletChange { .. }
        ),
        "{:?}",
        insp.outputs[1].kind
    );
    assert!(
        insp.warnings
            .iter()
            .all(|w| w.kind != osk_psbt::WarningKind::UnsupportedInput),
        "the input is one this device signs: {:?}",
        insp.warnings
    );
    assert_eq!(
        insp.inputs[0].spend_route,
        Some(osk_psbt::SpendRoute::Script(osk_bip::spend::SpendPath {
            keys: vec![0],
            locks: Vec::new(),
        })),
        "the sequence arms no wait, so the first key's path is the one it takes"
    );
    // Summary, outputs, change, then the inputs, where the row carries
    // the path in the words the wallet's own review used.
    for _ in 0..3 {
        h.tap(ids::SIGN_CONTINUE);
    }
    let path = strings::fill1(h.app.strings().wallet_path_key, "A");
    assert!(
        h.app.texts().iter().any(|t| t.contains(&path)),
        "the input row names the path: {:?}",
        h.app.texts()
    );
}
