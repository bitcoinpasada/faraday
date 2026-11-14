//! Single-signature signing for the four script types on regtest: the
//! inspection recognises the input and change, the signature verifies
//! against an independently computed sighash, and the finalized
//! transaction has the right structure.

use crate::common::builder::{Build, single_sig};
use crate::common::{ABANDON, ZOO, master};
use bitcoin::consensus::{deserialize, serialize};
use bitcoin::secp256k1::{Message, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::{Amount, ScriptBuf, Transaction};
use faraday_bip::keys::{Network, ScriptType};
use faraday_crypto::work::Compute;
use faraday_psbt::{
    Aux, Context, Error, KeyRef, Nonce, OutputKind, Psbt, ScriptKind, SigKind, WarningKind,
    finalize, inspect, sign,
};

const NET: Network = Network::Regtest;

fn foreign_script() -> ScriptBuf {
    let c = Compute::unchecked();
    master(ZOO, NET)
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap()
        .address(false, 0)
        .unwrap()
        .script_pubkey()
}

fn build(script_type: ScriptType) -> Psbt {
    let mut b = Build::new(script_type, NET);
    b.recipients.push((foreign_script(), 60_000));
    b.change = Some(39_000);
    single_sig(&master(ABANDON, NET), &b)
}

fn expected_kind(script_type: ScriptType) -> ScriptKind {
    match script_type {
        ScriptType::Legacy => ScriptKind::P2pkh,
        ScriptType::NestedSegwit => ScriptKind::P2shP2wpkh,
        ScriptType::NativeSegwit => ScriptKind::P2wpkh,
        ScriptType::Taproot => ScriptKind::P2trKey,
    }
}

/// Recomputes the sighash for input 0 without going through `faraday-psbt`.
fn independent_sighash(psbt: &Psbt, script_type: ScriptType) -> Message {
    let inner = psbt.inner();
    let mut cache = SighashCache::new(&inner.unsigned_tx);
    let utxo = inner.spend_utxo(0).unwrap();
    match script_type {
        ScriptType::Legacy => Message::from(
            cache
                .legacy_signature_hash(0, &utxo.script_pubkey, EcdsaSighashType::All.to_u32())
                .unwrap(),
        ),
        ScriptType::NestedSegwit => {
            let redeem = inner.inputs[0].redeem_script.as_ref().unwrap();
            Message::from(
                cache
                    .p2wpkh_signature_hash(0, redeem, utxo.value, EcdsaSighashType::All)
                    .unwrap(),
            )
        }
        ScriptType::NativeSegwit => Message::from(
            cache
                .p2wpkh_signature_hash(0, &utxo.script_pubkey, utxo.value, EcdsaSighashType::All)
                .unwrap(),
        ),
        ScriptType::Taproot => Message::from(
            cache
                .taproot_key_spend_signature_hash(
                    0,
                    &Prevouts::All(core::slice::from_ref(utxo)),
                    TapSighashType::Default,
                )
                .unwrap(),
        ),
    }
}

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "one test that signs, verifies and finalizes a transaction of every script type in turn"
)]
fn sign_verify_finalize_every_script_type() {
    let c = Compute::unchecked();
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    for script_type in ScriptType::ALL {
        let mut psbt = build(script_type);
        let unsigned = psbt.clone();

        let insp = inspect(&c, &psbt, &ctx);
        assert!(!insp.has_danger(), "{script_type}: {:?}", insp.warnings);
        assert_eq!(insp.inputs.len(), 1);
        assert_eq!(insp.inputs[0].script_type, expected_kind(script_type));
        assert!(insp.inputs[0].is_ours);
        assert_eq!(insp.inputs[0].amount, Some(Amount::from_sat(100_000)));
        assert_eq!(
            insp.inputs[0].origin.as_ref().unwrap().fingerprint,
            abandon.fingerprint()
        );
        assert!(insp.rbf);
        assert_eq!(insp.outputs[0].kind, OutputKind::Recipient);
        assert_eq!(
            insp.outputs[1].kind,
            OutputKind::Change {
                key: abandon.fingerprint(),
                change: true,
                index: 0
            }
        );
        assert_eq!(insp.fee, Amount::from_sat(1_000));
        assert_eq!(insp.amount_to_others, Amount::from_sat(60_000));
        assert_eq!(insp.change_total, Amount::from_sat(39_000));
        assert_eq!(insp.participating_keys, vec![abandon.fingerprint()]);
        assert!(insp.fee_rate_sat_vb.unwrap() > 1.0);
        assert!(!insp.is_self_transfer);

        let result = sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            &[],
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
        )
        .unwrap();
        assert!(result.complete, "{script_type}");
        assert_eq!(result.signed_inputs.len(), 1);
        let s = &result.signed_inputs[0];
        assert_eq!(s.index, 0);
        assert_eq!(s.fingerprint, abandon.fingerprint());
        assert!(s.verified && s.deterministic_ok);

        // Independent verification against our own sighash computation.
        let msg = independent_sighash(&unsigned, script_type);
        let spk = &unsigned.inner().spend_utxo(0).unwrap().script_pubkey;
        if script_type == ScriptType::Taproot {
            assert_eq!(s.kind, SigKind::Schnorr);
            assert_eq!(s.sig_bytes.len(), 64);
            let sig = bitcoin::secp256k1::schnorr::Signature::from_slice(&s.sig_bytes).unwrap();
            let output_key = XOnlyPublicKey::from_slice(&spk.as_bytes()[2..]).unwrap();
            secp.verify_schnorr(&sig, &msg, &output_key).unwrap();
            assert_eq!(psbt.inner().inputs[0].tap_key_sig.unwrap().signature, sig);
        } else {
            assert_eq!(s.kind, SigKind::Ecdsa);
            assert_eq!(*s.sig_bytes.last().unwrap(), 0x01, "SIGHASH_ALL byte");
            let sig = bitcoin::ecdsa::Signature::from_slice(&s.sig_bytes).unwrap();
            let (pk, _) = unsigned.inner().inputs[0]
                .bip32_derivation
                .iter()
                .next()
                .unwrap();
            secp.verify_ecdsa(&msg, &sig.signature, pk).unwrap();
            let stored = psbt.inner().inputs[0].partial_sigs.values().next().unwrap();
            assert_eq!(stored.to_vec(), s.sig_bytes);
        }

        // Finalize: extract, consensus round trip, witness structure.
        let tx = finalize(&mut psbt).unwrap().expect("complete");
        let bytes = serialize(&tx);
        let again: Transaction = deserialize(&bytes).unwrap();
        assert_eq!(again, tx);
        assert_eq!(tx.input.len(), 1);
        assert_eq!(tx.output, unsigned.unsigned_tx().output);
        let txin = &tx.input[0];
        match script_type {
            ScriptType::Legacy => {
                assert!(txin.witness.is_empty());
                let pushes: Vec<_> = txin.script_sig.instructions().map(|i| i.unwrap()).collect();
                assert_eq!(pushes.len(), 2);
                assert_eq!(pushes[0].push_bytes().unwrap().as_bytes(), s.sig_bytes);
            }
            ScriptType::NestedSegwit => {
                assert_eq!(txin.witness.len(), 2);
                let redeem = unsigned.inner().inputs[0].redeem_script.as_ref().unwrap();
                let pushes: Vec<_> = txin.script_sig.instructions().map(|i| i.unwrap()).collect();
                assert_eq!(pushes.len(), 1);
                assert_eq!(
                    pushes[0].push_bytes().unwrap().as_bytes(),
                    redeem.as_bytes()
                );
                assert_eq!(txin.witness.nth(0).unwrap(), s.sig_bytes);
            }
            ScriptType::NativeSegwit => {
                assert!(txin.script_sig.is_empty());
                assert_eq!(txin.witness.len(), 2);
                assert_eq!(txin.witness.nth(0).unwrap(), s.sig_bytes);
                assert_eq!(txin.witness.nth(1).unwrap().len(), 33);
            }
            ScriptType::Taproot => {
                assert!(txin.script_sig.is_empty());
                assert_eq!(txin.witness.len(), 1);
                assert_eq!(txin.witness.nth(0).unwrap(), s.sig_bytes);
            }
        }
        // The finalized input keeps only UTXO data.
        let fin = &psbt.inner().inputs[0];
        assert!(fin.partial_sigs.is_empty() && fin.bip32_derivation.is_empty());
        assert!(fin.redeem_script.is_none() && fin.tap_key_origins.is_empty());
        assert!(fin.witness_utxo.is_some() || fin.non_witness_utxo.is_some());
        // A second finalize is a no-op that still extracts.
        assert_eq!(finalize(&mut psbt).unwrap().unwrap(), tx);
        // A finalized PSBT names no keys any more: refused as "nothing to
        // sign", with no hold past it, and the attempt touches nothing.
        assert_eq!(
            sign(
                &c,
                &mut psbt,
                &[&abandon],
                &[abandon.fingerprint()],
                &ctx,
                &[],
                Nonce::LowR,
                Aux::Deterministic,
                &mut None,
                Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
            ),
            Err(Error::Danger(vec![WarningKind::NoParticipatingKey]))
        );
        assert_eq!(finalize(&mut psbt).unwrap().unwrap(), tx);
    }
}

