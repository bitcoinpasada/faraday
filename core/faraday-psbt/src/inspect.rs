//! The structured inspection behind the Sign screens:
//! summary, outputs with verified change, inputs, and ranked warnings.
//!
//! Inspection needs no private key. Ownership is verified by re-deriving
//! key-origin paths from the account xpubs in the [`Context`] and
//! comparing the resulting script with the one in the PSBT; a claim that
//! names one of our fingerprints but does not reproduce the script blocks
//! signing, never a silent skip.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use bitcoin::absolute::LockTime;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use bitcoin::hashes::Hash;
use bitcoin::psbt::{Input, Output};
use bitcoin::secp256k1::{PublicKey, Secp256k1, Verification};
use bitcoin::sighash::{EcdsaSighashType, TapSighashType};
use bitcoin::taproot::TapTweakHash;
use bitcoin::{Amount, ScriptBuf, Sequence, Transaction, TxOut, Txid};
use faraday_bip::keys::{Fingerprint, ScriptType};
use faraday_crypto::work::{Compute, Work};

use faraday_bip::spend::{Lock, SpendPath};
use miniscript::{ExtParams, Miniscript, SigType, Tap, ToPublicKey};

use crate::Psbt;
use crate::classify::{Classified, Multisig, ScriptKind};
use crate::context::Context;
use crate::reason::Reason;

mod checks;
mod detail;
mod inputs;
mod outputs;
mod sighash;

use checks::{Fee, Signed, Totals};
use inputs::{Inputs, Walk, read_inputs};
use outputs::{Outputs, read_outputs};

pub use detail::Detail;
pub(crate) use sighash::{sighash_answers, sighash_stricter};

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
/// A change output at this address index or above, on the change or
/// receive chain, is not verified change (F242): a wallet scanning with a
/// gap limit does not find coins that far along.
pub const FAR_CHANGE_INDEX: u32 = 10_000;

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
    /// An output is change that derives correctly, at an address index of
    /// [`FAR_CHANGE_INDEX`] or more, which a wallet with a gap limit does
    /// not find (F242).
    ChangeFarIndex,
    /// An output claims to be change of a loaded key but the derived
    /// script does not match. Treat as an attack.
    ChangeSpoof,
    /// An output is an address of one of the person's wallets other than
    /// the one their inputs spend: counted as sent (F254).
    TransferToOwnWallet,
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
    /// A time lock is set; the signer has no clock, so a caution states
    /// it as a date. A height lock is no warning.
    LocktimeInFuture,
    /// An input's sequence sets a relative wait (BIP 68).
    RelativeLock,
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
    /// An amount, or a total of them, is over 21,000,000 BTC, which
    /// Bitcoin Core refuses (`bad-txns-vout-toolarge`,
    /// `bad-txns-txouttotal-toolarge`, `bad-txns-inputvalues-outofrange`).
    /// The totals and the fee shown are not what the PSBT states.
    AmountOutOfRange,
    /// The transaction is a BIP-322 signed-message request (version 0, or
    /// one OP_RETURN output spending a message's `to_spend`), not a spend.
    MessageRequest,
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
    /// A signature already on the transaction does not verify against
    /// the key it is under: the transaction was altered after it was
    /// signed, or the signer that made it is broken.
    InvalidSignature {
        /// The input it signs.
        input: usize,
        /// The key it is under, named as the review names one.
        key: String,
    },
    /// A finalized `scriptSig` or witness does not match the output the
    /// input spends: its script is not the one the output hashes, its
    /// shape is not the output kind's, a taproot script-path witness does
    /// not tie its leaf to the output key, or a multisig holds fewer
    /// signatures than it asks for. No node relays such a spend (a few
    /// shapes a miner could still take, such as a p2sh-wrapped taproot
    /// witness whose control block consensus ignores, are refused all the
    /// same, as nobody should sign them).
    WitnessMismatch {
        /// The input.
        input: usize,
    },
    /// A finalized input's fields satisfy the shape of the output it spends
    /// and no signature in them reads wrong, but the interpreter refuses
    /// them (a time lock the transaction does not meet, a push that is not
    /// minimal).
    FinalFieldsRefused {
        /// The input.
        input: usize,
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
    /// The PSBT carries pairs this crate does not read: unknown or
    /// proprietary fields, less the MuSig2 pairs it does read.
    UnknownFields {
        /// How many pairs, over the whole PSBT.
        count: usize,
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

/// A ranked warning and what it states, which the app words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Severity.
    pub level: Level,
    /// Category.
    pub kind: WarningKind,
    /// What it states, with the data it states it of.
    pub detail: Detail,
}

/// The transaction's absolute locktime in human terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Locktime {
    /// The consensus value.
    pub raw: u32,
    /// What the value means.
    pub kind: LocktimeKind,
    /// The date and time (UTC) of a time lock, worked out once here.
    pub date: Option<LockDate>,
    /// Whether the locktime holds: some input's sequence is not final,
    /// 0xFFFFFFFF, which turns the locktime off for that input (BIP 65).
    /// Worked out once here; a lock that does not hold is not stated.
    pub holds: bool,
}

