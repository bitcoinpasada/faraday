//! The structured inspection behind the Sign screens:
//! summary, outputs with verified change, inputs, and ranked warnings.
//!
//! Inspection needs no private key. Ownership is verified by re-deriving
//! key-origin paths from the account xpubs in the [`Context`] and
//! comparing the resulting script with the one in the PSBT; a claim that
//! names one of our fingerprints but does not reproduce the script blocks
//! signing, never a silent skip.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use bitcoin::absolute::LockTime;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::hashes::Hash;
use bitcoin::psbt::{Input, Output};
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};
use bitcoin::sighash::{EcdsaSighashType, TapSighashType};
use bitcoin::taproot::TapTweakHash;
use bitcoin::{Address, Amount, ScriptBuf, Sequence, Transaction, TxOut, Txid, VarInt};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::{Compute, Work};

use faraday_bip::spend::{Lock, SpendPath};
use miniscript::{ExtParams, Miniscript, SigType, Tap, ToPublicKey};

use crate::Psbt;
use crate::classify::{Classified, Multisig, ScriptKind, classify};
use crate::context::Context;

mod sighash;

/// Fee at or above this share of the amount sent is a caution.
pub const HIGH_FEE_CAUTION_PCT: u64 = 5;
/// Fee at or above this share of the amount sent is a danger.
pub const HIGH_FEE_DANGER_PCT: u64 = 20;
/// Fee above this absolute amount (0.01 BTC) is a danger whatever the
/// share.
pub const HIGH_FEE_DANGER_ABS: Amount = Amount::from_sat(1_000_000);
/// Fee rate at or above this many sat/vB is a danger (Bitcoin Core's
/// `-maxfeerate`-style sanity limit is far above any real market rate).
pub const ABSURD_FEE_RATE_SAT_VB: u64 = 1000;

/// How serious a warning is (`docs/STANDARDS.md` §0, F136). A danger
/// refuses signing at the API, unless its kind is one a decision names
/// as holdable ([`WarningKind::is_holdable`]) and the caller passes that
/// kind by name, which the app does only after the person's hold. A
/// blocked warning is a transaction that lies about itself or can never
/// be valid, and nothing signs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Worth knowing; nothing wrong.
    Info,
    /// Unusual; read before signing.
    Caution,
    /// Likely theft or loss; signing is refused, but for a holdable kind
    /// the caller passes by name.
    Danger,
    /// The transaction lies about itself, hides what it is spending, or
    /// can never be valid; signing is refused, whatever is passed.
    Blocked,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Level::Info => "info",
            Level::Caution => "caution",
            Level::Danger => "danger",
            Level::Blocked => "blocked",
        })
    }
}

/// What a warning is about. The text on the [`Warning`] carries the
/// details; the kind lets the UI pick an explainer.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WarningKind {
    /// An output claims a key origin this inspection cannot verify: the
    /// fingerprint is not loaded, or the path is not under a loaded
    /// account.
    UnverifiedChange,
    /// An output claims to be change of a loaded key but the derived
    /// script does not match. Treat as an attack.
    ChangeSpoof,
    /// Fee is a large share of the amount sent, or above 0.01 BTC.
    HighFee {
        /// Fee as a whole percentage of the amount sent to others (or of
        /// all outputs for a self-transfer).
        pct_of_amount: u32,
    },
    /// An output is below the dust limit for its script.
    DustOutput,
    /// An input uses a sighash other than `ALL`/`DEFAULT`.
    UnusualSighash,
    /// Inputs spend more than one script type.
    MixedScriptTypes,
    /// A key path, global xpub or loaded account belongs to a different
    /// network than the context.
    NetworkMismatch,
    /// None of the loaded keys can sign any input.
    NoParticipatingKey,
    /// An input's key origin names a loaded key but cannot be verified
    /// from the loaded accounts.
    UnknownDerivation,
    /// Fee rate at or above [`ABSURD_FEE_RATE_SAT_VB`].
    AbsurdFeeRate,
    /// A locktime is set; the signer has no clock, so it is reported as
    /// information.
    LocktimeInFuture,
    /// An output pays a script with no address form.
    NonStandardScript,
    /// Two outputs share a script, or an output pays back to an input's
    /// script.
    AddressReuse,
    /// The UTXO or script data an input carries is inconsistent (BIP-174
    /// signer checks). Amounts or scripts may be lies.
    UtxoMismatch,
    /// An input has no UTXO data; amounts and fee cannot be verified.
    MissingUtxo,
    /// A SegWit v0 input carries no previous transaction, so its stated
    /// amount is the coordinator's word alone (`witness_utxo` is not
    /// committed to by the signature). Two rounds of that lie pay the
    /// difference to the miner.
    AmountUnverified,
    /// An input spends a script this crate cannot sign.
    UnsupportedInput,
    /// A MuSig2 input's participants are not those of any registered
    /// wallet.
    MusigWalletNotRegistered,
    /// An input this device would sign spends a script of several keys
    /// (`multi`, a miniscript, a taproot leaf of several) that no
    /// registered wallet gives at the chain and index it states.
    WalletNotRegistered,
    /// An input names a loaded key in a miniscript or a taproot tree
    /// (a single-key leaf, or the key path of a tree) that no registered
    /// wallet gives at the chain and index it states: it is left
    /// unsigned, and the rest of the transaction is signed (F141).
    NoWalletForScript {
        /// The input left unsigned.
        input: usize,
        /// Whether the script is a taproot tree, not a `wsh` miniscript.
        taproot: bool,
    },
    /// A MuSig2 input carries a public nonce of this device's that no
    /// open session holds the secret for, so this pass draws a new one
    /// and drops the partial signatures made against the old one.
    MusigNonceReplaced,
    /// A taproot key-path input carries proprietary records under the
    /// identifier `osk`, which this build does not read: it is not signed
    /// for what those records would have it be.
    UnreadRecords,
    /// A signature already on the transaction does not verify against
    /// the key it is under: the transaction was altered after it was
    /// signed, or the signer that made it is broken.
    InvalidSignature {
        /// The input it signs.
        input: usize,
        /// The key it is under, named as the review names one.
        key: String,
    },
    /// Two signatures under one key share a nonce, anywhere in the
    /// transaction. The key can be worked out from them.
    NonceReuse {
        /// The key both are under.
        key: String,
    },
    /// A signature under a key this device holds was not made by either
    /// RFC 6979 rule, or by BIP-340 with zero auxiliary randomness.
    Nondeterministic {
        /// The input it signs.
        input: usize,
        /// The key it is under.
        key: String,
    },
    /// A script carries an `ord` inscription envelope.
    Inscription {
        /// The input whose script carries it.
        input: usize,
        /// The content type the envelope states, empty where it states
        /// none.
        content_type: String,
    },
    /// A signature on the transaction states a sighash other than the
    /// one its input asks for, or other than `ALL`/`DEFAULT` where the
    /// input asks for none: the signer answered a different question,
    /// and what that sighash leaves out can be changed by anyone.
    SighashMismatch {
        /// The input it signs.
        input: usize,
        /// The key it is under, named as the review names one.
        key: String,
    },
    /// A signature is present that could not be checked.
    SignatureUnchecked {
        /// The input it signs.
        input: usize,
        /// Why nothing was checked.
        reason: crate::verify::Unchecked,
    },
    /// The spend path an input takes waits on a timelock the
    /// transaction's own sequence and locktime do not satisfy. The chain
    /// will refuse it until they do.
    TimelockNotMet {
        /// The path the input takes.
        path: SpendPath,
        /// The wait it does not meet.
        lock: Lock,
    },
}

impl WarningKind {
    /// Whether a danger of this kind has a way past it: a hold, for the
    /// legitimate case F136 names. A fee this high is paid on purpose by
    /// a child paying for its parent, a large batch or a consolidation.
    /// No other kind is holdable, and [`crate::sign`] refuses any other
    /// kind passed to it as accepted.
    pub fn is_holdable(&self) -> bool {
        matches!(
            self,
            WarningKind::HighFee { .. } | WarningKind::AbsurdFeeRate
        )
    }
}

/// A ranked warning with display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Severity.
    pub level: Level,
    /// Category.
    pub kind: WarningKind,
    /// One sentence for the screen.
    pub text: String,
}

/// The transaction's absolute locktime in human terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locktime {
    /// The consensus value.
    pub raw: u32,
    /// What the value means.
    pub kind: LocktimeKind,
}

/// The meaning of a locktime value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocktimeKind {
    /// Zero: no locktime.
    None,
    /// Below 500,000,000: a block height.
    Height(u32),
    /// 500,000,000 and above: a Unix timestamp.
    Time(u32),
}

impl fmt::Display for Locktime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            LocktimeKind::None => f.write_str("none"),
            LocktimeKind::Height(h) => write!(f, "block {h}"),
            LocktimeKind::Time(t) => write!(f, "unix time {t}"),
        }
    }
}

/// The sighash an input will be signed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SighashKind {
    /// Legacy or SegWit v0.
    Ecdsa(EcdsaSighashType),
    /// Taproot.
    Taproot(TapSighashType),
    /// A value neither algorithm accepts.
    NonStandard(u32),
}

impl SighashKind {
    /// Whether this is the default (`ALL`, or `DEFAULT` for taproot).
    pub fn is_default(self) -> bool {
        matches!(
            self,
            SighashKind::Ecdsa(EcdsaSighashType::All)
                | SighashKind::Taproot(TapSighashType::Default | TapSighashType::All)
        )
    }

    /// Whether outputs are not committed to at all (`NONE` variants).
    pub fn is_none(self) -> bool {
        matches!(
            self,
            SighashKind::Ecdsa(EcdsaSighashType::None | EcdsaSighashType::NonePlusAnyoneCanPay)
                | SighashKind::Taproot(TapSighashType::None | TapSighashType::NonePlusAnyoneCanPay)
        )
    }
}

impl fmt::Display for SighashKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SighashKind::Ecdsa(t) => write!(f, "{t}"),
            SighashKind::Taproot(t) => write!(f, "{t}"),
            SighashKind::NonStandard(v) => write!(f, "non-standard 0x{v:02x}"),
        }
    }
}

