//! Signing a threshold wallet through the app, with one harness playing
//! both locations (`docs/PLANNING.md` §16.103).
//!
//! The group is the committed 2-of-3 regtest record, the transaction is
//! the spend Bitcoin Core funded, and the file that travels between the
//! locations is the carry file the first pass saves.

mod common;

use common::{Harness, PANEL};
use opensigner_core::{ScreenKind, ids, strings};
use osk_shell_api::{App, Event, FileKind, TouchPhase};

const RECORD: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest.record");
const SHARE_0: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-0.txt");
const SHARE_1: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-1.txt");
const SHARE_2: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-2.txt");
const FUNDED: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-first.psbt");
const SIGNED: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-signed.psbt");

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

fn funded_bytes() -> Vec<u8> {
    osk_psbt::Psbt::parse_base64(FUNDED.trim())
        .expect("the committed PSBT")
        .to_bytes()
}

/// A device on regtest with the group record in use and one share
/// loaded, sitting at the file picker of the scanner.
fn location(share: &str) -> Harness {
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: RECORD.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    h.tap(ids::INSPECT_USE_WALLET);
    h.go_home();
    h.start_load_24(&words(share));
    h.finish_load(None);
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h
}

/// Continue through the review until the hold, or until the review ends
/// because something blocks it.
fn advance_to_confirm(h: &mut Harness) -> bool {
    for _ in 0..8 {
        if h.app.rect_of(ids::SIGN_HOLD).is_some() {
            return true;
        }
        if h.app.rect_of(ids::SIGN_CONTINUE).is_none() {
            return false;
        }
        h.tap(ids::SIGN_CONTINUE);
    }
    false
}

/// "Save to file", and the bytes the shell was handed.
fn save(h: &mut Harness) -> Vec<u8> {
    let (x, y) = h.center(ids::SIGN_SAVE);
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        h.app.event(Event::Touch { x, y, phase });
    }
    let mut written = h.drain().into_iter().filter_map(|c| match c {
        osk_shell_api::Command::WriteFile {
            name_hint, bytes, ..
        } => Some((name_hint, bytes)),
        _ => None,
    });
    let (name, bytes) = written.next().expect("one WriteFile");
    assert!(written.next().is_none(), "exactly one WriteFile");
    assert_eq!(name, osk_psbt::threshold::CARRY_NAME);
    bytes
}

/// The whole route with one harness per location: share 0 signs and
/// chooses share 1, the carry file is saved, and a fresh device holding
/// share 1 reads it and finishes the transaction.
#[test]
fn a_threshold_spend_is_carried_from_one_location_to_the_next() {
    // ----- the first location -----
    let mut h = location(SHARE_0);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: funded_bytes(),
    });
    let insp = h.app.sign_inspection().expect("inspected");
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    let summary = insp.threshold.as_ref().expect("a threshold input");
    assert_eq!((summary.t, summary.n), (2, 3));
    assert_eq!(summary.ours, vec![0]);

    // The review's wallet row says what kind of wallet this is.
    assert!(advance_to_confirm(&mut h), "{:?}", h.app.texts());
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.sign_then_with)),
        "the second chip row is offered at the first location: {texts:?}"
    );
    assert!(
        texts.contains(&String::from(strings::EN.sign_hold)),
        "the hold reads the same at both locations: {texts:?}"
    );

    // The hold is dead until exactly one other share is chosen, and dead
    // again when two are.
    h.hold(ids::SIGN_HOLD);
    assert!(h.app.sign_outcome_signatures().is_none(), "nothing signed");
    h.tap(ids::at(ids::SIGN_OTHER_BASE, 1));
    h.tap(ids::at(ids::SIGN_OTHER_BASE, 2));
    h.hold(ids::SIGN_HOLD);
    assert!(h.app.sign_outcome_signatures().is_none(), "two is not one");
    h.tap(ids::at(ids::SIGN_OTHER_BASE, 2));
    h.hold(ids::SIGN_HOLD);

    // Partly signed: one of the two partial signatures, and the file to
    // carry, with no QR offered for it.
    let texts = h.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.sign_result_partial)),
        "{texts:?}"
    );
    assert!(
        texts.contains(&strings::fill(strings::EN.words_page, &["1", "2"])),
        "one of two: {texts:?}"
    );
    assert!(
        h.app.rect_of(ids::SIGN_QR).is_none(),
        "a carry file never goes through a code"
    );
    let carried = save(&mut h);
    assert!(carried.starts_with(b"OSKC"), "the carry file's magic");

    // ----- the second location -----
    let mut g = location(SHARE_1);
    g.send(Event::File {
        kind: FileKind::Any,
        bytes: carried.clone(),
    });
    let insp = g.app.sign_inspection().expect("inspected");
    assert!(!insp.has_blocked(), "{:?}", insp.warnings);
    assert_eq!(
        insp.threshold.as_ref().map(|t| t.ours.clone()),
        Some(vec![1])
    );
    assert!(advance_to_confirm(&mut g), "{:?}", g.app.texts());
    assert!(
        !g.app
            .texts()
            .contains(&String::from(strings::EN.sign_then_with)),
        "a later location chooses nobody: the set is the file's"
    );
    g.hold(ids::SIGN_HOLD);

    let texts = g.app.texts();
    assert!(
        texts.contains(&String::from(strings::EN.sign_result_complete)),
        "{texts:?}"
    );
    assert!(
        texts.contains(&strings::fill(strings::EN.words_page, &["2", "2"])),
        "two of two: {texts:?}"
    );
    // The finished transaction spends what the fixture spends: the
    // witness differs, because this device drew its own nonces.
    let mut fixture = osk_psbt::Psbt::parse_base64(SIGNED.trim()).expect("the committed PSBT");
    let expected = osk_psbt::finalize(&mut fixture)
        .expect("finalize")
        .expect("a transaction")
        .compute_txid();
    assert_eq!(g.app.sign_outcome_txid(), Some(expected));
}

