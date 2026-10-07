//! Add a wallet, one wizard for all six kinds (`docs/PLANNING.md`
//! §16.104 rule 2, §16.106).
//!
//! The 2-of-3 wallet built from the loaded test key and the two other
//! keys of `tools/vectors/psbt/wallet-2of3.policy` is that wallet: same
//! descriptor, same checksum, and a transaction of it verifies its own
//! change. The MuSig2 wallet is BIP-390's two vector keys, and the keys
//! of one are written in the order BIP-327 sorts them, so the same keys
//! always make the same wallet.

mod common;

use common::{ABANDON, Element, Harness, PANEL, SECURE_PHONE};
use opensigner_core::build::Step;
use opensigner_core::sign::{Stage, Step as SignStep};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_psbt::{OutputKind, Psbt};
use osk_shell_api::{ButtonId, Event, FileKind, Key};
use osk_ui::fonts::outlines;
use osk_ui::widgets::Icon;
use osk_ui::widgets::keyboard::KeyInput;

const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");
const PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.psbt");
const TAPMULTI: &str = include_str!("../../../tools/vectors/psbt/wallet-tapmulti.policy");
const TAPMULTI_FIRST: &str = include_str!("../../../tools/vectors/psbt/wallet-tapmulti-first.psbt");
const TAPMULTI_SIGNED: &str =
    include_str!("../../../tools/vectors/psbt/wallet-tapmulti-signed.psbt");
const ZOO: &str = "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong";
const MUSIG: &str = include_str!("../../../tools/vectors/psbt/wallet-musig.policy");
const MUSIG_REGTEST: &str = include_str!("../../../tools/vectors/psbt/wallet-musig-regtest.policy");
const SHARE_0: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-0.txt");
const SHARE_1: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-1.txt");

/// The kinds the first Choice lists, in `build::WalletKind::ALL` order.
const SINGLE: usize = 0;
const MULTISIG: usize = 1;
const TAPROOT_MULTISIG: usize = 2;
const MUSIG_KIND: usize = 3;
const FROST: usize = 4;
const RECOVERY: usize = 5;
/// SegWit's place in `ScriptType::ALL`, which the script Choice lists.
const SEGWIT: usize = 2;
/// Taproot's place in the same list.
const TAPROOT: usize = 3;

fn policy() -> WalletPolicy {
    WalletPolicy::parse(POLICY).expect("the committed policy")
}

fn musig_policy() -> WalletPolicy {
    WalletPolicy::parse(MUSIG).expect("the committed policy")
}

/// The same two keys as the wallet a builder writes: BIP-327 orders the
/// participants by their compressed serialisation, which puts the
/// fixture's second key first, and the derivation follows the
/// aggregate.
fn sorted_musig_wallet() -> WalletPolicy {
    let keys: Vec<String> = musig_policy()
        .keys()
        .iter()
        .rev()
        .map(|k| k.key_text())
        .collect();
    let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
    WalletPolicy::from_parts("tr(musig(@0,@1)/**)", &refs).expect("two keys aggregate")
}

/// The cosigners of the 2-of-3 fixture that are not this device's key,
/// as the text a coordinator hands over.
fn cosigners() -> Vec<String> {
    policy().keys()[1..].iter().map(|k| k.key_text()).collect()
}

fn psbt_bytes() -> Vec<u8> {
    Psbt::parse_base64(PSBT.trim())
        .expect("the committed PSBT")
        .to_bytes()
}

/// The loaded test key on the fixture's chain.
fn with_key(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    assert_eq!(h.app.network(), Network::Regtest);
}

/// Add a wallet › "New wallet", at the kind step.
fn open_builder(h: &mut Harness) {
    h.open_add_wallet(ids::BUILD_NEW);
    assert_eq!(h.app.screen(), ScreenKind::Build);
    assert_eq!(h.app.build_step(), Some(Step::Kind));
}

/// The kind Choice, and on to the keys.
fn choose_kind(h: &mut Harness, kind: usize) {
    choose_kind_no_keys(h, kind);
    assert_eq!(h.app.build_step(), Some(Step::Keys));
}

/// The kind Choice alone; FROST's counts stand between it and the keys.
fn choose_kind_no_keys(h: &mut Harness, kind: usize) {
    h.choose(
        ids::at(ids::BUILD_KIND_BASE, kind),
        ids::BUILD_KIND_CONTINUE,
    );
}

/// The 24 words of one member of the committed regtest group.
fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// A loaded key's fingerprint as the screens print it.
fn hex(fp: osk_bip::keys::Fingerprint) -> String {
    String::from_utf8(fp.to_hex().to_vec()).expect("ascii hex")
}

/// The row of the loaded key at `i` on the Keys step.
fn add_loaded(h: &mut Harness, i: usize) {
    h.tap(ids::at(ids::BUILD_WHICH_BASE, i));
}

/// "Scan a key", and `text` read there as a file.
fn scan_key(h: &mut Harness, text: &str) {
    h.tap(ids::BUILD_WHICH_SCAN);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: text.as_bytes().to_vec(),
    });
}

/// The 2-of-3 wallet of the loaded key and the fixture's two cosigners,
/// built and added.
fn build_the_fixture(h: &mut Harness) {
    open_builder(h);
    choose_kind(h, MULTISIG);
    add_loaded(h, 0);
    for key in cosigners() {
        scan_key(h, &key);
    }
    assert_eq!(h.app.build_step(), Some(Step::Keys));
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Script));
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, SEGWIT),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Threshold));
    h.choose(
        ids::at(ids::BUILD_THRESHOLD_BASE, 1),
        ids::BUILD_THRESHOLD_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Review));
}

