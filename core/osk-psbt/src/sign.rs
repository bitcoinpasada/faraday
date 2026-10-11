//! Signing: the only place in the core where a private key is used.
//!
//! Per input, the key is derived from the PSBT's own key-origin data
//! (fingerprint and path in `bip32_derivation` or `tap_key_origins`),
//! never from a guessed path, and checked against the public key the PSBT
//! names and against the script being spent before any signature is made.
//!
//! Nonces (`docs/PLANNING.md` §8.4 #7, §16.14, §16.38): ECDSA uses
//! libsecp256k1's RFC 6979 nonce function, either stopping at the first
//! nonce or retrying for a low `r` ([`Nonce`]). Schnorr signs under
//! BIP-340 with the auxiliary randomness [`Aux`] carries: all-zero, so
//! the bytes are reproducible, or 32 fresh bytes from the session, which
//! blind the nonce against a fault attack and make every signature
//! different. With [`Aux::Deterministic`] the same PSBT, key and
//! [`Nonce`] always give the same bytes, which is what the
//! cross-implementation check (#8) compares.
//!
//! Every signature is verified against its public key before it is
//! written (#10), and produced twice to confirm the bytes are identical
//! (#8) — the second signing reuses the first's auxiliary randomness, so
//! the check is of the signing itself and not of the randomness.
//! Anti-exfil is a later milestone.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use bitcoin::bip32::DerivationPath;
use bitcoin::hashes::{Hash, HashEngine, sha256};
use bitcoin::key::TapTweak;
use bitcoin::secp256k1::{self, Keypair, Message, XOnlyPublicKey};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{TapLeafHash, TapNodeHash};
use bitcoin::{PublicKey, ScriptBuf, TxOut, ecdsa, taproot};
use miniscript::{ExtParams, Miniscript, SigType, Tap, ToPublicKey};
use osk_bip::keys::{Fingerprint, MasterKey};
use osk_crypto::Zeroize;

use crate::classify::{Classified, ScriptKind, classify};
use crate::context::Context;
use crate::finalize::{is_complete, key_in_script};
use crate::inspect::{MusigRound, inspect, musig_keyed_here, musig_round};
use crate::musig::MusigSession;
use crate::psbt::{Error, Psbt};

/// Which of the two standard RFC 6979 nonces an ECDSA signature uses.
///
/// Both are deterministic and both are in wide use, so the choice is the
/// user's: a signature only compares byte for byte against another
/// implementation that picked its nonce the same way (§16.38).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Nonce {
    /// Bitcoin Core's grinding: plain RFC 6979 first, then, while the
    /// signature's `r` is at least 2^255 (a compact first byte of 0x80 or
    /// more, a DER `r` of 33 bytes), retry with RFC 6979 "additional
    /// data" of 32 bytes holding a little-endian `u32` counter, 1, 2, 3…
    /// The result is the shortest form of the signature.
    ///
    /// What Bitcoin Core, Sparrow and Electrum produce for transactions,
    /// and what Sparrow and Electrum produce for messages.
    #[default]
    LowR,
    /// The first RFC 6979 nonce, with no additional data.
    ///
    /// What the RFC's own vectors, BIP-174's role vectors, Trezor and
    /// Bitcoin Core's message signing produce.
    First,
}

/// Where a Schnorr signature's auxiliary randomness comes from
/// (§16.14, security review 2026-09-11, L3).
///
/// BIP-340 mixes 32 auxiliary bytes into the nonce. Zero is allowed and
/// is what a device produces when its signatures are meant to be
/// reproduced byte for byte by another implementation; fresh randomness
/// is what protects the nonce from a fault or side-channel attack on the
/// hash that derives it, at the cost of a signature no one can
/// reproduce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Schnorr {
    /// All-zero auxiliary randomness, as `sign_schnorr_no_aux_rand`.
    #[default]
    Deterministic,
    /// 32 bytes from the session, new for every signature.
    Fresh,
}

/// The auxiliary randomness one signing call uses: the [`Schnorr`]
/// choice with the bytes the caller drew for this call.
///
/// The caller draws them once per call, from the session key
/// (`SessionKey::derive`) with a counter, so no two calls of a session
/// draw the same; [`sign`] mixes the input's index in so no two inputs
/// of one call share them either.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Aux {
    /// Zero auxiliary randomness.
    #[default]
    Deterministic,
    /// The 32 bytes this call was given.
    Fresh([u8; 32]),
}

impl fmt::Debug for Aux {
    /// The bytes are not secret, but they are never worth printing and a
    /// signature is easier to reason about when they cannot be.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Aux::Deterministic => f.write_str("Aux::Deterministic"),
            Aux::Fresh(_) => f.write_str("Aux::Fresh(..)"),
        }
    }
}

impl Aux {
    /// The 32 bytes input `index` signs under: zero in the
    /// deterministic mode, and otherwise this call's bytes hashed with
    /// the index, so that two inputs of one call never sign under the
    /// same auxiliary randomness.
    pub(crate) fn for_input(self, index: usize) -> [u8; 32] {
        match self {
            Aux::Deterministic => [0u8; 32],
            Aux::Fresh(bytes) => {
                let mut engine = sha256::Hash::engine();
                engine.input(AUX_LABEL);
                engine.input(&bytes);
                engine.input(&(index as u64).to_le_bytes());
                sha256::Hash::from_engine(engine).to_byte_array()
            }
        }
    }
}

/// The label the per-input auxiliary randomness is hashed under.
const AUX_LABEL: &[u8] = b"osk-schnorr-aux";

/// What part a MuSig2 signature plays on its input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusigRole {
    /// One participant's partial signature, written as BIP-373's
    /// `PSBT_IN_MUSIG2_PARTIAL_SIG` beside the public nonce it was made
    /// under.
    Partial,
    /// Every participant's partial signature aggregated, written as the
    /// input's `tap_key_sig`, which finalizes it as any key-path spend.
    Aggregated,
}

/// Which signature algorithm an input used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigKind {
    /// ECDSA with RFC 6979 nonce (legacy and SegWit v0).
    Ecdsa,
    /// BIP-340 Schnorr (taproot).
    Schnorr,
}

/// One signature written to the PSBT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSignature {
    /// Input index.
    pub index: usize,
    /// The key that signed.
    pub fingerprint: Fingerprint,
    /// Algorithm.
    pub kind: SigKind,
    /// The bytes as written to the PSBT: DER plus sighash byte for ECDSA,
    /// 64 bytes (plus sighash byte when not `DEFAULT`) for Schnorr. For
    /// comparison with another implementation's output.
    pub sig_bytes: Vec<u8>,
    /// Verified against the public key before being written. Always
    /// `true` in a returned result; a failure is an error.
    pub verified: bool,
    /// Signing twice gave identical bytes. Always `true` in a returned
    /// result; a failure is an error.
    pub deterministic_ok: bool,
    /// What part this signature plays in a MuSig2 spend, when it is one.
    pub musig: Option<MusigRole>,
}

