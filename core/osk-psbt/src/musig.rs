//! BIP-373's MuSig2 PSBT fields.
//!
//! `bitcoin` 0.32 has no type for any of them, so they live in the
//! `unknown` map of the input or output they belong to, which is exactly
//! where a field the crate has no type for survives a round trip. Four
//! fields:
//!
//! | Field | Map | Type | Key data | Value |
//! |---|---|---|---|---|
//! | `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` | input | `0x1a` | 33-byte aggregate | the participants, 33 bytes each |
//! | `PSBT_IN_MUSIG2_PUB_NONCE` | input | `0x1b` | participant ‖ aggregate, and an optional leaf hash | 66-byte public nonce |
//! | `PSBT_IN_MUSIG2_PARTIAL_SIG` | input | `0x1c` | the same | 32-byte partial signature |
//! | `PSBT_OUT_MUSIG2_PARTICIPANT_PUBKEYS` | output | `0x08` | 33-byte aggregate | the participants |
//!
//! A nonce or partial-signature key with the 32-byte tapleaf hash
//! appended is a script-path MuSig2 spend, which is not a wallet this
//! tree loads, and reading it is [`Error::ScriptPath`].
//!
//! Reading another signer's nonce or partial signature accepts any of
//! the three aggregate forms BIP-373's text and Bitcoin Core's code
//! between them produce — the root aggregate, the aggregate derived at
//! the sub-path, or the taproot output key. What this crate writes is
//! the output key in compressed form, which is what Bitcoin Core 31.1
//! writes and reads (`docs/PLANNING.md` §16.100).

use alloc::vec::Vec;
use core::fmt;

use bitcoin::hashes::{Hash, HashEngine};
use bitcoin::psbt::{Input, Output, raw};
use bitcoin::secp256k1::PublicKey;
use osk_bip::musig::{PartialSig, PubNonce};

/// `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS`.
pub const IN_PARTICIPANT_PUBKEYS: u8 = 0x1a;
/// `PSBT_IN_MUSIG2_PUB_NONCE`.
pub const IN_PUB_NONCE: u8 = 0x1b;
/// `PSBT_IN_MUSIG2_PARTIAL_SIG`: this is the per-participant partial
/// signature, not the final one, which is written as `tap_key_sig`.
pub const IN_PARTIAL_SIG: u8 = 0x1c;
/// `PSBT_OUT_MUSIG2_PARTICIPANT_PUBKEYS`.
pub const OUT_PARTICIPANT_PUBKEYS: u8 = 0x08;

/// Why a BIP-373 field could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The participants field's key is not a 33-byte aggregate key, or
    /// its value is not a whole number of 33-byte keys, or it lists
    /// none.
    Participants,
    /// A nonce or partial-signature key is not a participant key
    /// followed by an aggregate key.
    EntryKey,
    /// A public nonce is not 66 bytes, or either half is not a point.
    Nonce,
    /// A partial signature is not a 32-byte scalar below the curve
    /// order.
    Signature,
    /// The key carries a 32-byte tapleaf hash: a script-path MuSig2
    /// spend, which this tree does not load.
    ScriptPath,
}

impl Error {
    /// The reason as the blocked-input warning states it.
    pub fn reason(self) -> &'static str {
        match self {
            Error::Participants => "the MuSig2 participants field is malformed",
            Error::EntryKey => "a MuSig2 nonce or partial signature names no participant",
            Error::Nonce => "a MuSig2 public nonce is malformed",
            Error::Signature => "a MuSig2 partial signature is malformed",
            Error::ScriptPath => "the MuSig2 field names a script path",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl core::error::Error for Error {}

/// One `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` entry: the aggregate the
/// field is keyed by, and the participants in aggregation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participants {
    /// The key the field is keyed by, which is `KeyAgg` of the sorted
    /// participants before any derivation.
    pub aggregate: PublicKey,
    /// The participants, in the order the field lists them.
    pub keys: Vec<PublicKey>,
}

/// One participant's public nonce or partial signature, and the
/// aggregate key its entry names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<T> {
    /// Whose it is.
    pub participant: PublicKey,
    /// The aggregate key the entry is keyed by, in whichever of the
    /// three forms the writer used.
    pub aggregate: PublicKey,
    /// The nonce or the partial signature.
    pub value: T,
}

fn key_of(bytes: &[u8]) -> Result<PublicKey, Error> {
    PublicKey::from_slice(bytes).map_err(|_| Error::Participants)
}

fn read_participants(map: &[(&raw::Key, &Vec<u8>)]) -> Result<Vec<Participants>, Error> {
    let mut out = Vec::new();
    for (key, value) in map {
        let aggregate = key_of(&key.key)?;
        if value.is_empty() || !value.len().is_multiple_of(33) {
            return Err(Error::Participants);
        }
        let mut keys = Vec::with_capacity(value.len() / 33);
        for chunk in value.chunks(33) {
            keys.push(key_of(chunk)?);
        }
        out.push(Participants { aggregate, keys });
    }
    Ok(out)
}

