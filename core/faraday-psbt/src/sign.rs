//! Signing: the only place in the core where a private key is used.
//!
//! Per input, the key is derived from the PSBT's own key-origin data
//! (fingerprint and path in `bip32_derivation` or `tap_key_origins`),
//! never from a guessed path, and checked against the public key the PSBT
//! names and against the script being spent before any signature is made.
//!
//! Nonces: ECDSA uses
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
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::Zeroizing;
use faraday_crypto::work::{Compute, Work};
use miniscript::{ExtParams, Miniscript, SigType, Tap, ToPublicKey};

use crate::classify::{Classified, ScriptKind, classify, spent_output};
use crate::context::Context;
use crate::finalize::{is_complete, key_in_script};
use crate::inspect::{
    MusigRound, WarningKind, inspect, musig_keyed_here, musig_round, signed_through_wallet,
};
use crate::musig::MusigSession;
use crate::psbt::{Error, Psbt};
use crate::reason::Reason;

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
            // not shown: Debug's text, never printed for people
            Aux::Deterministic => f.write_str("Aux::Deterministic"),
            // not shown: Debug's text, never printed for people
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
}

/// Costs what `inspect`, `sign_ecdsa`, `sign_musig` and the rest it
/// calls cost. Signs every input that names one of the `selection`
/// fingerprints in its key origins, using the matching master key from
/// `keys`, the RFC 6979 nonce `nonce` names and the Schnorr auxiliary
/// randomness `aux` carries.
///
/// Refuses (before touching the PSBT) when `accepted` names a kind no
/// hold accepts ([`WarningKind::is_holdable`]); when the inspection has
/// any blocked warning; when it has a danger warning `accepted` does not
/// name, which is every danger but a holdable one the person held past
/// (F136); when a selected fingerprint has no master key;
/// and when an input that names a selected key spends a script this crate
/// cannot sign. A SegWit v0 input with no previous transaction, in a
/// transaction with more than one input, is refused in both places: the
/// block, and [`Error::Unsupported`] on the input itself. The same input
/// alone in its transaction is signed from its `witness_utxo`, because
/// the fee attack the refusal stops needs a second signed input to
/// combine with. A key-origin claim that the derived key does not match is
/// [`Error::KeyMismatch`]. Inputs that are already final, or name none of
/// the selected keys, are left alone, and so is a miniscript or a taproot
/// tree that no registered wallet gives (F141): inspection's
/// `NoWalletForScript` names it.
///
/// `seed` is where every MuSig2 nonce this pass draws
/// comes from, and it must be fresh, uniformly random bytes for every
/// call: a seed used twice on one transaction can draw one secret nonce
/// for two different signatures, which gives away the key. A MuSig2
/// session takes the seed of the call that opens it and moves it forward
/// after every draw, so nonces drawn while it stays open never repeat;
/// a new session for the same transaction needs a new seed. Faraday
/// derives one per signing press from its session key and a count of
/// the draws.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn sign(
    c: &Compute,
    psbt: &mut Psbt,
    keys: &[&MasterKey],
    selection: &[Fingerprint],
    ctx: &Context,
    accepted: &[WarningKind],
    nonce: Nonce,
    aux: Aux,
    session: &mut Option<MusigSession>,
    seed: &Zeroizing<[u8; 32]>,
) -> Result<SignResult, Error> {
    c.count(Work::Sign);
    if let Some(kind) = accepted.iter().find(|k| !k.is_holdable()) {
        return Err(Error::NotHoldable(kind.clone()));
    }
    let inspection = inspect(c, psbt, ctx);
    if inspection.has_blocked() {
        return Err(Error::Blocked(inspection.blocked()));
    }
    let dangers = inspection.dangers_left(accepted);
    if !dangers.is_empty() {
        return Err(Error::Danger(dangers));
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

    for i in 0..psbt.inner().inputs.len() {
        let input = &psbt.inner().inputs[i];
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            continue;
        }
        let cl = classify(psbt.inner(), i);
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
        if let Some(problem) = cl.problems.first() {
            return Err(Error::Unsupported {
                input: i,
                reason: *problem,
            });
        }
        if cl.utxo.is_none() {
            return Err(Error::MissingUtxo { input: i });
        }
        if cl.amount_unverified {
            return Err(Error::Unsupported {
                input: i,
                reason: Reason::NoPrevTx,
            });
        }
        // A miniscript or a taproot tree is signed only through a
        // registered wallet whose script it is, which is inspection's
        // answer: one it does not count as the person's is left
        // unsigned, as its review says (F135, F141).
        if signed_through_wallet(cl.kind) && !inspection.inputs[i].is_ours {
            continue;
        }

        match cl.kind {
            ScriptKind::P2pkh
            | ScriptKind::P2shP2wpkh
            | ScriptKind::P2wpkh
            | ScriptKind::Multisig { .. }
            | ScriptKind::Miniscript { .. } => {
                for (pk, fp, path, master) in ecdsa_claims {
                    let sig = sign_ecdsa(c, psbt, &mut cache, i, &cl, pk, &path, master, nonce)?;
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
                    c, psbt, &mut cache, i, &cl, musig, &mine, &selected, aux, txid, session, seed,
                )?;
                nonces_shared += done.nonces_shared;
                signed_inputs.extend(done.signatures);
            }
            ScriptKind::P2trKey => {
                for (xonly, fp, path, master) in tap_claims {
                    let sig =
                        sign_taproot(c, psbt, &mut cache, i, &cl, xonly, &path, master, aux, None)?;
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
                            c, psbt, &mut cache, i, &cl, *xonly, *leaf_hash, path, master, aux,
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
                let spk = &cl.utxo.as_ref().expect("checked by caller").script_pubkey;
                for (xonly, fp, path, master) in tap_claims {
                    if psbt.inner().inputs[i].tap_internal_key != Some(xonly)
                        || ScriptBuf::new_p2tr(master.secp(), xonly, merkle) != *spk
                    {
                        continue;
                    }
                    let sig = sign_taproot(
                        c, psbt, &mut cache, i, &cl, xonly, &path, master, aux, merkle,
                    )?;
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
                        reason: Reason::NoTaprootPathOurs,
                    });
                }
            }
            ScriptKind::Unknown => {
                return Err(Error::Unsupported {
                    input: i,
                    reason: Reason::UnknownScript,
                });
            }
        }
    }

    Ok(SignResult {
        complete: is_complete(psbt),
        signed_inputs,
        nonces_shared,
    })
}

