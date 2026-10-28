//! Why an input is not signed or finished, or a MuSig2 input not
//! established: a kind, which the app words from its strings file. This
//! crate holds no wording.

/// Why an input cannot be signed, finished or taken for a MuSig2 input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The previous transaction the input carries is not the one it
    /// spends (`non_witness_utxo`).
    PrevTxNotSpent,
    /// The previous transaction has no output at the index spent.
    PrevTxNoOutput,
    /// The stated output (`witness_utxo`) and the previous transaction's
    /// disagree.
    UtxosDisagree,
    /// A stated output (`witness_utxo`) alone, for an input that is not
    /// SegWit.
    WitnessUtxoForLegacy,
    /// A P2SH input with no redeem script.
    NoRedeemScript,
    /// The redeem script is not the one the output pays to.
    RedeemScriptMismatch,
    /// A P2WSH input with no witness script.
    NoWitnessScript,
    /// The witness script is not the one the witness program commits to.
    WitnessScriptMismatch,
    /// A SegWit v0 input with no previous transaction, whose amount
    /// would be the coordinator's word alone.
    NoPrevTx,
    /// No leaf and no key path of a taproot input is one of this
    /// device's keys.
    NoTaprootPathOurs,
    /// A taproot input names no leaf script and no internal key of its
    /// tree.
    NoTaprootLeafOrKey,
    /// A script this crate does not sign or finish.
    UnknownScript,
    /// A taproot input states no internal key.
    NoInternalKey,
    /// A redeem script too long to push.
    RedeemScriptTooLong,
    /// A MuSig2 field that could not be read.
    Musig(crate::musig::Error),
    /// A MuSig2 field malformed in a way no single field names.
    MusigMalformed,
    /// A MuSig2 input asks for a sighash other than `DEFAULT`.
    MusigSighashDefault,
    /// No MuSig2 nonce could be drawn for the input.
    MusigNonceNotDrawn,
    /// Another participant's MuSig2 nonce is missing.
    MusigNonceMissing,
    /// The other participants' MuSig2 nonces do not aggregate.
    MusigOtherNonces,
    /// The MuSig2 nonces on the input do not aggregate.
    MusigNonces,
    /// No registered MuSig2 wallet has the input's participants.
    MusigNoWallet,
    /// The MuSig2 participants do not aggregate.
    MusigNoAggregate,
    /// The MuSig2 participants are not the key their field names.
    MusigNotFieldKey,
    /// The MuSig2 input states no internal key.
    MusigNoInternalKey,
    /// The MuSig2 input states no chain and index below the aggregate.
    MusigNoChainIndex,
    /// The aggregate key does not derive at the stated chain and index.
    MusigNotDerived,
    /// The internal key is not the aggregate at the stated chain and
    /// index.
    MusigInternalNotAggregate,
    /// The aggregate key does not tweak to the output key.
    MusigNoTweak,
    /// The input does not pay to the registered wallet's address.
    MusigNotWalletAddress,
}
