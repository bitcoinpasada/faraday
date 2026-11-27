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

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use bitcoin::hashes::Hash;
use bitcoin::opcodes::OP_0;
use bitcoin::psbt::Input;
use bitcoin::script::{Builder, PushBytesBuf};
use bitcoin::secp256k1::{Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::Prevouts;
use bitcoin::taproot::ControlBlock;
use bitcoin::{PublicKey, ScriptBuf, Transaction, TxOut, Witness};
use miniscript::interpreter::SatisfiedConstraint;
use miniscript::psbt::PsbtExt;
use miniscript::{ExtParams, Interpreter, Legacy, Miniscript, Segwitv0, Tap, Terminal};

use crate::classify::{Classified, ScriptKind, Wrapper, classify, spent_output};
use crate::psbt::{Error, Psbt};
use crate::reason::Reason;

/// Whether every input is final or has enough signatures to be finalized,
/// and none states a false output (a final one included).
pub fn is_complete(psbt: &Psbt) -> bool {
    if prevout_missing(psbt.inner()).is_some() {
        return false;
    }
    let mut work = psbt.inner().clone();
    (0..work.inputs.len()).all(|i| {
        let c = classify(&work, i);
        if !c.problems.is_empty() {
            return false;
        }
        let input = &work.inputs[i];
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            return true;
        }
        build_final(&mut work, i, &c).is_ok_and(|f| f.is_some())
    })
}

/// Costs a classify, a copy of the whole PSBT and the finalizer's work on
/// one input, as [`finalize`] does it. Whether input `index` of `psbt`, taken as it stands, has enough
/// signatures to be finalized, whatever any other input needs: no input
/// misses its spent output, the input states nothing false and a final
/// witness is built for it. An input that is final already, or that the
/// finalizer refuses, is not.
pub fn input_finishes(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    if prevout_missing(psbt).is_some()
        || input.final_script_sig.is_some()
        || input.final_script_witness.is_some()
    {
        return false;
    }
    let c = classify(psbt, index);
    c.problems.is_empty()
        && c.utxo.is_some()
        && build_final(&mut psbt.clone(), index, &c).is_ok_and(|f| f.is_some())
}