/// A date and time in UTC, to the minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockDate {
    /// Year.
    pub year: u16,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of the month, 1 to 31.
    pub day: u8,
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute, 0 to 59.
    pub minute: u8,
}

/// A relative wait an input's sequence sets (BIP 68).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativeLock {
    /// This many blocks after the coin confirmed.
    Blocks(u16),
    /// This many intervals of 512 seconds after the coin confirmed.
    Intervals(u16),
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
            // not shown: a developer's text; the app words a locktime and a sighash
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
            // not shown: a developer's text; the app words a locktime and a sighash
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
    /// The relative wait the sequence sets, when it sets one: version 2
    /// or later, the disable flag clear, a count above zero (BIP 68).
    pub relative_lock: Option<RelativeLock>,
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
    /// Verified as an address of one of the person's wallets, but not of
    /// the wallet every input of theirs spends: a payment from one of
    /// their wallets into another, counted as sent (F254).
    Transfer {
        /// The wallet it pays into.
        to: OwnWallet,
        /// Whether it is on the change chain rather than the receive
        /// chain.
        change: bool,
        /// Address index.
        index: u32,
    },
}

/// One of the person's wallets: a loaded key's single-key account, or a
/// registered wallet that holds a loaded key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnWallet {
    /// A loaded key's single-key account (BIP 44, 49, 84 or 86).
    Account {
        /// The loaded key.
        key: Fingerprint,
        /// The account's path from the master key.
        path: DerivationPath,
    },
    /// Position in [`Context::wallets`].
    Registered(usize),
}

impl OwnWallet {
    /// One wallet named one way: a registered wallet that is one key's own
    /// single-key script (`wpkh`, `sh(wpkh)`, `pkh`, `tr(key)`) is that
    /// key's account when its script type is that account's purpose (`wpkh`
    /// at 84', `sh(wpkh)` at 49', `pkh` at 44', `tr(key)` at 86'), so an
    /// input read by the account and its change read through the wallet
    /// are the same wallet's; any other pairing is a wallet of its own. A
    /// lookup.
    fn named(self, ctx: &Context) -> Self {
        if let OwnWallet::Registered(w) = self
            && let Some(p) = ctx.wallets.get(w)
            && let Some(account) = Self::account_of_wallet(p)
        {
            return account;
        }
        self
    }

    /// The account a wallet is, by the rule of [`OwnWallet::named`]: a
    /// policy that is one key's own single-key script whose type is that
    /// path's purpose, `None` for any other. A lookup.
    #[must_use]
    pub fn account_of_wallet(p: &faraday_bip::policy::WalletPolicy) -> Option<Self> {
        if let faraday_bip::policy::Template::Single { script } = p.template()
            && let [k] = p.keys()
            && let (Some(key), Some(path)) = (k.fingerprint(), k.path())
            && let Some(ChildNumber::Hardened { index }) = path.as_ref().first().copied()
            && index
                == match script {
                    ScriptType::Legacy => 44,
                    ScriptType::NestedSegwit => 49,
                    ScriptType::NativeSegwit => 84,
                    ScriptType::Taproot => 86,
                }
        {
            return Some(OwnWallet::Account {
                key,
                path: path.clone(),
            });
        }
        None
    }
}

