//! Finalization for the supported script types: build the final
//! scriptSig and witness from the partial signatures, clear the signing
//! fields as BIP-174 requires, and extract the transaction when every
//! input is final.
//!
//! `bitcoin` has no finalizer (that lives in `miniscript`), so this is
//! in-house for single-sig and plain multisig; it is checked against the
//! BIP-174 finalizer and extractor vectors.
//!
//! A miniscript or taproot-tree input is finalized by `miniscript`'s own
//! finalizer ([`miniscript::psbt::PsbtExt::finalize_inp_mut`]), which
//! reads the satisfaction out of the PSBT's own fields and runs the
//! interpreter over the witness it builds. That check is where a
//! timelock the transaction does not meet, or a cosigner's missing
//! signature, stops the input: the finalizer refuses it and the PSBT
//! stays partial, as a multisig waiting on a cosigner does. The crate
//! needs no `std` for any of this.

use alloc::string::ToString;
use alloc::vec::Vec;

use bitcoin::opcodes::OP_0;
use bitcoin::psbt::Input;
use bitcoin::script::{Builder, PushBytesBuf};
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{PublicKey, ScriptBuf, Transaction, Witness, ecdsa};
use miniscript::psbt::PsbtExt;

use crate::classify::{Classified, ScriptKind, Wrapper, classify};
use crate::psbt::{Error, Psbt};

/// Whether every input is final or has enough signatures to be finalized.
pub fn is_complete(psbt: &Psbt) -> bool {
    (0..psbt.inner().inputs.len()).all(|i| {
        let input = &psbt.inner().inputs[i];
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            return true;
        }
        let c = classify(psbt.inner(), i);
        c.problems.is_empty() && build_final(psbt.inner(), i, &c, input).is_ok_and(|f| f.is_some())
    })
}

/// Finalizes every input that has enough signatures. Returns the
/// extracted transaction when all inputs are final, `None` when some are
/// still waiting for signatures.
pub fn finalize(psbt: &mut Psbt) -> Result<Option<Transaction>, Error> {
    let mut all_final = true;
    for i in 0..psbt.inner().inputs.len() {
        let input = &psbt.inner().inputs[i];
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            continue;
        }
        let c = classify(psbt.inner(), i);
        if let Some(problem) = c.problems.first() {
            return Err(Error::Finalize {
                input: i,
                reason: problem,
            });
        }
        if c.utxo.is_none() {
            all_final = false;
            continue;
        }
        match build_final(psbt.inner(), i, &c, input)? {
            Some((script_sig, witness)) => {
                let input = &mut psbt.inner_mut().inputs[i];
                input.final_script_sig = script_sig;
                input.final_script_witness = witness;
                // BIP-174: keep the UTXO and unknown fields, clear the rest.
                input.partial_sigs.clear();
                input.sighash_type = None;
                input.redeem_script = None;
                input.witness_script = None;
                input.bip32_derivation.clear();
                input.tap_key_sig = None;
                input.tap_script_sigs.clear();
                input.tap_scripts.clear();
                input.tap_key_origins.clear();
                input.tap_internal_key = None;
                input.tap_merkle_root = None;
                input.proprietary.clear();
            }
            None => all_final = false,
        }
    }
    if !all_final {
        return Ok(None);
    }
    psbt.inner()
        .clone()
        .extract_tx()
        .map(Some)
        .map_err(|e| Error::Extract(e.to_string()))
}

/// Whether `pk` is the key the input's script pays to (single-sig) or one
/// of the keys in its multisig script.
pub(crate) fn key_in_script(c: &Classified, input: &Input, pk: &PublicKey) -> bool {
    let Some(utxo) = &c.utxo else {
        return false;
    };
    match c.kind {
        ScriptKind::P2pkh => utxo.script_pubkey == ScriptBuf::new_p2pkh(&pk.pubkey_hash()),
        ScriptKind::P2wpkh => pk
            .wpubkey_hash()
            .is_ok_and(|h| utxo.script_pubkey == ScriptBuf::new_p2wpkh(&h)),
        ScriptKind::P2shP2wpkh => pk.wpubkey_hash().is_ok_and(|h| {
            input.redeem_script.as_deref() == Some(ScriptBuf::new_p2wpkh(&h).as_script())
        }),
        ScriptKind::Multisig { .. } => c.multisig.as_ref().is_some_and(|ms| ms.keys.contains(pk)),
        ScriptKind::Miniscript { .. } => c.script_keys.contains(pk),
        ScriptKind::P2trKey | ScriptKind::P2trScript | ScriptKind::Unknown => false,
    }
}

type Final = (Option<ScriptBuf>, Option<Witness>);

