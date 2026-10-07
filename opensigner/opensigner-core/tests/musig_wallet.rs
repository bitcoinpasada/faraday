//! A BIP-390 MuSig2 wallet, loaded the way any other wallet is
//! (`docs/PLANNING.md` §16.70): what its review says, what its menu is
//! called, the addresses it lists, what it exports, and what forgetting
//! it leaves.
//!
//! The wallet is `tr(musig(A,B)/**)` over BIP-390's two vector extended
//! public keys, so its receive addresses are the aggregate keys the BIP
//! publishes for `rawtr(musig(A,B)/0/*)`, each taproot-tweaked as `tr()`
//! pays. The keys are mainnet, which is the device's own default.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::verify::AddressResult;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::Network;
use osk_bip::policy::WalletPolicy;
use osk_psbt::bitcoin::secp256k1::PublicKey;
use osk_shell_api::{App, Event, FileKind, TouchPhase};

const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-musig.policy");

/// The first three receive addresses of the wallet: BIP-390's published
/// aggregate keys for indices 0, 1 and 2 under the taproot tweak.
const RECEIVE: [&str; 3] = [
    "bc1pn0wx4shq5asjmlvdt7a33edrqsw9aw6c70wl4x590dm69u4ud97q35plfv",
    "bc1pvax58twf3wukd57fs0rgdna0nejtxwwaztrnxh32nfxngv4w6ymsuppwd6",
    "bc1ptfpkgnexaa07wad6hm6wy7fe8vfdu78fr5xt2khw003ef84apatqne7g7f",
];

fn policy() -> WalletPolicy {
    WalletPolicy::parse(POLICY).expect("the committed policy")
}

/// "2 keys", the wallet's own count.
fn keys_value() -> String {
    strings::fill(strings::EN.wallet_musig_keys, &["2"])
}

/// Add › "Load a wallet", and the policy read from a file there.
fn load_wallet(h: &mut Harness) {
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: POLICY.as_bytes().to_vec(),
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
fn a_musig_wallet_is_reviewed_as_its_keys_and_its_script() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    assert_eq!(h.app.network(), Network::Mainnet);
    load_wallet(&mut h);
    assert_eq!(h.app.screen(), ScreenKind::Inspect);

    let texts = h.app.texts();
    let says = |t: &str| texts.iter().any(|x| x == t);
    assert!(says(strings::EN.wallet_musig), "MuSig2: {texts:?}");
    assert!(says(&keys_value()), "how many keys: {texts:?}");
    assert!(says(strings::EN.script_taproot), "the script: {texts:?}");
    assert!(says(strings::EN.inspect_checksum_ok), "{texts:?}");
    for key in policy().keys() {
        // Every key of a MuSig2 wallet carries an origin: BIP-388 writes
        // one on each, so the review states a master for each.
        let fingerprint = key.fingerprint().expect("a participant has an origin");
        assert!(
            texts.iter().any(|t| *t == fingerprint.to_string()),
            "the participant {fingerprint} is on the review: {texts:?}"
        );
    }
    assert!(says(strings::EN.wallet_use), "the way to use it: {texts:?}");
}

#[test]
fn the_musig_wallet_menu_is_named_by_its_keys_and_lists_its_addresses() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    let texts = h.app.texts();
    assert!(
        texts
            .iter()
            .any(|t| t.contains(strings::EN.wallet_musig) && t.contains(&keys_value())),
        "the menu is titled with the wallet: {texts:?}"
    );

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
    for address in RECEIVE {
        assert!(
            shows_address(&h, address),
            "{address} is on the list: {:?}",
            h.app.texts()
        );
    }

    // The change chain is the wallet's other chain, and not the receive
    // one.
    h.tap(ids::ADDR_CHANGE);
    let change = policy()
        .address_at(Network::Mainnet, true, 0)
        .unwrap()
        .to_string();
    assert!(change != RECEIVE[0], "the chains differ");
    assert!(shows_address(&h, &change), "{:?}", h.app.texts());
}