/// A key origin as stated in the PSBT: master fingerprint and path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyOrigin {
    /// Master fingerprint.
    pub fingerprint: Fingerprint,
    /// Derivation path from that master.
    pub path: DerivationPath,
}

/// Which of a wallet's ways to spend its coins an input takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpendRoute {
    /// A taproot input that claims only its internal key, so the
    /// transaction spends the key path and no leaf.
    KeyPath,
    /// One of the wallet's spend paths: the leaf the PSBT carries a
    /// script for, or, for a `wsh` script, the one path the keys the
    /// PSBT names and the transaction's own sequence and locktime leave.
    Script(SpendPath),
}

/// One input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputInfo {
    /// Position in the transaction.
    pub index: usize,
    /// The transaction being spent.
    pub txid: Txid,
    /// The output index being spent.
    pub vout: u32,
    /// Value of the spent output, when the PSBT carries it.
    pub amount: Option<Amount>,
    /// What script is being spent.
    pub script_type: ScriptKind,
    /// Whether a loaded key is verified (single-sig) or claimed (multisig)
    /// to control this input. A miniscript or a taproot tree is the
    /// person's only where a registered wallet holding a loaded key gives
    /// its script, and only then is it signed (F141).
    pub is_ours: bool,
    /// The key origin the PSBT states, preferring one of our keys.
    pub origin: Option<KeyOrigin>,
    /// The sighash type the input will be signed with.
    pub sighash: SighashKind,
    /// Fingerprints of keys whose signatures are already present (only
    /// those the PSBT states an origin for).
    pub already_signed_by: Vec<Fingerprint>,
    /// Number of signatures present, with or without a known origin.
    pub signatures: usize,
    /// Whether the input already has final scriptSig/witness.
    pub finalized: bool,
    /// The input's sequence number.
    pub sequence: Sequence,
    /// The way this input spends the wallet's script, when the wallet is
    /// in use and the PSBT says which way that is.
    pub spend_route: Option<SpendRoute>,
    /// What a MuSig2 input was established to be, when it is one.
    pub musig: Option<MusigInput>,
}

/// Who an output pays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputKind {
    /// Not ours (or not claimed to be).
    Recipient,
    /// Verified: derived from our account xpub, script matches.
    Change {
        /// Which loaded key.
        key: Fingerprint,
        /// Whether it is on the change chain (`…/1/i`) rather than the
        /// receive chain.
        change: bool,
        /// Address index.
        index: u32,
    },
    /// Verified: the script is a registered wallet's own at the index
    /// its key origins name.
    WalletChange {
        /// Position in [`Context::wallets`].
        wallet: usize,
        /// Whether it is on the change chain rather than the receive
        /// chain.
        change: bool,
        /// Address index.
        index: u32,
    },
    /// Claims a key origin that could not be verified. Treated as a
    /// payment to others in the totals.
    UnverifiedChange {
        /// The fingerprint the PSBT names.
        key: Fingerprint,
    },
}

/// One output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputInfo {
    /// Position in the transaction.
    pub index: usize,
    /// The address for the context's network, or `non-standard script`
    /// followed by the script hex.
    pub address: String,
    /// The script itself.
    pub script_pubkey: ScriptBuf,
    /// Value.
    pub amount: Amount,
    /// Recipient, verified change, or unverified claim.
    pub kind: OutputKind,
    /// Below the dust limit for its script.
    pub is_dust: bool,
}

impl OutputInfo {
    /// Whether the output is verified as ours.
    pub fn is_ours(&self) -> bool {
        matches!(
            self.kind,
            OutputKind::Change { .. } | OutputKind::WalletChange { .. }
        )
    }
}

/// One key of a multisig script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cosigner {
    /// Master fingerprint, when the PSBT states an origin for the key.
    pub fingerprint: Option<Fingerprint>,
    /// Whether this key's signature is present on the first multisig
    /// input.
    pub signed: bool,
    /// Whether a loaded key, derived at the path the input states for
    /// this key, is this key.
    pub ours: bool,
}

/// The quorum and cosigners of the multisig inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultisigSummary {
    /// Signatures required.
    pub m: usize,
    /// Keys in the script.
    pub n: usize,
    /// Keys in script order.
    pub cosigners: Vec<Cosigner>,
    /// Whether a loaded key is one of the cosigners.
    pub ours: bool,
    /// The registered wallet the inputs spend, when they spend one.
    pub wallet: Option<usize>,
}

impl MultisigSummary {
    /// How many cosigners have signed.
    pub fn signed(&self) -> usize {
        self.cosigners.iter().filter(|c| c.signed).count()
    }
}

/// The MuSig2 wallet the inputs spend, when they spend one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusigSummary {
    /// Position in [`Context::wallets`].
    pub wallet: usize,
    /// Keys that aggregate into it.
    pub keys: usize,
    /// Partial signatures present on the first MuSig2 input.
    pub partial_signatures: usize,
    /// Loaded keys that are participants.
    pub ours: Vec<Fingerprint>,
}

/// Everything the Sign screens show.
#[derive(Debug, Clone, PartialEq)]
pub struct Inspection {
    /// Transaction version.
    pub version: i32,
    /// Absolute locktime.
    pub locktime: Locktime,
    /// Whether any input signals replace-by-fee (BIP-125).
    pub rbf: bool,
    /// Inputs in order.
    pub inputs: Vec<InputInfo>,
    /// Outputs in order.
    pub outputs: Vec<OutputInfo>,
    /// Inputs minus outputs; zero when an input's value is unknown (a
    /// `MissingUtxo` danger is raised).
    pub fee: Amount,
    /// Fee divided by the estimated virtual size, when the fee is known.
    pub fee_rate_sat_vb: Option<f32>,
    /// Estimated virtual size after signing, from the input script
    /// types (`ScriptKind::estimated_weight`); exact for finalized inputs.
    pub vsize_estimate: usize,
    /// Sum of known input values.
    pub total_in: Amount,
    /// Sum of outputs.
    pub total_out: Amount,
    /// Sum of outputs that are not verified change.
    pub amount_to_others: Amount,
    /// Sum of verified change outputs.
    pub change_total: Amount,
    /// Loaded keys that can sign at least one input.
    pub participating_keys: Vec<Fingerprint>,
    /// Quorum and cosigners, when an input is multisig.
    pub multisig: Option<MultisigSummary>,
    /// The MuSig2 wallet, when an input spends one.
    pub musig: Option<MusigSummary>,
    /// Ranked warnings, blocks first, then dangers.
    pub warnings: Vec<Warning>,
    /// Every signature the transaction already carries, checked against
    /// the key it is under.
    pub signatures: Vec<crate::verify::InputSignatures>,
    /// Keys whose signatures share a nonce (rule 2).
    pub nonce_reuse: Vec<crate::verify::NonceReuse>,
    /// Inscription envelopes in the transaction's scripts (rule 5).
    pub inscriptions: Vec<crate::verify::Inscription>,
    /// Every output is verified change.
    pub is_self_transfer: bool,
    /// The wallet the PSBT defines, offered for registering, where the
    /// only blocks are [`WarningKind::WalletNotRegistered`] and the PSBT
    /// carries every cosigner's extended key ([`crate::register`]).
    pub register: Option<crate::register::Registration>,
}

impl Inspection {
    /// Every signature the transaction carries, in input order.
    pub fn present_signatures(&self) -> impl Iterator<Item = &crate::verify::SignaturePresent> {
        self.signatures.iter().flat_map(|i| i.signatures.iter())
    }
}

impl Inspection {
    /// Whether any warning is a danger.
    pub fn has_danger(&self) -> bool {
        self.warnings.iter().any(|w| w.level == Level::Danger)
    }

    /// The kinds of every danger warning.
    pub fn dangers(&self) -> Vec<WarningKind> {
        self.warnings
            .iter()
            .filter(|w| w.level == Level::Danger)
            .map(|w| w.kind.clone())
            .collect()
    }

    /// Whether any warning blocks signing outright.
    pub fn has_blocked(&self) -> bool {
        self.warnings.iter().any(|w| w.level == Level::Blocked)
    }

    /// The kinds of every blocked warning.
    pub fn blocked(&self) -> Vec<WarningKind> {
        self.warnings
            .iter()
            .filter(|w| w.level == Level::Blocked)
            .map(|w| w.kind.clone())
            .collect()
    }

    /// Whether nothing signs this transaction: a blocked warning, or a
    /// danger with no hold past it (F136).
    pub fn refuses(&self) -> bool {
        self.warnings.iter().any(|w| {
            w.level == Level::Blocked || (w.level == Level::Danger && !w.kind.is_holdable())
        })
    }

    /// The kinds of the dangers a hold accepts, in the order they are
    /// ranked: what [`crate::sign`] is passed once the person has held.
    pub fn holdable(&self) -> Vec<WarningKind> {
        self.warnings
            .iter()
            .filter(|w| w.level == Level::Danger && w.kind.is_holdable())
            .map(|w| w.kind.clone())
            .collect()
    }

    /// The fee as a whole percentage of what the transaction sends to
    /// others, or of every output for a transfer to itself: the share a
    /// [`WarningKind::HighFee`] is about. `None` when the fee is not known
    /// or nothing is sent.
    pub fn fee_share_pct(&self) -> Option<u64> {
        self.fee_rate_sat_vb?;
        let base = if self.amount_to_others > Amount::ZERO {
            self.amount_to_others
        } else {
            self.total_out
        };
        (base > Amount::ZERO).then(|| self.fee.to_sat().saturating_mul(100) / base.to_sat())
    }

    /// The kinds of the dangers `accepted` does not accept.
    pub fn dangers_left(&self, accepted: &[WarningKind]) -> Vec<WarningKind> {
        self.warnings
            .iter()
            .filter(|w| {
                w.level == Level::Danger && !(w.kind.is_holdable() && accepted.contains(&w.kind))
            })
            .map(|w| w.kind.clone())
            .collect()
    }
}

fn warn(warnings: &mut Vec<Warning>, level: Level, kind: WarningKind, text: String) {
    warnings.push(Warning { level, kind, text });
}

