//! A silent payments wallet, end to end (`docs/PLANNING.md` §16.113):
//! the address it publishes, the labels it hands out, the transaction it
//! says paid it, the three forms it exports, and what comes back after a
//! restart on a device that keeps keys.
//!
//! The transactions here are built by the module's own sending side: a
//! payer spends a P2WPKH output with a known key and pays the address
//! this device published. That is the check that matters — a payment
//! made to this wallet is a payment this wallet finds — and it is the
//! same arithmetic `core/osk-bip/tests/silent.rs` holds against every
//! case of BIP-352's published vectors.

mod common;

use common::{ABANDON, Element, Harness, PANEL, PIN, SECURE_PHONE};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::key::TapTweak;
use osk_bip::bitcoin::secp256k1::{Secp256k1, SecretKey};
use osk_bip::bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute, consensus,
    transaction,
};
use osk_bip::keys::{MasterKey, Network};
use osk_bip::silent::{self, Receiver, SendKey};
use osk_shell_api::{Event, FileKind};

/// Silent payments' place in the kind Choice, which is last.
const SILENT: usize = 6;

/// The address the twelve test words publish on mainnet, computed once
/// from BIP-352's own paths. `core/osk-bip/tests/silent.rs` checks the
/// arithmetic behind it against every published vector; this states the
/// one string this device shows for the key every other test loads, so
/// that a change in the derivation or the encoding is caught here too.
const ADDRESS: &str = "sp1qqfqnnv8czppwysafq3uwgwvsc638hc8rx3hscuddh0xa2yd746s7xqh6yy9ncjnqhqxazct0fzh98w7lpkm5fvlepqec2yy0sxlq4j6ccc3h6t0g";

/// The receiver the device derives, computed here from the same words.
fn receiver() -> (MasterKey, Receiver) {
    let words = ABANDON.join(" ");
    let m = Mnemonic::parse(Language::English, &words).expect("the test words");
    let master = MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Mainnet);
    let receiver = Receiver::derive(&master, 0);
    (master, receiver)
}

/// The key a payer spends with, and the P2WPKH output it pays from.
fn payer() -> (SecretKey, ScriptBuf) {
    let secp = Secp256k1::new();
    let key = SecretKey::from_slice(&[0x11u8; 32]).expect("a key");
    let public = osk_bip::bitcoin::PublicKey::new(key.public_key(&secp));
    let script = ScriptBuf::new_p2wpkh(&public.wpubkey_hash().expect("compressed"));
    (key, script)
}

/// The transaction that funded the payer, which is what "Check a
/// payment" asks for so that the input's own output is known.
fn funding(script: &ScriptBuf) -> Transaction {
    Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(200_000),
            script_pubkey: script.clone(),
        }],
    }
}

/// A transaction paying `spend_keys` — the spend key of the address or
/// of one of its labels — from that funding output, with one output of
/// someone else's beside it.
fn payment(spend_keys: &[osk_bip::bitcoin::secp256k1::PublicKey]) -> (Transaction, Transaction) {
    let secp = Secp256k1::new();
    let (key, script) = payer();
    let previous = funding(&script);
    let outpoint = OutPoint {
        txid: previous.compute_txid(),
        vout: 0,
    };
    let mut serialized = [0u8; 36];
    serialized[..32].copy_from_slice(&consensus::serialize(&outpoint.txid));
    serialized[32..].copy_from_slice(&0u32.to_le_bytes());
    let (_, r) = receiver();
    let scan = r.scan_public_key(&secp);
    let outputs = silent::sender_outputs(
        &secp,
        &[SendKey {
            key,
            taproot: false,
        }],
        &[serialized],
        &scan,
        spend_keys,
    )
    .expect("the payer works out the outputs");
    // A witness the receiver reads the public key out of: BIP-352 takes
    // the last item, and the signature before it is never looked at.
    let mut witness = Witness::new();
    witness.push([0x30u8; 71]);
    witness.push(key.public_key(&secp).serialize());
    let mut out: Vec<TxOut> = outputs
        .iter()
        .map(|key| TxOut {
            value: Amount::from_sat(150_000),
            script_pubkey: ScriptBuf::new_p2tr_tweaked(
                osk_bip::bitcoin::XOnlyPublicKey::from_slice(key)
                    .expect("a key")
                    .dangerous_assume_tweaked(),
            ),
        })
        .collect();
    out.push(TxOut {
        value: Amount::from_sat(40_000),
        script_pubkey: script.clone(),
    });
    let tx = Transaction {
        version: transaction::Version::TWO,
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: outpoint,
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness,
        }],
        output: out,
    };
    (tx, previous)
}

