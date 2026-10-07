//! The `osk` PSBT records of a threshold spend, and the carry file that
//! takes a secret nonce from one location to the next
//! (`docs/PLANNING.md` §16.103 items 3 and 4).
//!
//! Two BIP-174 proprietary records, both in an input's `proprietary`
//! map under the identifier `osk`:
//!
//! | Record | subtype | Key data | Value |
//! |---|---|---|---|
//! | public nonce | `0x00` | participant's public share (33) ‖ taproot output key (33) | 66-byte `PubNonce` |
//! | partial signature | `0x01` | the same | 32-byte `PartialSig` |
//!
//! Nothing of the group is in the PSBT. A participant is named by its
//! public share, so the records are self-describing, but the threshold,
//! the group key and the whole share list come from the registered
//! record and from nowhere else: an input carrying `osk` records for a
//! group no registered wallet has is blocked.
//!
//! The secret nonce is never in the PSBT. It travels in the carry file,
//! which is one file holding both the section and the PSBT, because the
//! shell's file channel answers one request with one file and a picker
//! shell cannot fetch a sibling by name. Its magic is `OSKC`, so nothing
//! that reads PSBTs mistakes it for one.
//!
//! ```text
//! magic "OSKC" (4)  version 0x01 (1)
//! section:
//!   signers u8, then that many public shares (33 each), in identifier order
//!   entries u8, then per entry:
//!     input index u32 BE
//!     public share of the signer whose nonce this is (33)
//!     sighash (32)
//!     secret nonce (64)
//! psbt: the rest of the file, binary PSBT bytes
//! ```
//!
//! A [`CarrySection`] is a secret: it is not `Clone`, it never prints
//! its nonces, and it is wiped on drop. This file is in the
//! secret-hygiene lint's list, so it carries no heap text.

use alloc::vec::Vec;
use core::fmt;

use bitcoin::psbt::{Input, raw};
use bitcoin::secp256k1::PublicKey;
use osk_bip::frost::{PartialSig, PubNonce, SecNonce};
use osk_crypto::Zeroize;

/// The proprietary identifier both records live under.
pub const PREFIX: &[u8] = b"osk";
/// A participant's public nonce.
pub const SUBTYPE_PUB_NONCE: u8 = 0x00;
/// A participant's partial signature, which is not the final one: that
/// is written as `tap_key_sig`.
pub const SUBTYPE_PARTIAL_SIG: u8 = 0x01;

/// The carry file's magic.
pub const CARRY_MAGIC: &[u8; 4] = b"OSKC";
/// The carry file's version, the only one this build reads.
pub const CARRY_VERSION: u8 = 0x01;

/// The name a carry file is saved under.
pub const CARRY_NAME: &str = "partly-signed.osk";

/// Why an `osk` record or a carry file could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A record's key data is not a public share followed by a taproot
    /// output key.
    EntryKey,
    /// A public nonce is not 66 bytes, or either half is not a point.
    Nonce,
    /// A partial signature is not a 32-byte scalar below the curve
    /// order.
    Signature,
    /// The file does not begin with `OSKC` and a version this build
    /// reads.
    CarryMagic,
    /// The section ends before it says it does, or a field in it is not
    /// the length the format gives.
    CarryTruncated,
    /// A public share in the section is no point on the curve.
    CarryShare,
    /// A secret nonce in the section is not two scalars in `1..n`.
    CarryNonce,
}