/// Costs a script derivation for each wallet an input or output claims,
/// and the checks: inspects `psbt` for the keys and network in `ctx`.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub fn inspect(c: &Compute, psbt: &Psbt, ctx: &Context) -> Inspection {
    c.count(Work::PsbtInspect);
    let secp = Secp256k1::verification_only();
    let inner = psbt.inner();
    let tx = &inner.unsigned_tx;
    // The id the open MuSig2 session, if any, is keyed by (§16.100).
    let txid = tx.compute_txid();
    let mut warnings = Vec::new();

    let mut inputs = Vec::with_capacity(inner.inputs.len());
    let mut participating: Vec<Fingerprint> = Vec::new();
    let mut multisig: Option<MultisigSummary> = None;
    let mut musig: Option<MusigSummary> = None;
    let mut kinds: Vec<ScriptKind> = Vec::new();
    let mut input_scripts: Vec<ScriptBuf> = Vec::new();
    // The inputs held for want of a registered wallet, and which keys of
    // each multisig input a loaded key derives, for the register offer.
    let mut unregistered: Vec<usize> = Vec::new();
    let mut multisig_ours: alloc::collections::BTreeMap<usize, Vec<bool>> =
        alloc::collections::BTreeMap::new();
    let mut total_in = Amount::ZERO;
    let mut fee_known = true;
    let mut has_witness = false;
    let mut weight = (4 + 4) * 4
        + VarInt::from(tx.input.len()).size() * 4
        + VarInt::from(tx.output.len()).size() * 4;

    for (i, input) in inner.inputs.iter().enumerate() {
        let cl = classify(inner, i);
        for p in &cl.problems {
            warn(
                &mut warnings,
                Level::Blocked,
                WarningKind::UtxoMismatch,
                format!("input {i}: {p}"),
            );
        }
        if cl.amount_unverified {
            warn(
                &mut warnings,
                Level::Blocked,
                WarningKind::AmountUnverified,
                format!("input {i} carries no previous transaction; its amount cannot be checked"),
            );
        }
        let amount = match &cl.utxo {
            Some(u) => {
                total_in = total_in.checked_add(u.value).unwrap_or(Amount::MAX_MONEY);
                input_scripts.push(u.script_pubkey.clone());
                Some(u.value)
            }
            None => {
                fee_known = false;
                warn(
                    &mut warnings,
                    Level::Danger,
                    WarningKind::MissingUtxo,
                    format!("input {i} has no UTXO data; its value and the fee cannot be verified"),
                );
                None
            }
        };
        let spk = cl.utxo.as_ref().map(|u| u.script_pubkey.clone());
        let txin = &tx.input[i];
        let finalized = input.final_script_sig.is_some() || input.final_script_witness.is_some();

        if finalized {
            let ss = input.final_script_sig.as_ref().map_or(0, |s| s.len());
            let wit = input.final_script_witness.as_ref().map_or(0, |w| w.size());
            weight += (32 + 4 + 4 + VarInt::from(ss).size() + ss) * 4 + wit;
            has_witness |= wit > 0;
        } else {
            weight += cl
                .satisfaction_weight
                .unwrap_or_else(|| cl.kind.estimated_weight());
            has_witness |= !matches!(cl.kind, ScriptKind::P2pkh);
        }
        let signable = cl.kind.is_supported()
            && (cl.kind != ScriptKind::P2trScript || tap_signable(&secp, input, spk.as_deref()));
        if signable {
            if !kinds.contains(&cl.kind) {
                kinds.push(cl.kind);
            }
        } else {
            let what = match cl.kind {
                ScriptKind::P2trScript => {
                    "the taproot input names no leaf script and no internal key of this tree"
                }
                _ => "unknown script type",
            };
            if cl.utxo.is_some() && !finalized {
                warn(
                    &mut warnings,
                    Level::Caution,
                    WarningKind::UnsupportedInput,
                    format!("input {i} cannot be signed here: {what}"),
                );
            }
        }

        let mut is_ours = false;
        let mut origin: Option<KeyOrigin> = None;
        let mut musig_established: Option<MusigInput> = None;
        // The registered wallet whose script this input is, for the kinds
        // a wallet of several keys can be.
        let registered = match cl.kind {
            ScriptKind::Multisig { .. }
            | ScriptKind::Miniscript { .. }
            | ScriptKind::P2trScript
            | ScriptKind::Unknown => wallet_input(c, ctx, input, spk.as_deref()),
            _ => None,
        };
        match cl.kind {
            ScriptKind::P2pkh | ScriptKind::P2shP2wpkh | ScriptKind::P2wpkh => {
                for (pk, (fp, path)) in &input.bip32_derivation {
                    let fp = Fingerprint::from(*fp);
                    let claim = KeyOrigin {
                        fingerprint: fp,
                        path: path.clone(),
                    };
                    let Some(key) = ctx.key(fp) else {
                        origin.get_or_insert(claim);
                        continue;
                    };
                    origin = Some(claim);
                    match key.derive(&secp, path) {
                        Some(d) if d.key == *pk && Some(&d.script_pubkey) == spk.as_ref() => {
                            is_ours = true;
                            push_unique(&mut participating, fp);
                        }
                        Some(d) => mismatch(&mut warnings, i, fp, path, d.key == *pk),
                        None => {
                            push_unique(&mut participating, fp);
                            warn(
                                &mut warnings,
                                Level::Caution,
                                WarningKind::UnknownDerivation,
                                format!(
                                    "input {i} names your key {fp} at {path}, which is not under a loaded account; ownership is checked only when signing"
                                ),
                            );
                        }
                    }
                }
            }
            ScriptKind::Multisig { m, n, .. } => {
                let ms = cl
                    .multisig
                    .as_ref()
                    .expect("multisig kind carries the script");
                let cosigners = cosigners(c, &secp, ms, input, ctx);
                let ours = cosigners.iter().any(|c| c.ours);
                multisig_ours.insert(i, cosigners.iter().map(|c| c.ours).collect());
                for (key, cosigner) in ms.keys.iter().zip(&cosigners) {
                    if let Some(fp) = cosigner.fingerprint {
                        let path = input.bip32_derivation[&key.inner].1.clone();
                        let claim = KeyOrigin {
                            fingerprint: fp,
                            path,
                        };
                        if cosigner.ours {
                            origin = Some(claim);
                            push_unique(&mut participating, fp);
                            continue;
                        }
                        let path = &claim.path;
                        if ctx.is_ours(fp) {
                            match derives_to(c, &secp, ctx, fp, path, &key.inner) {
                                Some(_) => mismatch(&mut warnings, i, fp, path, false),
                                None => {
                                    push_unique(&mut participating, fp);
                                    warn(
                                        &mut warnings,
                                        Level::Caution,
                                        WarningKind::UnknownDerivation,
                                        format!(
                                            "input {i} names your key {fp} at {path}, which is not under a loaded account; ownership is checked only when signing"
                                        ),
                                    );
                                }
                            }
                        }
                        origin.get_or_insert(claim);
                    }
                }
                is_ours = ours;
                match &multisig {
                    None => {
                        multisig = Some(MultisigSummary {
                            m: usize::from(m),
                            n: usize::from(n),
                            cosigners,
                            ours,
                            wallet: registered,
                        })
                    }
                    Some(first) if first.m != usize::from(m) || first.n != usize::from(n) => warn(
                        &mut warnings,
                        Level::Info,
                        WarningKind::MixedScriptTypes,
                        format!(
                            "input {i} is {m}-of-{n} multisig; earlier inputs are {}-of-{}",
                            first.m, first.n
                        ),
                    ),
                    Some(_) => {}
                }
            }
            ScriptKind::P2trKey if has_musig_field(input) => {
                match musig_input(c, &secp, ctx, input, spk.as_deref()) {
                    Err(block) => warn(
                        &mut warnings,
                        Level::Blocked,
                        block.kind,
                        format!("input {i}: {}", block.reason),
                    ),
                    Ok(None) => {}
                    Ok(Some(mut established)) => {
                        is_ours = !established.ours.is_empty();
                        if let Some(first) = established.ours.first() {
                            origin = Some(KeyOrigin {
                                fingerprint: first.fingerprint,
                                path: first.path.clone(),
                            });
                            for participation in &established.ours {
                                push_unique(&mut participating, participation.fingerprint);
                            }
                        }
                        // What this pass will do with each participant
                        // this device holds, which is what the hold's
                        // label and the caution below are about.
                        let rounds: Vec<MusigRound> = established
                            .ours
                            .iter()
                            .map(|p| {
                                musig_round(input, &established, p, ctx.musig_session, txid, i)
                            })
                            .collect();
                        established.will_share_nonce = !rounds.is_empty()
                            && rounds.iter().all(|r| matches!(r, MusigRound::Share { .. }));
                        established.nonce_stale = rounds
                            .iter()
                            .any(|r| matches!(r, MusigRound::Share { stale: true }));
                        if established.nonce_stale {
                            warn(
                                &mut warnings,
                                Level::Caution,
                                WarningKind::MusigNonceReplaced,
                                format!(
                                    "input {i} carries a nonce of this device's that no open session holds; it is replaced and the partial signatures made against it are dropped"
                                ),
                            );
                        }
                        if musig.is_none() {
                            musig = Some(MusigSummary {
                                wallet: established.wallet,
                                keys: established.participants.len(),
                                partial_signatures: crate::musig::partial_sigs(input)
                                    .map_or(0, |sigs| sigs.len()),
                                ours: established.ours.iter().map(|p| p.fingerprint).collect(),
                            });
                        }
                        musig_established = Some(established);
                    }
                }
            }
            // Faraday's own `osk` records are read by no code in this
            // build, so an input carrying them is blocked, not signed as a
            // plain key-path input.
            ScriptKind::P2trKey if has_osk_records(input) => warn(
                &mut warnings,
                Level::Blocked,
                WarningKind::UnreadRecords,
                format!("input {i}: it carries `osk` records this version does not read"),
            ),
            ScriptKind::P2trKey => {
                for (xonly, (_, (fp, path))) in &input.tap_key_origins {
                    let fp = Fingerprint::from(*fp);
                    let claim = KeyOrigin {
                        fingerprint: fp,
                        path: path.clone(),
                    };
                    let Some(key) = ctx.key(fp) else {
                        origin.get_or_insert(claim);
                        continue;
                    };
                    origin = Some(claim);
                    match key.derive(&secp, path) {
                        Some(d)
                            if d.key.x_only_public_key().0 == *xonly
                                && Some(&d.script_pubkey) == spk.as_ref()
                                && input.tap_internal_key == Some(*xonly) =>
                        {
                            is_ours = true;
                            push_unique(&mut participating, fp);
                        }
                        Some(d) => mismatch(
                            &mut warnings,
                            i,
                            fp,
                            path,
                            d.key.x_only_public_key().0 == *xonly,
                        ),
                        None => {
                            push_unique(&mut participating, fp);
                            warn(
                                &mut warnings,
                                Level::Caution,
                                WarningKind::UnknownDerivation,
                                format!(
                                    "input {i} names your key {fp} at {path}, which is not under a loaded account; ownership is checked only when signing"
                                ),
                            );
                        }
                    }
                }
            }
            ScriptKind::Miniscript { .. } | ScriptKind::P2trScript | ScriptKind::Unknown => {
                origin = input
                    .bip32_derivation
                    .values()
                    .map(|(fp, path)| ((*fp).into(), path))
                    .chain(
                        input
                            .tap_key_origins
                            .values()
                            .map(|(_, (fp, path))| ((*fp).into(), path)),
                    )
                    .max_by_key(|(fp, _)| ctx.is_ours(*fp))
                    .map(|(fingerprint, path)| KeyOrigin {
                        fingerprint,
                        path: path.clone(),
                    });
                // A miniscript or taproot-tree wallet in use answers for
                // its own inputs: the script in front of us is that
                // wallet's own at the index its key origins name. This
                // is the rule a multisig input is read by, and it is the
                // only thing that says whose these coins are, since the
                // script itself is not one this crate takes apart.
                if registered.is_some() {
                    is_ours = true;
                    for (fp, _) in input
                        .bip32_derivation
                        .values()
                        .map(|(fp, path)| (Fingerprint::from(*fp), path))
                        .chain(
                            input
                                .tap_key_origins
                                .values()
                                .map(|(_, (fp, path))| (Fingerprint::from(*fp), path)),
                        )
                    {
                        if ctx.is_ours(fp) {
                            push_unique(&mut participating, fp);
                        }
                    }
                }
            }
        }

        // A script of several keys is signed only for a registered
        // wallet whose script it is: the coordinator's word for who the
        // cosigners are is not taken. Only an input that names a loaded
        // key is held to it, since no other input is signed here. A
        // miniscript or a taproot tree of one key is not held, and not
        // signed either: it is left unsigned, said so, while the rest
        // of the transaction is signed (F141).
        if !finalized && registered.is_none() {
            let several = crate::register::several_keys(c, &secp, input, cl.kind, spk.as_deref());
            let names_ours = input_origins(input).any(|(fp, _)| ctx.is_ours(fp));
            if several && names_ours {
                warn(
                    &mut warnings,
                    Level::Blocked,
                    WarningKind::WalletNotRegistered,
                    format!(
                        "input {i} spends from a wallet of several keys that is not registered here; register the wallet to sign"
                    ),
                );
                unregistered.push(i);
            } else if names_ours && signed_through_wallet(cl.kind) {
                let taproot = cl.kind == ScriptKind::P2trScript;
                let what = if taproot { "taproot tree" } else { "script" };
                warn(
                    &mut warnings,
                    Level::Caution,
                    WarningKind::NoWalletForScript { input: i, taproot },
                    format!(
                        "input {i} is left unsigned: no registered wallet covers this {what}; register the wallet to sign it"
                    ),
                );
            }
        }

        let sighash = sighash_kind(input, cl.kind, spk.as_deref());
        sighash::sighash_card(&mut warnings, i, sighash, inner.unsigned_tx.output.len());

        let mut already_signed_by = Vec::new();
        for pk in input.partial_sigs.keys() {
            if let Some((fp, _)) = input.bip32_derivation.get(&pk.inner) {
                push_unique(&mut already_signed_by, (*fp).into());
            }
        }
        if input.tap_key_sig.is_some()
            && let Some(internal) = input.tap_internal_key
            && let Some((_, (fp, _))) = input.tap_key_origins.get(&internal)
        {
            push_unique(&mut already_signed_by, (*fp).into());
        }
        let signatures = input.partial_sigs.len()
            + usize::from(input.tap_key_sig.is_some())
            + input.tap_script_sigs.len();

        let spend_route = spend_route(c, &secp, ctx, input, &cl, tx, i);
        // Refused only when no way the person's keys take part in is
        // open: the path read from the PSBT may be the coordinator's
        // guess, its other keys' origins left off (F136).
        if let Some(SpendRoute::Script(path)) = &spend_route
            && path.locks.iter().any(|l| !lock_met(*l, tx, i))
            && !own_path_open(c, ctx, input, &cl, tx, i)
        {
            for lock in &path.locks {
                if !lock_met(*lock, tx, i) {
                    warn(
                        &mut warnings,
                        Level::Blocked,
                        WarningKind::TimelockNotMet {
                            path: path.clone(),
                            lock: *lock,
                        },
                        format!(
                            "input {i} spends a path that waits {}, which this transaction's sequence and locktime do not meet; it can never be valid",
                            wait(*lock)
                        ),
                    );
                }
            }
        }

        inputs.push(InputInfo {
            index: i,
            txid: txin.previous_output.txid,
            vout: txin.previous_output.vout,
            amount,
            script_type: cl.kind,
            is_ours,
            origin,
            sighash,
            already_signed_by,
            signatures,
            finalized,
            sequence: txin.sequence,
            spend_route,
            musig: musig_established,
        });
    }
    if has_witness {
        weight += 2;
    }

    // Outputs.
    let network = bitcoin::Network::from(ctx.network);
    let mut outputs = Vec::with_capacity(tx.output.len());
    let mut total_out = Amount::ZERO;
    let mut amount_to_others = Amount::ZERO;
    let mut change_total = Amount::ZERO;
    for (i, txout) in tx.output.iter().enumerate() {
        let out = &inner.outputs[i];
        let spk = &txout.script_pubkey;
        let script_len = spk.len();
        weight += (8 + VarInt::from(script_len).size() + script_len) * 4;
        total_out = total_out
            .checked_add(txout.value)
            .unwrap_or(Amount::MAX_MONEY);

        let address = match Address::from_script(spk, network) {
            Ok(a) => a.to_string(),
            Err(_) => {
                let (level, what) = if spk.is_op_return() {
                    (Level::Info, "is a data (OP_RETURN) output")
                } else {
                    (Level::Caution, "pays a non-standard script")
                };
                warn(
                    &mut warnings,
                    level,
                    WarningKind::NonStandardScript,
                    format!("output {i} {what}"),
                );
                format!("non-standard script {}", hex(spk.as_bytes()))
            }
        };

        let kind = output_kind(c, &secp, out, txout, ctx, i, &mut warnings);
        match kind {
            OutputKind::Change { .. } | OutputKind::WalletChange { .. } => {
                change_total = change_total
                    .checked_add(txout.value)
                    .unwrap_or(Amount::MAX_MONEY);
            }
            _ => {
                amount_to_others = amount_to_others
                    .checked_add(txout.value)
                    .unwrap_or(Amount::MAX_MONEY);
            }
        }

        let is_dust = txout.value < spk.minimal_non_dust();
        if is_dust {
            warn(
                &mut warnings,
                Level::Caution,
                WarningKind::DustOutput,
                format!(
                    "output {i} ({} sat) is below the dust limit",
                    txout.value.to_sat()
                ),
            );
        }

        outputs.push(OutputInfo {
            index: i,
            address,
            script_pubkey: spk.clone(),
            amount: txout.value,
            kind,
            is_dust,
        });
    }

    // Address reuse.
    for (j, b) in outputs.iter().enumerate() {
        if let Some(a) = outputs[..j]
            .iter()
            .find(|a| a.script_pubkey == b.script_pubkey)
        {
            warn(
                &mut warnings,
                Level::Caution,
                WarningKind::AddressReuse,
                format!("outputs {} and {j} pay the same address", a.index),
            );
        } else if let Some(k) = input_scripts.iter().position(|s| *s == b.script_pubkey) {
            warn(
                &mut warnings,
                Level::Caution,
                WarningKind::AddressReuse,
                format!("output {j} pays back to the address of input {k}"),
            );
        }
    }

    // Fee.
    let vsize_estimate = weight.div_ceil(4);
    let mut fee = Amount::ZERO;
    let mut fee_rate_sat_vb = None;
    if fee_known {
        match total_in.checked_sub(total_out) {
            Some(f) => fee = f,
            None => {
                fee_known = false;
                warn(
                    &mut warnings,
                    Level::Blocked,
                    WarningKind::UtxoMismatch,
                    "outputs exceed inputs; the UTXO data is wrong".to_string(),
                );
            }
        }
    }
    if fee_known {
        let rate = fee.to_sat() as f32 / vsize_estimate as f32;
        fee_rate_sat_vb = Some(rate);
        let base = if amount_to_others > Amount::ZERO {
            amount_to_others
        } else {
            total_out
        };
        if base > Amount::ZERO {
            let pct = fee.to_sat().saturating_mul(100) / base.to_sat();
            let kind = WarningKind::HighFee {
                pct_of_amount: pct.min(u64::from(u32::MAX)) as u32,
            };
            if fee > HIGH_FEE_DANGER_ABS || pct >= HIGH_FEE_DANGER_PCT {
                warn(
                    &mut warnings,
                    Level::Danger,
                    kind,
                    format!("fee {} is {pct}% of the amount sent", fee.to_sat()),
                );
            } else if pct >= HIGH_FEE_CAUTION_PCT {
                warn(
                    &mut warnings,
                    Level::Caution,
                    kind,
                    format!("fee {} is {pct}% of the amount sent", fee.to_sat()),
                );
            }
        }
        if fee.to_sat() >= ABSURD_FEE_RATE_SAT_VB.saturating_mul(vsize_estimate as u64) {
            warn(
                &mut warnings,
                Level::Danger,
                WarningKind::AbsurdFeeRate,
                format!("fee rate is about {} sat/vB", rate as u64),
            );
        }
    }

    // Network.
    network_checks(inner, ctx, &mut warnings);

    // Script mix.
    if kinds.len() > 1 {
        let list: Vec<String> = kinds.iter().map(|k| k.to_string()).collect();
        warn(
            &mut warnings,
            Level::Info,
            WarningKind::MixedScriptTypes,
            format!("inputs mix script types: {}", list.join(", ")),
        );
    }

    // Locktime and RBF.
    let locktime = locktime(tx.lock_time);
    if locktime.kind != LocktimeKind::None {
        warn(
            &mut warnings,
            Level::Info,
            WarningKind::LocktimeInFuture,
            format!("cannot be mined before {locktime}; this signer has no clock to check it"),
        );
    }
    let rbf = tx.input.iter().any(|i| i.sequence.is_rbf());

    if participating.is_empty() {
        warn(
            &mut warnings,
            Level::Danger,
            WarningKind::NoParticipatingKey,
            "none of your keys can sign this transaction".to_string(),
        );
    }

    let is_self_transfer = !outputs.is_empty() && outputs.iter().all(OutputInfo::is_ours);

    // §16.111 rules 1, 2 and 5: what is already signed here is checked
    // here, so a signature that is not this transaction's, a nonce used
    // twice and an inscription envelope all reach the review as cards.
    let signatures = crate::verify::verify_signatures(psbt, ctx);
    sighash::signature_cards(&mut warnings, &signatures, &inputs);
    let nonce_reuse = crate::verify::repeated_nonces(psbt, ctx);
    for reuse in &nonce_reuse {
        warn(
            &mut warnings,
            Level::Danger,
            WarningKind::NonceReuse { key: reuse.name() },
            format!(
                "two signatures under {} share one nonce; the private key follows from them",
                reuse.name()
            ),
        );
    }
    let inscriptions = crate::verify::inscriptions(psbt);
    for inscription in &inscriptions {
        warn(
            &mut warnings,
            Level::Info,
            WarningKind::Inscription {
                input: inscription.input,
                content_type: inscription.content_type.clone().unwrap_or_default(),
            },
            format!(
                "input {} carries an inscription envelope",
                inscription.input
            ),
        );
    }

    // The wallet the PSBT defines, offered where nothing else blocks. A
    // key under a loaded fingerprint that is not its key blocks (F136)
    // and is still offered: registering refuses that key by name.
    let register = warnings
        .iter()
        .filter(|w| w.level == Level::Blocked)
        .all(|w| {
            matches!(
                w.kind,
                WarningKind::WalletNotRegistered | WarningKind::UnknownDerivation
            )
        })
        .then(|| crate::register::offer(c, psbt, &unregistered, &multisig_ours))
        .flatten();

    warnings.sort_by_key(|w| core::cmp::Reverse(w.level));

    Inspection {
        version: tx.version.0,
        locktime,
        rbf,
        inputs,
        outputs,
        fee,
        fee_rate_sat_vb,
        vsize_estimate,
        total_in,
        total_out,
        amount_to_others,
        change_total,
        participating_keys: participating,
        multisig,
        musig,
        warnings,
        signatures,
        nonce_reuse,
        inscriptions,
        is_self_transfer,
        register,
    }
}

