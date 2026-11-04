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
use faraday_core::strings::EN;
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

/// The nonce check of Inbox file 0, opened from Files, though nothing in
/// the file is a signature that stands (Files then offers Sign).
fn opened(app: &mut Faraday) -> &faraday_core::decode::OutsideCheck {
    app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        0,
    )));
    assert_eq!(app.screen, Screen::NonceCheck);
    app.outside_check().expect("the check")
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

/// One input of a finalized taproot transaction.
enum Tap {
    /// A key-path spend, signed over the whole transaction.
    Key,
    /// A script-path spend of a hash-lock leaf: `[preimage, leaf,
    /// control]`, the control block with a byte flipped when `spoil`.
    Lock { preimage: Vec<u8>, spoil: bool },
    /// The real key-path signature hidden as `[sig, a made-up leaf,
    /// control]` under a key-path output: a signature that is right, in
    /// a witness that confirms nowhere.
    Hidden,
}

/// A PSBT whose inputs are finalized as `inputs` says, each spending a
/// real taproot output.
fn taproot_psbt(inputs: &[Tap]) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::key::TapTweak;
    use faraday_bip::bitcoin::secp256k1::Keypair;
    use faraday_bip::bitcoin::sighash::{Prevouts, TapSighashType};
    use faraday_bip::bitcoin::taproot::{LeafVersion, TaprootBuilder};
    use faraday_bip::bitcoin::{
        Amount, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness, absolute::LockTime,
        transaction::Version,
    };
    let secp = Secp256k1::new();
    let pair = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[7; 32]).unwrap());
    let (internal, _) = pair.x_only_public_key();
    let signer = pair.tap_tweak(&secp, None).to_keypair();
    let other = Keypair::from_secret_key(&secp, &SecretKey::from_slice(&[9; 32]).unwrap());
    let (other_key, _) = other.x_only_public_key();
    let mut prevouts = Vec::new();
    let mut leaves = Vec::new();
    for tap in inputs {
        let (spk, leaf) = match tap {
            Tap::Key | Tap::Hidden => (ScriptBuf::new_p2tr(&secp, internal, None), None),
            Tap::Lock { preimage, spoil } => {
                let hash = faraday_bip::bitcoin::hashes::sha256::Hash::hash(preimage);
                let script = faraday_bip::bitcoin::script::Builder::new()
                    .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_SHA256)
                    .push_slice(hash.to_byte_array())
                    .push_opcode(faraday_bip::bitcoin::opcodes::all::OP_EQUAL)
                    .into_script();
                let info = TaprootBuilder::new()
                    .add_leaf(0, script.clone())
                    .unwrap()
                    .finalize(&secp, other_key)
                    .unwrap();
                let mut control = info
                    .control_block(&(script.clone(), LeafVersion::TapScript))
                    .unwrap()
                    .serialize();
                if *spoil {
                    control[2] ^= 1;
                }
                (
                    ScriptBuf::new_p2tr(&secp, other_key, info.merkle_root()),
                    Some(Witness::from_slice(&[
                        preimage.clone(),
                        script.to_bytes(),
                        control,
                    ])),
                )
            }
        };
        prevouts.push(TxOut {
            value: Amount::from_sat(50_000),
            script_pubkey: spk,
        });
        leaves.push(leaf);
    }
    let hidden: Vec<bool> = inputs.iter().map(|t| matches!(t, Tap::Hidden)).collect();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: (0..inputs.len())
            .map(|i| TxIn {
                previous_output: OutPoint::new(
                    faraday_bip::bitcoin::Txid::from_byte_array([1; 32]),
                    i as u32,
                ),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: prevouts[0].script_pubkey.clone(),
        }],
    };
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx.clone()).unwrap();
    for (i, leaf) in leaves.into_iter().enumerate() {
        inner.inputs[i].witness_utxo = Some(prevouts[i].clone());
        inner.inputs[i].final_script_witness = Some(leaf.unwrap_or_else(|| {
            let hash = SighashCache::new(&tx)
                .taproot_key_spend_signature_hash(
                    i,
                    &Prevouts::All(&prevouts),
                    TapSighashType::Default,
                )
                .unwrap();
            let sig = Secp256k1::new()
                .sign_schnorr_no_aux_rand(&Message::from_digest(hash.to_byte_array()), &signer);
            if hidden[i] {
                let mut control = vec![0xc0];
                control.extend_from_slice(&internal.serialize());
                Witness::from_slice(&[sig.serialize().to_vec(), vec![0x51], control])
            } else {
                Witness::from_slice(&[sig.serialize().to_vec()])
            }
        }));
    }
    faraday_psbt::Psbt::from(inner)
}