/// The reason on the scanner's refusal Result.
fn refusal(h: &mut Harness, text: &str) -> Vec<String> {
    scan_key(h, text);
    let texts = h.app.texts();
    h.tap(ids::SCAN_AGAIN);
    h.tap(ids::BACK);
    texts
}

/// The wallet built here is the wallet the coordinator's policy names:
/// the same descriptor and checksum, the key glyph for the cosigner
/// this device holds, and the change of its own transaction verified.
#[test]
fn the_built_wallet_is_the_wallet_the_policy_names() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    build_the_fixture(&mut h);

    let texts = h.app.texts();
    let quorum = strings::fill(strings::EN.wallet_quorum, &["2", "3"]);
    assert!(texts.contains(&quorum), "the review's quorum: {texts:?}");
    // Nothing was given to this device to check, so the review of a
    // wallet built here states no checksum and no count of its keys.
    assert!(
        !texts.iter().any(|t| *t == strings::EN.inspect_checksum),
        "a built wallet has no checksum row: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| *t == strings::EN.row_keys),
        "a built wallet has no key count row: {texts:?}"
    );

    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet, "the wallet's own page");
    h.tap(ids::WALLET_EXPORT);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains(&policy().checksum())),
        "the export is the fixture's descriptor: {:?}",
        h.app.texts()
    );

    h.open_wallets();
    let glyphs = h.app.row_glyphs();
    let row = glyphs
        .iter()
        .find(|(label, _)| label.starts_with(strings::EN.wallet_kind_multisig))
        .expect("the built wallet's row");
    assert_eq!(row.1, Icon::Wallet, "this device holds one of its keys");

    // The wallet verifies the change of its own transaction, as the
    // same wallet loaded from the policy does.
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));
    let insp = h.app.sign_inspection().expect("inspected");
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0
        }
    );
    assert!(insp.inputs[0].is_ours);
}

/// A MuSig2 wallet aggregates every key it holds, so it asks no
/// threshold, and its participants are written in BIP-327's key order
/// whichever order they were gathered in.
#[test]
fn a_musig_wallet_is_its_keys_in_sorted_order() {
    let keys: Vec<String> = musig_policy().keys().iter().map(|k| k.key_text()).collect();
    for order in [[0, 1], [1, 0]] {
        let mut h = Harness::new(PANEL);
        // A key is loaded because Add is reached from a Home with
        // something on it; no key of this device is in the wallet.
        h.start_load(&ABANDON);
        h.finish_load(None);
        assert_eq!(h.app.network(), Network::Mainnet);
        open_builder(&mut h);
        choose_kind(&mut h, MUSIG_KIND);
        for i in order {
            scan_key(&mut h, &keys[i]);
        }
        // Every key signs, so Continue goes straight to the review.
        h.tap(ids::BUILD_CONTINUE);
        assert_eq!(h.app.build_step(), Some(Step::Review));
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| *t == strings::EN.wallet_musig),
            "the review says what it is: {texts:?}"
        );

        h.tap(ids::BUILD_ADD_WALLET);
        h.tap(ids::WALLET_EXPORT);
        assert!(
            h.app
                .texts()
                .iter()
                .any(|t| t.contains(&sorted_musig_wallet().checksum())),
            "the same wallet whichever order, gathered {order:?}: {:?}",
            h.app.texts()
        );

        // No key of it is this device's, so the row carries the eye.
        h.open_wallets();
        let glyphs = h.app.row_glyphs();
        let row = glyphs
            .iter()
            .find(|(label, _)| label.contains(strings::EN.wallet_musig))
            .expect("the MuSig2 wallet's row");
        assert_eq!(row.1, Icon::Eye, "public keys only");
    }
}

/// A wallet needs two keys, and a key can be taken back out.
#[test]
fn one_key_is_not_a_wallet_and_a_key_can_be_removed() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    choose_kind(&mut h, MULTISIG);
    add_loaded(&mut h, 0);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(
        h.app.build_step(),
        Some(Step::Keys),
        "one key goes no further"
    );

    scan_key(&mut h, &cosigners()[0]);
    // A second tap on a scanned cosigner's row takes it back out.
    h.tap(ids::at(ids::BUILD_KEY_ROW_BASE, 1));
    assert_eq!(h.app.build_step(), Some(Step::Keys));
    assert!(
        h.app.rect_of(ids::at(ids::BUILD_KEY_ROW_BASE, 1)).is_none(),
        "the key is off the list"
    );
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys), "one key again");
}

/// What the scanner refuses here, each with the reason §4.11 gives:
/// a key already in the wallet, a key of another chain, a bare extended
/// public key that states no master, and a whole wallet.
#[test]
fn the_scanner_refuses_what_is_no_cosigner_key() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    choose_kind(&mut h, MULTISIG);
    add_loaded(&mut h, 0);
    scan_key(&mut h, &cosigners()[0]);

    let says = |texts: &[String], reason: &str| texts.iter().any(|t| t == reason);
    let texts = refusal(&mut h, &cosigners()[0]);
    assert!(
        says(&texts, strings::EN.build_reason_added),
        "a key already added: {texts:?}"
    );

    // The device is on regtest; a mainnet key is not one of its
    // cosigners.
    let mainnet = musig_policy().keys()[0].key_text();
    let texts = refusal(&mut h, &mainnet);
    assert!(
        says(&texts, strings::EN.build_reason_mainnet),
        "a mainnet key: {texts:?}"
    );

    let bare = policy().keys()[1].xpub().to_string();
    let texts = refusal(&mut h, &bare);
    assert!(
        says(&texts, strings::EN.build_reason_origin),
        "a key with no origin: {texts:?}"
    );

    let texts = refusal(&mut h, POLICY);
    assert!(
        says(&texts, strings::EN.build_reason_wallet),
        "a whole wallet: {texts:?}"
    );

    // None of them joined the wallet.
    assert_eq!(h.app.build_step(), Some(Step::Keys));
    assert!(h.app.rect_of(ids::at(ids::BUILD_KEY_ROW_BASE, 2)).is_none());
}