#[test]
fn signing_needs_a_matching_master_key() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let mut psbt = build(ScriptType::NativeSegwit);
    assert_eq!(
        sign(
            &c,
            &mut psbt,
            &[&zoo],
            &[abandon.fingerprint()],
            &ctx,
            &[],
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
        ),
        Err(Error::NoSuchKey(abandon.fingerprint()))
    );
    // Selecting a key that is loaded but not named by any input signs nothing.
    let r = sign(
        &c,
        &mut psbt,
        &[&zoo],
        &[zoo.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .unwrap();
    assert!(r.signed_inputs.is_empty() && !r.complete);
    assert_eq!(finalize(&mut psbt).unwrap(), None);
}

#[test]
fn key_origin_that_does_not_match_the_derived_key_is_refused() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    // Zoo's PSBT with the origin relabelled to abandon's fingerprint.
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 90_000));
    let mut psbt = single_sig(&zoo, &b);
    let input = &mut psbt.inner_mut().inputs[0];
    for (fp, _) in input.bip32_derivation.values_mut() {
        *fp = abandon.fingerprint().into();
    }
    let insp = inspect(&c, &psbt, &ctx);
    assert!(!insp.inputs[0].is_ours);
    // A false claim about the person's own key: Review refuses it as
    // signing does (F136).
    assert!(insp.refuses());
    assert_eq!(
        sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            &[],
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
        ),
        Err(Error::Blocked(vec![WarningKind::UnknownDerivation]))
    );
    assert!(psbt.inner().inputs[0].partial_sigs.is_empty());
}