fn lock(spoil: bool) -> Tap {
    Tap::Lock {
        preimage: vec![0xcd; 32],
        spoil,
    }
}

/// Inbox file 0 is offered as checked rather than signed.
fn offered(psbt: &faraday_psbt::Psbt) -> bool {
    with_inbox(vec![("t.psbt", psbt.to_bytes())]).checked_not_signed(0)
}

#[test]
fn a_failing_witness_with_no_signature_row_is_not_sound() {
    // Input 0 a valid key-path spend, input 1 a hash-lock leaf whose
    // control block does not commit: no row for it, and still wrong.
    let psbt = taproot_psbt(&[Tap::Key, lock(true)]);
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert!(
        o.check
            .rows
            .iter()
            .all(|r| r.input == 0 && r.verdict == Verdict::Valid)
    );
    assert_eq!(o.check.mismatched, vec![1]);
    assert!(o.check.wrong());
    assert!(!o.check.sound());
    assert!(o.finished.is_none(), "no finished transaction is offered");
    let good = taproot_psbt(&[Tap::Key, lock(false)]);
    let mut app = with_inbox(vec![("t.psbt", good.to_bytes())]);
    let o = checked(&mut app);
    assert!(o.check.mismatched.is_empty());
    assert!(o.check.sound());
}

#[test]
fn a_single_failing_witness_is_not_checked_as_signed() {
    let failing = taproot_psbt(&[lock(true)]);
    let committing = taproot_psbt(&[lock(false)]);
    assert!(!offered(&failing));
    assert!(offered(&committing));
    assert!(!faraday_core::wallet::PsbtSeen::of(&failing).signed);
    assert!(faraday_core::wallet::PsbtSeen::of(&committing).signed);
    let mixed = taproot_psbt(&[Tap::Key, lock(true)]);
    assert!(faraday_core::wallet::PsbtSeen::of(&mixed).signed);
}

#[test]
fn a_cosigners_failing_witness_is_not_merged_and_a_true_one_is() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let unfinalized = |p: &faraday_psbt::Psbt| {
        let mut own = p.clone();
        own.inner_mut().inputs[0].final_script_witness = None;
        Spend::new(own, "own.psbt")
    };
    for (what, tap) in [
        ("no row", lock(true)),
        (
            "a proven preimage only",
            Tap::Lock {
                preimage: vec![0xcd; 64],
                spoil: true,
            },
        ),
    ] {
        let theirs = taproot_psbt(&[tap]);
        let mut spend = unfinalized(&theirs);
        let before = spend.psbt.to_bytes();
        let why = spend
            .collect(&c, &session, &theirs.to_bytes(), "cosigner.psbt")
            .expect_err(what);
        assert_eq!(why, mismatch_text(0), "{what}");
        assert!(!why.contains("cosigner.psbt"), "{what}");
        assert_eq!(spend.psbt.to_bytes(), before, "{what}");
        // The spend keeps its own state: nothing to finish for lack of a signature.
        assert_eq!(
            spend.finish().unwrap_err(),
            EN.wallet_not_enough_signatures,
            "{what}"
        );
        assert!(spend.finished.is_none(), "{what}");
    }
    for tap in [Tap::Key, lock(false)] {
        let theirs = taproot_psbt(&[tap]);
        let mut spend = unfinalized(&theirs);
        spend
            .collect(&c, &session, &theirs.to_bytes(), "cosigner.psbt")
            .expect("a true spend merges");
        assert!(spend.psbt.inner().inputs[0].final_script_witness.is_some());
        assert!(spend.finish().is_ok());
    }
    // A spend opened on a failing witness finishes nothing either.
    let mut spend = Spend::new(taproot_psbt(&[lock(true)]), "own.psbt");
    assert_eq!(spend.finish().unwrap_err(), mismatch_text(0));
    assert!(spend.finished.is_none());
}

