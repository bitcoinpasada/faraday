//! A BIP-370 (version 2) PSBT, held as its key-value maps, which is the
//! form BIP-375 silent payments arrive in.
//!
//! `bitcoin` reads version 0 only, so this module reads the maps
//! itself and keeps every pair as it came: [`PsbtV2::to_bytes`] writes
//! back exactly what was read plus what was set. Two bridges let the
//! signer the rest of the crate is built around do its work on one:
//! [`PsbtV2::to_v0`] writes the version 0 PSBT of the same transaction,
//! once every output has its script, and [`PsbtV2::merge_signed`] takes
//! the signatures a signing pass added there back into this one.
//! [`PsbtV2::from_v0`] goes the other way: the version 2 PSBT of a
//! version 0 one, which [`PsbtV2::to_v0`] turns back into the same
//! bytes.
//!
//! What it checks on reading is BIP-370's shape: the version is 2, the
//! counts are there and match the maps, no `PSBT_GLOBAL_UNSIGNED_TX`,
//! every input names its previous output and every output its amount
//! and its script (BIP-375 lets a silent payment output, which carries
//! `PSBT_OUT_SP_V0_INFO`, leave the script out), the fallback locktime,
//! the modifiable flags, the sequence and the required locktimes of
//! their sizes, a required time locktime from 500 000 000 and a
//! required height locktime from 1 to 499 999 999, no taproot signature
//! with a sighash byte of 0 (BIP-341), no key twice in a map, nothing
//! after the last map. What a silent payment needs on top is
//! [`crate::silent`]'s.

use alloc::vec::Vec;
use core::fmt;

use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness};
use faraday_crypto::work::{Compute, Work};

use crate::base64;

/// The magic bytes every binary PSBT starts with.
const MAGIC: &[u8; 5] = b"psbt\xff";

/// `PSBT_GLOBAL_UNSIGNED_TX`, which a version 2 PSBT must not carry.
pub const GLOBAL_UNSIGNED_TX: u8 = 0x00;
/// `PSBT_GLOBAL_TX_VERSION`.
pub const GLOBAL_TX_VERSION: u8 = 0x02;
/// `PSBT_GLOBAL_FALLBACK_LOCKTIME`.
pub const GLOBAL_FALLBACK_LOCKTIME: u8 = 0x03;
/// `PSBT_GLOBAL_INPUT_COUNT`.
pub const GLOBAL_INPUT_COUNT: u8 = 0x04;
/// `PSBT_GLOBAL_OUTPUT_COUNT`.
pub const GLOBAL_OUTPUT_COUNT: u8 = 0x05;
/// `PSBT_GLOBAL_TX_MODIFIABLE`.
pub const GLOBAL_TX_MODIFIABLE: u8 = 0x06;
/// `PSBT_GLOBAL_VERSION`.
pub const GLOBAL_VERSION: u8 = 0xfb;