fn entries_of(key: &raw::Key) -> Result<(PublicKey, PublicKey), Error> {
    match key.key.len() {
        66 => Ok((
            PublicKey::from_slice(&key.key[..33]).map_err(|_| Error::EntryKey)?,
            PublicKey::from_slice(&key.key[33..]).map_err(|_| Error::EntryKey)?,
        )),
        98 => Err(Error::ScriptPath),
        _ => Err(Error::EntryKey),
    }
}

fn of_type(
    unknown: &alloc::collections::BTreeMap<raw::Key, Vec<u8>>,
    type_value: u8,
) -> Vec<(&raw::Key, &Vec<u8>)> {
    unknown
        .iter()
        .filter(|(k, _)| k.type_value == type_value)
        .collect()
}

/// The `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` entries of an input.
pub fn input_participants(input: &Input) -> Result<Vec<Participants>, Error> {
    read_participants(&of_type(&input.unknown, IN_PARTICIPANT_PUBKEYS))
}

/// The `PSBT_OUT_MUSIG2_PARTICIPANT_PUBKEYS` entries of an output.
pub fn output_participants(output: &Output) -> Result<Vec<Participants>, Error> {
    read_participants(&of_type(&output.unknown, OUT_PARTICIPANT_PUBKEYS))
}

/// Every `PSBT_IN_MUSIG2_PUB_NONCE` an input carries.
pub fn pub_nonces(input: &Input) -> Result<Vec<Entry<PubNonce>>, Error> {
    let mut out = Vec::new();
    for (key, value) in of_type(&input.unknown, IN_PUB_NONCE) {
        let (participant, aggregate) = entries_of(key)?;
        out.push(Entry {
            participant,
            aggregate,
            value: PubNonce::from_bytes(value).map_err(|_| Error::Nonce)?,
        });
    }
    Ok(out)
}

/// Every `PSBT_IN_MUSIG2_PARTIAL_SIG` an input carries.
pub fn partial_sigs(input: &Input) -> Result<Vec<Entry<PartialSig>>, Error> {
    let mut out = Vec::new();
    for (key, value) in of_type(&input.unknown, IN_PARTIAL_SIG) {
        let (participant, aggregate) = entries_of(key)?;
        out.push(Entry {
            participant,
            aggregate,
            value: PartialSig::from_bytes(value).map_err(|_| Error::Signature)?,
        });
    }
    Ok(out)
}

fn entry_key(type_value: u8, participant: &PublicKey, aggregate: &PublicKey) -> raw::Key {
    let mut key = Vec::with_capacity(66);
    key.extend_from_slice(&participant.serialize());
    key.extend_from_slice(&aggregate.serialize());
    raw::Key { type_value, key }
}

/// Writes this device's public nonce for `participant`, keyed by
/// `aggregate`.
pub fn write_pub_nonce(
    input: &mut Input,
    participant: &PublicKey,
    aggregate: &PublicKey,
    nonce: &PubNonce,
) {
    input.unknown.insert(
        entry_key(IN_PUB_NONCE, participant, aggregate),
        nonce.serialize().to_vec(),
    );
}

/// Writes this device's partial signature for `participant`, keyed by
/// `aggregate`.
pub fn write_partial_sig(
    input: &mut Input,
    participant: &PublicKey,
    aggregate: &PublicKey,
    sig: &PartialSig,
) {
    input.unknown.insert(
        entry_key(IN_PARTIAL_SIG, participant, aggregate),
        sig.serialize().to_vec(),
    );
}

/// Removes every `PSBT_IN_MUSIG2_PARTIAL_SIG` an input carries.
///
/// A partial signature is made against one aggregate nonce, so a round
/// that puts a different nonce of this device's on the input leaves
/// every signature already there unverifiable (§16.100).
pub fn clear_partial_sigs(input: &mut Input) {
    input.unknown.retain(|k, _| k.type_value != IN_PARTIAL_SIG);
}

/// Writes a `PSBT_IN_MUSIG2_PARTICIPANT_PUBKEYS` entry.
pub fn write_input_participants(input: &mut Input, participants: &Participants) {
    let mut value = Vec::with_capacity(participants.keys.len() * 33);
    for key in &participants.keys {
        value.extend_from_slice(&key.serialize());
    }
    input.unknown.insert(
        raw::Key {
            type_value: IN_PARTICIPANT_PUBKEYS,
            key: participants.aggregate.serialize().to_vec(),
        },
        value,
    );
}

// ---------------------------------------------------------------------
// The session (docs/PLANNING.md §16.100)
// ---------------------------------------------------------------------