/// What [`sign`] did.
#[derive(Debug, PartialEq, Eq)]
pub struct SignResult {
    /// Signatures written, in input order.
    pub signed_inputs: Vec<InputSignature>,
    /// Public nonces this pass drew, wrote and kept in the session:
    /// round 1 of §16.100, one per input and participant.
    pub nonces_shared: usize,
    /// Every input now has enough signatures to finalize.
    pub complete: bool,
    /// What the pass did for the threshold wallet it spent, when it
    /// spent one (`docs/PLANNING.md` §16.103).
    pub threshold: Option<ThresholdOutcome>,
}

/// Signs every input that names one of the `selection` fingerprints in
/// its key origins, using the matching master key from `keys`, the
/// RFC 6979 nonce `nonce` names and the Schnorr auxiliary randomness
/// `aux` carries.
///
/// Refuses (before touching the PSBT) when the inspection has any
/// blocked warning, whatever `force` says; when it has any danger warning
/// and `force` is false; when a selected fingerprint has no master key;
/// and when an input that names a selected key spends a script this crate
/// cannot sign. A SegWit v0 input with no previous transaction, in a
/// transaction with more than one input, is refused in both places: the
/// block, and [`Error::Unsupported`] on the input itself. The same input
/// alone in its transaction is signed from its `witness_utxo`, because
/// the fee attack the refusal stops needs a second signed input to
/// combine with. A key-origin claim that the derived key does not match is
/// [`Error::KeyMismatch`]. Inputs that are already final, or name none of
/// the selected keys, are left alone.
///
/// `seed` is where every MuSig2 and threshold nonce this pass draws
/// comes from, and it must be fresh, uniformly random bytes for every
/// call: a seed used twice on one transaction can draw one secret nonce
/// for two different signatures, which gives away the key. A MuSig2
/// session takes the seed of the call that opens it and moves it forward
/// after every draw, so nonces drawn while it stays open never repeat;
/// a new session for the same transaction needs a new seed. OpenSigner
/// derives one per signing press from its session key and a count of
/// the draws.
#[allow(clippy::too_many_arguments)]
pub fn sign(
    psbt: &mut Psbt,
    keys: &[&MasterKey],
    selection: &[Fingerprint],
    ctx: &Context,
    force: bool,
    nonce: Nonce,
    aux: Aux,
    session: &mut Option<MusigSession>,
    seed: [u8; 32],
    shares: &[ShareKey],
    others: &[u32],
) -> Result<SignResult, Error> {
    let inspection = inspect(psbt, ctx);
    if inspection.has_blocked() {
        return Err(Error::Blocked(inspection.blocked()));
    }
    if !force && inspection.has_danger() {
        return Err(Error::Danger(inspection.dangers()));
    }
    let mut selected: Vec<(Fingerprint, &MasterKey)> = Vec::with_capacity(selection.len());
    for fp in selection {
        let key = keys
            .iter()
            .find(|k| k.fingerprint() == *fp)
            .ok_or(Error::NoSuchKey(*fp))?;
        selected.push((*fp, key));
    }

    let tx = psbt.inner().unsigned_tx.clone();
    let txid = tx.compute_txid();
    let mut cache = SighashCache::new(&tx);
    let mut signed_inputs = Vec::new();
    let mut nonces_shared = 0usize;
    let mut threshold_entries: Vec<crate::threshold::CarryEntry> = Vec::new();
    let mut threshold_signers: Vec<secp256k1::PublicKey> = Vec::new();
    let mut threshold: Option<(usize, usize)> = None;

    for i in 0..psbt.inner().inputs.len() {
        let input = &psbt.inner().inputs[i];
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            continue;
        }
        let c = classify(psbt.inner(), i);
        // A threshold input names no loaded key in its origins — its one
        // key is the group's, which no master derives — so it is handled
        // before the key claims are read (§16.103).
        if let Some(ti) = inspection.inputs[i].threshold.as_ref() {
            if ti.ours.is_empty() {
                continue;
            }
            if let Some(problem) = c.problems.first() {
                return Err(Error::Unsupported {
                    input: i,
                    reason: problem,
                });
            }
            if c.utxo.is_none() {
                return Err(Error::MissingUtxo { input: i });
            }
            let pass = sign_threshold(
                psbt, &mut cache, i, &c, ti, shares, others, ctx.carry, &seed,
            )?;
            signed_inputs.extend(pass.signatures);
            threshold_entries.extend(pass.entries);
            if threshold_signers.is_empty() {
                threshold_signers = pass.signers;
            }
            if threshold.is_none() {
                threshold = Some((ti.info.t, pass.partial_sigs));
            }
            continue;
        }
        let ecdsa_claims: Vec<(
            secp256k1::PublicKey,
            Fingerprint,
            DerivationPath,
            &MasterKey,
        )> = input
            .bip32_derivation
            .iter()
            .filter_map(|(pk, (fp, path))| {
                let fp = Fingerprint::from(*fp);
                let (_, key) = selected.iter().find(|(f, _)| *f == fp)?;
                Some((*pk, fp, path.clone(), *key))
            })
            .collect();
        let tap_claims: Vec<(
            secp256k1::XOnlyPublicKey,
            Fingerprint,
            DerivationPath,
            &MasterKey,
        )> = input
            .tap_key_origins
            .iter()
            .filter(|(_, (leaves, _))| leaves.is_empty())
            .filter_map(|(xonly, (_, (fp, path)))| {
                let fp = Fingerprint::from(*fp);
                let (_, key) = selected.iter().find(|(f, _)| *f == fp)?;
                Some((*xonly, fp, path.clone(), *key))
            })
            .collect();
        let leaf_claims: Vec<(
            secp256k1::XOnlyPublicKey,
            Vec<TapLeafHash>,
            Fingerprint,
            DerivationPath,
            &MasterKey,
        )> = input
            .tap_key_origins
            .iter()
            .filter(|(_, (leaves, _))| !leaves.is_empty())
            .filter_map(|(xonly, (leaves, (fp, path)))| {
                let fp = Fingerprint::from(*fp);
                let (_, key) = selected.iter().find(|(f, _)| *f == fp)?;
                Some((*xonly, leaves.clone(), fp, path.clone(), *key))
            })
            .collect();
        if ecdsa_claims.is_empty() && tap_claims.is_empty() && leaf_claims.is_empty() {
            continue;
        }
        if let Some(problem) = c.problems.first() {
            return Err(Error::Unsupported {
                input: i,
                reason: problem,
            });
        }
        if c.utxo.is_none() {
            return Err(Error::MissingUtxo { input: i });
        }
        if c.amount_unverified {
            return Err(Error::Unsupported {
                input: i,
                reason: "no previous transaction for a SegWit input",
            });
        }

        match c.kind {
            ScriptKind::P2pkh
            | ScriptKind::P2shP2wpkh
            | ScriptKind::P2wpkh
            | ScriptKind::Multisig { .. }
            | ScriptKind::Miniscript { .. } => {
                for (pk, fp, path, master) in ecdsa_claims {
                    let sig = sign_ecdsa(psbt, &mut cache, i, &c, pk, &path, master, nonce)?;
                    signed_inputs.push(InputSignature {
                        index: i,
                        fingerprint: fp,
                        kind: SigKind::Ecdsa,
                        sig_bytes: sig.to_vec(),
                        verified: true,
                        deterministic_ok: true,
                        musig: None,
                    });
                    psbt.inner_mut().inputs[i]
                        .partial_sigs
                        .insert(PublicKey::new(pk), sig);
                }
            }
            ScriptKind::P2trKey if inspection.inputs[i].musig.is_some() => {
                let musig = inspection.inputs[i]
                    .musig
                    .as_ref()
                    .expect("checked by the match arm");
                let mine: Vec<&crate::inspect::MusigParticipation> = musig
                    .ours
                    .iter()
                    .filter(|p| selected.iter().any(|(f, _)| *f == p.fingerprint))
                    .collect();
                if mine.is_empty() {
                    continue;
                }
                let done = sign_musig(
                    psbt, &mut cache, i, &c, musig, &mine, &selected, aux, txid, session, &seed,
                )?;
                nonces_shared += done.nonces_shared;
                signed_inputs.extend(done.signatures);
            }
            ScriptKind::P2trKey => {
                for (xonly, fp, path, master) in tap_claims {
                    let sig =
                        sign_taproot(psbt, &mut cache, i, &c, xonly, &path, master, aux, None)?;
                    signed_inputs.push(InputSignature {
                        index: i,
                        fingerprint: fp,
                        kind: SigKind::Schnorr,
                        sig_bytes: sig.to_vec(),
                        verified: true,
                        deterministic_ok: true,
                        musig: None,
                    });
                    psbt.inner_mut().inputs[i].tap_key_sig = Some(sig);
                }
            }
            ScriptKind::P2trScript => {
                let before = signed_inputs.len();
                // The leaves the PSBT carries scripts for, in the order
                // their control blocks sort: one signature per leaf a
                // key of ours appears in, because the coordinator is the
                // one that chooses which leaf the transaction spends.
                let leaves: Vec<(TapLeafHash, ScriptBuf)> = psbt.inner().inputs[i]
                    .tap_scripts
                    .values()
                    .map(|(script, version)| {
                        (TapLeafHash::from_script(script, *version), script.clone())
                    })
                    .collect();
                for (leaf_hash, script) in &leaves {
                    for (xonly, claimed, fp, path, master) in &leaf_claims {
                        let here = &psbt.inner().inputs[i];
                        if !claimed.contains(leaf_hash) || !key_in_leaf(here, script, *xonly) {
                            continue;
                        }
                        let sig = sign_tap_leaf(
                            psbt, &mut cache, i, &c, *xonly, *leaf_hash, path, master, aux,
                        )?;
                        signed_inputs.push(InputSignature {
                            index: i,
                            fingerprint: *fp,
                            kind: SigKind::Schnorr,
                            sig_bytes: sig.to_vec(),
                            verified: true,
                            deterministic_ok: true,
                            musig: None,
                        });
                        psbt.inner_mut().inputs[i]
                            .tap_script_sigs
                            .insert((*xonly, *leaf_hash), sig);
                    }
                }
                // The internal key, when it is one of ours and the
                // output really is that key tweaked by the tree the
                // input states. The coordinator picks which of the two
                // paths the transaction ends up spending.
                let merkle = psbt.inner().inputs[i].tap_merkle_root;
                let spk = &c.utxo.as_ref().expect("checked by caller").script_pubkey;
                for (xonly, fp, path, master) in tap_claims {
                    if psbt.inner().inputs[i].tap_internal_key != Some(xonly)
                        || ScriptBuf::new_p2tr(master.secp(), xonly, merkle) != *spk
                    {
                        continue;
                    }
                    let sig =
                        sign_taproot(psbt, &mut cache, i, &c, xonly, &path, master, aux, merkle)?;
                    signed_inputs.push(InputSignature {
                        index: i,
                        fingerprint: fp,
                        kind: SigKind::Schnorr,
                        sig_bytes: sig.to_vec(),
                        verified: true,
                        deterministic_ok: true,
                        musig: None,
                    });
                    psbt.inner_mut().inputs[i].tap_key_sig = Some(sig);
                }
                if signed_inputs.len() == before {
                    return Err(Error::Unsupported {
                        input: i,
                        reason: "no leaf or key path of this taproot input is one of ours",
                    });
                }
            }
            ScriptKind::Unknown => {
                return Err(Error::Unsupported {
                    input: i,
                    reason: "unknown script type",
                });
            }
        }
    }

    // A finished transaction carries no `osk` record: Sparrow reads the
    // PSBT it built and the transaction, never the session.
    let finished: Vec<usize> = (0..psbt.inner().inputs.len())
        .filter(|i| psbt.inner().inputs[*i].tap_key_sig.is_some())
        .collect();
    for i in finished {
        psbt.inner_mut().inputs[i]
            .proprietary
            .retain(|k, _| k.prefix != crate::threshold::PREFIX);
    }

    let threshold = threshold.map(|(t, partial_sigs)| ThresholdOutcome {
        t,
        partial_sigs,
        carry: (!threshold_entries.is_empty()).then(|| crate::threshold::CarrySection {
            signers: threshold_signers,
            entries: threshold_entries,
        }),
    });
    Ok(SignResult {
        complete: is_complete(psbt),
        signed_inputs,
        nonces_shared,
        threshold,
    })
}