/// A taproot input that states a script tree and then gives no leaf of
/// it, and whose internal key with that tree is not the output being
/// spent, has nothing this device can sign, and says so.
#[test]
fn a_taproot_tree_with_no_way_into_it_is_refused() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let mut psbt = build(ScriptType::Taproot);
    psbt.inner_mut().inputs[0].tap_merkle_root = Some(bitcoin::TapNodeHash::from_script(
        bitcoin::Script::new(),
        bitcoin::taproot::LeafVersion::TapScript,
    ));
    let insp = inspect(&c, &psbt, &ctx);
    assert_eq!(insp.inputs[0].script_type, ScriptKind::P2trScript);
    assert!(
        insp.warnings
            .iter()
            .any(|w| w.kind == WarningKind::UnsupportedInput),
        "{:?}",
        insp.warnings
    );
    assert_eq!(
        sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            &[],
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
        ),
        Err(Error::Danger(vec![WarningKind::NoParticipatingKey]))
    );
}

/// A PSBT read back from its bytes, as a coordinator's file is read.
fn reread(psbt: bitcoin::Psbt) -> Psbt {
    Psbt::parse_bytes(&Compute::unchecked(), &psbt.serialize()).unwrap()
}

/// An input that arrives final, carrying a funding transaction with no
/// output at the index it spends, is refused when the PSBT is finalized
/// rather than stopping the program (the release fuzz run's psbt_parse
/// crash: `bitcoin`'s extractor panics on it).
#[test]
fn a_final_input_whose_funding_transaction_lacks_the_spent_output_is_refused() {
    let mut inner = build(ScriptType::Legacy).inner().clone();
    inner.unsigned_tx.input[0].previous_output.vout = 3;
    let input = &mut inner.inputs[0];
    input.final_script_sig = Some(ScriptBuf::from_bytes(vec![0x51]));
    input.bip32_derivation.clear();
    let mut psbt = reread(inner);
    assert_eq!(
        finalize(&mut psbt),
        Err(Error::Finalize {
            input: 0,
            reason: faraday_psbt::Reason::PrevTxNoOutput
        })
    );
}