/// The loaded test key, on mainnet, which is where the address the file
/// states is published.
fn with_key(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
    assert_eq!(h.app.network(), Network::Mainnet);
}

/// Add a wallet › Silent payments over the loaded key, accepted.
fn add_wallet(h: &mut Harness) {
    h.open_add_wallet(ids::BUILD_NEW);
    h.choose(
        ids::at(ids::BUILD_KIND_BASE, SILENT),
        ids::BUILD_KIND_CONTINUE,
    );
    h.tap(ids::at(ids::BUILD_WHICH_BASE, 0));
    h.tap(ids::BUILD_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::Build, "the review");
    h.tap(ids::BUILD_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet, "the wallet's own page");
}

/// A file read at the scanner the row opened.
fn read_file(h: &mut Harness, row: ids::Id, bytes: &[u8]) {
    h.tap(row);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: bytes.to_vec(),
    });
}

/// Whether one of the strings on screen is `want`. A long string is a
/// reference row wherever it stands inside another screen, so it is
/// chunked in fours and elided in the middle; the screen that shows it
/// whole writes it as it is.
fn shows(texts: &[String], want: &str) -> bool {
    texts.iter().any(|t| {
        let t: String = t.chars().filter(|c| *c != ' ').collect();
        match t.split_once('\u{2026}') {
            Some((head, tail)) => {
                !head.is_empty() && want.starts_with(head) && want.ends_with(tail)
            }
            None => t == want,
        }
    })
}

/// The wallet is the two keys BIP-352 derives from the chosen key, and
/// the address it publishes is the one the arithmetic gives.
#[test]
fn the_wallet_publishes_the_address_the_key_derives() {
    let secp = Secp256k1::new();
    let (master, r) = receiver();
    let address = r.address(&secp).expect("an address");
    assert_eq!(address.as_str(), ADDRESS, "the address the words publish");
    // The two halves are the key at BIP-352's own paths, read again
    // through plain BIP-32 derivation.
    let scan = master.derive(&silent::scan_path(Network::Mainnet, 0));
    let (_, decoded_scan, decoded_spend) =
        silent::decode_address(ADDRESS).expect("the address reads");
    assert_eq!(decoded_scan, scan.secret_key().public_key(&secp));
    assert_eq!(decoded_spend, r.spend_public_key());

    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    add_wallet(&mut h);
    h.tap(ids::SILENT_ADDRESS);
    assert_eq!(h.app.screen(), ScreenKind::SilentAddress);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == ADDRESS),
        "the address whole: {texts:?}"
    );
    assert!(
        texts.contains(&String::from(strings::EN.silent_address_label)),
        "the label under the code: {texts:?}"
    );

    // The Format row shows the BIP-321 URI in place of the address.
    h.tap(ids::SILENT_FORM);
    h.choose(ids::at(ids::PICK_BASE, 1), ids::PICK_CONTINUE);
    let texts = h.app.texts();
    let uri = format!("bitcoin:?sp={ADDRESS}");
    assert!(shows(&texts, &uri), "the URI: {texts:?}");
}