/// Building a wallet that is already in use opens the one in use rather
/// than listing it twice.
#[test]
fn a_wallet_already_in_use_opens_instead_of_joining_the_list() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    build_the_fixture(&mut h);
    h.tap(ids::BUILD_ADD_WALLET);
    h.open_wallets();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_some()
    );

    build_the_fixture(&mut h);
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.go_home();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 1))
            .is_none(),
        "the same wallet is one row"
    );
}

/// Back walks the wizard: from the Keys step to the kind step with the
/// keys kept, and from the kind step to the question that guards them,
/// which discards them or keeps them (§16.128 rule 2).
#[test]
fn back_walks_the_steps_and_asks_once_on_the_way_out() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    choose_kind(&mut h, MULTISIG);
    add_loaded(&mut h, 0);

    h.tap(ids::BACK);
    assert_eq!(
        h.app.build_step(),
        Some(Step::Kind),
        "Back from the keys is the kind step, not a question"
    );
    h.tap(ids::BUILD_KIND_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys));

    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.build_step(), Some(Step::Discard));
    h.tap(ids::BUILD_KEEP);
    assert_eq!(
        h.app.build_step(),
        Some(Step::Kind),
        "Keep returns to the step the question covered"
    );
    h.tap(ids::BUILD_KIND_CONTINUE);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(
        h.app.build_step(),
        Some(Step::Keys),
        "one key of the two a multisig wallet needs is still checked"
    );

    h.tap(ids::BACK);
    h.tap(ids::BACK);
    h.tap(ids::BUILD_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
    h.go_home();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "nothing was added"
    );
}

/// The discard question has no chevron, and the system back on it keeps
/// the keys (§16.128 rule 3).
#[test]
fn the_discard_question_has_no_way_back_to_itself() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    choose_kind(&mut h, MULTISIG);
    add_loaded(&mut h, 0);
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.build_step(), Some(Step::Discard));
    assert!(
        h.app.rect_of(ids::BACK).is_none(),
        "the question offers no chevron"
    );

    h.send(Event::Button(ButtonId::Back));
    assert_eq!(
        h.app.build_step(),
        Some(Step::Kind),
        "the system back keeps the keys"
    );

    h.tap(ids::BACK);
    h.tap(ids::BUILD_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
    h.go_home();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "nothing was kept"
    );
}

/// A wallet built on one key moves its check: a tap on another key
/// replaces the one checked, and the wallet is that second key's
/// (§16.128 rule 1).
#[test]
fn a_one_key_wallet_moves_its_check() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    h.start_load(&words(ZOO));
    h.finish_load(None);
    let first = h.app.fingerprints()[0];
    let second = h.app.fingerprints()[1];

    open_builder(&mut h);
    choose_kind(&mut h, SINGLE);
    add_loaded(&mut h, 0);
    add_loaded(&mut h, 1);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Script));
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, SEGWIT),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Review));

    let texts = h.app.texts();
    assert!(
        texts.contains(&hex(second)),
        "the review is the second key's wallet: {texts:?}"
    );
    assert!(
        !texts.contains(&hex(first)),
        "the first key was replaced: {texts:?}"
    );
}

/// Back on the kind step, where nothing has been gathered, leaves the
/// wizard without asking.
#[test]
fn back_on_the_first_step_leaves() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
}

/// A device that keeps keys keeps a wallet built here beside them, like
/// any other wallet in use.
#[test]
fn a_built_wallet_comes_back_with_the_kept_keys() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "the shell is holding the blob");
    h.go_home();
    h.set_network(3);

    build_the_fixture(&mut h);
    h.tap(ids::BUILD_ADD_WALLET);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, common::PIN);
    h.open_wallets();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_some(),
        "the built wallet came back with the key"
    );
}

/// A single-sig wallet is one loaded key and one script type, and it
/// ends at the same review every other kind does (§16.104 rule 2).
#[test]
fn a_single_sig_wallet_is_one_key_and_one_script_type() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    choose_kind(&mut h, SINGLE);
    // With no key chosen the wizard goes no further.
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys));
    add_loaded(&mut h, 0);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Script));
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, SEGWIT),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Review));
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert_eq!(h.app.wallet_count(), 1);
}