/// `PSBT_IN_NON_WITNESS_UTXO`.
pub const IN_NON_WITNESS_UTXO: u8 = 0x00;
/// `PSBT_IN_WITNESS_UTXO`.
pub const IN_WITNESS_UTXO: u8 = 0x01;
/// `PSBT_IN_PARTIAL_SIG`.
pub const IN_PARTIAL_SIG: u8 = 0x02;
/// `PSBT_IN_SIGHASH_TYPE`.
pub const IN_SIGHASH_TYPE: u8 = 0x03;
/// `PSBT_IN_REDEEM_SCRIPT`.
pub const IN_REDEEM_SCRIPT: u8 = 0x04;
/// `PSBT_IN_BIP32_DERIVATION`.
pub const IN_BIP32_DERIVATION: u8 = 0x06;
/// `PSBT_IN_FINAL_SCRIPTSIG`.
pub const IN_FINAL_SCRIPTSIG: u8 = 0x07;
/// `PSBT_IN_FINAL_SCRIPTWITNESS`.
pub const IN_FINAL_SCRIPTWITNESS: u8 = 0x08;
/// `PSBT_IN_PREVIOUS_TXID`.
pub const IN_PREVIOUS_TXID: u8 = 0x0e;
/// `PSBT_IN_OUTPUT_INDEX`.
pub const IN_OUTPUT_INDEX: u8 = 0x0f;
/// `PSBT_IN_SEQUENCE`.
pub const IN_SEQUENCE: u8 = 0x10;
/// `PSBT_IN_REQUIRED_TIME_LOCKTIME`.
pub const IN_REQUIRED_TIME_LOCKTIME: u8 = 0x11;
/// `PSBT_IN_REQUIRED_HEIGHT_LOCKTIME`.
pub const IN_REQUIRED_HEIGHT_LOCKTIME: u8 = 0x12;
/// `PSBT_IN_TAP_KEY_SIG`.
pub const IN_TAP_KEY_SIG: u8 = 0x13;
/// `PSBT_IN_TAP_SCRIPT_SIG`.
pub const IN_TAP_SCRIPT_SIG: u8 = 0x14;
/// `PSBT_IN_TAP_BIP32_DERIVATION`.
pub const IN_TAP_BIP32_DERIVATION: u8 = 0x16;
/// `PSBT_IN_TAP_INTERNAL_KEY`.
pub const IN_TAP_INTERNAL_KEY: u8 = 0x17;
/// `PSBT_IN_TAP_MERKLE_ROOT`.
pub const IN_TAP_MERKLE_ROOT: u8 = 0x18;

/// `PSBT_OUT_AMOUNT`.
pub const OUT_AMOUNT: u8 = 0x03;
/// `PSBT_OUT_SCRIPT`.
pub const OUT_SCRIPT: u8 = 0x04;
/// BIP-370: a required time locktime is at least this, and a required
/// height locktime below it.
const LOCKTIME_THRESHOLD: u32 = 500_000_000;

/// Whether a pair of an input map is a taproot signature
/// (`PSBT_IN_TAP_KEY_SIG`, `PSBT_IN_TAP_SCRIPT_SIG`) of 65 bytes whose
/// sighash byte is 0: BIP-341 makes it invalid, since the default type
/// is written as 64 bytes.
pub(crate) fn is_zero_sighash_tap_sig(key: &[u8], value: &[u8]) -> bool {
    let tap_sig = key == [IN_TAP_KEY_SIG] || key.first() == Some(&IN_TAP_SCRIPT_SIG);
    tap_sig && value.len() == 65 && value[64] == 0
}

/// The global fields version 0 does not have, which [`PsbtV2::to_v0`]
/// leaves out: BIP-370's and BIP-375's.
const GLOBAL_V2_ONLY: &[u8] = &[
    GLOBAL_TX_VERSION,
    GLOBAL_FALLBACK_LOCKTIME,
    GLOBAL_INPUT_COUNT,
    GLOBAL_OUTPUT_COUNT,
    GLOBAL_TX_MODIFIABLE,
    GLOBAL_VERSION,
    0x07,
    0x08,
];
/// The input fields version 0 does not have: BIP-370's and BIP-375's.
const INPUT_V2_ONLY: &[u8] = &[
    IN_PREVIOUS_TXID,
    IN_OUTPUT_INDEX,
    IN_SEQUENCE,
    IN_REQUIRED_TIME_LOCKTIME,
    IN_REQUIRED_HEIGHT_LOCKTIME,
    0x1d,
    0x1e,
];
/// The output fields version 0 does not have: BIP-370's and BIP-375's.
const OUTPUT_V2_ONLY: &[u8] = &[OUT_AMOUNT, OUT_SCRIPT, 0x09, 0x0a];

/// The input fields a signing pass adds, which [`PsbtV2::merge_signed`]
/// takes back: partial signatures, the final scriptSig and witness,
/// taproot key and script signatures, and MuSig2's public nonces and
/// partial signatures (BIP-373).
const SIGNING_FIELDS: &[u8] = &[
    IN_PARTIAL_SIG,
    IN_FINAL_SCRIPTSIG,
    IN_FINAL_SCRIPTWITNESS,
    IN_TAP_KEY_SIG,
    IN_TAP_SCRIPT_SIG,
    crate::musig::IN_PUB_NONCE,
    crate::musig::IN_PARTIAL_SIG,
];