/// Costs a key derivation (through `derive` and `to_xpub`). Derives the
/// key at `path`, checks it is the key the PSBT names and that it
/// appears in the script, then signs and verifies. The derived key
/// lives only in this frame.
///
/// The curve context is the master key's own, blinded by the session
/// (`faraday_bip::keys`, security review M1), not one made here.
#[allow(clippy::too_many_arguments)]
fn sign_ecdsa(
    c: &Compute,
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    cl: &Classified,
    pk: secp256k1::PublicKey,
    path: &DerivationPath,
    master: &MasterKey,
    nonce: Nonce,
) -> Result<ecdsa::Signature, Error> {
    let secp = master.secp();
    let derived = master.derive(c, path);
    if derived.to_xpub(c).public_key != pk {
        return Err(Error::KeyMismatch { input: index });
    }
    let input = &psbt.inner().inputs[index];
    if !key_in_script(cl, input, &PublicKey::new(pk)) {
        return Err(Error::KeyMismatch { input: index });
    }
    if spent_output(psbt.inner(), index).is_none() {
        return Err(Error::MissingUtxo { input: index });
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

/// Costs a key derivation (through `derive` and `to_xpub`). BIP-341
/// script-path signing: the derived key must be the one the PSBT names
/// for this leaf, and the signature commits to the leaf's hash. It is
/// verified against the key itself, which is what a script-path
/// signature is checked against — no tweak.
///
/// The curve context is the master key's own, as in [`sign_ecdsa`].
#[allow(clippy::too_many_arguments)]
fn sign_tap_leaf(
    c: &Compute,
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    cl: &Classified,
    xonly: XOnlyPublicKey,
    leaf_hash: TapLeafHash,
    path: &DerivationPath,
    master: &MasterKey,
    aux: Aux,
) -> Result<taproot::Signature, Error> {
    let secp = master.secp();
    let derived = master.derive(c, path);
    if derived.to_xpub(c).public_key.x_only_public_key().0 != xonly {
        return Err(Error::KeyMismatch { input: index });
    }
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index)?;
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
    cl: &Classified,
    index: usize,
) -> Result<(TapSighashType, PrevoutSet), Error> {
    let input = &psbt.inner().inputs[index];
    let sighash_type = match input.sighash_type {
        None => TapSighashType::Default,
        Some(t) => t
            .taproot_hash_ty()
            .map_err(|_| Error::Sighash { input: index })?,
    };
    Ok((sighash_type, tap_prevouts(psbt, cl, index, sighash_type)?))
}

/// The prevouts a taproot sighash of type `sighash_type` commits to.
/// Split out so that [`crate::verify`] checks a signature that arrived
/// against the prevouts this crate would have signed over, under the
/// sighash type the signature itself states.
pub(crate) fn tap_prevouts(
    psbt: &Psbt,
    cl: &Classified,
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
        let utxo = cl.utxo.as_ref().expect("checked by caller").clone();
        return Ok(PrevoutSet::One(index, utxo));
    }
    let mut prevouts: Vec<TxOut> = Vec::with_capacity(psbt.inner().inputs.len());
    for j in 0..psbt.inner().inputs.len() {
        prevouts.push(
            spent_output(psbt.inner(), j)
                .cloned()
                .ok_or(Error::MissingUtxo { input: j })?,
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

/// Costs a key derivation (through `derive` and `to_xpub`). BIP-86 and
/// BIP-341 key-path signing: the derived key must be the input's
/// internal key, the output must be that key tweaked by `merkle_root`
/// (nothing for a key-only wallet, the tree's root for a `tr()` with
/// leaves), and the signature is verified against the tweaked output
/// key. The auxiliary randomness is `aux`'s value for this input.
///
/// The curve context is the master key's own, as in [`sign_ecdsa`].
#[allow(clippy::too_many_arguments)]
fn sign_taproot(
    c: &Compute,
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    cl: &Classified,
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
        reason: Reason::NoInternalKey,
    })?;
    let utxo = cl.utxo.as_ref().expect("checked by caller");
    if internal != xonly || ScriptBuf::new_p2tr(secp, internal, merkle_root) != utxo.script_pubkey {
        return Err(Error::KeyMismatch { input: index });
    }
    let derived = master.derive(c, path);
    if derived.to_xpub(c).public_key.x_only_public_key().0 != xonly {
        return Err(Error::KeyMismatch { input: index });
    }

    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index)?;
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

/// Costs a key derivation (through `derive` and `to_xpub`). Signing a
/// BIP-373 MuSig2 input.
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
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn sign_musig(
    c: &Compute,
    psbt: &mut Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    index: usize,
    cl: &Classified,
    musig: &crate::inspect::MusigInput,
    mine: &[&crate::inspect::MusigParticipation],
    selected: &[(Fingerprint, &MasterKey)],
    aux: Aux,
    txid: bitcoin::Txid,
    session: &mut Option<MusigSession>,
    seed: &[u8; 32],
) -> Result<MusigPass, Error> {
    let secp = secp256k1::Secp256k1::new();
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index)?;
    if sighash_type != TapSighashType::Default {
        return Err(Error::Unsupported {
            input: index,
            reason: Reason::MusigSighashDefault,
        });
    }
    let sighash = cache
        .taproot_key_spend_signature_hash(index, &prevouts.as_ref(), sighash_type)
        .map_err(|_| Error::Sighash { input: index })?;
    let msg: [u8; 32] = sighash.to_byte_array();

    // Every nonce and partial signature the input already carries,
    // whichever of the three aggregate forms its key names.
    let input = &psbt.inner().inputs[index];
    let mut nonces: Vec<(secp256k1::PublicKey, faraday_bip::musig::PubNonce)> =
        crate::musig::pub_nonces(input)
            .map_err(|e| Error::Unsupported {
                input: index,
                reason: Reason::Musig(e),
            })?
            .into_iter()
            .filter(|e| musig_keyed_here(musig, &e.aggregate))
            .map(|e| (e.participant, e.value))
            .collect();
    let mut psigs: Vec<(secp256k1::PublicKey, faraday_bip::musig::PartialSig)> =
        crate::musig::partial_sigs(input)
            .map_err(|e| Error::Unsupported {
                input: index,
                reason: Reason::Musig(e),
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

    // A nonce shared clears every partial signature on the input, since
    // they were made under the aggregate nonce it changes. The rounds
    // that share go first, so a signature this pass makes is made under
    // the nonces the input ends with and stays on it, and what the pass
    // reports is what the PSBT holds.
    let order = mine
        .iter()
        .zip(&rounds)
        .filter(|(_, r)| matches!(r, MusigRound::Share { .. }))
        .chain(
            mine.iter()
                .zip(&rounds)
                .filter(|(_, r)| !matches!(r, MusigRound::Share { .. })),
        );
    let mut written = Vec::new();
    let mut nonces_shared = 0usize;
    for (participation, round) in order {
        let (_, master) = selected
            .iter()
            .find(|(f, _)| *f == participation.fingerprint)
            .expect("the caller selected this key");
        let derived = master.derive(c, &participation.path);
        if derived.to_xpub(c).public_key != participation.key {
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
                let (secnonce, pubnonce) = faraday_bip::musig::nonce_gen(
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
                    reason: Reason::MusigNonceNotDrawn,
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
                let psig =
                    faraday_bip::musig::sign(signer, secnonce, derived.secret_key(), &values)
                        .map_err(|_| Error::Sighash { input: index })?;
                faraday_bip::musig::partial_sig_verify_internal(
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
                let others: Vec<faraday_bip::musig::PubNonce> = musig
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
                                reason: Reason::MusigNonceMissing,
                            })
                    })
                    .collect::<Result<_, _>>()?;
                let aggothernonce =
                    faraday_bip::musig::nonce_agg_with(&secp, &others).map_err(|_| {
                        Error::Unsupported {
                            input: index,
                            reason: Reason::MusigOtherNonces,
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
                    faraday_bip::musig::deterministic_sign(
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
                faraday_bip::musig::partial_sig_verify_internal(
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
    let all: Vec<faraday_bip::musig::PubNonce> = musig
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
    let aggnonce =
        faraday_bip::musig::nonce_agg_with(&secp, &all).map_err(|_| Error::Unsupported {
            input: index,
            reason: Reason::MusigNonces,
        })?;
    let values = faraday_bip::musig::session_values(
        &secp,
        &musig.participants,
        &musig.tweaks,
        &aggnonce,
        &msg,
    )
    .map_err(|_| Error::Sighash { input: index })?;
    for (pk, psig) in &psigs {
        let Some((_, nonce)) = nonces.iter().find(|(n, _)| n == pk) else {
            continue;
        };
        if faraday_bip::musig::partial_sig_verify_internal(&secp, psig, nonce, pk, &values).is_err()
        {
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

    let ordered: Vec<faraday_bip::musig::PartialSig> = musig
        .participants
        .iter()
        .filter_map(|p| psigs.iter().find(|(pk, _)| pk == p).map(|(_, s)| *s))
        .collect();
    let sig = faraday_bip::musig::partial_sig_agg(&ordered, &values)
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
    nonces: &[(secp256k1::PublicKey, faraday_bip::musig::PubNonce)],
    mine: &faraday_bip::musig::PubNonce,
    key: secp256k1::PublicKey,
    msg: &[u8],
) -> Option<faraday_bip::musig::SessionValues> {
    let all: Vec<faraday_bip::musig::PubNonce> = musig
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
    let aggnonce = faraday_bip::musig::nonce_agg_with(secp, &all).ok()?;
    faraday_bip::musig::session_values(secp, &musig.participants, &musig.tweaks, &aggnonce, msg)
        .ok()
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