/// FROST, 2 of 3, from two loaded 24-word keys: the counts come first,
/// the third key's words are shown and quizzed, the record is shown and
/// saved, all three keys are in Keys afterwards, the wallet is in use
/// with the key glyph, and its Keys review names three members with the
/// glyph on all three.
#[test]
fn a_frost_group_is_dealt_from_two_loaded_keys() {
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.go_home();
    h.start_load_24(&words(SHARE_1));
    h.finish_load(None);
    h.go_home();

    open_builder(&mut h);
    choose_kind_no_keys(&mut h, FROST);
    assert_eq!(h.app.build_step(), Some(Step::Count));
    h.tap(ids::THRESHOLD_COUNT_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Quorum));
    h.tap(ids::THRESHOLD_QUORUM_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys));

    // Two of the three keys are chosen; Continue is dead until both are.
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys));
    add_loaded(&mut h, 0);
    add_loaded(&mut h, 1);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Review));

    // Accepting the review deals the group: the one computed key's words
    // are shown and quizzed, then the record.
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.build_step(), Some(Step::Words));
    h.tap(ids::THRESHOLD_WORDS_CONTINUE);
    h.tap(ids::QUIZ_SKIP);
    h.tap(ids::QUIZ_SKIP_CONFIRM);
    assert_eq!(h.app.build_step(), Some(Step::Record));

    h.seen.clear();
    h.tap(ids::THRESHOLD_RECORD_SAVE);
    let bytes = h
        .seen
        .iter()
        .find_map(|c| match c {
            osk_shell_api::Command::WriteFile { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .expect("the record was written");
    let record =
        osk_bip::threshold::ThresholdRecord::parse(core::str::from_utf8(&bytes).expect("text"))
            .expect("the saved record parses and validates");
    assert_eq!(record.t(), 2);
    assert_eq!(record.n(), 3);

    h.tap(ids::THRESHOLD_RECORD_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);

    // Every key of the group this device made is loaded.
    h.open_keys();
    assert_eq!(h.app.fingerprints().len(), 3, "all three keys are in Keys");

    h.open_wallets();
    let glyph = h
        .app
        .row_glyphs()
        .into_iter()
        .find(|(label, _)| label.starts_with(strings::EN.wallet_kind_threshold))
        .expect("the FROST row")
        .1;
    assert_eq!(glyph, Icon::Wallet);

    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_KEYS);
    let held: Vec<Icon> = h.app.row_glyphs().iter().map(|(_, icon)| *icon).collect();
    assert_eq!(held.len(), 3, "three members: {held:?}");
    assert!(
        held.iter().all(|i| *i == Icon::Wallet),
        "this device holds all three: {held:?}"
    );
}

/// A key this kind cannot use is dimmed with the reason, and the count a
/// group of this shape needs is stated where Continue is dead.
#[test]
fn a_frost_group_says_what_it_needs_and_why_a_key_cannot_join() {
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    // A 12-word key is the only one loaded.
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.go_home();

    open_builder(&mut h);
    choose_kind_no_keys(&mut h, FROST);
    h.tap(ids::THRESHOLD_COUNT_CONTINUE);
    h.tap(ids::THRESHOLD_QUORUM_CONTINUE);
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| *t == strings::fill1(strings::EN.build_reason_words, "12")),
        "the 12-word key's reason: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| *t == strings::fill1(strings::EN.build_needs_keys, "2")),
        "what the group needs: {texts:?}"
    );
    // The dimmed row cannot be taken, and Continue goes nowhere.
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys));

    // One usable key is still one short of two.
    h.go_home();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.go_home();
    open_builder(&mut h);
    choose_kind_no_keys(&mut h, FROST);
    h.tap(ids::THRESHOLD_COUNT_CONTINUE);
    h.tap(ids::THRESHOLD_QUORUM_CONTINUE);
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 1));
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Keys));
}

