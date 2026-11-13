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

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use bitcoin::bip32::DerivationPath;
use bitcoin::hashes::Hash;
use bitcoin::key::TapTweak;
use bitcoin::secp256k1::{self, Keypair, Message, XOnlyPublicKey};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{TapLeafHash, TapNodeHash};
use bitcoin::{PublicKey, ScriptBuf, TxOut, ecdsa, taproot};
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::Zeroizing;
use faraday_crypto::work::{Compute, Work};
use miniscript::{ExtParams, Miniscript, SigType, Tap, ToPublicKey};

use crate::classify::{Classified, ScriptKind, classify, legacy_code_separator, spent_output};
use crate::context::Context;
use crate::finalize::{is_complete, key_in_script};
use crate::inspect::{MusigRound, WarningKind, inspect, musig_round, signed_through_wallet};
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
/// The app passes [`Aux::Deterministic`] for every signature (F156):
/// no caller draws fresh bytes today. A caller that passes
/// [`Aux::Fresh`] draws them once per call, new for each call, and
/// [`sign`] mixes the input's index in so no two inputs of one call
/// share them.
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
    /// deterministic mode, and otherwise HKDF-Expand (RFC 5869) from this
    /// call's bytes, with `info` the label and the index (eight bytes,
    /// little-endian), so that two inputs of one call never sign under
    /// the same auxiliary randomness.
    pub(crate) fn for_input(self, index: usize) -> [u8; 32] {
        match self {
            Aux::Deterministic => [0u8; 32],
            Aux::Fresh(bytes) => {
                let mut info = [0u8; AUX_INFO.len() + 8];
                info[..AUX_INFO.len()].copy_from_slice(AUX_INFO);
                info[AUX_INFO.len()..].copy_from_slice(&(index as u64).to_le_bytes());
                let mut aux = [0u8; 32];
                faraday_crypto::hkdf_sha256_expand(&bytes, &info, &mut aux)
                    .expect("32 bytes is one block");
                aux
            }
        }
    }
}

/// The HKDF-Expand `info` label the per-input auxiliary randomness is
/// drawn under, before the input's index.
const AUX_INFO: &[u8] = b"faraday-schnorr-aux";

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

/// A MuSig2 public nonce a pass wrote for one of this device's keys: the
/// input, the participant key, the aggregate key it is keyed by and the
/// 66-byte nonce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonceWritten {
    /// The input it is on.
    pub input: usize,
    /// The participant key it is for.
    pub participant: secp256k1::PublicKey,
    /// The aggregate key the entry is keyed by, as the signer wrote it.
    pub aggregate: secp256k1::PublicKey,
    /// The public nonce, serialized.
    pub nonce: [u8; 66],
}

