//! Single-signature signing for the four script types on regtest: the
//! inspection recognises the input and change, the signature verifies
//! against an independently computed sighash, and the finalized
//! transaction has the right structure.

mod common;

use bitcoin::consensus::{deserialize, serialize};
use bitcoin::secp256k1::{Message, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, Prevouts, SighashCache, TapSighashType};
use bitcoin::{Amount, ScriptBuf, Transaction};
use common::builder::{Build, single_sig};
use common::{ABANDON, ZOO, master};
use osk_bip::keys::{Network, ScriptType};
use osk_psbt::{
    Aux, Context, Error, KeyRef, Nonce, OutputKind, Psbt, ScriptKind, SigKind, WarningKind,
    finalize, inspect, sign,
};

const NET: Network = Network::Regtest;

fn foreign_script() -> ScriptBuf {
    master(ZOO, NET)
        .account_xpub(ScriptType::NativeSegwit, 0)
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

/// Recomputes the sighash for input 0 without going through `osk-psbt`.
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
fn sign_verify_finalize_every_script_type() {
    let secp = Secp256k1::new();
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    for script_type in ScriptType::ALL {
        let mut psbt = build(script_type);
        let unsigned = psbt.clone();

        let insp = inspect(&psbt, &ctx);
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
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
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
                &mut psbt,
                &[&abandon],
                &[abandon.fingerprint()],
                &ctx,
                false,
                Nonce::LowR,
                Aux::Deterministic,
                &mut None,
                [0u8; 32],
                &[],
                &[],
            ),
            Err(Error::Danger(vec![WarningKind::NoParticipatingKey]))
        );
        let again = sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        )
        .unwrap();
        assert!(again.signed_inputs.is_empty() && again.complete);
    }
}

#[test]
fn signing_needs_a_matching_master_key() {
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut psbt = build(ScriptType::NativeSegwit);
    assert_eq!(
        sign(
            &mut psbt,
            &[&zoo],
            &[abandon.fingerprint()],
            &ctx,
            false,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        ),
        Err(Error::NoSuchKey(abandon.fingerprint()))
    );
    // Selecting a key that is loaded but not named by any input signs nothing.
    let r = sign(
        &mut psbt,
        &[&zoo],
        &[zoo.fingerprint()],
        &ctx,
        false,
        Nonce::LowR,
        Aux::Deterministic,
        &mut None,
        [0u8; 32],
        &[],
        &[],
    )
    .unwrap();
    assert!(r.signed_inputs.is_empty() && !r.complete);
    assert_eq!(finalize(&mut psbt).unwrap(), None);
}

#[test]
fn key_origin_that_does_not_match_the_derived_key_is_refused() {
    let abandon = master(ABANDON, NET);
    let zoo = master(ZOO, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    // Zoo's PSBT with the origin relabelled to abandon's fingerprint.
    let mut b = Build::new(ScriptType::NativeSegwit, NET);
    b.recipients.push((foreign_script(), 90_000));
    let mut psbt = single_sig(&zoo, &b);
    let input = &mut psbt.inner_mut().inputs[0];
    for (fp, _) in input.bip32_derivation.values_mut() {
        *fp = abandon.fingerprint().into();
    }
    let insp = inspect(&psbt, &ctx);
    assert!(!insp.inputs[0].is_ours);
    assert_eq!(
        sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
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
    let abandon = master(ABANDON, NET);
    let keys = [KeyRef::from_master(&abandon, 0).unwrap()];
    let ctx = Context {
        network: NET,
        keys: &keys,
        wallets: &[],
        musig_session: None,
        shares: &[],
        carry: None,
    };
    let mut psbt = build(ScriptType::Taproot);
    psbt.inner_mut().inputs[0].tap_merkle_root = Some(bitcoin::TapNodeHash::from_script(
        bitcoin::Script::new(),
        bitcoin::taproot::LeafVersion::TapScript,
    ));
    let insp = inspect(&psbt, &ctx);
    assert_eq!(insp.inputs[0].script_type, ScriptKind::P2trScript);
    assert_eq!(
        sign(
            &mut psbt,
            &[&abandon],
            &[abandon.fingerprint()],
            &ctx,
            true,
            Nonce::LowR,
            Aux::Deterministic,
            &mut None,
            [0u8; 32],
            &[],
            &[],
        ),
        Err(Error::Unsupported {
            input: 0,
            reason: "no leaf or key path of this taproot input is one of ours"
        })
    );
}