/// An input the finalizer cannot build names itself: the second input,
/// a script this crate does not know, is the one the refusal reports.
#[test]
fn a_refused_second_input_is_named_when_the_psbt_is_finalized() {
    let mut inner = build(ScriptType::NativeSegwit).inner().clone();
    let funding = crate::common::builder::funding_tx(ScriptBuf::from_bytes(vec![0x51]), 5_000);
    inner.unsigned_tx.input.push(bitcoin::TxIn {
        previous_output: bitcoin::OutPoint::new(funding.compute_txid(), 0),
        ..Default::default()
    });
    inner.inputs.push(bitcoin::psbt::Input {
        witness_utxo: Some(funding.output[0].clone()),
        non_witness_utxo: Some(funding),
        ..Default::default()
    });
    let mut psbt = reread(inner);
    assert_eq!(
        finalize(&mut psbt),
        Err(Error::Finalize {
            input: 1,
            reason: faraday_psbt::Reason::UnknownScript
        })
    );
}

/// A taproot input's signature commits to every input's spent output;
/// another input whose funding transaction lacks the output it spends
/// leaves that signature unchecked and the PSBT unsigned, and stops
/// nothing (`bitcoin`'s `spend_utxo` panics on it).
#[test]
fn a_taproot_signature_beside_an_input_without_its_spent_output_does_not_stop_the_program() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let mut inner = build(ScriptType::Taproot).inner().clone();
    inner.inputs[0].tap_key_sig =
        Some(bitcoin::taproot::Signature::from_slice(&[1u8; 64]).unwrap());
    let funding = crate::common::builder::funding_tx(foreign_script(), 5_000);
    inner.unsigned_tx.input.push(bitcoin::TxIn {
        previous_output: bitcoin::OutPoint::new(funding.compute_txid(), 3),
        ..Default::default()
    });
    inner.inputs.push(bitcoin::psbt::Input {
        non_witness_utxo: Some(funding),
        ..Default::default()
    });
    let mut psbt = reread(inner);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let _ = inspect(&c, &psbt, &ctx);
    let signed = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    );
    assert!(signed.is_err(), "{signed:?}");
    assert!(finalize(&mut psbt).is_err());
}

/// A taproot key-path input of a loaded key that carries another
/// program's proprietary records is signed, and the records stay as they
/// were: BIP 174 has a signer pass over proprietary fields it does not
/// know.
#[test]
fn a_key_path_input_with_another_programs_proprietary_records_is_signed() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let mut psbt = build(ScriptType::Taproot);
    let key = bitcoin::psbt::raw::ProprietaryKey {
        prefix: b"another-wallet".to_vec(),
        subtype: 0,
        key: vec![2; 66],
    };
    psbt.inner_mut().inputs[0]
        .proprietary
        .insert(key.clone(), vec![0; 66]);
    assert!(inspect(&c, &psbt, &ctx).blocked().is_empty());
    let r = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    );
    assert!(r.is_ok(), "{r:?}");
    let input = &psbt.inner().inputs[0];
    assert!(input.tap_key_sig.is_some(), "signed");
    assert_eq!(
        input.proprietary.get(&key),
        Some(&vec![0; 66]),
        "the records kept"
    );
}