/// Derives the key at `path`, checks it is the key the PSBT names and
/// that it appears in the script, then signs and verifies. The derived
/// key lives only in this frame.
///
/// The curve context is the master key's own, blinded by the session
/// (`osk_bip::keys`, security review M1), not one made here.
#[allow(clippy::too_many_arguments)]
fn sign_ecdsa(
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    c: &Classified,
    pk: secp256k1::PublicKey,
    path: &DerivationPath,
    master: &MasterKey,
    nonce: Nonce,
) -> Result<ecdsa::Signature, Error> {
    let secp = master.secp();
    let derived = master.derive(path);
    if derived.to_xpub().public_key != pk {
        return Err(Error::KeyMismatch { input: index });
    }
    let input = &psbt.inner().inputs[index];
    if !key_in_script(c, input, &PublicKey::new(pk)) {
        return Err(Error::KeyMismatch { input: index });
    }
    let (msg, sighash_type) = psbt
        .inner()
        .sighash_ecdsa(index, cache)
        .map_err(|_| Error::Sighash { input: index })?;

    let once = |secret: &secp256k1::SecretKey| match nonce {
        Nonce::LowR => secp.sign_ecdsa_low_r(&msg, secret),
        Nonce::First => secp.sign_ecdsa(&msg, secret),
    };
    let first = once(derived.secret_key());
    let second = once(derived.secret_key());
    if first.serialize_der()[..] != second.serialize_der()[..] {
        return Err(Error::Nondeterministic { input: index });
    }
    secp.verify_ecdsa(&msg, &first, &pk)
        .map_err(|_| Error::VerifyFailed { input: index })?;
    Ok(ecdsa::Signature {
        signature: first,
        sighash_type,
    })
}

/// Whether `xonly` is a key the leaf script checks a signature against,
/// read through the miniscript and not by searching the leaf's bytes. A
/// `pkh` fragment leaves a hash in the script; the key behind it comes
/// from the input's own `tap_key_origins`.
fn key_in_leaf(input: &bitcoin::psbt::Input, leaf: &ScriptBuf, xonly: XOnlyPublicKey) -> bool {
    let ext = ExtParams {
        raw_pkh: true,
        ..ExtParams::sane()
    };
    let Ok(ms) = Miniscript::<XOnlyPublicKey, Tap>::parse_with_ext(leaf, &ext) else {
        return false;
    };
    let mut hashes = BTreeMap::new();
    for key in input.tap_key_origins.keys() {
        hashes.insert(key.to_pubkeyhash(SigType::Schnorr), *key);
    }
    crate::classify::script_key_set(&ms, &hashes).contains(&xonly)
}