/// What [`sign`] did.
#[derive(Debug, PartialEq, Eq)]
pub struct SignResult {
    /// Signatures written, in input order.
    pub signed_inputs: Vec<InputSignature>,
    /// Public nonces this pass drew, wrote and kept in the session:
    /// round 1 of §16.100, one per input and participant.
    pub nonces_shared: usize,
    /// The public nonces this pass wrote into the PSBT for this
    /// device's own keys (a round 1 draw, or the kept nonce a mend
    /// puts back): what a version 2 file's older nonce of this device's
    /// is replaced by (`PsbtV2::merge_signed_by`).
    pub nonces_written: Vec<NonceWritten>,
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
/// the draws. With no seed, a pass that would draw a nonce is refused
/// ([`Error::NoNonceSeed`]), never drawn from a fixed one.
#[allow(
    clippy::too_many_arguments,
    reason = "the signing request: PSBT, keys, selection, context, accepted warnings, nonce, aux and MuSig session"
)]
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
    seed: Option<&Zeroizing<[u8; 32]>>,
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

    // A legacy input whose script code holds OP_CODESEPARATOR, named
    // by a selected key, refuses the whole PSBT before anything is
    // signed (INTEROP §5, Sweeping a private key).
    for (i, input) in psbt.inner().inputs.iter().enumerate() {
        let names_selected = input
            .bip32_derivation
            .values()
            .any(|(fp, _)| selected.iter().any(|(f, _)| *f == Fingerprint::from(*fp)));
        if names_selected && legacy_code_separator(psbt.inner(), i) {
            return Err(Error::Unsupported {
                input: i,
                reason: Reason::CodeSeparator,
            });
        }
    }

    let tx = psbt.inner().unsigned_tx.clone();
    let txid = tx.compute_txid();
    let mut cache = SighashCache::new(&tx);
    // Every input's taproot sighash reads the same outputs: worked out once.
    let outputs = Outputs::default();
    let mut signed_inputs = Vec::new();
    let mut nonces_shared = 0usize;
    let mut nonces_written = Vec::new();

    // A refusal on any MuSig2 input comes before any input takes a kept
    // nonce, so a pass that refuses burns none, whatever input it
    // refuses on.
    for i in 0..psbt.inner().inputs.len() {
        let input = &psbt.inner().inputs[i];
        if input.final_script_sig.is_some() || input.final_script_witness.is_some() {
            continue;
        }
        let Some(musig) = inspection.inputs[i].musig.as_ref() else {
            continue;
        };
        let mine: Vec<&crate::inspect::MusigParticipation> = musig
            .ours
            .iter()
            .filter(|p| selected.iter().any(|(f, _)| *f == p.fingerprint))
            .collect();
        if mine.is_empty() {
            continue;
        }
        let cl = classify(psbt.inner(), i);
        if cl.kind == ScriptKind::P2trKey {
            musig_precheck(
                c, psbt, &mut cache, &outputs, i, &cl, musig, &mine, txid, session,
            )?;
        }
    }

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
                    c,
                    psbt,
                    &mut cache,
                    &outputs,
                    i,
                    &cl,
                    musig,
                    &mine,
                    &selected,
                    aux,
                    txid,
                    session,
                    seed.map(|s| &**s),
                )?;
                nonces_shared += done.nonces_shared;
                nonces_written.extend(done.nonces_written.into_iter().map(
                    |(participant, nonce)| NonceWritten {
                        input: i,
                        participant,
                        aggregate: musig.output_key,
                        nonce,
                    },
                ));
                signed_inputs.extend(done.signatures);
            }
            ScriptKind::P2trKey => {
                for (xonly, fp, path, master) in tap_claims {
                    let sig = sign_taproot(
                        c, psbt, &mut cache, &outputs, i, &cl, xonly, &path, master, aux, None,
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
                            c, psbt, &mut cache, &outputs, i, &cl, *xonly, *leaf_hash, path,
                            master, aux,
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
                        c, psbt, &mut cache, &outputs, i, &cl, xonly, &path, master, aux, merkle,
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
        nonces_written,
    })
}

/// Costs a key derivation (through `derive` and `to_xpub`). Derives the
/// key at `path`, checks it is the key the PSBT names and that it
/// appears in the script, then signs and verifies. The derived key
/// lives only in this frame.
///
/// The curve context is the master key's own, blinded by the session
/// (`faraday_bip::keys`, security review M1), not one made here.
#[allow(
    clippy::too_many_arguments,
    reason = "one ECDSA input's sighash cache, index, classification, key, path and master key"
)]
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
#[allow(
    clippy::too_many_arguments,
    reason = "one taproot script-path input's sighash cache, index, classification, key, leaf hash, path and master key"
)]
fn sign_tap_leaf(
    c: &Compute,
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    outputs: &Outputs,
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
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index, outputs)?;
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
/// otherwise (from `outputs`, worked out once for the pass).
fn tap_sighash_inputs<'a>(
    psbt: &Psbt,
    cl: &Classified,
    index: usize,
    outputs: &'a Outputs,
) -> Result<(TapSighashType, PrevoutSet<'a>), Error> {
    let input = &psbt.inner().inputs[index];
    let sighash_type = match input.sighash_type {
        None => TapSighashType::Default,
        Some(t) => t
            .taproot_hash_ty()
            .map_err(|_| Error::Sighash { input: index })?,
    };
    Ok((
        sighash_type,
        tap_prevouts_from(outputs.of(psbt.inner()), cl, index, sighash_type)?,
    ))
}

