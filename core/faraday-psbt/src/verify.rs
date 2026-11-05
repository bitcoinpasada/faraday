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

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;

use bitcoin::WitnessVersion;
use bitcoin::key::TapTweak;
use bitcoin::opcodes::all::{OP_DUP, OP_EQUALVERIFY, OP_HASH160};
use bitcoin::script::Instruction;
use bitcoin::secp256k1::{self, Keypair, Message, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TapLeafHash};
use bitcoin::{PublicKey, Script, ScriptBuf, Sequence, TxOut, Witness, ecdsa, taproot};
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::work::Compute;

use crate::classify::{Classified, Multisig, classify, spent_output};
use crate::context::Context;
use crate::finalize::{self, Judgement};
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
    /// The input states a key origin for the key, and it is left out of
    /// `fingerprint` because the leaf the signature is on is not run.
    pub origin_left_out: bool,
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

/// Costs a classification per input. How many signatures the PSBT
/// carries, read the way [`verify_signatures`] reads them (partial,
/// taproot key and script, and final fields), none of them checked.
pub fn count_signatures(psbt: &Psbt) -> usize {
    psbt.inner()
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            // One signature is one, by its bytes, whether it sits in a
            // partial field and again in a final witness; a hash preimage
            // in a script-path witness is not one.
            collect(&classify(psbt.inner(), index), input)
                .into_iter()
                .filter(|p| !p.preimage)
                .map(|p| p.bytes)
                .collect::<BTreeSet<_>>()
                .len()
        })
        .sum()
}

fn input_signatures<C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    psbt: &Psbt,
    ctx: &Context,
    index: usize,
) -> Vec<SignaturePresent> {
    input_rows(secp, psbt, ctx, index).0
}

/// The most finalized ECDSA signatures of one input that are verified.
/// Consensus allows 201 opcodes in a script and a signature check is one,
/// so no true spend holds more; the rows past it read Unchecked.
const ROW_BOUND: usize = 201;

/// The rows of input `index`, and the interpreter's answer on its final
/// fields (never asked of an input whose final data does not match its
/// output, or is over consensus's limits).
fn input_rows<C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    psbt: &Psbt,
    ctx: &Context,
    index: usize,
) -> (Vec<SignaturePresent>, Judgement) {
    let c = classify(psbt.inner(), index);
    let input = &psbt.inner().inputs[index];
    let filed = FiledLeaves::new();
    let separators = core::cell::OnceCell::new();
    let present = collect(&c, input);
    let restored = restore(psbt, index, &c);
    let mismatch = hand_mismatch(&c, input, false);
    // An input over consensus's limits is a mismatch whatever its rows say:
    // none of them is verified.
    let limited = mismatch && over_consensus_limits(&c, input);
    let is_final = input.final_script_sig.is_some() || input.final_script_witness.is_some();
    let judgement = match c.utxo.as_ref() {
        Some(utxo) if is_final && !mismatch => finalize::judge(psbt.inner(), index, utxo),
        _ => Judgement::NotJudged,
    };
    // A script that is not run proves no signature on it wrong.
    let unrun = judgement == Judgement::NotJudged && not_evaluated_of(&c, input);
    let mut out = Vec::with_capacity(present.len());
    let mut checked = Vec::with_capacity(present.len());
    let mut messages = BTreeMap::new();
    let mut verified = 0;
    for p in &present {
        let mut preimage = false;
        if p.finalized && p.kind == SigKind::Ecdsa {
            verified += 1;
            if limited || verified > ROW_BOUND {
                let unchecked = Verdict::Unchecked(Unchecked::UnsupportedScript);
                checked.push((p.key.clone(), unchecked, false));
                continue;
            }
        }
        let (key, verdict) = check(secp, &restored, &c, index, p, &mut preimage, &mut messages);
        checked.push((key, verdict, preimage));
    }
    // A multisig's signatures stand in the script's key order, each key
    // walked forward as `OP_CHECKMULTISIG` walks them. What the
    // interpreter accepts needs no walk of ours.
    let mismatch = mismatch
        || (judgement != Judgement::Accepts
            && out_of_order(
                final_multisig(&restored, &c, index).as_ref(),
                &present,
                &checked,
            ));
    for (p, (key, mut verdict, preimage)) in present.into_iter().zip(checked) {
        // A finalized script-path witness, or a finalized ECDSA scriptSig or
        // witness, that does not commit to the output signs nothing the
        // output holds: no row of it is valid (a proven preimage is data,
        // not a signature). A taproot key-path row stays as it verifies,
        // and so does the row of a signature in a multisig with too few of
        // them: the signature is true, the input is short.
        if mismatch && p.finalized && (p.leaf.is_some() || p.kind == SigKind::Ecdsa) && !preimage {
            verdict = Verdict::Invalid;
        }
        // An input the interpreter accepts has no signature that is wrong:
        // an element shaped like one is data.
        if judgement == Judgement::Accepts && p.finalized && verdict.is_invalid() {
            verdict = Verdict::Unchecked(Unchecked::UnsupportedScript);
        }
        if unrun && p.finalized && p.kind == SigKind::Ecdsa && verdict.is_invalid() {
            verdict = Verdict::Unchecked(Unchecked::UnsupportedScript);
        }
        // A Schnorr row of a script-path leaf that is not run: neither true
        // nor false is said. A final input whose witness does not commit to
        // the output is never `unrun`, so it stays as it reads.
        let leaf_unrun = match p.leaf {
            Some(leaf) if p.kind == SigKind::Schnorr && verdict.is_invalid() => {
                if p.finalized {
                    unrun
                } else {
                    filed_leaf_not_run(secp, &c, input, leaf, &filed)
                }
            }
            _ => false,
        };
        if leaf_unrun {
            verdict = Verdict::Unchecked(Unchecked::UnsupportedScript);
        }
        // Under `OP_CODESEPARATOR` the message worked out here is not the
        // real one, so a signature that verifies against it is no true
        // signature: it is not said to be true.
        if let Some(leaf) = p.leaf
            && p.kind == SigKind::Schnorr
            && verdict == Verdict::Valid
            && separator_leaves(input, &separators).contains(&leaf)
        {
            verdict = Verdict::Unchecked(Unchecked::UnsupportedScript);
        }
        let (fingerprint, ours) = origin(secp, ctx, &c, input, &key, p.leaf, &filed);
        let origin_left_out = fingerprint.is_none()
            && match (&key, p.leaf) {
                (SigKey::Schnorr(xonly), Some(leaf)) => {
                    input.tap_key_origins.contains_key(xonly)
                        && in_leaf(secp, &c, input, leaf, xonly, &filed)
                        && filed_leaf_not_run(secp, &c, input, leaf, &filed)
                }
                _ => false,
            };
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
            origin_left_out,
        });
    }
    (out, judgement)
}

