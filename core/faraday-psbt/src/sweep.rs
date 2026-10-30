//! Signing with one private key read from a WIF, for a sweep
//! (`docs/INTEROP.md` §5 I11, F171): the key's coins, which an online
//! wallet watching its addresses gathered into a PSBT, signed over to
//! the person's own wallet.
//!
//! A WIF key has no BIP-32 origin, so an input is not matched by a key
//! origin as [`crate::sign`] matches one: it is the key's when the
//! output it spends pays one of the key's own scripts
//! ([`faraday_bip::wif::SingleKey::scripts`]: P2PKH, and for a
//! compressed key P2WPKH and P2SH-P2WPKH). Every other input is left
//! alone. The checks are [`crate::sign`]'s: the PSBT is inspected
//! against the context, a blocked warning refuses it, and a danger
//! refuses it unless it is a holdable one the caller passes, but for
//! [`WarningKind::NoParticipatingKey`], which says only that no loaded
//! master key takes part, and none does here. Each input's BIP-174
//! signer checks hold as in signing ([`Reason`]), a SegWit v0 input of a
//! transaction of several inputs needs its previous transaction, and a
//! legacy input needs its previous transaction too.
//!
//! The signature is ECDSA under RFC 6979 ([`Nonce`]), made twice and
//! compared, and verified against the public key before it is written,
//! keyed by the public key in the form the WIF states.

use alloc::vec::Vec;

use bitcoin::sighash::SighashCache;
use bitcoin::{PublicKey, ecdsa};
use faraday_bip::keys::Fingerprint;
use faraday_bip::wif::SingleKey;
use faraday_crypto::work::{Compute, Work};

use crate::classify::{ScriptKind, classify, legacy_code_separator, spent_output};
use crate::context::Context;
use crate::finalize::is_complete;
use crate::inspect::{WarningKind, inspect};
use crate::psbt::{Error, Psbt};
use crate::reason::Reason;
use crate::sign::{InputSignature, Nonce, SigKind, SignResult};

/// Which inputs of `psbt` spend an output paying one of `key`'s
/// scripts, and are not finalized. A lookup over the inputs.
pub fn inputs_of(psbt: &Psbt, key: &SingleKey) -> Vec<usize> {
    let scripts = key.scripts();
    (0..psbt.inner().inputs.len())
        .filter(|&i| {
            let input = &psbt.inner().inputs[i];
            input.final_script_sig.is_none()
                && input.final_script_witness.is_none()
                && spent_output(psbt.inner(), i).is_some_and(|o| scripts.contains(&o.script_pubkey))
        })
        .collect()
}

/// Costs what `inspect` costs and an ECDSA signature and its check per
/// input signed. Signs every input of `psbt` that spends one of `key`'s
/// scripts, under the RFC 6979 nonce `nonce` names, after the checks
/// the module states. `accepted` is the holdable dangers the person held
/// past (F136). An input that is the key's and cannot be signed refuses
/// the whole PSBT, and so does a legacy one whose script code holds
/// `OP_CODESEPARATOR`, before anything is signed. A PSBT none of whose
/// inputs is the key's signs nothing and is not an error.
pub fn sign(
    c: &Compute,
    psbt: &mut Psbt,
    key: &SingleKey,
    ctx: &Context,
    accepted: &[WarningKind],
    nonce: Nonce,
) -> Result<SignResult, Error> {
    c.count(Work::Sign);
    if let Some(kind) = accepted.iter().find(|k| !k.is_holdable()) {
        return Err(Error::NotHoldable(kind.clone()));
    }
    let inspection = inspect(c, psbt, ctx);
    if inspection.has_blocked() {
        return Err(Error::Blocked(inspection.blocked()));
    }
    let dangers: Vec<WarningKind> = inspection
        .dangers_left(accepted)
        .into_iter()
        .filter(|k| *k != WarningKind::NoParticipatingKey)
        .collect();
    if !dangers.is_empty() {
        return Err(Error::Danger(dangers));
    }
    let mine = inputs_of(psbt, key);
    // Every input is checked before the first signature is written, so
    // a refusal leaves the PSBT as it came.
    for &i in &mine {
        let cl = classify(psbt.inner(), i);
        if legacy_code_separator(psbt.inner(), i) {
            return Err(Error::Unsupported {
                input: i,
                reason: Reason::CodeSeparator,
            });
        }
        if let Some(problem) = cl.problems.first() {
            return Err(Error::Unsupported {
                input: i,
                reason: *problem,
            });
        }
        if cl.amount_unverified {
            return Err(Error::Unsupported {
                input: i,
                reason: Reason::NoPrevTx,
            });
        }
        if !matches!(
            cl.kind,
            ScriptKind::P2pkh | ScriptKind::P2wpkh | ScriptKind::P2shP2wpkh
        ) {
            return Err(Error::Unsupported {
                input: i,
                reason: Reason::UnknownScript,
            });
        }
    }

    let tx = psbt.inner().unsigned_tx.clone();
    let mut cache = SighashCache::new(&tx);
    let fingerprint = Fingerprint(key.id());
    let mut signed_inputs = Vec::with_capacity(mine.len());
    for i in mine {
        let sig = sign_input(psbt, &mut cache, i, key, nonce)?;
        signed_inputs.push(InputSignature {
            index: i,
            fingerprint,
            kind: SigKind::Ecdsa,
            sig_bytes: sig.to_vec(),
            verified: true,
            deterministic_ok: true,
            musig: None,
        });
        psbt.inner_mut().inputs[i]
            .partial_sigs
            .insert(key.public_key(), sig);
    }
    Ok(SignResult {
        complete: is_complete(psbt),
        signed_inputs,
        nonces_shared: 0,
    })
}

/// Costs an ECDSA signature twice and its check. One input's signature
/// by `key`, over the hash the input's own sighash type asks for.
fn sign_input(
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    key: &SingleKey,
    nonce: Nonce,
) -> Result<ecdsa::Signature, Error> {
    let secp = key.secp();
    let (msg, sighash_type) = psbt
        .inner()
        .sighash_ecdsa(index, cache)
        .map_err(|_| Error::Sighash { input: index })?;
    let once = || match nonce {
        Nonce::LowR => secp.sign_ecdsa_low_r(&msg, key.secret_key()),
        Nonce::First => secp.sign_ecdsa(&msg, key.secret_key()),
    };
    let first = once();
    let second = once();
    if first.serialize_der()[..] != second.serialize_der()[..] {
        return Err(Error::Nondeterministic { input: index });
    }
    let public: PublicKey = key.public_key();
    secp.verify_ecdsa(&msg, &first, &public.inner)
        .map_err(|_| Error::VerifyFailed { input: index })?;
    Ok(ecdsa::Signature {
        signature: first,
        sighash_type,
    })
}
