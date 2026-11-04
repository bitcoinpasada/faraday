//! What a signature already on a PSBT is checked for.
//!
//! Nothing here signs. [`verify_signatures`] reads every signature a
//! transaction carries — the partial-signature fields and the ones
//! inside a finalized `scriptSig` or witness — and checks each against
//! the key it is under and the sighash the input's own fields give.
//! [`repeated_nonces`] looks for one `r` (or one `R`) used twice under
//! any keys, or one point of a MuSig2 public nonce in two places, which
//! is the key itself in two pieces.
//! [`deterministic`] recomputes a signature under a key this device
//! holds and says which nonce rule produced it. [`compare`] diffs two
//! PSBTs, and [`inscriptions`] states an `ord` envelope without
//! rendering anything.
//!
//! The sighash is the one [`crate::sign`] computes: the ECDSA path goes
//! through `bitcoin`'s own `Psbt::sighash_ecdsa` and the taproot path
//! through [`crate::sign::tap_prevouts`], so a signature is checked
//! against the message this crate would have signed. A finalized input
//! has had its `redeem_script` and `witness_script` cleared, so they are
//! read back out of the final fields before the sighash is computed:
//! the reverse of what [`crate::finalize`] wrote.
//!
//! The only door private material comes through is [`deterministic`]'s
//! `keys`, and what it returns is a verdict, never a key.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use bitcoin::key::TapTweak;
use bitcoin::opcodes::all::{OP_DUP, OP_EQUALVERIFY, OP_HASH160};
use bitcoin::script::Instruction;
use bitcoin::secp256k1::{self, Keypair, Message, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TapLeafHash};
use bitcoin::{PublicKey, Script, ScriptBuf, Sequence, TxOut, Witness, ecdsa, taproot};
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::work::Compute;

use crate::classify::{Classified, classify, spent_output};
use crate::context::Context;
use crate::inspect::SighashKind;
use crate::psbt::Psbt;
use crate::reason::Reason;
use crate::sign::{SigKind, tap_prevouts};
use crate::v2;

/// The key a signature is under, as the transaction names it. One key is
/// one point: the compressed and the uncompressed form of a public key
/// are equal, so a signature filed under either is under the same key.
#[derive(Debug, Clone, Eq)]
pub enum SigKey {
    /// A compressed public key, which is what an ECDSA signature is
    /// checked against.
    Ecdsa(PublicKey),
    /// An x-only key: the output key for a taproot key path, the leaf's
    /// own key for a script path.
    Schnorr(XOnlyPublicKey),
    /// The script does not name the key, so there is nothing to check
    /// the signature against.
    Unknown,
}

impl PartialEq for SigKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SigKey::Ecdsa(a), SigKey::Ecdsa(b)) => a.inner == b.inner,
            (SigKey::Schnorr(a), SigKey::Schnorr(b)) => a == b,
            (SigKey::Unknown, SigKey::Unknown) => true,
            _ => false,
        }
    }
}

impl SigKey {
    /// The key in hex, whole: 33 bytes for ECDSA, 32 for x-only.
    pub fn hex(&self) -> String {
        match self {
            SigKey::Ecdsa(pk) => hex(&pk.inner.serialize()),
            SigKey::Schnorr(xonly) => hex(&xonly.serialize()),
            SigKey::Unknown => String::new(),
        }
    }
}

impl fmt::Display for SigKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.hex())
    }
}

/// Why a signature that is present could not be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unchecked {
    /// The input carries no previous output, so there is no sighash.
    NoPrevout,
    /// The input spends a script this crate does not take apart, or a
    /// script-path element that is a proven preimage, not a signature.
    UnsupportedScript,
    /// The script names no key the signature could be under.
    UnknownKey,
}

impl fmt::Display for Unchecked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}") // not shown: a developer's text; the app words each kind
    }
}

/// What checking one signature came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The signature is the key's, over this transaction.
    Valid,
    /// The key did not make this signature over this transaction: the
    /// transaction changed after it was signed, or the signer is broken.
    Invalid,
    /// Nothing was checked, for the reason named.
    Unchecked(Unchecked),
}

impl Verdict {
    /// Whether the signature was checked and did not verify.
    pub fn is_invalid(self) -> bool {
        self == Verdict::Invalid
    }
}

/// One signature the transaction carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePresent {
    /// The input it signs.
    pub input: usize,
    /// The key it is under.
    pub key: SigKey,
    /// The master fingerprint, where a key origin on the input names
    /// one. A finalized input carries no origins, so this is `None`.
    pub fingerprint: Option<Fingerprint>,
    /// Whether that fingerprint is one of this device's keys.
    pub ours: bool,
    /// ECDSA or Schnorr.
    pub kind: SigKind,
    /// The sighash the signature itself states.
    pub sighash: SighashKind,
    /// The signature as it appears in the transaction.
    pub bytes: Vec<u8>,
    /// The leaf it signs, for a taproot script path.
    pub leaf: Option<TapLeafHash>,
    /// It was read out of a final `scriptSig` or witness rather than a
    /// partial-signature field.
    pub finalized: bool,
    /// What checking it came to.
    pub verdict: Verdict,
    /// A hash of this element is pushed in the taproot leaf, so it is
    /// data and not a signature: its sighash byte is not compared with
    /// what the input asks.
    pub preimage: bool,
}

impl SignaturePresent {
    /// How a card and a row name the key: the master fingerprint where
    /// an origin gives one, and otherwise the first eight hex
    /// characters of the public key itself.
    pub fn name(&self) -> String {
        match self.fingerprint {
            Some(fp) => format!("{fp}"),
            None => short(&self.key),
        }
    }

    /// The nonce point the signature commits to: `r` for ECDSA, `R` for
    /// Schnorr. Two signatures under one key with one of these are the
    /// key.
    pub fn nonce_point(&self) -> Vec<u8> {
        match self.kind {
            SigKind::Ecdsa => match ecdsa::Signature::from_slice(&self.bytes) {
                Ok(sig) => sig.signature.serialize_compact()[..32].to_vec(),
                Err(_) => Vec::new(),
            },
            SigKind::Schnorr => self.bytes.iter().take(32).copied().collect(),
        }
    }
}

/// Every signature on one input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputSignatures {
    /// The input's position in the transaction.
    pub index: usize,
    /// Its signatures, partial-signature fields first.
    pub signatures: Vec<SignaturePresent>,
}

/// One nonce point in more than one place: an `r` or `R` under any keys
/// (compared as its 32-byte x coordinate, ECDSA and Schnorr alike), or a
/// point of a MuSig2 public nonce. The signatures that share it
/// give the private key away.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonceReuse {
    /// The keys the signatures sharing it are under (a MuSig2 nonce's
    /// participants), each once.
    pub keys: Vec<SigKey>,
    /// The raw keys of MuSig2 pairs that do not read as a public key, each
    /// once: named by their first bytes.
    pub unread: Vec<Vec<u8>>,
    /// ECDSA or Schnorr.
    pub kind: SigKind,
    /// The repeated nonce point, in hex.
    pub nonce: String,
    /// The inputs it is on, in order, each once.
    pub inputs: Vec<usize>,
}

impl NonceReuse {
    /// How a card names the keys: the first eight hex characters of each,
    /// separated by commas.
    pub fn name(&self) -> String {
        let unread = self
            .unread
            .iter()
            .map(|raw| hex(raw).chars().take(8).collect::<String>());
        let names: Vec<String> = self.keys.iter().map(short).chain(unread).collect();
        names.join(", ")
    }
}

/// The first eight hex characters of a key, which is what names it
/// where no key origin does.
fn short(key: &SigKey) -> String {
    key.hex().chars().take(8).collect()
}

/// Which rule produced a signature's nonce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceMode {
    /// RFC 6979, ground for a low `r` (`Nonce::LowR`).
    LowR,
    /// RFC 6979, first nonce (`Nonce::First`).
    First,
    /// BIP-340 with all-zero auxiliary randomness.
    Bip340,
}

impl fmt::Display for NonceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            // not wording: the rule's name, as signers call it
            NonceMode::LowR => "low R",
            NonceMode::First => "first",
            NonceMode::Bip340 => "BIP-340",
        })
    }
}

/// What recomputing one signature under a key this device holds came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Determinism {
    /// The input the signature signs.
    pub input: usize,
    /// The key it is under.
    pub key: SigKey,
    /// The leaf it signs, for a taproot script path.
    pub leaf: Option<TapLeafHash>,
    /// The master the key was derived from.
    pub fingerprint: Fingerprint,
    /// The signature as its field holds it (what [`SignaturePresent::bytes`]
    /// is): a result is for this signature, not for the place alone.
    pub signature: Vec<u8>,
    /// Whether the signature hash could be computed. `false`: no rule
    /// was tried, and `mode` is `None` for that reason; only
    /// [`deterministic_all`] gives such a result.
    pub hashed: bool,
    /// The rule that reproduced it, or `None` when none did.
    pub mode: Option<NonceMode>,
}

