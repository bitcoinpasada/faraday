//! Bitcoin Secure Multisig Setup in the product (BIP 129,
//! `docs/PLANNING.md` §16.106): the descriptor record read by Scan, the
//! key record refused by what it is, and the record this device's Export
//! hands the coordinator.
//!
//! The wallet is `tools/vectors/psbt/wallet-2of3.policy`, the 2-of-3 on
//! regtest one of whose keys this device holds.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::{ExportFormat, ScreenKind, ids, strings};
use osk_bip::bsms::{DescriptorRecord, SignerRecord};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_shell_api::{Event, FileKind};
use osk_ui::widgets::keyboard::KeyInput;

const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");

fn policy() -> WalletPolicy {
    WalletPolicy::parse(POLICY).expect("the committed policy")
}

/// The descriptor record a coordinator hands back for that wallet.
fn descriptor_record() -> String {
    let policy = policy();
    let address = policy
        .address_at(Network::Regtest, false, 0)
        .expect("the wallet's first address");
    alloc_format(&policy, &address.to_string())
}

fn alloc_format(policy: &WalletPolicy, address: &str) -> String {
    format!(
        "BSMS 1.0\n{}\n/0/*,/1/*\n{address}",
        policy.to_descriptor_template()
    )
}

/// A device on regtest with the test key loaded, which is one of the
/// wallet's three.
fn regtest_device() -> Harness {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    h
}

/// Scan › a file, which is how anything read reaches the app.
fn scan_text(h: &mut Harness, text: &str) {
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: text.as_bytes().to_vec(),
    });
}

/// Round 2 arrives at the wallet review, with the four facts BIP 129
/// asks every signer to check before it registers the wallet, and "Add
/// this wallet" puts it in use.
#[test]
fn a_descriptor_record_is_the_wallet_with_the_rows_the_bip_asks_for() {
    let mut h = regtest_device();
    scan_text(&mut h, &descriptor_record());
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);

    assert!(
        says(&strings::fill(strings::EN.wallet_quorum, &["2", "3"])),
        "M of N: {texts:?}"
    );
    assert!(says("/0/*,/1/*"), "the paths: {texts:?}");
    let first = policy()
        .address_at(Network::Regtest, false, 0)
        .expect("the first address")
        .to_string();
    assert!(
        texts.iter().any(|t| t
            .replace(' ', "")
            .starts_with(&first.chars().take(8).collect::<String>())),
        "the first address: {texts:?}"
    );
    // This device holds the wallet's first key, so the record's own
    // question — which of these is mine — is answered by letter.
    assert!(
        says(&strings::fill1(strings::EN.wallet_path_key, "A")),
        "this device's place: {texts:?}"
    );

    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    assert_eq!(h.app.wallet_count(), 1);
    assert_eq!(h.app.wallet_policy(0), Some(&policy()));
}

/// Round 1 is one signer's key. It is not a wallet, and the refusal
/// says so where the record was read.
#[test]
fn a_key_record_is_refused_by_what_it_is() {
    let signer = "BSMS 1.0\n00\n[73c5da0a/48'/1'/0'/2']tpubDFH9dgzveyD8zTbPUFuLrGmCydNvxehyNdUXKJAQN8x4aZ4j6UZqGfnqFrD4NqyaTVGKbvEW54tsvPTK2UoSbCC1PJY8iCNiwTL3RWZEheQ\nOpenSigner key\nsignature";
    let mut h = regtest_device();
    scan_text(&mut h, signer);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.scan_reason_bsms_signer),
        "the reason: {:?}",
        h.app.texts()
    );
}

/// Export on a multisig wallet hands the coordinator a record the
/// coordinator reads back as the same wallet.
#[test]
fn export_writes_a_record_that_parses_back_to_the_same_wallet() {
    let mut h = regtest_device();
    scan_text(&mut h, &descriptor_record());
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_EXPORT);
    let bsms = ExportFormat::ALL
        .iter()
        .position(|f| *f == ExportFormat::Bsms)
        .expect("the format is in the list");
    h.tap(ids::EXPORT_FORMAT);
    h.tap(ids::at(ids::PICK_BASE, bsms));
    h.tap(ids::PICK_CONTINUE);

    let written = h.app.export_string().expect("the record");
    let read = DescriptorRecord::parse(&written).expect("what it wrote");
    assert_eq!(read.policy, policy());
    assert_eq!(read.paths, ["/0/*", "/1/*"]);

    // The info button on that screen is about the files a coordinator
    // and a signer exchange, not about what an xpub shows.
    h.tap(ids::INFO);
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| *t == strings::EN.learn.coordinators.title),
        "the page it opened: {:?}",
        h.app.texts()
    );
}

/// The review of a wallet read from a record is about the same page.
#[test]
fn the_review_of_a_record_is_about_coordinator_files() {
    let mut h = regtest_device();
    scan_text(&mut h, &descriptor_record());
    h.tap(ids::INFO);
    assert_eq!(h.app.screen(), ScreenKind::LearnPage);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| *t == strings::EN.learn.coordinators.title),
        "the page it opened: {:?}",
        h.app.texts()
    );
}

