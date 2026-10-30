//! What a warning states, with what it states it of: a kind and its
//! data, which the app words from its strings file. This crate holds no
//! wording.

use alloc::string::String;
use alloc::vec::Vec;

use bitcoin::bip32::DerivationPath;
use faraday_bip::keys::{Fingerprint, Network};
use faraday_bip::spend::Lock;

use super::{Locktime, SighashKind};
use crate::classify::ScriptKind;
use crate::reason::Reason;
use crate::verify::Unchecked;

/// What one warning states. `input` and `output` are indexes in the
/// transaction; `key` and `path` a key origin the PSBT states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detail {
    /// An input's data contradicts itself, or a MuSig2 input could not be
    /// taken as one.
    InputFault {
        /// The input.
        input: usize,
        /// What is wrong.
        reason: Reason,
    },
    /// A SegWit v0 input carries no previous transaction, so its amount
    /// cannot be checked.
    AmountUnverified {
        /// The input.
        input: usize,
    },
    /// An input has no UTXO data; its value and the fee cannot be
    /// verified.
    MissingUtxo {
        /// The input.
        input: usize,
    },
    /// An input cannot be signed here.
    CannotSign {
        /// The input.
        input: usize,
        /// Why.
        reason: Reason,
    },
    /// An input names a loaded key at a path under no loaded account.
    KeyNotUnderAccount {
        /// The input.
        input: usize,
        /// The key's fingerprint.
        key: Fingerprint,
        /// The path stated.
        path: DerivationPath,
    },
    /// An input names a loaded key in a script other than its account's.
    KeyInOtherScript {
        /// The input.
        input: usize,
        /// The key's fingerprint.
        key: Fingerprint,
        /// The path stated.
        path: DerivationPath,
    },
    /// An input names a loaded key whose derived key is not the input's.
    KeyNotInput {
        /// The input.
        input: usize,
        /// The key's fingerprint.
        key: Fingerprint,
        /// The path stated.
        path: DerivationPath,
    },
    /// A multisig input of another shape than the inputs before it.
    MultisigShapes {
        /// The input.
        input: usize,
        /// Its threshold and keys.
        m: usize,
        /// Its keys.
        n: usize,
        /// The first inputs' threshold.
        first_m: usize,
        /// The first inputs' keys.
        first_n: usize,
    },
    /// A MuSig2 nonce of this device's no open session holds is replaced.
    MusigNonceReplaced {
        /// The input.
        input: usize,
    },
    /// An input of several keys spends from a wallet not registered here.
    WalletNotRegistered {
        /// The input.
        input: usize,
    },
    /// An input is left unsigned: no registered wallet covers its script.
    NoWalletForScript {
        /// The input.
        input: usize,
        /// A taproot tree, not a `wsh` miniscript.
        taproot: bool,
    },
    /// An input's path waits longer than its sequence and locktime allow.
    TimelockNotMet {
        /// The input.
        input: usize,
        /// The wait.
        lock: Lock,
    },
    /// An output is a data (`OP_RETURN`) output.
    DataOutput {
        /// The output.
        output: usize,
    },
    /// An output pays a script with no address form.
    NonStandardOutput {
        /// The output.
        output: usize,
    },
    /// An output is below the dust limit.
    Dust {
        /// The output.
        output: usize,
        /// Its amount in sat.
        sat: u64,
    },
    /// Two outputs pay the same address.
    SameAddress {
        /// The first.
        first: usize,
        /// The second.
        second: usize,
    },
    /// An output pays back to an input's address.
    PaysToInput {
        /// The output.
        output: usize,
        /// The input.
        input: usize,
    },
    /// The outputs exceed the inputs: the UTXO data is wrong.
    OutputsExceedInputs,
    /// An output, or the outputs' total, is over 21,000,000 BTC.
    OutputsOverMaxMoney,
    /// An amount an input spends, or their total, is over 21,000,000 BTC.
    InputsOverMaxMoney,
    /// The fee is a large share of what is sent.
    FeeShare {
        /// The fee in sat.
        fee: u64,
        /// Its share of what is sent, in whole percent.
        pct: u64,
    },
    /// The fee rate is far above any network's.
    FeeRate {
        /// The rate in sat/vB, whole.
        sat_vb: u64,
    },
    /// The inputs spend more than one kind of script.
    MixedScripts {
        /// The kinds, in the order first met.
        kinds: Vec<ScriptKind>,
    },
    /// A locktime is set; this signer has no clock to check it.
    Locktime(Locktime),
    /// None of the loaded keys can sign the transaction.
    NoParticipatingKey,
    /// Two signatures under one key share a nonce.
    NonceReuse {
        /// The key, named as the review names one.
        key: String,
    },
    /// An input's script carries an inscription envelope.
    Inscription {
        /// The input.
        input: usize,
    },
    /// An output claims to be change of a MuSig2 wallet in use, and is not
    /// its address.
    MusigChangeSpoof {
        /// The output.
        output: usize,
    },
    /// An output claims to be change of a wallet in use, and is not its
    /// address.
    WalletChangeSpoof {
        /// The output.
        output: usize,
    },
    /// An output claims to belong to a key that is not loaded.
    ChangeKeyNotLoaded {
        /// The output.
        output: usize,
        /// The key's fingerprint.
        key: Fingerprint,
    },
    /// An output claims to be change of a loaded key, and is not that
    /// address.
    ChangeSpoof {
        /// The output.
        output: usize,
        /// The key's fingerprint.
        key: Fingerprint,
        /// The path stated.
        path: DerivationPath,
    },
    /// An output claims to be change of a loaded key at a multisig path
    /// no registered wallet holds.
    ChangeNotRegistered {
        /// The output.
        output: usize,
        /// The key's fingerprint.
        key: Fingerprint,
        /// The path stated.
        path: DerivationPath,
    },
    /// An output claims to be change of a loaded key at a path under no
    /// loaded account.
    ChangeNotUnderAccount {
        /// The output.
        output: usize,
        /// The key's fingerprint.
        key: Fingerprint,
        /// The path stated.
        path: DerivationPath,
    },
    /// A key is loaded for another network than the one checked for.
    KeyNetwork {
        /// The key's fingerprint.
        key: Fingerprint,
        /// The network it is loaded for.
        loaded: Network,
        /// The network the transaction is checked for.
        checked: Network,
    },
    /// A global xpub is for another network than the one checked for.
    XpubNetwork {
        /// The network checked for.
        checked: Network,
    },
    /// A key path is for another coin type than the network's.
    CoinType {
        /// An output's path, not an input's.
        output: bool,
        /// The input or output.
        index: usize,
        /// The path.
        path: DerivationPath,
        /// Its coin type.
        coin: u32,
        /// The network checked for.
        checked: Network,
        /// That network's coin type.
        expected: u32,
    },
    /// An input signs with a `NONE` sighash: no output is committed to.
    SighashNone {
        /// The input.
        input: usize,
        /// The sighash.
        sighash: SighashKind,
    },
    /// An input asks for a sighash no algorithm accepts.
    SighashNonStandard {
        /// The input.
        input: usize,
        /// The sighash.
        sighash: SighashKind,
    },
    /// An input asks for `SINGLE` with no output of its index.
    SingleNoOutput {
        /// The input.
        input: usize,
    },
    /// An input signs with `SINGLE`: only its own output is committed to.
    Single {
        /// The input.
        input: usize,
    },
    /// An input signs with a sighash that leaves part of the transaction
    /// out.
    SighashPartial {
        /// The input.
        input: usize,
        /// The sighash.
        sighash: SighashKind,
    },
    /// A signature on the transaction is not a signature of it.
    SignatureInvalid {
        /// The input.
        input: usize,
        /// The key, named as the review names one.
        key: String,
    },
    /// A signature could not be checked.
    SignatureUnchecked {
        /// The input.
        input: usize,
        /// Why.
        reason: Unchecked,
    },
    /// A signature signs with a `NONE` sighash, not the one asked for.
    SignedNone {
        /// The input.
        input: usize,
        /// The key, named as the review names one.
        key: String,
        /// The sighash it signs with.
        signed: SighashKind,
        /// The sighash asked for.
        asked: SighashKind,
    },
    /// A signature signs with a stricter sighash than the one asked for.
    SignedStricter {
        /// The input.
        input: usize,
        /// The key, named as the review names one.
        key: String,
        /// The sighash it signs with.
        signed: SighashKind,
        /// The sighash asked for.
        asked: SighashKind,
    },
    /// A signature signs with another sighash than the one asked for.
    SignedOther {
        /// The input.
        input: usize,
        /// The key, named as the review names one.
        key: String,
        /// The sighash it signs with.
        signed: SighashKind,
        /// The sighash asked for.
        asked: SighashKind,
    },
}