/// Where a difference between two PSBTs is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    /// The transaction itself: version, locktime, the counts.
    Transaction,
    /// One input.
    Input(usize),
    /// One output.
    Output(usize),
}

/// A field two PSBTs are compared on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// The transaction's version.
    Version,
    /// Its absolute locktime.
    Locktime,
    /// How many inputs it has.
    InputCount,
    /// How many outputs it has.
    OutputCount,
    /// The outpoint an input spends.
    Spends,
    /// An input's sequence number.
    Sequence,
    /// An output's amount.
    Amount,
    /// An output's script.
    Script,
    /// The transaction an input spends, whole.
    PreviousTransaction,
    /// The output an input spends.
    SpentOutput,
    /// A p2sh redeem script.
    RedeemScript,
    /// A p2wsh witness script.
    WitnessScript,
    /// The sighash type an input asks for.
    Sighash,
    /// The key origins a map carries.
    KeyOrigins,
    /// The taproot key origins.
    TaprootKeyOrigins,
    /// A taproot internal key.
    InternalKey,
    /// A taproot merkle root.
    MerkleRoot,
    /// The leaf scripts and their control blocks.
    LeafScripts,
    /// Key-value pairs this crate does not know.
    Unknown,
    /// Proprietary key-value pairs.
    Proprietary,
    /// The global extended public keys.
    Xpubs,
    /// An input's hash preimages (ripemd160, sha256, hash160, hash256).
    Preimages,
    /// An output's taproot tree.
    TapTree,
    /// The PSBT's version, and whether the file writes it.
    PsbtVersion,
    /// The fields only a version 2 PSBT has, other than the modifiable
    /// flags.
    V2Fields,
    /// `PSBT_GLOBAL_TX_MODIFIABLE`.
    ModifiableFlags,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}") // not shown: a developer's text; the comparison is a developer's
    }
}

/// One field that differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDiff {
    /// Where it is.
    pub at: Where,
    /// Which field.
    pub field: Field,
    /// The first transaction's value.
    pub a: String,
    /// The second transaction's value.
    pub b: String,
}

/// How the signing state of one input differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SigningChange {
    /// Only the first transaction has this key's signature.
    OnlyFirst(String),
    /// Only the second has it.
    OnlySecond(String),
    /// Only the first is finalized.
    FinalizedFirst,
    /// Only the second is finalized.
    FinalizedSecond,
}

/// One input's signing state, where the two differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningDiff {
    /// The input.
    pub input: usize,
    /// What differs.
    pub change: SigningChange,
}

/// Two PSBTs, field by field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Comparison {
    /// The unsigned transaction's own fields.
    pub transaction: Vec<FieldDiff>,
    /// Which keys have signed which input, and what is finalized.
    pub signing: Vec<SigningDiff>,
    /// Every other key-value pair, compared on what it decodes to, so
    /// that a map written in another order is not a difference.
    pub metadata: Vec<FieldDiff>,
}

impl Comparison {
    /// Whether the two are the same transaction, signed the same way,
    /// carrying the same metadata.
    pub fn same(&self) -> bool {
        self.transaction.is_empty() && self.signing.is_empty() && self.metadata.is_empty()
    }
}

/// An `ord` envelope in a leaf script or a witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inscription {
    /// The input whose script carries it.
    pub input: usize,
    /// The content type the envelope states, where it states one.
    pub content_type: Option<String>,
    /// The payload's size in bytes, where the envelope has a body.
    pub size: Option<usize>,
}

/// Lowercase hex.
fn hex(bytes: &[u8]) -> String {
    faraday_crypto::hex::encode_string(bytes)
}

/// One signature as it was found, before it was checked.
struct Present {
    key: SigKey,
    kind: SigKind,
    bytes: Vec<u8>,
    leaf: Option<TapLeafHash>,
    finalized: bool,
    /// A hash of the element is pushed in the finalized script-path
    /// witness's leaf: a signature valid in this leaf commits to it, so
    /// the leaf cannot hold its hash (one made for another leaf can meet
    /// this, and fails in this leaf anyway); an element that meets it and
    /// verifies under no key is data, not an invalid signature.
    preimage: bool,
    /// The keys the script names, when the signature is not filed under
    /// one: a finalized multisig witness gives signatures in script
    /// order and no keys with them.
    candidates: Vec<SigKey>,
}

/// Every signature every input carries, checked.
///
/// `ctx` is what says whether the key a signature is under is one this
/// device holds, which is what decides whether [`deterministic`] has
/// anything to say about it.
pub fn verify_signatures(psbt: &Psbt, ctx: &Context) -> Vec<InputSignatures> {
    let secp = Secp256k1::verification_only();
    (0..psbt.inner().inputs.len())
        .map(|index| InputSignatures {
            index,
            signatures: input_signatures(&secp, psbt, ctx, index),
        })
        .collect()
}

fn input_signatures<C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    psbt: &Psbt,
    ctx: &Context,
    index: usize,
) -> Vec<SignaturePresent> {
    let c = classify(psbt.inner(), index);
    let input = &psbt.inner().inputs[index];
    let present = collect(&c, input);
    let restored = restore(psbt, index, &c);
    let mismatch = witness_mismatch_of(&c, input);
    let mut out = Vec::with_capacity(present.len());
    for p in present {
        let mut preimage = false;
        let (key, mut verdict) = check(secp, &restored, &c, index, &p, &mut preimage);
        // A finalized script-path witness that does not commit to the
        // output signs nothing the output holds: no row of it is valid
        // (a proven preimage is data, not a signature).
        if mismatch && p.finalized && p.leaf.is_some() && !preimage {
            verdict = Verdict::Invalid;
        }
        let (fingerprint, ours) = origin(ctx, input, &key);
        out.push(SignaturePresent {
            input: index,
            sighash: sighash_of(&p),
            kind: p.kind,
            bytes: p.bytes,
            leaf: p.leaf,
            finalized: p.finalized,
            key,
            fingerprint,
            ours,
            verdict,
            preimage,
        });
    }
    out
}

/// A taproot signature as BIP-341 reads it: 64 bytes, or 65 with a
/// sighash byte other than 0.
fn schnorr_signature(bytes: &[u8]) -> Option<taproot::Signature> {
    if v2::is_zero_sighash_schnorr(bytes) {
        return None;
    }
    taproot::Signature::from_slice(bytes).ok()
}

/// The sighash a signature itself states.
fn sighash_of(p: &Present) -> SighashKind {
    match p.kind {
        SigKind::Ecdsa => match ecdsa::Signature::from_slice(&p.bytes) {
            Ok(sig) => SighashKind::Ecdsa(sig.sighash_type),
            Err(_) => SighashKind::NonStandard(u32::from(*p.bytes.last().unwrap_or(&0))),
        },
        SigKind::Schnorr => match schnorr_signature(&p.bytes) {
            Some(sig) => SighashKind::Taproot(sig.sighash_type),
            None => SighashKind::NonStandard(u32::from(*p.bytes.last().unwrap_or(&0))),
        },
    }
}

/// The master fingerprint a key origin on the input names for `key`, and
/// whether that master is one of this device's.
fn origin(
    ctx: &Context,
    input: &bitcoin::psbt::Input,
    key: &SigKey,
) -> (Option<Fingerprint>, bool) {
    let fp = origin_fingerprint(input, key);
    let ours = fp.is_some_and(|fp| ctx.is_ours(fp));
    (fp, ours)
}

/// The master fingerprint a key origin on the input names for `key`.
pub(crate) fn origin_fingerprint(
    input: &bitcoin::psbt::Input,
    key: &SigKey,
) -> Option<Fingerprint> {
    let fp = match key {
        SigKey::Ecdsa(pk) => input.bip32_derivation.get(&pk.inner).map(|(fp, _)| *fp),
        SigKey::Schnorr(xonly) => input
            .tap_key_origins
            .get(xonly)
            .map(|(_, (fp, _))| *fp)
            .or_else(|| {
                // A key-path signature is checked against the output
                // key, which is the internal key tweaked; the origin is
                // filed under the internal key.
                let internal = input.tap_internal_key?;
                input.tap_key_origins.get(&internal).map(|(_, (fp, _))| *fp)
            }),
        SigKey::Unknown => None,
    };
    fp.map(Fingerprint::from)
}