/// Why a version 2 PSBT could not be read or bridged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The text is not valid base64.
    Base64,
    /// The bytes do not start with the PSBT magic.
    NotPsbt,
    /// The bytes end inside a map.
    Truncated,
    /// A key appears twice in one map.
    DuplicateKey,
    /// Bytes follow the last map.
    TrailingBytes,
    /// `PSBT_GLOBAL_VERSION` is absent or not 2.
    Version,
    /// A field BIP-370 requires is absent, or has the wrong length; the
    /// text names it.
    Field(&'static str),
    /// `PSBT_GLOBAL_UNSIGNED_TX` is present, which version 2 excludes.
    UnsignedTx,
    /// An output has no script yet, so there is no transaction to sign.
    NoScript {
        /// Output index.
        output: usize,
    },
    /// The inputs' required locktimes cannot all be met by one
    /// locktime: some ask for a time and others for a height.
    Locktime,
    /// The version 0 PSBT written from this one did not read back.
    Bridge,
    /// The signed version 0 PSBT is not of this transaction, or it
    /// changes a field this one already has.
    Mismatch,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}") // not shown: a developer's text; the app shows no PSBT's parse error
    }
}

impl core::error::Error for Error {}

/// One map of a PSBT: its key-value pairs in the order they were read,
/// with what was set since at the end.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Map {
    pairs: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Map {
    /// The value under the whole key `key` (type byte and key data).
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_slice())
    }

    /// The value under the key that is the type `ty` alone.
    pub fn get_type(&self, ty: u8) -> Option<&[u8]> {
        self.get(&[ty])
    }

    /// Every pair of type `ty`, as (key data, value). `ty` is below
    /// `0xfd`, so it is the key's whole first byte.
    pub fn all_of(&self, ty: u8) -> impl Iterator<Item = (&[u8], &[u8])> {
        self.pairs
            .iter()
            .filter(move |(k, _)| k.first() == Some(&ty))
            .map(|(k, v)| (&k[1..], v.as_slice()))
    }

    /// Every pair, as (whole key, value).
    pub fn pairs(&self) -> impl Iterator<Item = (&[u8], &[u8])> {
        self.pairs.iter().map(|(k, v)| (k.as_slice(), v.as_slice()))
    }

    /// Sets `key` to `value`, in place if it is there, at the end if not.
    pub fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        match self.pairs.iter_mut().find(|(k, _)| *k == key) {
            Some((_, v)) => *v = value,
            None => self.pairs.push((key, value)),
        }
    }

    /// Removes `key`, where it is there.
    pub fn remove(&mut self, key: &[u8]) {
        self.pairs.retain(|(k, _)| k != key);
    }

    /// Removes every pair of type `ty`.
    pub fn remove_type(&mut self, ty: u8) {
        self.pairs.retain(|(k, _)| k.first() != Some(&ty));
    }

    /// A map of these pairs, in this order.
    pub(crate) fn from_pairs(pairs: Vec<(Vec<u8>, Vec<u8>)>) -> Map {
        Map { pairs }
    }

    /// Writes the map: its pairs, then the separator.
    pub(crate) fn write_to(&self, out: &mut Vec<u8>) {
        self.write(out);
    }

    fn write(&self, out: &mut Vec<u8>) {
        for (k, v) in &self.pairs {
            write_compact(out, k.len() as u64);
            out.extend_from_slice(k);
            write_compact(out, v.len() as u64);
            out.extend_from_slice(v);
        }
        out.push(0);
    }

    /// A copy without the pairs whose type is in `drop`.
    fn without(&self, drop: &[u8]) -> Map {
        Map {
            pairs: self
                .pairs
                .iter()
                .filter(|(k, _)| !k.first().is_some_and(|t| drop.contains(t)))
                .cloned()
                .collect(),
        }
    }
}