/// `bytes` of an unsigned one-input file with `n` derivations of other
/// keys, each key written as 65 uncompressed bytes, put first in the
/// input's map. Every key's compressed form sorts before `real`, the
/// device's own key, so that writing the file out has to place the
/// signature after each of them.
fn with_uncompressed_derivations(
    bytes: &[u8],
    n: u32,
    real: &bitcoin::secp256k1::PublicKey,
) -> Vec<u8> {
    // Past the magic and the global map: pairs up to a zero length key.
    let mut at = 5;
    while bytes[at] != 0 {
        let key = usize::from(bytes[at]);
        let value = usize::from(bytes[at + 1 + key]);
        assert!(value < 0xfd);
        at += 2 + key + value;
    }
    at += 1;
    let secp = Secp256k1::new();
    let one = {
        let mut secret = [0u8; 32];
        secret[31] = 1;
        bitcoin::secp256k1::PublicKey::from_secret_key(
            &secp,
            &bitcoin::secp256k1::SecretKey::from_slice(&secret).unwrap(),
        )
    };
    let limit = real.serialize();
    // The points are 1G, 2G, ...: one addition each.
    let mut pk = one;
    let mut added = Vec::new();
    let mut count = 0;
    while count < n {
        if pk.serialize() < limit {
            added.extend_from_slice(&[0x42, 0x06]);
            added.extend_from_slice(&pk.serialize_uncompressed());
            added.extend_from_slice(&[0x08, 1, 2, 3, 4, 0x2c, 0, 0, 0x80]);
            count += 1;
        }
        pk = pk.combine(&one).unwrap();
    }
    let mut out = bytes[..at].to_vec();
    out.extend_from_slice(&added);
    out.extend_from_slice(&bytes[at..]);
    out
}

/// The file `with_uncompressed_derivations` makes, signed and written out:
/// the bytes read, with only the signature put in, before the key that
/// `bitcoin` writes after it. The third value is the time the write-out
/// took; the file is built, read and signed before it starts.
fn signed_with_uncompressed_derivations(n: u32) -> (Vec<u8>, Vec<u8>, std::time::Duration) {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let unsigned = build(ScriptType::NativeSegwit);
    let real = *unsigned.inner().inputs[0]
        .bip32_derivation
        .keys()
        .next()
        .unwrap();
    let before = with_uncompressed_derivations(&unsigned.to_bytes(), n, &real);
    let mut psbt = Psbt::parse_bytes(&c, &before).expect("it parses");
    sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &[],
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .unwrap();
    let start = std::time::Instant::now();
    let after = psbt.to_bytes();
    let took = start.elapsed();
    // The signature goes before the real key's derivation.
    let mut marker = vec![0x22, 0x06];
    marker.extend_from_slice(&real.serialize());
    let at = before
        .windows(marker.len())
        .position(|w| w == marker)
        .expect("the real key's derivation");
    let added = after.len() - before.len();
    assert_eq!(&after[..at], &before[..at]);
    assert_eq!(&after[at + added..], &before[at..]);
    assert_eq!(&after[at..at + 2], &[0x22, 0x02]);
    (before, after, took)
}

#[test]
fn a_signed_file_with_uncompressed_derivations_is_written_with_its_bytes_as_read() {
    let (before, after, _) = signed_with_uncompressed_derivations(20);
    assert!(after.len() > before.len());
}

#[test]
fn a_signed_file_with_many_uncompressed_derivations_is_written_out_quickly() {
    let (_, _, took) = signed_with_uncompressed_derivations(200_000);
    assert!(took < std::time::Duration::from_secs(10), "{took:?}");
}