/// Every signature one input carries, from the partial-signature fields
/// and from a final `scriptSig` or witness.
fn collect(c: &Classified, input: &bitcoin::psbt::Input) -> Vec<Present> {
    let mut out = Vec::new();
    for (pk, sig) in &input.partial_sigs {
        out.push(Present {
            key: SigKey::Ecdsa(*pk),
            kind: SigKind::Ecdsa,
            bytes: sig.to_vec(),
            leaf: None,
            finalized: false,
            preimage: false,
            candidates: Vec::new(),
        });
    }
    if let Some(sig) = input.tap_key_sig {
        out.push(Present {
            key: key_path_key(c, input),
            kind: SigKind::Schnorr,
            bytes: sig.to_vec(),
            leaf: None,
            finalized: false,
            preimage: false,
            candidates: Vec::new(),
        });
    }
    for ((xonly, leaf), sig) in &input.tap_script_sigs {
        out.push(Present {
            key: SigKey::Schnorr(*xonly),
            kind: SigKind::Schnorr,
            bytes: sig.to_vec(),
            leaf: Some(*leaf),
            finalized: false,
            preimage: false,
            candidates: Vec::new(),
        });
    }
    out.extend(final_signatures(c, input));
    out
}

/// The key a taproot key-path signature is checked against: the output
/// key the previous output pays, or the internal key when the previous
/// output is not there to read it from.
fn key_path_key(c: &Classified, input: &bitcoin::psbt::Input) -> SigKey {
    match c.utxo.as_ref().and_then(|u| output_key(&u.script_pubkey)) {
        Some(xonly) => SigKey::Schnorr(xonly),
        None => match input.tap_internal_key {
            Some(internal) => SigKey::Schnorr(internal),
            None => SigKey::Unknown,
        },
    }
}

/// The x-only key a p2tr script pays.
fn output_key(spk: &Script) -> Option<XOnlyPublicKey> {
    if !spk.is_p2tr() {
        return None;
    }
    XOnlyPublicKey::from_slice(&spk.as_bytes()[2..34]).ok()
}

/// The script a finalized input really spends: the previous output's, or
/// for p2sh the redeem script the `scriptSig` ends in. A redeem script
/// that does not hash to the output's own hash is not the script it
/// spends: `None`.
fn effective_script(spk: &Script, script_sig: &Script) -> Option<ScriptBuf> {
    if spk.is_p2sh() {
        let redeem = pushes(script_sig).pop()?;
        let hash = <bitcoin::hashes::hash160::Hash as bitcoin::hashes::Hash>::hash(&redeem);
        let same = spk.as_bytes().get(2..22) == Some(bitcoin::hashes::Hash::as_byte_array(&hash));
        return same.then(|| ScriptBuf::from(redeem));
    }
    Some(ScriptBuf::from(spk.as_bytes().to_vec()))
}

/// Costs a classify, and a tweak when the witness is a script-path
/// spend: whether input `index`'s finalized witness is one no node
/// confirms, but for a p2sh whose redeem script is the one its output
/// hashes. It is a script-path spend of a taproot output that does not
/// commit to it (its control block does not decode, or does not tie the
/// leaf script to the output key), or a witness under an output key not
/// on the curve, or one under a p2sh whose redeem script is not the one
/// the output hashes. A key-path witness, a witness with no previous
/// output to read ([`final_unread`] says that one) and any other input
/// give `false`. The signing state, the nonce check, `collect`, `finish`
/// and the review share it.
pub fn witness_mismatch(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    witness_mismatch_of(&classify(psbt, index), input)
}

/// Costs a classify. Whether input `index` is final but what it says it
/// spends does not read or is false: no previous output stated, or a
/// previous transaction of another txid or without that output, beside a
/// `witness_utxo` or not, or a `witness_utxo` that states another output
/// than the previous transaction. Nothing of such an input can be checked
/// (a signature on it is read over the previous transaction's output, or
/// not at all), so it is neither sound nor finished.
pub fn final_unread(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    if input.final_script_sig.is_none() && input.final_script_witness.is_none() {
        return false;
    }
    let c = classify(psbt, index);
    c.utxo.is_none() || is_false(&c)
}

/// Costs a txid per input with a previous transaction. The outputs a
/// PSBT's inputs spend, in order, as [`final_unread`] and the signature
/// checks read them: the output of a previous transaction of the right
/// txid, else the `witness_utxo` where there is no previous transaction;
/// none where the previous transaction is of another txid or lacks the
/// output, or where nothing is stated.
pub fn spent_outputs(psbt: &bitcoin::Psbt) -> Vec<Option<TxOut>> {
    (0..psbt.inputs.len())
        .map(|i| spent_output_of(psbt, i))
        .collect()
}

/// Costs a txid. The output input `index` spends, as [`spent_outputs`]
/// reads it: the one answer every reader of a spent output takes.
pub fn spent_output_of(psbt: &bitcoin::Psbt, index: usize) -> Option<TxOut> {
    spent_output(psbt, index).cloned()
}

/// Costs a classify. Whether input `index`, final or not, states
/// something false of what it spends: a `witness_utxo` that states another
/// output than its previous transaction of the right txid, a previous
/// transaction of another txid, or one without the output spent. An input
/// that merely states no output is not one.
pub fn states_false(psbt: &bitcoin::Psbt, index: usize) -> bool {
    index < psbt.inputs.len() && is_false(&classify(psbt, index))
}

/// Costs a classify. Whether input `index`, final or not, has a
/// `witness_utxo` that states another output than its previous
/// transaction of the right txid.
pub fn utxos_disagree(psbt: &bitcoin::Psbt, index: usize) -> bool {
    index < psbt.inputs.len()
        && classify(psbt, index)
            .problems
            .contains(&Reason::UtxosDisagree)
}

fn is_false(c: &Classified) -> bool {
    c.problems.iter().any(|r| {
        matches!(
            r,
            Reason::PrevTxNotSpent | Reason::PrevTxNoOutput | Reason::UtxosDisagree
        )
    })
}

/// Costs a classify per input. The inputs of `psbt` whose spent output
/// does not read, and of them the ones that disagree: `(unread, disagree)`.
/// Unread is every input that states something false ([`states_false`]),
/// final or not, and every final input that states no output
/// ([`final_unread`]); disagree is every input whose `witness_utxo` and
/// previous transaction state different outputs ([`utxos_disagree`]).
pub fn unread_counts(psbt: &bitcoin::Psbt) -> (usize, usize) {
    let mut counts = (0, 0);
    for (i, input) in psbt.inputs.iter().enumerate() {
        let c = classify(psbt, i);
        let is_final = input.final_script_sig.is_some() || input.final_script_witness.is_some();
        if is_false(&c) || (is_final && c.utxo.is_none()) {
            counts.0 += 1;
        }
        if c.problems.contains(&Reason::UtxosDisagree) {
            counts.1 += 1;
        }
    }
    counts
}

fn witness_mismatch_of(c: &Classified, input: &bitcoin::psbt::Input) -> bool {
    let Some(utxo) = c.utxo.as_ref() else {
        return false;
    };
    // A native witness output takes an empty scriptSig (BIP-141, BIP-341),
    // with a witness or without.
    if utxo.script_pubkey.is_p2tr()
        && input
            .final_script_sig
            .as_ref()
            .is_some_and(|s| !s.is_empty())
    {
        return true;
    }
    let Some(witness) = input.final_script_witness.as_ref() else {
        return false;
    };
    let mut elements: Vec<&[u8]> = witness.iter().collect();
    if elements.len() >= 2 && elements.last().is_some_and(|e| e.first() == Some(&0x50)) {
        elements.pop();
    }
    let script_path = elements.len() >= 2;
    // Where nothing can say the output is a taproot one, any witness
    // stands for a spend that cannot be confirmed.
    let any_witness = !witness.is_empty();
    let script_sig = input.final_script_sig.clone().unwrap_or_default();
    let Some(effective) = effective_script(&utxo.script_pubkey, &script_sig) else {
        return any_witness;
    };
    if !effective.is_p2tr() {
        return false;
    }
    let Some(key) = output_key(&effective) else {
        return any_witness;
    };
    if !script_path {
        return false;
    }
    let control = elements.pop().unwrap_or_default();
    let script = elements.pop().unwrap_or_default();
    let secp = Secp256k1::verification_only();
    !bitcoin::taproot::ControlBlock::decode(control)
        .is_ok_and(|cb| cb.verify_taproot_commitment(&secp, key, Script::from_bytes(script)))
}

/// The pushes of a scriptSig, in order.
fn pushes(script: &Script) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for instruction in script.instructions() {
        match instruction {
            Ok(Instruction::PushBytes(b)) => out.push(b.as_bytes().to_vec()),
            Ok(Instruction::Op(_)) => out.push(Vec::new()),
            Err(_) => return Vec::new(),
        }
    }
    out
}