/// A taproot multisig is BIP 387's `sortedmulti_a` under BIP 341's NUMS
/// point: the review says the key path cannot spend, the addresses are
/// the ones the leaf's keys make once they are derived and sorted, and
/// the wallet is the one Bitcoin Core imported and funded. Two of its
/// three keys are on this device, so one pass over Core's spend writes
/// both signatures.
#[test]
fn a_taproot_multisig_pays_to_its_leaf_and_nothing_else() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    // The second key of the wallet, so that the device can sign twice.
    h.start_load(&words(ZOO));
    h.finish_load(None);
    open_builder(&mut h);
    choose_kind(&mut h, TAPROOT_MULTISIG);
    add_loaded(&mut h, 0);
    add_loaded(&mut h, 1);
    scan_key(&mut h, &cosigners()[1]);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Threshold));
    h.choose(
        ids::at(ids::BUILD_THRESHOLD_BASE, 1),
        ids::BUILD_THRESHOLD_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Review));

    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.wallet_unspendable)),
        "the review states the key path cannot spend: {texts:?}"
    );
    assert!(
        texts.contains(&strings::fill(strings::EN.wallet_quorum, &["2", "3"])),
        "the review states the quorum: {texts:?}"
    );

    h.tap(ids::BUILD_ADD_WALLET);
    let built = h.app.wallet_policy(0).expect("the built wallet").clone();
    assert_eq!(
        built.to_descriptor(),
        WalletPolicy::parse_any(TAPMULTI)
            .expect("the committed policy")
            .to_descriptor(),
        "the wallet the wizard builds is the one Bitcoin Core imported"
    );
    assert_eq!(built.tapscript_quorum(), Some((2, 3)));
    assert!(built.key_path_unspendable());
    assert!(built.to_descriptor().contains("sortedmulti_a(2,"));

    // The addresses are the leaf's own: the derived x-only keys sorted,
    // under the NUMS internal key.
    let secp = osk_bip::bitcoin::secp256k1::Secp256k1::verification_only();
    let internal =
        osk_bip::bitcoin::secp256k1::XOnlyPublicKey::from_slice(&osk_bip::tapmulti::NUMS)
            .expect("the NUMS point");
    for index in 0..3 {
        let keys: Vec<_> = built
            .keys()
            .iter()
            .map(|k| {
                k.xpub()
                    .derive_pub(
                        &secp,
                        &[
                            osk_bip::bitcoin::bip32::ChildNumber::from_normal_idx(0).unwrap(),
                            osk_bip::bitcoin::bip32::ChildNumber::from_normal_idx(index).unwrap(),
                        ],
                    )
                    .expect("derive")
                    .public_key
                    .x_only_public_key()
                    .0
            })
            .collect();
        let leaf = osk_bip::tapmulti::sorted_multi_a_script(2, &keys);
        let script =
            osk_bip::tapmulti::output_script(&secp, internal, &leaf).expect("one-leaf tree");
        assert_eq!(
            built.script_at(false, index).expect("the wallet's script"),
            script,
            "receive address {index}"
        );
    }

    // Core's spend of that wallet, read the way any transaction is.
    h.tap(ids::WALLET_SIGN);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: Psbt::parse_base64(TAPMULTI_FIRST.trim())
            .expect("the committed PSBT")
            .to_bytes(),
    });
    assert_eq!(h.app.sign_stage(), Some(Stage::Wizard(SignStep::Summary)));
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(insp.inputs[0].is_ours);
    assert_eq!(
        insp.outputs[1].kind,
        OutputKind::WalletChange {
            wallet: 0,
            change: true,
            index: 0
        },
        "the rest goes back to the wallet's own change address"
    );
    assert_eq!(
        insp.participating_keys.len(),
        2,
        "both loaded keys are in the leaf"
    );

    for _ in 0..8 {
        if h.app.rect_of(ids::SIGN_HOLD).is_some() {
            break;
        }
        h.tap(ids::SIGN_CONTINUE);
    }
    h.hold(ids::SIGN_HOLD);
    assert_eq!(h.app.sign_outcome_signatures(), Some(2));

    // What the device hands over is the committed signed fixture, whose
    // two signatures satisfy the leaf, so the flow finalizes it.
    let mut fixture = Psbt::parse_base64(TAPMULTI_SIGNED.trim()).expect("the committed PSBT");
    osk_psbt::finalize(&mut fixture).expect("a satisfied leaf");
    assert_eq!(
        h.app.sign_outcome_psbt().expect("a signed PSBT"),
        fixture.to_bytes(),
        "the signed fixture, finalized"
    );
}

/// A recovery wallet is the wallet Liana writes: the keys that spend
/// now, the keys that spend after the wait, and the wait itself.
#[test]
fn a_recovery_wallet_is_the_policy_its_two_paths_state() {
    for (script, form) in [
        (SEGWIT, osk_bip::recovery::Form::SegWit),
        (TAPROOT, osk_bip::recovery::Form::Taproot),
    ] {
        let mut h = Harness::new(PANEL);
        with_key(&mut h);
        open_builder(&mut h);
        choose_kind(&mut h, RECOVERY);
        add_loaded(&mut h, 0);
        // One key now needs no threshold: one key signs.
        h.tap(ids::BUILD_CONTINUE);
        assert_eq!(h.app.build_step(), Some(Step::Later));
        scan_key(&mut h, &cosigners()[0]);
        h.tap(ids::BUILD_LATER_CONTINUE);
        assert_eq!(h.app.build_step(), Some(Step::Delay));
        // Each wait's block count is on its row, in the digits and the
        // separator the faces draw, and never as `?`.
        let blocks: Vec<String> = h
            .app
            .texts()
            .into_iter()
            .filter(|t| t.ends_with(&strings::fill1(strings::EN.build_delay_blocks, "")))
            .collect();
        assert_eq!(blocks.len(), 4, "a block count under each wait: {blocks:?}");
        assert!(
            blocks.contains(&strings::fill1(
                strings::EN.build_delay_blocks,
                &format!("4{}320", osk_ui::tokens::GROUP_SEPARATOR)
            )),
            "30 days in blocks: {blocks:?}"
        );
        for text in &blocks {
            for c in text.chars() {
                for family in osk_ui::fonts::TEXT_FAMILIES {
                    let g = outlines(family).glyph_or_fallback(c).expect("a glyph");
                    assert_eq!(g.code_point, c, "{family:?} draws {text:?} with a fallback");
                }
            }
        }
        h.choose(ids::at(ids::BUILD_DELAY_BASE, 3), ids::BUILD_DELAY_CONTINUE);
        // The wait is followed by the question that offers a further
        // path; a wallet with one recovery path answers No.
        assert_eq!(h.app.build_step(), Some(Step::Another));
        h.choose(ids::BUILD_ANOTHER_NO, ids::BUILD_ANOTHER_CONTINUE);
        assert_eq!(h.app.build_step(), Some(Step::Script));
        h.choose(
            ids::at(ids::BUILD_SCRIPT_BASE, script),
            ids::BUILD_SCRIPT_CONTINUE,
        );
        assert_eq!(h.app.build_step(), Some(Step::Review));

        let texts = h.app.texts();
        assert!(
            texts.contains(&String::from(strings::EN.wallet_recovery)),
            "the review names the kind: {texts:?}"
        );
        assert!(
            texts.contains(&strings::fill1(strings::EN.wallet_recovery_after, "365")),
            "the review names the wait: {texts:?}"
        );

        h.tap(ids::BUILD_ADD_WALLET);
        let built = h.app.wallet_policy(0).expect("the built wallet").clone();
        let keys: Vec<String> = built.keys().iter().map(|k| k.key_text()).collect();
        let read = osk_bip::recovery::RecoveryPolicy::from_wallet_policy(&built)
            .expect("a wallet with a timelocked path");
        assert_eq!(read.recovery.len(), 1);
        assert_eq!(read.recovery[0].delay, 365 * 144);
        assert_eq!(read.primary.keys.len(), 1);
        assert!(keys.iter().any(|k| *k == read.recovery[0].path.keys[0]));
        assert_eq!(
            built.to_descriptor(),
            read.to_wallet_policy(form)
                .expect("the same policy")
                .to_descriptor()
        );
    }
}