/// BIP-322's `to_spend` and `to_sign` for `script_pubkey` and a message,
/// written from the BIP's text: `to_spend` (version 0) spends the
/// outpoint of all zeros at index `0xFFFFFFFF` with `OP_0 <message hash>`
/// and pays 0 to the script; `to_sign` (version 0) spends `to_spend`'s
/// output 0 at sequence 0 and pays 0 to `OP_RETURN`.
fn bip322_transactions(script_pubkey: &ScriptBuf, message: &str) -> (Transaction, Transaction) {
    use bitcoin::hashes::Hash;
    let hash = faraday_psbt::message::message_hash(message);
    let push: &bitcoin::script::PushBytes = (&hash).into();
    let to_spend = Transaction {
        version: bitcoin::transaction::Version(0),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint {
                txid: bitcoin::Txid::from_byte_array([0; 32]),
                vout: 0xFFFF_FFFF,
            },
            script_sig: bitcoin::script::Builder::new()
                .push_opcode(bitcoin::opcodes::OP_0)
                .push_slice(push)
                .into_script(),
            sequence: bitcoin::Sequence(0),
            witness: bitcoin::Witness::new(),
        }],
        output: vec![bitcoin::TxOut {
            value: Amount::ZERO,
            script_pubkey: script_pubkey.clone(),
        }],
    };
    let to_sign = Transaction {
        version: bitcoin::transaction::Version(0),
        lock_time: bitcoin::absolute::LockTime::ZERO,
        input: vec![bitcoin::TxIn {
            previous_output: bitcoin::OutPoint::new(to_spend.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: bitcoin::Sequence(0),
            witness: bitcoin::Witness::new(),
        }],
        output: vec![bitcoin::TxOut {
            value: Amount::ZERO,
            script_pubkey: bitcoin::script::Builder::new()
                .push_opcode(bitcoin::opcodes::all::OP_RETURN)
                .into_script(),
        }],
    };
    (to_spend, to_sign)
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= chunk.len() {
                ABC[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    out
}

/// A coordinator hands over BIP-322's `to_sign` for "Hello World" and
/// the person's first receive address, as a PSBT: a version 0
/// transaction spending 0 from that address to an `OP_RETURN`. Signing
/// it would be signing that message, whose text the Review never shows,
/// so the signer refuses it, even with every hold it allows held; it
/// does not sign it as a spend.
#[test]
fn a_bip322_message_dressed_as_a_spend_is_not_signed() {
    let c = Compute::unchecked();
    let vectors: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tools/vectors/message/bip322-basic.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let hello = vectors["tx_hashes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["message"] == "Hello World")
        .expect("BIP-322's Hello World vector");
    let address: bitcoin::Address = hello["address"]
        .as_str()
        .unwrap()
        .parse::<bitcoin::Address<bitcoin::address::NetworkUnchecked>>()
        .unwrap()
        .assume_checked();
    let (to_spend, to_sign) = bip322_transactions(&address.script_pubkey(), "Hello World");
    assert_eq!(
        to_spend.compute_txid().to_string(),
        hello["to_spend_tx_hash"].as_str().unwrap()
    );
    assert_eq!(
        to_sign.compute_txid().to_string(),
        hello["to_sign_tx_hash"].as_str().unwrap(),
        "the transactions are BIP-322's own"
    );

    // The same two for the person's first receive address.
    let abandon = master(ABANDON, NET);
    let account = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap();
    let mine = account.address(false, 0).unwrap();
    let (to_spend, to_sign) = bip322_transactions(&mine.script_pubkey(), "Hello World");
    let mut inner = bitcoin::Psbt::from_unsigned_tx(to_sign).unwrap();
    inner.inputs[0].witness_utxo = Some(to_spend.output[0].clone());
    inner.inputs[0].non_witness_utxo = Some(to_spend.clone());
    let h = |i| bitcoin::bip32::ChildNumber::from_hardened_idx(i).unwrap();
    let n = |i| bitcoin::bip32::ChildNumber::from_normal_idx(i).unwrap();
    let path =
        bitcoin::bip32::DerivationPath::from(vec![h(84), h(NET.coin_type()), h(0), n(0), n(0)]);
    let key = abandon.derive(&c, &path).to_xpub(&c).public_key;
    inner.inputs[0]
        .bip32_derivation
        .insert(key, (abandon.fingerprint().into(), path));
    let mut psbt = Psbt::from(inner);

    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, &psbt, &ctx);
    let held: Vec<WarningKind> = insp
        .warnings
        .iter()
        .filter(|w| w.kind.is_holdable())
        .map(|w| w.kind.clone())
        .collect();
    let result = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &held,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    );
    let witness = psbt.inner().inputs[0]
        .partial_sigs
        .iter()
        .next()
        .map(|(pk, sig)| {
            let mut w = bitcoin::Witness::new();
            w.push(sig.to_vec());
            w.push(pk.to_bytes());
            base64(&serialize(&w))
        });
    let as_message = witness.as_ref().map(|w| {
        faraday_psbt::message::verify(
            &bitcoin::Address::from_script(&mine.script_pubkey(), bitcoin::Network::Regtest)
                .unwrap(),
            "Hello World",
            w,
        )
    });
    assert!(
        matches!(&result, Err(Error::Blocked(k)) if k.contains(&WarningKind::MessageRequest))
            && witness.is_none(),
        "signed: {result:?}; warnings {:?}; the signature read as BIP-322's for \"Hello World\": {as_message:?}",
        insp.warnings
    );
}

fn op_return() -> ScriptBuf {
    bitcoin::script::Builder::new()
        .push_opcode(bitcoin::opcodes::all::OP_RETURN)
        .into_script()
}

/// What signing `psbt` with the abandon key does with every hold held:
/// the result and whether any input holds a signature afterwards.
fn sign_held(psbt: &Psbt) -> (Result<(), Error>, bool, Vec<WarningKind>) {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&c, &abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
    };
    let insp = inspect(&c, psbt, &ctx);
    let held: Vec<WarningKind> = insp
        .warnings
        .iter()
        .filter(|w| w.kind.is_holdable())
        .map(|w| w.kind.clone())
        .collect();
    let mut psbt = psbt.clone();
    let result = sign(
        &c,
        &mut psbt,
        &[&abandon],
        &[abandon.fingerprint()],
        &ctx,
        &held,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        Some(&faraday_crypto::Zeroizing::new([0u8; 32])),
    )
    .map(|_| ());
    let signed = psbt.inner().inputs.iter().any(|i| {
        !i.partial_sigs.is_empty() || i.tap_key_sig.is_some() || !i.tap_script_sigs.is_empty()
    });
    let kinds = insp.warnings.iter().map(|w| w.kind.clone()).collect();
    (result, signed, kinds)
}