#[test]
fn the_musig_wallet_exports_its_descriptor_and_its_policy() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::WALLET_EXPORT);
    assert_eq!(h.app.screen(), ScreenKind::Export);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains(&policy().checksum())),
        "the descriptor row names its checksum: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains(
            &policy().keys()[0]
                .fingerprint()
                .expect("a participant has an origin")
                .to_string()
        )),
        "and the first fingerprint: {texts:?}"
    );

    h.tap(ids::EXPORT_FORMAT);
    h.tap(ids::at(ids::PICK_BASE, 3));
    h.tap(ids::PICK_CONTINUE);
    h.tap(ids::EXPORT_TEXT);
    let whole: String = h.app.texts().join(" ");
    assert!(
        whole.replace(' ', "").contains("tr(musig(@0,@1)/**)"),
        "the policy template, whole: {whole}"
    );
}

#[test]
fn forgetting_a_musig_wallet_removes_its_row() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    let row = ids::at(ids::WALLETS_POLICY_ROW_BASE, 0);
    assert!(
        h.app.rect_of(row).is_some(),
        "the wallet is a row on Wallets"
    );
    h.tap(row);
    h.tap(ids::WALLET_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Wallets);
    assert!(h.app.rect_of(row).is_none(), "the row is gone");
}

/// A MuSig2 wallet in use is not consulted for the change of a
/// transaction: a participant's origin names its own account key and
/// stops there, so nothing in the PSBT says which address of the wallet
/// an output is. The output stays a claim the device has not checked,
/// and the Sign flow reads the transaction as it would with no wallet at
/// all (`docs/PLANNING.md` §16.70).
#[test]
fn signing_with_a_musig_wallet_in_use_leaves_a_change_output_unverified() {
    const PSBT: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.psbt");
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);

    // This device holds none of the wallet's keys, so the wallet's page
    // has no Sign row; the transaction arrives at the scanner.
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: osk_psbt::Psbt::parse_base64(PSBT.trim())
            .expect("the committed PSBT")
            .to_bytes(),
    });
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(matches!(
        insp.outputs[1].kind,
        osk_psbt::OutputKind::UnverifiedChange { .. }
    ));
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.sign_change_unverified),
        "{:?}",
        h.app.texts()
    );
}

/// Verify answers for an address of a MuSig2 wallet in use: the check
/// derives the wallet's own addresses, which a MuSig2 wallet has like
/// any other.
#[test]
fn verify_answers_for_an_address_of_a_musig_wallet_in_use() {
    let mut h = Harness::new(PANEL);
    load_wallet(&mut h);
    h.tap(ids::INSPECT_USE_WALLET);
    h.open_policy(0);
    h.tap(ids::WALLET_CHECK);
    assert_eq!(h.app.screen(), ScreenKind::Scan);
    h.tap(ids::SCAN_TYPE);
    h.type_text(RECEIVE[2]);
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

// ---------------------------------------------------------------------
// Signing a MuSig2 wallet this device holds a key of
// (docs/PLANNING.md §16.100)
// ---------------------------------------------------------------------

/// The regtest wallet of the Bitcoin Core round trip: the "abandon …
/// about" key as `@0`, Core's key as `@1`.
const REGTEST_POLICY: &str =
    include_str!("../../../tools/vectors/psbt/wallet-musig-regtest.policy");
/// Core's round-1 PSBT of that wallet: Core's nonce is on it, this
/// device's is not.
const CORE_FIRST: &str = include_str!("../../../tools/vectors/psbt/wallet-musig-core-first.psbt");
/// The same spend before anyone's nonce.
const DEVICE_FIRST: &str =
    include_str!("../../../tools/vectors/psbt/wallet-musig-device-first.psbt");

fn psbt_bytes(text: &str) -> Vec<u8> {
    osk_psbt::Psbt::parse_base64(text.trim())
        .expect("the committed PSBT")
        .to_bytes()
}

/// The device with the key loaded on regtest and the regtest wallet in
/// use, at the scanner, ready for a transaction.
fn regtest_wallet_at_the_scanner(h: &mut Harness) {
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.set_network(3);
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: REGTEST_POLICY.as_bytes().to_vec(),
    });
    h.tap(ids::INSPECT_USE_WALLET);
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
}