/// The compressed keys a script names.
fn script_keys(script: &Script) -> Vec<SigKey> {
    pushes(script)
        .iter()
        .filter(|b| b.len() == 33)
        .filter_map(|b| PublicKey::from_slice(b).ok())
        .map(SigKey::Ecdsa)
        .collect()
}

/// The x-only keys a leaf script names.
fn leaf_keys(script: &Script) -> Vec<SigKey> {
    pushes(script)
        .iter()
        .filter(|b| b.len() == 32)
        .filter_map(|b| XOnlyPublicKey::from_slice(b).ok())
        .map(SigKey::Schnorr)
        .collect()
}

/// What a taproot leaf holds that the witness rules look up, from one
/// parse of the leaf: its 20- and 32-byte pushes, and the 20-byte pushes
/// of its `DUP HASH160 <h20> EQUALVERIFY` windows, each sorted for binary
/// search. A leaf that does not parse holds nothing, so it proves nothing.
#[derive(Default)]
struct LeafFacts<'a> {
    hash_pushes: Vec<&'a [u8]>,
    pkh_hashes: Vec<&'a [u8]>,
}

impl<'a> LeafFacts<'a> {
    fn parse(script: &'a Script) -> Self {
        let Ok(ins) = script.instructions().collect::<Result<Vec<_>, _>>() else {
            return Self::default();
        };
        let mut hash_pushes: Vec<&[u8]> = ins
            .iter()
            .filter_map(|i| match i {
                Instruction::PushBytes(b) if matches!(b.len(), 20 | 32) => Some(b.as_bytes()),
                _ => None,
            })
            .collect();
        let mut pkh_hashes: Vec<&[u8]> = ins
            .windows(4)
            .filter_map(|w| match w {
                [
                    Instruction::Op(OP_DUP),
                    Instruction::Op(OP_HASH160),
                    Instruction::PushBytes(h),
                    Instruction::Op(OP_EQUALVERIFY),
                ] if h.len() == 20 => Some(h.as_bytes()),
                _ => None,
            })
            .collect();
        hash_pushes.sort_unstable();
        pkh_hashes.sort_unstable();
        Self {
            hash_pushes,
            pkh_hashes,
        }
    }

    /// The 32-byte witness elements that are x-only keys whose HASH160 is
    /// the push of a pkh window of the leaf: a `pkh` leaf holds the hash,
    /// the key is in the witness.
    fn pkh_candidates(&self, elements: &[Vec<u8>]) -> Vec<SigKey> {
        elements
            .iter()
            .filter(|e| e.len() == 32)
            .filter_map(|e| {
                let key = XOnlyPublicKey::from_slice(e).ok()?;
                let h = <bitcoin::hashes::hash160::Hash as bitcoin::hashes::Hash>::hash(e);
                let h: &[u8] = h.as_ref();
                self.pkh_hashes
                    .binary_search(&h)
                    .is_ok()
                    .then_some(SigKey::Schnorr(key))
            })
            .collect()
    }

    /// Whether a hash of `element` (SHA256 or HASH256, RIPEMD160 or
    /// HASH160) is a data push of the leaf: then the element is data,
    /// since a signature valid in this leaf commits to the leaf, so the
    /// leaf cannot hold its hash. (A signature made for another leaf can
    /// meet this; it fails in this leaf anyway.)
    fn holds_hash_of(&self, element: &[u8]) -> bool {
        use bitcoin::hashes::{Hash, hash160, ripemd160, sha256, sha256d};
        let h32 = [
            sha256::Hash::hash(element).to_byte_array(),
            sha256d::Hash::hash(element).to_byte_array(),
        ];
        let h20 = [
            ripemd160::Hash::hash(element).to_byte_array(),
            hash160::Hash::hash(element).to_byte_array(),
        ];
        let has = |h: &[u8]| self.hash_pushes.binary_search(&h).is_ok();
        h32.iter().any(|h| has(h)) || h20.iter().any(|h| has(h))
    }
}

/// The signatures inside a final `scriptSig` and witness, read the way
/// [`crate::finalize`] writes them.
fn final_signatures(c: &Classified, input: &bitcoin::psbt::Input) -> Vec<Present> {
    let Some(utxo) = c.utxo.as_ref() else {
        return Vec::new();
    };
    let script_sig = input.final_script_sig.clone().unwrap_or_default();
    let witness = input.final_script_witness.clone().unwrap_or_default();
    if script_sig.is_empty() && witness.is_empty() {
        return Vec::new();
    }
    let spk = &utxo.script_pubkey;
    let sig_pushes = pushes(&script_sig);
    // p2sh: the last push of the scriptSig is the redeem script, and
    // what is really being spent is whatever that script is.
    let redeem = if spk.is_p2sh() {
        sig_pushes.last().map(|b| ScriptBuf::from(b.clone()))
    } else {
        None
    };
    let effective = redeem.clone().unwrap_or_else(|| spk.clone());

    if effective.is_p2tr() {
        return taproot_witness(&witness);
    }
    if effective.is_p2wpkh() || effective.is_p2wsh() {
        return segwit_witness(&effective, &witness);
    }
    if spk.is_p2pkh() {
        // `<sig> <pubkey>`.
        let (sig, key) = match sig_pushes.as_slice() {
            [sig, key] => (sig.clone(), PublicKey::from_slice(key).ok()),
            _ => return Vec::new(),
        };
        return alloc::vec![Present {
            key: key.map_or(SigKey::Unknown, SigKey::Ecdsa),
            kind: SigKind::Ecdsa,
            bytes: sig,
            leaf: None,
            finalized: true,
            preimage: false,
            candidates: Vec::new(),
        }];
    }
    if let Some(redeem) = redeem {
        // `OP_0 <sig>… <redeem>`: bare multisig inside p2sh.
        let keys = script_keys(&redeem);
        return ecdsa_candidates(&sig_pushes[..sig_pushes.len().saturating_sub(1)], &keys);
    }
    Vec::new()
}

/// The signatures in a SegWit v0 witness.
fn segwit_witness(effective: &Script, witness: &Witness) -> Vec<Present> {
    let elements: Vec<Vec<u8>> = witness.iter().map(|e| e.to_vec()).collect();
    if effective.is_p2wpkh() {
        let (sig, key) = match elements.as_slice() {
            [sig, key] => (sig.clone(), PublicKey::from_slice(key).ok()),
            _ => return Vec::new(),
        };
        return alloc::vec![Present {
            key: key.map_or(SigKey::Unknown, SigKey::Ecdsa),
            kind: SigKind::Ecdsa,
            bytes: sig,
            leaf: None,
            finalized: true,
            preimage: false,
            candidates: Vec::new(),
        }];
    }
    // p2wsh: the last element is the witness script.
    let Some(script) = elements.last() else {
        return Vec::new();
    };
    let keys = script_keys(Script::from_bytes(script));
    ecdsa_candidates(&elements[..elements.len() - 1], &keys)
}

/// The signatures in a taproot witness: one element for a key path, and
/// for a script path the elements before the leaf script and its
/// control block.
fn taproot_witness(witness: &Witness) -> Vec<Present> {
    let mut elements: Vec<Vec<u8>> = witness.iter().map(|e| e.to_vec()).collect();
    // BIP-341's annex, which is not part of the spend.
    if elements.len() >= 2 && elements.last().is_some_and(|e| e.first() == Some(&0x50)) {
        elements.pop();
    }
    if elements.len() == 1 {
        return alloc::vec![Present {
            // The key path's key is the output key, which the caller
            // knows from the previous output.
            key: SigKey::Unknown,
            kind: SigKind::Schnorr,
            bytes: elements[0].clone(),
            leaf: None,
            finalized: true,
            preimage: false,
            candidates: Vec::new(),
        }];
    }
    if elements.len() < 2 {
        return Vec::new();
    }
    let control = elements.pop().unwrap_or_default();
    let script = elements.pop().unwrap_or_default();
    let version = control.first().copied().unwrap_or(0xc0) & 0xfe;
    let Ok(version) = LeafVersion::from_consensus(version) else {
        return Vec::new();
    };
    let leaf = TapLeafHash::from_script(Script::from_bytes(&script), version);
    let keys = leaf_keys(Script::from_bytes(&script));
    let facts = LeafFacts::parse(Script::from_bytes(&script));
    let pkh_keys = facts.pkh_candidates(&elements);
    let keys: Vec<SigKey> = keys.into_iter().chain(pkh_keys).collect();
    elements
        .into_iter()
        // A zero sighash byte stays in, to be reported invalid.
        .filter(|e| schnorr_signature(e).is_some() || v2::is_zero_sighash_schnorr(e))
        .map(|bytes| Present {
            key: SigKey::Unknown,
            kind: SigKind::Schnorr,
            leaf: Some(leaf),
            finalized: true,
            preimage: facts.holds_hash_of(&bytes),
            bytes,
            candidates: keys.clone(),
        })
        .collect()
}

