//! Faraday as a check on another signer (`docs/FLOWS.md`, Nonce check):
//! a PSBT signed elsewhere, or a finished transaction, brought in by
//! Files, the camera or Tools opens its nonce check rather than signing.
//! Every signature is checked against what the transaction holds, a
//! damaged one and two under one nonce are refused, a signature under a
//! key loaded here is recomputed and named by the nonce rule that made
//! it, and a final raw transaction says which keys it could not match. A
//! PSBT a key here is still to sign goes to signing as before.
//!
//! The signer elsewhere is rust-bitcoin's secp256k1 signing over
//! rust-bitcoin's sighash, not Faraday's signing code.

use crate::common::add_key;
use faraday_bip::bitcoin::secp256k1::{
    Message, PublicKey as Point, Scalar, Secp256k1, SecretKey, ecdsa::Signature,
};
use faraday_bip::bitcoin::sighash::{EcdsaSighashType, SighashCache};
use faraday_bip::bitcoin::{OutPoint, PublicKey, ecdsa, hashes::Hash};
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::Unmatched;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent};
use faraday_psbt::verify::{NonceMode, Verdict};
use faraday_shell_api::{App, Event};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_inbox(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

/// How the signer elsewhere picks its nonce.
#[derive(Clone, Copy)]
enum How {
    /// RFC 6979, ground to a low R (Bitcoin Core, Sparrow).
    LowR,
    /// RFC 6979's first nonce (Trezor, the RFC's own vectors).
    First,
    /// One nonce chosen for every input: the key given away.
    SameNonce,
}

/// ECDSA with the nonce `k` chosen here rather than by RFC 6979, so that
/// one nonce can sign two inputs. `s = k⁻¹(z + r·d)`, the inverse by
/// Fermat over the curve order.
#[expect(
    clippy::disallowed_methods,
    reason = "the test's own signer elsewhere, apart from the app's (docs/COMPUTE.md §7)"
)]
fn ecdsa_with_k(d: &SecretKey, z: [u8; 32], k: &SecretKey) -> Signature {
    const N_MINUS_2: [u8; 32] = [
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xfe, 0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36,
        0x41, 0x3f,
    ];
    let secp = Secp256k1::new();
    let point = Point::from_secret_key(&secp, k);
    let x: [u8; 32] = point.serialize()[1..33].try_into().unwrap();
    let r = SecretKey::from_slice(&x).unwrap();
    let z = SecretKey::from_slice(&z).unwrap();
    let zrd = d
        .mul_tweak(&Scalar::from(r))
        .unwrap()
        .add_tweak(&Scalar::from(z))
        .unwrap();
    let mut inverse: Option<SecretKey> = None;
    for byte in N_MINUS_2 {
        for bit in (0..8).rev() {
            if let Some(acc) = inverse {
                inverse = Some(acc.mul_tweak(&Scalar::from(acc)).unwrap());
            }
            if byte >> bit & 1 == 1 {
                inverse = Some(match inverse {
                    Some(acc) => acc.mul_tweak(&Scalar::from(*k)).unwrap(),
                    None => *k,
                });
            }
        }
    }
    let s = zrd.mul_tweak(&Scalar::from(inverse.unwrap())).unwrap();
    let mut compact = [0u8; 64];
    compact[..32].copy_from_slice(&r.secret_bytes());
    compact[32..].copy_from_slice(&s.secret_bytes());
    let mut sig = Signature::from_compact(&compact).unwrap();
    sig.normalize_s();
    sig
}