/// The output every input of a PSBT spends, or the first input that
/// states none: what a taproot sighash that commits to all inputs
/// reads. The same for every input of a PSBT, so a pass works it out once.
pub(crate) type AllOutputs = Result<Vec<TxOut>, usize>;

/// Costs a txid per input that carries a previous transaction. The
/// output every input of `psbt` spends ([`AllOutputs`]).
pub(crate) fn all_outputs(psbt: &bitcoin::Psbt) -> AllOutputs {
    (0..psbt.inputs.len())
        .map(|j| spent_output(psbt, j).cloned().ok_or(j))
        .collect()
}

/// [`AllOutputs`] of one PSBT, worked out when first asked for and kept
/// for the rest of a pass over it.
#[derive(Default)]
pub(crate) struct Outputs(core::cell::OnceCell<AllOutputs>);

impl Outputs {
    /// Costs what [`all_outputs`] costs, on the first call only.
    pub(crate) fn of(&self, psbt: &bitcoin::Psbt) -> &AllOutputs {
        self.0.get_or_init(|| all_outputs(psbt))
    }
}

/// Costs [`all_outputs`] unless the type is `ANYONECANPAY`. The prevouts a
/// taproot sighash of type `sighash_type` commits to. Split out so that
/// [`crate::verify`] checks a signature that arrived against the prevouts
/// this crate would have signed over, under the sighash type the signature
/// itself states. A pass over many inputs uses [`tap_prevouts_from`].
pub(crate) fn tap_prevouts(
    psbt: &bitcoin::Psbt,
    cl: &Classified,
    index: usize,
    sighash_type: TapSighashType,
) -> Result<PrevoutSet<'static>, Error> {
    if anyone_can_pay(sighash_type) {
        return one_prevout(cl, index);
    }
    match all_outputs(psbt) {
        Ok(all) => Ok(PrevoutSet::All(Cow::Owned(all))),
        Err(input) => Err(Error::MissingUtxo { input }),
    }
}

/// [`tap_prevouts`] over `all`, the outputs of the whole PSBT worked out
/// once, copying none of them.
pub(crate) fn tap_prevouts_from<'a>(
    all: &'a AllOutputs,
    cl: &Classified,
    index: usize,
    sighash_type: TapSighashType,
) -> Result<PrevoutSet<'a>, Error> {
    if anyone_can_pay(sighash_type) {
        return one_prevout(cl, index);
    }
    match all {
        Ok(all) => Ok(PrevoutSet::All(Cow::Borrowed(all))),
        Err(input) => Err(Error::MissingUtxo { input: *input }),
    }
}

fn anyone_can_pay(sighash_type: TapSighashType) -> bool {
    matches!(
        sighash_type,
        TapSighashType::AllPlusAnyoneCanPay
            | TapSighashType::NonePlusAnyoneCanPay
            | TapSighashType::SinglePlusAnyoneCanPay
    )
}

fn one_prevout(cl: &Classified, index: usize) -> Result<PrevoutSet<'static>, Error> {
    let utxo = cl.utxo.clone().ok_or(Error::MissingUtxo { input: index })?;
    Ok(PrevoutSet::One(index, utxo))
}

/// The prevouts a taproot sighash commits to, owned or borrowed so that
/// the caller can hand out the borrowed `Prevouts` the sighash cache wants.
pub(crate) enum PrevoutSet<'a> {
    One(usize, TxOut),
    All(Cow<'a, [TxOut]>),
}