fn mismatch_text(input: usize) -> String {
    EN.psbt_warn_witness_mismatch
        .replace("{}", &input.to_string())
}

/// One input spending `spk` by `script_sig` and `witness`.
fn second_input(
    spk: faraday_bip::bitcoin::ScriptBuf,
    script_sig: faraday_bip::bitcoin::ScriptBuf,
    witness: Vec<Vec<u8>>,
) -> faraday_psbt::Psbt {
    let mut psbt = taproot_psbt(&[Tap::Key]);
    let input = &mut psbt.inner_mut().inputs[0];
    input.witness_utxo.as_mut().unwrap().script_pubkey = spk;
    input.final_script_sig = Some(script_sig);
    input.final_script_witness = Some(faraday_bip::bitcoin::Witness::from_slice(&witness));
    psbt
}

#[test]
fn a_previous_transaction_of_another_txid_takes_nothing_out_of_the_check() {
    use faraday_bip::bitcoin::{Amount, Sequence, Transaction, TxIn, TxOut, Witness};
    use faraday_core::wallet::Session;
    use faraday_core::wallet::Spend;
    let mut psbt = taproot_psbt(&[Tap::Key, lock(true)]);
    let out = psbt.inner().inputs[1].witness_utxo.clone().unwrap();
    // Another transaction than the one input 1 spends, with that output
    // at that index.
    let other = Transaction {
        version: faraday_bip::bitcoin::transaction::Version::TWO,
        lock_time: faraday_bip::bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: Default::default(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![
            TxOut {
                value: Amount::from_sat(1),
                script_pubkey: Default::default(),
            },
            out,
        ],
    };
    psbt.inner_mut().inputs[1].witness_utxo = None;
    psbt.inner_mut().inputs[1].non_witness_utxo = Some(other);
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.unread, 1);
    assert!(!o.check.sound());
    assert!(o.finished.is_none(), "no finished transaction is offered");
    let mut spend = Spend::new(psbt.clone(), "t.psbt");
    assert_eq!(
        spend.finish().unwrap_err(),
        EN.psbt_final_prev_unread.replace("{}", "1")
    );
    assert!(spend.finished.is_none());
    // A cosigner's copy carrying it is refused where the own copy states
    // no previous output of that input.
    let mut own = psbt.clone();
    own.inner_mut().inputs[1].non_witness_utxo = None;
    own.inner_mut().inputs[1].final_script_witness = None;
    let mut spend = Spend::new(own, "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &psbt.to_bytes(),
            "c.psbt",
        )
        .unwrap_err();
    assert_eq!(why, EN.psbt_final_prev_unread.replace("{}", "1"));
    assert_eq!(spend.psbt.to_bytes(), before);
}