/// A reader over the PSBT's bytes.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], Error> {
        let end = self.at.checked_add(n).ok_or(Error::Truncated)?;
        let slice = self.bytes.get(self.at..end).ok_or(Error::Truncated)?;
        self.at = end;
        Ok(slice)
    }

    fn compact(&mut self) -> Result<u64, Error> {
        let first = self.take(1)?[0];
        Ok(match first {
            0xfd => u64::from(u16::from_le_bytes(
                self.take(2)?.try_into().expect("two bytes"),
            )),
            0xfe => u64::from(u32::from_le_bytes(
                self.take(4)?.try_into().expect("four bytes"),
            )),
            0xff => u64::from_le_bytes(self.take(8)?.try_into().expect("eight bytes")),
            n => u64::from(n),
        })
    }

    fn map(&mut self) -> Result<Map, Error> {
        let mut map = Map::default();
        loop {
            let key_len = self.compact()?;
            if key_len == 0 {
                return Ok(map);
            }
            let key = self.take(usize::try_from(key_len).map_err(|_| Error::Truncated)?)?;
            let key = key.to_vec();
            let value_len = self.compact()?;
            let value = self
                .take(usize::try_from(value_len).map_err(|_| Error::Truncated)?)?
                .to_vec();
            if map.get(&key).is_some() {
                return Err(Error::DuplicateKey);
            }
            map.pairs.push((key, value));
        }
    }
}

/// One map of a PSBT's bytes, from `at`, which is moved past it.
pub(crate) fn read_map(bytes: &[u8], at: &mut usize) -> Result<Map, Error> {
    let mut r = Reader { bytes, at: *at };
    let map = r.map()?;
    *at = r.at;
    Ok(map)
}

/// A field that may be absent and, present, is `n` bytes.
fn sized<'m>(
    map: &'m Map,
    ty: u8,
    n: usize,
    name: &'static str,
) -> Result<Option<&'m [u8]>, Error> {
    match map.get_type(ty) {
        Some(v) if v.len() != n => Err(Error::Field(name)),
        v => Ok(v),
    }
}

fn write_compact(out: &mut Vec<u8>, n: u64) {
    match n {
        0..=0xfc => out.push(n as u8),
        0xfd..=0xffff => {
            out.push(0xfd);
            out.extend_from_slice(&(n as u16).to_le_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(0xfe);
            out.extend_from_slice(&(n as u32).to_le_bytes());
        }
        _ => {
            out.push(0xff);
            out.extend_from_slice(&n.to_le_bytes());
        }
    }
}

/// A compact size that is the whole of `bytes`.
fn compact_value(bytes: &[u8]) -> Option<u64> {
    let mut r = Reader { bytes, at: 0 };
    let n = r.compact().ok()?;
    (r.at == bytes.len()).then_some(n)
}

/// A little-endian `u32` that is the whole of `bytes`.
fn u32_value(bytes: &[u8]) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

/// A version 2 PSBT as its maps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsbtV2 {
    /// The global map.
    pub global: Map,
    /// One map per input, in order.
    pub inputs: Vec<Map>,
    /// One map per output, in order.
    pub outputs: Vec<Map>,
}

/// An output's script, where it has one. BIP-370's `PSBT_OUT_SCRIPT` is
/// the scriptPubKey, so an empty value is the empty script; only on a
/// BIP-375 silent payment output (`PSBT_OUT_SP_V0_INFO`) does an empty
/// value stand for the script not yet given, as BIP-375's reference
/// reads it.
fn script_given(map: &Map) -> Option<&[u8]> {
    let script = map.get_type(OUT_SCRIPT)?;
    let pending = script.is_empty() && map.get_type(crate::silent::OUT_SP_V0_INFO).is_some();
    (!pending).then_some(script)
}