fn assert_message_request_blocked(psbt: &Psbt) {
    let (result, signed, _) = sign_held(psbt);
    assert!(
        matches!(&result, Err(Error::Blocked(k)) if k.contains(&WarningKind::MessageRequest)),
        "{result:?}"
    );
    assert!(!signed);
}

/// A message request in a version 2 transaction, with the transaction
/// it spends attached, is refused as one.
#[test]
fn a_version_2_bip322_request_with_its_to_spend_is_not_signed() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let account = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap();
    let mine = account.address(false, 0).unwrap();
    let (to_spend, mut to_sign) = bip322_transactions(&mine.script_pubkey(), "Hello World");
    to_sign.version = bitcoin::transaction::Version::TWO;
    let mut inner = bitcoin::Psbt::from_unsigned_tx(to_sign).unwrap();
    inner.inputs[0].witness_utxo = Some(to_spend.output[0].clone());
    inner.inputs[0].non_witness_utxo = Some(to_spend);
    assert_message_request_blocked(&Psbt::from(inner));
}

/// A message request in a version 2 transaction that states only the
/// amount spent (0, which the signature commits to) is refused too, for
/// a native SegWit key and for a taproot key.
#[test]
fn a_version_2_bip322_request_with_only_the_amount_spent_is_not_signed() {
    for script_type in [ScriptType::NativeSegwit, ScriptType::Taproot] {
        let mut b = Build::new(script_type, NET);
        b.input_value = 0;
        b.recipients.push((op_return(), 0));
        let built = single_sig(&master(ABANDON, NET), &b);
        let mut inner = built.inner().clone();
        inner.inputs[0].non_witness_utxo = None;
        assert!(inner.inputs[0].witness_utxo.is_some());
        assert_message_request_blocked(&Psbt::from(inner));
    }
}

/// A message request in a version 2 transaction whose input 0 carries
/// only the transaction it spends (no `witness_utxo`) is refused.
#[test]
fn a_version_2_bip322_request_with_only_the_transaction_spent_is_not_signed() {
    let c = Compute::unchecked();
    let abandon = master(ABANDON, NET);
    let account = abandon
        .account_xpub(&c, ScriptType::NativeSegwit, 0)
        .unwrap();
    let mine = account.address(false, 0).unwrap();
    let (to_spend, mut to_sign) = bip322_transactions(&mine.script_pubkey(), "Hello World");
    to_sign.version = bitcoin::transaction::Version::TWO;
    let mut inner = bitcoin::Psbt::from_unsigned_tx(to_sign).unwrap();
    inner.inputs[0].witness_utxo = None;
    inner.inputs[0].non_witness_utxo = Some(to_spend);
    assert_message_request_blocked(&Psbt::from(inner));
}