/// A tampered file and a file for another share both reach the review as
/// blocked cards that name the reason.
#[test]
fn a_carry_file_that_does_not_check_out_is_blocked_by_name() {
    let mut h = location(SHARE_0);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: funded_bytes(),
    });
    advance_to_confirm(&mut h);
    h.tap(ids::at(ids::SIGN_OTHER_BASE, 1));
    h.hold(ids::SIGN_HOLD);
    let carried = save(&mut h);

    // One byte of the stored secret nonce: magic (4), version (1),
    // signer count (1), two shares (66), entry count (1), input (4),
    // share (33), sighash (32).
    let mut tampered = carried.clone();
    tampered[4 + 1 + 1 + 66 + 1 + 4 + 33 + 32] ^= 0x01;
    let mut g = location(SHARE_1);
    g.send(Event::File {
        kind: FileKind::Any,
        bytes: tampered,
    });
    let insp = g.app.sign_inspection().expect("inspected");
    assert!(insp.has_blocked(), "{:?}", insp.warnings);
    assert!(
        g.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.warn_threshold_nonce)
            || {
                advance_to_confirm(&mut g);
                g.app
                    .texts()
                    .iter()
                    .any(|t| t == strings::EN.warn_threshold_nonce)
            },
        "the reason is on a card: {:?}",
        g.app.texts()
    );

    // The same file at a device holding share 2, which is not a signer.
    let mut k = location(SHARE_2);
    k.send(Event::File {
        kind: FileKind::Any,
        bytes: carried,
    });
    let insp = k.app.sign_inspection().expect("inspected");
    assert!(insp.has_blocked(), "{:?}", insp.warnings);
    advance_to_confirm(&mut k);
    assert!(
        k.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.warn_threshold_not_a_signer),
        "{:?}",
        k.app.texts()
    );
}

/// The wallet's own page offers Sign once this device holds a key of
/// the group, which is matched by public share (§16.104 rule 3).
#[test]
fn a_threshold_wallet_this_device_holds_a_share_of_can_sign() {
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: RECORD.as_bytes().to_vec(),
    });
    h.tap(ids::INSPECT_USE_WALLET);
    h.go_home();
    h.open_policy(0);
    assert!(
        h.app.rect_of(ids::WALLET_SIGN).is_none(),
        "the record alone signs nothing"
    );
    h.go_home();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.go_home();
    h.open_policy(0);
    assert!(
        h.app.rect_of(ids::WALLET_SIGN).is_some(),
        "the loaded key is what makes the wallet signable: {:?}",
        h.app.texts()
    );
}