impl PrevoutSet<'_> {
    pub(crate) fn as_ref(&self) -> Prevouts<'_, TxOut> {
        match self {
            PrevoutSet::One(index, utxo) => Prevouts::One(*index, utxo.clone()),
            PrevoutSet::All(all) => Prevouts::All(all.as_ref()),
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
#[allow(
    clippy::too_many_arguments,
    reason = "one taproot key-path input's sighash cache, index, classification, key, path and master key"
)]
fn sign_taproot(
    c: &Compute,
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    outputs: &Outputs,
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

    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index, outputs)?;
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
    /// The (participant key, public nonce) pairs this pass wrote for
    /// keys of `musig.ours` that passed the derivation check.
    nonces_written: Vec<(secp256k1::PublicKey, [u8; 66])>,
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
#[allow(
    clippy::too_many_arguments,
    reason = "one MuSig2 input's PSBT, sighash cache, index, classification, participants and session state"
)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn sign_musig(
    c: &Compute,
    psbt: &mut Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    outputs: &Outputs,
    index: usize,
    cl: &Classified,
    musig: &crate::inspect::MusigInput,
    mine: &[&crate::inspect::MusigParticipation],
    selected: &[(Fingerprint, &MasterKey)],
    aux: Aux,
    txid: bitcoin::Txid,
    session: &mut Option<MusigSession>,
    seed: Option<&[u8; 32]>,
) -> Result<MusigPass, Error> {
    // The first selected key's own blinded context, not a plain one.
    let secp = selected
        .first()
        .map_or_else(secp256k1::Secp256k1::new, |(_, m)| m.secp().clone());
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index, outputs)?;
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
        crate::musig::picked(
            &crate::musig::pub_nonces(input).map_err(|e| Error::Unsupported {
                input: index,
                reason: Reason::Musig(e),
            })?,
            musig,
        );
    let mut psigs: Vec<(secp256k1::PublicKey, faraday_bip::musig::PartialSig)> =
        crate::musig::picked(
            &crate::musig::partial_sigs(input).map_err(|e| Error::Unsupported {
                input: index,
                reason: Reason::Musig(e),
            })?,
            musig,
        );

    // Which way each participant goes, read off the input before this
    // pass writes anything to it.
    let view = session.as_ref().map(|s| s.view());
    let rounds: Vec<MusigRound> = mine
        .iter()
        .map(|p| musig_round(c, psbt, musig, p, view.as_ref(), txid, index))
        .collect();
    drop(view);

    // A nonce shared clears every partial signature on the input, since
    // they were made under the aggregate nonce it changes. The rounds
    // that share go first, so a signature this pass makes is made under
    // the nonces the input ends with and stays on it, and what the pass
    // reports is what the PSBT holds. The deterministic signatures
    // follow, then the check of the others' partial signatures, and only
    // then the rounds that take a kept secret nonce: a pass that refuses
    // burns none.
    let phase = |r: &MusigRound| match r {
        MusigRound::Share { .. } => 0,
        MusigRound::SignLast => 1,
        MusigRound::Sign => 2,
        _ => 3,
    };
    let mut order: Vec<_> = mine.iter().zip(&rounds).collect();
    order.sort_by_key(|(_, r)| phase(r));
    let replaced: Vec<secp256k1::PublicKey> = mine
        .iter()
        .zip(&rounds)
        .filter(|(_, r)| matches!(r, MusigRound::Sign))
        .map(|(p, _)| p.key)
        .collect();
    // A file that carries another nonce of this device's than the one it
    // kept (an older copy won a merge) is mended, not signed: the kept
    // nonce goes in, the partial signatures that cannot verify under it
    // go out, and the cosigners sign again. Nothing signs under a nonce
    // set a partial failed on.
    if let Some((stale, unverified)) = mendable(
        &secp, musig, &nonces, &psigs, &replaced, session, index, &msg,
    ) {
        let here = &mut psbt.inner_mut().inputs[index];
        // A participant whose partial goes used its nonce for it, so the
        // nonce is spent and goes too (a key of ours gets its kept one
        // below): the round waits for a new share from that device.
        here.unknown.retain(|k, _| {
            !matches!(
                k.type_value,
                crate::musig::IN_PARTIAL_SIG | crate::musig::IN_PUB_NONCE
            ) || !unverified
                .iter()
                .any(|pk| k.key.starts_with(&pk.serialize()))
        });
        let mut mended = Vec::new();
        for key in &stale {
            let kept = session
                .as_ref()
                .and_then(|s| s.public_of(index, key))
                .expect("the key was found among the kept nonces");
            crate::musig::write_pub_nonce(here, key, &musig.output_key, &kept);
            mended.push((*key, kept.serialize()));
        }
        return Ok(MusigPass {
            signatures: Vec::new(),
            nonces_shared: 0,
            nonces_written: mended,
        });
    }
    // The public nonces this device kept are the ones that count for its
    // own keys, whatever the file says for them.
    hold_session_nonces(&mut nonces, &replaced, session, index);
    let mut checked = false;
    let mut written = Vec::new();
    let mut nonces_shared = 0usize;
    let mut nonces_written = Vec::new();
    for (participation, round) in order {
        if matches!(round, MusigRound::Sign) && !checked {
            check_others(
                &secp, musig, index, &nonces, &psigs, &replaced, &msg, session,
            )?;
            checked = true;
        }
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
                    let seed = seed.ok_or(Error::NoNonceSeed)?;
                    *session = Some(MusigSession::new(txid, *seed));
                }
                let open = session.as_mut().expect("just set");
                let rand = open
                    .draw(index, &participation.key)
                    .ok_or(Error::Unsupported {
                        input: index,
                        reason: Reason::MusigNonceNotDrawn,
                    })?;
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
                nonces_written.push((participation.key, pubnonce.serialize()));
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
                nonces.retain(|(pk, _)| *pk != participation.key);
                nonces.push((participation.key, pubnonce));
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
        }
    }

    // The others' partial signatures, against the aggregate nonce they
    // were all made under, when no round took a nonce.
    if !checked {
        check_others(
            &secp, musig, index, &nonces, &psigs, &replaced, &msg, session,
        )?;
    }
    let complete = musig
        .participants
        .iter()
        .all(|p| psigs.iter().any(|(pk, _)| pk == p));
    if !complete {
        return Ok(MusigPass {
            signatures: written,
            nonces_shared,
            nonces_written,
        });
    }
    let taproot_sig = crate::musig::aggregate(c, psbt, index, musig)
        .ok_or(Error::VerifyFailed { input: index })?;
    let sig = taproot_sig.signature.serialize();
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
        nonces_written,
    })
}