/// A key that spends on one of a recovery wallet's paths cannot also
/// spend on the other: the review of who can spend would be a lie.
#[test]
fn a_recovery_key_cannot_stand_on_both_paths() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder(&mut h);
    choose_kind(&mut h, RECOVERY);
    add_loaded(&mut h, 0);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Later));
    // The one loaded key already spends now, so its row is dimmed with
    // the reason §4.11 gives and carries no hit target at all.
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.build_reason_other_path)),
        "the reason: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.reveal(ids::at(ids::BUILD_WHICH_BASE, 0)).is_none(),
        "the dimmed row cannot be tapped"
    );
}

/// A fourth key on the fixture's chain, which a third recovery path
/// needs: the regtest MuSig2 wallet's second participant, named by no
/// other path here.
fn fourth_key() -> String {
    WalletPolicy::parse(MUSIG_REGTEST)
        .expect("the committed policy")
        .keys()[1]
        .key_text()
}

/// The primary path of every recovery wallet built here: the loaded
/// key, which needs no threshold because one key signs.
fn recovery_now(h: &mut Harness) {
    with_key(h);
    open_builder(h);
    choose_kind(h, RECOVERY);
    add_loaded(h, 0);
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Later));
}

/// One recovery path: `key` scanned in, then the wait on row `row` of
/// "After how long?".
fn recovery_path(h: &mut Harness, key: &str, row: usize) {
    scan_key(h, key);
    h.tap(ids::BUILD_LATER_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Delay));
    h.choose(
        ids::at(ids::BUILD_DELAY_BASE, row),
        ids::BUILD_DELAY_CONTINUE,
    );
}

/// "Yes" on "Another path later?", and on to the next path's keys.
fn another_path(h: &mut Harness) {
    assert_eq!(h.app.build_step(), Some(Step::Another));
    h.choose(ids::BUILD_ANOTHER_YES, ids::BUILD_ANOTHER_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Later));
}

/// "No" on the same question, and on to the form.
fn no_more_paths(h: &mut Harness, script: usize) {
    assert_eq!(h.app.build_step(), Some(Step::Another));
    h.choose(ids::BUILD_ANOTHER_NO, ids::BUILD_ANOTHER_CONTINUE);
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, script),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Review));
}

/// Types `digits` on the Days pad.
fn type_days(h: &mut Harness, digits: &str) {
    for c in digits.chars() {
        h.pad(ids::BUILD_DAYS_PAD, KeyInput::Char(c));
    }
}

/// Where a text sits among the texts on screen.
fn place(h: &Harness, text: &str) -> usize {
    h.app
        .texts()
        .iter()
        .position(|t| t == text)
        .unwrap_or_else(|| panic!("{text:?} is not on screen: {:?}", h.app.texts()))
}

/// A recovery wallet can have a second path that opens later than the
/// first: one key now, one after 30 days, one after a year. The review
/// states the two waits shortest first, and the wallet is the policy
/// those three paths make.
#[test]
fn a_second_recovery_path_opens_after_the_first() {
    for (script, form) in [
        (SEGWIT, osk_bip::recovery::Form::SegWit),
        (TAPROOT, osk_bip::recovery::Form::Taproot),
    ] {
        let mut h = Harness::new(PANEL);
        recovery_now(&mut h);
        recovery_path(&mut h, &cosigners()[0], 0);
        another_path(&mut h);
        recovery_path(&mut h, &cosigners()[1], 3);
        no_more_paths(&mut h, script);

        let first = place(&h, &strings::fill1(strings::EN.wallet_recovery_after, "30"));
        let second = place(
            &h,
            &strings::fill1(strings::EN.wallet_recovery_after, "365"),
        );
        assert!(first < second, "the waits are not shortest first");

        h.tap(ids::BUILD_ADD_WALLET);
        let built = h.app.wallet_policy(0).expect("the built wallet").clone();
        let read = osk_bip::recovery::RecoveryPolicy::from_wallet_policy(&built)
            .expect("a wallet with two timelocked paths");
        assert_eq!(read.recovery.len(), 2);
        assert_eq!(read.recovery[0].delay, 30 * 144);
        assert_eq!(read.recovery[1].delay, 365 * 144);
        assert_eq!(
            built.to_descriptor(),
            read.to_wallet_policy(form)
                .expect("the same policy")
                .to_descriptor()
        );
    }
}