/// The stack elements that are ECDSA signatures, with the script's keys
/// to check them against.
fn ecdsa_candidates(elements: &[Vec<u8>], keys: &[SigKey]) -> Vec<Present> {
    elements
        .iter()
        .filter(|e| ecdsa::Signature::from_slice(e).is_ok())
        .map(|bytes| Present {
            key: SigKey::Unknown,
            kind: SigKind::Ecdsa,
            bytes: bytes.clone(),
            leaf: None,
            finalized: true,
            preimage: false,
            candidates: keys.to_vec(),
        })
        .collect()
}

/// The PSBT with input `index`'s scripts put back where a finalized
/// input cleared them, so that the sighash can be computed from the
/// same fields the signer computed it from.
fn restore(psbt: &Psbt, index: usize, c: &Classified) -> bitcoin::Psbt {
    let mut inner = psbt.inner().clone();
    let input = &mut inner.inputs[index];
    let Some(utxo) = c.utxo.as_ref() else {
        return inner;
    };
    // `bitcoin`'s signature hash reads `witness_utxo` first; a node reads
    // the previous transaction, which is what `classify` holds.
    if input.witness_utxo.is_some() {
        input.witness_utxo = Some(utxo.clone());
    }
    let Some(script_sig) = input.final_script_sig.clone() else {
        if let Some(witness) = input.final_script_witness.clone() {
            restore_witness(input, &utxo.script_pubkey, &witness);
        }
        return inner;
    };
    if utxo.script_pubkey.is_p2sh()
        && input.redeem_script.is_none()
        && let Some(redeem) = pushes(&script_sig).last()
    {
        input.redeem_script = Some(ScriptBuf::from(redeem.clone()));
    }
    let effective = input
        .redeem_script
        .clone()
        .unwrap_or_else(|| utxo.script_pubkey.clone());
    if let Some(witness) = input.final_script_witness.clone() {
        restore_witness(input, &effective, &witness);
    }
    inner
}

/// The witness script a finalized p2wsh witness carries as its last
/// element.
fn restore_witness(input: &mut bitcoin::psbt::Input, effective: &Script, witness: &Witness) {
    if !effective.is_p2wsh() || input.witness_script.is_some() {
        return;
    }
    if let Some(script) = witness.last() {
        input.witness_script = Some(ScriptBuf::from(script.to_vec()));
    }
}

/// Checks one signature, and says which key it is under.
fn check<C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    restored: &bitcoin::Psbt,
    c: &Classified,
    index: usize,
    p: &Present,
    preimage: &mut bool,
) -> (SigKey, Verdict) {
    let Some(utxo) = c.utxo.as_ref() else {
        return (p.key.clone(), Verdict::Unchecked(Unchecked::NoPrevout));
    };
    match p.kind {
        SigKind::Ecdsa => {
            let Ok(sig) = ecdsa::Signature::from_slice(&p.bytes) else {
                return (
                    p.key.clone(),
                    Verdict::Unchecked(Unchecked::UnsupportedScript),
                );
            };
            let Some(msg) = ecdsa_message(restored, index, sig.sighash_type) else {
                return (
                    p.key.clone(),
                    Verdict::Unchecked(Unchecked::UnsupportedScript),
                );
            };
            let verify = |key: &SigKey| match key {
                SigKey::Ecdsa(pk) => secp.verify_ecdsa(&msg, &sig.signature, &pk.inner).is_ok(),
                _ => false,
            };
            settle(p, verify, preimage)
        }
        SigKind::Schnorr => {
            // BIP-341 refuses a 65-byte signature ending 0x00: it is
            // invalid under whichever key it is filed, named as any
            // other invalid signature is.
            if v2::is_zero_sighash_schnorr(&p.bytes) {
                let key = if p.leaf.is_none() && p.key == SigKey::Unknown {
                    output_key(&utxo.script_pubkey).map_or(SigKey::Unknown, SigKey::Schnorr)
                } else {
                    let (key, verdict) = settle(p, |_| false, preimage);
                    if *preimage {
                        return (key, verdict);
                    }
                    key
                };
                return (key, Verdict::Invalid);
            }
            let Some(sig) = schnorr_signature(&p.bytes) else {
                return (
                    p.key.clone(),
                    Verdict::Unchecked(Unchecked::UnsupportedScript),
                );
            };
            let Some(msg) = taproot_message(restored, c, index, p.leaf, sig.sighash_type) else {
                *preimage = p.preimage;
                return (
                    p.key.clone(),
                    Verdict::Unchecked(Unchecked::UnsupportedScript),
                );
            };
            let verify = |key: &SigKey| match key {
                SigKey::Schnorr(xonly) => secp.verify_schnorr(&sig.signature, &msg, xonly).is_ok(),
                _ => false,
            };
            // A finalized key-path signature is under the output key,
            // which the previous output names.
            if p.leaf.is_none() && p.key == SigKey::Unknown {
                let key = match output_key(&utxo.script_pubkey) {
                    Some(xonly) => SigKey::Schnorr(xonly),
                    None => return (SigKey::Unknown, Verdict::Unchecked(Unchecked::UnknownKey)),
                };
                let verdict = if verify(&key) {
                    Verdict::Valid
                } else {
                    Verdict::Invalid
                };
                return (key, verdict);
            }
            settle(p, verify, preimage)
        }
    }
}

/// The verdict on a signature: the key it is filed under where it has
/// one, and otherwise whichever of the script's keys it verifies under.
fn settle(p: &Present, verify: impl Fn(&SigKey) -> bool, preimage: &mut bool) -> (SigKey, Verdict) {
    if p.key != SigKey::Unknown {
        let verdict = if verify(&p.key) {
            Verdict::Valid
        } else {
            Verdict::Invalid
        };
        return (p.key.clone(), verdict);
    }
    if let Some(key) = p.candidates.iter().find(|k| verify(k)) {
        return (key.clone(), Verdict::Valid);
    }
    if p.preimage {
        // A hash of the element is in the leaf: it is data.
        *preimage = true;
        return (
            SigKey::Unknown,
            Verdict::Unchecked(Unchecked::UnsupportedScript),
        );
    }
    match p.candidates.first() {
        // The script names keys and none of them made this signature.
        Some(_) => (SigKey::Unknown, Verdict::Invalid),
        // The script names no key at all, and no `pkh` window's key is in
        // the witness, so there is nothing to check against.
        None => (SigKey::Unknown, Verdict::Unchecked(Unchecked::UnknownKey)),
    }
}

/// The ECDSA sighash for `ty`, computed the way [`crate::sign`] computes
/// it: through `bitcoin`'s own signer path, with the type the signature
/// states in place of the one the input asks for.
fn ecdsa_message(restored: &bitcoin::Psbt, index: usize, ty: EcdsaSighashType) -> Option<Message> {
    spent_output(restored, index)?;
    let mut psbt = restored.clone();
    psbt.inputs[index].sighash_type = Some(ty.into());
    let tx = psbt.unsigned_tx.clone();
    let mut cache = SighashCache::new(&tx);
    psbt.sighash_ecdsa(index, &mut cache)
        .ok()
        .map(|(msg, _)| msg)
}

/// The taproot sighash for a key path (`leaf` is `None`) or a script
/// path, over the prevouts [`crate::sign`] commits to.
fn taproot_message(
    restored: &bitcoin::Psbt,
    c: &Classified,
    index: usize,
    leaf: Option<TapLeafHash>,
    ty: TapSighashType,
) -> Option<Message> {
    let psbt = Psbt::from(restored.clone());
    let prevouts = tap_prevouts(&psbt, c, index, ty).ok()?;
    let tx = psbt.unsigned_tx().clone();
    let mut cache = SighashCache::new(&tx);
    let sighash = match leaf {
        Some(leaf) => cache
            .taproot_script_spend_signature_hash(index, &prevouts.as_ref(), leaf, ty)
            .ok()?,
        None => cache
            .taproot_key_spend_signature_hash(index, &prevouts.as_ref(), ty)
            .ok()?,
    };
    Some(Message::from(sighash))
}

