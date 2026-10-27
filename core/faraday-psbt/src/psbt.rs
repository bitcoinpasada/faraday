//! The PSBT container and the crate error type.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use bitcoin::Transaction;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::{Compute, Work};

use crate::base64;
use crate::inspect::WarningKind;
use crate::v2::{self, Map};

/// The magic bytes every binary PSBT starts with.
const MAGIC: &[u8; 5] = b"psbt\xff";

/// The global fields BIP-370 defines for version 2 only, which a
/// version 0 PSBT must not carry: `PSBT_GLOBAL_TX_VERSION`,
/// `_FALLBACK_LOCKTIME`, `_INPUT_COUNT`, `_OUTPUT_COUNT` and
/// `_TX_MODIFIABLE`.
const GLOBAL_V2: &[u8] = &[0x02, 0x03, 0x04, 0x05, 0x06];
/// The input fields BIP-370 defines for version 2 only:
/// `PSBT_IN_PREVIOUS_TXID`, `_OUTPUT_INDEX`, `_SEQUENCE`,
/// `_REQUIRED_TIME_LOCKTIME` and `_REQUIRED_HEIGHT_LOCKTIME`.
const INPUT_V2: &[u8] = &[0x0e, 0x0f, 0x10, 0x11, 0x12];
/// The output fields BIP-370 defines for version 2 only:
/// `PSBT_OUT_AMOUNT` and `PSBT_OUT_SCRIPT`.
const OUTPUT_V2: &[u8] = &[0x03, 0x04];

/// A BIP-174 (version 0) partially signed Bitcoin transaction.
///
/// Wraps `bitcoin::Psbt`; the inner value is reachable for callers that
/// need fields this crate does not surface. A PSBT that was read keeps
/// its maps as they came, so [`Psbt::to_bytes`] writes every field
/// Faraday did not change exactly as it was read, where `bitcoin` alone
/// would write some otherwise (a `PSBT_OUT_TAP_TREE`'s leaves in another
/// order, a `PSBT_GLOBAL_VERSION` of 0 left out, a map's keys sorted).
/// Two PSBTs are equal when their contents are, however they were
/// written.
#[derive(Debug, Clone)]
pub struct Psbt {
    inner: bitcoin::Psbt,
    /// The maps as read, and as `bitcoin` wrote them just after reading;
    /// `None` for a PSBT made here rather than read.
    read: Option<Read>,
}

/// A read PSBT's maps: as they came (`raw`), and as `bitcoin` wrote the
/// PSBT back right after reading it (`canonical`). A field whose
/// `bitcoin` form is still the one in `canonical` was not changed, so
/// its bytes in `raw` are the ones written. Both describe the PSBT as
/// read and never change after.
#[derive(Debug, Clone)]
struct Read {
    raw: Vec<Map>,
    canonical: Vec<Map>,
}

impl PartialEq for Psbt {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Psbt {}

/// A PSBT's maps: the global map, then one per input and one per
/// output, with nothing after the last.
fn maps_of(bytes: &[u8], inputs: usize, outputs: usize) -> Result<Vec<Map>, Error> {
    let mut at = MAGIC.len();
    let mut maps = Vec::with_capacity(1 + inputs + outputs);
    for _ in 0..1 + inputs + outputs {
        maps.push(v2::read_map(bytes, &mut at).map_err(|e| Error::Parse(e.to_string()))?);
    }
    if at != bytes.len() {
        return Err(Error::Parse("bytes follow the last map".into()));
    }
    Ok(maps)
}

/// What BIP-370 and BIP-341 refuse in a version 0 PSBT that `bitcoin`
/// reads: a field of version 2 (BIP-370's "PSBTv0 but with" vectors),
/// and a taproot signature of 65 bytes whose sighash byte is 0, which
/// BIP-341 makes invalid (the default type is written as 64 bytes).
fn refuse_in_v0(maps: &[Map], inputs: usize) -> Result<(), Error> {
    for (i, map) in maps.iter().enumerate() {
        let (v2_only, what) = if i == 0 {
            (GLOBAL_V2, "global")
        } else if i <= inputs {
            (INPUT_V2, "input")
        } else {
            (OUTPUT_V2, "output")
        };
        for (key, value) in map.pairs() {
            let ty = key.first().copied().unwrap_or(0);
            if v2_only.contains(&ty) {
                return Err(Error::Parse(alloc::format!(
                    "{what} field {ty:#04x} is for version 2 PSBTs only"
                )));
            }
            if what == "input" && v2::is_zero_sighash_tap_sig(key, value) {
                return Err(Error::Parse(
                    "a taproot signature with a sighash byte of 0".into(),
                ));
            }
        }
    }
    Ok(())
}

/// A key origin of more than [`faraday_bip::keys::MAX_PATH_STEPS`]
/// steps, anywhere in the PSBT, is refused: signing derives a key along
/// it, and no wallet's path is that long.
fn refuse_long_paths(inner: &bitcoin::Psbt) -> Result<(), Error> {
    let long = |p: &bitcoin::bip32::DerivationPath| p.len() > faraday_bip::keys::MAX_PATH_STEPS;
    let any = inner.xpub.values().any(|(_, p)| long(p))
        || inner.inputs.iter().any(|i| {
            i.bip32_derivation.values().any(|(_, p)| long(p))
                || i.tap_key_origins.values().any(|(_, (_, p))| long(p))
        })
        || inner.outputs.iter().any(|o| {
            o.bip32_derivation.values().any(|(_, p)| long(p))
                || o.tap_key_origins.values().any(|(_, (_, p))| long(p))
        });
    if any {
        return Err(Error::Parse(alloc::format!(
            "a key origin of more than {} steps",
            faraday_bip::keys::MAX_PATH_STEPS
        )));
    }
    Ok(())
}

impl Psbt {
    /// Costs a decode of the whole PSBT, counted once its magic bytes
    /// match (bytes without them cost a comparison): parses the binary
    /// encoding (`psbt\xff` magic first).
    pub fn parse_bytes(c: &Compute, bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
            return Err(Error::NotPsbt);
        }
        c.count(Work::PsbtParse);
        let inner = bitcoin::Psbt::deserialize(bytes).map_err(|e| Error::Parse(e.to_string()))?;
        if inner.version != 0 {
            return Err(Error::UnsupportedVersion(inner.version));
        }
        let (inputs, outputs) = (inner.inputs.len(), inner.outputs.len());
        let raw = maps_of(bytes, inputs, outputs)?;
        refuse_in_v0(&raw, inputs)?;
        refuse_long_paths(&inner)?;
        let canonical = maps_of(&inner.serialize(), inputs, outputs)?;
        Ok(Self {
            inner,
            read: Some(Read { raw, canonical }),
        })
    }