/// BIP-341 script-path signing: the derived key must be the one the PSBT
/// names for this leaf, and the signature commits to the leaf's hash. It
/// is verified against the key itself, which is what a script-path
/// signature is checked against — no tweak.
///
/// The curve context is the master key's own, as in [`sign_ecdsa`].
#[allow(clippy::too_many_arguments)]
fn sign_tap_leaf(
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    c: &Classified,
    xonly: XOnlyPublicKey,
    leaf_hash: TapLeafHash,
    path: &DerivationPath,
    master: &MasterKey,
    aux: Aux,
) -> Result<taproot::Signature, Error> {
    let secp = master.secp();
    let derived = master.derive(path);
    if derived.to_xpub().public_key.x_only_public_key().0 != xonly {
        return Err(Error::KeyMismatch { input: index });
    }
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, c, index)?;
    let sighash = cache
        .taproot_script_spend_signature_hash(index, &prevouts.as_ref(), leaf_hash, sighash_type)
        .map_err(|_| Error::Sighash { input: index })?;
    let msg = Message::from(sighash);

    let mut keypair = Keypair::from_secret_key(secp, derived.secret_key());
    let aux_rand = aux.for_input(index);
    let first = secp.sign_schnorr_with_aux_rand(&msg, &keypair, &aux_rand);
    let second = secp.sign_schnorr_with_aux_rand(&msg, &keypair, &aux_rand);
    keypair.non_secure_erase();

    if first.serialize() != second.serialize() {
        return Err(Error::Nondeterministic { input: index });
    }
    secp.verify_schnorr(&first, &msg, &xonly)
        .map_err(|_| Error::VerifyFailed { input: index })?;
    Ok(taproot::Signature {
        signature: first,
        sighash_type,
    })
}

/// The taproot sighash type the input asks for and the prevouts that
/// sighash commits to: the one input under `ANYONECANPAY`, every input
/// otherwise.
fn tap_sighash_inputs(
    psbt: &Psbt,
    c: &Classified,
    index: usize,
) -> Result<(TapSighashType, PrevoutSet), Error> {
    let input = &psbt.inner().inputs[index];
    let sighash_type = match input.sighash_type {
        None => TapSighashType::Default,
        Some(t) => t
            .taproot_hash_ty()
            .map_err(|_| Error::Sighash { input: index })?,
    };
    Ok((sighash_type, tap_prevouts(psbt, c, index, sighash_type)?))
}

/// The prevouts a taproot sighash of type `sighash_type` commits to.
/// Split out so that [`crate::verify`] checks a signature that arrived
/// against the prevouts this crate would have signed over, under the
/// sighash type the signature itself states.
pub(crate) fn tap_prevouts(
    psbt: &Psbt,
    c: &Classified,
    index: usize,
    sighash_type: TapSighashType,
) -> Result<PrevoutSet, Error> {
    let anyone_can_pay = matches!(
        sighash_type,
        TapSighashType::AllPlusAnyoneCanPay
            | TapSighashType::NonePlusAnyoneCanPay
            | TapSighashType::SinglePlusAnyoneCanPay
    );
    if anyone_can_pay {
        let utxo = c.utxo.as_ref().expect("checked by caller").clone();
        return Ok(PrevoutSet::One(index, utxo));
    }
    let mut prevouts: Vec<TxOut> = Vec::with_capacity(psbt.inner().inputs.len());
    for j in 0..psbt.inner().inputs.len() {
        prevouts.push(
            psbt.inner()
                .spend_utxo(j)
                .cloned()
                .map_err(|_| Error::MissingUtxo { input: j })?,
        );
    }
    Ok(PrevoutSet::All(prevouts))
}

/// The prevouts a taproot sighash commits to, owned so that the caller
/// can hand out the borrowed `Prevouts` the sighash cache wants.
pub(crate) enum PrevoutSet {
    One(usize, TxOut),
    All(Vec<TxOut>),
}

impl PrevoutSet {
    pub(crate) fn as_ref(&self) -> Prevouts<'_, TxOut> {
        match self {
            PrevoutSet::One(index, utxo) => Prevouts::One(*index, utxo.clone()),
            PrevoutSet::All(all) => Prevouts::All(all),
        }
    }
}

/// BIP-86 and BIP-341 key-path signing: the derived key must be the
/// input's internal key, the output must be that key tweaked by
/// `merkle_root` (nothing for a key-only wallet, the tree's root for a
/// `tr()` with leaves), and the signature is verified against the
/// tweaked output key. The auxiliary randomness is `aux`'s value for
/// this input.
///
/// The curve context is the master key's own, as in [`sign_ecdsa`].
#[allow(clippy::too_many_arguments)]
fn sign_taproot(
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    c: &Classified,
    xonly: secp256k1::XOnlyPublicKey,
    path: &DerivationPath,
    master: &MasterKey,
    aux: Aux,
    merkle_root: Option<TapNodeHash>,
) -> Result<taproot::Signature, Error> {
    let secp = master.secp();
    let input = &psbt.inner().inputs[index];
    let internal = input.tap_internal_key.ok_or(Error::Unsupported {
        input: index,
        reason: "taproot input without tap_internal_key",
    })?;
    let utxo = c.utxo.as_ref().expect("checked by caller");
    if internal != xonly || ScriptBuf::new_p2tr(secp, internal, merkle_root) != utxo.script_pubkey {
        return Err(Error::KeyMismatch { input: index });
    }
    let derived = master.derive(path);
    if derived.to_xpub().public_key.x_only_public_key().0 != xonly {
        return Err(Error::KeyMismatch { input: index });
    }

    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, c, index)?;
    let sighash = cache
        .taproot_key_spend_signature_hash(index, &prevouts.as_ref(), sighash_type)
        .map_err(|_| Error::Sighash { input: index })?;
    let msg = Message::from(sighash);

    // `bitcoin` forces two secret copies here: the untweaked keypair and
    // the tweaked one. Both are erased before returning.
    let mut keypair = Keypair::from_secret_key(secp, derived.secret_key());
    let mut tweaked = keypair.tap_tweak(secp, merkle_root).to_keypair();
    // Both signings use the one value, so that a second signature that
    // differs is the signing going wrong and not the randomness doing
    // its job.
    let aux_rand = aux.for_input(index);
    let first = secp.sign_schnorr_with_aux_rand(&msg, &tweaked, &aux_rand);
    let second = secp.sign_schnorr_with_aux_rand(&msg, &tweaked, &aux_rand);
    let (output_key, _) = tweaked.x_only_public_key();
    keypair.non_secure_erase();
    tweaked.non_secure_erase();

    if first.serialize() != second.serialize() {
        return Err(Error::Nondeterministic { input: index });
    }
    secp.verify_schnorr(&first, &msg, &output_key)
        .map_err(|_| Error::VerifyFailed { input: index })?;
    Ok(taproot::Signature {
        signature: first,
        sighash_type,
    })
}