#[test]
fn a_witness_of_one_element_under_an_output_that_cannot_confirm_is_refused() {
    use faraday_bip::bitcoin::{ScriptBuf, opcodes::all::OP_PUSHNUM_1, script::Builder};
    use faraday_core::wallet::Spend;
    let sig = vec![1u8; 64];
    // An output key off the curve, spent with [sig] and with [sig, annex].
    let off_curve = Builder::new()
        .push_opcode(OP_PUSHNUM_1)
        .push_slice([0xff; 32])
        .into_script();
    // A p2sh of all zeros, its scriptSig pushing a p2tr script.
    let p2tr = ScriptBuf::new_p2tr(
        &Secp256k1::new(),
        faraday_bip::bitcoin::key::Keypair::from_secret_key(
            &Secp256k1::new(),
            &SecretKey::from_slice(&[7; 32]).unwrap(),
        )
        .x_only_public_key()
        .0,
        None,
    );
    let wrapped = Builder::new()
        .push_slice(<&faraday_bip::bitcoin::script::PushBytes>::try_from(p2tr.as_bytes()).unwrap())
        .into_script();
    let zero_hash =
        ScriptBuf::new_p2sh(&faraday_bip::bitcoin::ScriptHash::from_byte_array([0; 20]));
    let psbts = [
        (
            "off the curve",
            second_input(off_curve.clone(), ScriptBuf::new(), vec![sig.clone()]),
        ),
        (
            "off the curve, annex",
            second_input(
                off_curve,
                ScriptBuf::new(),
                vec![sig.clone(), vec![0x50, 1]],
            ),
        ),
        ("p2sh of zeros", second_input(zero_hash, wrapped, vec![sig])),
    ];
    for (what, psbt) in psbts {
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        app.press(Action::Nav(Screen::Files));
        app.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
            0,
        )));
        assert_eq!(app.screen, Screen::NonceCheck, "{what}");
        let o = app.outside_check().expect("the check");
        assert_eq!(o.check.mismatched, vec![0], "{what}");
        assert!(!o.check.sound(), "{what}");
        assert!(o.finished.is_none(), "{what}");
        assert_eq!(o.check.headline(), EN.nonce_check_witness_wrong, "{what}");
        let mut spend = Spend::new(psbt, "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), mismatch_text(0), "{what}");
    }
}

#[test]
fn a_cosigners_clean_file_is_not_blamed_for_a_failing_witness_already_in_the_spend() {
    use faraday_core::wallet::{Session, Spend};
    let failing = taproot_psbt(&[Tap::Key, lock(true)]);
    let mut clean = taproot_psbt(&[Tap::Key, lock(false)]);
    clean.inner_mut().inputs[1].final_script_witness = None;
    let mut spend = Spend::new(failing, "own.psbt");
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &clean.to_bytes(),
            "clean.psbt",
        )
        .unwrap_err();
    assert_eq!(why, mismatch_text(1));
    assert!(!why.contains("clean.psbt"));
}