/// Wallets names the kind, and the page carries the rows a silent
/// payments wallet has and none of the ones it has not.
#[test]
fn the_wallet_page_carries_its_own_rows_and_no_sign_row() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    add_wallet(&mut h);
    for row in [
        ids::SILENT_ADDRESS,
        ids::SILENT_LABELS,
        ids::SILENT_CHECK,
        ids::WALLET_EXPORT,
        ids::WALLET_KEYS,
        ids::WALLET_FORGET,
    ] {
        assert!(h.app.rect_of(row).is_some(), "{row:?} is on the page");
    }
    for row in [
        ids::WALLET_SIGN,
        ids::WALLET_SIGN_MESSAGE,
        ids::WALLET_ADDRESSES,
        ids::WALLET_CHECK,
        ids::WALLET_SHEET,
    ] {
        assert!(h.app.rect_of(row).is_none(), "{row:?} is not on the page");
    }
    // The key it was built on is its one member, with the key glyph.
    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLET_KEY_ROW_BASE, 0))
            .is_some()
    );
    h.tap(ids::BACK);

    h.go_home();
    h.open_wallets();
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with(strings::EN.wallet_kind_silent)),
        "the Wallets row names the kind: {texts:?}"
    );
    assert!(
        texts.contains(&String::from(strings::EN.wallet_silent)),
        "and what it is called: {texts:?}"
    );
}

/// A label is an address of its own, found by the same scan key.
#[test]
fn a_label_is_an_address_of_its_own() {
    let secp = Secp256k1::new();
    let (_, r) = receiver();
    let first = r.labelled_address(&secp, 1).expect("label 1");
    assert_ne!(first.as_str(), ADDRESS, "a label is another address");
    let (_, scan, spend) = silent::decode_address(first.as_str()).expect("it reads");
    assert_eq!(scan, r.scan_public_key(&secp), "the same scan key");
    assert_ne!(spend, r.spend_public_key(), "a different spend key");

    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    add_wallet(&mut h);
    h.tap(ids::SILENT_LABELS);
    assert_eq!(h.app.screen(), ScreenKind::SilentLabels);
    // A wallet that has handed out no label lists none.
    assert!(h.app.rect_of(ids::at(ids::SILENT_LABEL_BASE, 0)).is_none());
    h.tap(ids::SILENT_ADD_LABEL);
    let texts = h.app.texts();
    let name = strings::fill1(strings::EN.silent_label, "1");
    assert!(texts.contains(&name), "the label's row: {texts:?}");
    assert!(shows(&texts, first.as_str()), "with its address: {texts:?}");
    h.tap(ids::at(ids::SILENT_LABEL_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::SilentAddress);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == first.as_str()),
        "the label's own Address screen: {texts:?}"
    );
}

/// A transaction that pays this wallet is read as paid, with the output
/// it pays, the amount and the label; one that pays someone else is not.
#[test]
fn a_payment_to_this_wallet_is_found_and_another_is_not() {
    let secp = Secp256k1::new();
    let (_, r) = receiver();
    let (tx, previous) = payment(&[r.spend_public_key()]);

    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    add_wallet(&mut h);
    h.tap(ids::SILENT_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::SilentCheck);
    read_file(&mut h, ids::SILENT_READ, &consensus::serialize(&tx));
    // A raw transaction states no previous output, so the flow asks for
    // the transaction that made the one its input spends.
    assert_eq!(h.app.screen(), ScreenKind::SilentCheck);
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.silent_previous_row)),
        "the previous transaction is asked for: {texts:?}"
    );
    read_file(
        &mut h,
        ids::SILENT_PREVIOUS,
        &consensus::serialize(&previous),
    );

    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.silent_paid)),
        "the result: {texts:?}"
    );
    assert!(
        texts.contains(&strings::fill1(strings::EN.silent_output_row, "0")),
        "which output: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("150\u{a0}000")),
        "and what it pays: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|t| t.contains(strings::EN.silent_output_plain)),
        "paid to the address itself: {texts:?}"
    );
    h.tap(ids::SILENT_DONE);

    // The same payer paying somebody else's address.
    let other = SecretKey::from_slice(&[0x22u8; 32]).expect("a key");
    let (tx, previous) = payment(&[other.public_key(&secp)]);
    h.tap(ids::SILENT_CHECK);
    read_file(&mut h, ids::SILENT_READ, &consensus::serialize(&tx));
    read_file(
        &mut h,
        ids::SILENT_PREVIOUS,
        &consensus::serialize(&previous),
    );
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.silent_not_paid)),
        "not paid here: {texts:?}"
    );
}