/// The multisig script input `index`'s final data spends, where it is one
/// (up to 20 keys).
fn final_multisig(restored: &bitcoin::Psbt, c: &Classified, index: usize) -> Option<Multisig> {
    let input = &restored.inputs[index];
    let script = input
        .witness_script
        .as_deref()
        .or(input.redeem_script.as_deref())
        .or_else(|| c.utxo.as_ref().map(|u| u.script_pubkey.as_script()))?;
    Multisig::parse_wide(script)
}

/// Whether the finalized ECDSA signatures that verify under a multisig's
/// keys are not in the order `OP_CHECKMULTISIG` takes: each under a key
/// after the one the signature before it used (a key named twice is two
/// places). Only a multisig script has an order.
fn out_of_order(
    multisig: Option<&Multisig>,
    present: &[Present],
    checked: &[(SigKey, Verdict, bool)],
) -> bool {
    let Some(multisig) = multisig else {
        return false;
    };
    let mut next = 0;
    for (p, (key, verdict, _)) in present.iter().zip(checked) {
        if !p.finalized || p.kind != SigKind::Ecdsa || *verdict != Verdict::Valid {
            continue;
        }
        let SigKey::Ecdsa(pk) = key else {
            continue;
        };
        match multisig.keys[next..].iter().position(|k| k == pk) {
            Some(at) => next += at + 1,
            None => return true,
        }
    }
    false
}