/// Savings' spend (2 of 3) signed elsewhere by test key 2 on every input,
/// as a SeedSigner or a hardware wallet returns it: a partial signature
/// on each, nothing finished. One nonce on every input needs two inputs
/// under one key: a second, spending the first one's output script, is
/// added where the spend has one.
fn signed_elsewhere(how: How) -> faraday_psbt::Psbt {
    let c = testkit::compute();
    let (mut psbt, _) = testkit::spend_of(&kit("savings")).unwrap();
    if matches!(how, How::SameNonce) && psbt.unsigned_tx().input.len() < 2 {
        let inner = psbt.inner_mut();
        let mut txin = inner.unsigned_tx.input[0].clone();
        txin.previous_output =
            OutPoint::new(txin.previous_output.txid, txin.previous_output.vout + 1);
        inner.unsigned_tx.input.push(txin);
        let mut second = inner.inputs[0].clone();
        second.non_witness_utxo = None;
        inner.inputs.push(second);
    }
    let words = testkit::test_words("zebra");
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
        .unwrap();
    let zebra =
        faraday_bip::keys::MasterKey::from_seed(&c, &m.to_seed(&c, b"").unwrap(), testkit::net());
    let tx = psbt.unsigned_tx().clone();
    let secp = Secp256k1::new();
    let k = SecretKey::from_slice(&[7u8; 32]).unwrap();
    for i in 0..tx.input.len() {
        let input = psbt.inner().inputs[i].clone();
        let (pk, (_, path)) = input
            .bip32_derivation
            .iter()
            .find(|(_, (fp, _))| fp.to_bytes() == zebra.fingerprint().0)
            .expect("test key 2 is a key of Savings");
        let derived = zebra.derive(&c, path);
        let ws = input.witness_script.clone().expect("a P2WSH input");
        let value = input.witness_utxo.as_ref().expect("its output").value;
        let hash = SighashCache::new(&tx)
            .p2wsh_signature_hash(i, &ws, value, EcdsaSighashType::All)
            .unwrap();
        let msg = Message::from_digest(hash.to_byte_array());
        let signature = match how {
            How::LowR => secp.sign_ecdsa_low_r(&msg, derived.secret_key()),
            How::First => secp.sign_ecdsa(&msg, derived.secret_key()),
            How::SameNonce => ecdsa_with_k(derived.secret_key(), hash.to_byte_array(), &k),
        };
        psbt.inner_mut().inputs[i].partial_sigs.insert(
            PublicKey::new(*pk),
            ecdsa::Signature {
                signature,
                sighash_type: EcdsaSighashType::All,
            },
        );
    }
    psbt
}

/// The same PSBT with input 0's signature changed by one bit of `s`.
fn damaged(mut psbt: faraday_psbt::Psbt) -> faraday_psbt::Psbt {
    let sig = psbt.inner_mut().inputs[0]
        .partial_sigs
        .values_mut()
        .next()
        .unwrap();
    let mut compact = sig.signature.serialize_compact();
    compact[63] ^= 1;
    sig.signature = Signature::from_compact(&compact).unwrap();
    psbt
}

/// The nonce check of Inbox file 0, opened from Files.
fn checked(app: &mut Faraday) -> &faraday_core::decode::OutsideCheck {
    app.press(Action::Nav(Screen::Files));
    assert!(
        app.checked_not_signed(0),
        "Files offers Sign, not Nonce check"
    );
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        0,
    )));
    assert_eq!(app.screen, Screen::NonceCheck);
    app.outside_check().expect("the check")
}

#[test]
fn a_psbt_signed_elsewhere_is_checked_valid_and_its_nonces_need_that_device() {
    let mut app = with_inbox(vec![(
        "seedsigner.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert!(!o.check.rows.is_empty(), "a signature per input");
    assert!(o.check.rows.iter().all(|r| r.verdict == Verdict::Valid));
    assert!(
        o.check
            .rows
            .iter()
            .all(|r| r.recomputed.is_none() && r.unmatched.is_none()),
        "no key here: signed elsewhere"
    );
    assert!(o.check.sound());
    assert!(o.finished.is_none(), "one of two signatures is no spend");
    assert_eq!(o.back, Screen::Files);
    // Carried on to the next signer as it came.
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(app.outbox.iter().any(|i| i.name == "seedsigner.psbt"));
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
}

#[test]
fn with_the_other_signers_seed_loaded_here_each_signature_is_recomputed() {
    for how in [How::LowR, How::First] {
        let psbt = signed_elsewhere(how);
        // RFC 6979's first nonce that already gives a low R is the low-R
        // rule's too, and is named as that.
        let high_r = psbt.inner().inputs[0]
            .partial_sigs
            .values()
            .next()
            .is_some_and(|s| s.signature.serialize_compact()[0] >= 0x80);
        let mode = match how {
            How::First if high_r => NonceMode::First,
            _ => NonceMode::LowR,
        };
        let mut app = with_inbox(vec![("signed.psbt", psbt.to_bytes())]);
        add_key(&mut app, "zebra");
        let o = checked(&mut app);
        assert!(
            o.check
                .rows
                .iter()
                .all(|r| r.recomputed == Some(Some(mode))),
            "{:?}",
            o.check
                .rows
                .iter()
                .map(|r| r.recomputed)
                .collect::<Vec<_>>()
        );
        assert!(o.check.sound());
    }
}

#[test]
fn a_psbt_a_key_here_is_still_to_sign_goes_to_signing() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    add_key(&mut app, "bacon");
    assert!(!app.checked_not_signed(0), "test key 1 has not signed");
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Nonce check")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.sheet, Some(Sheet::Scan), "nothing to check: the camera");
}

#[test]
fn a_damaged_signature_refuses_the_transaction() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        damaged(signed_elsewhere(How::LowR)).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert!(o.check.wrong());
    assert_eq!(o.check.rows[0].verdict, Verdict::Invalid);
    assert!(o.finished.is_none());
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(
        app.outbox.is_empty(),
        "a refused transaction is not carried on"
    );
}

#[test]
fn two_signatures_under_one_nonce_are_refused() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::SameNonce).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert!(!o.check.reused.is_empty(), "the shared nonce is named");
    assert!(o.check.wrong());
    assert!(!o.check.sound());
}