/// The account a single-key origin `…/chain/index` is under.
fn account_of(path: &DerivationPath) -> DerivationPath {
    let steps: &[ChildNumber] = path.as_ref();
    DerivationPath::from(&steps[..steps.len().saturating_sub(2)])
}

/// One output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputInfo {
    /// Position in the transaction.
    pub index: usize,
    /// The address for the context's network, empty where the script has
    /// no address form (the app names it by its script).
    pub address: String,
    /// The script itself.
    pub script_pubkey: ScriptBuf,
    /// Value.
    pub amount: Amount,
    /// Recipient, verified change, or unverified claim.
    pub kind: OutputKind,
    /// The wallet of the person's the output is verified as an address of,
    /// named the way the inputs name theirs (a registered wallet that is
    /// one key's own single-key account is that account): set for
    /// [`OutputKind::Change`], [`OutputKind::WalletChange`] and
    /// [`OutputKind::Transfer`], `None` for any other.
    pub own: Option<OwnWallet>,
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
    /// Sum of known input values; not their sum where an
    /// `AmountOutOfRange` block says they are over 21,000,000 BTC.
    pub total_in: Amount,
    /// Sum of outputs; not their sum where an `AmountOutOfRange` block
    /// says they are over 21,000,000 BTC.
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

/// A transaction whose only output is an `OP_RETURN` of 0: the shape of
/// BIP-322's `to_sign`, simple or in the proof-of-funds form. BIP-322
/// requires exactly one 0-value `OP_RETURN` output, and with
/// `ANYONECANPAY` a signature commits to neither the other inputs nor its
/// own index, so a `to_spend` can be put at input 0 afterwards; the inputs
/// are therefore not looked at. The amount input 0 states is not checked:
/// SegWit v0 and legacy signatures do not commit to it. A burn of a coin
/// to a 0 `OP_RETURN` is refused with it.
fn looks_like_message_request(psbt: &Psbt) -> bool {
    let tx = &psbt.inner().unsigned_tx;
    tx.output.len() == 1
        && tx.output[0].script_pubkey.is_op_return()
        && tx.output[0].value == Amount::ZERO
}

fn warn(warnings: &mut Vec<Warning>, level: Level, kind: WarningKind, detail: Detail) {
    warnings.push(Warning {
        level,
        kind,
        detail,
    });
}