// ----- Round 1: the key record this device writes (§16.110 rule 2) -----

/// The Choice's SegWit multisig row, which is BIP 48's `2'`.
const SEGWIT_MULTISIG: usize = 5;

/// The key's page › Account key › SegWit multisig › BSMS key record,
/// which lands on the session token.
fn start_key_record(h: &mut Harness) {
    h.open_key(0);
    h.tap(ids::DETAIL_ACCOUNT);
    h.choose(ids::at(ids::PICK_BASE, SEGWIT_MULTISIG), ids::PICK_CONTINUE);
    let row = ExportFormat::ALL
        .iter()
        .position(|f| *f == ExportFormat::BsmsSigner)
        .expect("the format is in the list");
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, row), ids::PICK_CONTINUE);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.bsms_token_title),
        "the token is asked for first: {:?}",
        h.app.texts()
    );
}

fn type_on(h: &mut Harness, keyboard: osk_ui::Id, text: &str) {
    for c in text.chars() {
        h.pad(keyboard, KeyInput::Char(c));
    }
}

/// The record with `token`, the description left as the fingerprint the
/// field arrives with.
fn key_record(h: &mut Harness, token: Option<&str>) -> String {
    start_key_record(h);
    match token {
        None => h.tap(ids::BSMS_TOKEN_NONE),
        Some(t) => type_on(h, ids::BSMS_TOKEN_KEYBOARD, t),
    }
    h.pad(ids::BSMS_TOKEN_KEYBOARD, KeyInput::Done);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.bsms_description_title),
        "then the description: {:?}",
        h.app.texts()
    );
    h.pad(ids::BSMS_DESCRIPTION_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    h.app.export_string().expect("the record")
}

/// The record a coordinator gets: it parses, its signature holds
/// against the key it names, and that key is the account key the other
/// format on the same screen shows.
#[test]
fn the_key_record_parses_verifies_and_names_the_account_key() {
    let mut h = regtest_device();
    let written = key_record(&mut h, None);

    let record = SignerRecord::parse(&written).expect("what it wrote");
    record.verify().expect("its own signature");
    assert_eq!(record.token, osk_bip::bsms::NO_ENCRYPTION);
    // The field arrives with this device's fingerprint, and ✓ takes it
    // as it stands.
    assert_eq!(record.description, "73c5da0a");

    // The same screen's other format is the same key.
    let row = ExportFormat::ALL
        .iter()
        .position(|f| *f == ExportFormat::Xpub)
        .expect("the account key is always a format");
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, row), ids::PICK_CONTINUE);
    let account = h.app.export_string().expect("the account key");
    // The record writes BIP 129's hardened apostrophe, because the
    // signature is over the text as the BIP's own vectors write it;
    // every other screen of this app writes `h`.
    assert_eq!(account, record.key.key_text().replace('\'', "h"));
}

/// The None row above the field writes the token that says the session
/// is not encrypted, and a token the coordinator gave is written into
/// the record as it stands.
#[test]
fn the_token_is_none_or_the_coordinators_nonce() {
    let mut h = regtest_device();
    start_key_record(&mut h);
    h.tap(ids::BSMS_TOKEN_NONE);
    assert!(
        h.app.texts().iter().any(|t| t == "00"),
        "the preset filled the field: {:?}",
        h.app.texts()
    );

    let mut h = regtest_device();
    let written = key_record(&mut h, Some("A54044308CEAC9B7"));
    let record = SignerRecord::parse(&written).expect("what it wrote");
    assert_eq!(record.token, "a54044308ceac9b7");
    record.verify().expect("its own signature");
    assert!(
        written.lines().nth(1) == Some("a54044308ceac9b7"),
        "the token is the second line: {written}"
    );
}

/// BIP 129 allows eighty characters, and a longer description is
/// refused where it is typed.
#[test]
fn a_description_over_eighty_characters_is_refused() {
    let s = &strings::EN;
    let mut h = regtest_device();
    start_key_record(&mut h);
    h.tap(ids::BSMS_TOKEN_NONE);
    h.pad(ids::BSMS_TOKEN_KEYBOARD, KeyInput::Done);

    // The field arrives with eight characters; seventy-three more make
    // eighty-one.
    type_on(&mut h, ids::BSMS_DESCRIPTION_KEYBOARD, &"x".repeat(73));
    assert!(
        h.app.texts().iter().any(|t| t == s.bsms_description_long),
        "the limit is stated: {:?}",
        h.app.texts()
    );
    assert!(
        h.app
            .key_rect(ids::BSMS_DESCRIPTION_KEYBOARD, KeyInput::Done)
            .is_none(),
        "and there is no way on: {:?}",
        h.app.texts()
    );

    // One character back is eighty, which is written.
    h.pad(ids::BSMS_DESCRIPTION_KEYBOARD, KeyInput::Backspace);
    h.pad(ids::BSMS_DESCRIPTION_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    let record = SignerRecord::parse(&h.app.export_string().expect("the record")).expect("it");
    assert_eq!(record.description.chars().count(), 80);
    record.verify().expect("its own signature");
}