/// A second path's wait must be longer than the first's: the waits at
/// or below it cannot be chosen and say why, and the chevron off that
/// path's keys goes back to the question that offered it.
#[test]
fn a_later_paths_wait_cannot_be_the_earlier_paths_or_shorter() {
    let mut h = Harness::new(PANEL);
    recovery_now(&mut h);
    recovery_path(&mut h, &cosigners()[0], 0);
    another_path(&mut h);

    h.tap(ids::BACK);
    assert_eq!(h.app.build_step(), Some(Step::Another));
    another_path(&mut h);

    scan_key(&mut h, &cosigners()[1]);
    h.tap(ids::BUILD_LATER_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Delay));
    assert!(
        h.app
            .texts()
            .contains(&strings::fill1(strings::EN.build_delay_too_short, "30")),
        "the reason: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.reveal(ids::at(ids::BUILD_DELAY_BASE, 0)).is_none(),
        "30 days again can be chosen"
    );
    assert!(
        h.app.reveal(ids::at(ids::BUILD_DELAY_BASE, 1)).is_some(),
        "90 days cannot be chosen"
    );
}

/// The wizard gathers three recovery paths, and after the third it asks
/// no more: it goes straight to the form.
#[test]
fn the_wizard_asks_for_no_fourth_path() {
    let mut h = Harness::new(PANEL);
    recovery_now(&mut h);
    recovery_path(&mut h, &cosigners()[0], 0);
    another_path(&mut h);
    recovery_path(&mut h, &cosigners()[1], 1);
    another_path(&mut h);
    recovery_path(&mut h, &fourth_key(), 3);
    assert_eq!(h.app.build_step(), Some(Step::Script));

    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, SEGWIT),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    h.tap(ids::BUILD_ADD_WALLET);
    let built = h.app.wallet_policy(0).expect("the built wallet").clone();
    let read = osk_bip::recovery::RecoveryPolicy::from_wallet_policy(&built)
        .expect("a wallet with three timelocked paths");
    let delays: Vec<u32> = read.recovery.iter().map(|r| r.delay).collect();
    assert_eq!(delays, vec![30 * 144, 90 * 144, 365 * 144]);
}

/// A wait can be a number of days of one's own: 45 days is 6 480
/// blocks, and that is what the wallet holds and what the review says.
#[test]
fn a_wait_can_be_typed_in_days() {
    let mut h = Harness::new(PANEL);
    recovery_now(&mut h);
    scan_key(&mut h, &cosigners()[0]);
    h.tap(ids::BUILD_LATER_CONTINUE);
    h.choose(ids::BUILD_DELAY_TYPE, ids::BUILD_DELAY_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Days));

    type_days(&mut h, "45");
    let blocks = strings::fill1(
        strings::EN.build_delay_blocks,
        &format!("6{}480", osk_ui::tokens::GROUP_SEPARATOR),
    );
    assert!(
        h.app.texts().contains(&blocks),
        "what 45 days is in blocks: {:?}",
        h.app.texts()
    );
    h.pad(ids::BUILD_DAYS_PAD, KeyInput::Done);
    no_more_paths(&mut h, SEGWIT);
    assert!(
        h.app
            .texts()
            .contains(&strings::fill1(strings::EN.wallet_recovery_after, "45")),
        "the review names the typed wait: {:?}",
        h.app.texts()
    );

    h.tap(ids::BUILD_ADD_WALLET);
    let built = h.app.wallet_policy(0).expect("the built wallet").clone();
    let read = osk_bip::recovery::RecoveryPolicy::from_wallet_policy(&built)
        .expect("a wallet with a timelocked path");
    assert_eq!(read.recovery[0].delay, 6_480);
}

/// A typed wait no timelock can hold, and one no later than the path
/// before it, are both refused where they were typed.
#[test]
fn a_typed_wait_outside_what_the_wallet_can_hold_is_refused() {
    let mut h = Harness::new(PANEL);
    recovery_now(&mut h);
    scan_key(&mut h, &cosigners()[0]);
    h.tap(ids::BUILD_LATER_CONTINUE);
    h.choose(ids::BUILD_DELAY_TYPE, ids::BUILD_DELAY_CONTINUE);

    type_days(&mut h, "0");
    assert!(
        h.app
            .texts()
            .contains(&strings::fill1(strings::EN.build_delay_too_short, "0")),
        "no wait at all: {:?}",
        h.app.texts()
    );
    assert!(
        h.app
            .key_rect(ids::BUILD_DAYS_PAD, KeyInput::Done)
            .is_none(),
        "a wait of no days can be taken"
    );

    h.pad(ids::BUILD_DAYS_PAD, KeyInput::Backspace);
    type_days(&mut h, "456");
    assert!(
        h.app
            .texts()
            .contains(&strings::fill1(strings::EN.build_days_too_many, "455")),
        "the bound: {:?}",
        h.app.texts()
    );
    assert!(
        h.app
            .key_rect(ids::BUILD_DAYS_PAD, KeyInput::Done)
            .is_none(),
        "a wait longer than a timelock holds can be taken"
    );

    // 30 days again on a path that follows a 30-day one is refused on
    // the same line.
    h.pad(ids::BUILD_DAYS_PAD, KeyInput::Backspace);
    h.pad(ids::BUILD_DAYS_PAD, KeyInput::Backspace);
    h.pad(ids::BUILD_DAYS_PAD, KeyInput::Backspace);
    type_days(&mut h, "30");
    h.pad(ids::BUILD_DAYS_PAD, KeyInput::Done);
    another_path(&mut h);
    scan_key(&mut h, &cosigners()[1]);
    h.tap(ids::BUILD_LATER_CONTINUE);
    h.choose(ids::BUILD_DELAY_TYPE, ids::BUILD_DELAY_CONTINUE);
    type_days(&mut h, "30");
    assert!(
        h.app
            .texts()
            .contains(&strings::fill1(strings::EN.build_delay_too_short, "30")),
        "the reason: {:?}",
        h.app.texts()
    );
    assert!(
        h.app
            .key_rect(ids::BUILD_DAYS_PAD, KeyInput::Done)
            .is_none(),
        "a second path can open with the first"
    );
}