/// Costs a classify, a signature check per row of the input, a control
/// block check per `tap_scripts` entry (once for the input), and per row a
/// read of its leaf's script, which under a large committing leaf is a pass
/// over that leaf for each row. The verdicts of the partial Schnorr rows of input `index`,
/// each as the review reads it with no key loaded, by the key and leaf of
/// the row; empty where the input is not there or has none.
pub fn script_row_verdicts(
    psbt: &bitcoin::Psbt,
    index: usize,
) -> Vec<(XOnlyPublicKey, TapLeafHash, Verdict)> {
    if psbt.inputs.get(index).is_none() {
        return Vec::new();
    }
    let own = Psbt::from(psbt.clone());
    let ctx = Context {
        network: faraday_bip::keys::Network::Mainnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let secp = Secp256k1::verification_only();
    input_rows(&secp, &own, &ctx, index)
        .0
        .into_iter()
        .filter(|s| !s.finalized)
        .filter_map(|s| match (s.key, s.leaf) {
            (SigKey::Schnorr(key), Some(leaf)) => Some((key, leaf, s.verdict)),
            _ => None,
        })
        .collect()
}

/// Costs a classify and a control block check. Whether a `tap_scripts`
/// entry of input `index` commits to the output the input spends: its
/// control block proves `script` a leaf of that output's key, at the
/// leaf version the block states. `false` where no output is read.
pub fn tap_entry_commits(
    psbt: &bitcoin::Psbt,
    index: usize,
    control: &bitcoin::taproot::ControlBlock,
    script: &Script,
    version: bitcoin::taproot::LeafVersion,
) -> bool {
    let Some(paid) = classify(psbt, index)
        .utxo
        .as_ref()
        .and_then(|u| output_key(&u.script_pubkey))
    else {
        return false;
    };
    control.leaf_version == version
        && control.verify_taproot_commitment(&Secp256k1::verification_only(), paid, script)
}

/// Costs a classify, a verification per signature (up to a bound of 201
/// per input, each tried against the script's keys, so up to 201 times the
/// keys a script of 10,000 bytes holds: some 58,000 verifications for an
/// input of 999 signature-shaped elements under 290 keys, none of them
/// true) and the interpreter's signature checks. The cost is per input,
/// linear in that input's fields, with no bound over the whole file. An input over consensus's limits
/// verifies nothing. Whether input `index` is final and its ECDSA signatures are
/// refused: `miniscript`'s interpreter, where the script it spends parses
/// as a miniscript, says so (and an input it accepts is never one); else
/// one of the signatures, read as the nonce check reads them, is Invalid:
/// another key's, a wrong message, a repeat or one out of the script's key
/// order. A script holding `OP_CODESEPARATOR` is left out (its message is
/// not worked out here). A taproot key-path signature that fails is
/// refused; a script-path witness is the interpreter's answer where its leaf
/// is a Tap miniscript (version 0xc0), each signature read over the message
/// that includes the annex, and else is not run and not refused here.
pub fn final_invalid(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    if input.final_script_sig.is_none() && input.final_script_witness.is_none() {
        return false;
    }
    let c = classify(psbt, index);
    let own = Psbt::from(psbt.clone());
    let restored = restore(&own, index, &c);
    let separator = |s: &Script| {
        s.instructions()
            .any(|i| matches!(i, Ok(Instruction::Op(op)) if op == bitcoin::opcodes::all::OP_CODESEPARATOR))
    };
    let restored_input = &restored.inputs[index];
    if c.utxo.as_ref().is_some_and(|u| separator(&u.script_pubkey))
        || restored_input
            .redeem_script
            .as_deref()
            .is_some_and(separator)
        || restored_input
            .witness_script
            .as_deref()
            .is_some_and(separator)
    {
        return false;
    }
    let ctx = Context {
        network: faraday_bip::keys::Network::Mainnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let secp = Secp256k1::verification_only();
    let (rows, judgement) = input_rows(&secp, &own, &ctx, index);
    let wrong = || rows.iter().any(wrong_row);
    match judgement {
        Judgement::Accepts => false,
        Judgement::Rejects => true,
        // A script not run is never refused here. Its ECDSA rows read
        // Unchecked, and so do its Schnorr rows.
        Judgement::NotJudged if not_evaluated_of(&c, input) => false,
        Judgement::NotJudged => wrong(),
    }
}

/// Costs what [`final_invalid`] costs. Whether a finalized ECDSA signature
/// of input `index` reads Invalid (it has a row of its own on the review
/// and the nonce check), as against a refusal by the script alone, such as
/// a time lock the transaction does not meet or a non-minimal push.
pub fn final_signature_wrong(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    if input.final_script_sig.is_none() && input.final_script_witness.is_none() {
        return false;
    }
    let own = Psbt::from(psbt.clone());
    let ctx = Context {
        network: faraday_bip::keys::Network::Mainnet,
        keys: &[],
        wallets: &[],
        musig_session: None,
    };
    let secp = Secp256k1::verification_only();
    input_rows(&secp, &own, &ctx, index).0.iter().any(wrong_row)
}

/// A final row that is a signature read wrong: an ECDSA one, or a taproot
/// key-path one (a script-path element shaped like a signature may be
/// data).
fn wrong_row(s: &SignaturePresent) -> bool {
    s.finalized
        && s.verdict.is_invalid()
        && (s.kind == SigKind::Ecdsa || (s.kind == SigKind::Schnorr && s.leaf.is_none()))
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

/// Costs a parse of each script and two hashes per script. Whether `pk` is
/// a key of the multisig script a finalized `p2wsh` or
/// `p2sh` input spends: the script is the one its output commits to, taken
/// from the final witness or script, or the witness or redeem script the
/// finalizer left. `classify` clears the script of a final input, so
/// [`crate::finalize::key_in_script`] knows no key of it.
fn final_script_names(c: &Classified, input: &bitcoin::psbt::Input, pk: &PublicKey) -> bool {
    let Some(utxo) = &c.utxo else {
        return false;
    };
    if input.final_script_witness.is_none() && input.final_script_sig.is_none() {
        return false;
    }
    let spk = utxo.script_pubkey.as_script();
    let mut scripts: Vec<ScriptBuf> = Vec::new();
    scripts.extend(input.witness_script.clone());
    scripts.extend(input.redeem_script.clone());
    if let Some(last) = input
        .final_script_witness
        .as_ref()
        .and_then(|w| w.last())
        .filter(|_| spk.is_p2wsh())
    {
        scripts.push(ScriptBuf::from(last.to_vec()));
    }
    if spk.is_p2sh()
        && let Some(push) = input
            .final_script_sig
            .as_ref()
            .and_then(|s| s.instructions().last())
            .and_then(Result::ok)
            .and_then(|i| match i {
                Instruction::PushBytes(b) => Some(ScriptBuf::from(b.as_bytes().to_vec())),
                Instruction::Op(_) => None,
            })
    {
        scripts.push(push);
    }
    scripts.iter().any(|script| {
        // The output commits to the witness script (`p2wsh`, or under
        // `p2sh`), or to the redeem script itself (legacy `p2sh`).
        let wsh = ScriptBuf::new_p2wsh(&script.wscript_hash());
        let committed = if spk.is_p2wsh() {
            wsh.as_script() == spk
        } else {
            spk.is_p2sh()
                && (script.to_p2sh().as_script() == spk || wsh.to_p2sh().as_script() == spk)
        };
        committed && Multisig::parse(script).is_some_and(|ms| ms.keys.contains(pk))
    })
}

/// Costs a tweak for a key-path row. The master fingerprint a key origin
/// on the input names for `key`, and whether that master is one of this
/// device's. An origin is the online side's word, so a row takes one
/// only for a key the spend really names: an ECDSA key of the script
/// being spent; a Schnorr key of the leaf the row signs (its push, or the
/// hash a `pkh` window holds); and for a key-path row the internal key's
/// origin, when the previous output's key is that internal key tweaked
/// with the input's merkle root (BIP-341). A key filed with an origin
/// that is none of these reports none.
fn origin<'a, C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    ctx: &Context,
    c: &Classified,
    input: &'a bitcoin::psbt::Input,
    key: &SigKey,
    leaf: Option<TapLeafHash>,
    filed: &FiledLeaves<'a>,
) -> (Option<Fingerprint>, bool) {
    let fp = match key {
        SigKey::Ecdsa(pk) => (crate::finalize::key_in_script(c, input, pk)
            || final_script_names(c, input, pk))
        .then(|| input.bip32_derivation.get(&pk.inner).map(|(fp, _)| *fp))
        .flatten(),
        SigKey::Schnorr(xonly) => match leaf {
            Some(leaf) => (in_leaf(secp, c, input, leaf, xonly, filed)
                && !filed_leaf_not_run(secp, c, input, leaf, filed))
            .then(|| input.tap_key_origins.get(xonly).map(|(_, (fp, _))| *fp))
            .flatten(),
            None => key_path_internal(secp, c, input, xonly)
                .and_then(|internal| input.tap_key_origins.get(&internal))
                .map(|(_, (fp, _))| *fp),
        },
        SigKey::Unknown => None,
    };
    let fp = fp.map(Fingerprint::from);
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

/// The place the input states for the key `row` is under, and the key
/// itself: a key-path row is under the output key, so the internal key it
/// is tweaked from is the one looked up. A lookup. `None` where the input
/// states no place for it. The online side's word: a wallet or a held
/// master derives the key at that place to prove it.
pub fn stated_place(
    input: &bitcoin::psbt::Input,
    row: &SignaturePresent,
) -> Option<(bitcoin::bip32::DerivationPath, StatedKey)> {
    match &row.key {
        SigKey::Ecdsa(pk) => input
            .bip32_derivation
            .get(&pk.inner)
            .map(|(_, p)| (p.clone(), StatedKey::Ecdsa(pk.inner))),
        SigKey::Schnorr(x) => {
            let key = match (row.leaf, input.tap_internal_key) {
                (Some(_), _) => *x,
                (None, Some(internal)) => internal,
                (None, None) => return None,
            };
            input
                .tap_key_origins
                .get(&key)
                .map(|(_, (_, p))| (p.clone(), StatedKey::XOnly(key)))
        }
        SigKey::Unknown => None,
    }
}

/// The key a row's stated place is claimed to derive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatedKey {
    /// A compressed key.
    Ecdsa(secp256k1::PublicKey),
    /// An x-only key.
    XOnly(XOnlyPublicKey),
}

impl StatedKey {
    /// Whether `derived` is this key.
    pub fn is(&self, derived: &secp256k1::PublicKey) -> bool {
        match self {
            StatedKey::Ecdsa(pk) => pk == derived,
            StatedKey::XOnly(x) => derived.x_only_public_key().0 == *x,
        }
    }
}

/// Costs a tweak. The internal key whose tweak with the input's merkle
/// root is the output key `output` the previous output pays: the one key
/// a key-path signature's origin may be filed under.
fn key_path_internal<C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    c: &Classified,
    input: &bitcoin::psbt::Input,
    output: &XOnlyPublicKey,
) -> Option<XOnlyPublicKey> {
    let internal = input.tap_internal_key?;
    let paid = c.utxo.as_ref().and_then(|u| output_key(&u.script_pubkey))?;
    let (tweaked, _) = internal.tap_tweak(secp, input.tap_merkle_root);
    (paid == *output && tweaked.to_x_only_public_key() == paid).then_some(internal)
}

/// Whether `key` is one of the keys the input's leaf `leaf` names: a
/// push of it, or the hash a `pkh` window of the leaf holds.
/// The leaf counts only when its control block commits to the output key
/// the previous p2tr output pays: a leaf filed in `tap_scripts` that the
/// output does not hold names no signer.
fn in_leaf<'a, C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    c: &Classified,
    input: &'a bitcoin::psbt::Input,
    leaf: TapLeafHash,
    key: &XOnlyPublicKey,
    filed: &FiledLeaves<'a>,
) -> bool {
    let Some((script, _)) = filed_leaves(secp, c, input, filed).get(&leaf) else {
        return false;
    };
    if leaf_keys(script).contains(&SigKey::Schnorr(*key)) {
        return true;
    }
    let h = <bitcoin::hashes::hash160::Hash as bitcoin::hashes::Hash>::hash(&key.serialize());
    let h: &[u8] = h.as_ref();
    LeafFacts::parse(script)
        .pkh_hashes
        .binary_search(&h)
        .is_ok()
}