/// A payment to a label is found under that label.
#[test]
fn a_payment_to_a_label_names_the_label() {
    let secp = Secp256k1::new();
    let (_, r) = receiver();
    let spend = r.labelled_spend_key(&secp, 1).expect("label 1");
    let (tx, previous) = payment(&[spend]);

    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    add_wallet(&mut h);
    h.tap(ids::SILENT_LABELS);
    h.tap(ids::SILENT_ADD_LABEL);
    h.tap(ids::BACK);
    h.tap(ids::SILENT_CHECK);
    read_file(&mut h, ids::SILENT_READ, &consensus::serialize(&tx));
    read_file(
        &mut h,
        ids::SILENT_PREVIOUS,
        &consensus::serialize(&previous),
    );
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.silent_paid)),
        "the result: {texts:?}"
    );
    let label = strings::fill1(strings::EN.silent_output_label, "1");
    assert!(
        texts.iter().any(|t| t.contains(&label)),
        "under its label: {texts:?}"
    );
}

/// The three forms the wallet exports: the scan descriptor on a Secret
/// screen, the URI, and the DNS record under a name and a domain.
#[test]
fn the_wallet_exports_a_scan_descriptor_a_uri_and_a_record() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    add_wallet(&mut h);
    let fingerprint = h.app.fingerprints()[0];
    h.tap(ids::WALLET_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.export_silent_scan)),
        "the export opens on the scan descriptor: {texts:?}"
    );
    h.tap(ids::SILENT_SECRET_ROW);
    assert_eq!(h.app.screen(), ScreenKind::SilentSecret);
    h.hold(ids::SILENT_REVEAL);
    h.press(ids::SILENT_REVEAL);
    let texts = h.app.texts();
    let origin = format!("sp([{fingerprint}/352h/0h/0h]spscan1q");
    assert!(
        texts.iter().any(|t| t.starts_with(&origin)),
        "the descriptor with its key origin: {texts:?}"
    );
    h.tap(ids::BACK);

    // The URI.
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, 8), ids::PICK_CONTINUE);
    let texts = h.app.texts();
    assert!(
        shows(&texts, &format!("bitcoin:?sp={ADDRESS}")),
        "the URI: {texts:?}"
    );

    // The record, which asks for the two names it is published under.
    h.tap(ids::EXPORT_FORMAT);
    h.choose(ids::at(ids::PICK_BASE, 9), ids::PICK_CONTINUE);
    assert_eq!(h.app.screen(), ScreenKind::SilentDns);
    h.type_text("alice");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::SilentDns, "then the domain");
    h.type_text("example.com");
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    let texts = h.app.texts();
    let record =
        format!("alice.user._bitcoin-payment.example.com. IN TXT \"bitcoin:?sp={ADDRESS}\"");
    assert!(shows(&texts, &record), "the record: {texts:?}");
}

/// A silent payment address read by the scanner is shown for what it
/// is: no wallet's chain of addresses holds one.
#[test]
fn a_silent_payment_address_read_by_the_scanner_is_shown_not_checked() {
    let mut h = Harness::new(PANEL);
    with_key(&mut h);
    h.open_tile(common::TILE_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: ADDRESS.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Verify);
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.verify_silent_title)),
        "what it is: {texts:?}"
    );
    assert!(shows(&texts, ADDRESS), "and the address itself: {texts:?}");
}

/// The wallet is kept with the keys on a device that keeps them, and
/// comes back with its address and its labels after a restart.
#[test]
fn a_kept_silent_wallet_comes_back_after_a_restart() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "the key is on the device");
    add_wallet(&mut h);
    h.tap(ids::SILENT_LABELS);
    h.tap(ids::SILENT_ADD_LABEL);
    h.tap(ids::SILENT_ADD_LABEL);
    let element = h.element.clone().expect("an element");

    let mut h = Harness::kept(SECURE_PHONE, element);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    h.open_policy(0);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.tap(ids::SILENT_ADDRESS);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == ADDRESS),
        "the same address: {texts:?}"
    );
    h.tap(ids::BACK);
    h.tap(ids::SILENT_LABELS);
    let texts = h.app.texts();
    assert!(
        texts.contains(&strings::fill1(strings::EN.silent_label, "2")),
        "and the labels it had handed out: {texts:?}"
    );
}