/// Costs a script derivation for each wallet an input or output claims,
/// and the checks: inspects `psbt` for the keys and network in `ctx`.
pub fn inspect(c: &Compute, psbt: &Psbt, ctx: &Context) -> Inspection {
    c.count(Work::PsbtInspect);
    let secp = Secp256k1::verification_only();
    let inner = psbt.inner();
    let tx = &inner.unsigned_tx;
    let w = Walk {
        c,
        psbt,
        ctx,
        secp: &secp,
        txid: tx.compute_txid(),
    };

    let Inputs {
        mut warnings,
        inputs,
        participating,
        multisig,
        musig,
        kinds,
        input_scripts,
        unregistered,
        multisig_ours,
        spent,
        total_in,
        fee_known,
        mut weight,
        ..
    } = read_inputs(&w);
    let Outputs {
        outputs,
        total_out,
        amount_to_others,
        change_total,
    } = read_outputs(&w, &spent, &mut warnings, &mut weight);

    checks::address_reuse(&outputs, &input_scripts, &mut warnings);
    checks::message_request(psbt, &mut warnings);
    let fee_known = checks::money_range(&inputs, tx, fee_known, &mut warnings);
    let Fee {
        fee,
        fee_rate_sat_vb,
        vsize_estimate,
    } = checks::fee_checks(
        &Totals {
            total_in,
            total_out,
            amount_to_others,
            weight,
        },
        fee_known,
        &mut warnings,
    );

    network_checks(inner, ctx, &mut warnings);
    script_mix(kinds, &mut warnings);

    // Locktime and RBF.
    let locktime = locktime(tx);
    // A height lock is no warning: wallets set the current height on
    // nearly every spend, so it is a plain row on Review. A lock is
    // stated only when it holds.
    if locktime.holds && matches!(locktime.kind, LocktimeKind::Time(_)) {
        warn(
            &mut warnings,
            Level::Caution,
            WarningKind::LocktimeInFuture,
            Detail::Locktime(locktime),
        );
    }
    for inp in &inputs {
        if let Some(lock) = inp.relative_lock {
            warn(
                &mut warnings,
                Level::Caution,
                WarningKind::RelativeLock,
                Detail::RelativeLock {
                    input: inp.index,
                    lock,
                },
            );
        }
    }
    // Version 3 (BIP 431) is replaceable whatever its sequences say.
    let rbf = tx.version.0 == 3 || tx.input.iter().any(|i| i.sequence.is_rbf());

    if participating.is_empty() {
        warn(
            &mut warnings,
            Level::Danger,
            WarningKind::NoParticipatingKey,
            Detail::NoParticipatingKey,
        );
    }

    let is_self_transfer = !outputs.is_empty() && outputs.iter().all(OutputInfo::is_ours);

    let Signed {
        signatures,
        nonce_reuse,
        inscriptions,
    } = checks::signature_checks(psbt, ctx, &inputs, &mut warnings);

    let unread = unread_fields(inner);
    if unread > 0 {
        warn(
            &mut warnings,
            Level::Info,
            WarningKind::UnknownFields { count: unread },
            Detail::UnknownFields { count: unread },
        );
    }

    let register = register_offer(c, psbt, &warnings, &unregistered, &multisig_ours);

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

/// An info card when the inputs are of more than one script type.
fn script_mix(kinds: Vec<ScriptKind>, warnings: &mut Vec<Warning>) {
    if kinds.len() > 1 {
        warn(
            warnings,
            Level::Info,
            WarningKind::MixedScriptTypes,
            Detail::MixedScripts { kinds },
        );
    }
}

/// Costs what `register::offer` costs. The wallet the PSBT defines,
/// offered where nothing else blocks. A key under a loaded fingerprint
/// that is not its key blocks (F136) and is still offered: registering
/// refuses that key by name.
fn register_offer(
    c: &Compute,
    psbt: &Psbt,
    warnings: &[Warning],
    unregistered: &[usize],
    multisig_ours: &alloc::collections::BTreeMap<usize, Vec<bool>>,
) -> Option<crate::register::Registration> {
    warnings
        .iter()
        .filter(|w| w.level == Level::Blocked)
        .all(|w| {
            matches!(
                w.kind,
                WarningKind::WalletNotRegistered | WarningKind::UnknownDerivation
            )
        })
        .then(|| crate::register::offer(c, psbt, unregistered, multisig_ours))
        .flatten()
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
    let path = path.clone();
    let (level, detail) = if same_key {
        (
            Level::Caution,
            Detail::KeyInOtherScript {
                input: i,
                key: fp,
                path,
            },
        )
    } else {
        (
            Level::Blocked,
            Detail::KeyNotInput {
                input: i,
                key: fp,
                path,
            },
        )
    };
    warn(warnings, level, WarningKind::UnknownDerivation, detail);
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
) -> (OutputKind, Option<OwnWallet>) {
    let mut claim_warnings = Vec::new();
    let (kind, own) = output_claims(c, secp, out, txout, ctx, i, &mut claim_warnings);
    let verified = matches!(
        kind,
        OutputKind::Change { .. } | OutputKind::WalletChange { .. }
    );
    if verified {
        claim_warnings.retain(|w| w.level >= Level::Danger);
    }
    warnings.append(&mut claim_warnings);
    match kind {
        OutputKind::Change { index, .. } | OutputKind::WalletChange { index, .. } => {
            far_change(warnings, i, index);
        }
        _ => {}
    }
    (kind, own)
}

/// Change of `own`, one of the person's wallets, is their change only
/// when every input of theirs spends that wallet (`spent`, F254); where
/// they spend another wallet as well or instead, it is a transfer into
/// `own`, counted as sent, with a Caution. Where no input is the
/// person's, nothing here is signed and the kind stays as it is.
fn change_or_transfer(
    kind: OutputKind,
    own: Option<OwnWallet>,
    spent: &[OwnWallet],
    ctx: &Context,
    i: usize,
    warnings: &mut Vec<Warning>,
) -> OutputKind {
    let Some(to) = own else { return kind };
    let named = to.clone().named(ctx);
    if spent.iter().all(|w| *w == named) {
        return kind;
    }
    let (change, index) = match kind {
        OutputKind::Change { change, index, .. }
        | OutputKind::WalletChange { change, index, .. } => (change, index),
        other => return other,
    };
    warn(
        warnings,
        Level::Caution,
        WarningKind::TransferToOwnWallet,
        Detail::TransferToOwnWallet { output: i },
    );
    OutputKind::Transfer { to, change, index }
}

/// Costs what `musig_output` and `wallet_claim` cost. The body of
/// [`output_kind`], with every warning its claims raise, and the
/// person's wallet the output is verified as an address of.
fn output_claims<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    out: &Output,
    txout: &TxOut,
    ctx: &Context,
    i: usize,
    warnings: &mut Vec<Warning>,
) -> (OutputKind, Option<OwnWallet>) {
    let spk = &txout.script_pubkey;
    match musig_output(c, secp, out, txout, ctx) {
        Some(Ok((wallet, change, index))) => {
            return (
                OutputKind::WalletChange {
                    wallet,
                    change,
                    index,
                },
                Some(OwnWallet::Registered(wallet)),
            );
        }
        Some(Err(())) => {
            warn(
                warnings,
                Level::Blocked,
                WarningKind::ChangeSpoof,
                Detail::MusigChangeSpoof { output: i },
            );
            let claimed = out
                .tap_key_origins
                .values()
                .map(|(_, (fp, _))| Fingerprint::from(*fp))
                .find(|fp| ctx.is_ours(*fp));
            let kind = match claimed {
                Some(key) => OutputKind::UnverifiedChange { key },
                None => OutputKind::Recipient,
            };
            return (kind, None);
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
    let Some(claim) = ctx.wallet_claim(c, origins, spk) else {
        return single_key_claims(secp, out, spk, ctx, i, warnings);
    };
    if claim.verified {
        return (
            OutputKind::WalletChange {
                wallet: claim.wallet,
                change: claim.change,
                index: claim.index,
            },
            Some(OwnWallet::Registered(claim.wallet)),
        );
    }
    // A wallet of several keys over a single-key account names that
    // account's paths too: the account's own change at one of them is
    // still the person's change, and only an output neither gives is a
    // spoof of the wallet's.
    let mut single = Vec::new();
    let answer = single_key_claims(secp, out, spk, ctx, i, &mut single);
    if matches!(answer.0, OutputKind::Change { .. }) {
        warnings.append(&mut single);
        return answer;
    }
    warn(
        warnings,
        Level::Blocked,
        WarningKind::ChangeSpoof,
        Detail::WalletChangeSpoof { output: i },
    );
    (
        OutputKind::UnverifiedChange {
            key: claim.fingerprint,
        },
        None,
    )
}

/// An output's claims to be a loaded key's single-key change, with
/// every warning they raise, and the account it is verified under.
fn single_key_claims<C: Verification>(
    secp: &Secp256k1<C>,
    out: &Output,
    spk: &bitcoin::Script,
    ctx: &Context,
    i: usize,
    warnings: &mut Vec<Warning>,
) -> (OutputKind, Option<OwnWallet>) {
    let mut kind = OutputKind::Recipient;
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
                Detail::ChangeKeyNotLoaded { output: i, key: fp },
            );
            continue;
        };
        match key.derive(secp, path) {
            Some(d)
                if !tainted
                    && d.key.x_only_public_key().0 == pk.x_only_public_key().0
                    && d.script_pubkey == *spk =>
            {
                let kind = OutputKind::Change {
                    key: fp,
                    change: d.change,
                    index: d.index,
                };
                let own = OwnWallet::Account {
                    key: fp,
                    path: account_of(path),
                };
                return (kind, Some(own));
            }
            Some(_) => {
                kind = OutputKind::UnverifiedChange { key: fp };
                warn(
                    warnings,
                    Level::Blocked,
                    WarningKind::ChangeSpoof,
                    Detail::ChangeSpoof {
                        output: i,
                        key: fp,
                        path: path.clone(),
                    },
                );
            }
            None => {
                kind = OutputKind::UnverifiedChange { key: fp };
                let detail = if path
                    .into_iter()
                    .next()
                    .is_some_and(|c| *c == ChildNumber::Hardened { index: 48 })
                {
                    Detail::ChangeNotRegistered {
                        output: i,
                        key: fp,
                        path: path.clone(),
                    }
                } else {
                    Detail::ChangeNotUnderAccount {
                        output: i,
                        key: fp,
                        path: path.clone(),
                    }
                };
                warn(
                    warnings,
                    Level::Caution,
                    WarningKind::UnverifiedChange,
                    detail,
                );
            }
        }
    }
    (kind, None)
}