/// A key origin naming loaded key `fp` at a `path` under its account
/// that does not give input `i`'s key or script. A key other than the
/// one the input names is a false claim about the person's own key, and
/// is blocked: signing refuses it too ([`crate::Error::KeyMismatch`]).
/// The input's own key in a script other than the account's is a
/// caution: the key is the person's, and signing checks it against the
/// script it spends.
fn mismatch(
    warnings: &mut Vec<Warning>,
    i: usize,
    fp: Fingerprint,
    path: &DerivationPath,
    same_key: bool,
) {
    let (level, text) = if same_key {
        (
            Level::Caution,
            format!(
                "input {i} names your key {fp} at {path}, in a script other than that account's"
            ),
        )
    } else {
        (
            Level::Blocked,
            format!(
                "input {i} names your key {fp} at {path}, but the derived key does not match the input"
            ),
        )
    };
    warn(warnings, level, WarningKind::UnknownDerivation, text);
}

/// Whether an input of `kind` is signed only through a registered
/// wallet whose script it is: a miniscript or a taproot tree (F135,
/// F141). Inspection counts a loaded key as taking part in such an
/// input only through that wallet, and [`crate::sign`] signs it only
/// where inspection does, so the two answer alike.
pub(crate) fn signed_through_wallet(kind: ScriptKind) -> bool {
    matches!(kind, ScriptKind::Miniscript { .. } | ScriptKind::P2trScript)
}

