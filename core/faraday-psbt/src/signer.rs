//! Check a signer: whether a signing device did only what it was asked.
//!
//! The person has the PSBT they sent and the one that came back.
//! [`check`] says whether the difference between them is signatures
//! added under keys the sent PSBT names, that verify over this
//! transaction, or an input finalized from such signatures; anything
//! else is a [`Finding`]. The check fails closed: a signature that is not
//! checked is never clean, and a difference with no id of its own is
//! [`Kind::Unnamed`], at Danger.
//!
//! Nothing here holds a key or words anything for a person: a finding is
//! a typed value, and the app writes the sentence. The one rule that needs
//! a key, whether a signature's nonce is one a loaded key would have made,
//! is [`with_nonces`], which folds `verify::deterministic`'s result into a
//! report.
//!
//! Unknown and proprietary pairs are read pair by pair at every level,
//! by key type and sizes and never by content, and BIP-373's MuSig2 pairs
//! are known ([`crate::musig`]), not unknown.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::hashes::Hash;
use bitcoin::key::TapTweak;
use bitcoin::psbt::{Input, Output, raw};
use bitcoin::secp256k1::{PublicKey, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache, TapSighashType};
use bitcoin::taproot::{self, TapLeafHash};
use bitcoin::{OutPoint, ScriptBuf, Witness, ecdsa};
use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::Compute;

use crate::classify::classify;
use crate::context::Context;
use crate::finalize::{clear_signing_fields, finalize};
use crate::inspect::{
    Inspection, Level, MusigInput, SighashKind, inspect, musig_keyed_here, sighash_answers,
    sighash_stricter,
};
use crate::musig::{
    self, IN_PARTIAL_SIG, IN_PARTICIPANT_PUBKEYS, IN_PUB_NONCE, OUT_PARTICIPANT_PUBKEYS,
};
use crate::psbt::Psbt;
use crate::sign::{SigKind, tap_prevouts};
use crate::v2;
use crate::verify::{
    Determinism, Field, SigKey, SignaturePresent, Unchecked, Verdict as SigVerdict, Where, compare,
    nonce_reuse_among, origin_fingerprint, repeated_pub_nonces, verify_signatures,
};

/// A field a difference is in: one `compare` names, or one of BIP-373's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldName {
    /// A field `compare` reads.
    Compared(Field),
    /// A MuSig2 participants field, on an input or an output.
    MusigParticipants,
    /// A MuSig2 public nonce.
    MusigNonce,
    /// A MuSig2 partial signature.
    MusigPartialSig,
}

/// Why a signature present could not be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotChecked {
    /// The reason an ordinary signature gives (`verify_signatures`).
    Signature(Unchecked),
    /// A MuSig2 input whose wallet is not registered, so the session's
    /// keys and tweaks are not known.
    MusigWallet,
    /// A MuSig2 participant's public nonce is missing or unreadable.
    MusigNonce,
}

/// What a finding is: one kind for each id of SIGNER-CHECK §6. `key`
/// is named as the review names a key (`SignaturePresent::name`). Each
/// variant's fields are the values its id's wording states.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Kind {
    /// T1: the version.
    Version { sent: i32, returned: i32 },
    /// T2: the locktime.
    Locktime { sent: u32, returned: u32 },
    /// T3: the number of inputs.
    InputCount { sent: usize, returned: usize },
    /// T4: the number of outputs.
    OutputCount { sent: usize, returned: usize },
    /// T5: the outpoint an input spends.
    Spends { sent: OutPoint, returned: OutPoint },
    /// T6: an input's sequence.
    Sequence { sent: u32, returned: u32 },
    /// T7: an output's amount, in satoshis.
    OutputAmount { sent: u64, returned: u64 },
    /// T8: an output's script.
    OutputScript {
        sent: ScriptBuf,
        returned: ScriptBuf,
    },
    /// S1: an added signature that is not a signature of this transaction.
    SignatureInvalid { key: String },
    /// S2: an added signature that was not checked.
    SignatureUnchecked { key: String, reason: NotChecked },
    /// S3: an added signature under a key the sent PSBT does not name.
    SignatureUnnamed { key: String },
    /// S3l: an added script-path signature over a leaf the sent PSBT does
    /// not hold.
    SignatureLeafUnknown { key: String },
    /// S4: an added signature made under another sighash than asked.
    SighashOther {
        key: String,
        signed: SighashKind,
        asked: SighashKind,
    },
    /// S4n: the same, where the sighash commits to no output.
    SighashNone {
        key: String,
        signed: SighashKind,
        asked: SighashKind,
    },
    /// S4m: the same, where the sighash commits to more than asked.
    SighashStricter {
        key: String,
        signed: SighashKind,
        asked: SighashKind,
    },
    /// S5: one nonce in more than one place, under one key or several (`key`
    /// names them).
    RepeatedNonce { key: String, inputs: Vec<usize> },
    /// S6: a signature that RFC 6979 and BIP-340 with zero auxiliary
    /// randomness do not reproduce under a loaded key.
    NonceNotDeterministic { fingerprint: Fingerprint },
    /// S6u: a signature under a loaded key's master that could not be
    /// recomputed (no result for its bytes).
    NonceNotRecomputed { fingerprint: Fingerprint },
    /// S7: a signature on the sent PSBT that is gone.
    SignatureRemoved { key: String },
    /// S8: a signature on both, with other bytes.
    SignatureChanged { key: String },
    /// S9: nothing was added and nothing finalized.
    NoSignatureAdded,
    /// F1: the final fields differ from what Faraday's finalizer writes.
    FinalDiffers,
    /// F2: signatures were read out of the final fields but Faraday's
    /// finalizer refuses the input, so there is nothing to compare and
    /// nothing accounts for what the final fields carry.
    FinalNotCompared,
    /// F3: final fields on both, with other bytes.
    FinalChanged,
    /// F4: final fields on the sent PSBT, none returned.
    FinalRemoved,
    /// M1: a field added.
    FieldAdded(FieldName),
    /// M2: a field changed.
    FieldChanged(FieldName),
    /// M3: a field removed.
    FieldRemoved(FieldName),
    /// P1: an unknown pair added.
    UnknownAdded {
        type_value: u8,
        key_len: usize,
        value_len: usize,
    },
    /// P2: an unknown pair's value changed.
    UnknownChanged {
        type_value: u8,
        from: usize,
        to: usize,
    },
    /// P3: a proprietary pair added.
    ProprietaryAdded {
        prefix_len: usize,
        subtype: u64,
        value_len: usize,
    },
    /// P4: a proprietary pair's value changed.
    ProprietaryChanged {
        prefix_len: usize,
        subtype: u64,
        from: usize,
        to: usize,
    },
    /// P5: an unknown pair removed.
    UnknownRemoved { type_value: u8 },
    /// P6: a proprietary pair removed.
    ProprietaryRemoved { prefix_len: usize, subtype: u64 },
    /// E1: bytes in the returned file its fields do not account for.
    UnaccountedBytes { count: usize },
    /// A difference the spec names no id for, at Danger (SIGNER-CHECK
    /// §10.1): the field `compare` found it in.
    Unnamed(Field),
}

impl Kind {
    /// The level of this kind (SIGNER-CHECK §6).
    pub fn level(&self) -> Level {
        match self {
            Kind::Version { .. }
            | Kind::Locktime { .. }
            | Kind::InputCount { .. }
            | Kind::OutputCount { .. }
            | Kind::Spends { .. }
            | Kind::Sequence { .. }
            | Kind::OutputAmount { .. }
            | Kind::OutputScript { .. }
            | Kind::SignatureInvalid { .. }
            | Kind::SighashNone { .. }
            | Kind::RepeatedNonce { .. } => Level::Blocked,
            Kind::SignatureUnchecked { .. }
            | Kind::SighashStricter { .. }
            | Kind::SignatureRemoved { .. }
            | Kind::NoSignatureAdded
            | Kind::FinalRemoved
            | Kind::FieldRemoved(_)
            | Kind::UnknownRemoved { .. }
            | Kind::ProprietaryRemoved { .. } => Level::Caution,
            Kind::SignatureUnnamed { .. }
            | Kind::SignatureLeafUnknown { .. }
            | Kind::SighashOther { .. }
            | Kind::NonceNotDeterministic { .. }
            | Kind::NonceNotRecomputed { .. }
            | Kind::SignatureChanged { .. }
            | Kind::FinalDiffers
            | Kind::FinalNotCompared
            | Kind::FinalChanged
            | Kind::FieldAdded(_)
            | Kind::FieldChanged(_)
            | Kind::UnknownAdded { .. }
            | Kind::UnknownChanged { .. }
            | Kind::ProprietaryAdded { .. }
            | Kind::ProprietaryChanged { .. }
            | Kind::UnaccountedBytes { .. }
            | Kind::Unnamed(_) => Level::Danger,
        }
    }
}