/// Raises F242's Caution for a change output at `index` where it is
/// [`FAR_CHANGE_INDEX`] or more.
fn far_change(warnings: &mut Vec<Warning>, output: usize, index: u32) {
    if index >= FAR_CHANGE_INDEX {
        warn(
            warnings,
            Level::Caution,
            WarningKind::ChangeFarIndex,
            Detail::ChangeFarIndex { output, index },
        );
    }
}

fn locktime(tx: &Transaction) -> Locktime {
    let lt = tx.lock_time;
    let holds = tx.input.iter().any(|i| i.sequence != Sequence::MAX);
    let raw = lt.to_consensus_u32();
    let kind = match lt {
        LockTime::Blocks(h) if h.to_consensus_u32() == 0 => LocktimeKind::None,
        LockTime::Blocks(h) => LocktimeKind::Height(h.to_consensus_u32()),
        LockTime::Seconds(t) => LocktimeKind::Time(t.to_consensus_u32()),
    };
    let date = match kind {
        LocktimeKind::Time(t) => Some(lock_date(t)),
        _ => None,
    };
    Locktime {
        raw,
        kind,
        date,
        holds,
    }
}

/// A Unix time as a UTC date, by the civil-from-days rule.
fn lock_date(t: u32) -> LockDate {
    let days = i64::from(t / 86_400) + 719_468;
    let era = days / 146_097;
    let doe = days % 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let secs = t % 86_400;
    LockDate {
        year: year as u16,
        month: month as u8,
        day: day as u8,
        hour: (secs / 3_600) as u8,
        minute: (secs % 3_600 / 60) as u8,
    }
}