/// Whether a taproot script-path input carries something this crate can
/// sign: a leaf script, or an internal key that with the tree the input
/// states really is the output being spent.
fn tap_signable<C: Verification>(
    secp: &Secp256k1<C>,
    input: &Input,
    spk: Option<&bitcoin::Script>,
) -> bool {
    if !input.tap_scripts.is_empty() {
        return true;
    }
    match (input.tap_internal_key, spk) {
        (Some(internal), Some(spk)) => {
            ScriptBuf::new_p2tr(secp, internal, input.tap_merkle_root).as_script() == spk
        }
        _ => false,
    }
}

/// A wait in the plainest words this crate has; the screen's own
/// wording is `faraday-strings`'.
fn wait(lock: Lock) -> String {
    match lock {
        Lock::Blocks(n) => format!("{n} blocks"),
        Lock::Intervals(n) => format!("{n} intervals of 512 seconds"),
        Lock::Height(n) => format!("for block {n}"),
        Lock::Time(n) => format!("until Unix time {n}"),
        Lock::Preimage => "for a preimage".to_string(),
    }
}

/// Whether input `index` of `tx` has already waited the wait out, as
/// the chain reads it. A relative wait is a rule of version 2 and later
/// (BIP 68): at version 1 the sequence holds no wait. An absolute wait
/// holds only while the input's sequence is not final, 0xFFFFFFFF, which
/// turns the locktime off for it (BIP 65). A preimage is not a wait:
/// nothing about the transaction says whether the spender has it.
fn lock_met(lock: Lock, tx: &Transaction, index: usize) -> bool {
    let Some(txin) = tx.input.get(index) else {
        return false;
    };
    let relative = txin.sequence.to_consensus_u32();
    let versioned = tx.version.0.cast_unsigned() >= 2;
    let disabled = relative & (1 << 31) != 0;
    let is_time = relative & (1 << 22) != 0;
    let count = relative & 0xffff;
    let locked = txin.sequence != Sequence::MAX;
    match lock {
        Lock::Blocks(n) => versioned && !disabled && !is_time && count >= n,
        Lock::Intervals(n) => versioned && !disabled && is_time && count >= n,
        Lock::Height(n) => {
            locked
                && match tx.lock_time {
                    LockTime::Blocks(h) => h.to_consensus_u32() >= n,
                    LockTime::Seconds(_) => false,
                }
        }
        Lock::Time(n) => {
            locked
                && match tx.lock_time {
                    LockTime::Blocks(_) => false,
                    LockTime::Seconds(t) => t.to_consensus_u32() >= n,
                }
        }
        Lock::Preimage => true,
    }
}

/// Costs what `wallet_claim` costs. Whether input `index` spends a
/// registered wallet with a way to spend it that a loaded key takes part
/// in and that the transaction's own sequence and locktime leave open.
fn own_path_open(
    work: &Compute,
    ctx: &Context,
    input: &Input,
    c: &Classified,
    tx: &Transaction,
    index: usize,
) -> bool {
    let Some(utxo) = c.utxo.as_ref() else {
        return false;
    };
    let Some(claim) = ctx.wallet_claim(work, input_origins(input), &utxo.script_pubkey) else {
        return false;
    };
    let Some(policy) = ctx.wallets.get(claim.wallet) else {
        return false;
    };
    let keys = policy.keys();
    let own = |k: usize| {
        keys.get(k)
            .and_then(|key| key.fingerprint())
            .is_some_and(|fp| ctx.is_ours(fp))
    };
    policy
        .spend_paths()
        .iter()
        .any(|p| p.keys.iter().any(|&k| own(k)) && p.locks.iter().all(|l| lock_met(*l, tx, index)))
}

/// The keys a registered wallet's policy derives at one address, in the
/// order the policy lists them.
fn wallet_keys_at<C: Verification>(
    secp: &Secp256k1<C>,
    policy: &faraday_bip::policy::WalletPolicy,
    change: bool,
    index: u32,
) -> Option<Vec<PublicKey>> {
    let leaf = [
        ChildNumber::from_normal_idx(u32::from(change)).ok()?,
        ChildNumber::from_normal_idx(index).ok()?,
    ];
    policy
        .keys()
        .iter()
        .map(|k| {
            k.xpub()
                .derive_pub(secp, &leaf.to_vec())
                .ok()
                .map(|x| x.public_key)
        })
        .collect()
}

/// The keys a taproot leaf script checks signatures against, read
/// through the miniscript. A `pkh` fragment leaves only a hash behind;
/// the key it stands for comes from the input's own `tap_key_origins`.
fn leaf_keys(input: &Input, leaf: &ScriptBuf) -> Vec<bitcoin::secp256k1::XOnlyPublicKey> {
    let ext = ExtParams {
        raw_pkh: true,
        ..ExtParams::sane()
    };
    let Ok(ms) = Miniscript::<bitcoin::secp256k1::XOnlyPublicKey, Tap>::parse_with_ext(leaf, &ext)
    else {
        return Vec::new();
    };
    let mut hashes = alloc::collections::BTreeMap::new();
    for key in input.tap_key_origins.keys() {
        hashes.insert(key.to_pubkeyhash(SigType::Schnorr), *key);
    }
    crate::classify::script_key_set(&ms, &hashes)
}