#[test]
fn a_failing_witness_alone_is_not_called_a_wrong_signature() {
    let mut app = with_inbox(vec![(
        "t.psbt",
        taproot_psbt(&[Tap::Key, lock(true)]).to_bytes(),
    )]);
    let o = checked(&mut app);
    assert_eq!(o.check.headline(), EN.nonce_check_witness_wrong);
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(EN.nonce_screen_refused_witness), "{shown}");
    assert!(!shown.contains(EN.nonce_screen_refused), "{shown}");
    assert!(shown.contains(&mismatch_text(1)), "{shown}");
    // A wrong signature as well: the signature's words.
    let mut bad = taproot_psbt(&[Tap::Key, lock(true)]);
    let w = bad.inner().inputs[0].final_script_witness.clone().unwrap();
    let mut sig = w.nth(0).unwrap().to_vec();
    sig[63] ^= 1;
    bad.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[sig]));
    let mut app = with_inbox(vec![("t.psbt", bad.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.headline(), EN.nonce_check_nonce_check);
    let shown = app.drawn_texts().join(" ");
    assert!(shown.contains(EN.nonce_screen_refused), "{shown}");
}

#[test]
fn the_reason_a_spend_is_not_finished_is_the_refusal_and_holds_it() {
    use faraday_core::wallet::Spend;
    let spend = Spend::new(taproot_psbt(&[lock(true)]), "t.psbt");
    // The finalize of a copy alone accepts this witness.
    assert_eq!(spend.finish_reason(None), None);
    let refused = spend.refused();
    assert_eq!(refused.as_deref(), Some(mismatch_text(0).as_str()));
    assert_eq!(spend.finish_reason(refused.as_deref()), refused);
    let true_one = Spend::new(taproot_psbt(&[lock(false)]), "t.psbt");
    assert_eq!(true_one.refused(), None);
    assert_eq!(true_one.finish_reason(None), None);
}

/// The previous transaction of another txid than input `vout` spends, with
/// `out` at index `vout`.
fn other_previous(
    out: faraday_bip::bitcoin::TxOut,
    vout: usize,
) -> faraday_bip::bitcoin::Transaction {
    use faraday_bip::bitcoin::{Amount, Sequence, Transaction, TxIn, TxOut, Witness};
    let mut output = vec![
        TxOut {
            value: Amount::from_sat(1),
            script_pubkey: Default::default(),
        };
        vout
    ];
    output.push(out);
    Transaction {
        version: faraday_bip::bitcoin::transaction::Version::TWO,
        lock_time: faraday_bip::bitcoin::absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: Default::default(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output,
    }
}

/// A PSBT whose input 1 (a hash-lock spend) has its previous output
/// unreadable in one of three ways, input 0 a key-path spend (or, for the
/// right txid without the output, another hash-lock spend: the outpoint
/// it changes would spoil a signature).
fn unread_final(how: &str) -> faraday_psbt::Psbt {
    use faraday_bip::bitcoin::{Amount, Transaction, TxOut, transaction::Version};
    if how == "no output at the index" {
        // The txid is the one the input names, but that transaction has
        // no output at the index.
        let mut psbt = taproot_psbt(&[lock(false), lock(false)]);
        let previous = Transaction {
            version: Version::TWO,
            lock_time: faraday_bip::bitcoin::absolute::LockTime::ZERO,
            input: Vec::new(),
            output: vec![TxOut {
                value: Amount::from_sat(1),
                script_pubkey: Default::default(),
            }],
        };
        psbt.inner_mut().unsigned_tx.input[1].previous_output =
            OutPoint::new(previous.compute_txid(), 1);
        let i = &mut psbt.inner_mut().inputs[1];
        i.witness_utxo = None;
        i.non_witness_utxo = Some(previous);
        return psbt;
    }
    let mut psbt = taproot_psbt(&[Tap::Key, lock(false)]);
    let out = psbt.inner().inputs[1].witness_utxo.clone().unwrap();
    match how {
        "another txid" => {
            let i = &mut psbt.inner_mut().inputs[1];
            i.witness_utxo = None;
            i.non_witness_utxo = Some(other_previous(out, 1));
        }
        "none" => psbt.inner_mut().inputs[1].witness_utxo = None,
        _ => unreachable!(),
    }
    psbt
}

#[test]
fn a_final_input_whose_output_cannot_be_read_offers_nothing_to_carry_on() {
    use faraday_core::decode::CheckPress;
    for how in ["another txid", "none", "no output at the index"] {
        let psbt = unread_final(how);
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = checked(&mut app);
        assert_eq!(o.check.unread, 1, "{how}");
        assert!(!o.check.sound() && o.finished.is_none(), "{how}");
        assert!(o.check.refused(o.finished.is_some()), "{how}");
        let shown = app.drawn_texts().join(" | ");
        assert!(
            shown.contains(EN.nonce_screen_refused_unread),
            "{how}: {shown}"
        );
        for offer in [
            EN.nonce_screen_more_wanted,
            EN.nonce_screen_refused,
            EN.vault_save_to_a_stick,
            EN.step_show_as_qr,
        ] {
            assert!(!shown.contains(offer), "{how}: {offer}");
        }
        app.press(Action::NonceCheck(CheckPress::Out));
        assert!(app.outbox.is_empty(), "{how}");
        app.press(Action::NonceCheck(CheckPress::Qr));
        assert_eq!(app.screen, Screen::NonceCheck, "{how}");
        assert_eq!(app.sheet, None, "{how}: no QR sheet opens");
    }
    // A raw transaction with an input it cannot check keeps its offers.
    let (hex, _) = finished_savings();
    let mut raw = with_inbox(vec![("broadcast-me.txn", hex.into_bytes())]);
    let o = checked(&mut raw);
    assert!(o.check.unread > 0 && o.finished.is_some());
    assert!(!o.check.refused(o.finished.is_some()));
    let shown = raw.drawn_texts().join(" | ");
    assert!(shown.contains(EN.vault_save_to_a_stick), "{shown}");
    assert!(shown.contains(EN.step_show_as_qr), "{shown}");
    // A PSBT with no final input stays offered, and both offers work.
    let mut app = with_inbox(vec![("t.psbt", signed_elsewhere(How::LowR).to_bytes())]);
    let o = checked(&mut app);
    assert!(o.finished.is_none());
    assert!(!o.check.refused(o.finished.is_some()));
    let shown = app.drawn_texts().join(" | ");
    assert!(shown.contains(EN.vault_save_to_a_stick), "{shown}");
    assert!(shown.contains(EN.step_show_as_qr), "{shown}");
    app.press(Action::NonceCheck(CheckPress::Qr));
    assert_eq!(app.sheet, Some(Sheet::Qr), "the QR sheet opens");
}

#[test]
fn a_real_signature_in_a_failing_witness_is_a_witness_that_is_wrong() {
    for inputs in [vec![Tap::Hidden], vec![Tap::Key, Tap::Hidden]] {
        let at = inputs.len() - 1;
        let mut app = with_inbox(vec![("t.psbt", taproot_psbt(&inputs).to_bytes())]);
        let o = opened(&mut app);
        assert!(
            o.check
                .rows
                .iter()
                .any(|r| r.input == at && r.verdict == Verdict::Invalid)
        );
        assert_eq!(o.check.mismatched, vec![at]);
        assert!(o.check.only_witness_wrong());
        assert_eq!(o.check.headline(), EN.nonce_check_witness_wrong);
        let shown = app.drawn_texts().join(" ");
        assert!(shown.contains(EN.nonce_screen_refused_witness), "{shown}");
    }
    // A truly wrong signature on another input still reads as one.
    let mut bad = taproot_psbt(&[Tap::Key, Tap::Hidden]);
    let w = bad.inner().inputs[0].final_script_witness.clone().unwrap();
    let mut sig = w.nth(0).unwrap().to_vec();
    sig[63] ^= 1;
    bad.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[sig]));
    let mut app = with_inbox(vec![("t.psbt", bad.to_bytes())]);
    let o = checked(&mut app);
    assert!(!o.check.only_witness_wrong());
    assert_eq!(o.check.headline(), EN.nonce_check_nonce_check);
}

