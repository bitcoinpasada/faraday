//! The PSBT container and the crate error type.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use bitcoin::Transaction;
use osk_bip::keys::Fingerprint;

use crate::base64;
use crate::inspect::WarningKind;

/// The magic bytes every binary PSBT starts with.
const MAGIC: &[u8; 5] = b"psbt\xff";

/// A BIP-174 (version 0) partially signed Bitcoin transaction.
///
/// Wraps `bitcoin::Psbt`; the inner value is reachable for callers that
/// need fields this crate does not surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Psbt {
    inner: bitcoin::Psbt,
}

impl Psbt {
    /// Parses the binary encoding (`psbt\xff` magic first).
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
            return Err(Error::NotPsbt);
        }
        let inner = bitcoin::Psbt::deserialize(bytes).map_err(|e| Error::Parse(e.to_string()))?;
        if inner.version != 0 {
            return Err(Error::UnsupportedVersion(inner.version));
        }
        Ok(Self { inner })
    }

    /// Parses the base64 text encoding. Surrounding whitespace is ignored.
    pub fn parse_base64(text: &str) -> Result<Self, Error> {
        let bytes = base64::decode(text).map_err(|_| Error::Base64)?;
        Self::parse_bytes(&bytes)
    }

    /// The binary encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.inner.serialize()
    }

    /// The base64 text encoding.
    pub fn to_base64(&self) -> String {
        base64::encode(&self.to_bytes())
    }

    /// The unsigned transaction the PSBT is built around.
    pub fn unsigned_tx(&self) -> &Transaction {
        &self.inner.unsigned_tx
    }

    /// The underlying `bitcoin::Psbt`.
    pub fn inner(&self) -> &bitcoin::Psbt {
        &self.inner
    }

    /// Mutable access to the underlying `bitcoin::Psbt`.
    pub fn inner_mut(&mut self) -> &mut bitcoin::Psbt {
        &mut self.inner
    }

    /// Unwraps into the underlying `bitcoin::Psbt`.
    pub fn into_inner(self) -> bitcoin::Psbt {
        self.inner
    }
}

impl From<bitcoin::Psbt> for Psbt {
    fn from(inner: bitcoin::Psbt) -> Self {
        Self { inner }
    }
}

/// Why an operation on a PSBT failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The text is not valid base64.
    Base64,
    /// The bytes do not start with the PSBT magic.
    NotPsbt,
    /// `bitcoin` rejected the encoding; the text is its reason.
    Parse(String),
    /// The PSBT declares a version other than 0.
    UnsupportedVersion(u32),
    /// An input has neither `witness_utxo` nor `non_witness_utxo`.
    MissingUtxo {
        /// Input index.
        input: usize,
    },
    /// The input spends a script this crate cannot sign or finalize.
    Unsupported {
        /// Input index.
        input: usize,
        /// What was found.
        reason: &'static str,
    },
    /// The key derived at the PSBT's stated origin is not the key the PSBT
    /// names, or is not part of the script being spent.
    KeyMismatch {
        /// Input index.
        input: usize,
    },
    /// A selected fingerprint is not among the supplied master keys.
    NoSuchKey(Fingerprint),
    /// The inspection has danger-level warnings and `force` was not set.
    Danger(Vec<WarningKind>),
    /// The inspection has blocked-level warnings. No `force` signs these.
    Blocked(Vec<WarningKind>),
    /// The signature hash could not be computed (missing script or UTXO
    /// data, non-standard sighash type).
    Sighash {
        /// Input index.
        input: usize,
    },
    /// A freshly produced signature failed verification against its public
    /// key. Nothing was written to the PSBT.
    VerifyFailed {
        /// Input index.
        input: usize,
    },
    /// Signing the same message twice gave different bytes. Nothing was
    /// written to the PSBT.
    Nondeterministic {
        /// Input index.
        input: usize,
    },
    /// A participant's MuSig2 partial signature on an input does not
    /// verify, so the transaction cannot be finished until that
    /// participant signs again.
    MusigPartialSig {
        /// Input index.
        input: usize,
        /// The participant's public key, compressed, in hex.
        participant: String,
    },
    /// Finalization failed for an input.
    Finalize {
        /// Input index.
        input: usize,
        /// What was wrong.
        reason: &'static str,
    },
    /// Every input is final but the transaction could not be extracted;
    /// the text is `bitcoin`'s reason.
    Extract(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Base64 => f.write_str("invalid base64"),
            Error::NotPsbt => f.write_str("not a PSBT (missing magic bytes)"),
            Error::Parse(reason) => write!(f, "invalid PSBT: {reason}"),
            Error::UnsupportedVersion(v) => write!(f, "unsupported PSBT version {v}"),
            Error::MissingUtxo { input } => write!(f, "input {input}: no UTXO information"),
            Error::Unsupported { input, reason } => write!(f, "input {input}: {reason}"),
            Error::KeyMismatch { input } => {
                write!(
                    f,
                    "input {input}: derived key does not match the PSBT's key origin"
                )
            }
            Error::NoSuchKey(fp) => write!(f, "no loaded key with fingerprint {fp}"),
            Error::Danger(kinds) => {
                write!(
                    f,
                    "refused: {} danger warning(s) need explicit confirmation",
                    kinds.len()
                )
            }
            Error::Blocked(kinds) => {
                write!(f, "refused: {} blocking warning(s)", kinds.len())
            }
            Error::Sighash { input } => write!(f, "input {input}: cannot compute signature hash"),
            Error::VerifyFailed { input } => {
                write!(f, "input {input}: produced signature failed verification")
            }
            Error::Nondeterministic { input } => {
                write!(f, "input {input}: signing twice gave different signatures")
            }
            Error::MusigPartialSig { input, participant } => write!(
                f,
                "input {input}: the MuSig2 partial signature of {participant} does not verify"
            ),
            Error::Finalize { input, reason } => write!(f, "input {input}: {reason}"),
            Error::Extract(reason) => write!(f, "cannot extract transaction: {reason}"),
        }
    }
}

impl core::error::Error for Error {}