/// Finalizes every input that has enough signatures, and refuses an input
/// that states a false output, final or not. Returns the
/// extracted transaction when all inputs are final, `None` when some are
/// still waiting for signatures.
pub fn finalize(psbt: &mut Psbt) -> Result<Option<Transaction>, Error> {
    // `miniscript`'s finalizer reads the spent output of every input
    // before it finalizes one, and `bitcoin`'s extractor does the same
    // for the fee; both index a `non_witness_utxo` unchecked and panic
    // on a missing output. So no input is touched until every one is
    // known to have its spent output.
    if let Some(input) = prevout_missing(psbt.inner()) {
        // The reason is what `classify` found (a previous transaction of
        // another txid, or without the output).
        let reason = classify(psbt.inner(), input)
            .problems
            .first()
            .copied()
            .unwrap_or(Reason::PrevTxNoOutput);
        return Err(Error::Finalize { input, reason });
    }
    let mut all_final = true;
    // One working copy for the pass: the satisfier puts an input's fields
    // in it and takes them out, so no input copies the whole file.
    let mut work = psbt.inner().clone();
    for i in 0..psbt.inner().inputs.len() {
        let input = &psbt.inner().inputs[i];
        let c = classify(psbt.inner(), i);
        // An input that states a false output is refused final or not.
        if let Some(problem) = c.problems.first() {
            return Err(Error::Finalize {
                input: i,
                reason: *problem,
            });
        }
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            continue;
        }
        if c.utxo.is_none() {
            all_final = false;
            continue;
        }
        match build_final(&mut work, i, &c)? {
            Some((script_sig, witness)) => {
                let input = &mut psbt.inner_mut().inputs[i];
                input.final_script_sig = script_sig;
                input.final_script_witness = witness;
                // BIP-174 keeps the UTXO and unknown fields and clears the
                // rest; proprietary fields are cleared too, which it does not
                // require keeping.
                clear_signing_fields(input);
                work.inputs[i] = input.clone();
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

/// Costs a txid per input with a previous transaction. The first input
/// that states no `witness_utxo` and whose `non_witness_utxo` lacks the
/// output at the spent index, whatever its txid: `bitcoin`'s extractor
/// and finalizer index it unchecked and would panic. `None` when there is
/// none.
fn prevout_missing(inner: &bitcoin::Psbt) -> Option<usize> {
    (0..inner.inputs.len()).find(|&i| {
        let input = &inner.inputs[i];
        let vout = inner.unsigned_tx.input[i].previous_output.vout as usize;
        input.witness_utxo.is_none()
            && input
                .non_witness_utxo
                .as_ref()
                .is_some_and(|tx| tx.output.get(vout).is_none())
    })
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
    psbt: &mut bitcoin::Psbt,
    index: usize,
    c: &Classified,
) -> Result<Option<Final>, Error> {
    let index_err = |reason| Error::Finalize {
        input: index,
        reason,
    };
    Ok(match c.kind {
        ScriptKind::P2pkh | ScriptKind::P2wpkh => satisfy_key_hash(psbt, index),
        ScriptKind::P2shP2wpkh => {
            let redeem = psbt.inputs[index]
                .redeem_script
                .as_ref()
                .expect("classified as p2sh-p2wpkh");
            push_script(redeem).ok_or(index_err(Reason::RedeemScriptTooLong))?;
            satisfy_key_hash(psbt, index)
        }
        ScriptKind::P2trKey => psbt.inputs[index].tap_key_sig.and_then(|sig| {
            let found = (None, Some(Witness::p2tr_key_spend(&sig)));
            // The key-path signature is checked like any witness.
            let utxo = c.utxo.as_ref()?;
            accepted(psbt, index, utxo, &found).then_some(found)
        }),
        ScriptKind::Multisig { wrapper, .. } => {
            let redeem = psbt.inputs[index].redeem_script.as_ref();
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
                    let found = first_signers(c, &psbt.inputs[index], wrapper, &(None, None))?;
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
fn satisfy(psbt: &mut bitcoin::Psbt, index: usize) -> Option<Final> {
    satisfy_leaves(psbt, index, |_| true)
}

/// [`satisfy`] for a key hash output (p2pkh, p2wpkh, p2sh-p2wpkh), whose
/// descriptor `miniscript` builds from the input's own partial signatures
/// and from no other input's key origins. `miniscript` hashes every
/// input's key origins on each call to make a map only a script with a
/// raw key hash reads, so the other inputs' are set aside for the call
/// and put back: the answer is the same and the call no longer reads the
/// whole file's keys.
fn satisfy_key_hash(psbt: &mut bitcoin::Psbt, index: usize) -> Option<Final> {
    let aside: Vec<_> = psbt
        .inputs
        .iter_mut()
        .enumerate()
        .filter(|(i, input)| *i != index && !input.bip32_derivation.is_empty())
        .map(|(i, input)| (i, core::mem::take(&mut input.bip32_derivation)))
        .collect();
    let found = satisfy(psbt, index);
    for (i, origins) in aside {
        psbt.inputs[i].bip32_derivation = origins;
    }
    found
}

/// [`satisfy`] with only the taproot leaves `keep` accepts, by control
/// block, left in the input.
fn satisfy_leaves(
    psbt: &mut bitcoin::Psbt,
    index: usize,
    keep: impl Fn(&ControlBlock) -> bool,
) -> Option<Final> {
    let secp = Secp256k1::verification_only();
    // The attempt is made on the input itself and the input put back, so
    // an input costs its own size and not the file's.
    let saved = psbt.inputs[index].clone();
    psbt.inputs[index]
        .tap_scripts
        .retain(|control, (script, _)| keep(control) && !leaf_has_pkh(script));
    let done = psbt.finalize_inp_mut(&secp, index).ok().map(|()| {
        let input = &psbt.inputs[index];
        (
            input.final_script_sig.clone(),
            input.final_script_witness.clone(),
        )
    });
    psbt.inputs[index] = saved;
    done
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
fn by_leaf(psbt: &mut bitcoin::Psbt, index: usize) -> Option<Final> {
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

/// What `miniscript`'s interpreter says of an input's final fields.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Judgement {
    /// The script the input spends parses as a miniscript, and every step
    /// of the interpreter passes.
    Accepts,
    /// It parses, and the interpreter refuses the fields.
    Rejects,
    /// The script is not a miniscript (a puzzle, an anchor, a raw script),
    /// or the output has no script to read: nothing is judged.
    NotJudged,
}

/// The script an input's final fields spend, where one is judged, and
/// whether it is a witness script: the witness script of p2wsh or
/// p2sh-p2wsh, the redeem script of p2sh, or the bare output. The key
/// hash forms (p2pkh, p2wpkh) and taproot have none.
fn spent_script(
    spk: &bitcoin::Script,
    script_sig: &ScriptBuf,
    witness: &Witness,
) -> Option<(bool, ScriptBuf)> {
    let from_witness = || witness.last().map(|w| (true, ScriptBuf::from(w.to_vec())));
    if spk.is_p2wsh() {
        return from_witness();
    }
    if spk.is_p2sh() {
        let redeem = script_sig
            .instructions()
            .filter_map(|i| match i {
                Ok(bitcoin::script::Instruction::PushBytes(b)) => Some(b.as_bytes().to_vec()),
                _ => None,
            })
            .last()?;
        let redeem = ScriptBuf::from(redeem);
        if redeem.is_p2wsh() {
            return from_witness();
        }
        return (!redeem.is_witness_program()).then_some((false, redeem));
    }
    (!spk.is_witness_program() && !spk.is_p2pkh())
        .then(|| (false, ScriptBuf::from(spk.as_bytes().to_vec())))
}

/// Whether `script` parses as a miniscript in its context, as
/// `finalize` reads it.
pub(crate) fn parses(witness: bool, script: &bitcoin::Script) -> bool {
    if witness {
        Miniscript::<PublicKey, Segwitv0>::parse_with_ext(script, &ExtParams::allow_all()).is_ok()
    } else {
        Miniscript::<PublicKey, Legacy>::parse_with_ext(script, &ExtParams::allow_all()).is_ok()
    }
}

/// The parts of a final taproot script-path witness (BIP-341): the stack
/// below the leaf, the leaf script, the control block, and the annex (a
/// last element of two or more that starts 0x50).
pub(crate) struct TapSpend<'a> {
    pub stack: Vec<&'a [u8]>,
    pub leaf: ScriptBuf,
    pub control: &'a [u8],
    pub annex: Option<&'a [u8]>,
}

impl TapSpend<'_> {
    /// Whether the leaf is one the interpreter runs: leaf version 0xc0
    /// (another version is not defined, and consensus accepts it as it
    /// stands), a Tap miniscript, and not a `pkh` leaf holding a key of an
    /// unknown type.
    pub(crate) fn is_run(&self) -> bool {
        self.control.first().is_none_or(|b| b & 0xfe == 0xc0)
            && tap_miniscript(&self.leaf)
            && !self.pkh_of_unknown_key_type()
    }

    /// Whether the leaf is `DUP HASH160 <h> EQUALVERIFY CHECKSIG`, spent by
    /// `[signature, key]` with `h` the hash of a key that is neither empty
    /// nor 32 bytes and a signature that is not empty. BIP-342 gives such a
    /// key an unknown type, and any non-empty signature passes it, whatever
    /// the bytes of either: the interpreter reads only 32-byte keys, so the
    /// leaf is left unrun rather than refused. With an empty signature or
    /// key, a hash the key does not match, or an element over 520 bytes,
    /// the leaf fails and is run.
    fn pkh_of_unknown_key_type(&self) -> bool {
        use bitcoin::opcodes::all::{OP_CHECKSIG, OP_DUP, OP_EQUALVERIFY, OP_HASH160};
        use bitcoin::script::Instruction;
        let [sig, key] = self.stack[..] else {
            return false;
        };
        if sig.is_empty() || key.is_empty() || key.len() == 32 {
            return false;
        }
        // An element over 520 bytes is refused by the interpreter
        // (consensus's push size limit), so the leaf is run and fails.
        if sig.len() > 520 || key.len() > 520 {
            return false;
        }
        let mut it = self.leaf.instructions();
        let (Some(Ok(Instruction::Op(OP_DUP))), Some(Ok(Instruction::Op(OP_HASH160)))) =
            (it.next(), it.next())
        else {
            return false;
        };
        let Some(Ok(Instruction::PushBytes(h))) = it.next() else {
            return false;
        };
        matches!(
            (it.next(), it.next(), it.next()),
            (
                Some(Ok(Instruction::Op(OP_EQUALVERIFY))),
                Some(Ok(Instruction::Op(OP_CHECKSIG))),
                None
            )
        ) && h.as_bytes() == bitcoin::hashes::hash160::Hash::hash(key).to_byte_array()
    }
}

/// The parts of a final taproot script-path witness; `None` for a
/// key-path spend or an empty witness.
pub(crate) fn tap_spend(witness: &Witness) -> Option<TapSpend<'_>> {
    let mut elements: Vec<&[u8]> = witness.iter().collect();
    let annexed = elements.len() >= 2 && elements.last().is_some_and(|e| e.first() == Some(&0x50));
    let annex = if annexed { elements.pop() } else { None };
    if elements.len() < 2 {
        return None;
    }
    let control = elements.pop()?;
    let leaf = ScriptBuf::from(elements.pop()?.to_vec());
    Some(TapSpend {
        stack: elements,
        leaf,
        control,
        annex,
    })
}

/// Whether a taproot leaf parses as a Tap miniscript, which the
/// interpreter runs.
pub(crate) fn tap_miniscript(script: &bitcoin::Script) -> bool {
    Miniscript::<XOnlyPublicKey, Tap>::parse_with_ext(script, &ExtParams::allow_all()).is_ok()
}

/// Whether a script holds `OP_CODESEPARATOR`, whose message is not worked
/// out here.
pub(crate) fn has_codeseparator(script: &bitcoin::Script) -> bool {
    script.instructions().any(|i| {
        matches!(i, Ok(bitcoin::script::Instruction::Op(op)) if op == bitcoin::opcodes::all::OP_CODESEPARATOR)
    })
}

/// Whether the script the final fields spend is one `miniscript` reads.
pub(crate) fn judged(spk: &bitcoin::Script, script_sig: &ScriptBuf, witness: &Witness) -> bool {
    spent_script(spk, script_sig, witness).is_some_and(|(w, s)| parses(w, &s))
}

/// Costs one signature check per signature the script asks for: the
/// interpreter's answer on input `index`'s final fields, for a final
/// input: a taproot script-path spend of a Tap miniscript leaf is run with
/// the annex-aware signature check ([`accepts_tap`]), a taproot key-path
/// spend is not judged here. The script must be within consensus's limits
/// (the caller checks them first).
pub(crate) fn judge(psbt: &bitcoin::Psbt, index: usize, utxo: &TxOut) -> Judgement {
    let input = &psbt.inputs[index];
    let empty = ScriptBuf::new();
    let sig = input.final_script_sig.as_ref().unwrap_or(&empty);
    let witness = input.final_script_witness.clone().unwrap_or_default();
    if utxo.script_pubkey.is_p2tr() {
        let Some(tap) = tap_spend(&witness) else {
            return Judgement::NotJudged;
        };
        if !tap.is_run() || has_codeseparator(&tap.leaf) {
            return Judgement::NotJudged;
        }
        // A 65-byte stack element ending 0x00 is a signature BIP-341
        // refuses, or no data a miniscript takes.
        if tap
            .stack
            .iter()
            .any(|e| crate::v2::is_zero_sighash_schnorr(e))
        {
            return Judgement::Rejects;
        }
        return accepts_tap(psbt, index, utxo, &tap);
    }
    if !judged(&utxo.script_pubkey, sig, &witness) {
        return Judgement::NotJudged;
    }
    if accepts(psbt, index, utxo, sig, &witness) {
        Judgement::Accepts
    } else {
        Judgement::Rejects
    }
}

/// Costs one signature check per signature the leaf asks for: the
/// interpreter's answer on a taproot script-path spend, with each
/// signature read over the message BIP-341 gives it, the annex included.
/// Not judged where a signature commits to the other inputs' outputs and
/// one of them is not known.
fn accepts_tap(psbt: &bitcoin::Psbt, index: usize, utxo: &TxOut, tap: &TapSpend) -> Judgement {
    use bitcoin::sighash::{Annex, SighashCache, TapSighashType};
    use bitcoin::taproot::{LeafVersion, TapLeafHash};
    use core::cell::Cell;
    let tx = &psbt.unsigned_tx;
    // The interpreter takes no annex: it runs the witness without it.
    let mut elements: Vec<&[u8]> = tap.stack.clone();
    elements.push(tap.leaf.as_bytes());
    elements.push(tap.control);
    let witness = Witness::from_slice(&elements);
    let script_sig = ScriptBuf::new();
    let Ok(interpreter) = Interpreter::from_txdata(
        &utxo.script_pubkey,
        &script_sig,
        &witness,
        tx.input[index].sequence,
        tx.lock_time,
    ) else {
        return Judgement::Rejects;
    };
    let annex = match tap.annex.map(Annex::new) {
        Some(Ok(annex)) => Some(annex),
        Some(Err(_)) => return Judgement::Rejects,
        None => None,
    };
    let all: Option<Vec<&TxOut>> = if utxo.script_pubkey.is_p2tr() {
        (0..tx.input.len()).map(|i| spent_output(psbt, i)).collect()
    } else {
        None
    };
    let prevouts = match &all {
        Some(outs) => Prevouts::All(outs),
        None => Prevouts::One(index, utxo),
    };
    let leaf_hash = TapLeafHash::from_script(&tap.leaf, LeafVersion::TapScript);
    let unknown = Cell::new(false);
    let secp = Secp256k1::verification_only();
    let verify = |pair: &miniscript::interpreter::KeySigPair| {
        let miniscript::interpreter::KeySigPair::Schnorr(key, sig) = pair else {
            return false;
        };
        let anyone = matches!(
            sig.sighash_type,
            TapSighashType::AllPlusAnyoneCanPay
                | TapSighashType::NonePlusAnyoneCanPay
                | TapSighashType::SinglePlusAnyoneCanPay
        );
        if all.is_none() && !anyone {
            unknown.set(true);
            return false;
        }
        let Ok(hash) = SighashCache::new(tx).taproot_signature_hash(
            index,
            &prevouts,
            annex.clone(),
            Some((leaf_hash, 0xFFFF_FFFF)),
            sig.sighash_type,
        ) else {
            return false;
        };
        let msg = bitcoin::secp256k1::Message::from_digest(hash.to_byte_array());
        secp.verify_schnorr(&sig.signature, &msg, key).is_ok()
    };
    let sequence_final = tx.input[index].sequence == bitcoin::Sequence::MAX;
    let version_two = tx.version.0 as u32 >= 2;
    let accepted = interpreter
        .iter_custom(Box::new(verify))
        .all(|step| match step {
            Ok(SatisfiedConstraint::AbsoluteTimelock { .. }) => !sequence_final,
            Ok(SatisfiedConstraint::RelativeTimelock { .. }) => version_two,
            Ok(_) => true,
            Err(_) => false,
        });
    if accepted {
        Judgement::Accepts
    } else if unknown.get() {
        Judgement::NotJudged
    } else {
        Judgement::Rejects
    }
}

/// Costs one signature check per signature of the final fields: whether
/// `miniscript`'s interpreter accepts them for the input.
fn accepted(psbt: &bitcoin::Psbt, index: usize, utxo: &TxOut, fields: &Final) -> bool {
    let empty = ScriptBuf::new();
    let sig = fields.0.as_ref().unwrap_or(&empty);
    let witness = fields.1.clone().unwrap_or_default();
    accepts(psbt, index, utxo, sig, &witness)
}

fn accepts(
    psbt: &bitcoin::Psbt,
    index: usize,
    utxo: &TxOut,
    sig: &ScriptBuf,
    witness: &Witness,
) -> bool {
    let tx = &psbt.unsigned_tx;
    let Ok(interpreter) = Interpreter::from_txdata(
        &utxo.script_pubkey,
        sig,
        witness,
        tx.input[index].sequence,
        tx.lock_time,
    ) else {
        return false;
    };
    let secp = Secp256k1::verification_only();
    // Taproot signs every spent output; the others only their own.
    let all: Option<Vec<&TxOut>> = if utxo.script_pubkey.is_p2tr() {
        (0..tx.input.len()).map(|i| spent_output(psbt, i)).collect()
    } else {
        None
    };
    let prevouts = match &all {
        Some(outs) => Prevouts::All(outs),
        None => Prevouts::One(index, utxo),
    };
    // The interpreter reads a lock's value; consensus also needs this
    // input's sequence non-final for `after` (BIP-65) and a transaction
    // version of 2 or more for `older` (BIP-112).
    let sequence_final = tx.input[index].sequence == bitcoin::Sequence::MAX;
    // Consensus reads the version here as unsigned.
    let version_two = tx.version.0 as u32 >= 2;
    interpreter
        .iter(&secp, tx, index, &prevouts)
        .all(|step| match step {
            Ok(SatisfiedConstraint::AbsoluteTimelock { .. }) => !sequence_final,
            Ok(SatisfiedConstraint::RelativeTimelock { .. }) => version_two,
            Ok(_) => true,
            Err(_) => false,
        })
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