#[test]
fn collect_blames_the_input_and_not_the_file_where_the_cosigner_states_no_output() {
    use faraday_core::wallet::{Session, Spend};
    let theirs = taproot_psbt(&[Tap::Hidden]);
    let mut own = theirs.clone();
    own.inner_mut().inputs[0].final_script_witness = None;
    let mut theirs = theirs;
    theirs.inner_mut().inputs[0].witness_utxo = None;
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &theirs.to_bytes(),
            "c.psbt",
        )
        .unwrap_err();
    assert_eq!(why, mismatch_text(0));
}

#[test]
fn collect_refuses_an_own_final_input_only_if_it_is_still_unread_after_the_merge() {
    use faraday_core::wallet::{Session, Spend};
    let clean = taproot_psbt(&[lock(false)]);
    let mut own = clean.clone();
    own.inner_mut().inputs[0].witness_utxo = None;
    let session = Session::default();
    let c = testkit::compute();
    // The file states the output the own copy lacks: merged, finishes.
    let mut spend = Spend::new(own.clone(), "own.psbt");
    spend
        .collect(&c, &session, &clean.to_bytes(), "c.psbt")
        .expect("the merged PSBT stands");
    assert!(spend.psbt.inner().inputs[0].witness_utxo.is_some());
    assert!(spend.finish().is_ok());
    // A file that states none leaves it unread: refused, nothing merged.
    let mut spend = Spend::new(own.clone(), "own.psbt");
    let before = spend.psbt.to_bytes();
    let why = spend
        .collect(&c, &session, &own.to_bytes(), "c.psbt")
        .unwrap_err();
    assert_eq!(why, EN.psbt_final_prev_unread.replace("{}", "0"));
    assert_eq!(spend.psbt.to_bytes(), before);
}