/// One finding: where, how bad, and what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Its level, `kind.level()`.
    pub level: Level,
    /// "Transaction", "Input {i}" or "Output {i}"; the returned file as a
    /// whole is the transaction's place.
    pub at: Where,
    /// What it is.
    pub kind: Kind,
}

impl Finding {
    fn new(at: Where, kind: Kind) -> Self {
        Finding {
            level: kind.level(),
            at,
            kind,
        }
    }
}

/// A line the report states that is not a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Info {
    /// A MuSig2 public nonce was added for this participant.
    NonceAdded { input: usize, key: String },
    /// An input was finalized and nothing else is wrong with it.
    Finalized { input: usize },
    /// A field the signer added that is checked in full against the
    /// unchanged transaction or Sent alone (F199, SIGNER-CHECK §6.9).
    /// The words must never say the signer "changed" or that the field is
    /// "mine": whether a signer adds such a field is itself a signal.
    FieldStated { at: Where, field: Stated },
}

/// An added field that can be checked in full, and was (F199). Each is
/// absent on Sent and present on Returned under a key of one byte, the
/// type byte: the parser refuses a longer key for these fields, so such a
/// file never reaches the check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stated {
    /// A witness script whose SHA256 is the P2WSH program of the output's
    /// script or of the spent output's script, or whose P2WSH script is
    /// the one a P2SH script hashes (P2SH-P2WSH), anchored directly.
    WitnessScript,
    /// A redeem script whose HASH160 is in the P2SH script of the output
    /// or of the spent output.
    RedeemScript,
    /// A witness UTXO equal to the output Sent's non-witness UTXO holds at
    /// this input's index.
    SpentOutput,
    /// A non-witness UTXO whose txid is this input's previous txid and
    /// whose inputs carry no witness.
    PreviousTransaction,
    /// An input's internal key, with its merkle root where there is one,
    /// that recompute the spent output's key.
    TaprootKey,
    /// A sighash type equal to that of every signature on the input.
    Sighash,
}

/// What the check came to, in the screen's terms (SIGNER-CHECK §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Verdict {
    /// No finding: only signatures (and nonces) were added, and `finalized`
    /// inputs were finalized from them.
    Clean { finalized: usize, nonces: bool },
    /// Findings: how many, and the worst level among them.
    Findings { count: usize, worst: Level },
    /// The two PSBTs have no input in common.
    NotSameTransaction,
}

/// A signature the device added, for the nonce fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddedSignature {
    /// The input it signs.
    pub input: usize,
    /// The key it is under.
    pub key: SigKey,
    /// The leaf it signs, for a taproot script path.
    pub leaf: Option<TapLeafHash>,
    /// The key as the review names it.
    pub name: String,
    /// The master fingerprint a key origin of the sent PSBT gives.
    pub fingerprint: Option<Fingerprint>,
    /// The signature as its field holds it.
    pub bytes: Vec<u8>,
}

/// How many added signatures' nonces were recomputed from a loaded key
/// (SIGNER-CHECK §4, "Nonces recomputed").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recomputed {
    /// Signatures whose nonce a loaded key's rule reproduced.
    pub recomputed: usize,
    /// Signatures added under an ordinary key.
    pub of: usize,
    /// Masters named on added signatures that no loaded key is.
    pub not_loaded: Vec<Fingerprint>,
}

/// What the screen states beside the verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    /// The two PSBTs share an input.
    pub same_transaction: bool,
    /// Every signature added under an ordinary key, in input order.
    pub added: Vec<AddedSignature>,
    /// MuSig2 partial signatures added.
    pub partial_sigs_added: usize,
    /// The inputs finalized.
    pub finalized: Vec<usize>,
    /// Set by [`with_nonces`]; `None` until a key was asked.
    pub recomputed: Option<Recomputed>,
}

/// The result of [`check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The verdict.
    pub verdict: Verdict,
    /// The findings, in the order of SIGNER-CHECK §4: Blocked first, then
    /// Danger, then Caution; within a level the transaction, then the
    /// inputs in order, then the outputs in order, and as found within one
    /// place.
    pub findings: Vec<Finding>,
    /// What is stated and is no finding.
    pub info: Vec<Info>,
    /// The facts.
    pub facts: Facts,
}

/// What `sent` was changed into, as a report of what the device did.
struct Run<'a> {
    sent: &'a Psbt,
    returned: &'a Psbt,
    sent_inspection: Inspection,
    returned_inspection: Inspection,
    findings: Vec<Finding>,
    info: Vec<Info>,
    added: Vec<AddedSignature>,
    partial_sigs_added: usize,
    finalized: Vec<usize>,
    /// For each input finalized, the sent PSBT with that input finalized
    /// by Faraday's own finalizer: what the finalizer clears is what may
    /// be gone from the returned input.
    finalized_as: BTreeMap<usize, Psbt>,
    /// The sighash of every signature added, with the input it is on:
    /// the flags rule (BIP-370) reads them.
    added_sighashes: Vec<SighashKind>,
}

/// Costs a parse-free reading of both PSBTs: an inspection of each (a
/// script derivation for each wallet an input or output claims), a
/// signature verification per signature on either, a finalization per
/// finalized input and a MuSig2 partial-signature check per partial
/// signature added. Says whether the device that returned `returned`
/// did only what `sent` asked of it.
///
/// `sent_bytes` and `returned_bytes` are the two files as the person has
/// them, base64 decoded where they are text; `sent` and `returned` are
/// their version 0 forms. The files are read for what the version 0
/// form does not hold (the PSBT's version, the fields only version 2 has,
/// and BIP-370's modifiable flags); a file that cannot be read is
/// [`Kind::Unnamed`]. The returned file longer than its fields written
/// out again is E1; the sent file's own length is the person's and is
/// not a finding. Pure, and needs no key.
pub fn check(
    c: &Compute,
    sent: &Psbt,
    returned: &Psbt,
    sent_bytes: &[u8],
    returned_bytes: &[u8],
    ctx: &Context,
) -> Report {
    let mut run = Run {
        sent,
        returned,
        sent_inspection: inspect(c, sent, ctx),
        returned_inspection: inspect(c, returned, ctx),
        findings: Vec::new(),
        info: Vec::new(),
        added: Vec::new(),
        partial_sigs_added: 0,
        finalized: Vec::new(),
        finalized_as: BTreeMap::new(),
        added_sighashes: Vec::new(),
    };
    let comparison = compare(sent, returned);
    run.transaction(&comparison.transaction);
    let inputs = sent.inner().inputs.len().min(returned.inner().inputs.len());
    for i in 0..inputs {
        run.finalization(i);
    }
    run.fields(&comparison.metadata);
    for i in 0..inputs {
        run.signatures(i);
        run.input_pairs(i);
    }
    let outputs = sent
        .inner()
        .outputs
        .len()
        .min(returned.inner().outputs.len());
    for i in 0..outputs {
        run.output_pairs(i);
    }
    run.global_pairs();
    // Sent's signatures count with Returned's: a signature changed to one
    // with the same nonce gives the key away from the pair.
    let both = run
        .sent_inspection
        .signatures
        .iter()
        .chain(&run.returned_inspection.signatures)
        .flat_map(|i| i.signatures.iter().cloned());
    let mut reuses = nonce_reuse_among(both);
    reuses.extend(repeated_pub_nonces(&[run.sent, run.returned]));
    for reuse in &reuses {
        let kind = Kind::RepeatedNonce {
            key: reuse.name(),
            inputs: reuse.inputs.clone(),
        };
        run.findings.push(Finding::new(Where::Transaction, kind));
    }
    run.file_fields(sent_bytes, returned_bytes);
    let nonces = run
        .info
        .iter()
        .any(|i| matches!(i, Info::NonceAdded { .. }));
    if run.added.is_empty() && run.partial_sigs_added == 0 && !nonces && run.finalized.is_empty() {
        run.push(Where::Transaction, Kind::NoSignatureAdded);
    }
    sort_findings(&mut run.findings);
    run.into_report()
}