/// What one MuSig2 input's pass produced.
struct MusigPass {
    signatures: Vec<InputSignature>,
    nonces_shared: usize,
}

/// Signing a BIP-373 MuSig2 input (`docs/PLANNING.md` §16.100).
///
/// Each participant this device holds and the caller selected is handled
/// one of three ways, decided from the input as it arrived so that two
/// participants of one wallet do not decide against each other's fresh
/// nonce:
///
/// - Round 2, when the input carries this participant's public nonce and
///   `session` holds its secret: BIP-327's `Sign` with that secret nonce,
///   taken out of the session by value so it cannot sign twice.
/// - Signing last, when no nonce of this participant's is on the input
///   and every other participant's is: BIP-327's `DeterministicSign`,
///   which needs nothing kept between rounds.
/// - Round 1 otherwise: `NonceGen` from the session's seed, which moves
///   forward after every draw, the public nonce written and the secret
///   kept. Every partial signature already
///   on the input is dropped, because each was made against an aggregate
///   nonce this one is not in.
///
/// Every partial signature is verified before it is written, every other
/// participant's partial signature already present is verified too, and
/// when the last one is there they are aggregated, checked as a BIP-340
/// signature of the output key, and written as `tap_key_sig`, which
/// finalizes the input as any key-path spend.
#[allow(clippy::too_many_arguments)]
fn sign_musig(
    psbt: &mut Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    c: &Classified,
    musig: &crate::inspect::MusigInput,
    mine: &[&crate::inspect::MusigParticipation],
    selected: &[(Fingerprint, &MasterKey)],
    aux: Aux,
    txid: bitcoin::Txid,
    session: &mut Option<MusigSession>,
    seed: &[u8; 32],
) -> Result<MusigPass, Error> {
    let secp = secp256k1::Secp256k1::new();
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, c, index)?;
    if sighash_type != TapSighashType::Default {
        return Err(Error::Unsupported {
            input: index,
            reason: "a MuSig2 input signs under SIGHASH_DEFAULT only",
        });
    }
    let sighash = cache
        .taproot_key_spend_signature_hash(index, &prevouts.as_ref(), sighash_type)
        .map_err(|_| Error::Sighash { input: index })?;
    let msg: [u8; 32] = sighash.to_byte_array();

    // Every nonce and partial signature the input already carries,
    // whichever of the three aggregate forms its key names.
    let input = &psbt.inner().inputs[index];
    let mut nonces: Vec<(secp256k1::PublicKey, osk_bip::musig::PubNonce)> =
        crate::musig::pub_nonces(input)
            .map_err(|e| Error::Unsupported {
                input: index,
                reason: e.reason(),
            })?
            .into_iter()
            .filter(|e| musig_keyed_here(musig, &e.aggregate))
            .map(|e| (e.participant, e.value))
            .collect();
    let mut psigs: Vec<(secp256k1::PublicKey, osk_bip::musig::PartialSig)> =
        crate::musig::partial_sigs(input)
            .map_err(|e| Error::Unsupported {
                input: index,
                reason: e.reason(),
            })?
            .into_iter()
            .filter(|e| musig_keyed_here(musig, &e.aggregate))
            .map(|e| (e.participant, e.value))
            .collect();

    // Which way each participant goes, read off the input before this
    // pass writes anything to it.
    let view = session.as_ref().map(|s| s.view());
    let rounds: Vec<MusigRound> = mine
        .iter()
        .map(|p| musig_round(input, musig, p, view.as_ref(), txid, index))
        .collect();
    drop(view);

    let mut written = Vec::new();
    let mut nonces_shared = 0usize;
    for (participation, round) in mine.iter().zip(&rounds) {
        let (_, master) = selected
            .iter()
            .find(|(f, _)| *f == participation.fingerprint)
            .expect("the caller selected this key");
        let derived = master.derive(&participation.path);
        if derived.to_xpub().public_key != participation.key {
            return Err(Error::KeyMismatch { input: index });
        }
        // Every use of the secret key goes through the master key's
        // blinded context, as the single-signature paths do.
        let signer = master.secp();
        match round {
            MusigRound::Done | MusigRound::Wait => continue,
            MusigRound::Share { .. } => {
                // The session this device keeps is for one transaction:
                // another one replaces it, and the nonces of the old one
                // are dropped and wiped with it.
                if session.as_ref().is_none_or(|s| s.txid() != txid) {
                    *session = Some(MusigSession::new(txid, *seed));
                }
                let open = session.as_mut().expect("just set");
                let rand = open.draw(index, &participation.key);
                let aggpk = musig.output_key.x_only_public_key().0.serialize();
                let (secnonce, pubnonce) = osk_bip::musig::nonce_gen(
                    signer,
                    &rand,
                    Some(derived.secret_key()),
                    &participation.key,
                    Some(&aggpk),
                    Some(&msg),
                    None,
                )
                .map_err(|_| Error::Unsupported {
                    input: index,
                    reason: "a MuSig2 nonce could not be drawn for this input",
                })?;
                open.keep(index, participation.key, pubnonce, secnonce);
                let here = &mut psbt.inner_mut().inputs[index];
                crate::musig::clear_partial_sigs(here);
                crate::musig::write_pub_nonce(
                    here,
                    &participation.key,
                    &musig.output_key,
                    &pubnonce,
                );
                psigs.clear();
                nonces.retain(|(pk, _)| *pk != participation.key);
                nonces.push((participation.key, pubnonce));
                nonces_shared += 1;
            }
            MusigRound::Sign => {
                let open = session.as_mut().expect("the round says it holds one");
                let (pubnonce, secnonce) = open
                    .take(index, &participation.key)
                    .expect("the round says it holds one");
                nonces.retain(|(pk, _)| *pk != participation.key);
                nonces.push((participation.key, pubnonce));
                let values =
                    musig_session(&secp, musig, &nonces, &pubnonce, participation.key, &msg)
                        .ok_or(Error::Sighash { input: index })?;
                let psig = osk_bip::musig::sign(signer, secnonce, derived.secret_key(), &values)
                    .map_err(|_| Error::Sighash { input: index })?;
                osk_bip::musig::partial_sig_verify_internal(
                    &secp,
                    &psig,
                    &pubnonce,
                    &participation.key,
                    &values,
                )
                .map_err(|_| Error::VerifyFailed { input: index })?;
                let here = &mut psbt.inner_mut().inputs[index];
                crate::musig::write_pub_nonce(
                    here,
                    &participation.key,
                    &musig.output_key,
                    &pubnonce,
                );
                crate::musig::write_partial_sig(here, &participation.key, &musig.output_key, &psig);
                psigs.retain(|(pk, _)| *pk != participation.key);
                psigs.push((participation.key, psig));
                written.push(InputSignature {
                    index,
                    fingerprint: participation.fingerprint,
                    kind: SigKind::Schnorr,
                    sig_bytes: psig.serialize().to_vec(),
                    verified: true,
                    deterministic_ok: true,
                    musig: Some(MusigRole::Partial),
                });
            }
            MusigRound::SignLast => {
                let others: Vec<osk_bip::musig::PubNonce> = musig
                    .participants
                    .iter()
                    .filter(|p| **p != participation.key)
                    .map(|p| {
                        nonces
                            .iter()
                            .find(|(pk, _)| pk == p)
                            .map(|(_, n)| *n)
                            .ok_or(Error::Unsupported {
                                input: index,
                                reason: "another participant's MuSig2 nonce is missing",
                            })
                    })
                    .collect::<Result<_, _>>()?;
                let aggothernonce =
                    osk_bip::musig::nonce_agg_with(&secp, &others).map_err(|_| {
                        Error::Unsupported {
                            input: index,
                            reason: "the other participants' MuSig2 nonces do not aggregate",
                        }
                    })?;
                // The same switch every Schnorr signature here follows:
                // fresh bytes blind the nonce, and their absence makes
                // the pass reproducible.
                let rand = match aux {
                    Aux::Deterministic => None,
                    Aux::Fresh(_) => Some(aux.for_input(index)),
                };
                let once = || {
                    osk_bip::musig::deterministic_sign(
                        signer,
                        derived.secret_key(),
                        &aggothernonce,
                        &musig.participants,
                        &musig.tweaks,
                        &msg,
                        rand.as_ref(),
                    )
                };
                let (pubnonce, psig) = once().map_err(|_| Error::Sighash { input: index })?;
                let (again_nonce, again_sig) =
                    once().map_err(|_| Error::Sighash { input: index })?;
                if pubnonce != again_nonce || psig != again_sig {
                    return Err(Error::Nondeterministic { input: index });
                }
                let values =
                    musig_session(&secp, musig, &nonces, &pubnonce, participation.key, &msg)
                        .ok_or(Error::Sighash { input: index })?;
                osk_bip::musig::partial_sig_verify_internal(
                    &secp,
                    &psig,
                    &pubnonce,
                    &participation.key,
                    &values,
                )
                .map_err(|_| Error::VerifyFailed { input: index })?;
                let here = &mut psbt.inner_mut().inputs[index];
                crate::musig::write_pub_nonce(
                    here,
                    &participation.key,
                    &musig.output_key,
                    &pubnonce,
                );
                crate::musig::write_partial_sig(here, &participation.key, &musig.output_key, &psig);
                nonces.push((participation.key, pubnonce));
                psigs.push((participation.key, psig));
                written.push(InputSignature {
                    index,
                    fingerprint: participation.fingerprint,
                    kind: SigKind::Schnorr,
                    sig_bytes: psig.serialize().to_vec(),
                    verified: true,
                    deterministic_ok: true,
                    musig: Some(MusigRole::Partial),
                });
            }
        }
    }

    // Every partial signature on the input, this device's and the
    // others', against the aggregate nonce they were all made under.
    let all: Vec<osk_bip::musig::PubNonce> = musig
        .participants
        .iter()
        .filter_map(|p| nonces.iter().find(|(pk, _)| pk == p).map(|(_, n)| *n))
        .collect();
    if all.len() != musig.participants.len() {
        return Ok(MusigPass {
            signatures: written,
            nonces_shared,
        });
    }
    let aggnonce = osk_bip::musig::nonce_agg_with(&secp, &all).map_err(|_| Error::Unsupported {
        input: index,
        reason: "the MuSig2 nonces on this input do not aggregate",
    })?;
    let values =
        osk_bip::musig::session_values(&secp, &musig.participants, &musig.tweaks, &aggnonce, &msg)
            .map_err(|_| Error::Sighash { input: index })?;
    for (pk, psig) in &psigs {
        let Some((_, nonce)) = nonces.iter().find(|(n, _)| n == pk) else {
            continue;
        };
        if osk_bip::musig::partial_sig_verify_internal(&secp, psig, nonce, pk, &values).is_err() {
            return Err(Error::MusigPartialSig {
                input: index,
                participant: hex33(pk),
            });
        }
    }
    if psigs.len() != musig.participants.len() {
        return Ok(MusigPass {
            signatures: written,
            nonces_shared,
        });
    }

    let ordered: Vec<osk_bip::musig::PartialSig> = musig
        .participants
        .iter()
        .filter_map(|p| psigs.iter().find(|(pk, _)| pk == p).map(|(_, s)| *s))
        .collect();
    let sig = osk_bip::musig::partial_sig_agg(&ordered, &values)
        .map_err(|_| Error::Sighash { input: index })?;
    let signature = secp256k1::schnorr::Signature::from_slice(&sig)
        .map_err(|_| Error::VerifyFailed { input: index })?;
    let output_key = musig.output_key.x_only_public_key().0;
    secp.verify_schnorr(&signature, &Message::from(sighash), &output_key)
        .map_err(|_| Error::VerifyFailed { input: index })?;
    let taproot_sig = taproot::Signature {
        signature,
        sighash_type,
    };
    psbt.inner_mut().inputs[index].tap_key_sig = Some(taproot_sig);
    written.push(InputSignature {
        index,
        fingerprint: mine
            .first()
            .map_or_else(|| Fingerprint([0; 4]), |p| p.fingerprint),
        kind: SigKind::Schnorr,
        sig_bytes: sig.to_vec(),
        verified: true,
        deterministic_ok: true,
        musig: Some(MusigRole::Aggregated),
    });
    Ok(MusigPass {
        signatures: written,
        nonces_shared,
    })
}