#[test]
fn a_previous_transaction_of_another_txid_beside_a_witness_utxo_is_refused() {
    use faraday_core::wallet::{Session, Spend};
    let mut psbt = taproot_psbt(&[Tap::Key, lock(false)]);
    let out = psbt.inner().inputs[1].witness_utxo.clone().unwrap();
    psbt.inner_mut().inputs[1].non_witness_utxo = Some(other_previous(out, 1));
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    assert_eq!(o.check.unread, 1);
    assert!(!o.check.sound());
    assert!(o.finished.is_none());
    assert!(!offered_after(&psbt));
    let mut spend = Spend::new(psbt.clone(), "t.psbt");
    assert_eq!(
        spend.finish().unwrap_err(),
        EN.psbt_final_prev_unread.replace("{}", "1")
    );
    assert!(spend.finished.is_none());
    // Over an own witness_utxo, the cosigner's file is not merged.
    let mut own = psbt.clone();
    own.inner_mut().inputs[1].non_witness_utxo = None;
    own.inner_mut().inputs[1].final_script_witness = None;
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(
            &testkit::compute(),
            &Session::default(),
            &psbt.to_bytes(),
            "c.psbt",
        )
        .unwrap_err();
    assert_eq!(why, EN.psbt_final_prev_unread.replace("{}", "1"));
}

/// The nonce check of `psbt` offers to carry on.
fn offered_after(psbt: &faraday_psbt::Psbt) -> bool {
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = checked(&mut app);
    !o.check.refused(o.finished.is_some())
}

#[test]
fn a_raw_transaction_beside_a_psbt_whose_previous_transaction_is_another_one_reads_no_output() {
    use faraday_bip::bitcoin::{Amount, TxOut};
    use faraday_core::decode::CheckPress;
    let (hex, unsigned) = finished_savings();
    let mut psbt = faraday_psbt::Psbt::parse_bytes(&testkit::compute(), &unsigned).expect("psbt");
    for (i, input) in psbt.inner_mut().inputs.iter_mut().enumerate() {
        input.witness_utxo = None;
        // Rich outputs under another txid: amounts read from them would
        // show a fee.
        let rich = TxOut {
            value: Amount::from_sat(10_000_000_000),
            script_pubkey: Default::default(),
        };
        input.non_witness_utxo = Some(other_previous(rich, i));
    }
    let mut app = with_inbox(vec![
        ("broadcast-me.txn", hex.into_bytes()),
        ("spend.psbt", psbt.to_bytes()),
    ]);
    add_key(&mut app, "bacon");
    let o = checked(&mut app);
    assert!(o.check.unread > 0, "no output is read from another txid");
    app.press(Action::NonceCheck(CheckPress::Decode));
    assert_eq!(app.screen, Screen::Decode);
    assert!(app.decode().unwrap().decoded.fee.is_none());
}

#[test]
fn a_native_taproot_input_with_a_script_sig_and_no_witness_is_refused() {
    use faraday_bip::bitcoin::script::Builder;
    use faraday_core::wallet::Spend;
    let mut psbt = taproot_psbt(&[Tap::Key]);
    let input = &mut psbt.inner_mut().inputs[0];
    input.final_script_witness = None;
    input.final_script_sig = Some(Builder::new().push_int(1).into_script());
    let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
    let o = opened(&mut app);
    assert_eq!(o.check.mismatched, vec![0]);
    assert!(!o.check.sound() && o.finished.is_none());
    let mut spend = Spend::new(psbt, "t.psbt");
    assert_eq!(spend.finish().unwrap_err(), mismatch_text(0));
    assert!(spend.finished.is_none());
}