/// Blocked first, then Danger, then Caution; within a level the
/// transaction, the inputs in order, the outputs in order. Stable.
fn sort_findings(findings: &mut [Finding]) {
    findings.sort_by_key(|f| {
        let place = match f.at {
            Where::Transaction => (0, 0),
            Where::Input(i) => (1, i),
            Where::Output(i) => (2, i),
        };
        (core::cmp::Reverse(f.level), place)
    });
}

/// Costs a signature verification per signature on `returned`. The sent
/// PSBT with every signature of the returned one carried as a partial or
/// taproot signature, those read out of final fields included, and the
/// sent PSBT's own key origins: what `verify::deterministic` needs to
/// recompute a nonce, since a finalized input carries no origins and
/// names no key of its own. faraday-core calls it, then `deterministic`
/// over the view with the keys loaded, then [`with_nonces`].
pub fn nonce_view(sent: &Psbt, returned: &Psbt) -> Psbt {
    let ctx = Context {
        network: Network::Mainnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let mut view = sent.clone();
    for read in verify_signatures(returned, &ctx) {
        let Some(input) = view.inner_mut().inputs.get_mut(read.index) else {
            continue;
        };
        clear_signatures(input);
        for sig in &read.signatures {
            // A signature under no known key has nowhere to be filed.
            let _ = put_signature(input, sig);
        }
        // The fields stated at Info (F199) are checked in full: the view
        // carries them, so the signature hash can be computed where only
        // Returned holds the UTXO or the script.
        let Some(returned_input) = returned.inner().inputs.get(read.index) else {
            continue;
        };
        for field in [
            Field::WitnessScript,
            Field::RedeemScript,
            Field::SpentOutput,
            Field::PreviousTransaction,
            Field::InternalKey,
        ] {
            if stated_field(sent, returned, Where::Input(read.index), field).is_none() {
                continue;
            }
            match field {
                Field::WitnessScript => {
                    input.witness_script = returned_input.witness_script.clone();
                }
                Field::RedeemScript => input.redeem_script = returned_input.redeem_script.clone(),
                Field::SpentOutput => input.witness_utxo = returned_input.witness_utxo.clone(),
                Field::PreviousTransaction => {
                    input.non_witness_utxo = returned_input.non_witness_utxo.clone();
                }
                _ => {
                    input.tap_internal_key = returned_input.tap_internal_key;
                    input.tap_merkle_root = returned_input.tap_merkle_root;
                }
            }
        }
    }
    view
}

/// Folds what `verify::deterministic` found over the [`nonce_view`] into
/// `report`: S6 for an added signature that no nonce rule of a loaded key
/// gives, and the "Nonces recomputed" fact. A lookup.
///
/// `found` is `deterministic`'s result for the view and the keys loaded,
/// and an empty one when none is loaded. A report of two PSBTs that are
/// not one transaction is returned as it is.
pub fn with_nonces(mut report: Report, found: &[Determinism]) -> Report {
    if !report.facts.same_transaction {
        return report;
    }
    let mut recomputed = 0;
    // The masters given: those a result names.
    let mut loaded: Vec<Fingerprint> = found.iter().map(|d| d.fingerprint).collect();
    for added in &report.facts.added {
        // A result is for this signature's own bytes, not for its place.
        let hit = found.iter().find(|d| {
            d.input == added.input
                && d.key == added.key
                && d.leaf == added.leaf
                && d.signature == added.bytes
        });
        let given = added.fingerprint.filter(|fp| loaded.contains(fp));
        match (hit, given) {
            (Some(hit), _) if hit.hashed && hit.mode.is_some() => recomputed += 1,
            (Some(hit), _) if hit.hashed => report.findings.push(Finding::new(
                Where::Input(added.input),
                Kind::NonceNotDeterministic {
                    fingerprint: hit.fingerprint,
                },
            )),
            (hit, Some(fingerprint)) => {
                let fingerprint = hit.map_or(fingerprint, |h| h.fingerprint);
                report.findings.push(Finding::new(
                    Where::Input(added.input),
                    Kind::NonceNotRecomputed { fingerprint },
                ));
            }
            (_, None) => {}
        }
    }
    loaded.dedup();
    let mut not_loaded: Vec<Fingerprint> = Vec::new();
    for fp in report.facts.added.iter().filter_map(|a| a.fingerprint) {
        if !loaded.contains(&fp) && !not_loaded.contains(&fp) {
            not_loaded.push(fp);
        }
    }
    report.facts.recomputed = Some(Recomputed {
        recomputed,
        of: report.facts.added.len(),
        not_loaded,
    });
    sort_findings(&mut report.findings);
    let findings = &report.findings;
    report
        .info
        .retain(|i| !matches!(i, Info::Finalized { input } if findings.iter().any(|f| f.at == Where::Input(*input))));
    report.verdict = verdict_of(&report.findings, &report.facts, &report.info);
    report
}

fn verdict_of(findings: &[Finding], facts: &Facts, info: &[Info]) -> Verdict {
    if !facts.same_transaction {
        return Verdict::NotSameTransaction;
    }
    match findings.iter().map(|f| f.level).max() {
        None => Verdict::Clean {
            finalized: facts.finalized.len(),
            nonces: info.iter().any(|i| matches!(i, Info::NonceAdded { .. })),
        },
        Some(worst) => Verdict::Findings {
            count: findings.len(),
            worst,
        },
    }
}

fn hex8(bytes: &[u8]) -> String {
    bytes.iter().take(4).map(|b| format!("{b:02x}")).collect()
}

/// Files `sig` in `input` as a partial or taproot signature; `None` for
/// one that is not under a key, or that does not parse.
fn put_signature(input: &mut Input, sig: &SignaturePresent) -> Option<()> {
    match (&sig.key, sig.kind, sig.leaf) {
        (SigKey::Ecdsa(pk), SigKind::Ecdsa, _) => {
            let parsed = ecdsa::Signature::from_slice(&sig.bytes).ok()?;
            input.partial_sigs.insert(*pk, parsed);
        }
        (SigKey::Schnorr(_), SigKind::Schnorr, None) => {
            input.tap_key_sig = Some(taproot::Signature::from_slice(&sig.bytes).ok()?);
        }
        (SigKey::Schnorr(x), SigKind::Schnorr, Some(leaf)) => {
            let parsed = taproot::Signature::from_slice(&sig.bytes).ok()?;
            input.tap_script_sigs.insert((*x, leaf), parsed);
        }
        _ => return None,
    }
    Some(())
}

fn clear_signatures(input: &mut Input) {
    input.partial_sigs.clear();
    input.tap_key_sig = None;
    input.tap_script_sigs.clear();
}

fn has_final(input: &Input) -> bool {
    input.final_script_sig.is_some() || input.final_script_witness.is_some()
}

/// One pair of a map that is in one and not the other, or in both with
/// another value.
enum Change<'a, K, V> {
    Added(&'a K, &'a V),
    Changed(&'a K, &'a V, &'a V),
    Removed(&'a K),
}

fn map_diff<'a, K: Ord, V: PartialEq>(
    sent: &BTreeMap<&'a K, &'a V>,
    returned: &BTreeMap<&'a K, &'a V>,
) -> Vec<Change<'a, K, V>> {
    let mut out = Vec::new();
    for (k, v) in returned {
        match sent.get(k) {
            None => out.push(Change::Added(*k, *v)),
            Some(old) if old != v => out.push(Change::Changed(*k, *old, *v)),
            Some(_) => {}
        }
    }
    for k in sent.keys() {
        if !returned.contains_key(k) {
            out.push(Change::Removed(*k));
        }
    }
    out
}

/// The pairs of an unknown map whose type `keep` lets through.
fn unknown_of(
    map: &BTreeMap<raw::Key, Vec<u8>>,
    keep: impl Fn(u8) -> bool,
) -> BTreeMap<&raw::Key, &Vec<u8>> {
    map.iter().filter(|(k, _)| keep(k.type_value)).collect()
}

/// The participant of a nonce's or partial signature's key, whatever the
/// rest of it says: participant (33 bytes), aggregate (33), and an
/// optional leaf hash (32); [`named_pair_participant`] refuses the last.
fn pair_participant(key: &raw::Key) -> Option<PublicKey> {
    if !matches!(key.key.len(), 66 | 98) {
        return None;
    }
    PublicKey::from_slice(&key.key[..33]).ok()
}