/// The relative wait `txin`'s sequence sets in `tx` (BIP 68): none at
/// version 1, with the disable flag set, or with a count of zero.
pub(crate) fn relative_lock(tx: &Transaction, txin: &bitcoin::TxIn) -> Option<RelativeLock> {
    let seq = txin.sequence.to_consensus_u32();
    let count = (seq & 0xffff) as u16;
    if tx.version.0.cast_unsigned() < 2 || seq & (1 << 31) != 0 || count == 0 {
        return None;
    }
    Some(if seq & (1 << 22) != 0 {
        RelativeLock::Intervals(count)
    } else {
        RelativeLock::Blocks(count)
    })
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
                Detail::KeyNetwork {
                    key: key.fingerprint,
                    loaded: acct.network(),
                    checked: ctx.network,
                },
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
            Detail::XpubNetwork {
                checked: ctx.network,
            },
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
                .map(move |(_, p)| (false, i, p))
                .chain(
                    input
                        .tap_key_origins
                        .values()
                        .map(move |(_, (_, p))| (false, i, p)),
                )
        })
        .chain(inner.outputs.iter().enumerate().flat_map(|(i, out)| {
            out.bip32_derivation
                .values()
                .map(move |(_, p)| (true, i, p))
                .chain(
                    out.tap_key_origins
                        .values()
                        .map(move |(_, (_, p))| (true, i, p)),
                )
        }));
    for (output, i, path) in paths {
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
                Detail::CoinType {
                    output,
                    index: i,
                    path: path.clone(),
                    coin: c,
                    checked: ctx.network,
                    expected: coin,
                },
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
    /// this pass and sign none of it: round 1.
    pub will_share_nonce: bool,
    /// At least one public nonce of this device's on the input belongs
    /// to no open session, so this pass replaces it.
    pub nonce_stale: bool,
}