/// Costs what `wallet_claim` costs. Which way an input spends its
/// wallet's script.
///
/// A taproot input that carries leaf scripts spends one of those leaves;
/// one that carries only its internal key spends the key path. A `wsh`
/// script has one script and several ways through it, and what narrows
/// them is the PSBT itself: the keys it states origins for, and then the
/// sequence and locktime the coordinator set, which are what arm a
/// timelocked path. The answer is `None` unless exactly one way is left.
fn spend_route<C: Verification>(
    work: &Compute,
    secp: &Secp256k1<C>,
    ctx: &Context,
    input: &Input,
    c: &Classified,
    tx: &Transaction,
    index: usize,
) -> Option<SpendRoute> {
    let spk = &c.utxo.as_ref()?.script_pubkey;
    let claim = ctx.wallet_claim(work, input_origins(input), spk)?;
    if !claim.verified {
        return None;
    }
    let policy = ctx.wallets.get(claim.wallet)?;
    let paths = policy.spend_paths();
    if paths.is_empty() {
        return None;
    }
    let keys = wallet_keys_at(secp, policy, claim.change, claim.index)?;

    let candidates: Vec<SpendPath> = match c.kind {
        ScriptKind::Miniscript { .. } => {
            let named: Vec<usize> = keys
                .iter()
                .enumerate()
                .filter(|(_, k)| input.bip32_derivation.contains_key(k))
                .map(|(i, _)| i)
                .collect();
            paths
                .iter()
                .filter(|p| !p.keys.is_empty() && p.keys.iter().all(|k| named.contains(k)))
                .cloned()
                .collect()
        }
        ScriptKind::P2trScript if !input.tap_scripts.is_empty() => {
            let xonly: Vec<bitcoin::secp256k1::XOnlyPublicKey> =
                keys.iter().map(|k| k.x_only_public_key().0).collect();
            let mut out: Vec<SpendPath> = Vec::new();
            for (leaf, _) in input.tap_scripts.values() {
                let mut here: Vec<usize> = leaf_keys(input, leaf)
                    .iter()
                    .filter_map(|k| xonly.iter().position(|x| x == k))
                    .collect();
                here.sort_unstable();
                here.dedup();
                for path in &paths {
                    if path.keys == here && !out.contains(path) {
                        out.push(path.clone());
                    }
                }
            }
            out
        }
        ScriptKind::P2trScript => {
            return tap_signable(secp, input, Some(spk)).then_some(SpendRoute::KeyPath);
        }
        _ => return None,
    };

    // One way left is the answer. Where several are, the transaction's
    // own sequence and locktime say which of them the coordinator armed,
    // and the most specific of those — the one that waited longest — is
    // the path it spends. Where they arm none, the paths are read as
    // they stand, which is how an unmet timelock still names its path.
    let met: Vec<SpendPath> = candidates
        .iter()
        .filter(|p| p.locks.iter().all(|l| lock_met(*l, tx, index)))
        .cloned()
        .collect();
    let pool = if met.is_empty() { candidates } else { met };
    let best = pool.iter().map(|p| (p.locks.len(), p.keys.len())).max()?;
    let mut top = pool
        .iter()
        .filter(|p| (p.locks.len(), p.keys.len()) == best);
    let chosen = top.next()?.clone();
    if top.next().is_some() {
        return None;
    }
    Some(SpendRoute::Script(chosen))
}

fn push_unique(list: &mut Vec<Fingerprint>, fp: Fingerprint) {
    if !list.contains(&fp) {
        list.push(fp);
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from(DIGITS[usize::from(b >> 4)]));
        s.push(char::from(DIGITS[usize::from(b & 15)]));
    }
    s
}

/// Costs what `wallet_claim` costs. The registered wallet an input
/// spends: the one its key origins name, when the wallet's own script
/// at that index is the input's. Taproot states its origins in their
/// own field, so both are read.
fn wallet_input(
    c: &Compute,
    ctx: &Context,
    input: &Input,
    spk: Option<&bitcoin::Script>,
) -> Option<usize> {
    let spk = spk?;
    let claim = ctx.wallet_claim(c, input_origins(input), spk)?;
    claim.verified.then_some(claim.wallet)
}

/// Every key origin an input states, whichever field it states it in.
fn input_origins(input: &Input) -> impl Iterator<Item = (Fingerprint, &DerivationPath)> + Clone {
    input
        .bip32_derivation
        .values()
        .map(|(fp, path)| (Fingerprint::from(*fp), path))
        .chain(
            input
                .tap_key_origins
                .values()
                .map(|(_, (fp, path))| (Fingerprint::from(*fp), path)),
        )
}

/// Costs a key derivation per cosigner a loaded fingerprint names. The
/// keys of a multisig script, with whose each one is: a key is a loaded
/// key's only where that key, derived at the path the input states for
/// it, is this key, so a fingerprint stated against a cosigner's key
/// claims nothing.
fn cosigners<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    ms: &Multisig,
    input: &Input,
    ctx: &Context,
) -> Vec<Cosigner> {
    ms.keys
        .iter()
        .map(|key| {
            let stated = input.bip32_derivation.get(&key.inner);
            let fingerprint = stated.map(|(fp, _)| Fingerprint::from(*fp));
            let ours = stated.is_some_and(|(fp, path)| {
                derives_to(c, secp, ctx, (*fp).into(), path, &key.inner) == Some(true)
            });
            Cosigner {
                fingerprint,
                signed: input.partial_sigs.contains_key(key),
                ours,
            }
        })
        .collect()
}

/// Costs a key derivation. Whether `key` is what the loaded key `fp`
/// derives at `path`: from its accounts, single-signature or BIP-48, or
/// from the extended keys the session worked out from it for its
/// wallets' paths ([`crate::KeyRef::extra`]). A wallet's own word for a key
/// under the person's fingerprint is never taken. `None` where `fp` is
/// not loaded or none of those derives the path.
fn derives_to<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    ctx: &Context,
    fp: Fingerprint,
    path: &DerivationPath,
    key: &PublicKey,
) -> Option<bool> {
    let derived = ctx.key(fp)?.derive_key(c, secp, path)?;
    Some(derived == *key)
}

fn sighash_kind(input: &Input, kind: ScriptKind, spk: Option<&bitcoin::Script>) -> SighashKind {
    let taproot = match kind {
        ScriptKind::P2trKey | ScriptKind::P2trScript => true,
        ScriptKind::Unknown => spk.is_some_and(|s| s.is_p2tr()),
        _ => false,
    };
    match input.sighash_type {
        None if taproot => SighashKind::Taproot(TapSighashType::Default),
        None => SighashKind::Ecdsa(EcdsaSighashType::All),
        Some(t) if taproot => t
            .taproot_hash_ty()
            .map_or(SighashKind::NonStandard(t.to_u32()), SighashKind::Taproot),
        Some(t) => t
            .ecdsa_hash_ty()
            .map_or(SighashKind::NonStandard(t.to_u32()), SighashKind::Ecdsa),
    }
}

/// Costs what `output_claims` costs. Change detection:
/// verified only when a stated origin
/// names a loaded key and re-deriving it reproduces the output script
/// exactly.
///
/// An output states one origin per key of the script, so a multisig
/// change output names cosigners this device does not hold. What those
/// origins are worth is decided by the output as a whole: once an origin
/// has verified the output, a caution another origin raised is noise and
/// goes. A danger, and the block above it, stay either way — an origin
/// that claims our key and derives to some other script is worth saying
/// even about an output we went on to verify.
fn output_kind<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    out: &Output,
    txout: &TxOut,
    ctx: &Context,
    i: usize,
    warnings: &mut Vec<Warning>,
) -> OutputKind {
    let mut claim_warnings = Vec::new();
    let kind = output_claims(c, secp, out, txout, ctx, i, &mut claim_warnings);
    let verified = matches!(
        kind,
        OutputKind::Change { .. } | OutputKind::WalletChange { .. }
    );
    if verified {
        claim_warnings.retain(|w| w.level >= Level::Danger);
    }
    warnings.append(&mut claim_warnings);
    kind
}