/// The review of a MuSig2 spend names the wallet by its keys, offers the
/// participating key to sign with, and ends in a partial signature: one
/// of the wallet's two, because the other signer aggregates.
#[test]
fn a_musig_spend_is_reviewed_and_partially_signed() {
    let mut h = Harness::new(PANEL);
    regtest_wallet_at_the_scanner(&mut h);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(CORE_FIRST),
    });
    let insp = h.app.sign_inspection().expect("inspected");
    assert_eq!(insp.musig.as_ref().map(|m| m.keys), Some(2));
    assert_eq!(insp.participating_keys.len(), 1);
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);

    // The wallet's row on the Inputs step: "MuSig2 · 2 keys".
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    h.tap(ids::SIGN_CONTINUE);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.wallet_musig),
        "{texts:?}"
    );
    assert!(texts.contains(&keys_value()), "{texts:?}");

    h.tap(ids::SIGN_CONTINUE);
    h.hold(ids::SIGN_HOLD);

    // The Result counts the partial signatures on the input: one of two.
    let texts = h.app.texts();
    let expected = strings::fill(strings::EN.words_page, &["1", "2"]);
    assert!(texts.contains(&expected), "{texts:?}");
}

/// The device going first (`docs/PLANNING.md` §16.100): a spend with
/// nobody's nonce on it ends in a shared nonce and an open session, the
/// file it saves carries the nonce, and when the transaction comes back
/// with the other participant's nonce the same session signs it.
#[test]
fn the_device_shares_a_nonce_first_and_signs_when_the_transaction_returns() {
    let mut h = Harness::new(PANEL);
    regtest_wallet_at_the_scanner(&mut h);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(DEVICE_FIRST),
    });
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    let other = other_participant(&h);
    advance_to_confirm(&mut h);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.sign_hold_share)),
        "the hold says what the pass will do: {:?}",
        h.app.texts()
    );

    h.hold(ids::SIGN_HOLD);
    let texts = h.app.texts();
    for expected in [
        strings::EN.sign_result_nonce,
        strings::EN.sign_session_row,
        strings::EN.sign_session_open,
        strings::EN.sign_session_ends_row,
        strings::EN.sign_session_ends,
    ] {
        assert!(texts.iter().any(|t| t == expected), "{expected}: {texts:?}");
    }
    let none_yet = strings::fill(strings::EN.words_page, &["0", "2"]);
    assert!(texts.contains(&none_yet), "no signature yet: {texts:?}");

    // The bytes the coordinator is handed carry this device's nonce.
    let saved = save(&mut h);
    let shared = osk_psbt::Psbt::parse_bytes(&saved).expect("a PSBT");
    assert_eq!(
        osk_psbt::musig::pub_nonces(&shared.inner().inputs[0])
            .unwrap()
            .len(),
        1
    );

    // Home says a session is open while the device holds the secret.
    h.go_home();
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.status_session_open)),
        "{:?}",
        h.app.texts()
    );

    // The other participant answers with its own nonce, and the same
    // session signs the transaction that comes back.
    let returned = with_other_nonce(other, saved);
    read_transaction(&mut h, returned);
    advance_to_confirm(&mut h);
    assert!(
        h.app.texts().contains(&String::from(strings::EN.sign_hold)),
        "{:?}",
        h.app.texts()
    );
    h.hold(ids::SIGN_HOLD);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(
            |t| *t == strings::EN.sign_result_partial || *t == strings::EN.sign_result_complete
        ),
        "{texts:?}"
    );
    let one_of_two = strings::fill(strings::EN.words_page, &["1", "2"]);
    assert!(texts.contains(&one_of_two), "{texts:?}");

    h.go_home();
    assert!(
        !h.app
            .texts()
            .contains(&String::from(strings::EN.status_session_open)),
        "the session ended with the signature: {:?}",
        h.app.texts()
    );
}

/// A lock ends the session. The transaction that comes back carries a
/// nonce this device no longer holds the secret for, so the review says
/// it is replaced and the hold shares a new one.
#[test]
fn a_lock_ends_the_session_and_the_returning_nonce_is_replaced() {
    let mut h = Harness::new(PANEL);
    regtest_wallet_at_the_scanner(&mut h);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(DEVICE_FIRST),
    });
    let other = other_participant(&h);
    advance_to_confirm(&mut h);
    h.hold(ids::SIGN_HOLD);
    let saved = save(&mut h);

    h.go_home();
    assert!(h.app.rect_of(ids::STATUS_LOCK).is_some(), "lock button");
    assert!(h.app.has_pin(), "a PIN to unlock with");
    h.tap(ids::STATUS_LOCK);
    assert_eq!(
        h.app.screen(),
        opensigner_core::ScreenKind::Lock,
        "{:?}",
        h.app.texts()
    );
    h.unlock(common::PIN);
    h.go_home();
    assert!(
        !h.app
            .texts()
            .contains(&String::from(strings::EN.status_session_open)),
        "a lock ends the session: {:?}",
        h.app.texts()
    );

    let returned = with_other_nonce(other, saved);
    read_transaction(&mut h, returned);
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.kind == osk_psbt::WarningKind::MusigNonceReplaced),
        "{:?}",
        insp.warnings
    );
    advance_to_confirm(&mut h);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.sign_hold_share)),
        "{:?}",
        h.app.texts()
    );
    h.hold(ids::SIGN_HOLD);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.sign_result_nonce)),
        "{:?}",
        h.app.texts()
    );
}