/// The participant of a pair's key read whole: the participants field of
/// the sent input names the aggregate with the participant under it, and
/// a leaf hash, where there is one, is a leaf the sent input holds. Any
/// other length or value is `None`.
fn named_pair_participant(
    key: &raw::Key,
    named: &[musig::Participants],
    input: &Input,
    output_key: Option<XOnlyPublicKey>,
    musig: Option<&MusigInput>,
) -> Option<PublicKey> {
    let participant = pair_participant(key)?;
    let aggregate = PublicKey::from_slice(&key.key[33..66]).ok()?;
    // The aggregate is one of the three forms the crate reads (README):
    // the root aggregate the participants field is keyed by, the aggregate
    // derived at the sub-path (the input's internal key), or the taproot
    // output key of the output the sent input spends, which is what a
    // key-path pair is keyed by (BIP-373, Core). Every anchor is Sent's.
    // With the wallet established, the crate's own reader decides
    // (`musig_keyed_here`: the output key with its parity, the derived
    // aggregate by x only); without it the output key is the script's
    // x-only program and the derived aggregate Sent's internal key.
    let named_participant = named.iter().any(|n| n.keys.contains(&participant));
    let is_root = named
        .iter()
        .any(|n| n.aggregate == aggregate && n.keys.contains(&participant));
    let x = aggregate.x_only_public_key().0;
    let is_other = named_participant
        && match musig {
            Some(m) => musig_keyed_here(m, &aggregate),
            None => output_key == Some(x) || input.tap_internal_key == Some(x),
        };
    if !is_root && !is_other {
        return None;
    }
    // A key with a leaf hash (BIP-373's script-path form) is not read:
    // script-path MuSig2 is not loaded, so the pair is a field finding.
    if key.key.len() == 98 {
        return None;
    }
    Some(participant)
}