/// Costs what `musig_output` and `wallet_claim` cost. The body of
/// [`output_kind`], with every warning its claims raise.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn output_claims<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    out: &Output,
    txout: &TxOut,
    ctx: &Context,
    i: usize,
    warnings: &mut Vec<Warning>,
) -> OutputKind {
    let spk = &txout.script_pubkey;
    let mut kind = OutputKind::Recipient;
    match musig_output(c, secp, out, txout, ctx) {
        Some(Ok((wallet, change, index))) => {
            return OutputKind::WalletChange {
                wallet,
                change,
                index,
            };
        }
        Some(Err(())) => {
            warn(
                warnings,
                Level::Blocked,
                WarningKind::ChangeSpoof,
                format!(
                    "output {i} claims to be change of a MuSig2 wallet you are using, but the script is not that address"
                ),
            );
            let claimed = out
                .tap_key_origins
                .values()
                .map(|(_, (fp, _))| Fingerprint::from(*fp))
                .find(|fp| ctx.is_ours(*fp));
            return match claimed {
                Some(key) => OutputKind::UnverifiedChange { key },
                None => OutputKind::Recipient,
            };
        }
        None => {}
    }
    let origins = out
        .bip32_derivation
        .values()
        .map(|(fp, path)| (Fingerprint::from(*fp), path))
        .chain(
            out.tap_key_origins
                .values()
                .map(|(_, (fp, path))| (Fingerprint::from(*fp), path)),
        );
    if let Some(claim) = ctx.wallet_claim(c, origins, spk) {
        if claim.verified {
            return OutputKind::WalletChange {
                wallet: claim.wallet,
                change: claim.change,
                index: claim.index,
            };
        }
        warn(
            warnings,
            Level::Blocked,
            WarningKind::ChangeSpoof,
            format!(
                "output {i} claims to be change of a wallet you are using, but the script is not that address"
            ),
        );
        return OutputKind::UnverifiedChange {
            key: claim.fingerprint,
        };
    }
    let claims = out
        .bip32_derivation
        .iter()
        .map(|(pk, (fp, path))| (*pk, Fingerprint::from(*fp), path, false))
        .chain(
            out.tap_key_origins
                .iter()
                .map(|(xonly, (leaves, (fp, path)))| {
                    // A taproot claim is only a key-path claim when the
                    // internal key is the stated key and there is no tree.
                    let plain = leaves.is_empty()
                        && out.tap_internal_key == Some(*xonly)
                        && out.tap_tree.is_none();
                    (
                        PublicKey::from_x_only_public_key(*xonly, bitcoin::key::Parity::Even),
                        Fingerprint::from(*fp),
                        path,
                        !plain,
                    )
                }),
        );
    for (pk, fp, path, tainted) in claims {
        let Some(key) = ctx.key(fp) else {
            kind = OutputKind::UnverifiedChange { key: fp };
            warn(
                warnings,
                Level::Caution,
                WarningKind::UnverifiedChange,
                format!(
                    "output {i} claims to belong to key {fp}, which is not loaded; treat it as a payment to others"
                ),
            );
            continue;
        };
        match key.derive(secp, path) {
            Some(d)
                if !tainted
                    && d.key.x_only_public_key().0 == pk.x_only_public_key().0
                    && d.script_pubkey == *spk =>
            {
                kind = OutputKind::Change {
                    key: fp,
                    change: d.change,
                    index: d.index,
                };
                return kind;
            }
            Some(_) => {
                kind = OutputKind::UnverifiedChange { key: fp };
                warn(
                    warnings,
                    Level::Blocked,
                    WarningKind::ChangeSpoof,
                    format!(
                        "output {i} claims to be change of your key {fp} at {path}, but the script is not that address"
                    ),
                );
            }
            None => {
                kind = OutputKind::UnverifiedChange { key: fp };
                let text = if path
                    .into_iter()
                    .next()
                    .is_some_and(|c| *c == ChildNumber::Hardened { index: 48 })
                {
                    format!(
                        "output {i} claims to be change of your key {fp} at {path}; no wallet registered on this device holds that script, so it is counted as a payment to others. Register the wallet to check its change."
                    )
                } else {
                    format!(
                        "output {i} claims to be change of your key {fp} at {path}, which is not under a loaded account; treat it as a payment to others"
                    )
                };
                warn(
                    warnings,
                    Level::Caution,
                    WarningKind::UnverifiedChange,
                    text,
                );
            }
        }
    }
    kind
}

fn locktime(lt: LockTime) -> Locktime {
    let raw = lt.to_consensus_u32();
    let kind = match lt {
        LockTime::Blocks(h) if h.to_consensus_u32() == 0 => LocktimeKind::None,
        LockTime::Blocks(h) => LocktimeKind::Height(h.to_consensus_u32()),
        LockTime::Seconds(t) => LocktimeKind::Time(t.to_consensus_u32()),
    };
    Locktime { raw, kind }
}

/// A scriptPubKey carries no network, so the network is checked where it
/// is visible: the loaded accounts, global xpubs, and the coin type of
/// every BIP-44/49/84/86/48 key path.
fn network_checks(inner: &bitcoin::Psbt, ctx: &Context, warnings: &mut Vec<Warning>) {
    for key in ctx.keys {
        if let Some(acct) = key.accounts.iter().find(|a| a.network() != ctx.network) {
            warn(
                warnings,
                Level::Danger,
                WarningKind::NetworkMismatch,
                format!(
                    "key {} is loaded for {} but the transaction is being checked for {}",
                    key.fingerprint,
                    acct.network(),
                    ctx.network
                ),
            );
            break;
        }
    }
    if inner
        .xpub
        .keys()
        .any(|xpub| xpub.network != ctx.network.kind())
    {
        warn(
            warnings,
            Level::Danger,
            WarningKind::NetworkMismatch,
            format!(
                "a global xpub is for a different network than {}",
                ctx.network
            ),
        );
    }
    let coin = ctx.network.coin_type();
    let paths = inner
        .inputs
        .iter()
        .enumerate()
        .flat_map(|(i, input)| {
            input
                .bip32_derivation
                .values()
                .map(move |(_, p)| ("input", i, p))
                .chain(
                    input
                        .tap_key_origins
                        .values()
                        .map(move |(_, (_, p))| ("input", i, p)),
                )
        })
        .chain(inner.outputs.iter().enumerate().flat_map(|(i, out)| {
            out.bip32_derivation
                .values()
                .map(move |(_, p)| ("output", i, p))
                .chain(
                    out.tap_key_origins
                        .values()
                        .map(move |(_, (_, p))| ("output", i, p)),
                )
        }));
    for (what, i, path) in paths {
        let v: Vec<ChildNumber> = path.into_iter().copied().collect();
        if let [
            ChildNumber::Hardened {
                index: 44 | 49 | 84 | 86 | 48,
            },
            ChildNumber::Hardened { index: c },
            ..,
        ] = v[..]
            && c != coin
        {
            warn(
                warnings,
                Level::Danger,
                WarningKind::NetworkMismatch,
                format!(
                    "{what} {i} key path {path} is for coin type {c}; {} uses {coin}",
                    ctx.network
                ),
            );
            return;
        }
    }
}

// ---------------------------------------------------------------------
// BIP-373 MuSig2 inputs and outputs
// ---------------------------------------------------------------------

/// What one of this device's keys is on a MuSig2 input: a participant of
/// the aggregate, named by its own origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusigParticipation {
    /// The loaded key's master fingerprint.
    pub fingerprint: Fingerprint,
    /// The origin the PSBT states for it, which is the account path the
    /// registered wallet states.
    pub path: DerivationPath,
    /// The participant public key at that path.
    pub key: PublicKey,
}

/// Everything establishing a MuSig2 input settled, so that signing reads
/// it rather than working it out again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusigInput {
    /// Position in [`Context::wallets`].
    pub wallet: usize,
    /// The participants, sorted with `KeySort`, which is the order they
    /// aggregate in.
    pub participants: Vec<PublicKey>,
    /// `KeyAgg` of those participants, before any derivation: the key
    /// the participants field is keyed by.
    pub root_aggregate: PublicKey,
    /// The change chain rather than the receive chain.
    pub change: bool,
    /// Address index.
    pub index: u32,
    /// The tweaks from the root aggregate to the output key: one plain
    /// tweak per sub-path step, then BIP-341's x-only tap tweak.
    pub tweaks: Vec<faraday_bip::musig::Tweak>,
    /// The taproot internal key, which is the aggregate derived along
    /// the sub-path.
    pub internal_key: bitcoin::secp256k1::XOnlyPublicKey,
    /// The taproot output key with its parity byte, which is what a
    /// nonce or partial signature this device writes is keyed by.
    pub output_key: PublicKey,
    /// This device's participants on the input.
    pub ours: Vec<MusigParticipation>,
    /// Every participant this device holds will share a public nonce
    /// this pass and sign none of it: round 1 (§16.100).
    pub will_share_nonce: bool,
    /// At least one public nonce of this device's on the input belongs
    /// to no open session, so this pass replaces it.
    pub nonce_stale: bool,
}

/// Which of §16.100's three ways one participant of a MuSig2 input is
/// handled in this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusigRound {
    /// This participant's partial signature is already on the input, so
    /// nothing is drawn and nothing is signed: BIP-327 says a
    /// participant signs once. What is left is the aggregation.
    Done,
    /// Round 2: the input carries this participant's public nonce, the
    /// session holds its secret, and every other nonce is there, so the
    /// partial signature is made now.
    Sign,
    /// The session holds this participant's secret nonce and the input
    /// carries its public one, but another participant has not written
    /// theirs yet. The session waits; nothing is drawn and nothing is
    /// signed.
    Wait,
    /// Signing last with `DeterministicSign`: no nonce of this
    /// participant's is on the input and every other one is.
    SignLast,
    /// Round 1: a nonce is drawn, kept in the session and written to the
    /// input. `stale` is a nonce of this participant's already there
    /// that no session holds, which this round replaces.
    Share {
        /// The nonce being replaced.
        stale: bool,
    },
}

/// Whether a nonce or partial-signature entry's aggregate key is one of
/// the three forms this input's aggregate takes: the root aggregate, the
/// aggregate derived at the sub-path, or the taproot output key. BIP-373
/// reads as the first; Bitcoin Core writes the third.
pub(crate) fn musig_keyed_here(musig: &MusigInput, aggregate: &PublicKey) -> bool {
    *aggregate == musig.root_aggregate
        || *aggregate == musig.output_key
        || aggregate.x_only_public_key().0 == musig.internal_key
}

/// Which way `participation` is handled on input `index` of the
/// transaction `txid`, decided in §16.100's order.
pub(crate) fn musig_round(
    input: &Input,
    musig: &MusigInput,
    participation: &MusigParticipation,
    session: Option<&crate::musig::MusigSessionView>,
    txid: bitcoin::Txid,
    index: usize,
) -> MusigRound {
    let nonces = crate::musig::pub_nonces(input).unwrap_or_default();
    let here = |key: &PublicKey| {
        nonces
            .iter()
            .any(|n| n.participant == *key && musig_keyed_here(musig, &n.aggregate))
    };
    let signed = crate::musig::partial_sigs(input)
        .unwrap_or_default()
        .iter()
        .any(|e| e.participant == participation.key && musig_keyed_here(musig, &e.aggregate));
    if signed {
        return MusigRound::Done;
    }
    let ours = here(&participation.key);
    let others = musig
        .participants
        .iter()
        .filter(|p| **p != participation.key)
        .all(here);
    let held = session.is_some_and(|s| s.holds(txid, index, &participation.key));
    match (ours, held, others) {
        (true, true, true) => MusigRound::Sign,
        (true, true, false) => MusigRound::Wait,
        (false, _, true) => MusigRound::SignLast,
        (ours, _, _) => MusigRound::Share { stale: ours },
    }
}

/// Whether an input carries `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` at all,
/// which is what makes a `P2trKey` input a MuSig2 input.
fn has_musig_field(input: &Input) -> bool {
    input
        .unknown
        .keys()
        .any(|k| k.type_value == crate::musig::IN_PARTICIPANT_PUBKEYS)
}