/// Every repeated nonce in the transaction: one `r` or `R` in two places
/// under any keys, or one point of a MuSig2 public nonce in two places,
/// which is the private key in two pieces.
///
/// Finalized signatures count: a transaction that carries one signature
/// in a witness and another in a partial-signature field has still
/// leaked the key.
pub fn repeated_nonces(psbt: &Psbt, ctx: &Context) -> Vec<NonceReuse> {
    let mut out = nonce_reuse_among(
        verify_signatures(psbt, ctx)
            .into_iter()
            .flat_map(|i| i.signatures),
    );
    out.extend(repeated_pub_nonces(&[psbt]));
    out
}

/// A point of a MuSig2 public nonce (`PSBT_IN_MUSIG2_PUB_NONCE`, two
/// 33-byte points, compared by the 32-byte x coordinate, as a point and
/// its negation give one) that is in two places among `psbts` taken
/// together: another input, or another participant or aggregate on one input. The same pair
/// on two of `psbts` at one place is one place. Three partial signatures
/// over one nonce solve for the key (BIP-327).
pub fn repeated_pub_nonces(psbts: &[&Psbt]) -> Vec<NonceReuse> {
    /// A place: the input and the pair's key, whole.
    type Place = (usize, Vec<u8>);
    let mut points: Vec<(Vec<u8>, Vec<Place>)> = Vec::new();
    for psbt in psbts {
        for (i, input) in psbt.inner().inputs.iter().enumerate() {
            for (key, value) in &input.unknown {
                if key.type_value != crate::musig::IN_PUB_NONCE || value.len() != 66 {
                    continue;
                }
                for point in [&value[..33], &value[33..]] {
                    let place = (i, key.key.clone());
                    match points.iter_mut().find(|(p, _)| p[1..] == point[1..]) {
                        Some((_, places)) if places.contains(&place) => {}
                        Some((_, places)) => places.push(place),
                        None => points.push((point.to_vec(), alloc::vec![place])),
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for (point, places) in points {
        if places.len() < 2 {
            continue;
        }
        let mut keys: Vec<SigKey> = Vec::new();
        let mut unread: Vec<Vec<u8>> = Vec::new();
        let mut inputs: Vec<usize> = Vec::new();
        for (input, key) in &places {
            match key
                .get(..33)
                .and_then(|b| secp256k1::PublicKey::from_slice(b).ok())
            {
                Some(participant) => {
                    let participant = SigKey::Ecdsa(PublicKey::new(participant));
                    if !keys.contains(&participant) {
                        keys.push(participant);
                    }
                }
                None if !unread.contains(key) => unread.push(key.clone()),
                None => {}
            }
            if !inputs.contains(input) {
                inputs.push(*input);
            }
        }
        inputs.sort_unstable();
        out.push(NonceReuse {
            keys,
            unread,
            kind: SigKind::Schnorr,
            nonce: hex(&point[1..]),
            inputs,
        });
    }
    out
}

/// A nonce used for more than one signature, under one key or several,
/// among `signatures` taken together (each carries the input it signs).
/// Keys do not matter: sibling keys of one xpub differ by a number the
/// xpub gives, and one point is an ECDSA key in one input and an x-only
/// key in another, so the nonce is compared as its x coordinate. The same
/// bytes twice under one key, or on one input, count once.
pub(crate) fn nonce_reuse_among(
    signatures: impl IntoIterator<Item = SignaturePresent>,
) -> Vec<NonceReuse> {
    /// One signature as the search for a repeated nonce holds it.
    struct Seen {
        key: SigKey,
        kind: SigKind,
        nonce: Vec<u8>,
        bytes: Vec<u8>,
        input: usize,
    }
    let mut seen: Vec<Seen> = Vec::new();
    for sig in signatures {
        if sig.key == SigKey::Unknown {
            continue;
        }
        let nonce = sig.nonce_point();
        if nonce.is_empty() {
            continue;
        }
        // The same bytes in two places are one signature written
        // twice, not two signatures under one nonce: under one key, or on
        // one input, whichever key each field reads it under.
        if seen.iter().any(|s| {
            s.nonce == nonce && s.bytes == sig.bytes && (s.key == sig.key || s.input == sig.input)
        }) {
            continue;
        }
        seen.push(Seen {
            key: sig.key,
            kind: sig.kind,
            nonce,
            bytes: sig.bytes,
            input: sig.input,
        });
    }
    let mut out: Vec<(NonceReuse, usize)> = Vec::new();
    for found in seen {
        let nonce = hex(&found.nonce);
        match out.iter_mut().find(|(r, _)| r.nonce == nonce) {
            Some((hit, count)) => {
                *count += 1;
                if !hit.keys.contains(&found.key) {
                    hit.keys.push(found.key);
                }
                if !hit.inputs.contains(&found.input) {
                    hit.inputs.push(found.input);
                }
            }
            None => out.push((
                NonceReuse {
                    keys: alloc::vec![found.key],
                    unread: Vec::new(),
                    kind: found.kind,
                    nonce,
                    inputs: alloc::vec![found.input],
                },
                1,
            )),
        }
    }
    out.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(mut r, _)| {
            r.inputs.sort_unstable();
            r
        })
        .collect()
}

/// Costs a key derivation per signature checked (through `ecdsa_mode`
/// and `schnorr_mode`). Which nonce rule made each signature that is
/// under a key one of `keys` holds.
///
/// The key is derived from the PSBT's own origin data, the way
/// [`crate::sign`] derives it, and lives only inside this call.
pub fn deterministic(c: &Compute, psbt: &Psbt, keys: &[&MasterKey]) -> Vec<Determinism> {
    let mut all = deterministic_all(c, psbt, keys);
    all.retain(|d| d.hashed);
    all
}

/// Costs what [`deterministic`] costs. The same, with a result also for
/// a signature whose hash cannot be computed (`hashed` false, no rule
/// tried): the signer check reads from it which masters were given.
pub fn deterministic_all(c: &Compute, psbt: &Psbt, keys: &[&MasterKey]) -> Vec<Determinism> {
    let mut out = Vec::new();
    for index in 0..psbt.inner().inputs.len() {
        let cl = classify(psbt.inner(), index);
        let input = &psbt.inner().inputs[index];
        let restored = restore(psbt, index, &cl);
        for (pk, sig) in &input.partial_sigs {
            let Some((fp, path)) = input.bip32_derivation.get(&pk.inner) else {
                continue;
            };
            let Some(master) = keys
                .iter()
                .find(|k| k.fingerprint() == Fingerprint::from(*fp))
            else {
                continue;
            };
            let msg = ecdsa_message(&restored, index, sig.sighash_type);
            out.push(Determinism {
                input: index,
                key: SigKey::Ecdsa(*pk),
                leaf: None,
                fingerprint: master.fingerprint(),
                signature: sig.to_vec(),
                hashed: msg.is_some(),
                mode: msg.and_then(|m| ecdsa_mode(c, master, path, &m, &pk.inner, &sig.signature)),
            });
        }
        // One signature's skip is its own: the leaf signatures below are
        // read whatever became of the key path's.
        'key_path: {
            let Some(sig) = input.tap_key_sig else {
                break 'key_path;
            };
            let Some(internal) = input.tap_internal_key else {
                break 'key_path;
            };
            let Some((_, (fp, path))) = input.tap_key_origins.get(&internal) else {
                break 'key_path;
            };
            let Some(master) = keys
                .iter()
                .find(|k| k.fingerprint() == Fingerprint::from(*fp))
            else {
                break 'key_path;
            };
            let msg = taproot_message(&restored, &cl, index, None, sig.sighash_type);
            out.push(Determinism {
                input: index,
                key: key_path_key(&cl, input),
                leaf: None,
                fingerprint: master.fingerprint(),
                signature: sig.to_vec(),
                hashed: msg.is_some(),
                mode: msg.and_then(|m| {
                    schnorr_mode(c, master, path, &m, Some(input.tap_merkle_root), &sig)
                }),
            });
        }
        for ((xonly, leaf), sig) in &input.tap_script_sigs {
            let Some((_, (fp, path))) = input.tap_key_origins.get(xonly) else {
                continue;
            };
            let Some(master) = keys
                .iter()
                .find(|k| k.fingerprint() == Fingerprint::from(*fp))
            else {
                continue;
            };
            let msg = taproot_message(&restored, &cl, index, Some(*leaf), sig.sighash_type);
            out.push(Determinism {
                input: index,
                key: SigKey::Schnorr(*xonly),
                leaf: Some(*leaf),
                fingerprint: master.fingerprint(),
                signature: sig.to_vec(),
                hashed: msg.is_some(),
                mode: msg.and_then(|m| schnorr_mode(c, master, path, &m, None, sig)),
            });
        }
    }
    out
}

/// Costs a key derivation (through `derive` and `to_xpub`). Which RFC
/// 6979 rule reproduces an ECDSA signature, if either does.
fn ecdsa_mode(
    c: &Compute,
    master: &MasterKey,
    path: &bitcoin::bip32::DerivationPath,
    msg: &Message,
    pk: &secp256k1::PublicKey,
    sig: &secp256k1::ecdsa::Signature,
) -> Option<NonceMode> {
    let secp = master.secp();
    let derived = master.derive(c, path);
    if derived.to_xpub(c).public_key != *pk {
        return None;
    }
    let der = sig.serialize_der();
    let low_r = secp.sign_ecdsa_low_r(msg, derived.secret_key());
    if low_r.serialize_der()[..] == der[..] {
        return Some(NonceMode::LowR);
    }
    let first = secp.sign_ecdsa(msg, derived.secret_key());
    if first.serialize_der()[..] == der[..] {
        return Some(NonceMode::First);
    }
    None
}

/// Costs a key derivation (through `derive`). Whether BIP-340 with zero
/// auxiliary randomness reproduces a Schnorr signature. `tweak` is the
/// merkle root for a key-path signature — `Some(None)` for a key-only
/// wallet — and `None` for a script path, which signs under the key
/// itself.
fn schnorr_mode(
    c: &Compute,
    master: &MasterKey,
    path: &bitcoin::bip32::DerivationPath,
    msg: &Message,
    tweak: Option<Option<bitcoin::taproot::TapNodeHash>>,
    sig: &taproot::Signature,
) -> Option<NonceMode> {
    let secp = master.secp();
    let derived = master.derive(c, path);
    let mut keypair = Keypair::from_secret_key(secp, derived.secret_key());
    let mut signing = match tweak {
        Some(root) => keypair.tap_tweak(secp, root).to_keypair(),
        None => keypair,
    };
    let made = secp.sign_schnorr_with_aux_rand(msg, &signing, &[0u8; 32]);
    keypair.non_secure_erase();
    signing.non_secure_erase();
    (made.serialize() == sig.signature.serialize()).then_some(NonceMode::Bip340)
}

/// Two PSBTs, diffed: the transaction, the signing state, the metadata.
///
/// Every comparison is of decoded contents, so a PSBT whose maps were
/// written in another order is the same PSBT.
pub fn compare(a: &Psbt, b: &Psbt) -> Comparison {
    let (x, y) = (a.inner(), b.inner());
    let mut out = Comparison::default();
    let (ta, tb) = (&x.unsigned_tx, &y.unsigned_tx);
    field(
        &mut out.transaction,
        Where::Transaction,
        Field::Version,
        ta.version.0,
        tb.version.0,
    );
    field(
        &mut out.transaction,
        Where::Transaction,
        Field::Locktime,
        ta.lock_time.to_consensus_u32(),
        tb.lock_time.to_consensus_u32(),
    );
    field(
        &mut out.transaction,
        Where::Transaction,
        Field::InputCount,
        ta.input.len(),
        tb.input.len(),
    );
    field(
        &mut out.transaction,
        Where::Transaction,
        Field::OutputCount,
        ta.output.len(),
        tb.output.len(),
    );
    for i in 0..ta.input.len().min(tb.input.len()) {
        let (ia, ib) = (&ta.input[i], &tb.input[i]);
        field(
            &mut out.transaction,
            Where::Input(i),
            Field::Spends,
            ia.previous_output,
            ib.previous_output,
        );
        field(
            &mut out.transaction,
            Where::Input(i),
            Field::Sequence,
            Seq(ia.sequence),
            Seq(ib.sequence),
        );
    }
    for i in 0..ta.output.len().min(tb.output.len()) {
        let (oa, ob) = (&ta.output[i], &tb.output[i]);
        field(
            &mut out.transaction,
            Where::Output(i),
            Field::Amount,
            oa.value.to_sat(),
            ob.value.to_sat(),
        );
        field(
            &mut out.transaction,
            Where::Output(i),
            Field::Script,
            hex(oa.script_pubkey.as_bytes()),
            hex(ob.script_pubkey.as_bytes()),
        );
    }
    out.signing = signing_diff(x, y);
    out.metadata = metadata_diff(x, y);
    out
}

/// `Sequence` prints as a struct; a comparison wants the number.
#[derive(PartialEq)]
struct Seq(Sequence);

impl fmt::Display for Seq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_consensus_u32())
    }
}