/// The tag the `rand'` of one [`nonce_gen`](osk_bip::musig::nonce_gen)
/// call is hashed under, so that one session seed gives a different
/// value for every input and participant, which is what BIP-327 asks
/// for.
const RAND_TAG: &[u8] = b"OpenSigner/musig-rand";

/// BIP-340's tagged hash.
pub(crate) fn tagged(tag: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let prefix = bitcoin::hashes::sha256::Hash::hash(tag).to_byte_array();
    let mut engine = bitcoin::hashes::sha256::Hash::engine();
    engine.input(&prefix);
    engine.input(&prefix);
    for part in parts {
        engine.input(part);
    }
    bitcoin::hashes::sha256::Hash::from_engine(engine).to_byte_array()
}

/// One open MuSig2 signing session: the transaction it is for, and the
/// secret nonce this device drew for every input and participant of it.
///
/// The session is memory and nothing else (§16.47): it is not `Clone`,
/// nothing serialises it, signing takes each secret nonce out by value
/// so none can sign twice, and dropping it wipes the seed and every
/// nonce still in it.
pub struct MusigSession {
    txid: bitcoin::Txid,
    seed: [u8; 32],
    held: Vec<(usize, PublicKey, PubNonce, osk_bip::musig::SecNonce)>,
}

impl MusigSession {
    /// An empty session for the unsigned transaction `txid`, whose
    /// nonces are drawn from `seed`.
    pub fn new(txid: bitcoin::Txid, seed: [u8; 32]) -> Self {
        MusigSession {
            txid,
            seed,
            held: Vec::new(),
        }
    }

    /// The unsigned transaction this session is for.
    pub fn txid(&self) -> bitcoin::Txid {
        self.txid
    }

    /// How many secret nonces are still in it.
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether every nonce has been signed away.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    /// Whether this session holds the secret nonce for `participant` on
    /// input `input`.
    pub fn holds(&self, input: usize, participant: &PublicKey) -> bool {
        self.held
            .iter()
            .any(|(i, pk, _, _)| *i == input && pk == participant)
    }

    /// What the inspector is told about the session: the transaction and
    /// the pairs it holds, and nothing secret.
    pub fn view(&self) -> MusigSessionView {
        MusigSessionView {
            txid: self.txid,
            held: self.held.iter().map(|(i, pk, _, _)| (*i, *pk)).collect(),
        }
    }

    /// The `rand'` one `NonceGen` call uses: the seed with the input
    /// index and the participant key hashed into it, so no two calls of
    /// a session share one.
    pub(crate) fn rand(&self, input: usize, participant: &PublicKey) -> [u8; 32] {
        tagged(
            RAND_TAG,
            &[
                &self.seed,
                &(input as u32).to_le_bytes(),
                &participant.serialize(),
            ],
        )
    }

    /// Keeps the secret nonce drawn for `participant` on `input`, with
    /// the public nonce that was written beside it.
    pub(crate) fn keep(
        &mut self,
        input: usize,
        participant: PublicKey,
        public: PubNonce,
        secret: osk_bip::musig::SecNonce,
    ) {
        self.held
            .retain(|(i, pk, _, _)| *i != input || *pk != participant);
        self.held.push((input, participant, public, secret));
    }

    /// Takes the secret nonce for `participant` on `input` out of the
    /// session, with the public nonce it belongs to. It is gone from the
    /// session whether or not the signature that follows is written.
    pub(crate) fn take(
        &mut self,
        input: usize,
        participant: &PublicKey,
    ) -> Option<(PubNonce, osk_bip::musig::SecNonce)> {
        let at = self
            .held
            .iter()
            .position(|(i, pk, _, _)| *i == input && pk == participant)?;
        let (_, _, public, secret) = self.held.remove(at);
        Some((public, secret))
    }
}

impl core::fmt::Debug for MusigSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MusigSession(..)")
    }
}

impl Drop for MusigSession {
    fn drop(&mut self) {
        use osk_crypto::Zeroize;
        self.seed.zeroize();
    }
}

/// What a session holds, as everything but signing sees it: the
/// transaction it is for and the inputs and participants it covers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusigSessionView {
    /// The unsigned transaction's id.
    pub txid: bitcoin::Txid,
    /// The `(input index, participant key)` pairs a secret nonce is held
    /// for.
    pub held: Vec<(usize, PublicKey)>,
}

impl MusigSessionView {
    /// Whether the session is for `txid` and holds `participant`'s
    /// secret nonce on input `input`.
    pub fn holds(&self, txid: bitcoin::Txid, input: usize, participant: &PublicKey) -> bool {
        self.txid == txid
            && self
                .held
                .iter()
                .any(|(i, pk)| *i == input && pk == participant)
    }
}