/// A MuSig2 input that could not be established, with the warning that
/// blocks it.
struct MusigBlock {
    kind: WarningKind,
    reason: &'static str,
}

fn musig_block(kind: WarningKind, reason: &'static str) -> Result<Option<MusigInput>, MusigBlock> {
    Err(MusigBlock { kind, reason })
}

/// The tweaks from the root aggregate down to the output key, and the
/// internal key the sub-path derives.
///
/// One plain tweak per unhardened step, each `Xpub::ckd_pub_tweak` of
/// the BIP-328 aggregate xpub at that step so that the chain code chains
/// along, then the x-only tap tweak of the derived key with no merkle
/// root.
fn musig_tweaks<C: Verification>(
    secp: &Secp256k1<C>,
    aggregate_xpub: Xpub,
    steps: &[ChildNumber],
) -> Option<(Vec<faraday_bip::musig::Tweak>, PublicKey)> {
    let mut xpub = aggregate_xpub;
    let mut tweaks = Vec::with_capacity(steps.len() + 1);
    for step in steps {
        let (tweak, _) = xpub.ckd_pub_tweak(*step).ok()?;
        tweaks.push(faraday_bip::musig::Tweak {
            bytes: tweak.secret_bytes(),
            x_only: false,
        });
        xpub = xpub.ckd_pub(secp, *step).ok()?;
    }
    let internal = xpub.public_key;
    let tap = TapTweakHash::from_key_and_tweak(internal.x_only_public_key().0, None);
    tweaks.push(faraday_bip::musig::Tweak {
        bytes: tap.to_byte_array(),
        x_only: true,
    });
    Some((tweaks, internal))
}

/// The registered `Template::MuSig` wallet whose participant keys are
/// `keys` as a set, with those keys in aggregation order.
fn musig_wallet(ctx: &Context, keys: &[PublicKey]) -> Option<(usize, Vec<PublicKey>)> {
    for (i, policy) in ctx.wallets.iter().enumerate() {
        if policy.template() != faraday_bip::policy::Template::MuSig {
            continue;
        }
        let wallet: Vec<PublicKey> = policy.keys().iter().map(|k| k.xpub().public_key).collect();
        if wallet.len() != keys.len() {
            continue;
        }
        if wallet.iter().all(|k| keys.contains(k)) && keys.iter().all(|k| wallet.contains(k)) {
            return Some((i, faraday_bip::musig::sort_keys(&wallet)));
        }
    }
    None
}

/// The two unhardened steps a MuSig2 sub-path must be: `chain/index`
/// with the chain 0 or 1, which is what the wallet's `/<0;1>/*` allows.
fn musig_sub_path(path: &DerivationPath) -> Option<(bool, u32)> {
    match path.into_iter().copied().collect::<Vec<_>>()[..] {
        [
            ChildNumber::Normal {
                index: chain @ (0 | 1),
            },
            ChildNumber::Normal { index },
        ] => Some((chain == 1, index)),
        _ => None,
    }
}

/// Costs an address derivation (through `script_at_with`). Establishes
/// what a `P2trKey` input carrying `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS`
/// is, in the order §16.100 states: the registered wallet, the
/// aggregate, this device's participants, the sub-path, and the script.
/// `Ok(None)` is an input that is not a MuSig2 input at all.
///
/// A merkle root never reaches here: an input that states one is
/// classified [`ScriptKind::P2trScript`], which this crate refuses to
/// sign for a `musig()` wallet, since `tr(musig())` has no tree.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn musig_input<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    ctx: &Context,
    input: &Input,
    spk: Option<&bitcoin::Script>,
) -> Result<Option<MusigInput>, MusigBlock> {
    let fields = match crate::musig::input_participants(input) {
        Ok(fields) => fields,
        Err(e) => return musig_block(WarningKind::UtxoMismatch, e.reason()),
    };
    if fields.is_empty() {
        return Ok(None);
    }
    if crate::musig::pub_nonces(input).is_err() || crate::musig::partial_sigs(input).is_err() {
        let reason = crate::musig::pub_nonces(input)
            .err()
            .or_else(|| crate::musig::partial_sigs(input).err())
            .map_or("a MuSig2 field is malformed", |e| e.reason());
        return musig_block(WarningKind::UtxoMismatch, reason);
    }

    // 1. The registered wallet: the participant set is what the
    //    aggregate is, so a set no registered wallet has is a different
    //    wallet and never one this device signs for.
    let Some((field, (wallet, participants))) = fields
        .iter()
        .find_map(|f| musig_wallet(ctx, &f.keys).map(|w| (f, w)))
    else {
        return musig_block(
            WarningKind::MusigWalletNotRegistered,
            "no registered MuSig2 wallet has these participants",
        );
    };

    // 2. The aggregate the field is keyed by.
    let Ok(root) = faraday_bip::musig::key_agg_with(secp, &participants) else {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 participants do not aggregate",
        );
    };
    if root.public_key() != field.aggregate {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 participants are not the key the field names",
        );
    }

    // 3. Which loaded keys are participants. The registered wallet is
    //    the authority for the account path; the PSBT's own origin is
    //    what names the key on this input.
    let policy = ctx.wallets.get(wallet).expect("wallet found above");
    let mut ours: Vec<MusigParticipation> = Vec::new();
    for (xonly, (leaves, (fp, path))) in &input.tap_key_origins {
        if !leaves.is_empty() {
            continue;
        }
        let fingerprint = Fingerprint::from(*fp);
        let Some(key) = ctx.key(fingerprint) else {
            continue;
        };
        let Some(policy_key) = policy.key(fingerprint) else {
            continue;
        };
        if policy_key.path() != Some(path) {
            continue;
        }
        let Some(account) = key.accounts.iter().find(|a| a.path() == path) else {
            continue;
        };
        let derived = account.xpub().public_key;
        if derived.x_only_public_key().0 != *xonly || !participants.contains(&derived) {
            continue;
        }
        ours.push(MusigParticipation {
            fingerprint,
            path: path.clone(),
            key: derived,
        });
    }

    // 4. The sub-path, off the entry the BIP-328 aggregate xpub's own
    //    fingerprint names for the internal key.
    let Some(internal_key) = input.tap_internal_key else {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 input states no internal key",
        );
    };
    let aggregate_xpub = faraday_bip::musig::aggregate_xpub(&participants, ctx.network)
        .expect("the participants aggregate");
    let aggregate_fingerprint = Fingerprint(aggregate_xpub.fingerprint().to_bytes());
    let sub_path = input
        .tap_key_origins
        .get(&internal_key)
        .filter(|(leaves, (fp, _))| {
            leaves.is_empty() && Fingerprint::from(*fp) == aggregate_fingerprint
        })
        .and_then(|(_, (_, path))| musig_sub_path(path));
    let Some((change, index)) = sub_path else {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 input states no chain and index below the aggregate key",
        );
    };

    // 5. The internal key and the script the wallet pays to at that
    //    address.
    let steps = [
        ChildNumber::from_normal_idx(u32::from(change)).expect("0 or 1"),
        ChildNumber::from_normal_idx(index).expect("an unhardened index"),
    ];
    let Some((tweaks, derived)) = musig_tweaks(secp, aggregate_xpub, &steps) else {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 aggregate key does not derive at the stated chain and index",
        );
    };
    if derived.x_only_public_key().0 != internal_key {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 internal key is not the aggregate at the stated chain and index",
        );
    }
    let mut output = root;
    for tweak in &tweaks {
        match output.apply_tweak(secp, &tweak.bytes, tweak.x_only) {
            Ok(next) => output = next,
            Err(_) => {
                return musig_block(
                    WarningKind::UtxoMismatch,
                    "the MuSig2 aggregate key does not tweak to the output key",
                );
            }
        }
    }
    let wallet_script = policy.script_at_with(c, secp, change, index).ok();
    if wallet_script.as_deref() != spk {
        return musig_block(
            WarningKind::UtxoMismatch,
            "the MuSig2 input does not pay to the registered wallet's address",
        );
    }

    Ok(Some(MusigInput {
        wallet,
        participants,
        root_aggregate: root.public_key(),
        change,
        index,
        tweaks,
        internal_key,
        output_key: output.public_key(),
        ours,
        will_share_nonce: false,
        nonce_stale: false,
    }))
}

/// Costs an address derivation (through `script_at_with`). A MuSig2
/// change output: the participants field names a registered wallet, the
/// internal key's origin carries the aggregate's fingerprint and a
/// two-step sub-path, and the script is the wallet's own there.
///
/// `Some(Err(()))` is an output that claims the wallet and pays
/// somewhere else, which is the `ChangeSpoof` a multisig claim would be.
fn musig_output<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    out: &Output,
    txout: &TxOut,
    ctx: &Context,
) -> Option<Result<(usize, bool, u32), ()>> {
    let fields = crate::musig::output_participants(out).ok()?;
    let (_, (wallet, participants)) = fields
        .iter()
        .find_map(|f| musig_wallet(ctx, &f.keys).map(|w| (f, w)))?;
    let policy = ctx.wallets.get(wallet)?;
    let aggregate_xpub = faraday_bip::musig::aggregate_xpub(&participants, ctx.network).ok()?;
    let aggregate_fingerprint = Fingerprint(aggregate_xpub.fingerprint().to_bytes());
    let claim = out.tap_internal_key.and_then(|internal| {
        out.tap_key_origins
            .get(&internal)
            .filter(|(leaves, (fp, _))| {
                leaves.is_empty() && Fingerprint::from(*fp) == aggregate_fingerprint
            })
            .and_then(|(_, (_, path))| musig_sub_path(path))
    });
    let Some((change, index)) = claim else {
        return Some(Err(()));
    };
    match policy.script_at_with(c, secp, change, index) {
        Ok(script) if script == txout.script_pubkey => Some(Ok((wallet, change, index))),
        _ => Some(Err(())),
    }
}

/// The proprietary identifier of Faraday's own `osk` records.
const OSK: &[u8] = b"osk";

/// Whether an input carries any `osk` record at all.
fn has_osk_records(input: &Input) -> bool {
    input.proprietary.keys().any(|k| k.prefix == OSK)
}
