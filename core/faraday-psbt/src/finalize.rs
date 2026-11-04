//! Finalization for the supported script types: build the final
//! scriptSig and witness from the partial signatures, clear the signing
//! fields as BIP-174 requires, and extract the transaction when every
//! input is final.
//!
//! `bitcoin` has no finalizer (that lives in `miniscript`); it is checked
//! against the BIP-174 finalizer and extractor vectors.
//!
//! Every input of a supported script (pk-hash and witness pk-hash, plain
//! and `sh`-wrapped, multisig under `sh`, `wsh` and `sh(wsh)`, miniscript,
//! taproot) is finalized by `miniscript`'s own finalizer ([`miniscript::psbt::PsbtExt::finalize_inp_mut`]), which
//! reads the satisfaction out of the PSBT's own fields and runs the
//! interpreter over the witness it builds. That check is where a
//! timelock the transaction does not meet, or a cosigner's missing
//! signature, stops the input: the finalizer refuses it and the PSBT
//! stays partial, as a multisig waiting on a cosigner does. The crate
//! needs no `std` for any of this.
//!
//! For a `wsh` or `sh(wsh)` miniscript, which of several satisfactions
//! the witness carries is then chosen again by the real size of its
//! signatures ([`smallest`]; a `multi` inside it keeps its k smallest).
//! A plain multisig, under any wrapper, carries the signatures of the
//! first m keys that signed, as Bitcoin Core writes it. Each result,
//! where it differs from `miniscript`'s, goes through the same
//! interpreter check.
//!
//! For a taproot script path, which leaf the witness spends is chosen
//! by the real size of the witness each leaf gives ([`by_leaf`]).

mod smallest;

use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use bitcoin::opcodes::OP_0;
use bitcoin::psbt::Input;
use bitcoin::script::{Builder, PushBytesBuf};
use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::Prevouts;
use bitcoin::taproot::ControlBlock;
use bitcoin::{PublicKey, ScriptBuf, Transaction, TxOut, Witness};
use miniscript::psbt::PsbtExt;
use miniscript::{ExtParams, Interpreter, Miniscript, Segwitv0, Tap, Terminal};

use crate::classify::{Classified, ScriptKind, Wrapper, classify, spent_output};
use crate::psbt::{Error, Psbt};
use crate::reason::Reason;