/// Savings signed by test keys 1 (here) and 2 (elsewhere), finished:
/// the raw transaction, and the unsigned PSBT that states its amounts.
fn finished_savings() -> (String, Vec<u8>) {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut b = with_inbox(vec![
        ("spend.psbt", unsigned.to_bytes()),
        ("savings-wallet.txt", k.descriptor.into_bytes()),
        ("from-a.psbt", signed_elsewhere(How::LowR).to_bytes()),
    ]);
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    b.press(Action::LoadWallet(at(&b, "savings-wallet.txt")));
    add_key(&mut b, "bacon");
    b.press(Action::StartSpend(at(&b, "spend.psbt")));
    b.press(Action::SignHere);
    b.press(Action::Collect(at(&b, "from-a.psbt")));
    let hex = b.spend().unwrap().spend.finished_hex().expect("finished");
    (hex, unsigned.to_bytes())
}

#[test]
fn a_final_raw_transaction_says_which_keys_it_could_not_match() {
    let (hex, unsigned) = finished_savings();
    // Alone: no output it spends is stated, so nothing can be checked,
    // and the headline says so.
    let mut alone = with_inbox(vec![("broadcast-me.txn", hex.clone().into_bytes())]);
    let o = checked(&mut alone);
    assert!(o.check.unread > 0);
    assert!(
        o.check.headline().contains("not checked"),
        "{}",
        o.check.headline()
    );
    assert!(!o.check.sound() && !o.check.wrong());

    // Beside the PSBT that states the amounts: every signature valid,
    // and its key not matched, a raw transaction naming no origin.
    let mut beside = with_inbox(vec![
        ("broadcast-me.txn", hex.into_bytes()),
        ("spend.psbt", unsigned),
    ]);
    add_key(&mut beside, "bacon");
    let o = checked(&mut beside);
    assert!(o.check.rows.iter().all(|r| r.verdict == Verdict::Valid));
    assert!(
        o.check
            .rows
            .iter()
            .all(|r| r.unmatched == Some(Unmatched::NoOrigin))
    );
    assert!(o.finished.is_some());
    beside.press(Action::NonceCheck(faraday_core::decode::CheckPress::Decode));
    assert_eq!(beside.screen, Screen::Decode);
    assert!(beside.decode().unwrap().decoded.fee.is_some());
    beside.press(Action::NonceCheck(faraday_core::decode::CheckPress::Back));
    assert_eq!(beside.screen, Screen::NonceCheck);
    beside.press(Action::NonceCheck(faraday_core::decode::CheckPress::Out));
    assert!(beside.outbox.iter().any(|i| i.name == "broadcast-me.txn"));
}

#[test]
fn a_signed_psbt_scanned_opens_its_check_and_goes_back_where_the_scan_started() {
    let psbt = signed_elsewhere(How::LowR);
    let mut app = with_inbox(Vec::new());
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    app.event(Event::Scanned {
        bytes: psbt.to_base64().into_bytes(),
    });
    assert_eq!(app.screen, Screen::NonceCheck);
    assert_eq!(app.sheet, None);
    assert_eq!(app.outside_check().unwrap().back, Screen::Home);
    let _ = app.frame();
}

#[test]
fn tools_nonce_check_opens_a_signed_file_here() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Nonce check")
        .unwrap();
    assert!(app.tile_opens(faraday_core::catalog::TILES[tile].go));
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::NonceCheck);
    assert_eq!(app.outside_check().unwrap().back, Screen::Catalog);
}

#[test]
fn a_key_loaded_since_the_check_opened_is_counted_on_the_way_back() {
    let mut app = with_inbox(vec![(
        "signed.psbt",
        signed_elsewhere(How::LowR).to_bytes(),
    )]);
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        0,
    )));
    assert!(
        app.outside_check()
            .unwrap()
            .check
            .rows
            .iter()
            .all(|r| r.recomputed.is_none())
    );
    add_key(&mut app, "zebra");
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Back));
    assert_eq!(app.screen, Screen::NonceCheck);
    assert!(
        app.outside_check()
            .unwrap()
            .check
            .rows
            .iter()
            .all(|r| r.recomputed == Some(Some(NonceMode::LowR))),
        "the check follows the keys"
    );
}
