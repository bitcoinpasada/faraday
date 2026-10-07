//! PSBT (BIP-174) parsing, inspection, policy warnings, signing and
//! finalization for OpenSignerKit.
//!
//! The crate is the only place in the core where a private key is used.
//! Everything else works from non-secret data:
//!
//! - [`Psbt`]: parse and serialise (binary and base64).
//! - [`inspect`]: a complete structured view for the Sign screens
//!   (`docs/UX.md` §7.3): amounts, fee, recipients, verified change,
//!   participating keys, multisig quorum, ranked warnings. Needs only the
//!   loaded keys' fingerprints and account xpubs ([`Context`], [`KeyRef`]).
//! - [`sign`]: derives each input's key from the PSBT's own key-origin
//!   data (never a guessed path), signs with the deterministic nonce
//!   [`Nonce`] names, verifies every signature before writing it, and
//!   checks determinism by signing twice (`docs/PLANNING.md` §8.4 #7,
//!   #8, #10).
//! - [`message`]: signs and checks a signed message, BIP-137 or BIP-322
//!   "simple", which is the one flow here that has no PSBT in it at all.
//! - [`finalize`]: builds the final scriptSig/witness for the supported
//!   script types and extracts the network transaction when every input is
//!   complete.
//! - [`verify`]: checks the signatures a transaction already carries
//!   against their keys, looks for a nonce used twice, says which nonce
//!   rule made a signature under a key this device holds, diffs two
//!   PSBTs and states an inscription envelope (§16.111). Nothing there
//!   signs.
//! - [`transaction`]: reads a raw transaction, or a PSBT as the
//!   transaction it carries, with the previous outputs its inputs spend,
//!   for a check that is not a signature.
//!
//! Supported spends: BIP-44 p2pkh, BIP-49 p2sh-p2wpkh, BIP-84 p2wpkh,
//! BIP-86 p2tr key path, `multi`/`sortedmulti` inside p2sh, p2wsh or
//! p2sh-p2wsh, a `wsh`/`sh(wsh)` miniscript, and a taproot script path.
//! Any other script is inspected but refused for signing.
//!
//! A miniscript or taproot-tree input is signed the way a hardware
//! signer signs one: every signature this device's keys can give, and
//! then whoever holds the rest finishes it. [`finalize`] completes such
//! an input only when the PSBT satisfies its script, so a timelock that
//! is not yet spendable or a cosigner who has not signed leaves the
//! transaction partial.
//!
//! A SegWit v0 input of a transaction with more than one input must
//! carry the transaction it spends (`non_witness_utxo`). BIP-143 signs
//! the amount the PSBT states, so without that transaction the amount is
//! the coordinator's word, and two signing rounds over two different lies
//! pay the difference to the miner. Such an input is a [`Level::Blocked`]
//! warning and is never signed, with or without `force`. A transaction
//! with one input is signed from its `witness_utxo` alone: the attack
//! needs two signatures the chain accepts, and a lone input signed over a
//! lie gives a signature no node accepts and nothing to combine it with.
//!
//! # Secrets
//!
//! [`sign`] takes `&MasterKey` and derives a [`osk_bip::keys::DerivedKey`]
//! per input, which is erased when it goes out of scope at the end of that
//! input. `bitcoin` forces two more copies: `secp256k1::Keypair` for
//! Schnorr signing (the untweaked pair and the BIP-341-tweaked pair); both
//! are erased with `non_secure_erase` before the input is done. Nothing
//! else in the crate holds private material, and no secret is written to
//! the heap by this crate; `Xpriv` derivation intermediates inside
//! `bitcoin` are stack copies outside our reach (`osk_bip::keys`).

#![no_std]

extern crate alloc;

/// Standard base64, which this crate reads PSBTs in and which lives
/// in [`osk_bip`] because the BSMS records there carry base64 too.
pub use osk_bip::base64;

mod classify;
mod context;
mod finalize;
mod inspect;
pub mod message;
pub mod musig;
mod psbt;
mod sign;
pub mod threshold;
pub mod transaction;
pub mod verify;

pub use classify::{Multisig, ScriptKind, Wrapper};
pub use context::{Context, KeyRef, ShareRef};
pub use finalize::{finalize, is_complete};
pub use inspect::{
    ABSURD_FEE_RATE_SAT_VB, Cosigner, HIGH_FEE_CAUTION_PCT, HIGH_FEE_DANGER_ABS,
    HIGH_FEE_DANGER_PCT, InputInfo, Inspection, KeyOrigin, Level, Locktime, LocktimeKind,
    MultisigSummary, MusigInput, MusigParticipation, MusigRound, MusigSummary, OutputInfo,
    OutputKind, SighashKind, SpendRoute, ThresholdInput, ThresholdRound, ThresholdSummary, Warning,
    WarningKind, inspect,
};
pub use musig::{MusigSession, MusigSessionView};
pub use psbt::{Error, Psbt};
pub use sign::{
    Aux, InputSignature, MusigRole, Nonce, Schnorr, ShareDoor, ShareKey, SigKind, SignResult,
    ThresholdOutcome, sign,
};

/// The rust-bitcoin version this crate is built against, re-exported so
/// callers use the same types without a second dependency.
pub use bitcoin;