/// BIP-322's proof of funds: input 0 is the `to_spend` and states only
/// 1 sat, which is not checked (SegWit v0 and legacy signatures do not
/// commit to it), and input 1 is the person's own coin. It is refused as a message request, whatever
/// the script type of the person's coin, and nothing is signed.
#[test]
fn a_bip322_proof_of_funds_stating_an_amount_on_its_first_input_is_not_signed() {
    for script_type in [ScriptType::NativeSegwit, ScriptType::Taproot] {
        let mut b = Build::new(script_type, NET);
        b.recipients.push((op_return(), 0));
        let built = single_sig(&master(ABANDON, NET), &b);
        let mut inner = built.inner().clone();
        let to_spend_id = {
            use bitcoin::hashes::Hash;
            bitcoin::Txid::from_byte_array([7; 32])
        };
        inner.unsigned_tx.input.insert(
            0,
            bitcoin::TxIn {
                previous_output: bitcoin::OutPoint::new(to_spend_id, 0),
                script_sig: ScriptBuf::new(),
                sequence: bitcoin::Sequence(0),
                witness: bitcoin::Witness::new(),
            },
        );
        let mut taproot = vec![0x51, 0x20];
        taproot.extend([9u8; 32]);
        inner.inputs.insert(
            0,
            bitcoin::psbt::Input {
                witness_utxo: Some(bitcoin::TxOut {
                    value: Amount::from_sat(1),
                    script_pubkey: ScriptBuf::from_bytes(taproot),
                }),
                ..Default::default()
            },
        );
        assert_message_request_blocked(&Psbt::from(inner));
    }
}

/// A payment with an OP_RETURN note beside it is not taken for a message
/// request. A coin burned to a 0 OP_RETURN as the only output is one,
/// whichever output of its transaction it is and whatever the sighash:
/// BIP-322 asks for exactly that output, and an ANYONECANPAY signature
/// does not commit to the other inputs, so a to_spend can be put first
/// afterwards. Nothing is signed. The amount of input 0 is not checked,
/// since SegWit v0 and legacy signatures do not commit to it.
#[test]
fn an_op_return_beside_a_payment_is_not_a_message_request_but_a_burn_is() {
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 60_000));
    b.recipients.push((op_return(), 0));
    b.change = Some(39_000);
    let (_, _, kinds) = sign_held(&single_sig(&master(ABANDON, NET), &b));
    assert!(!kinds.contains(&WarningKind::MessageRequest), "{kinds:?}");

    for script_type in [ScriptType::NativeSegwit, ScriptType::Taproot] {
        let mut b = Build::new(script_type, NET);
        b.recipients.push((op_return(), 0));
        let built = single_sig(&master(ABANDON, NET), &b);
        assert_message_request_blocked(&built);

        // The same coin as output 1 of the transaction that funds it.
        let mut inner = built.inner().clone();
        let mut funding = inner.inputs[0].non_witness_utxo.clone().unwrap_or_else(|| {
            crate::common::builder::funding_tx(
                inner.inputs[0]
                    .witness_utxo
                    .as_ref()
                    .unwrap()
                    .script_pubkey
                    .clone(),
                b.input_value,
            )
        });
        funding.output.insert(0, funding.output[0].clone());
        inner.unsigned_tx.input[0].previous_output =
            bitcoin::OutPoint::new(funding.compute_txid(), 1);
        if inner.inputs[0].non_witness_utxo.is_some() {
            inner.inputs[0].non_witness_utxo = Some(funding.clone());
        }
        assert_message_request_blocked(&Psbt::from(inner.clone()));

        // And with ALL|ANYONECANPAY asked.
        inner.inputs[0].sighash_type = Some(bitcoin::psbt::PsbtSighashType::from(
            bitcoin::sighash::EcdsaSighashType::AllPlusAnyoneCanPay,
        ));
        assert_message_request_blocked(&Psbt::from(inner));
    }
}