fn field<T: fmt::Display + PartialEq>(
    out: &mut Vec<FieldDiff>,
    at: Where,
    name: Field,
    a: T,
    b: T,
) {
    if a != b {
        out.push(FieldDiff {
            at,
            field: name,
            a: a.to_string(),
            b: b.to_string(),
        });
    }
}

/// Which keys have signed which input, and what is finalized.
fn signing_diff(x: &bitcoin::Psbt, y: &bitcoin::Psbt) -> Vec<SigningDiff> {
    let mut out = Vec::new();
    for i in 0..x.inputs.len().min(y.inputs.len()) {
        let (a, b) = (&x.inputs[i], &y.inputs[i]);
        let mut keys_a = signers(a);
        let mut keys_b = signers(b);
        keys_a.sort();
        keys_b.sort();
        for key in &keys_a {
            if !keys_b.contains(key) {
                out.push(SigningDiff {
                    input: i,
                    change: SigningChange::OnlyFirst(key.clone()),
                });
            }
        }
        for key in &keys_b {
            if !keys_a.contains(key) {
                out.push(SigningDiff {
                    input: i,
                    change: SigningChange::OnlySecond(key.clone()),
                });
            }
        }
        let fa = a.final_script_sig.is_some() || a.final_script_witness.is_some();
        let fb = b.final_script_sig.is_some() || b.final_script_witness.is_some();
        if fa && !fb {
            out.push(SigningDiff {
                input: i,
                change: SigningChange::FinalizedFirst,
            });
        }
        if fb && !fa {
            out.push(SigningDiff {
                input: i,
                change: SigningChange::FinalizedSecond,
            });
        }
    }
    out
}

/// The keys one input's signatures are under, in hex.
fn signers(input: &bitcoin::psbt::Input) -> Vec<String> {
    let mut out: Vec<String> = input
        .partial_sigs
        .keys()
        .map(|pk| hex(&pk.inner.serialize()))
        .collect();
    if input.tap_key_sig.is_some()
        && let Some(internal) = input.tap_internal_key
    {
        out.push(hex(&internal.serialize()));
    }
    for (xonly, _) in input.tap_script_sigs.keys() {
        out.push(hex(&xonly.serialize()));
    }
    out
}

/// Every key-value pair that is not the transaction and not a
/// signature, compared on what it decodes to.
///
/// Each level is taken apart by `Input { .. }`-style patterns that name
/// every field (no `..`), so a field a later `bitcoin` adds fails to
/// compile here instead of going unread.
fn metadata_diff(x: &bitcoin::Psbt, y: &bitcoin::Psbt) -> Vec<FieldDiff> {
    let mut out = Vec::new();
    pair_up(
        &mut out,
        Where::Transaction,
        global_texts(x),
        global_texts(y),
    );
    for i in 0..x.inputs.len().min(y.inputs.len()) {
        pair_up(
            &mut out,
            Where::Input(i),
            input_texts(&x.inputs[i]),
            input_texts(&y.inputs[i]),
        );
    }
    for i in 0..x.outputs.len().min(y.outputs.len()) {
        pair_up(
            &mut out,
            Where::Output(i),
            output_texts(&x.outputs[i]),
            output_texts(&y.outputs[i]),
        );
    }
    out
}

/// The fields of two same-shaped lists, one by one.
fn pair_up(out: &mut Vec<FieldDiff>, at: Where, a: Vec<(Field, String)>, b: Vec<(Field, String)>) {
    for ((name, left), (_, right)) in a.into_iter().zip(b) {
        field(out, at, name, left, right);
    }
}

/// The global fields this comparison reads, as text.
fn global_texts(psbt: &bitcoin::Psbt) -> Vec<(Field, String)> {
    let bitcoin::Psbt {
        unsigned_tx: _, // `compare` reads it
        version: _,     // `signer::check` reads the file's own version bytes
        xpub,
        proprietary: _, // `signer::check` reads them pair by pair
        unknown,
        inputs: _,
        outputs: _,
    } = psbt;
    vec![
        (
            Field::Xpubs,
            map(xpub.iter().map(|(k, v)| (k.to_string(), origin_text(v)))),
        ),
        (Field::Unknown, unknown_text(unknown)),
    ]
}