/// A key's page › "Add a wallet", at "Passphrase?" with No checked.
fn open_builder_from_key(h: &mut Harness, key: usize) {
    h.open_key(key);
    h.tap(ids::DETAIL_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Build);
    assert_eq!(h.app.build_step(), Some(Step::Passphrase));
    assert_eq!(
        h.app.checked_rows(),
        vec![String::from(strings::EN.build_passphrase_no)]
    );
}

/// From a key's page, a single-sig wallet over that key is three taps
/// past "Passphrase?": the kind, the script type and the review's
/// action, with no Keys step between (§16.129 rules 1 to 3).
#[test]
fn a_single_sig_wallet_from_a_key_is_three_taps_past_the_passphrase() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    let key = h.app.fingerprints()[0];
    open_builder_from_key(&mut h, 0);
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Kind));

    h.tap(ids::BUILD_KIND_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Script));
    h.tap(ids::BUILD_SCRIPT_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Review));
    let texts = h.app.texts();
    assert!(
        texts.contains(&hex(key)),
        "the review is the key's wallet: {texts:?}"
    );
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert_eq!(h.app.wallet_count(), 1);
}

/// Yes on "Passphrase?" opens the key's passphrase entry, titled as a
/// new key; its ✓ puts the passphrase key on Keys beside the key it was
/// opened from, and the wallet is built over the new key.
#[test]
fn a_passphrase_on_the_way_adds_a_key_and_the_wallet_is_over_it() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    let parent = h.app.fingerprints()[0];
    open_builder_from_key(&mut h, 0);
    h.tap(ids::BUILD_PASSPHRASE_YES);
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::OpenPassphrase);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.build_passphrase_key_title)
    );

    // Back from the entry is the question, with Yes still checked.
    h.tap(ids::BACK);
    assert_eq!(h.app.build_step(), Some(Step::Passphrase));
    assert_eq!(
        h.app.checked_rows(),
        vec![String::from(strings::EN.build_passphrase_yes)]
    );
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);

    h.type_text("passphrase");
    h.send(Event::Key(Key::Enter));
    assert_eq!(h.app.build_step(), Some(Step::Kind));
    let keys = h.app.fingerprints();
    assert_eq!(keys.len(), 2, "the passphrase key is on Keys as a peer");
    assert_eq!(keys[0], parent);
    let opened = keys[1];

    h.tap(ids::BUILD_KIND_CONTINUE);
    h.tap(ids::BUILD_SCRIPT_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Review));
    let texts = h.app.texts();
    assert!(
        texts.contains(&hex(opened)),
        "the wallet is over the new key: {texts:?}"
    );
    assert!(
        !texts.contains(&hex(parent)),
        "and not over the key it was opened from: {texts:?}"
    );
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.wallet_count(), 1);
}

/// A kind of more than one key, from a key's page, opens its Keys step
/// with that key checked, and gathers the rest there.
#[test]
fn a_multisig_from_a_key_opens_its_keys_with_the_key_checked() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder_from_key(&mut h, 0);
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);
    choose_kind(&mut h, MULTISIG);
    assert_eq!(h.app.checked_rows(), vec![hex(h.app.fingerprints()[0])]);
    for key in cosigners() {
        scan_key(&mut h, &key);
    }
    h.tap(ids::BUILD_CONTINUE);
    h.choose(
        ids::at(ids::BUILD_SCRIPT_BASE, SEGWIT),
        ids::BUILD_SCRIPT_CONTINUE,
    );
    h.choose(
        ids::at(ids::BUILD_THRESHOLD_BASE, 1),
        ids::BUILD_THRESHOLD_CONTINUE,
    );
    assert_eq!(h.app.build_step(), Some(Step::Review));
    h.tap(ids::BUILD_ADD_WALLET);
    h.tap(ids::WALLET_EXPORT);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t.contains(&policy().checksum())),
        "the fixture's wallet: {:?}",
        h.app.texts()
    );
}

/// Back from "Passphrase?" and from the kind step is the key's page;
/// the discard question is asked only once keys beyond the one in hand
/// have been gathered (§16.129 rule 5).
#[test]
fn back_from_a_wallet_started_on_a_key_is_the_key_page() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    open_builder_from_key(&mut h, 0);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);

    h.tap(ids::DETAIL_ADD_WALLET);
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);
    assert_eq!(h.app.build_step(), Some(Step::Kind));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);

    // The key in hand alone asks nothing.
    h.tap(ids::DETAIL_ADD_WALLET);
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);
    choose_kind(&mut h, MULTISIG);
    h.tap(ids::BACK);
    assert_eq!(h.app.build_step(), Some(Step::Kind));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);

    // A cosigner beyond it does.
    h.tap(ids::DETAIL_ADD_WALLET);
    h.tap(ids::BUILD_PASSPHRASE_CONTINUE);
    choose_kind(&mut h, MULTISIG);
    scan_key(&mut h, &cosigners()[0]);
    h.tap(ids::BACK);
    h.tap(ids::BACK);
    assert_eq!(h.app.build_step(), Some(Step::Discard));
    h.tap(ids::BUILD_DISCARD);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert_eq!(h.app.wallet_count(), 0);
}