/// The session one partial signature is verified in: every nonce on the
/// input, with this participant's own in place of whatever was there.
fn musig_session<C: secp256k1::Verification>(
    secp: &secp256k1::Secp256k1<C>,
    musig: &crate::inspect::MusigInput,
    nonces: &[(secp256k1::PublicKey, osk_bip::musig::PubNonce)],
    mine: &osk_bip::musig::PubNonce,
    key: secp256k1::PublicKey,
    msg: &[u8],
) -> Option<osk_bip::musig::SessionValues> {
    let all: Vec<osk_bip::musig::PubNonce> = musig
        .participants
        .iter()
        .map(|p| {
            if *p == key {
                Some(*mine)
            } else {
                nonces.iter().find(|(pk, _)| pk == p).map(|(_, n)| *n)
            }
        })
        .collect::<Option<_>>()?;
    let aggnonce = osk_bip::musig::nonce_agg_with(secp, &all).ok()?;
    osk_bip::musig::session_values(secp, &musig.participants, &musig.tweaks, &aggnonce, msg).ok()
}

fn hex33(key: &secp256k1::PublicKey) -> alloc::string::String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = alloc::string::String::with_capacity(66);
    for b in key.serialize() {
        out.push(char::from(DIGITS[usize::from(b >> 4)]));
        out.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    out
}

// ---------------------------------------------------------------------
// Threshold signing (docs/PLANNING.md §16.103)
// ---------------------------------------------------------------------

/// The tag one `NonceGen` call's `rand` is hashed under, so that one
/// seed gives a different value for every input, signer and signer set.
const THRESHOLD_RAND_TAG: &[u8] = b"OpenSigner/threshold-rand";

/// One loaded share as the signer sees it: its public share, and the one
/// door its scalar comes through.
///
/// The secret exists only for the length of the call `open` makes, which
/// is how [`crate::ShareRef`]'s owner holds it (`LoadedShare::secret`).
pub struct ShareKey<'a> {
    /// `secshare · G`, which is what the group's record lists.
    pub pubshare: secp256k1::PublicKey,
    /// Runs the closure it is given on the secret share.
    pub open: &'a ShareDoor<'a>,
}