impl Error {
    /// The reason as the blocked-input warning states it.
    pub fn reason(self) -> &'static str {
        match self {
            Error::EntryKey => "a threshold nonce or partial signature names no share",
            Error::Nonce => "a threshold public nonce is malformed",
            Error::Signature => "a threshold partial signature is malformed",
            Error::CarryMagic => "the file is not a partly signed transaction",
            Error::CarryTruncated => "the partly signed transaction ends early",
            Error::CarryShare => "the file names a share that is no point on the curve",
            Error::CarryNonce => "the file holds a secret nonce that is out of range",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl core::error::Error for Error {}

/// Why a threshold input was refused, in the order §16.103 checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// A carry file was given for a transaction whose sighash on this
    /// input is not the one it binds.
    AnotherTransaction,
    /// The carry file holds no secret nonce for a share this device
    /// holds on this input.
    AnotherShare,
    /// The signer set the records name is not the one the file binds.
    SignerSet,
    /// A stored secret nonce's public nonce is not the record beside it.
    NonceMismatch,
    /// A partial signature already on the input does not verify.
    PartialSig,
    /// The input carries public nonces and no carry file was given.
    NoSection,
    /// A carry file was given for a PSBT that carries no nonce records.
    NoNonces,
    /// This device's share is not one of the signers the records name.
    NotASigner,
    /// The others chosen to sign are not `t - 1` distinct participants
    /// other than this one.
    ChosenCount,
}

impl Refusal {
    /// The reason as the blocked-input warning states it.
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::AnotherTransaction => "the file is for another transaction",
            Refusal::AnotherShare => "the file is for another share",
            Refusal::SignerSet => "the file names another signer set",
            Refusal::NonceMismatch => "a nonce does not match its record",
            Refusal::PartialSig => "a partial signature does not verify",
            Refusal::NoSection => "the transaction carries nonces and no file was read",
            Refusal::NoNonces => "the file is for a transaction with no nonces on it",
            Refusal::NotASigner => "this share is not one of the signers",
            Refusal::ChosenCount => "the chosen shares are not the number that must sign",
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl core::error::Error for Refusal {}

/// One participant's public nonce or partial signature, and the taproot
/// output key its record names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<T> {
    /// The public share of the participant whose record this is.
    pub pubshare: PublicKey,
    /// The taproot output key the record is keyed by.
    pub output_key: PublicKey,
    /// The nonce or the partial signature.
    pub value: T,
}

fn key_of(record: &raw::ProprietaryKey) -> Result<(PublicKey, PublicKey), Error> {
    if record.key.len() != 66 {
        return Err(Error::EntryKey);
    }
    Ok((
        PublicKey::from_slice(&record.key[..33]).map_err(|_| Error::EntryKey)?,
        PublicKey::from_slice(&record.key[33..]).map_err(|_| Error::EntryKey)?,
    ))
}

fn of_subtype(input: &Input, subtype: u8) -> Vec<(&raw::ProprietaryKey, &Vec<u8>)> {
    input
        .proprietary
        .iter()
        .filter(|(k, _)| k.prefix == PREFIX && k.subtype == subtype)
        .collect()
}

/// Every `osk` public-nonce record an input carries.
pub fn pub_nonces(input: &Input) -> Result<Vec<Entry<PubNonce>>, Error> {
    let mut out = Vec::new();
    for (key, value) in of_subtype(input, SUBTYPE_PUB_NONCE) {
        let (pubshare, output_key) = key_of(key)?;
        out.push(Entry {
            pubshare,
            output_key,
            value: PubNonce::from_bytes(value).map_err(|_| Error::Nonce)?,
        });
    }
    Ok(out)
}

/// Every `osk` partial-signature record an input carries.
pub fn partial_sigs(input: &Input) -> Result<Vec<Entry<PartialSig>>, Error> {
    let mut out = Vec::new();
    for (key, value) in of_subtype(input, SUBTYPE_PARTIAL_SIG) {
        let (pubshare, output_key) = key_of(key)?;
        out.push(Entry {
            pubshare,
            output_key,
            value: PartialSig::from_bytes(value).map_err(|_| Error::Signature)?,
        });
    }
    Ok(out)
}

fn record_key(subtype: u8, pubshare: &PublicKey, output_key: &PublicKey) -> raw::ProprietaryKey {
    let mut key = Vec::with_capacity(66);
    key.extend_from_slice(&pubshare.serialize());
    key.extend_from_slice(&output_key.serialize());
    raw::ProprietaryKey {
        prefix: PREFIX.to_vec(),
        subtype,
        key,
    }
}

/// Writes a participant's public nonce, keyed by the output key.
pub fn write_pub_nonce(
    input: &mut Input,
    pubshare: &PublicKey,
    output_key: &PublicKey,
    nonce: &PubNonce,
) {
    input.proprietary.insert(
        record_key(SUBTYPE_PUB_NONCE, pubshare, output_key),
        nonce.serialize().to_vec(),
    );
}

/// Writes a participant's partial signature, keyed by the output key.
pub fn write_partial_sig(
    input: &mut Input,
    pubshare: &PublicKey,
    output_key: &PublicKey,
    sig: &PartialSig,
) {
    input.proprietary.insert(
        record_key(SUBTYPE_PARTIAL_SIG, pubshare, output_key),
        sig.serialize().to_vec(),
    );
}

/// Removes every `osk` record from every input, which is what runs
/// before a finished transaction leaves the device: a coordinator sees
/// the PSBT it built and the transaction, and never the session.
pub fn strip(psbt: &mut crate::Psbt) {
    for input in &mut psbt.inner_mut().inputs {
        input.proprietary.retain(|k, _| k.prefix != PREFIX);
    }
}

// ---------------------------------------------------------------------
// The carry file
// ---------------------------------------------------------------------

/// One stored secret nonce: whose it is, which input and transaction it
/// is bound to, and the nonce itself.
pub struct CarryEntry {
    /// The input the nonce was drawn for.
    pub input: u32,
    /// The public share of the signer whose nonce this is.
    pub pubshare: PublicKey,
    /// The BIP-341 key-path sighash of that input, which binds the nonce
    /// to this transaction.
    pub sighash: [u8; 32],
    /// The secret nonce, consumed by the signing that uses it.
    pub secnonce: SecNonce,
}

impl fmt::Debug for CarryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CarryEntry(..)")
    }
}