/// Costs a signature check (BIP-327 `PartialSigVerify`) per partial
/// signature. Whether the pass mends input `index` instead of signing it,
/// with the keys whose kept nonce the file does not carry and the
/// participants whose partial signature cannot verify under the kept
/// nonces: only where both exist. The partial signatures of `replaced`
/// keys, which the pass overwrites, do not count.
#[allow(clippy::too_many_arguments, reason = "one input's round state")]
fn mendable<C: secp256k1::Verification>(
    secp: &secp256k1::Secp256k1<C>,
    musig: &crate::inspect::MusigInput,
    nonces: &[(secp256k1::PublicKey, faraday_bip::musig::PubNonce)],
    psigs: &[(secp256k1::PublicKey, faraday_bip::musig::PartialSig)],
    replaced: &[secp256k1::PublicKey],
    session: &Option<MusigSession>,
    index: usize,
    msg: &[u8],
) -> Option<(Vec<secp256k1::PublicKey>, Vec<secp256k1::PublicKey>)> {
    let stale = stale_own_nonces(nonces, replaced, session, index);
    if stale.is_empty() {
        return None;
    }
    let mut held = nonces.to_vec();
    hold_session_nonces(&mut held, replaced, session, index);
    let unverified = unverifiable_partials(secp, musig, &held, psigs, msg);
    unverified
        .iter()
        .any(|pk| !replaced.contains(pk))
        .then_some((stale, unverified))
}