/// Costs a script read per `tap_scripts` entry and one for the final
/// witness's leaf, worked out on first use and kept in `cell` (a later call
/// is a lookup). The hashes of the leaves, filed in `tap_scripts` or the
/// leaf of a final witness, whose script holds `OP_CODESEPARATOR`.
fn separator_leaves<'c>(
    input: &bitcoin::psbt::Input,
    cell: &'c core::cell::OnceCell<BTreeSet<TapLeafHash>>,
) -> &'c BTreeSet<TapLeafHash> {
    cell.get_or_init(|| separator_leaves_of(input))
}

fn separator_leaves_of(input: &bitcoin::psbt::Input) -> BTreeSet<TapLeafHash> {
    let separator = |s: &Script| {
        s.instructions()
            .any(|i| matches!(i, Ok(Instruction::Op(op)) if op == bitcoin::opcodes::all::OP_CODESEPARATOR))
    };
    let mut leaves = BTreeSet::new();
    for (script, version) in input.tap_scripts.values() {
        if separator(script) {
            leaves.insert(TapLeafHash::from_script(script, *version));
        }
    }
    let Some(witness) = input.final_script_witness.as_ref() else {
        return leaves;
    };
    let mut elements: Vec<&[u8]> = witness.iter().collect();
    if elements.len() >= 2 && elements.last().is_some_and(|e| e.first() == Some(&0x50)) {
        elements.pop();
    }
    if elements.len() < 2 {
        return leaves;
    }
    let script = Script::from_bytes(elements[elements.len() - 2]);
    let version = elements[elements.len() - 1]
        .first()
        .copied()
        .unwrap_or(0xc0)
        & 0xfe;
    if let Ok(v) = LeafVersion::from_consensus(version)
        && separator(script)
    {
        leaves.insert(TapLeafHash::from_script(script, v));
    }
    leaves
}

