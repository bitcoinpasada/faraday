//! What a signature already on a PSBT is checked for
//! (`docs/PLANNING.md` §16.111).
//!
//! Nothing here signs. [`verify_signatures`] reads every signature a
//! transaction carries — the partial-signature fields and the ones
//! inside a finalized `scriptSig` or witness — and checks each against
//! the key it is under and the sighash the input's own fields give.
//! [`repeated_nonces`] looks for one `r` (or one `R`) used twice under
//! one key, which is the key itself in two pieces.
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
use alloc::vec::Vec;
use core::fmt;

use bitcoin::key::TapTweak;
use bitcoin::script::Instruction;
use bitcoin::secp256k1::{self, Keypair, Message, Secp256k1, XOnlyPublicKey};
use bitcoin::sighash::{EcdsaSighashType, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TapLeafHash};
use bitcoin::{PublicKey, Script, ScriptBuf, Sequence, TxOut, Witness, ecdsa, taproot};
use osk_bip::keys::{Fingerprint, MasterKey};

use crate::classify::{Classified, classify};
use crate::context::Context;
use crate::inspect::SighashKind;
use crate::psbt::Psbt;
use crate::sign::{SigKind, tap_prevouts};

/// The key a signature is under, as the transaction names it.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// The input spends a script this crate does not take apart.
    UnsupportedScript,
    /// The script names no key the signature could be under.
    UnknownKey,
}

impl fmt::Display for Unchecked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Unchecked::NoPrevout => "no previous output",
            Unchecked::UnsupportedScript => "unsupported script",
            Unchecked::UnknownKey => "the script names no key",
        })
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

/// One `r` under one key, in more than one place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonceReuse {
    /// The key both signatures are under.
    pub key: SigKey,
    /// ECDSA or Schnorr.
    pub kind: SigKind,
    /// The repeated nonce point, in hex.
    pub nonce: String,
    /// The inputs the signatures sign, in order.
    pub inputs: Vec<usize>,
}

impl NonceReuse {
    /// How a card names the key: the first eight hex characters of it.
    pub fn name(&self) -> String {
        short(&self.key)
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
    /// The master the key was derived from.
    pub fingerprint: Fingerprint,
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
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Field::Version => "version",
            Field::Locktime => "locktime",
            Field::InputCount => "inputs",
            Field::OutputCount => "outputs",
            Field::Spends => "spends",
            Field::Sequence => "sequence",
            Field::Amount => "amount",
            Field::Script => "script",
            Field::PreviousTransaction => "previous transaction",
            Field::SpentOutput => "spent output",
            Field::RedeemScript => "redeem script",
            Field::WitnessScript => "witness script",
            Field::Sighash => "sighash",
            Field::KeyOrigins => "key origins",
            Field::TaprootKeyOrigins => "taproot key origins",
            Field::InternalKey => "internal key",
            Field::MerkleRoot => "merkle root",
            Field::LeafScripts => "leaf scripts",
            Field::Unknown => "unknown fields",
            Field::Proprietary => "proprietary fields",
            Field::Xpubs => "extended public keys",
        })
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
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// One signature as it was found, before it was checked.
struct Present {
    key: SigKey,
    kind: SigKind,
    bytes: Vec<u8>,
    leaf: Option<TapLeafHash>,
    finalized: bool,
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
    let mut out = Vec::with_capacity(present.len());
    for p in present {
        let (key, verdict) = check(secp, &restored, &c, index, &p);
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
        });
    }
    out
}