/// The keys of `replaced` whose public nonce on the input is not the one
/// the session kept for them.
fn stale_own_nonces(
    nonces: &[(secp256k1::PublicKey, faraday_bip::musig::PubNonce)],
    replaced: &[secp256k1::PublicKey],
    session: &Option<MusigSession>,
    index: usize,
) -> Vec<secp256k1::PublicKey> {
    replaced
        .iter()
        .filter(|key| {
            let kept = session.as_ref().and_then(|s| s.public_of(index, key));
            kept.is_some() && nonces.iter().find(|(pk, _)| pk == *key).map(|(_, n)| *n) != kept
        })
        .copied()
        .collect()
}

/// Costs a signature check (BIP-327 `PartialSigVerify`) per partial
/// signature. The participants whose partial signature does not verify
/// under `nonces`: all of them where a participant's nonce is missing.
fn unverifiable_partials<C: secp256k1::Verification>(
    secp: &secp256k1::Secp256k1<C>,
    musig: &crate::inspect::MusigInput,
    nonces: &[(secp256k1::PublicKey, faraday_bip::musig::PubNonce)],
    psigs: &[(secp256k1::PublicKey, faraday_bip::musig::PartialSig)],
    msg: &[u8],
) -> Vec<secp256k1::PublicKey> {
    let all: Option<Vec<_>> = musig
        .participants
        .iter()
        .map(|p| nonces.iter().find(|(pk, _)| pk == p).map(|(_, n)| *n))
        .collect();
    let values = all.as_ref().and_then(|all| {
        let agg = faraday_bip::musig::nonce_agg_with(secp, all).ok()?;
        faraday_bip::musig::session_values(secp, &musig.participants, &musig.tweaks, &agg, msg).ok()
    });
    let (Some(all), Some(values)) = (all, values) else {
        return psigs
            .iter()
            .filter(|(pk, _)| musig.participants.contains(pk))
            .map(|(pk, _)| *pk)
            .collect();
    };
    psigs
        .iter()
        .filter(|(pk, psig)| {
            musig
                .participants
                .iter()
                .position(|p| p == pk)
                .is_some_and(|at| {
                    faraday_bip::musig::partial_sig_verify_internal(
                        secp, psig, &all[at], pk, &values,
                    )
                    .is_err()
                })
        })
        .map(|(pk, _)| *pk)
        .collect()
}

/// Puts the public nonce the session kept for each of `replaced` in
/// `nonces` in place of any the file carries for that key.
fn hold_session_nonces(
    nonces: &mut Vec<(secp256k1::PublicKey, faraday_bip::musig::PubNonce)>,
    replaced: &[secp256k1::PublicKey],
    session: &Option<MusigSession>,
    index: usize,
) {
    for key in replaced {
        if let Some(kept) = session.as_ref().and_then(|s| s.public_of(index, key)) {
            nonces.retain(|(pk, _)| pk != key);
            nonces.push((*key, kept));
        }
    }
}