/// Whether every input is final or has enough signatures to be finalized.
pub fn is_complete(psbt: &Psbt) -> bool {
    if prevout_missing(psbt.inner()).is_some() {
        return false;
    }
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
    // `miniscript`'s finalizer reads the spent output of every input
    // before it finalizes one, and `bitcoin`'s extractor does the same
    // for the fee; both index a `non_witness_utxo` unchecked and panic
    // on a missing output. So no input is touched until every one is
    // known to have its spent output.
    if let Some(input) = prevout_missing(psbt.inner()) {
        return Err(Error::Finalize {
            input,
            reason: Reason::PrevTxNoOutput,
        });
    }
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
                reason: *problem,
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
                // BIP-174 keeps the UTXO and unknown fields and clears the
                // rest; proprietary fields are cleared too, which it does not
                // require keeping.
                clear_signing_fields(input);
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

/// Clears what a finalized input no longer needs.
pub(crate) fn clear_signing_fields(input: &mut Input) {
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
    input.ripemd160_preimages.clear();
    input.sha256_preimages.clear();
    input.hash160_preimages.clear();
    input.hash256_preimages.clear();
}

/// The first input that states no `witness_utxo` and whose
/// `non_witness_utxo` lacks the output spent, `None` when there is none.
fn prevout_missing(inner: &bitcoin::Psbt) -> Option<usize> {
    (0..inner.inputs.len())
        .find(|&i| inner.inputs[i].non_witness_utxo.is_some() && spent_output(inner, i).is_none())
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
    let index_err = |reason| Error::Finalize {
        input: index,
        reason,
    };
    Ok(match c.kind {
        ScriptKind::P2pkh | ScriptKind::P2wpkh => satisfy(psbt, index),
        ScriptKind::P2shP2wpkh => {
            let redeem = input
                .redeem_script
                .as_ref()
                .expect("classified as p2sh-p2wpkh");
            push_script(redeem).ok_or(index_err(Reason::RedeemScriptTooLong))?;
            satisfy(psbt, index)
        }
        ScriptKind::P2trKey => input.tap_key_sig.and_then(|sig| {
            let found = (None, Some(Witness::p2tr_key_spend(&sig)));
            // The key-path signature is checked like any witness.
            let utxo = c.utxo.as_ref()?;
            accepted(psbt, index, utxo, &found).then_some(found)
        }),
        ScriptKind::Multisig { wrapper, .. } => {
            let redeem = input.redeem_script.as_ref();
            if matches!(wrapper, Wrapper::P2sh | Wrapper::P2shP2wsh)
                && redeem.is_some_and(|r| push_script(r).is_none())
            {
                return Err(index_err(Reason::RedeemScriptTooLong));
            }
            match satisfy(psbt, index) {
                Some(chosen) => Some(by_size(psbt, index, c, chosen)),
                // `miniscript` 12.3.7 keeps the first m signers of a plain
                // multisig in script order, as Core does, so this finds
                // nothing it did not; it guards a later `miniscript` that keeps
                // the m smallest, whose check would fail on an invalid one
                // among them: the first m signers' are then tried alone.
                None => c.utxo.as_ref().and_then(|utxo| {
                    let found = first_signers(c, input, wrapper, &(None, None))?;
                    accepted(psbt, index, utxo, &found).then_some(found)
                }),
            }
        }
        ScriptKind::Miniscript { .. } => {
            satisfy(psbt, index).map(|chosen| by_size(psbt, index, c, chosen))
        }
        ScriptKind::P2trScript => by_leaf(psbt, index),
        ScriptKind::Unknown => return Err(index_err(Reason::UnknownScript)),
    })
}

/// The scriptSig and witness `miniscript`'s finalizer builds for one
/// input, `None` when the PSBT does not yet satisfy the script.
///
/// The finalizer runs on a copy, so an input it cannot satisfy leaves
/// the PSBT exactly as it was. Its own interpreter check is what makes a
/// refusal mean "not satisfied yet" rather than "wrong": an unmet
/// timelock and a missing cosigner both end here.
///
/// A taproot leaf with a `pkh` is left out of the attempt. `miniscript`
/// 12 fills a `pkh`'s key in a satisfaction from the input's ECDSA key
/// origins whatever the context, so for a taproot leaf it finds no key
/// and panics on its own `expect`, after a signature for that leaf was
/// written; and the key it would push is 33 bytes where the leaf hashed
/// 32. Such a leaf is signed for and left for the coordinator to
/// finalize; any other leaf, or the key path, still finalizes here.
fn satisfy(psbt: &bitcoin::Psbt, index: usize) -> Option<Final> {
    satisfy_leaves(psbt, index, |_| true)
}

/// [`satisfy`] with only the taproot leaves `keep` accepts, by control
/// block, left in the input.
fn satisfy_leaves(
    psbt: &bitcoin::Psbt,
    index: usize,
    keep: impl Fn(&ControlBlock) -> bool,
) -> Option<Final> {
    let secp = Secp256k1::verification_only();
    let mut attempt = psbt.clone();
    attempt.inputs[index]
        .tap_scripts
        .retain(|control, (script, _)| keep(control) && !leaf_has_pkh(script));
    attempt.finalize_inp_mut(&secp, index).ok()?;
    let input = &attempt.inputs[index];
    Some((
        input.final_script_sig.clone(),
        input.final_script_witness.clone(),
    ))
}

/// Costs one signature check per signature of every leaf the PSBT
/// carries (the interpreter's, once per leaf): a taproot script-path
/// input finalized with the leaf whose witness is smallest as written.
///
/// `miniscript` 12 chooses among leaves by a size that leaves out each
/// element's length byte, so on a tie it can keep the leaf that is one
/// byte heavier as serialized. Each leaf is finalized alone, so each
/// witness is one `miniscript`'s finalizer built and its interpreter
/// accepted, and the one of least serialized size is kept: its element
/// count, and every element with its length, plus the script and
/// control block. Where two weigh the same, the leaf kept is the one
/// Bitcoin Core keeps (F161): Core tries the leaves in the
/// order of their script's bytes, then their leaf version, each with
/// its shortest control block (then the smaller by bytes), and takes a
/// later leaf only when it is strictly smaller. What can be finalized
/// at all is unchanged: an input no leaf satisfies stays partial.
fn by_leaf(psbt: &bitcoin::Psbt, index: usize) -> Option<Final> {
    let mut controls: Vec<ControlBlock> = psbt.inputs[index].tap_scripts.keys().cloned().collect();
    if controls.is_empty() {
        return satisfy(psbt, index);
    }
    let scripts = &psbt.inputs[index].tap_scripts;
    controls.sort_by_cached_key(|control| {
        let (script, version) = &scripts[control];
        let control = control.serialize();
        (
            script.to_bytes(),
            version.to_consensus(),
            control.len(),
            control,
        )
    });
    let mut best: Option<(usize, Final)> = None;
    for control in &controls {
        let Some(found) = satisfy_leaves(psbt, index, |c| c == control) else {
            continue;
        };
        let size = witness_size(found.1.as_ref());
        if best.as_ref().is_none_or(|(least, _)| size < *least) {
            best = Some((size, found));
        }
    }
    best.map(|(_, found)| found)
}

/// A witness as a transaction writes it: the element count, then each
/// element with its length.
fn witness_size(witness: Option<&Witness>) -> usize {
    let Some(witness) = witness else {
        return usize::MAX;
    };
    compact_size(witness.len())
        + witness
            .iter()
            .map(|e| compact_size(e.len()) + e.len())
            .sum::<usize>()
}

/// The bytes a compact-size integer takes.
fn compact_size(n: usize) -> usize {
    match n {
        0..=0xfc => 1,
        0xfd..=0xffff => 3,
        0x10000..=0xffff_ffff => 5,
        _ => 9,
    }
}

/// Costs one signature check per signature when the final fields change
/// (the interpreter's): the signatures of the final fields as Bitcoin
/// Core's `finalizepsbt` chooses them, in place of the ones `miniscript`
/// built, `chosen`: a `wsh` or `sh(wsh)` miniscript's by [`smallest`]; a
/// plain multisig's, under any wrapper, those of the first m keys that
/// signed (Core does not choose among a multisig's signatures by size).
///
/// Only the signatures change. The new fields are taken only if
/// `miniscript`'s interpreter accepts them for this input; otherwise, as
/// when the choice is the same, `chosen` stands, so which inputs are
/// finalized is `miniscript`'s answer alone.
fn by_size(psbt: &bitcoin::Psbt, index: usize, c: &Classified, chosen: Final) -> Final {
    let Some(utxo) = &c.utxo else {
        return chosen;
    };
    let input = &psbt.inputs[index];
    let candidate = match (&c.kind, &input.witness_script, &chosen.1) {
        (ScriptKind::Multisig { wrapper, .. }, _, _) => first_signers(c, input, *wrapper, &chosen),
        (_, Some(ws), Some(witness)) => by_size_witness(psbt, index, ws, witness)
            .map(|candidate| (chosen.0.clone(), Some(candidate))),
        _ => None,
    };
    match candidate {
        Some(found) if found != chosen && accepted(psbt, index, utxo, &found) => found,
        _ => chosen,
    }
}

/// The witness for a `wsh` miniscript input with [`smallest`]'s
/// signatures, `None` where it is the one `miniscript` built.
fn by_size_witness(
    psbt: &bitcoin::Psbt,
    index: usize,
    ws: &ScriptBuf,
    witness: &Witness,
) -> Option<Witness> {
    let ms = Miniscript::<PublicKey, Segwitv0>::parse_with_ext(ws, &ExtParams::allow_all()).ok()?;
    let mut stack = smallest::stack(psbt, index, &ms)?;
    let elements: Vec<Vec<u8>> = witness.iter().map(<[u8]>::to_vec).collect();
    if elements.last() != Some(&ws.to_bytes()) || elements[..elements.len() - 1] == stack[..] {
        return None;
    }
    stack.push(ws.to_bytes());
    Some(Witness::from_slice(&stack))
}

/// Costs one signature check per signature (the interpreter's, in
/// [`accepted`]): a plain multisig input's final fields with the
/// signatures of the first m keys that signed, in key order, as Bitcoin
/// Core writes them; `chosen` supplies the scriptSig of a `wsh` wrapper.
fn first_signers(c: &Classified, input: &Input, wrapper: Wrapper, chosen: &Final) -> Option<Final> {
    let ms = c.multisig.as_ref()?;
    let sigs: Vec<Vec<u8>> = ms
        .keys
        .iter()
        .filter_map(|k| input.partial_sigs.get(k).map(|s| s.to_vec()))
        .take(ms.m)
        .collect();
    if sigs.len() < ms.m {
        return None;
    }
    match wrapper {
        Wrapper::P2sh => {
            let redeem = input.redeem_script.as_ref()?;
            let mut b = Builder::new().push_opcode(OP_0);
            for sig in sigs {
                b = b.push_slice(PushBytesBuf::try_from(sig).ok()?);
            }
            let script_sig = b
                .push_slice(PushBytesBuf::try_from(redeem.to_bytes()).ok()?)
                .into_script();
            Some((Some(script_sig), None))
        }
        Wrapper::P2wsh | Wrapper::P2shP2wsh => {
            let ws = input.witness_script.as_ref()?;
            let mut elements: Vec<Vec<u8>> = vec![Vec::new()];
            elements.extend(sigs);
            elements.push(ws.to_bytes());
            let script_sig = match wrapper {
                Wrapper::P2shP2wsh => Some(push_script(input.redeem_script.as_ref()?)?),
                _ => chosen.0.clone(),
            };
            Some((script_sig, Some(Witness::from_slice(&elements))))
        }
    }
}

/// Costs one signature check per signature of the final fields: whether
/// `miniscript`'s interpreter accepts them for the input.
fn accepted(psbt: &bitcoin::Psbt, index: usize, utxo: &TxOut, fields: &Final) -> bool {
    let empty = ScriptBuf::new();
    let sig = fields.0.as_ref().unwrap_or(&empty);
    let witness = fields.1.clone().unwrap_or_default();
    let tx = &psbt.unsigned_tx;
    let Ok(interpreter) = Interpreter::from_txdata(
        &utxo.script_pubkey,
        sig,
        &witness,
        tx.input[index].sequence,
        tx.lock_time,
    ) else {
        return false;
    };
    let secp = Secp256k1::verification_only();
    // Taproot signs every spent output; the others only their own.
    let all: Option<Vec<&TxOut>> = (0..tx.input.len()).map(|i| spent_output(psbt, i)).collect();
    let prevouts = match &all {
        Some(outs) => Prevouts::All(outs),
        None => Prevouts::One(index, utxo),
    };
    interpreter
        .iter(&secp, tx, index, &prevouts)
        .all(|step| step.is_ok())
}

/// Whether a taproot leaf, read as a miniscript, checks a key by its
/// hash (`pkh`, or a raw `pkh` read from the script).
fn leaf_has_pkh(script: &ScriptBuf) -> bool {
    let Ok(ms) = Miniscript::<XOnlyPublicKey, Tap>::parse_with_ext(script, &ExtParams::allow_all())
    else {
        return false;
    };
    ms.iter()
        .any(|node| matches!(node.node, Terminal::PkH(_) | Terminal::RawPkH(_)))
}

/// A scriptSig that pushes `script` as a single element.
fn push_script(script: &ScriptBuf) -> Option<ScriptBuf> {
    let bytes = PushBytesBuf::try_from(script.to_bytes()).ok()?;
    Some(Builder::new().push_slice(bytes).into_script())
}