/// Which of the three ways one participant of a MuSig2 input is
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
    musig_form_rank(musig, aggregate).is_some()
}

/// Which of the three aggregate forms an entry's key is, in the order a
/// reader prefers when one participant has entries under more than one: the
/// taproot output key (the form this device writes and Bitcoin Core
/// writes), the root aggregate, the internal key. `None` for any other key.
pub(crate) fn musig_form_rank(musig: &MusigInput, aggregate: &PublicKey) -> Option<u8> {
    if *aggregate == musig.output_key {
        Some(0)
    } else if *aggregate == musig.root_aggregate {
        Some(1)
    } else if aggregate.x_only_public_key().0 == musig.internal_key {
        Some(2)
    } else {
        None
    }
}

/// Costs `verified_partials` (a sighash, the aggregate nonce and session
/// once, and a signature check (BIP-327 `PartialSigVerify`) per partial
/// signature on the input) when the input carries a partial signature for
/// `participation` and this session does not hold its secret nonce, and
/// nothing otherwise. A caller asks once per held key. Which way
/// `participation` is handled on input `index` of the transaction `txid`,
/// decided in §16.100's order.
///
/// A partial signature in the file is the file's word: the online side
/// writes every nonce, so one that verifies can be built from public
/// data. It ends the participant's round only when this device has no
/// secret nonce for it and the entry verifies. A session that holds the
/// participant's secret nonce proves any entry for it false, and round 2
/// goes on with that nonce, which overwrites it. An entry that fails to
/// verify falls through to a new nonce or a deterministic signature.
pub(crate) fn musig_round(
    c: &Compute,
    psbt: &Psbt,
    musig: &MusigInput,
    participation: &MusigParticipation,
    session: Option<&crate::musig::MusigSessionView>,
    txid: bitcoin::Txid,
    index: usize,
) -> MusigRound {
    let input = &psbt.inner().inputs[index];
    let nonces = crate::musig::pub_nonces(input).unwrap_or_default();
    let here = |key: &PublicKey| {
        nonces
            .iter()
            .any(|n| n.participant == *key && musig_keyed_here(musig, &n.aggregate))
    };
    let held = session.is_some_and(|s| s.holds(txid, index, &participation.key));
    if !held {
        let has_entry = crate::musig::partial_sigs(input)
            .unwrap_or_default()
            .iter()
            .any(|e| e.participant == participation.key && musig_keyed_here(musig, &e.aggregate));
        if has_entry
            && crate::musig::verified_partials(c, psbt, index, musig).contains(&participation.key)
        {
            return MusigRound::Done;
        }
    }
    let ours = here(&participation.key);
    let others = musig
        .participants
        .iter()
        .filter(|p| **p != participation.key)
        .all(here);
    match (ours, held, others) {
        (true, true, true) => MusigRound::Sign,
        (true, true, false) => MusigRound::Wait,
        (false, _, true) => MusigRound::SignLast,
        (ours, _, _) => MusigRound::Share { stale: ours },
    }
}