/// The sighash a signature itself states.
fn sighash_of(p: &Present) -> SighashKind {
    match p.kind {
        SigKind::Ecdsa => match ecdsa::Signature::from_slice(&p.bytes) {
            Ok(sig) => SighashKind::Ecdsa(sig.sighash_type),
            Err(_) => SighashKind::NonStandard(u32::from(*p.bytes.last().unwrap_or(&0))),
        },
        SigKind::Schnorr => match taproot::Signature::from_slice(&p.bytes) {
            Ok(sig) => SighashKind::Taproot(sig.sighash_type),
            Err(_) => SighashKind::NonStandard(u32::from(*p.bytes.last().unwrap_or(&0))),
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
    let fp = fp.map(Fingerprint::from);
    let ours = fp.is_some_and(|fp| ctx.is_ours(fp));
    (fp, ours)
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
    elements
        .into_iter()
        .filter(|e| taproot::Signature::from_slice(e).is_ok())
        .map(|bytes| Present {
            key: SigKey::Unknown,
            kind: SigKind::Schnorr,
            bytes,
            leaf: Some(leaf),
            finalized: true,
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
            settle(p, verify)
        }
        SigKind::Schnorr => {
            let Ok(sig) = taproot::Signature::from_slice(&p.bytes) else {
                return (
                    p.key.clone(),
                    Verdict::Unchecked(Unchecked::UnsupportedScript),
                );
            };
            let Some(msg) = taproot_message(restored, c, index, p.leaf, sig.sighash_type) else {
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
            settle(p, verify)
        }
    }
}

/// The verdict on a signature: the key it is filed under where it has
/// one, and otherwise whichever of the script's keys it verifies under.
fn settle(p: &Present, verify: impl Fn(&SigKey) -> bool) -> (SigKey, Verdict) {
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
    match p.candidates.first() {
        // The script names keys and none of them made this signature.
        Some(_) => (SigKey::Unknown, Verdict::Invalid),
        // The script names no key at all — a `pkh` fragment leaves a
        // hash — so there is nothing to check against.
        None => (SigKey::Unknown, Verdict::Unchecked(Unchecked::UnknownKey)),
    }
}

/// The ECDSA sighash for `ty`, computed the way [`crate::sign`] computes
/// it: through `bitcoin`'s own signer path, with the type the signature
/// states in place of the one the input asks for.
fn ecdsa_message(restored: &bitcoin::Psbt, index: usize, ty: EcdsaSighashType) -> Option<Message> {
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

/// Every repeated nonce in the transaction: one `r` under one key in two
/// places, which is the private key in two pieces.
///
/// Finalized signatures count: a transaction that carries one signature
/// in a witness and another in a partial-signature field has still
/// leaked the key.
pub fn repeated_nonces(psbt: &Psbt, ctx: &Context) -> Vec<NonceReuse> {
    /// One signature as the search for a repeated nonce holds it.
    struct Seen {
        key: SigKey,
        kind: SigKind,
        nonce: Vec<u8>,
        bytes: Vec<u8>,
        input: usize,
    }
    let mut seen: Vec<Seen> = Vec::new();
    for input in verify_signatures(psbt, ctx) {
        for sig in input.signatures {
            if sig.key == SigKey::Unknown {
                continue;
            }
            let nonce = sig.nonce_point();
            if nonce.is_empty() {
                continue;
            }
            // The same bytes in two places are one signature written
            // twice, not two signatures under one nonce.
            if seen
                .iter()
                .any(|s| s.key == sig.key && s.nonce == nonce && s.bytes == sig.bytes)
            {
                continue;
            }
            seen.push(Seen {
                key: sig.key,
                kind: sig.kind,
                nonce,
                bytes: sig.bytes,
                input: input.index,
            });
        }
    }
    let mut out: Vec<NonceReuse> = Vec::new();
    for found in seen {
        let nonce = hex(&found.nonce);
        match out
            .iter_mut()
            .find(|r| r.key == found.key && r.nonce == nonce)
        {
            Some(hit) => hit.inputs.push(found.input),
            None => out.push(NonceReuse {
                key: found.key,
                kind: found.kind,
                nonce,
                inputs: alloc::vec![found.input],
            }),
        }
    }
    out.retain(|r| r.inputs.len() > 1);
    out
}

/// Which nonce rule made each signature that is under a key one of
/// `keys` holds.
///
/// The key is derived from the PSBT's own origin data, the way
/// [`crate::sign`] derives it, and lives only inside this call.
pub fn deterministic(psbt: &Psbt, keys: &[&MasterKey]) -> Vec<Determinism> {
    let mut out = Vec::new();
    for index in 0..psbt.inner().inputs.len() {
        let c = classify(psbt.inner(), index);
        let input = &psbt.inner().inputs[index];
        let restored = restore(psbt, index, &c);
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
            let Some(msg) = ecdsa_message(&restored, index, sig.sighash_type) else {
                continue;
            };
            out.push(Determinism {
                input: index,
                key: SigKey::Ecdsa(*pk),
                fingerprint: master.fingerprint(),
                mode: ecdsa_mode(master, path, &msg, &pk.inner, &sig.signature),
            });
        }
        if let Some(sig) = input.tap_key_sig {
            let Some(internal) = input.tap_internal_key else {
                continue;
            };
            let Some((_, (fp, path))) = input.tap_key_origins.get(&internal) else {
                continue;
            };
            let Some(master) = keys
                .iter()
                .find(|k| k.fingerprint() == Fingerprint::from(*fp))
            else {
                continue;
            };
            let Some(msg) = taproot_message(&restored, &c, index, None, sig.sighash_type) else {
                continue;
            };
            out.push(Determinism {
                input: index,
                key: key_path_key(&c, input),
                fingerprint: master.fingerprint(),
                mode: schnorr_mode(master, path, &msg, Some(input.tap_merkle_root), &sig),
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
            let Some(msg) = taproot_message(&restored, &c, index, Some(*leaf), sig.sighash_type)
            else {
                continue;
            };
            out.push(Determinism {
                input: index,
                key: SigKey::Schnorr(*xonly),
                fingerprint: master.fingerprint(),
                mode: schnorr_mode(master, path, &msg, None, sig),
            });
        }
    }
    out
}

/// Which RFC 6979 rule reproduces an ECDSA signature, if either does.
fn ecdsa_mode(
    master: &MasterKey,
    path: &bitcoin::bip32::DerivationPath,
    msg: &Message,
    pk: &secp256k1::PublicKey,
    sig: &secp256k1::ecdsa::Signature,
) -> Option<NonceMode> {
    let secp = master.secp();
    let derived = master.derive(path);
    if derived.to_xpub().public_key != *pk {
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

/// Whether BIP-340 with zero auxiliary randomness reproduces a Schnorr
/// signature. `tweak` is the merkle root for a key-path signature —
/// `Some(None)` for a key-only wallet — and `None` for a script path,
/// which signs under the key itself.
fn schnorr_mode(
    master: &MasterKey,
    path: &bitcoin::bip32::DerivationPath,
    msg: &Message,
    tweak: Option<Option<bitcoin::taproot::TapNodeHash>>,
    sig: &taproot::Signature,
) -> Option<NonceMode> {
    let secp = master.secp();
    let derived = master.derive(path);
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
fn metadata_diff(x: &bitcoin::Psbt, y: &bitcoin::Psbt) -> Vec<FieldDiff> {
    let mut out = Vec::new();
    field(
        &mut out,
        Where::Transaction,
        Field::Xpubs,
        map(x.xpub.iter().map(|(k, v)| (k.to_string(), origin_text(v)))),
        map(y.xpub.iter().map(|(k, v)| (k.to_string(), origin_text(v)))),
    );
    field(
        &mut out,
        Where::Transaction,
        Field::Unknown,
        unknown_text(&x.unknown),
        unknown_text(&y.unknown),
    );
    for i in 0..x.inputs.len().min(y.inputs.len()) {
        let (a, b) = (&x.inputs[i], &y.inputs[i]);
        let at = Where::Input(i);
        field(
            &mut out,
            at,
            Field::PreviousTransaction,
            txt(&a.non_witness_utxo),
            txt(&b.non_witness_utxo),
        );
        field(
            &mut out,
            at,
            Field::SpentOutput,
            utxo_text(&a.witness_utxo),
            utxo_text(&b.witness_utxo),
        );
        field(
            &mut out,
            at,
            Field::RedeemScript,
            script_text(&a.redeem_script),
            script_text(&b.redeem_script),
        );
        field(
            &mut out,
            at,
            Field::WitnessScript,
            script_text(&a.witness_script),
            script_text(&b.witness_script),
        );
        field(
            &mut out,
            at,
            Field::Sighash,
            txt(&a.sighash_type),
            txt(&b.sighash_type),
        );
        field(
            &mut out,
            at,
            Field::KeyOrigins,
            map(a
                .bip32_derivation
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), origin_text(v)))),
            map(b
                .bip32_derivation
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), origin_text(v)))),
        );
        field(
            &mut out,
            at,
            Field::TaprootKeyOrigins,
            map(a
                .tap_key_origins
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), format!("{v:?}")))),
            map(b
                .tap_key_origins
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), format!("{v:?}")))),
        );
        field(
            &mut out,
            at,
            Field::InternalKey,
            txt(&a.tap_internal_key),
            txt(&b.tap_internal_key),
        );
        field(
            &mut out,
            at,
            Field::MerkleRoot,
            txt(&a.tap_merkle_root),
            txt(&b.tap_merkle_root),
        );
        field(
            &mut out,
            at,
            Field::LeafScripts,
            map(a
                .tap_scripts
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), format!("{:?}", v)))),
            map(b
                .tap_scripts
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), format!("{:?}", v)))),
        );
        field(
            &mut out,
            at,
            Field::Unknown,
            unknown_text(&a.unknown),
            unknown_text(&b.unknown),
        );
        field(
            &mut out,
            at,
            Field::Proprietary,
            map(a
                .proprietary
                .iter()
                .map(|(k, v)| (format!("{k:?}"), hex(v)))),
            map(b
                .proprietary
                .iter()
                .map(|(k, v)| (format!("{k:?}"), hex(v)))),
        );
    }
    for i in 0..x.outputs.len().min(y.outputs.len()) {
        let (a, b) = (&x.outputs[i], &y.outputs[i]);
        let at = Where::Output(i);
        field(
            &mut out,
            at,
            Field::RedeemScript,
            script_text(&a.redeem_script),
            script_text(&b.redeem_script),
        );
        field(
            &mut out,
            at,
            Field::WitnessScript,
            script_text(&a.witness_script),
            script_text(&b.witness_script),
        );
        field(
            &mut out,
            at,
            Field::KeyOrigins,
            map(a
                .bip32_derivation
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), origin_text(v)))),
            map(b
                .bip32_derivation
                .iter()
                .map(|(k, v)| (hex(&k.serialize()), origin_text(v)))),
        );
        field(
            &mut out,
            at,
            Field::InternalKey,
            txt(&a.tap_internal_key),
            txt(&b.tap_internal_key),
        );
        field(
            &mut out,
            at,
            Field::Unknown,
            unknown_text(&a.unknown),
            unknown_text(&b.unknown),
        );
    }
    out
}

/// A map as one sorted string, so that the order the pairs were written
/// in is not a difference.
fn map(pairs: impl Iterator<Item = (String, String)>) -> String {
    let mut sorted: Vec<String> = pairs.map(|(k, v)| format!("{k}={v}")).collect();
    sorted.sort();
    sorted.join(",")
}

fn unknown_text(unknown: &BTreeMap<bitcoin::psbt::raw::Key, Vec<u8>>) -> String {
    map(unknown.iter().map(|(k, v)| (format!("{k:?}"), hex(v))))
}

fn txt<T: fmt::Debug>(value: &Option<T>) -> String {
    match value {
        Some(v) => format!("{v:?}"),
        None => String::new(),
    }
}

fn script_text(script: &Option<ScriptBuf>) -> String {
    script
        .as_ref()
        .map(|s| hex(s.as_bytes()))
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