/// Costs a control block check per `tap_scripts` entry, once per input
/// (the map is kept in `filed`, and a later call is a lookup). The leaves
/// the input files in `tap_scripts` under a control block that commits to
/// the output key the previous p2tr output pays, each with its script and
/// whether it is not run: another leaf version, or no Tap miniscript. A
/// leaf not filed, or filed under a block that does not commit, is not
/// among them. Entries of one leaf hash hold one script and version.
fn filed_leaves<'a, 'f, C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    c: &Classified,
    input: &'a bitcoin::psbt::Input,
    filed: &'f FiledLeaves<'a>,
) -> &'f BTreeMap<TapLeafHash, (&'a Script, bool)> {
    filed.get_or_init(|| {
        let mut leaves = BTreeMap::new();
        let Some(paid) = c.utxo.as_ref().and_then(|u| output_key(&u.script_pubkey)) else {
            return leaves;
        };
        for (control, (script, version)) in &input.tap_scripts {
            let leaf = TapLeafHash::from_script(script, *version);
            if leaves.contains_key(&leaf)
                || control.leaf_version != *version
                || !control.verify_taproot_commitment(secp, paid, script)
            {
                continue;
            }
            let not_run = *version != LeafVersion::TapScript || !finalize::tap_miniscript(script);
            leaves.insert(leaf, (script.as_script(), not_run));
        }
        leaves
    })
}

/// The leaves of one input filed under a committing control block, worked
/// out on first use.
type FiledLeaves<'a> = core::cell::OnceCell<BTreeMap<TapLeafHash, (&'a Script, bool)>>;

