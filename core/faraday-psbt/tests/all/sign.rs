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
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
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
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
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
        // sign", and a forced attempt touches nothing.
        assert_eq!(
            sign(
                &c,
                &mut psbt,
                &[&abandon],
                &[abandon.fingerprint()],
                &ctx,
                false,
                Nonce::LowR,
                Aux::Deterministic,
                &mut None,
                &faraday_crypto::Zeroizing::new([0u8; 32]),
            ),
            Err(Error::Danger(vec![WarningKind::NoParticipatingKey]))
        );
        let again = sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
        )
        .unwrap();
        assert!(again.signed_inputs.is_empty() && again.complete);
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
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
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
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
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
    assert_eq!(
        sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
        ),
        Err(Error::KeyMismatch { input: 0 })
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
    assert_eq!(
        sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
        ),
        Err(Error::Unsupported {
            input: 0,
            reason: "no leaf or key path of this taproot input is one of ours"
        })
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
            reason: "non_witness_utxo has no output at the spent index"
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
        true,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        &faraday_crypto::Zeroizing::new([0u8; 32]),
    );
    assert!(signed.is_err(), "{signed:?}");
    assert!(finalize(&mut psbt).is_err());
}

/// A taproot key-path input of a loaded key that carries proprietary
/// records under Faraday's own identifier `osk` is blocked: this build
/// reads no such record, so it neither signs the input nor drops them.
#[test]
fn a_key_path_input_with_osk_records_is_blocked_and_not_signed() {
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
        prefix: b"osk".to_vec(),
        subtype: 0,
        key: vec![2; 66],
    };
    psbt.inner_mut().inputs[0]
        .proprietary
        .insert(key, vec![0; 66]);
    let before = psbt.clone();
    let insp = inspect(&c, &psbt, &ctx);
    assert!(insp.blocked().contains(&WarningKind::UnreadRecords));
    for force in [false, true] {
        let r = sign(
            &c,
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            force,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            &faraday_crypto::Zeroizing::new([0u8; 32]),
        );
        assert!(matches!(r, Err(Error::Blocked(ref b)) if b.contains(&WarningKind::UnreadRecords)));
    }
    assert_eq!(psbt, before, "nothing written");
}