impl Zeroize for CarryEntry {
    fn zeroize(&mut self) {
        self.sighash.zeroize();
        self.secnonce.zeroize();
    }
}

impl Drop for CarryEntry {
    fn drop(&mut self) {
        self.sighash.zeroize();
    }
}

/// The secret half of a carry file: the signer set the transaction is
/// being signed by, and one secret nonce per signer that has not signed
/// yet.
///
/// Not `Clone`, never printed, wiped on drop.
pub struct CarrySection {
    /// The signers' public shares, in identifier order. Every location
    /// after the first checks the records against this.
    pub signers: Vec<PublicKey>,
    /// The nonces still to be used.
    pub entries: Vec<CarryEntry>,
}

impl CarrySection {
    /// An empty section for `signers`.
    pub fn new(signers: Vec<PublicKey>) -> Self {
        CarrySection {
            signers,
            entries: Vec::new(),
        }
    }

    /// Whether it holds nothing, which is a pass that left nobody to
    /// sign.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The entry for `pubshare` on `input`, if the section holds one.
    pub fn entry(&self, input: usize, pubshare: &PublicKey) -> Option<&CarryEntry> {
        self.entries
            .iter()
            .find(|e| e.input as usize == input && e.pubshare == *pubshare)
    }

    /// Takes that entry out of the section, so the nonce in it cannot be
    /// used twice in one pass and is not written into a new section.
    pub fn take(&mut self, input: usize, pubshare: &PublicKey) -> Option<CarryEntry> {
        let at = self
            .entries
            .iter()
            .position(|e| e.input as usize == input && e.pubshare == *pubshare)?;
        Some(self.entries.remove(at))
    }