impl Run<'_> {
    fn push(&mut self, at: Where, kind: Kind) {
        self.findings.push(Finding::new(at, kind));
    }

    fn into_report(mut self) -> Report {
        let facts = Facts {
            same_transaction: self.same_transaction(),
            added: self.added.clone(),
            partial_sigs_added: self.partial_sigs_added,
            finalized: self.finalized.clone(),
            recomputed: None,
        };
        for &input in &self.finalized {
            if !self.findings.iter().any(|f| f.at == Where::Input(input)) {
                self.info.push(Info::Finalized { input });
            }
        }
        if !facts.same_transaction {
            // Two transactions with an input in common are compared; with
            // none, nothing else is said.
            self.findings.clear();
            self.info.clear();
        }
        Report {
            verdict: verdict_of(&self.findings, &facts, &self.info),
            findings: self.findings,
            info: self.info,
            facts,
        }
    }

    /// The two share an input, by the outpoint it spends.
    fn same_transaction(&self) -> bool {
        let (a, b) = (
            &self.sent.unsigned_tx().input,
            &self.returned.unsigned_tx().input,
        );
        a.iter()
            .any(|x| b.iter().any(|y| x.previous_output == y.previous_output))
    }

    /// T1 to T8, read from the transaction the `compare` diffs name.
    fn transaction(&mut self, diffs: &[crate::verify::FieldDiff]) {
        let (a, b) = (self.sent.unsigned_tx(), self.returned.unsigned_tx());
        for d in diffs {
            let kind = match (d.at, d.field) {
                (Where::Transaction, Field::Version) => Kind::Version {
                    sent: a.version.0,
                    returned: b.version.0,
                },
                (Where::Transaction, Field::Locktime) => Kind::Locktime {
                    sent: a.lock_time.to_consensus_u32(),
                    returned: b.lock_time.to_consensus_u32(),
                },
                (Where::Transaction, Field::InputCount) => Kind::InputCount {
                    sent: a.input.len(),
                    returned: b.input.len(),
                },
                (Where::Transaction, Field::OutputCount) => Kind::OutputCount {
                    sent: a.output.len(),
                    returned: b.output.len(),
                },
                (Where::Input(i), Field::Spends) => Kind::Spends {
                    sent: a.input[i].previous_output,
                    returned: b.input[i].previous_output,
                },
                (Where::Input(i), Field::Sequence) => Kind::Sequence {
                    sent: a.input[i].sequence.to_consensus_u32(),
                    returned: b.input[i].sequence.to_consensus_u32(),
                },
                (Where::Output(i), Field::Amount) => Kind::OutputAmount {
                    sent: a.output[i].value.to_sat(),
                    returned: b.output[i].value.to_sat(),
                },
                (Where::Output(i), Field::Script) => Kind::OutputScript {
                    sent: a.output[i].script_pubkey.clone(),
                    returned: b.output[i].script_pubkey.clone(),
                },
                (_, field) => Kind::Unnamed(field),
            };
            self.push(d.at, kind);
        }
    }

    /// The two files' own bytes: the version, the fields only version 2
    /// has, the modifiable flags and the returned file's length (E1).
    fn file_fields(&mut self, sent_bytes: &[u8], returned_bytes: &[u8]) {
        let (s, r) = (v2::read_file(sent_bytes), v2::read_file(returned_bytes));
        if let Ok(r) = &r {
            let excess = returned_bytes.len().saturating_sub(self.expected_len(r));
            if excess > 0 {
                self.push(Where::Transaction, Kind::UnaccountedBytes { count: excess });
            }
        }
        let (Ok(s), Ok(r)) = (&s, &r) else {
            // A file the caller could not have read is never clean.
            self.push(Where::Transaction, Kind::Unnamed(Field::PsbtVersion));
            return;
        };
        let name = |f| FieldName::Compared(f);
        let at = Where::Transaction;
        if s.version != r.version {
            self.push(at, Kind::FieldChanged(name(Field::PsbtVersion)));
            return;
        }
        if s.version == 0 {
            match (s.explicit_version, r.explicit_version) {
                (false, true) => self.push(at, Kind::FieldAdded(name(Field::PsbtVersion))),
                (true, false) => self.push(at, Kind::FieldRemoved(name(Field::PsbtVersion))),
                _ => {}
            }
            return;
        }
        self.v2_pairs(at, &s.global, &r.global);
        for i in 0..s.inputs.len().min(r.inputs.len()) {
            self.v2_pairs(Where::Input(i), &s.inputs[i], &r.inputs[i]);
        }
        for i in 0..s.outputs.len().min(r.outputs.len()) {
            self.v2_pairs(Where::Output(i), &s.outputs[i], &r.outputs[i]);
        }
        self.modifiable_flags(&s.global, &r.global);
    }

    /// The length the returned file is held to: its version 0 form written
    /// again, and for a version 2 file less the transaction pair that form
    /// carries, plus every pair only version 2 has (all compared in
    /// `file_fields`).
    fn expected_len(&self, file: &v2::File) -> usize {
        let base = self.returned.inner().serialize().len();
        if file.version == 0 {
            // `bitcoin` does not write an explicit version of 0 back: its
            // key's length, type, the value's length and four bytes.
            return base
                + if file.explicit_version {
                    1 + 1 + 1 + 4
                } else {
                    0
                };
        }
        let tx = bitcoin::consensus::serialize(self.returned.unsigned_tx());
        let own = |map: &v2::Map, only: &[u8]| {
            map.pairs_of_types(only)
                .map(|(k, v)| v2::pair_len(k, v))
                .sum::<usize>()
        };
        let only: usize = own(&file.global, v2::GLOBAL_V2_ONLY)
            + file
                .inputs
                .iter()
                .map(|m| own(m, v2::INPUT_V2_ONLY))
                .sum::<usize>()
            + file
                .outputs
                .iter()
                .map(|m| own(m, v2::OUTPUT_V2_ONLY))
                .sum::<usize>();
        (base + only).saturating_sub(v2::pair_len(&[v2::GLOBAL_UNSIGNED_TX], &tx))
    }

    /// M1 to M3 for the pairs only version 2 has in one place, other than
    /// the modifiable flags: compared byte for byte, one finding for each
    /// of added, changed and removed.
    fn v2_pairs(&mut self, at: Where, sent: &v2::Map, returned: &v2::Map) {
        let only = match at {
            Where::Transaction => v2::GLOBAL_V2_ONLY,
            Where::Input(_) => v2::INPUT_V2_ONLY,
            Where::Output(_) => v2::OUTPUT_V2_ONLY,
        };
        let pairs = |m: &'_ v2::Map| -> BTreeMap<Vec<u8>, Vec<u8>> {
            m.pairs_of_types(only)
                .filter(|(k, _)| !(at == Where::Transaction && *k == [v2::GLOBAL_TX_MODIFIABLE]))
                .map(|(k, v)| (k.to_vec(), v.to_vec()))
                .collect()
        };
        let (s, r) = (pairs(sent), pairs(returned));
        let field = FieldName::Compared(Field::V2Fields);
        let (mut added, mut changed, mut removed) = (false, false, false);
        for change in map_diff(&s.iter().collect(), &r.iter().collect()) {
            match change {
                Change::Added(..) => added = true,
                Change::Changed(..) => changed = true,
                Change::Removed(_) => removed = true,
            }
        }
        if added {
            self.push(at, Kind::FieldAdded(field));
        }
        if changed {
            self.push(at, Kind::FieldChanged(field));
        }
        if removed {
            self.push(at, Kind::FieldRemoved(field));
        }
    }

    /// `PSBT_GLOBAL_TX_MODIFIABLE` against BIP-370's rule for a signer
    /// ("Signer": each signature added not using `SIGHASH_ANYONECANPAY`
    /// clears bit 0, each not using `SIGHASH_NONE` clears bit 1, each
    /// using `SIGHASH_SINGLE` sets bit 2). The result is worked out from
    /// Sent's byte; Returned must hold exactly it. A signer that leaves the
    /// field as it was where the rule changes it is a finding.
    fn modifiable_flags(&mut self, sent: &v2::Map, returned: &v2::Map) {
        let field = FieldName::Compared(Field::ModifiableFlags);
        let at = Where::Transaction;
        let (s, r) = (
            sent.get_type(v2::GLOBAL_TX_MODIFIABLE),
            returned.get_type(v2::GLOBAL_TX_MODIFIABLE),
        );
        let mut byte = match s {
            Some([b]) => *b,
            _ => 0,
        };
        for kind in &self.added_sighashes {
            let b = sighash_byte(*kind);
            if b & 0x80 == 0 {
                byte &= !0b001;
            }
            if b & 0x1f != 2 {
                byte &= !0b010;
            }
            if b & 0x1f == 3 {
                byte |= 0b100;
            }
        }
        match (s, r) {
            (Some(s), Some(r)) => {
                // A byte that is not one byte long is not a flags field:
                // it stays as it was or it is changed.
                let want: &[u8] = if s.len() == 1 { &[byte] } else { s };
                if r != want {
                    self.push(at, Kind::FieldChanged(field));
                }
            }
            (Some(_), None) => self.push(at, Kind::FieldRemoved(field)),
            // BIP-370's signer "must update" the field: with a signature
            // added, the result worked out from 0 is what it holds, `00`
            // included; with none added, the field is not the signer's to add.
            (None, Some(r)) => {
                if self.added_sighashes.is_empty() || r != [byte] {
                    self.push(at, Kind::FieldAdded(field));
                }
            }
            (None, None) => {}
        }
    }

    /// M1 to M3, from the fields `compare` decodes. Unknown and
    /// proprietary pairs are read pair by pair elsewhere.
    fn fields(&mut self, diffs: &[crate::verify::FieldDiff]) {
        for d in diffs {
            if matches!(d.field, Field::Unknown | Field::Proprietary) {
                continue;
            }
            let name = FieldName::Compared(d.field);
            let kind = match (d.a.is_empty(), d.b.is_empty()) {
                (true, _) => {
                    if let Some(field) = self.stated(d) {
                        let info = Info::FieldStated { at: d.at, field };
                        if !self.info.contains(&info) {
                            self.info.push(info);
                        }
                        continue;
                    }
                    Kind::FieldAdded(name)
                }
                (false, true) => {
                    if self.finalizer_cleared(d.at, d.field) {
                        continue;
                    }
                    Kind::FieldRemoved(name)
                }
                (false, false) => Kind::FieldChanged(name),
            };
            self.push(d.at, kind);
        }
    }

    /// Which stated field (F199) the added field `d` is, when its content
    /// is checked in full against the unchanged transaction or Sent; every
    /// anchor is Sent's, never Returned's own copy.
    fn stated(&self, d: &crate::verify::FieldDiff) -> Option<Stated> {
        if let (Where::Input(i), Field::Sighash) = (d.at, d.field) {
            let (s, r) = (
                self.sent.inner().inputs.get(i)?,
                self.returned.inner().inputs.get(i)?,
            );
            if s.sighash_type.is_some() {
                return None;
            }
            let want = r.sighash_type?.to_u32();
            let signatures = &self
                .returned_inspection
                .signatures
                .iter()
                .find(|x| x.index == i)?
                .signatures;
            return (!signatures.is_empty()
                && signatures.iter().all(|x| sighash_byte(x.sighash) == want))
            .then_some(Stated::Sighash);
        }
        stated_field(self.sent, self.returned, d.at, d.field)
    }

    /// Whether finalizing the sent input clears `field`: the finalizer's
    /// own result for it, compared with the sent PSBT.
    fn finalizer_cleared(&self, at: Where, field: Field) -> bool {
        let Where::Input(i) = at else { return false };
        let Some(copy) = self.finalized_as.get(&i) else {
            return false;
        };
        compare(self.sent, copy)
            .metadata
            .iter()
            .any(|d| d.at == at && d.field == field && d.b.is_empty())
    }

    /// F1 to F4, and which inputs were finalized.
    fn finalization(&mut self, i: usize) {
        let (s, r) = (
            &self.sent.inner().inputs[i],
            &self.returned.inner().inputs[i],
        );
        let at = Where::Input(i);
        match (has_final(s), has_final(r)) {
            (false, true) => {
                self.finalized.push(i);
                match self.expected_final(i) {
                    None => {
                        // No signature can be read out of the final fields,
                        // so nothing accounts for what they carry.
                        let read = self.returned_inspection.signatures.get(i);
                        let any = read.is_some_and(|r| r.signatures.iter().any(|s| s.finalized));
                        self.push(
                            at,
                            if any {
                                Kind::FinalNotCompared
                            } else {
                                Kind::FinalDiffers
                            },
                        );
                        let mut copy = self.sent.clone();
                        clear_signing_fields(&mut copy.inner_mut().inputs[i]);
                        self.finalized_as.insert(i, copy);
                    }
                    Some(copy) => {
                        let want = &copy.inner().inputs[i];
                        if (&want.final_script_sig, &want.final_script_witness)
                            != (&r.final_script_sig, &r.final_script_witness)
                        {
                            self.push(at, Kind::FinalDiffers);
                        }
                        self.finalized_as.insert(i, copy);
                    }
                }
            }
            (true, true) => {
                if (&s.final_script_sig, &s.final_script_witness)
                    != (&r.final_script_sig, &r.final_script_witness)
                {
                    self.push(at, Kind::FinalChanged);
                }
            }
            (true, false) => self.push(at, Kind::FinalRemoved),
            (false, false) => {}
        }
    }

    /// The sent PSBT with input `i` finalized by Faraday's finalizer from
    /// the signatures the returned PSBT's final fields carry, or `None`
    /// when it cannot.
    fn expected_final(&self, i: usize) -> Option<Psbt> {
        let mut copy = self.sent.clone();
        // The others are marked final, so that the finalizer works on
        // this input alone and refuses nothing for another's sake.
        for (j, input) in copy.inner_mut().inputs.iter_mut().enumerate() {
            if j != i && !has_final(input) {
                input.final_script_witness = Some(Witness::new());
            }
        }
        let read = self.returned_inspection.signatures.get(i)?;
        let input = &mut copy.inner_mut().inputs[i];
        // The finalizer is given exactly the signatures the final fields
        // carry, not the sent PSBT's own beside them.
        clear_signatures(input);
        for sig in read.signatures.iter().filter(|s| s.finalized) {
            put_signature(input, sig)?;
        }
        // Its result is not what is read: it may fail extracting the
        // transaction after this input is finalized.
        let _ = finalize(&mut copy);
        has_final(&copy.inner().inputs[i]).then_some(copy)
    }

    /// S1 to S4n, S7 and S8, input `i`.
    fn signatures(&mut self, i: usize) {
        let list = |insp: &Inspection| {
            insp.signatures
                .get(i)
                .map(|s| s.signatures.clone())
                .unwrap_or_default()
        };
        let (sent_sigs, returned_sigs) =
            (list(&self.sent_inspection), list(&self.returned_inspection));
        let sent_final = has_final(&self.sent.inner().inputs[i]);
        let returned_final = has_final(&self.returned.inner().inputs[i]);
        let outputs = self.returned.unsigned_tx().output.len();
        let at = Where::Input(i);
        for r in &returned_sigs {
            match sent_sigs
                .iter()
                .find(|s| same_signature(s, r) && same_form(s, r))
            {
                // Final on both: F3 says so, when the bytes differ.
                Some(s) if s.bytes != r.bytes && !(s.finalized && r.finalized) => {
                    self.push(at, Kind::SignatureChanged { key: r.name() });
                }
                Some(_) => {}
                None => self.added_signature(i, r, outputs),
            }
        }
        // Two signatures of Returned's under one key and leaf with other
        // bytes, in whichever fields: one of them is not the signer's.
        for (n, a) in returned_sigs.iter().enumerate() {
            for b in &returned_sigs[n + 1..] {
                let key = a.name();
                let said = |f: &Finding| {
                    f.at == at && matches!(&f.kind, Kind::SignatureChanged { key: k } if *k == key)
                };
                if a.key != SigKey::Unknown
                    && same_signature(a, b)
                    && a.bytes != b.bytes
                    && !self.findings.iter().any(said)
                {
                    self.push(at, Kind::SignatureChanged { key });
                }
            }
        }
        if sent_final && !returned_final {
            return;
        }
        for s in &sent_sigs {
            if !returned_sigs
                .iter()
                .any(|r| same_signature(s, r) && same_form(s, r))
            {
                self.push(at, Kind::SignatureRemoved { key: s.name() });
            }
        }
    }

    /// S1 to S4n for one signature the sent PSBT did not carry.
    fn added_signature(&mut self, i: usize, sig: &SignaturePresent, outputs: usize) {
        let at = Where::Input(i);
        let key = sig.name();
        match sig.verdict {
            SigVerdict::Valid => {}
            SigVerdict::Invalid => self.push(at, Kind::SignatureInvalid { key: key.clone() }),
            SigVerdict::Unchecked(reason) => self.push(
                at,
                Kind::SignatureUnchecked {
                    key: key.clone(),
                    reason: NotChecked::Signature(reason),
                },
            ),
        }
        let sent_input = &self.sent.inner().inputs[i];
        if sig.key != SigKey::Unknown
            && (!names(sent_input, &sig.key) || !form_held(sent_input, &sig.key))
        {
            self.push(at, Kind::SignatureUnnamed { key: key.clone() });
        }
        if let Some(leaf) = sig.leaf
            && !sent_holds_leaf(sent_input, &sig.key, leaf)
        {
            self.push(at, Kind::SignatureLeafUnknown { key: key.clone() });
        }
        let asked = self.sent_inspection.inputs.get(i).map(|x| x.sighash);
        if is_single(sig.sighash) && i >= outputs {
            // A SINGLE signature with no output at its index commits to
            // none, whatever is asked.
            let (signed, key) = (sig.sighash, key.clone());
            let asked = asked.unwrap_or(signed);
            self.push(at, Kind::SighashNone { key, signed, asked });
        } else if let Some(asked) = asked
            && !sighash_answers(asked, sig.sighash)
        {
            let (signed, key) = (sig.sighash, key.clone());
            let kind = if signed.is_none() {
                Kind::SighashNone { key, signed, asked }
            } else if sighash_stricter(asked, signed) {
                Kind::SighashStricter { key, signed, asked }
            } else {
                Kind::SighashOther { key, signed, asked }
            };
            self.push(at, kind);
        }
        self.added_sighashes.push(sig.sighash);
        self.added.push(AddedSignature {
            input: i,
            key: sig.key.clone(),
            leaf: sig.leaf,
            name: key,
            fingerprint: origin_fingerprint(sent_input, &sig.key),
            bytes: sig.bytes.clone(),
        });
    }

    fn global_pairs(&mut self) {
        let (s, r) = (self.sent.inner(), self.returned.inner());
        self.unknown_pairs(Where::Transaction, &s.unknown, &r.unknown, None, |_| true);
        self.proprietary_pairs(Where::Transaction, &s.proprietary, &r.proprietary, None);
    }

    fn output_pairs(&mut self, i: usize) {
        let (s, r): (&Output, &Output) = (
            &self.sent.inner().outputs[i],
            &self.returned.inner().outputs[i],
        );
        let at = Where::Output(i);
        self.unknown_pairs(at, &s.unknown, &r.unknown, None, |t| {
            t != OUT_PARTICIPANT_PUBKEYS
        });
        self.proprietary_pairs(at, &s.proprietary, &r.proprietary, None);
        let (sp, rp) = (
            unknown_of(&s.unknown, |t| t == OUT_PARTICIPANT_PUBKEYS),
            unknown_of(&r.unknown, |t| t == OUT_PARTICIPANT_PUBKEYS),
        );
        self.musig_pairs(at, FieldName::MusigParticipants, &sp, &rp);
    }

    fn input_pairs(&mut self, i: usize) {
        let (s, r): (&Input, &Input) = (
            &self.sent.inner().inputs[i],
            &self.returned.inner().inputs[i],
        );
        let at = Where::Input(i);
        // What the finalizer drops from the sent input may be gone.
        let kept = self.finalized_as.get(&i).map(|p| {
            let k = &p.inner().inputs[i];
            (k.unknown.clone(), k.proprietary.clone())
        });
        self.unknown_pairs(
            at,
            &s.unknown,
            &r.unknown,
            kept.as_ref().map(|k| &k.0),
            |t| ![IN_PARTICIPANT_PUBKEYS, IN_PUB_NONCE, IN_PARTIAL_SIG].contains(&t),
        );
        self.proprietary_pairs(
            at,
            &s.proprietary,
            &r.proprietary,
            kept.as_ref().map(|k| &k.1),
        );
        let (sp, rp) = (
            unknown_of(&s.unknown, |t| t == IN_PARTICIPANT_PUBKEYS),
            unknown_of(&r.unknown, |t| t == IN_PARTICIPANT_PUBKEYS),
        );
        self.musig_pairs(at, FieldName::MusigParticipants, &sp, &rp);
        self.musig_added(i, true);
        self.musig_added(i, false);
    }

    /// P1, P2 and P5: unknown pairs of the types `keep` lets through. With
    /// `kept`, the map a finalizer leaves, a pair it drops may be gone.
    fn unknown_pairs(
        &mut self,
        at: Where,
        sent: &BTreeMap<raw::Key, Vec<u8>>,
        returned: &BTreeMap<raw::Key, Vec<u8>>,
        kept: Option<&BTreeMap<raw::Key, Vec<u8>>>,
        keep: impl Fn(u8) -> bool,
    ) {
        let (s, r) = (unknown_of(sent, &keep), unknown_of(returned, &keep));
        for change in map_diff(&s, &r) {
            let kind = match change {
                Change::Added(k, v) => Kind::UnknownAdded {
                    type_value: k.type_value,
                    key_len: k.key.len(),
                    value_len: v.len(),
                },
                Change::Changed(k, a, b) => Kind::UnknownChanged {
                    type_value: k.type_value,
                    from: a.len(),
                    to: b.len(),
                },
                Change::Removed(k) if kept.is_some_and(|m| !m.contains_key(k)) => continue,
                Change::Removed(k) => Kind::UnknownRemoved {
                    type_value: k.type_value,
                },
            };
            self.push(at, kind);
        }
    }

    /// P3, P4 and P6; with `kept`, the map a finalizer leaves, a pair it
    /// drops may be gone.
    fn proprietary_pairs(
        &mut self,
        at: Where,
        sent: &BTreeMap<raw::ProprietaryKey, Vec<u8>>,
        returned: &BTreeMap<raw::ProprietaryKey, Vec<u8>>,
        kept: Option<&BTreeMap<raw::ProprietaryKey, Vec<u8>>>,
    ) {
        let (s, r) = (sent.iter().collect(), returned.iter().collect());
        for change in map_diff(&s, &r) {
            let kind = match change {
                Change::Added(k, v) => Kind::ProprietaryAdded {
                    prefix_len: k.prefix.len(),
                    subtype: u64::from(k.subtype),
                    value_len: v.len(),
                },
                Change::Changed(k, a, b) => Kind::ProprietaryChanged {
                    prefix_len: k.prefix.len(),
                    subtype: u64::from(k.subtype),
                    from: a.len(),
                    to: b.len(),
                },
                Change::Removed(k) if kept.is_some_and(|m| !m.contains_key(k)) => continue,
                Change::Removed(k) => Kind::ProprietaryRemoved {
                    prefix_len: k.prefix.len(),
                    subtype: u64::from(k.subtype),
                },
            };
            self.push(at, kind);
        }
    }

    /// M1 to M3 for a MuSig2 field that is read whole (participants).
    fn musig_pairs(
        &mut self,
        at: Where,
        field: FieldName,
        sent: &BTreeMap<&raw::Key, &Vec<u8>>,
        returned: &BTreeMap<&raw::Key, &Vec<u8>>,
    ) {
        for change in map_diff(sent, returned) {
            let kind = match change {
                Change::Added(..) => Kind::FieldAdded(field),
                Change::Changed(..) => Kind::FieldChanged(field),
                Change::Removed(_) if self.finalized_in_returned(at) => continue,
                Change::Removed(_) => Kind::FieldRemoved(field),
            };
            self.push(at, kind);
        }
    }

    /// Whether `at` is an input that is final on Returned. BIP-174's Input
    /// Finalizer clears every field but the UTXO and unknown ones, and
    /// BIP-373's MuSig2 fields are known to a reader of it, so their
    /// removal there is the finalizer's, not a finding.
    fn finalized_in_returned(&self, at: Where) -> bool {
        matches!(at, Where::Input(i) if has_final(&self.returned.inner().inputs[i]))
    }

    /// The master fingerprint of the sent PSBT's origin for a participant,
    /// else the first eight hex characters of its key.
    fn participant_name(&self, i: usize, participant: &PublicKey) -> String {
        let x = participant.x_only_public_key().0;
        match self.sent.inner().inputs[i].tap_key_origins.get(&x) {
            Some((_, (fp, _))) => format!("{}", Fingerprint::from(*fp)),
            None => hex8(&participant.serialize()),
        }
    }

    /// Public nonces (`nonces`) or partial signatures of input `i`: one
    /// added for a participant the sent PSBT names, where it carries none
    /// from it, is expected; anything else is a field finding.
    fn musig_added(&mut self, i: usize, nonces: bool) {
        let ty = if nonces { IN_PUB_NONCE } else { IN_PARTIAL_SIG };
        let field = if nonces {
            FieldName::MusigNonce
        } else {
            FieldName::MusigPartialSig
        };
        let (s, r) = (
            &self.sent.inner().inputs[i],
            &self.returned.inner().inputs[i],
        );
        let (sp, rp) = (
            unknown_of(&s.unknown, |t| t == ty),
            unknown_of(&r.unknown, |t| t == ty),
        );
        let named = musig::input_participants(s).unwrap_or_default();
        let output_key = classify(self.sent.inner(), i)
            .utxo
            .and_then(|o| {
                o.script_pubkey
                    .is_p2tr()
                    .then(|| o.script_pubkey.as_bytes()[2..34].to_vec())
            })
            .and_then(|b| XOnlyPublicKey::from_slice(&b).ok());
        let musig = self
            .sent_inspection
            .inputs
            .iter()
            .find(|x| x.index == i)
            .and_then(|x| x.musig.clone());
        let mut from: Vec<PublicKey> = sp.keys().filter_map(|k| pair_participant(k)).collect();
        let at = Where::Input(i);
        for change in map_diff(&sp, &rp) {
            let (key, value) = match change {
                Change::Changed(..) => {
                    self.push(at, Kind::FieldChanged(field));
                    continue;
                }
                Change::Removed(_) => {
                    if !self.finalized_in_returned(at) {
                        self.push(at, Kind::FieldRemoved(field));
                    }
                    continue;
                }
                Change::Added(k, v) => (k, v),
            };
            let readable = if nonces {
                faraday_bip::musig::PubNonce::from_bytes(value).is_ok()
            } else {
                faraday_bip::musig::PartialSig::from_bytes(value).is_ok()
            };
            let who = named_pair_participant(key, &named, s, output_key, musig.as_ref())
                .filter(|p| readable && !from.contains(p));
            let Some(who) = who else {
                self.push(at, Kind::FieldAdded(field));
                continue;
            };
            from.push(who);
            let name = self.participant_name(i, &who);
            if nonces {
                self.info.push(Info::NonceAdded {
                    input: i,
                    key: name,
                });
                continue;
            }
            self.partial_sigs_added += 1;
            // BIP-373's partial signature states no sighash: the input's own.
            if let Some(x) = self.sent_inspection.inputs.get(i) {
                self.added_sighashes.push(x.sighash);
            }
            match self.partial_sig_check(i, &who, value) {
                Ok(()) => {}
                Err(None) => self.push(at, Kind::SignatureInvalid { key: name }),
                Err(Some(reason)) => {
                    self.push(at, Kind::SignatureUnchecked { key: name, reason });
                }
            }
        }
    }

    /// Whether one participant's partial signature is theirs over this
    /// transaction (BIP-327 `PartialSigVerify`), on the returned PSBT's
    /// nonces and participants: `Err(None)` where it is not, and
    /// `Err(Some(reason))` where it could not be checked.
    fn partial_sig_check(
        &self,
        i: usize,
        who: &PublicKey,
        value: &[u8],
    ) -> Result<(), Option<NotChecked>> {
        let returned = &self.returned.inner().inputs[i];
        let session = self
            .returned_inspection
            .inputs
            .get(i)
            .and_then(|x| x.musig.as_ref())
            .ok_or(Some(NotChecked::MusigWallet))?;
        let index = session
            .participants
            .iter()
            .position(|p| p == who)
            .ok_or(Some(NotChecked::MusigWallet))?;
        let all = musig::pub_nonces(returned).map_err(|_| Some(NotChecked::MusigNonce))?;
        let nonces: Vec<_> = session
            .participants
            .iter()
            .map(|p| all.iter().find(|e| e.participant == *p).map(|e| e.value))
            .collect::<Option<_>>()
            .ok_or(Some(NotChecked::MusigNonce))?;
        let no_prevout = Some(NotChecked::Signature(Unchecked::NoPrevout));
        let ty = match returned.sighash_type {
            Some(t) => t.taproot_hash_ty().map_err(|_| no_prevout)?,
            None => TapSighashType::Default,
        };
        let cl = classify(self.returned.inner(), i);
        if cl.utxo.is_none() {
            return Err(no_prevout);
        }
        let prevouts = tap_prevouts(self.returned, &cl, i, ty).map_err(|_| no_prevout)?;
        let tx = self.returned.unsigned_tx().clone();
        let hash = SighashCache::new(&tx)
            .taproot_key_spend_signature_hash(i, &prevouts.as_ref(), ty)
            .map_err(|_| no_prevout)?;
        let partial =
            faraday_bip::musig::PartialSig::from_bytes(value).map_err(|_| None::<NotChecked>)?;
        faraday_bip::musig::partial_sig_verify(
            &Secp256k1::verification_only(),
            &partial,
            &nonces,
            &session.participants,
            &session.tweaks,
            &hash.to_byte_array(),
            index,
        )
        .map_err(|_| None)
    }
}