/// Pairs in any `unknown` or `proprietary` map of the PSBT, less the
/// BIP-373 pairs `musig.rs` reads.
fn unread_fields(psbt: &bitcoin::Psbt) -> usize {
    use crate::musig::{
        IN_PARTIAL_SIG, IN_PARTICIPANT_PUBKEYS, IN_PUB_NONCE, OUT_PARTICIPANT_PUBKEYS,
    };
    let read_in = [IN_PARTICIPANT_PUBKEYS, IN_PUB_NONCE, IN_PARTIAL_SIG];
    let global = psbt.unknown.len() + psbt.proprietary.len();
    let inputs: usize = psbt
        .inputs
        .iter()
        .map(|i| {
            i.proprietary.len()
                + i.unknown
                    .keys()
                    .filter(|k| !read_in.contains(&k.type_value))
                    .count()
        })
        .sum();
    let outputs: usize = psbt
        .outputs
        .iter()
        .map(|o| {
            o.proprietary.len()
                + o.unknown
                    .keys()
                    .filter(|k| k.type_value != OUT_PARTICIPANT_PUBKEYS)
                    .count()
        })
        .sum();
    global + inputs + outputs
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
    reason: Reason,
}

fn musig_block(kind: WarningKind, reason: Reason) -> Result<Option<MusigInput>, MusigBlock> {
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
/// is, in this order: the registered wallet, the
/// aggregate, this device's participants, the sub-path, and the script.
/// `Ok(None)` is an input that is not a MuSig2 input at all.
///
/// A merkle root never reaches here: an input that states one is
/// classified [`ScriptKind::P2trScript`], which this crate refuses to
/// sign for a `musig()` wallet, since `tr(musig())` has no tree.
fn musig_input<C: Verification>(
    c: &Compute,
    secp: &Secp256k1<C>,
    ctx: &Context,
    input: &Input,
    spk: Option<&bitcoin::Script>,
) -> Result<Option<MusigInput>, MusigBlock> {
    let fields = match crate::musig::input_participants(input) {
        Ok(fields) => fields,
        Err(e) => return musig_block(WarningKind::UtxoMismatch, Reason::Musig(e)),
    };
    if fields.is_empty() {
        return Ok(None);
    }
    if crate::musig::pub_nonces(input).is_err() || crate::musig::partial_sigs(input).is_err() {
        let reason = crate::musig::pub_nonces(input)
            .err()
            .or_else(|| crate::musig::partial_sigs(input).err())
            .map_or(Reason::MusigMalformed, Reason::Musig);
        return musig_block(WarningKind::UtxoMismatch, reason);
    }

    // 1. The registered wallet: the participant set is what the
    //    aggregate is, so a set no registered wallet has is a different
    //    wallet and never one this device signs for.
    let Some((field, (wallet, participants))) = fields
        .iter()
        .find_map(|f| musig_wallet(ctx, &f.keys).map(|w| (f, w)))
    else {
        return musig_block(WarningKind::MusigWalletNotRegistered, Reason::MusigNoWallet);
    };

    // 2. The aggregate the field is keyed by.
    let Ok(root) = faraday_bip::musig::key_agg_with(secp, &participants) else {
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigNoAggregate);
    };
    if root.public_key() != field.aggregate {
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigNotFieldKey);
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
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigNoInternalKey);
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
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigNoChainIndex);
    };

    // 5. The internal key and the script the wallet pays to at that
    //    address.
    let steps = [
        ChildNumber::from_normal_idx(u32::from(change)).expect("0 or 1"),
        ChildNumber::from_normal_idx(index).expect("an unhardened index"),
    ];
    let Some((tweaks, derived)) = musig_tweaks(secp, aggregate_xpub, &steps) else {
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigNotDerived);
    };
    if derived.x_only_public_key().0 != internal_key {
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigInternalNotAggregate);
    }
    let mut output = root;
    for tweak in &tweaks {
        match output.apply_tweak(secp, &tweak.bytes, tweak.x_only) {
            Ok(next) => output = next,
            Err(_) => {
                return musig_block(WarningKind::UtxoMismatch, Reason::MusigNoTweak);
            }
        }
    }
    let wallet_script = policy.script_at_with(c, secp, change, index).ok();
    if wallet_script.as_deref() != spk {
        return musig_block(WarningKind::UtxoMismatch, Reason::MusigNotWalletAddress);
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