    /// The section's bytes, without the magic or the PSBT.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(2 + self.signers.len() * 33 + self.entries.len() * 133);
        out.push(self.signers.len() as u8);
        for share in &self.signers {
            out.extend_from_slice(&share.serialize());
        }
        out.push(self.entries.len() as u8);
        for entry in &self.entries {
            out.extend_from_slice(&entry.input.to_be_bytes());
            out.extend_from_slice(&entry.pubshare.serialize());
            out.extend_from_slice(&entry.sighash);
            out.extend_from_slice(&entry.secnonce.serialize());
        }
        out
    }

    /// Reads a section, returning it and the bytes after it.
    fn parse(bytes: &[u8]) -> Result<(CarrySection, &[u8]), Error> {
        let (&count, mut rest) = bytes.split_first().ok_or(Error::CarryTruncated)?;
        let mut signers = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            if rest.len() < 33 {
                return Err(Error::CarryTruncated);
            }
            signers.push(PublicKey::from_slice(&rest[..33]).map_err(|_| Error::CarryShare)?);
            rest = &rest[33..];
        }
        let (&count, mut rest) = rest.split_first().ok_or(Error::CarryTruncated)?;
        let mut entries = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            if rest.len() < 4 + 33 + 32 + 64 {
                return Err(Error::CarryTruncated);
            }
            let input = u32::from_be_bytes(rest[..4].try_into().expect("four bytes"));
            let pubshare = PublicKey::from_slice(&rest[4..37]).map_err(|_| Error::CarryShare)?;
            let mut sighash = [0u8; 32];
            sighash.copy_from_slice(&rest[37..69]);
            let secnonce = SecNonce::from_bytes(&rest[69..133]).map_err(|_| Error::CarryNonce)?;
            entries.push(CarryEntry {
                input,
                pubshare,
                sighash,
                secnonce,
            });
            rest = &rest[133..];
        }
        Ok((CarrySection { signers, entries }, rest))
    }
}

impl fmt::Debug for CarrySection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CarrySection(..)")
    }
}

impl Zeroize for CarrySection {
    fn zeroize(&mut self) {
        for entry in &mut self.entries {
            entry.zeroize();
        }
        self.entries.clear();
    }
}

impl Drop for CarrySection {
    fn drop(&mut self) {
        for entry in &mut self.entries {
            entry.zeroize();
        }
    }
}

/// A carry file whole: the secret section and the PSBT it belongs to.
pub struct Carry {
    /// The signer set and the nonces still to be used.
    pub section: CarrySection,
    /// The PSBT, binary, with its `osk` records on it.
    pub psbt: Vec<u8>,
}

impl Carry {
    /// Whether `bytes` are offered as a carry file.
    pub fn looks_like_carry(bytes: &[u8]) -> bool {
        bytes.starts_with(CARRY_MAGIC)
    }

    /// Reads a carry file.
    pub fn parse(bytes: &[u8]) -> Result<Carry, Error> {
        if bytes.len() < 5 || !bytes.starts_with(CARRY_MAGIC) || bytes[4] != CARRY_VERSION {
            return Err(Error::CarryMagic);
        }
        let (section, rest) = CarrySection::parse(&bytes[5..])?;
        Ok(Carry {
            section,
            psbt: rest.to_vec(),
        })
    }

    /// The whole file.
    pub fn to_bytes(&self) -> Vec<u8> {
        carry_bytes(&self.section, &self.psbt)
    }
}

/// The whole file from a section and a PSBT, moving neither: what the
/// Sign flow hands the shell while it still holds the section.
pub fn carry_bytes(section: &CarrySection, psbt: &[u8]) -> Vec<u8> {
    let body = section.to_bytes();
    let mut out = Vec::with_capacity(5 + body.len() + psbt.len());
    out.extend_from_slice(CARRY_MAGIC);
    out.push(CARRY_VERSION);
    out.extend_from_slice(&body);
    out.extend_from_slice(psbt);
    out
}

impl fmt::Debug for Carry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Carry(..)")
    }
}