    /// Costs a base64 decode and what [`Psbt::parse_bytes`] costs: parses
    /// the base64 text encoding. Surrounding whitespace is ignored.
    pub fn parse_base64(c: &Compute, text: &str) -> Result<Self, Error> {
        let bytes = base64::decode(text).map_err(|_| Error::Base64)?;
        Self::parse_bytes(c, &bytes)
    }

    /// The binary encoding. A field not changed since the PSBT was read
    /// is written exactly as it was read, in its place; a field changed
    /// or added is written as `bitcoin` writes it, a new one before the
    /// first read field that `bitcoin` writes after it.
    pub fn to_bytes(&self) -> Vec<u8> {
        let now = self.inner.serialize();
        let Some(read) = &self.read else {
            return now;
        };
        let Ok(now_maps) = maps_of(&now, self.inner.inputs.len(), self.inner.outputs.len()) else {
            return now;
        };
        if now_maps.len() != read.raw.len() {
            return now;
        }
        let mut out = MAGIC.to_vec();
        for ((raw, canonical), now) in read.raw.iter().zip(&read.canonical).zip(&now_maps) {
            merged(raw, canonical, now).write_to(&mut out);
        }
        out
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

/// One map as written: `raw`, the map as read, with each field that
/// `bitcoin` now writes otherwise than it did just after reading
/// (`canonical`) taken from `now`, the fields it no longer writes left
/// out, and the fields it writes that were not read put in, each before
/// the first read field that comes after it in `now`. A read field
/// `bitcoin` does not write at all (a `PSBT_GLOBAL_VERSION` of 0) stays
/// as read.
fn merged(raw: &Map, canonical: &Map, now: &Map) -> Map {
    let mut out: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    for (key, value) in raw.pairs() {
        match (canonical.get(key), now.get(key)) {
            (None, _) => out.push((key.to_vec(), value.to_vec())),
            (Some(was), Some(is)) if was == is => out.push((key.to_vec(), value.to_vec())),
            (Some(_), Some(is)) => out.push((key.to_vec(), is.to_vec())),
            (Some(_), None) => {}
        }
    }
    let now_pairs: Vec<(&[u8], &[u8])> = now.pairs().collect();
    for (i, (key, value)) in now_pairs.iter().enumerate() {
        if raw.get(key).is_some() || canonical.get(key) == Some(*value) {
            continue;
        }
        let before = now_pairs[i + 1..]
            .iter()
            .find_map(|(later, _)| out.iter().position(|(k, _)| k == later));
        let pair = (key.to_vec(), value.to_vec());
        match before {
            Some(at) => out.insert(at, pair),
            None => out.push(pair),
        }
    }
    Map::from_pairs(out)
}

impl From<bitcoin::Psbt> for Psbt {
    fn from(inner: bitcoin::Psbt) -> Self {
        Self { inner, read: None }
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