/// The door a [`ShareKey`]'s scalar comes through: a call that runs the
/// closure it is given on the secret share and nothing else.
pub type ShareDoor<'a> = dyn Fn(&mut dyn FnMut(&osk_bip::frost::SecShare)) + 'a;

impl fmt::Debug for ShareKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ShareKey(..)")
    }
}

/// What a pass did for the threshold wallet it spent.
pub struct ThresholdOutcome {
    /// How many shares must sign.
    pub t: usize,
    /// Partial signatures on the first threshold input after the pass.
    pub partial_sigs: usize,
    /// The carry section to write, when signers are still to sign. It is
    /// the secret this pass leaves behind, and nothing else holds it.
    pub carry: Option<crate::threshold::CarrySection>,
}

impl fmt::Debug for ThresholdOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ThresholdOutcome(..)")
    }
}

/// Two outcomes are the same pass when they are for the same group, have
/// the same partial signatures, and either both leave a carry section or
/// neither does. The section's nonces are secret and are never compared.
impl PartialEq for ThresholdOutcome {
    fn eq(&self, other: &Self) -> bool {
        self.t == other.t
            && self.partial_sigs == other.partial_sigs
            && self.carry.is_some() == other.carry.is_some()
    }
}

impl Eq for ThresholdOutcome {}

/// What one threshold input's pass produced.
struct ThresholdPass {
    signatures: Vec<InputSignature>,
    entries: Vec<crate::threshold::CarryEntry>,
    signers: Vec<secp256k1::PublicKey>,
    partial_sigs: usize,
}