/// Costs a control block check per `tap_scripts` entry, once per input.
/// Whether leaf `leaf` is filed in the input's `tap_scripts` under a control
/// block that commits to the output key, and is not run: another leaf
/// version, or no Tap miniscript. A leaf not filed is not one not run.
fn filed_leaf_not_run<'a, C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    c: &Classified,
    input: &'a bitcoin::psbt::Input,
    leaf: TapLeafHash,
    filed: &FiledLeaves<'a>,
) -> bool {
    filed_leaves(secp, c, input, filed)
        .get(&leaf)
        .is_some_and(|(_, not_run)| *not_run)
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

/// Costs a classify, a tweak when the witness is a script-path spend, and
/// the interpreter's signature checks (one per signature the script asks
/// for) where the script parses. Whether input `index`'s finalized
/// `scriptSig` and witness are ones no node relays: from hashes and shapes
/// for what the output commits to, and, where the script the input spends
/// parses as a miniscript, from `miniscript`'s interpreter, which runs it
/// (a script that does not parse is not run). That is a `scriptSig` or
/// witness that does not match the output it spends, or that the
/// interpreter refuses:
///
/// - p2pkh: a `scriptSig` that is not `<signature> <key>` of pushes alone,
///   with the key the output hashes and a signature that parses (a stray
///   push or a non-push opcode is refused though a miner could take it);
/// - p2wpkh: a witness not `[signature, key]` of that key;
/// - p2wsh: a witness whose last element does not hash to the program;
/// - p2sh: a `scriptSig` of other than pushes, a redeem script that is not
///   the one the output hashes, or, of a witness program, a `scriptSig`
///   that is not that one push;
/// - a multisig script (p2sh, p2wsh or bare) whose stack is not an empty
///   dummy and exactly the signatures it asks for, each parsing (a key
///   named twice may take one signature twice), and a bare p2pk whose
///   `scriptSig` is not one signature;
/// - a script over consensus's limits (10,000 bytes, 1,000 stack elements,
///   201 opcodes that are not pushes) or a p2sh redeem script over 520
///   bytes: refused before any signature is read;
/// - a version 0 witness program of another length than 20 or 32 bytes;
/// - taproot: no witness or an empty one, a key-path element that is not
///   64 or 65 bytes, a script path whose control block does not tie the
///   leaf to the output key, or a witness under an output key not on the
///   curve (a p2sh-wrapped version 1 program with no witness, or with a
///   single element that is not 64 or 65 bytes, is refused too, though
///   consensus ignores a p2sh-wrapped program of that version);
/// - a native witness output with a `scriptSig`, an output of no witness
///   kind with a witness.
///
/// A p2pkh, p2wpkh or p2pk signature, and each of a multisig's, must
/// parse with its sighash byte: one with a sighash byte consensus accepts
/// and `ecdsa::Signature` does not is refused (fail closed). A p2sh
/// `scriptSig` holding `OP_RESERVED` is refused (executing it fails). Other witness program versions, an input with no previous
/// output to read ([`final_unread`] says that one), a bare output of
/// another script and any other input give `false`. The signing state, the
/// nonce check, `collect`, `finish` and the review share it.
pub fn witness_mismatch(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    let c = classify(psbt, index);
    witness_mismatch_of(&c, input) || interpreter_refuses(psbt, index, &c)
}

/// Costs a classify. [`witness_mismatch`] without the interpreter: the
/// shape and commitment rules alone (the part of the answer that is not a
/// signature's).
pub fn witness_shape_mismatch(psbt: &bitcoin::Psbt, index: usize) -> bool {
    psbt.inputs
        .get(index)
        .is_some_and(|input| witness_mismatch_of(&classify(psbt, index), input))
}

/// Costs one signature check per signature the script asks for. Whether a
/// final input's script parses as a miniscript and its interpreter refuses
/// the final fields. An input over consensus's limits is not run.
fn interpreter_refuses(psbt: &bitcoin::Psbt, index: usize, c: &Classified) -> bool {
    let input = &psbt.inputs[index];
    let is_final = input.final_script_sig.is_some() || input.final_script_witness.is_some();
    match c.utxo.as_ref() {
        Some(utxo) if is_final && !hand_mismatch(c, input, true) => {
            finalize::judge(psbt, index, utxo) == Judgement::Rejects
        }
        _ => false,
    }
}

/// Costs a classify. Whether input `index` is final, in shape, and spends
/// a script Faraday does not evaluate: the interpreter does not judge it
/// (not a miniscript) and no hand rule settles it. A key hash output
/// (p2pkh, p2wpkh, wrapped or not), a bare p2pk and a multisig of
/// `Multisig::parse_wide`'s form in any wrapping are settled by the shape check and the signature rows, and a
/// taproot input is read by its own rules; a puzzle, an anchor, a hash
/// time lock that is no miniscript are not run, so a signature on them is
/// not known to be true.
pub fn not_evaluated(psbt: &bitcoin::Psbt, index: usize) -> bool {
    let Some(input) = psbt.inputs.get(index) else {
        return false;
    };
    not_evaluated_of(&classify(psbt, index), input)
}

fn not_evaluated_of(c: &Classified, input: &bitcoin::psbt::Input) -> bool {
    if input.final_script_sig.is_none() && input.final_script_witness.is_none() {
        return false;
    }
    let Some(utxo) = c.utxo.as_ref() else {
        return false;
    };
    if hand_mismatch(c, input, true) {
        return false;
    }
    let spk = &utxo.script_pubkey;
    let script_sig = input.final_script_sig.clone().unwrap_or_default();
    let witness = input.final_script_witness.clone().unwrap_or_default();
    let Some(effective) = effective_script(spk, &script_sig) else {
        return false;
    };
    // A bare p2pk is settled where its key parses; one off the curve or
    // hybrid is settled by nothing here, nor is a multisig with such a key
    // or over 20 keys (`Multisig::parse_wide` refuses both).
    if effective.is_p2tr() {
        // A script-path leaf is not run when it is no Tap miniscript, is of
        // another leaf version, or is the exact `pkh` leaf spent with a key
        // of an unknown type. An unknown-type key inside a larger Tap
        // miniscript leaf stays refused, though consensus takes it.
        return finalize::tap_spend(&witness).is_some_and(|t| !t.is_run());
    }
    if effective.is_p2wpkh() || spk.is_p2pkh() || spk.p2pk_public_key().is_some() {
        return false;
    }
    let script = if effective.is_p2wsh() {
        witness.last().map(Script::from_bytes)
    } else {
        Some(effective.as_script())
    };
    if script.is_some_and(|s| Multisig::parse_wide(s).is_some()) {
        return false;
    }
    !finalize::judged(spk, &script_sig, &witness)
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
    hand_mismatch(c, input, true)
}

/// The shape and commitment rules of [`witness_mismatch`], with or without
/// its count of a multisig's signatures (`short`): the rows of an input
/// read without it, as their signatures verify. A multisig's count is by
/// hand; the interpreter's answer is [`witness_mismatch`]'s.
fn hand_mismatch(c: &Classified, input: &bitcoin::psbt::Input, short: bool) -> bool {
    let short_multisig = |script: &Script, items: &[&[u8]]| short && short_multisig(script, items);
    use bitcoin::hashes::{Hash, hash160, sha256};
    let Some(utxo) = c.utxo.as_ref() else {
        return false;
    };
    if input.final_script_sig.is_none() && input.final_script_witness.is_none() {
        return false;
    }
    let spk = &utxo.script_pubkey;
    let script_sig = input.final_script_sig.clone().unwrap_or_default();
    let witness = input.final_script_witness.clone().unwrap_or_default();
    // Consensus caps a scriptSig at 10,000 bytes.
    if script_sig.len() > 10_000 {
        return true;
    }
    // A native witness output takes an empty scriptSig (BIP-141, BIP-341),
    // with a witness or without.
    if spk.is_witness_program() && !script_sig.is_empty() {
        return true;
    }
    // A p2sh scriptSig is pushes alone, as consensus requires of it: a
    // script of other opcodes runs, and these checks do not.
    if spk.is_p2sh()
        && (!script_sig.is_push_only()
            || script_sig
                .instructions()
                .any(|i| matches!(i, Ok(Instruction::Op(op)) if op.to_u8() == 0x50)))
    {
        return true;
    }
    // The script the input really spends: the output's, or the redeem
    // script of a p2sh whose hash it is.
    let Some(effective) = effective_script(spk, &script_sig) else {
        return true;
    };
    // A p2sh of a witness program has a scriptSig of exactly that one push
    // (BIP-141).
    if spk.is_p2sh() && effective.is_witness_program() {
        let mut one_push = alloc::vec![effective.len() as u8];
        one_push.extend_from_slice(effective.as_bytes());
        if script_sig.as_bytes() != one_push.as_slice() {
            return true;
        }
    }
    let elements: Vec<&[u8]> = witness.iter().collect();
    if effective.is_p2wpkh() {
        let program = &effective.as_bytes()[2..22];
        return !(elements.len() == 2
            && hash160::Hash::hash(elements[1]).as_byte_array() == program
            && is_signature(elements[0])
            && PublicKey::from_slice(elements[1]).is_ok());
    }
    if effective.is_p2wsh() {
        let program = &effective.as_bytes()[2..34];
        let Some((script, signatures)) = elements.split_last() else {
            return true;
        };
        if sha256::Hash::hash(script).as_byte_array() != program {
            return true;
        }
        let script = Script::from_bytes(script);
        return over_limits(script, signatures.len()) || short_multisig(script, signatures);
    }
    // A version 0 program of another length than 20 or 32 bytes is one
    // consensus refuses (BIP-141).
    if effective.is_witness_program() && effective.witness_version() == Some(WitnessVersion::V0) {
        return true;
    }
    if !effective.is_witness_program() {
        // No witness kind: a witness is BIP-141's unexpected witness.
        if !witness.is_empty() {
            return true;
        }
        if spk.is_p2pkh() {
            // `<sig> <key>`, the key the output's hash.
            return match data_pushes(&script_sig).as_deref() {
                Some([sig, key]) => {
                    hash160::Hash::hash(key).as_byte_array() != &spk.as_bytes()[3..23]
                        || !is_signature(sig)
                        || PublicKey::from_slice(key).is_err()
                }
                _ => true,
            };
        }
        let sig_pushes = data_pushes(&script_sig);
        if spk.is_p2pk() {
            // One signature.
            return !matches!(sig_pushes.as_deref(), Some([sig]) if is_signature(sig));
        }
        if spk.is_p2sh() {
            // `OP_0 <sig>… <redeem>`: the signatures of a multisig redeem
            // script are as many as it asks for.
            let all = pushes(&script_sig);
            let signatures: Vec<&[u8]> = all
                .split_last()
                .map(|(_, rest)| rest.iter().map(Vec::as_slice).collect())
                .unwrap_or_default();
            // Consensus refuses a redeem script push of over 520 bytes.
            return effective.len() > 520
                || over_limits(&effective, signatures.len())
                || short_multisig(&effective, &signatures);
        }
        // A bare multisig: `OP_0 <sig>…`.
        if Multisig::parse_wide(spk).is_some() {
            return match sig_pushes {
                Some(p) => {
                    let signatures: Vec<&[u8]> = p.iter().map(Vec::as_slice).collect();
                    short_multisig(spk, &signatures)
                }
                None => true,
            };
        }
        return false;
    }
    if !effective.is_p2tr() {
        return false;
    }
    taproot_mismatch(&effective, input)
}

/// Whether the script input `index`'s final data spends is over consensus's
/// limits ([`over_limits`]), a p2sh redeem script over 520 bytes, or a
/// scriptSig over 10,000 bytes: such an input is a mismatch before any
/// signature is read.
fn over_consensus_limits(c: &Classified, input: &bitcoin::psbt::Input) -> bool {
    let Some(utxo) = c.utxo.as_ref() else {
        return false;
    };
    let script_sig = input.final_script_sig.clone().unwrap_or_default();
    // Consensus caps a scriptSig at 10,000 bytes.
    if script_sig.len() > 10_000 {
        return true;
    }
    let Some(effective) = effective_script(&utxo.script_pubkey, &script_sig) else {
        return false;
    };
    if effective.is_p2wsh() {
        let witness = input.final_script_witness.clone().unwrap_or_default();
        let elements: Vec<&[u8]> = witness.iter().collect();
        return elements
            .split_last()
            .is_some_and(|(script, rest)| over_limits(Script::from_bytes(script), rest.len()));
    }
    if utxo.script_pubkey.is_p2sh() && !effective.is_witness_program() {
        let stack = pushes(&script_sig).len().saturating_sub(1);
        return effective.len() > 520 || over_limits(&effective, stack);
    }
    false
}

/// Whether a script, with `stack` elements under it, is over consensus's
/// limits: 10,000 bytes, 1,000 stack elements, 201 opcodes that are not
/// pushes. The count is of the script's own opcodes: consensus also counts
/// the keys of each `OP_CHECKMULTISIG` it executes, which no static count
/// can tell without refusing a true spend of a branch not taken.
fn over_limits(script: &Script, stack: usize) -> bool {
    let ops = script
        .instructions()
        .map_while(Result::ok)
        .filter(|i| matches!(i, Instruction::Op(op) if op.to_u8() > 0x60))
        .count();
    script.len() > 10_000 || stack > 1_000 || ops > 201
}

/// Whether `bytes` is an ECDSA signature with its sighash byte.
fn is_signature(bytes: &[u8]) -> bool {
    ecdsa::Signature::from_slice(bytes).is_ok()
}

/// Whether `script` is a standard multisig (up to 20 keys) whose stack `items` is not what it takes: an
/// empty dummy first (BIP-147), then exactly as many elements that parse as
/// a signature with its sighash byte as the script asks for. A junk
/// element, a filler or a count other than `m` makes it defective (no node
/// relays it); a key named twice may take the same signature twice. Any
/// other script is not.
fn short_multisig(script: &Script, items: &[&[u8]]) -> bool {
    let Some(m) = Multisig::parse_wide(script) else {
        return false;
    };
    let Some((dummy, signatures)) = items.split_first() else {
        return true;
    };
    !dummy.is_empty() || signatures.len() != m.m || !signatures.iter().all(|s| is_signature(s))
}

/// The pushes of a scriptSig that holds nothing else; `None` where it holds
/// an opcode that is not a push, or does not parse.
fn data_pushes(script: &Script) -> Option<Vec<Vec<u8>>> {
    script
        .instructions()
        .map(|i| match i {
            Ok(Instruction::PushBytes(b)) => Some(b.as_bytes().to_vec()),
            _ => None,
        })
        .collect()
}

/// Whether a finalized witness under a taproot `effective` script is one no
/// node relays.
fn taproot_mismatch(effective: &Script, input: &bitcoin::psbt::Input) -> bool {
    // A taproot spend has a witness (BIP-341).
    let Some(witness) = input
        .final_script_witness
        .as_ref()
        .filter(|w| !w.is_empty())
    else {
        return true;
    };
    let mut elements: Vec<&[u8]> = witness.iter().collect();
    if elements.len() >= 2 && elements.last().is_some_and(|e| e.first() == Some(&0x50)) {
        elements.pop();
    }
    let script_path = elements.len() >= 2;
    let Some(key) = output_key(effective) else {
        return true;
    };
    if !script_path {
        // A key-path spend is one signature of 64 or 65 bytes.
        return !(elements.len() == 1 && schnorr_signature(elements[0]).is_some());
    }
    let control = elements.pop().unwrap_or_default();
    let script = elements.pop().unwrap_or_default();
    let secp = Secp256k1::verification_only();
    !bitcoin::taproot::ControlBlock::decode(control)
        .is_ok_and(|cb| cb.verify_taproot_commitment(&secp, key, Script::from_bytes(script)))
}

/// The pushes of a scriptSig, in order, each with the value it puts on the
/// stack: `OP_1` to `OP_16` push 1 to 16 and `OP_1NEGATE` pushes 0x81. Any
/// other opcode gives an empty entry (callers that need a push-only script
/// check [`Script::is_push_only`]).
fn pushes(script: &Script) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for instruction in script.instructions() {
        match instruction {
            Ok(Instruction::PushBytes(b)) => out.push(b.as_bytes().to_vec()),
            Ok(Instruction::Op(op)) => out.push(match op.to_u8() {
                n @ 0x51..=0x60 => alloc::vec![n - 0x50],
                0x4f => alloc::vec![0x81],
                _ => Vec::new(),
            }),
            Err(_) => return Vec::new(),
        }
    }
    out
}