/// What BIP-370 and BIP-341 ask of an input map on reading: its
/// previous output, the sizes of its sequence and required locktimes, a
/// required time locktime from 500 000 000 and a required height
/// locktime from 1 to 499 999 999, no taproot signature whose sighash
/// byte is 0.
fn check_input(map: &Map) -> Result<(), Error> {
    if map.get_type(IN_PREVIOUS_TXID).map(<[u8]>::len) != Some(32) {
        return Err(Error::Field("PSBT_IN_PREVIOUS_TXID"));
    }
    if map.get_type(IN_OUTPUT_INDEX).and_then(u32_value).is_none() {
        return Err(Error::Field("PSBT_IN_OUTPUT_INDEX"));
    }
    sized(map, IN_SEQUENCE, 4, "PSBT_IN_SEQUENCE")?;
    if let Some(t) = sized(
        map,
        IN_REQUIRED_TIME_LOCKTIME,
        4,
        "PSBT_IN_REQUIRED_TIME_LOCKTIME",
    )?
    .and_then(u32_value)
        && t < LOCKTIME_THRESHOLD
    {
        return Err(Error::Field("PSBT_IN_REQUIRED_TIME_LOCKTIME"));
    }
    if let Some(h) = sized(
        map,
        IN_REQUIRED_HEIGHT_LOCKTIME,
        4,
        "PSBT_IN_REQUIRED_HEIGHT_LOCKTIME",
    )?
    .and_then(u32_value)
        && !(1..LOCKTIME_THRESHOLD).contains(&h)
    {
        return Err(Error::Field("PSBT_IN_REQUIRED_HEIGHT_LOCKTIME"));
    }
    if map.pairs().any(|(k, v)| is_zero_sighash_tap_sig(k, v)) {
        return Err(Error::Field(
            // not wording: the fields BIP-370 names
            "PSBT_IN_TAP_KEY_SIG or PSBT_IN_TAP_SCRIPT_SIG",
        ));
    }
    Ok(())
}

impl PsbtV2 {
    /// Costs a decode of the whole PSBT: reads the binary encoding.
    pub fn parse_bytes(c: &Compute, bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < MAGIC.len() || &bytes[..MAGIC.len()] != MAGIC {
            return Err(Error::NotPsbt);
        }
        c.count(Work::PsbtParse);
        let mut r = Reader {
            bytes,
            at: MAGIC.len(),
        };
        let global = r.map()?;
        if global.get_type(GLOBAL_VERSION).and_then(u32_value) != Some(2) {
            return Err(Error::Version);
        }
        if global.get_type(GLOBAL_UNSIGNED_TX).is_some() {
            return Err(Error::UnsignedTx);
        }
        if global
            .get_type(GLOBAL_TX_VERSION)
            .and_then(u32_value)
            .is_none()
        {
            return Err(Error::Field("PSBT_GLOBAL_TX_VERSION"));
        }
        let count = |ty, name| {
            global
                .get_type(ty)
                .and_then(compact_value)
                .ok_or(Error::Field(name))
        };
        sized(
            &global,
            GLOBAL_FALLBACK_LOCKTIME,
            4,
            "PSBT_GLOBAL_FALLBACK_LOCKTIME",
        )?;
        sized(
            &global,
            GLOBAL_TX_MODIFIABLE,
            1,
            "PSBT_GLOBAL_TX_MODIFIABLE",
        )?;
        let n_in = count(GLOBAL_INPUT_COUNT, "PSBT_GLOBAL_INPUT_COUNT")?;
        let n_out = count(GLOBAL_OUTPUT_COUNT, "PSBT_GLOBAL_OUTPUT_COUNT")?;
        // Every map takes at least its terminating byte, so a count
        // above what is left is refused before anything is allocated.
        let left = (bytes.len() - r.at) as u64;
        if n_in.saturating_add(n_out) > left {
            return Err(Error::Truncated);
        }
        let mut inputs = Vec::with_capacity(n_in as usize);
        for _ in 0..n_in {
            let map = r.map()?;
            check_input(&map)?;
            inputs.push(map);
        }
        let mut outputs = Vec::with_capacity(n_out as usize);
        for _ in 0..n_out {
            let map = r.map()?;
            if map.get_type(OUT_AMOUNT).map(<[u8]>::len) != Some(8) {
                return Err(Error::Field("PSBT_OUT_AMOUNT"));
            }
            // BIP-375: a silent payment output (PSBT_OUT_SP_V0_INFO)
            // has no script until the shares give it.
            if map.get_type(OUT_SCRIPT).is_none()
                && map.get_type(crate::silent::OUT_SP_V0_INFO).is_none()
            {
                return Err(Error::Field("PSBT_OUT_SCRIPT"));
            }
            outputs.push(map);
        }
        if r.at != bytes.len() {
            return Err(Error::TrailingBytes);
        }
        Ok(PsbtV2 {
            global,
            inputs,
            outputs,
        })
    }