/// Signing a threshold input (`docs/PLANNING.md` §16.103).
///
/// Either the input carries no nonce and this is the first location, in
/// which case the signer set is this share plus the `others` the caller
/// chose and every signer's nonce is drawn here; or it carries the whole
/// signer set's nonces and the carry section holds this share's secret
/// one, which is a later location. Every check the inspection made is
/// made again here against the input as it stands.
#[allow(clippy::too_many_arguments)]
fn sign_threshold(
    psbt: &mut Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    c: &Classified,
    ti: &crate::inspect::ThresholdInput,
    shares: &[ShareKey],
    others: &[u32],
    carry: Option<&crate::threshold::CarrySection>,
    seed: &[u8; 32],
) -> Result<ThresholdPass, Error> {
    use osk_bip::frost;

    let secp = secp256k1::Secp256k1::new();
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, c, index)?;
    if sighash_type != TapSighashType::Default {
        return Err(Error::Unsupported {
            input: index,
            reason: "a threshold input signs under SIGHASH_DEFAULT only",
        });
    }
    let sighash = cache
        .taproot_key_spend_signature_hash(index, &prevouts.as_ref(), sighash_type)
        .map_err(|_| Error::Sighash { input: index })?;
    let msg: [u8; 32] = sighash.to_byte_array();

    let refuse = |r: crate::threshold::Refusal| Error::Unsupported {
        input: index,
        reason: r.reason(),
    };
    let pubshare = |id: u32| {
        ti.pubshare(id).ok_or(Error::Unsupported {
            input: index,
            reason: "a threshold signer is not a share of this group",
        })
    };

    // The input as it arrived.
    let input = &psbt.inner().inputs[index];
    let records = crate::threshold::pub_nonces(input).map_err(|e| Error::Unsupported {
        input: index,
        reason: e.reason(),
    })?;
    let sig_records = crate::threshold::partial_sigs(input).map_err(|e| Error::Unsupported {
        input: index,
        reason: e.reason(),
    })?;
    let mut signers: Vec<u32> = ti.signers.clone();
    let mut nonces: Vec<(u32, frost::PubNonce)> = Vec::new();
    for id in &signers {
        let key = pubshare(*id)?;
        let entry = records
            .iter()
            .find(|e| e.pubshare == key)
            .ok_or_else(|| refuse(crate::threshold::Refusal::SignerSet))?;
        nonces.push((*id, entry.value));
    }
    let mut psigs: Vec<(u32, frost::PartialSig)> = Vec::new();
    for id in &ti.signed {
        let key = pubshare(*id)?;
        if let Some(entry) = sig_records.iter().find(|e| e.pubshare == key) {
            psigs.push((*id, entry.value));
        }
    }

    // The secret nonces this pass can sign with: the ones the carry file
    // brought, copied out of it so the section the caller holds is not
    // disturbed, plus the ones drawn here.
    let mut secrets: Vec<(u32, frost::SecNonce)> = Vec::new();
    if let Some(section) = carry {
        for id in &signers {
            let key = pubshare(*id)?;
            let Some(entry) = section.entry(index, &key) else {
                continue;
            };
            if entry.sighash != msg {
                return Err(refuse(crate::threshold::Refusal::AnotherTransaction));
            }
            let mut bytes = entry.secnonce.serialize();
            let nonce = frost::SecNonce::from_bytes(&bytes);
            bytes.zeroize();
            let nonce = nonce.map_err(|_| Error::Unsupported {
                input: index,
                reason: "the file holds a secret nonce that is out of range",
            })?;
            if nonce
                .public_nonce(&secp)
                .ok()
                .zip(nonces.iter().find(|(i, _)| i == id).map(|(_, n)| *n))
                .is_none_or(|(a, b)| a != b)
            {
                return Err(refuse(crate::threshold::Refusal::NonceMismatch));
            }
            secrets.push((*id, nonce));
        }
    }

    let mine: Vec<u32> = ti.ours.clone();
    let mut entries: Vec<crate::threshold::CarryEntry> = Vec::new();

    // The first location: the signer set is chosen here and every
    // signer's nonce drawn from this device's seed.
    if signers.is_empty() {
        let Some(me) = mine.first().copied() else {
            return Ok(ThresholdPass {
                signatures: Vec::new(),
                entries,
                signers: Vec::new(),
                partial_sigs: 0,
            });
        };
        if carry.is_some() {
            return Err(refuse(crate::threshold::Refusal::NoNonces));
        }
        let mut chosen: Vec<u32> = alloc::vec![me];
        for id in others {
            if *id == me
                || chosen.contains(id)
                || *id as usize >= ti.info.n()
                || ti.pubshare(*id).is_none()
            {
                return Err(refuse(crate::threshold::Refusal::ChosenCount));
            }
            chosen.push(*id);
        }
        if chosen.len() != ti.info.t {
            return Err(refuse(crate::threshold::Refusal::ChosenCount));
        }
        chosen.sort_unstable();
        let thresh_pk_xonly = ti.output_key.x_only_public_key().0.serialize();
        // The signer set is in every `rand`: a seed reused on this
        // transaction with another cosigner chosen would otherwise draw
        // our nonce again for a different signature.
        let set: Vec<u8> = chosen.iter().flat_map(|id| id.to_le_bytes()).collect();
        for id in &chosen {
            let key = pubshare(*id)?;
            let rand = crate::musig::tagged(
                THRESHOLD_RAND_TAG,
                &[seed, &(index as u32).to_le_bytes(), &key.serialize(), &set],
            );
            // Our own nonce takes this share's secret; every other
            // signer's takes none, and `extra_in` is none for all of
            // them: the other nonces travel in a file, and mixing this
            // share's secret into one would open a path from the share
            // to the stick for no gain (§16.103).
            let drawn = match mine.contains(id) {
                true => with_share(shares, &key, index, |secshare| {
                    frost::nonce_gen(
                        &secp,
                        &rand,
                        Some(secshare),
                        Some(&key),
                        Some(&thresh_pk_xonly),
                        Some(&msg),
                        None,
                    )
                })?,
                false => frost::nonce_gen(
                    &secp,
                    &rand,
                    None,
                    Some(&key),
                    Some(&thresh_pk_xonly),
                    Some(&msg),
                    None,
                ),
            };
            let (secnonce, pubnonce) = drawn.map_err(|_| Error::Unsupported {
                input: index,
                reason: "a threshold nonce could not be drawn for this input",
            })?;
            let here = &mut psbt.inner_mut().inputs[index];
            crate::threshold::write_pub_nonce(here, &key, &ti.output_key, &pubnonce);
            nonces.push((*id, pubnonce));
            secrets.push((*id, secnonce));
        }
        signers = chosen;
    }

    // Every location: sign for each share this device holds that is a
    // signer and has not signed yet.
    let mut written = Vec::new();
    let ordered =
        |nonces: &[(u32, frost::PubNonce)], signers: &[u32]| -> Option<Vec<frost::PubNonce>> {
            signers
                .iter()
                .map(|id| nonces.iter().find(|(i, _)| i == id).map(|(_, n)| *n))
                .collect()
        };
    let mut established = ti.clone();
    established.signers = signers.clone();
    for id in &mine {
        if !signers.contains(id) || psigs.iter().any(|(i, _)| i == id) {
            continue;
        }
        let key = pubshare(*id)?;
        let at = secrets
            .iter()
            .position(|(i, _)| i == id)
            .ok_or_else(|| refuse(crate::threshold::Refusal::AnotherShare))?;
        let (_, secnonce) = secrets.remove(at);
        let all = ordered(&nonces, &signers)
            .ok_or_else(|| refuse(crate::threshold::Refusal::SignerSet))?;
        let session_ctx = established
            .session_ctx(&secp, &all, &msg)
            .ok_or(Error::Sighash { input: index })?;
        let values = osk_bip::frost::session_values(&secp, &session_ctx)
            .map_err(|_| Error::Sighash { input: index })?;
        // Every partial signature already on the input, verified before
        // this share adds its own: a substituted nonce shows up here,
        // and cannot be recomputed without the share that made it.
        for (other, psig) in &psigs {
            let other_key = pubshare(*other)?;
            let nonce = nonces
                .iter()
                .find(|(i, _)| i == other)
                .map(|(_, n)| *n)
                .ok_or_else(|| refuse(crate::threshold::Refusal::SignerSet))?;
            frost::partial_sig_verify_internal(&secp, psig, *other, &nonce, &other_key, &values)
                .map_err(|_| refuse(crate::threshold::Refusal::PartialSig))?;
        }
        let psig = with_share(shares, &key, index, |secshare| {
            frost::sign(&secp, secnonce, secshare, *id, &session_ctx)
        })?
        .map_err(|_| Error::VerifyFailed { input: index })?;
        let here = &mut psbt.inner_mut().inputs[index];
        crate::threshold::write_partial_sig(here, &key, &ti.output_key, &psig);
        psigs.push((*id, psig));
        written.push(InputSignature {
            index,
            fingerprint: osk_bip::threshold::share_fingerprint(&key),
            kind: SigKind::Schnorr,
            sig_bytes: psig.serialize().to_vec(),
            verified: true,
            deterministic_ok: true,
            musig: Some(MusigRole::Partial),
        });
    }

    // Whatever is still to sign travels on: one secret nonce per signer
    // that has not signed, bound to this input and this sighash.
    for (id, secnonce) in secrets.drain(..) {
        if psigs.iter().any(|(i, _)| *i == id) {
            continue;
        }
        entries.push(crate::threshold::CarryEntry {
            input: index as u32,
            pubshare: pubshare(id)?,
            sighash: msg,
            secnonce,
        });
    }

    // The last signer aggregates: the BIP-340 signature is checked
    // against the output key before it is written, and finalizing is
    // then any key-path spend's.
    if psigs.len() == signers.len() && !signers.is_empty() {
        let all = ordered(&nonces, &signers)
            .ok_or_else(|| refuse(crate::threshold::Refusal::SignerSet))?;
        let values = established
            .session(&secp, &all, &msg)
            .ok_or(Error::Sighash { input: index })?;
        let mut sigs = Vec::with_capacity(signers.len());
        for id in &signers {
            let key = pubshare(*id)?;
            let nonce = nonces
                .iter()
                .find(|(i, _)| i == id)
                .map(|(_, n)| *n)
                .ok_or_else(|| refuse(crate::threshold::Refusal::SignerSet))?;
            let psig = psigs
                .iter()
                .find(|(i, _)| i == id)
                .map(|(_, s)| *s)
                .ok_or_else(|| refuse(crate::threshold::Refusal::PartialSig))?;
            frost::partial_sig_verify_internal(&secp, &psig, *id, &nonce, &key, &values)
                .map_err(|_| refuse(crate::threshold::Refusal::PartialSig))?;
            sigs.push(psig);
        }
        let sig =
            frost::partial_sig_agg(&sigs, &values).map_err(|_| Error::Sighash { input: index })?;
        let signature = secp256k1::schnorr::Signature::from_slice(&sig)
            .map_err(|_| Error::VerifyFailed { input: index })?;
        let output_key = ti.output_key.x_only_public_key().0;
        secp.verify_schnorr(&signature, &Message::from(sighash), &output_key)
            .map_err(|_| Error::VerifyFailed { input: index })?;
        psbt.inner_mut().inputs[index].tap_key_sig = Some(taproot::Signature {
            signature,
            sighash_type,
        });
        written.push(InputSignature {
            index,
            fingerprint: osk_bip::threshold::share_fingerprint(&pubshare(signers[0])?),
            kind: SigKind::Schnorr,
            sig_bytes: sig.to_vec(),
            verified: true,
            deterministic_ok: true,
            musig: Some(MusigRole::Aggregated),
        });
    }

    let mut signer_keys = Vec::with_capacity(signers.len());
    for id in &signers {
        signer_keys.push(pubshare(*id)?);
    }
    Ok(ThresholdPass {
        signatures: written,
        entries,
        signers: signer_keys,
        partial_sigs: psigs.len(),
    })
}

/// Runs `f` on the secret share behind `key`, which is the only way a
/// share's scalar is reachable.
fn with_share<R>(
    shares: &[ShareKey],
    key: &secp256k1::PublicKey,
    index: usize,
    f: impl FnOnce(&osk_bip::frost::SecShare) -> R,
) -> Result<R, Error> {
    let share = shares
        .iter()
        .find(|s| s.pubshare == *key)
        .ok_or(Error::Unsupported {
            input: index,
            reason: "this device holds no share for a signer of this input",
        })?;
    let mut out = None;
    let mut once = Some(f);
    (share.open)(&mut |secshare| {
        if let Some(f) = once.take() {
            out = Some(f(secshare));
        }
    });
    out.ok_or(Error::Unsupported {
        input: index,
        reason: "this device's share could not be opened",
    })
}