/// A witness no output of a p2tr key takes, on input 0.
fn junk_witness(psbt: &mut faraday_psbt::Psbt) {
    psbt.inner_mut().inputs[0].final_script_witness =
        Some(faraday_bip::bitcoin::Witness::from_slice(&[
            vec![1u8; 10],
            vec![2u8; 33],
        ]));
}

#[test]
fn a_file_and_a_spend_are_each_blamed_for_the_witness_failing_under_their_own_output() {
    use faraday_core::wallet::{Session, Spend};
    let c = testkit::compute();
    let session = Session::default();
    let bare = |p: &mut faraday_psbt::Psbt| {
        for input in &mut p.inner_mut().inputs {
            input.final_script_witness = None;
        }
    };
    // The file's input 1 fails under its own output; its input 0 states
    // no output, and fails only under the one the spend states.
    let mut file = taproot_psbt(&[Tap::Key, lock(true)]);
    junk_witness(&mut file);
    file.inner_mut().inputs[0].witness_utxo = None;
    let mut own = taproot_psbt(&[Tap::Key, lock(false)]);
    bare(&mut own);
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(&c, &session, &file.to_bytes(), "file.psbt")
        .unwrap_err();
    assert_eq!(why, mismatch_text(1));
    // The same the other way round: the spend's input 1 fails under its
    // own output, the file's input 0 only under the spend's.
    let mut own = taproot_psbt(&[Tap::Key, lock(true)]);
    own.inner_mut().inputs[0].final_script_witness = None;
    let mut file = taproot_psbt(&[Tap::Key, lock(false)]);
    bare(&mut file);
    junk_witness(&mut file);
    file.inner_mut().inputs[0].witness_utxo = None;
    let mut spend = Spend::new(own, "own.psbt");
    let why = spend
        .collect(&c, &session, &file.to_bytes(), "file.psbt")
        .unwrap_err();
    assert_eq!(why, mismatch_text(1));
}

#[test]
fn a_native_taproot_spend_with_a_script_sig_is_refused() {
    use faraday_bip::bitcoin::script::Builder;
    use faraday_core::wallet::Spend;
    for (what, tap) in [("key path", Tap::Key), ("script path", lock(false))] {
        let mut psbt = taproot_psbt(&[tap]);
        psbt.inner_mut().inputs[0].final_script_sig =
            Some(Builder::new().push_slice([1u8]).into_script());
        let mut app = with_inbox(vec![("t.psbt", psbt.to_bytes())]);
        let o = opened(&mut app);
        assert_eq!(o.check.mismatched, vec![0], "{what}");
        assert!(!o.check.sound() && o.finished.is_none(), "{what}");
        let mut spend = Spend::new(psbt, "t.psbt");
        assert_eq!(spend.finish().unwrap_err(), mismatch_text(0), "{what}");
    }
}

#[test]
fn the_reason_shown_for_a_spend_follows_what_a_collect_changes() {
    use faraday_core::wallet::Spend;
    let clean = taproot_psbt(&[lock(false)]);
    let mut own = clean.clone();
    own.inner_mut().inputs[0].witness_utxo = None;
    let unread = EN.psbt_final_prev_unread.replace("{}", "0");
    let mut app = with_inbox(vec![
        ("own.psbt", own.to_bytes()),
        ("clean.psbt", clean.to_bytes()),
    ]);
    app.press(Action::StartSpend(0));
    let state = |app: &mut Faraday| {
        let _ = app.frame();
        let s = app.spend().expect("a spend");
        (s.refused.clone(), Spend::refused(&s.spend))
    };
    let (kept, fresh) = state(&mut app);
    assert_eq!(kept.as_deref(), Some(unread.as_str()));
    assert_eq!(kept, fresh);
    app.press(Action::Collect(1));
    let (kept, fresh) = state(&mut app);
    assert_eq!(fresh, None);
    assert_eq!(kept, fresh, "the kept reason is the fresh one");
}