/// One input's fields this comparison reads, as text.
fn input_texts(input: &bitcoin::psbt::Input) -> Vec<(Field, String)> {
    let bitcoin::psbt::Input {
        non_witness_utxo,
        witness_utxo,
        partial_sigs: _, // `signing_diff` and `signer::check` read the signatures
        sighash_type,
        redeem_script,
        witness_script,
        bip32_derivation,
        final_script_sig: _,     // read as the final fields, in `signer::check`
        final_script_witness: _, // as above
        ripemd160_preimages,
        sha256_preimages,
        hash160_preimages,
        hash256_preimages,
        tap_key_sig: _,     // read as a signature
        tap_script_sigs: _, // as above
        tap_scripts,
        tap_key_origins,
        tap_internal_key,
        tap_merkle_root,
        proprietary,
        unknown,
    } = input;
    let preimages = map(preimage_pairs("ripemd160", ripemd160_preimages)
        .chain(preimage_pairs("sha256", sha256_preimages))
        .chain(preimage_pairs("hash160", hash160_preimages))
        .chain(preimage_pairs("hash256", hash256_preimages)));
    vec![
        (
            Field::PreviousTransaction,
            non_witness_utxo
                .as_ref()
                .map(|t| hex(&bitcoin::consensus::serialize(t)))
                .unwrap_or_default(),
        ),
        (Field::SpentOutput, utxo_text(witness_utxo)),
        (Field::RedeemScript, script_text(redeem_script)),
        (Field::WitnessScript, script_text(witness_script)),
        (Field::Sighash, txt(sighash_type)),
        (
            Field::KeyOrigins,
            map(bip32_derivation
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), origin_text(v)))),
        ),
        (Field::Preimages, preimages),
        (Field::TaprootKeyOrigins, tap_origins_text(tap_key_origins)),
        (Field::InternalKey, txt(tap_internal_key)),
        (Field::MerkleRoot, txt(tap_merkle_root)),
        (
            Field::LeafScripts,
            map(tap_scripts.iter().map(|(k, (script, version))| {
                (
                    hex(&k.serialize()),
                    format!("{:02x}:{}", version.to_consensus(), hex(script.as_bytes())),
                )
            })),
        ),
        (Field::Unknown, unknown_text(unknown)),
        (
            Field::Proprietary,
            map(proprietary.iter().map(|(k, v)| (format!("{k:?}"), hex(v)))), // not shown: a comparison value; every byte of the key prints
        ),
    ]
}

/// One output's fields this comparison reads, as text.
fn output_texts(output: &bitcoin::psbt::Output) -> Vec<(Field, String)> {
    let bitcoin::psbt::Output {
        redeem_script,
        witness_script,
        bip32_derivation,
        tap_internal_key,
        tap_tree,
        tap_key_origins,
        proprietary: _, // `signer::check` reads them pair by pair
        unknown,
    } = output;
    let tree = tap_tree
        .as_ref()
        .map(|t| {
            t.script_leaves()
                .map(|l| {
                    format!(
                        "{}:{:02x}:{}",
                        l.merkle_branch().len(),
                        l.version().to_consensus(),
                        hex(l.script().as_bytes())
                    )
                })
                .collect::<Vec<_>>()
                .join("/")
        })
        .map(|t| format!("tree:{t}"))
        .unwrap_or_default();
    vec![
        (Field::RedeemScript, script_text(redeem_script)),
        (Field::WitnessScript, script_text(witness_script)),
        (
            Field::KeyOrigins,
            map(bip32_derivation
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), origin_text(v)))),
        ),
        (Field::InternalKey, txt(tap_internal_key)),
        (Field::TapTree, tree),
        (Field::TaprootKeyOrigins, tap_origins_text(tap_key_origins)),
        (Field::Unknown, unknown_text(unknown)),
    ]
}

/// A preimage map's pairs, each tagged with its hash's kind.
fn preimage_pairs<'a, H: fmt::Display + 'a>(
    kind: &'static str,
    preimages: &'a BTreeMap<H, Vec<u8>>,
) -> impl Iterator<Item = (String, String)> + 'a {
    preimages
        .iter()
        .map(move |(h, v)| (format!("{kind}:{h}"), hex(v)))
}

/// Taproot key origins as text: each key, its leaf hashes and its origin.
fn tap_origins_text(
    origins: &BTreeMap<
        XOnlyPublicKey,
        (
            Vec<bitcoin::taproot::TapLeafHash>,
            bitcoin::bip32::KeySource,
        ),
    >,
) -> String {
    map(origins.iter().map(|(k, (leaves, source))| {
        let leaves: Vec<String> = leaves.iter().map(ToString::to_string).collect();
        (
            hex(&k.serialize()),
            format!("{}|{}", leaves.join("+"), origin_text(source)),
        )
    }))
}

/// A map as one sorted string, so that the order the pairs were written
/// in is not a difference.
fn map(pairs: impl Iterator<Item = (String, String)>) -> String {
    let mut sorted: Vec<String> = pairs.map(|(k, v)| format!("{k}={v}")).collect();
    sorted.sort();
    sorted.join(",")
}

fn unknown_text(unknown: &BTreeMap<bitcoin::psbt::raw::Key, Vec<u8>>) -> String {
    map(unknown.iter().map(|(k, v)| (format!("{k:?}"), hex(v)))) // not shown: as above
}

fn txt<T: fmt::Debug>(value: &Option<T>) -> String {
    match value {
        Some(v) => format!("{v:?}"), // not shown: as above
        None => String::new(),
    }
}

/// A script as text: empty for an absent one, and `0x` then its hex for
/// one that is there, so that an empty script differs from none.
fn script_text(script: &Option<ScriptBuf>) -> String {
    script
        .as_ref()
        .map(|s| format!("0x{}", hex(s.as_bytes())))
        .unwrap_or_default()
}

fn utxo_text(utxo: &Option<TxOut>) -> String {
    match utxo {
        Some(u) => format!("{} {}", u.value.to_sat(), hex(u.script_pubkey.as_bytes())),
        None => String::new(),
    }
}

fn origin_text(source: &bitcoin::bip32::KeySource) -> String {
    format!("{}{}", source.0, source.1)
}

/// Every `ord` envelope the transaction's scripts carry.
///
/// Nothing is rendered: what comes back is the content type the envelope
/// states and how many bytes its body is.
pub fn inscriptions(psbt: &Psbt) -> Vec<Inscription> {
    let mut out = Vec::new();
    for (index, input) in psbt.inner().inputs.iter().enumerate() {
        let mut scripts: Vec<ScriptBuf> = Vec::new();
        for (script, _) in input.tap_scripts.values() {
            scripts.push(script.clone());
        }
        if let Some(script) = input.witness_script.clone() {
            scripts.push(script);
        }
        if let Some(witness) = input.final_script_witness.as_ref() {
            scripts.extend(witness.iter().map(|e| ScriptBuf::from(e.to_vec())));
        }
        for script in scripts {
            if let Some((content_type, size)) = envelope(&script) {
                let found = Inscription {
                    input: index,
                    content_type,
                    size,
                };
                if !out.contains(&found) {
                    out.push(found);
                }
            }
        }
    }
    out
}

/// `OP_FALSE OP_IF "ord" …`: the content type the envelope states and
/// the size of its body.
fn envelope(script: &Script) -> Option<(Option<String>, Option<usize>)> {
    let mut instructions = script.instructions().peekable();
    let mut armed = false;
    let mut opened = false;
    while let Some(Ok(instruction)) = instructions.next() {
        match instruction {
            Instruction::PushBytes(b) if b.as_bytes().is_empty() => armed = true,
            Instruction::Op(op) if armed && op == bitcoin::opcodes::all::OP_IF => {
                if let Some(Ok(Instruction::PushBytes(tag))) = instructions.peek()
                    && tag.as_bytes() == b"ord"
                {
                    instructions.next();
                    opened = true;
                    break;
                }
                armed = false;
            }
            _ => armed = false,
        }
    }
    if !opened {
        return None;
    }
    let mut content_type = None;
    let mut size: Option<usize> = None;
    let mut body = false;
    while let Some(Ok(instruction)) = instructions.next() {
        match instruction {
            Instruction::Op(op) if op == bitcoin::opcodes::all::OP_ENDIF => break,
            Instruction::PushBytes(b) if !body && b.as_bytes() == [1] => {
                if let Some(Ok(Instruction::PushBytes(value))) = instructions.next() {
                    content_type = Some(String::from_utf8_lossy(value.as_bytes()).into_owned());
                }
            }
            // The body tag is an empty push, and everything after it is
            // the payload.
            Instruction::PushBytes(b) if b.as_bytes().is_empty() => body = true,
            Instruction::PushBytes(b) if body => {
                size = Some(size.unwrap_or(0) + b.as_bytes().len());
            }
            _ => {}
        }
    }
    Some((content_type, size))
}