/// The final scriptSig and witness for a supported input, `None` when
/// signatures are missing.
fn build_final(
    psbt: &bitcoin::Psbt,
    index: usize,
    c: &Classified,
    input: &Input,
) -> Result<Option<Final>, Error> {
    let index_err = |reason| Error::Finalize { input: 0, reason };
    let single = || {
        input
            .partial_sigs
            .iter()
            .find(|(pk, _)| key_in_script(c, input, pk))
            .map(|(pk, sig)| (*pk, *sig))
    };
    Ok(match c.kind {
        ScriptKind::P2pkh => single().map(|(pk, sig)| {
            let script = Builder::new()
                .push_slice(push(&sig))
                .push_key(&pk)
                .into_script();
            (Some(script), None)
        }),
        ScriptKind::P2wpkh => {
            single().map(|(pk, sig)| (None, Some(Witness::p2wpkh(&sig, &pk.inner))))
        }
        ScriptKind::P2shP2wpkh => match single() {
            Some((pk, sig)) => {
                let redeem = input
                    .redeem_script
                    .as_ref()
                    .expect("classified as p2sh-p2wpkh");
                Some((
                    Some(push_script(redeem).ok_or(index_err("redeem script too long"))?),
                    Some(Witness::p2wpkh(&sig, &pk.inner)),
                ))
            }
            None => None,
        },
        ScriptKind::P2trKey => input
            .tap_key_sig
            .map(|sig| (None, Some(Witness::p2tr_key_spend(&sig)))),
        ScriptKind::Multisig { wrapper, .. } => {
            let ms = c
                .multisig
                .as_ref()
                .expect("multisig kind carries the script");
            let sigs: Vec<ecdsa::Signature> = ms
                .keys
                .iter()
                .filter_map(|k| input.partial_sigs.get(k).copied())
                .take(ms.m)
                .collect();
            if sigs.len() < ms.m {
                return Ok(None);
            }
            match wrapper {
                Wrapper::P2sh => {
                    let redeem = input.redeem_script.as_ref().expect("classified as p2sh");
                    let mut b = Builder::new().push_opcode(OP_0);
                    for sig in &sigs {
                        b = b.push_slice(push(sig));
                    }
                    let script = b
                        .push_slice(
                            PushBytesBuf::try_from(redeem.to_bytes())
                                .map_err(|_| index_err("redeem script too long"))?,
                        )
                        .into_script();
                    Some((Some(script), None))
                }
                Wrapper::P2wsh | Wrapper::P2shP2wsh => {
                    let ws = input.witness_script.as_ref().expect("classified as p2wsh");
                    let mut elements: Vec<Vec<u8>> = Vec::with_capacity(sigs.len() + 2);
                    elements.push(Vec::new());
                    elements.extend(sigs.iter().map(|s| s.to_vec()));
                    elements.push(ws.to_bytes());
                    let witness = Witness::from_slice(&elements);
                    let script_sig = match wrapper {
                        Wrapper::P2shP2wsh => {
                            let redeem = input
                                .redeem_script
                                .as_ref()
                                .expect("classified as p2sh-p2wsh");
                            Some(push_script(redeem).ok_or(index_err("redeem script too long"))?)
                        }
                        _ => None,
                    };
                    Some((script_sig, Some(witness)))
                }
            }
        }
        ScriptKind::Miniscript { .. } | ScriptKind::P2trScript => satisfy(psbt, index),
        ScriptKind::Unknown => return Err(index_err("unknown script type")),
    })
}

/// The scriptSig and witness `miniscript`'s finalizer builds for one
/// input, `None` when the PSBT does not yet satisfy the script.
///
/// The finalizer runs on a copy, so an input it cannot satisfy leaves
/// the PSBT exactly as it was. Its own interpreter check is what makes a
/// refusal mean "not satisfied yet" rather than "wrong": an unmet
/// timelock and a missing cosigner both end here.
fn satisfy(psbt: &bitcoin::Psbt, index: usize) -> Option<Final> {
    let secp = Secp256k1::verification_only();
    let mut attempt = psbt.clone();
    attempt.finalize_inp_mut(&secp, index).ok()?;
    let input = &attempt.inputs[index];
    Some((
        input.final_script_sig.clone(),
        input.final_script_witness.clone(),
    ))
}

fn push(sig: &ecdsa::Signature) -> PushBytesBuf {
    PushBytesBuf::try_from(sig.to_vec()).expect("a signature is far below 520 bytes")
}

/// A scriptSig that pushes `script` as a single element.
fn push_script(script: &ScriptBuf) -> Option<ScriptBuf> {
    let bytes = PushBytesBuf::try_from(script.to_bytes()).ok()?;
    Some(Builder::new().push_slice(bytes).into_script())
}