/// Which stated field (F199) the field `field` added at `at` is, apart
/// from the sighash type (which reads the signatures): its content is
/// checked in full against the unchanged transaction or Sent.
fn stated_field(sent: &Psbt, returned: &Psbt, at: Where, field: Field) -> Option<Stated> {
    let (sent_inner, ret) = (sent.inner(), returned.inner());
    match (at, field) {
        (Where::Output(i), Field::WitnessScript) => {
            let script = ret.outputs.get(i)?.witness_script.as_ref()?;
            let absent = sent_inner.outputs.get(i)?.witness_script.is_none();
            let spk = &sent_inner.unsigned_tx.output.get(i)?.script_pubkey;
            (absent && witness_script_of(script, spk)).then_some(Stated::WitnessScript)
        }
        (Where::Output(i), Field::RedeemScript) => {
            let script = ret.outputs.get(i)?.redeem_script.as_ref()?;
            let absent = sent_inner.outputs.get(i)?.redeem_script.is_none();
            let spk = &sent_inner.unsigned_tx.output.get(i)?.script_pubkey;
            (absent && redeem_script_of(script, spk)).then_some(Stated::RedeemScript)
        }
        (Where::Input(i), field) => {
            let (s, r) = (sent_inner.inputs.get(i)?, ret.inputs.get(i)?);
            let previous = sent.unsigned_tx().input.get(i)?.previous_output;
            match field {
                Field::WitnessScript if s.witness_script.is_none() => {
                    let spk = &sent_spent_output(sent, i)?.script_pubkey;
                    witness_script_of(r.witness_script.as_ref()?, spk)
                        .then_some(Stated::WitnessScript)
                }
                Field::RedeemScript if s.redeem_script.is_none() => {
                    let spk = &sent_spent_output(sent, i)?.script_pubkey;
                    redeem_script_of(r.redeem_script.as_ref()?, spk).then_some(Stated::RedeemScript)
                }
                Field::SpentOutput if s.witness_utxo.is_none() => {
                    let tx = s.non_witness_utxo.as_ref()?;
                    let held = (tx.compute_txid() == previous.txid)
                        .then(|| tx.output.get(previous.vout as usize))
                        .flatten()?;
                    (r.witness_utxo.as_ref()? == held).then_some(Stated::SpentOutput)
                }
                Field::PreviousTransaction if s.non_witness_utxo.is_none() => {
                    let tx = r.non_witness_utxo.as_ref()?;
                    // The txid leaves the witnesses out, so a matching
                    // txid says nothing of them: read them.
                    (tx.compute_txid() == previous.txid
                        && tx.input.iter().all(|x| x.witness.is_empty()))
                    .then_some(Stated::PreviousTransaction)
                }
                Field::InternalKey | Field::MerkleRoot => {
                    // Sent's own value may stand for the one Returned
                    // repeats; the spent output's script is Sent's.
                    let internal = r.tap_internal_key?;
                    let spk = &sent_spent_output(sent, i)?.script_pubkey;
                    let program = spk.is_p2tr().then(|| &spk.as_bytes()[2..34])?;
                    let secp = Secp256k1::verification_only();
                    let key = internal.tap_tweak(&secp, r.tap_merkle_root).0;
                    (key.to_x_only_public_key().serialize() == program)
                        .then_some(Stated::TaprootKey)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// The output input `i` spends, from Sent's own UTXO fields only: a
/// non-witness UTXO with the previous txid, or a witness UTXO.
fn sent_spent_output(sent: &Psbt, i: usize) -> Option<&bitcoin::TxOut> {
    let input = sent.inner().inputs.get(i)?;
    let previous = sent.unsigned_tx().input.get(i)?.previous_output;
    if let Some(tx) = &input.non_witness_utxo
        && tx.compute_txid() == previous.txid
    {
        return tx.output.get(previous.vout as usize);
    }
    input.witness_utxo.as_ref()
}

/// Two signatures are the same one's: the same key, leaf and algorithm,
/// or, where no key is known, the same bytes.
fn same_signature(a: &SignaturePresent, b: &SignaturePresent) -> bool {
    if a.key == SigKey::Unknown || b.key == SigKey::Unknown {
        return a.key == b.key && a.bytes == b.bytes;
    }
    a.key == b.key && a.leaf == b.leaf && a.kind == b.kind
}

/// Whether two signatures under one key are filed under the same form of
/// it, as a field writes it: a compressed key and its uncompressed form are
/// one key but not one field, and a finalizer looks up the one Sent names.
fn same_form(a: &SignaturePresent, b: &SignaturePresent) -> bool {
    match (&a.key, &b.key) {
        (SigKey::Ecdsa(x), SigKey::Ecdsa(y)) => x.compressed == y.compressed,
        _ => true,
    }
}

/// Whether `script`'s SHA256 is the P2WSH program of `spk`, or of the
/// P2WSH script `spk`, a P2SH script, hashes.
fn witness_script_of(script: &ScriptBuf, spk: &ScriptBuf) -> bool {
    let p2wsh = ScriptBuf::new_p2wsh(&script.wscript_hash());
    *spk == p2wsh || *spk == ScriptBuf::new_p2sh(&p2wsh.script_hash())
}

/// Whether `script`'s HASH160 is the one the P2SH script `spk` holds.
fn redeem_script_of(script: &ScriptBuf, spk: &ScriptBuf) -> bool {
    *spk == ScriptBuf::new_p2sh(&script.script_hash())
}

/// The byte a signature states for its sighash, as a field states it.
fn sighash_byte(kind: SighashKind) -> u32 {
    match kind {
        SighashKind::Ecdsa(t) => t as u32,
        SighashKind::Taproot(t) => t as u32,
        SighashKind::NonStandard(n) => n,
    }
}

/// A `SINGLE` sighash, with or without `ANYONECANPAY`.
fn is_single(kind: SighashKind) -> bool {
    match kind {
        SighashKind::Ecdsa(t) => matches!(
            t,
            EcdsaSighashType::Single | EcdsaSighashType::SinglePlusAnyoneCanPay
        ),
        SighashKind::Taproot(t) => matches!(
            t,
            TapSighashType::Single | TapSighashType::SinglePlusAnyoneCanPay
        ),
        SighashKind::NonStandard(_) => false,
    }
}

/// Whether the sent input holds `leaf`: in its `tap_scripts`, or in the
/// leaf list of `key`'s origin.
fn sent_holds_leaf(input: &Input, key: &SigKey, leaf: TapLeafHash) -> bool {
    in_tap_scripts(input, leaf)
        || matches!(key, SigKey::Schnorr(x)
            if input.tap_key_origins.get(x).is_some_and(|(leaves, _)| leaves.contains(&leaf)))
}

fn in_tap_scripts(input: &Input, leaf: TapLeafHash) -> bool {
    input
        .tap_scripts
        .values()
        .any(|(script, version)| TapLeafHash::from_script(script, *version) == leaf)
}

/// Whether the form `key` is written in is one Sent's scripts hold. Where a
/// script holds the key's point (its P2PKH hash, or its bytes in a redeem
/// or witness script), the form written must be a form a script holds;
/// where none does, only the compressed form is, as a P2WPKH or a bare key
/// origin names it.
fn form_held(input: &Input, key: &SigKey) -> bool {
    let SigKey::Ecdsa(pk) = key else { return true };
    let other = bitcoin::PublicKey {
        compressed: !pk.compressed,
        inner: pk.inner,
    };
    let holds = |form: &bitcoin::PublicKey| {
        let bytes = form.to_bytes();
        let in_script = |script: &Option<ScriptBuf>| {
            script
                .as_ref()
                .is_some_and(|s| s.as_bytes().windows(bytes.len()).any(|w| w == bytes))
        };
        let hash = form.pubkey_hash();
        let p2pkh = |spk: &ScriptBuf| *spk == ScriptBuf::new_p2pkh(&hash);
        in_script(&input.redeem_script)
            || in_script(&input.witness_script)
            || input
                .witness_utxo
                .as_ref()
                .is_some_and(|u| p2pkh(&u.script_pubkey))
            || input
                .non_witness_utxo
                .as_ref()
                .is_some_and(|tx| tx.output.iter().any(|o| p2pkh(&o.script_pubkey)))
    };
    if holds(pk) {
        true
    } else if holds(&other) {
        false
    } else {
        pk.compressed
    }
}

/// Whether the sent input names `key`: a key origin for it, or for a
/// taproot key path, an internal key with an origin whose output key it
/// is.
fn names(input: &Input, key: &SigKey) -> bool {
    match key {
        SigKey::Ecdsa(pk) => input.bip32_derivation.contains_key(&pk.inner),
        SigKey::Schnorr(x) => input.tap_key_origins.contains_key(x) || key_path_key(input, x),
        SigKey::Unknown => false,
    }
}

fn key_path_key(input: &Input, output_key: &XOnlyPublicKey) -> bool {
    let Some(internal) = input.tap_internal_key else {
        return false;
    };
    if !input.tap_key_origins.contains_key(&internal) {
        return false;
    }
    let secp = Secp256k1::verification_only();
    internal
        .tap_tweak(&secp, input.tap_merkle_root)
        .0
        .to_x_only_public_key()
        == *output_key
}