    /// Costs a base64 decode and what [`PsbtV2::parse_bytes`] costs:
    /// reads the base64 text encoding. Surrounding whitespace is
    /// ignored.
    pub fn parse_base64(c: &Compute, text: &str) -> Result<Self, Error> {
        let bytes = base64::decode(text).map_err(|_| Error::Base64)?;
        Self::parse_bytes(c, &bytes)
    }

    /// The binary encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        self.global.write(&mut out);
        for map in self.inputs.iter().chain(self.outputs.iter()) {
            map.write(&mut out);
        }
        out
    }

    /// The base64 text encoding.
    pub fn to_base64(&self) -> alloc::string::String {
        base64::encode(&self.to_bytes())
    }

    /// The outpoint input `index` spends, as a transaction serializes
    /// it: the txid as stored, then the index little-endian.
    pub fn outpoint_bytes(&self, index: usize) -> [u8; 36] {
        let map = &self.inputs[index];
        let mut out = [0u8; 36];
        out[..32].copy_from_slice(map.get_type(IN_PREVIOUS_TXID).expect("checked on reading"));
        out[32..].copy_from_slice(map.get_type(IN_OUTPUT_INDEX).expect("checked on reading"));
        out
    }

    /// Output `index`'s script, where it has one.
    pub fn output_script(&self, index: usize) -> Option<&[u8]> {
        self.outputs[index].get_type(OUT_SCRIPT)
    }

    /// BIP-370's locktime: the largest of the inputs' required
    /// locktimes of the one kind every input that states one can meet,
    /// heights first, and the fallback locktime (or 0) when none states
    /// one.
    pub fn locktime(&self) -> Result<u32, Error> {
        let mut any = false;
        let (mut heights_ok, mut times_ok) = (true, true);
        let (mut height, mut time) = (0u32, 0u32);
        for map in &self.inputs {
            let h = map.get_type(IN_REQUIRED_HEIGHT_LOCKTIME);
            let t = map.get_type(IN_REQUIRED_TIME_LOCKTIME);
            if h.is_none() && t.is_none() {
                continue;
            }
            any = true;
            match h {
                Some(h) => {
                    let h = u32_value(h).ok_or(Error::Field("PSBT_IN_REQUIRED_HEIGHT_LOCKTIME"))?;
                    height = height.max(h);
                }
                None => heights_ok = false,
            }
            match t {
                Some(t) => {
                    let t = u32_value(t).ok_or(Error::Field("PSBT_IN_REQUIRED_TIME_LOCKTIME"))?;
                    time = time.max(t);
                }
                None => times_ok = false,
            }
        }
        if !any {
            return match self.global.get_type(GLOBAL_FALLBACK_LOCKTIME) {
                Some(v) => u32_value(v).ok_or(Error::Field("PSBT_GLOBAL_FALLBACK_LOCKTIME")),
                None => Ok(0),
            };
        }
        if heights_ok {
            Ok(height)
        } else if times_ok {
            Ok(time)
        } else {
            Err(Error::Locktime)
        }
    }

    /// The unsigned transaction, once every output has its script
    /// ([`script_given`]).
    pub fn unsigned_tx(&self) -> Result<Transaction, Error> {
        let version = self
            .global
            .get_type(GLOBAL_TX_VERSION)
            .and_then(u32_value)
            .ok_or(Error::Field("PSBT_GLOBAL_TX_VERSION"))?;
        let input = self
            .inputs
            .iter()
            .enumerate()
            .map(|(i, map)| {
                let point = self.outpoint_bytes(i);
                let txid = Txid::from_byte_array(point[..32].try_into().expect("32 bytes"));
                let vout = u32::from_le_bytes(point[32..].try_into().expect("four bytes"));
                let sequence = map
                    .get_type(IN_SEQUENCE)
                    .and_then(u32_value)
                    .unwrap_or(u32::MAX);
                TxIn {
                    previous_output: OutPoint { txid, vout },
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence(sequence),
                    witness: Witness::new(),
                }
            })
            .collect();
        let mut output = Vec::with_capacity(self.outputs.len());
        for (i, map) in self.outputs.iter().enumerate() {
            let amount = u64::from_le_bytes(
                map.get_type(OUT_AMOUNT)
                    .expect("checked on reading")
                    .try_into()
                    .expect("eight bytes"),
            );
            let script = script_given(map).ok_or(Error::NoScript { output: i })?;
            output.push(TxOut {
                value: Amount::from_sat(amount),
                script_pubkey: ScriptBuf::from_bytes(script.to_vec()),
            });
        }
        Ok(Transaction {
            version: Version(version as i32),
            lock_time: LockTime::from_consensus(self.locktime()?),
            input,
            output,
        })
    }

    /// Costs a decode of the whole PSBT: the version 0 PSBT of the same
    /// transaction, with every field version 0 has, for the signer.
    pub fn to_v0(&self, c: &Compute) -> Result<crate::Psbt, Error> {
        let tx = self.unsigned_tx()?;
        let mut out = MAGIC.to_vec();
        let mut global = self.global.without(GLOBAL_V2_ONLY);
        global.remove_type(GLOBAL_UNSIGNED_TX);
        global.pairs.insert(
            0,
            (
                alloc::vec![GLOBAL_UNSIGNED_TX],
                bitcoin::consensus::serialize(&tx),
            ),
        );
        global.write(&mut out);
        for map in &self.inputs {
            map.without(INPUT_V2_ONLY).write(&mut out);
        }
        for map in &self.outputs {
            map.without(OUTPUT_V2_ONLY).write(&mut out);
        }
        crate::Psbt::parse_bytes(c, &out).map_err(|_| Error::Bridge)
    }

    /// Costs a write and a read of the version 0 PSBT's maps: takes the
    /// version 2 PSBT of `psbt`, a version 0 one. Every pair of `psbt`
    /// is kept in its order but `PSBT_GLOBAL_UNSIGNED_TX` and a
    /// `PSBT_GLOBAL_VERSION` of 0; the transaction's version, its
    /// locktime as the fallback locktime and the counts lead the global
    /// map, each input's outpoint and sequence lead its map, each
    /// output's amount and script lead its map, and the global map ends
    /// with a `PSBT_GLOBAL_VERSION` of 2, as BIP-370 lays a version 2
    /// PSBT out. [`PsbtV2::to_v0`] of it writes `psbt`'s bytes back
    /// where its unsigned transaction is its first pair, it carries no
    /// `PSBT_GLOBAL_VERSION`, and no pair of a type BIP-375 defines for
    /// version 2 alone (global `0x07`/`0x08`, input `0x1d`/`0x1e`,
    /// output `0x09`/`0x0a`): such a pair, unknown to version 0, is kept
    /// and then reads as that field, which `to_v0` leaves out.
    pub fn from_v0(c: &Compute, psbt: &crate::Psbt) -> PsbtV2 {
        c.count(Work::PsbtParse);
        let tx = psbt.unsigned_tx();
        let bytes = psbt.to_bytes();
        let mut r = Reader {
            bytes: &bytes,
            at: MAGIC.len(),
        };
        let read = |r: &mut Reader| r.map().expect("a PSBT this crate wrote reads back");
        let v0_global = read(&mut r);
        let mut global = Map::default();
        global.set(
            alloc::vec![GLOBAL_TX_VERSION],
            (tx.version.0 as u32).to_le_bytes().to_vec(),
        );
        global.set(
            alloc::vec![GLOBAL_FALLBACK_LOCKTIME],
            tx.lock_time.to_consensus_u32().to_le_bytes().to_vec(),
        );
        let mut count = |ty: u8, n: usize| {
            let mut v = Vec::new();
            write_compact(&mut v, n as u64);
            global.set(alloc::vec![ty], v);
        };
        count(GLOBAL_INPUT_COUNT, tx.input.len());
        count(GLOBAL_OUTPUT_COUNT, tx.output.len());
        for (k, v) in v0_global
            .without(&[GLOBAL_UNSIGNED_TX, GLOBAL_VERSION])
            .pairs
        {
            global.set(k, v);
        }
        global.set(alloc::vec![GLOBAL_VERSION], 2u32.to_le_bytes().to_vec());
        let inputs = tx
            .input
            .iter()
            .map(|txin| {
                let mut map = Map::default();
                let point = &txin.previous_output;
                map.set(
                    alloc::vec![IN_PREVIOUS_TXID],
                    point.txid.to_byte_array().to_vec(),
                );
                map.set(
                    alloc::vec![IN_OUTPUT_INDEX],
                    point.vout.to_le_bytes().to_vec(),
                );
                map.set(
                    alloc::vec![IN_SEQUENCE],
                    txin.sequence.0.to_le_bytes().to_vec(),
                );
                map.pairs.extend(read(&mut r).pairs);
                map
            })
            .collect();
        let outputs = tx
            .output
            .iter()
            .map(|txout| {
                let mut map = Map::default();
                map.set(
                    alloc::vec![OUT_AMOUNT],
                    txout.value.to_sat().to_le_bytes().to_vec(),
                );
                map.set(
                    alloc::vec![OUT_SCRIPT],
                    txout.script_pubkey.as_bytes().to_vec(),
                );
                map.pairs.extend(read(&mut r).pairs);
                map
            })
            .collect();
        PsbtV2 {
            global,
            inputs,
            outputs,
        }
    }

    /// Takes into this PSBT what a signing pass added to its version 0
    /// form ([`PsbtV2::to_v0`]): every input pair of a signing field
    /// (`SIGNING_FIELDS`: partial signatures, final scripts, taproot
    /// and MuSig2 signatures and nonces) this one does not have, and
    /// nothing else. Refuses, changing nothing, when the signed PSBT is
    /// of another transaction or changes a pair this one already has.
    pub fn merge_signed(&mut self, signed: &crate::Psbt) -> Result<usize, Error> {
        if signed.unsigned_tx() != &self.unsigned_tx()? {
            return Err(Error::Mismatch);
        }
        let bytes = signed.to_bytes();
        let mut r = Reader {
            bytes: &bytes,
            at: MAGIC.len(),
        };
        r.map().map_err(|_| Error::Bridge)?;
        let mut added: Vec<(usize, Vec<u8>, Vec<u8>)> = Vec::new();
        for (i, ours) in self.inputs.iter().enumerate() {
            let theirs = r.map().map_err(|_| Error::Bridge)?;
            let signing = |k: &[u8]| k.first().is_some_and(|t| SIGNING_FIELDS.contains(t));
            for (k, v) in theirs.pairs().filter(|(k, _)| signing(k)) {
                match ours.get(k) {
                    Some(existing) if existing == v => {}
                    Some(_) => return Err(Error::Mismatch),
                    None => added.push((i, k.to_vec(), v.to_vec())),
                }
            }
        }
        let n = added.len();
        for (i, k, v) in added {
            self.inputs[i].set(k, v);
        }
        Ok(n)
    }
}