/// Costs what `musig_round` costs for each participant of ours, and a
/// signature check (BIP-327 `PartialSigVerify`) per other partial
/// signature. The refusal `sign_musig` would make on input
/// `index` for another participant's partial signature that does not
/// verify, or any refusal it makes before it takes a nonce (the sighash type,
/// the sighash, an entry that cannot be read), found without taking a kept
/// nonce.
#[allow(clippy::too_many_arguments, reason = "one input's round state")]
fn musig_precheck(
    c: &Compute,
    psbt: &Psbt,
    cache: &mut SighashCache<&bitcoin::Transaction>,
    outputs: &Outputs,
    index: usize,
    cl: &Classified,
    musig: &crate::inspect::MusigInput,
    mine: &[&crate::inspect::MusigParticipation],
    txid: bitcoin::Txid,
    session: &Option<MusigSession>,
) -> Result<(), Error> {
    // The refusals `sign_musig` makes before it takes anything, with the
    // same errors: a pass that will refuse takes no kept nonce.
    let (sighash_type, prevouts) = tap_sighash_inputs(psbt, cl, index, outputs)?;
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
    let input = &psbt.inner().inputs[index];
    let all_nonces = crate::musig::pub_nonces(input).map_err(|e| Error::Unsupported {
        input: index,
        reason: Reason::Musig(e),
    })?;
    let all_psigs = crate::musig::partial_sigs(input).map_err(|e| Error::Unsupported {
        input: index,
        reason: Reason::Musig(e),
    })?;
    let mut nonces = crate::musig::picked(&all_nonces, musig);
    let psigs = crate::musig::picked(&all_psigs, musig);
    let view = session.as_ref().map(|s| s.view());
    let replaced: Vec<secp256k1::PublicKey> = mine
        .iter()
        .filter(|p| {
            matches!(
                musig_round(c, psbt, musig, p, view.as_ref(), txid, index),
                MusigRound::Sign
            )
        })
        .map(|p| p.key)
        .collect();
    let secp = secp256k1::Secp256k1::verification_only();
    if replaced.is_empty()
        || mendable(
            &secp, musig, &nonces, &psigs, &replaced, session, index, &msg,
        )
        .is_some()
    {
        // Nothing to take, or a file the pass mends instead of signing.
        return Ok(());
    }
    hold_session_nonces(&mut nonces, &replaced, session, index);
    check_others(
        &secp, musig, index, &nonces, &psigs, &replaced, &msg, session,
    )
}

/// Costs a signature check (BIP-327 `PartialSigVerify`) per partial
/// signature on the input. Refuses with the participant whose partial
/// signature does not verify, leaving out the keys in `replaced`, whose
/// entries this pass overwrites, and taking their nonces from the
/// session. Nothing is checked while a participant's nonce is missing.
#[allow(clippy::too_many_arguments, reason = "one input's round state")]
fn check_others<C: secp256k1::Verification>(
    secp: &secp256k1::Secp256k1<C>,
    musig: &crate::inspect::MusigInput,
    index: usize,
    nonces: &[(secp256k1::PublicKey, faraday_bip::musig::PubNonce)],
    psigs: &[(secp256k1::PublicKey, faraday_bip::musig::PartialSig)],
    replaced: &[secp256k1::PublicKey],
    msg: &[u8],
    session: &Option<MusigSession>,
) -> Result<(), Error> {
    let nonce_of = |p: &secp256k1::PublicKey| {
        if replaced.contains(p) {
            session.as_ref().and_then(|s| s.public_of(index, p))
        } else {
            nonces.iter().find(|(pk, _)| pk == p).map(|(_, n)| *n)
        }
    };
    let Some(all) = musig
        .participants
        .iter()
        .map(nonce_of)
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(());
    };
    let aggnonce =
        faraday_bip::musig::nonce_agg_with(secp, &all).map_err(|_| Error::Unsupported {
            input: index,
            reason: Reason::MusigNonces,
        })?;
    let values = faraday_bip::musig::session_values(
        secp,
        &musig.participants,
        &musig.tweaks,
        &aggnonce,
        msg,
    )
    .map_err(|_| Error::Sighash { input: index })?;
    for (pk, psig) in psigs.iter().filter(|(pk, _)| !replaced.contains(pk)) {
        let Some(at) = musig.participants.iter().position(|p| p == pk) else {
            continue;
        };
        if faraday_bip::musig::partial_sig_verify_internal(secp, psig, &all[at], pk, &values)
            .is_err()
        {
            return Err(Error::MusigPartialSig {
                input: index,
                participant: hex33(pk),
            });
        }
    }
    Ok(())
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
    alloc::string::String::from(&*faraday_crypto::hex::encode_string(&key.serialize()))
}