/// The keys a script names, compressed or not.
fn script_keys(script: &Script) -> Vec<SigKey> {
    pushes(script)
        .iter()
        .filter(|b| matches!(b.len(), 33 | 65))
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
    if spk.is_p2pk() || Multisig::parse_wide(spk).is_some() {
        // A bare output: `<sig>`, or `OP_0 <sig>…`, under the output's keys.
        return ecdsa_candidates(&sig_pushes, &script_keys(spk));
    }
    if let Some(redeem) = redeem {
        // `OP_0 <sig>… <redeem>`: bare multisig inside p2sh.
        let stack = &sig_pushes[..sig_pushes.len().saturating_sub(1)];
        let mut keys = script_keys(&redeem);
        keys.extend(ecdsa_pkh_keys(&redeem, stack));
        return ecdsa_candidates(stack, &keys);
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
    let stack = &elements[..elements.len() - 1];
    let mut keys = script_keys(Script::from_bytes(script));
    keys.extend(ecdsa_pkh_keys(Script::from_bytes(script), stack));
    ecdsa_candidates(stack, &keys)
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

/// The elements that are keys whose HASH160 a `DUP HASH160 <h20>
/// EQUALVERIFY` window of `script` holds: a `pkh` names the hash, the key
/// is in the witness.
fn ecdsa_pkh_keys(script: &Script, elements: &[Vec<u8>]) -> Vec<SigKey> {
    use bitcoin::hashes::{Hash, hash160};
    let Ok(ins) = script.instructions().collect::<Result<Vec<_>, _>>() else {
        return Vec::new();
    };
    let hashes: Vec<&[u8]> = ins
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
    if hashes.is_empty() {
        return Vec::new();
    }
    elements
        .iter()
        .filter(|e| matches!(e.len(), 33 | 65))
        .filter_map(|e| {
            let key = PublicKey::from_slice(e).ok()?;
            let h = hash160::Hash::hash(e);
            hashes
                .contains(&h.as_byte_array().as_slice())
                .then_some(SigKey::Ecdsa(key))
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
            input.redeem_script = None;
            restore_witness(input, &utxo.script_pubkey, &witness);
        }
        return inner;
    };
    // A final input's scripts are the ones its final data holds: a
    // `redeem_script` or `witness_script` field beside it is not read.
    let effective = if utxo.script_pubkey.is_p2sh() {
        input.redeem_script = pushes(&script_sig).pop().map(ScriptBuf::from);
        input.redeem_script.clone().unwrap_or_default()
    } else {
        input.redeem_script = None;
        utxo.script_pubkey.clone()
    };
    let witness = input.final_script_witness.clone().unwrap_or_default();
    restore_witness(input, &effective, &witness);
    inner
}

/// The witness script a finalized p2wsh witness carries as its last
/// element; any `witness_script` field is replaced by it, or cleared where
/// the input is no p2wsh.
fn restore_witness(input: &mut bitcoin::psbt::Input, effective: &Script, witness: &Witness) {
    input.witness_script = witness
        .last()
        .filter(|_| effective.is_p2wsh())
        .map(|s| ScriptBuf::from(s.to_vec()));
}

/// Checks one signature, and says which key it is under.
fn check<C: secp256k1::Verification>(
    secp: &Secp256k1<C>,
    restored: &bitcoin::Psbt,
    c: &Classified,
    index: usize,
    p: &Present,
    preimage: &mut bool,
    messages: &mut BTreeMap<u32, Option<Message>>,
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
            // One message per sighash type for the input.
            let msg = *messages
                .entry(sig.sighash_type.to_u32())
                .or_insert_with(|| ecdsa_message(restored, index, sig.sighash_type));
            let Some(msg) = msg else {
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
            // BIP-341 refuses a 65-byte signature ending 0x00, and a final
            // `SIGHASH_SINGLE` one on an input with no output of its index:
            // it is invalid under whichever key it is filed, named as any
            // other invalid signature is.
            let never = |preimage: &mut bool| {
                if p.leaf.is_none() && p.key == SigKey::Unknown {
                    let key =
                        output_key(&utxo.script_pubkey).map_or(SigKey::Unknown, SigKey::Schnorr);
                    return (key, Verdict::Invalid);
                }
                let (key, verdict) = settle(p, |_| false, preimage);
                if *preimage {
                    (key, verdict)
                } else {
                    (key, Verdict::Invalid)
                }
            };
            if v2::is_zero_sighash_schnorr(&p.bytes) {
                return never(preimage);
            }
            let Some(sig) = schnorr_signature(&p.bytes) else {
                return (
                    p.key.clone(),
                    Verdict::Unchecked(Unchecked::UnsupportedScript),
                );
            };
            let single = matches!(
                sig.sighash_type,
                TapSighashType::Single | TapSighashType::SinglePlusAnyoneCanPay
            );
            if p.finalized && single && index >= restored.unsigned_tx.output.len() {
                return never(preimage);
            }
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
    // A final witness with an annex (BIP-341: a last element of two or more
    // starting 0x50) signs over it.
    let annex = restored.inputs[index]
        .final_script_witness
        .as_ref()
        .filter(|w| w.len() >= 2)
        .and_then(|w| w.last())
        .and_then(|a| bitcoin::sighash::Annex::new(a).ok());
    let separator = leaf.map(|leaf| (leaf, 0xFFFF_FFFF));
    let sighash = cache
        .taproot_signature_hash(index, &prevouts.as_ref(), annex, separator, ty)
        .ok()?;
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