/// There is one session. Sharing a nonce for another transaction
/// replaces it, and the badge stands for the new one.
#[test]
fn another_transaction_replaces_the_session() {
    let mut h = Harness::new(PANEL);
    regtest_wallet_at_the_scanner(&mut h);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: psbt_bytes(DEVICE_FIRST),
    });
    let other = other_participant(&h);
    advance_to_confirm(&mut h);
    h.hold(ids::SIGN_HOLD);
    let first = save(&mut h);

    read_transaction(&mut h, another_transaction());
    advance_to_confirm(&mut h);
    h.hold(ids::SIGN_HOLD);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.sign_result_nonce)),
        "{:?}",
        h.app.texts()
    );
    h.go_home();
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.status_session_open)),
        "{:?}",
        h.app.texts()
    );

    // The first transaction's nonce is no longer one this device holds.
    let returned = with_other_nonce(other, first);
    read_transaction(&mut h, returned);
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.kind == osk_psbt::WarningKind::MusigNonceReplaced),
        "{:?}",
        insp.warnings
    );
}

/// Continue through the review until the hold.
fn advance_to_confirm(h: &mut Harness) {
    for _ in 0..8 {
        if h.app.rect_of(ids::SIGN_HOLD).is_some() {
            return;
        }
        h.tap(ids::SIGN_CONTINUE);
    }
    panic!("the hold was never reached: {:?}", h.app.texts());
}

/// "Save to file", and the bytes the shell was handed.
fn save(h: &mut Harness) -> Vec<u8> {
    let (x, y) = h.center(ids::SIGN_SAVE);
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        h.app.event(Event::Touch { x, y, phase });
    }
    let mut written = h.drain().into_iter().filter_map(|c| match c {
        osk_shell_api::Command::WriteFile { bytes, .. } => Some(bytes),
        _ => None,
    });
    let first = written.next().expect("one WriteFile");
    assert!(written.next().is_none(), "exactly one WriteFile");
    first
}

/// Home → Scan → a file with `bytes` in it.
fn read_transaction(h: &mut Harness, bytes: Vec<u8>) {
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes,
    });
}

/// The other participant's round 1, played here: a public nonce for the
/// participant key this device does not hold. Its secret stays with
/// whoever drew it, which is what makes this the other signer.
fn with_other_nonce(keys: (PublicKey, PublicKey), psbt: Vec<u8>) -> Vec<u8> {
    let (other, output_key) = keys;
    let secp = osk_psbt::bitcoin::secp256k1::Secp256k1::new();
    let (_, nonce) =
        osk_bip::musig::nonce_gen(&secp, &[0x55; 32], None, &other, None, None, None).unwrap();
    let mut psbt = osk_psbt::Psbt::parse_bytes(&psbt).expect("a PSBT");
    osk_psbt::musig::write_pub_nonce(&mut psbt.inner_mut().inputs[0], &other, &output_key, &nonce);
    psbt.to_bytes()
}

/// The participant key of the wallet this device does not hold, and the
/// taproot output key its entries are keyed by, read off the review
/// while it is on screen.
fn other_participant(h: &Harness) -> (PublicKey, PublicKey) {
    let insp = h.app.sign_inspection().expect("inspected");
    let musig = insp.inputs[0].musig.clone().expect("a MuSig2 input");
    let other = *musig
        .participants
        .iter()
        .find(|p| musig.ours.iter().all(|m| m.key != **p))
        .expect("the other participant");
    (other, musig.output_key)
}

/// The same spend with a locktime on it, which is another transaction
/// and so another session.
fn another_transaction() -> Vec<u8> {
    let mut psbt = osk_psbt::Psbt::parse_base64(DEVICE_FIRST.trim()).expect("the committed PSBT");
    psbt.inner_mut().unsigned_tx.lock_time =
        osk_psbt::bitcoin::absolute::LockTime::from_height(1).unwrap();
    psbt.to_bytes()
}
