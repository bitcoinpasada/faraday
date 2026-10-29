//! Silent payments in a PSBT (BIP-375): the signer's part.
//!
//! A PSBT that pays a silent payment address carries the address's two
//! keys in the output (`PSBT_OUT_SP_V0_INFO`) and no script, because
//! the script depends on every input. Each signer adds, for every scan
//! key the outputs name, an ECDH share `a·B_scan` for each input it
//! holds the key to, with a BIP-374 proof that the share came from that
//! input's key. Once every eligible input has a share (or one signer
//! holding them all has added a global one), anyone can compute the
//! outputs' scripts, and every signer checks them before signing.
//!
//! - [`check`] reads a [`PsbtV2`] against BIP-375 and returns what its
//!   silent payment outputs are and which scripts the shares give. It
//!   refuses a PSBT whose fields are malformed, whose shares' proofs do
//!   not verify, that spends a SegWit version above 1, that asks for a
//!   sighash other than `SIGHASH_ALL`, or whose stated script differs
//!   from the one the shares give.
//! - [`add_shares`] (from a [`MasterKey`], by the PSBT's own key
//!   origins) and [`add_shares_for_keys`] (from keys in hand) add this
//!   signer's shares and proofs, and set every script the shares now
//!   determine, with the inputs and outputs no longer modifiable.
//! - [`signable`] is the version 0 PSBT [`crate::sign`] signs, given
//!   only once every output has its script; [`PsbtV2::merge_signed`]
//!   brings the signatures back, and [`check`] then holds them to
//!   `SIGHASH_ALL`.
//!
//! What it refuses rather than assumes: an eligible input whose public
//! key the PSBT does not state (BIP-375 asks the updater for its
//! `PSBT_IN_BIP32_DERIVATION`), a P2SH input with no redeem script, a
//! previous transaction that is not the one spent. The share sum of a
//! global share is checked against the sum of every eligible input's
//! key, so such an input stops the check rather than being left out of
//! the sum.
//!
//! The order `k` runs in among outputs sharing a scan key: BIP-375's
//! text sorts their spend keys, while its published vectors and
//! reference validator use output order. Both give payments BIP-352's
//! receiver finds; stated scripts in either order are taken, and the
//! scripts set here are in output order, which reproduces every vector
//! (`tools/vectors/bip375/README.md`).
//!
//! Every multiplication by a private key goes through
//! [`dleq::secret_mul`], which takes the same time for every scalar;
//! each private key used is erased before its input is done.

use alloc::vec::Vec;
use core::fmt;

use bitcoin::bip32::{ChildNumber, DerivationPath};
use bitcoin::hashes::{Hash, hash160};
use bitcoin::key::{Keypair, TapTweak};
use bitcoin::secp256k1::{All, Parity, PublicKey, Scalar, Secp256k1, SecretKey, XOnlyPublicKey};
use bitcoin::taproot::TapNodeHash;
use bitcoin::{Transaction, TxOut, Witness, consensus};
use faraday_bip::dleq;
use faraday_bip::keys::MasterKey;
use faraday_bip::silent::{self as bip352, ScanInput};
use faraday_crypto::sha256;
use faraday_crypto::work::{Compute, Work};

use crate::v2::{self, PsbtV2};

/// `PSBT_GLOBAL_SP_ECDH_SHARE`: key data a scan key, value `a·B_scan`
/// for the sum `a` of every eligible input's key.
pub const GLOBAL_SP_ECDH_SHARE: u8 = 0x07;
/// `PSBT_GLOBAL_SP_DLEQ`: key data a scan key, value the proof of the
/// global share.
pub const GLOBAL_SP_DLEQ: u8 = 0x08;
/// `PSBT_IN_SP_ECDH_SHARE`: key data a scan key, value `a·B_scan` for
/// this input's key `a`.
pub const IN_SP_ECDH_SHARE: u8 = 0x1d;
/// `PSBT_IN_SP_DLEQ`: key data a scan key, value the proof of the
/// input's share.
pub const IN_SP_DLEQ: u8 = 0x1e;
/// `PSBT_OUT_SP_V0_INFO`: the address's scan key, then its spend key.
pub const OUT_SP_V0_INFO: u8 = 0x09;
/// `PSBT_OUT_SP_V0_LABEL`: the label, a little-endian `u32`.
pub const OUT_SP_V0_LABEL: u8 = 0x0a;

/// BIP-341's `H`, the internal key of a taproot output with no key
/// path. An input spending one contributes no key.
const NUMS: [u8; 32] = [
    0x50, 0x92, 0x9b, 0x74, 0xc1, 0xa0, 0x49, 0x54, 0xb7, 0x8b, 0x4b, 0x60, 0x35, 0xe9, 0x7a, 0x5e,
    0x07, 0x8a, 0x5a, 0x0f, 0x28, 0xec, 0x96, 0xd5, 0x47, 0xbf, 0xee, 0x9a, 0xce, 0x80, 0x3a, 0xc0,
];

/// Why a silent payment PSBT is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// An output has neither a script nor a silent payment address.
    NoDestination {
        /// Output index.
        output: usize,
    },
    /// An output has a label but no silent payment address.
    LabelWithoutAddress {
        /// Output index.
        output: usize,
    },
    /// A BIP-375 field has the wrong length or holds no point; the text
    /// names it.
    Field(&'static str),
    /// A silent payment output has its script and the transaction is
    /// still marked modifiable.
    Modifiable,
    /// An input spends a SegWit version above 1, which BIP-352 cannot
    /// pay alongside.
    FutureSegwit {
        /// Input index.
        input: usize,
    },
    /// An input asks for, or carries a signature under, a sighash
    /// other than `SIGHASH_ALL`.
    Sighash {
        /// Input index.
        input: usize,
    },
    /// What an input contributes cannot be worked out from the PSBT;
    /// the text says what is missing.
    Unverifiable {
        /// Input index.
        input: usize,
        /// What is missing or wrong.
        reason: &'static str,
    },
    /// A share has no proof beside it: `None` is the global share.
    MissingProof {
        /// Input index, or `None` for the global share.
        input: Option<usize>,
    },
    /// A share's proof does not verify: `None` is the global share.
    BadProof {
        /// Input index, or `None` for the global share.
        input: Option<usize>,
    },
    /// A silent payment output has a script and the shares to check it
    /// against are not all there.
    NoShare {
        /// Output index.
        output: usize,
    },
    /// A silent payment output's script is not the one the shares give.
    ScriptMismatch {
        /// Output index.
        output: usize,
    },
    /// The eligible inputs' keys sum to the point at infinity, or the
    /// input hash is not a scalar: BIP-352 cannot pay this transaction.
    InputHash,
    /// A key was given for an input that contributes no key.
    NotEligible {
        /// Input index.
        input: usize,
    },
    /// The key given or derived is not the input's key.
    KeyMismatch {
        /// Input index.
        input: usize,
    },
    /// The input already carries a different share for a scan key.
    ShareConflict {
        /// Input index.
        input: usize,
    },
    /// A silent payment output has no script yet, so nothing may be
    /// signed.
    NotComplete {
        /// Output index.
        output: usize,
    },
    /// The version 2 container refused.
    V2(v2::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}") // not shown: a developer's text; the app takes no such PSBT
    }
}

impl core::error::Error for Error {}

impl From<v2::Error> for Error {
    fn from(e: v2::Error) -> Self {
        Error::V2(e)
    }
}

/// One silent payment output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpOutput {
    /// Which output of the transaction it is.
    pub index: usize,
    /// The address's scan key.
    pub scan: PublicKey,
    /// The address's spend key, with its label already added.
    pub spend: PublicKey,
    /// The label the PSBT states for it, for change.
    pub label: Option<u32>,
    /// Its `k` among the outputs sharing its scan key.
    pub k: u32,
    /// The script the shares give, once they are all there.
    pub script: Option<[u8; 34]>,
    /// Whether the PSBT states the script already (then it is this one).
    pub stated: bool,
}

/// What [`check`] found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// The silent payment outputs, in output order.
    pub outputs: Vec<SpOutput>,
}

impl Report {
    /// Whether the PSBT pays any silent payment address.
    pub fn has_silent(&self) -> bool {
        !self.outputs.is_empty()
    }

    /// Whether every silent payment output's script is determined.
    pub fn complete(&self) -> bool {
        self.outputs.iter().all(|o| o.script.is_some())
    }
}

/// What one input gives the shared secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Contribution {
    /// Nothing: a script type BIP-352 does not read, a taproot output
    /// with no key path, an uncompressed key.
    Nothing,
    /// This public key, as BIP-352 takes it (even `y` for taproot).
    Key(PublicKey),
    /// Eligible or not, the PSBT does not say enough to tell, or to know
    /// its key.
    Unknown(&'static str),
}

/// BIP-340's tagged hash.
fn tagged(tag: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let prefix = sha256(tag);
    let mut data = Vec::with_capacity(64 + parts.iter().map(|p| p.len()).sum::<usize>());
    data.extend_from_slice(&prefix);
    data.extend_from_slice(&prefix);
    for part in parts {
        data.extend_from_slice(part);
    }
    sha256(&data)
}

fn is_p2tr(s: &[u8]) -> bool {
    s.len() == 34 && s[0] == 0x51 && s[1] == 0x20
}
fn is_p2wpkh(s: &[u8]) -> bool {
    s.len() == 22 && s[0] == 0x00 && s[1] == 0x14
}
fn is_p2sh(s: &[u8]) -> bool {
    s.len() == 23 && s[0] == 0xa9 && s[1] == 0x14 && s[22] == 0x87
}
fn is_p2pkh(s: &[u8]) -> bool {
    s.len() == 25 && s[..3] == [0x76, 0xa9, 0x14] && s[23] == 0x88 && s[24] == 0xac
}

/// The output input `i` spends.
fn prevout(psbt: &PsbtV2, i: usize) -> Result<TxOut, Error> {
    let map = &psbt.inputs[i];
    if let Some(bytes) = map.get_type(v2::IN_WITNESS_UTXO) {
        return consensus::deserialize::<TxOut>(bytes).map_err(|_| Error::Unverifiable {
            input: i,
            // not shown: a developer's text; the app takes no silent-payment PSBT
            reason: "the witness UTXO does not read",
        });
    }
    if let Some(bytes) = map.get_type(v2::IN_NON_WITNESS_UTXO) {
        let tx: Transaction = crate::transaction::decode(bytes).ok_or(Error::Unverifiable {
            input: i,
            // not shown: a developer's text; the app takes no silent-payment PSBT
            reason: "the previous transaction does not read",
        })?;
        let point = psbt.outpoint_bytes(i);
        if tx.compute_txid().to_byte_array()[..] != point[..32] {
            return Err(Error::Unverifiable {
                input: i,
                // not shown: a developer's text; the app takes no silent-payment PSBT
                reason: "the previous transaction is not the one spent",
            });
        }
        let vout = u32::from_le_bytes(point[32..].try_into().expect("four bytes"));
        return tx
            .output
            .get(vout as usize)
            .cloned()
            .ok_or(Error::Unverifiable {
                input: i,
                // not shown: a developer's text; the app takes no silent-payment PSBT
                reason: "the previous transaction has no such output",
            });
    }
    Err(Error::Unverifiable {
        input: i,
        // not shown: a developer's text; the app takes no silent-payment PSBT
        reason: "no previous output",
    })
}

/// The compressed key the input states (a key origin or a partial
/// signature's key) whose hash is `hash`; nothing for an uncompressed
/// one, which BIP-352 takes no key from.
///
/// A stated key is taken only when the input's script commits to it.
/// BIP-375's own reference validator takes the first key origin as it
/// is, and several of its published vectors pair a P2WPKH output with a
/// key it does not commit to; a coordinator could state any key there,
/// with a share and a proof that verify under it, and the scripts would
/// pay an output its recipient never finds. Faraday refuses that input
/// instead (`tools/vectors/bip375/README.md`).
fn key_for_hash(psbt: &PsbtV2, i: usize, hash: &[u8]) -> Contribution {
    let map = &psbt.inputs[i];
    let mut uncompressed = false;
    let mut stated = false;
    for (key, _) in map
        .all_of(v2::IN_BIP32_DERIVATION)
        .chain(map.all_of(v2::IN_PARTIAL_SIG))
    {
        stated = true;
        if hash160::Hash::hash(key).to_byte_array()[..] != *hash {
            continue;
        }
        match key.len() {
            33 => {
                if let Ok(k) = PublicKey::from_slice(key) {
                    return Contribution::Key(k);
                }
            }
            65 => uncompressed = true,
            _ => {}
        }
    }
    if uncompressed {
        Contribution::Nothing
    } else if stated {
        Contribution::Unknown(
            // not shown: a developer's text; the app takes no silent-payment PSBT
            "no public key the PSBT states is the one the input's script commits to",
        )
    } else {
        // not shown: a developer's text; the app takes no silent-payment PSBT
        Contribution::Unknown("the PSBT states no public key for the input")
    }
}

/// What input `i`, spending `spk`, gives the shared secret.
fn contribution(psbt: &PsbtV2, i: usize, spk: &[u8]) -> Result<Contribution, Error> {
    let map = &psbt.inputs[i];
    let final_sig = map.get_type(v2::IN_FINAL_SCRIPTSIG);
    let final_witness = map.get_type(v2::IN_FINAL_SCRIPTWITNESS);
    if final_sig.is_some() || final_witness.is_some() {
        // A finished input: BIP-352's own reading of the spend.
        let witness: Witness = match final_witness {
            Some(bytes) => consensus::deserialize(bytes).map_err(|_| Error::Unverifiable {
                input: i,
                // not shown: a developer's text; the app takes no silent-payment PSBT
                reason: "the final witness does not read",
            })?,
            None => Witness::new(),
        };
        let items: Vec<&[u8]> = witness.iter().collect();
        let scan = ScanInput {
            outpoint: psbt.outpoint_bytes(i),
            script_sig: final_sig.unwrap_or(&[]),
            witness: &items,
            prevout: spk,
        };
        return Ok(match bip352::input_public_key(&scan) {
            Some(k) => Contribution::Key(k),
            None => Contribution::Nothing,
        });
    }
    if is_p2tr(spk) {
        if map.get_type(v2::IN_TAP_INTERNAL_KEY) == Some(&NUMS[..]) {
            return Ok(Contribution::Nothing);
        }
        return Ok(match XOnlyPublicKey::from_slice(&spk[2..]) {
            Ok(x) => Contribution::Key(PublicKey::from_x_only_public_key(x, Parity::Even)),
            Err(_) => Contribution::Nothing,
        });
    }
    if is_p2wpkh(spk) {
        return Ok(key_for_hash(psbt, i, &spk[2..22]));
    }
    if is_p2pkh(spk) {
        return Ok(key_for_hash(psbt, i, &spk[3..23]));
    }
    if is_p2sh(spk) {
        let Some(redeem) = map.get_type(v2::IN_REDEEM_SCRIPT) else {
            // not shown: a developer's text; the app takes no silent-payment PSBT
            return Ok(Contribution::Unknown("a P2SH input with no redeem script"));
        };
        if hash160::Hash::hash(redeem).to_byte_array()[..] != spk[2..22] {
            return Err(Error::Unverifiable {
                input: i,
                // not shown: a developer's text; the app takes no silent-payment PSBT
                reason: "the redeem script is not the one the input spends",
            });
        }
        if is_p2wpkh(redeem) {
            return Ok(key_for_hash(psbt, i, &redeem[2..22]));
        }
    }
    Ok(Contribution::Nothing)
}

/// A 33-byte compressed point, or the field named.
fn point(bytes: &[u8], name: &'static str) -> Result<PublicKey, Error> {
    if bytes.len() != 33 {
        return Err(Error::Field(name));
    }
    PublicKey::from_slice(bytes).map_err(|_| Error::Field(name))
}

/// A 64-byte proof, or the field named.
fn proof(bytes: &[u8], name: &'static str) -> Result<dleq::Proof, Error> {
    bytes.try_into().map_err(|_| Error::Field(name))
}

/// The silent payment outputs as the PSBT states them, with BIP-375's
/// structure checked: every output has a destination, a label has an
/// address, every field has its length and its points are points, and
/// a stated script means the transaction is no longer modifiable.
fn structure(psbt: &PsbtV2) -> Result<Vec<SpOutput>, Error> {
    let mut outputs = Vec::new();
    for (index, map) in psbt.outputs.iter().enumerate() {
        let script = map.get_type(v2::OUT_SCRIPT).filter(|s| !s.is_empty());
        let info = map.get_type(OUT_SP_V0_INFO);
        let label = map.get_type(OUT_SP_V0_LABEL);
        let Some(info) = info else {
            if label.is_some() {
                return Err(Error::LabelWithoutAddress { output: index });
            }
            if script.is_none() {
                return Err(Error::NoDestination { output: index });
            }
            continue;
        };
        if info.len() != 66 {
            return Err(Error::Field("PSBT_OUT_SP_V0_INFO"));
        }
        let label = match label {
            Some(l) => Some(u32::from_le_bytes(
                l.try_into()
                    .map_err(|_| Error::Field("PSBT_OUT_SP_V0_LABEL"))?,
            )),
            None => None,
        };
        outputs.push(SpOutput {
            index,
            scan: point(&info[..33], "PSBT_OUT_SP_V0_INFO")?,
            spend: point(&info[33..], "PSBT_OUT_SP_V0_INFO")?,
            label,
            k: 0,
            script: None,
            stated: script.is_some(),
        });
    }
    for (key, value) in psbt.global.all_of(GLOBAL_SP_ECDH_SHARE) {
        point(key, "PSBT_GLOBAL_SP_ECDH_SHARE")?;
        point(value, "PSBT_GLOBAL_SP_ECDH_SHARE")?;
    }
    for (key, value) in psbt.global.all_of(GLOBAL_SP_DLEQ) {
        point(key, "PSBT_GLOBAL_SP_DLEQ")?;
        proof(value, "PSBT_GLOBAL_SP_DLEQ")?;
    }
    for map in &psbt.inputs {
        for (key, value) in map.all_of(IN_SP_ECDH_SHARE) {
            point(key, "PSBT_IN_SP_ECDH_SHARE")?;
            point(value, "PSBT_IN_SP_ECDH_SHARE")?;
        }
        for (key, value) in map.all_of(IN_SP_DLEQ) {
            point(key, "PSBT_IN_SP_DLEQ")?;
            proof(value, "PSBT_IN_SP_DLEQ")?;
        }
    }
    if outputs.iter().any(|o| o.stated)
        && psbt.global.get_type(v2::GLOBAL_TX_MODIFIABLE) != Some(&[0u8][..])
    {
        return Err(Error::Modifiable);
    }
    Ok(outputs)
}

/// Whether a signature's last byte names `SIGHASH_ALL`; a 64-byte
/// Schnorr signature names `SIGHASH_DEFAULT`, which commits to what ALL
/// does.
fn signs_all(sig: &[u8], schnorr: bool) -> bool {
    (schnorr && sig.len() == 64) || sig.last() == Some(&0x01)
}

/// BIP-375's input rules alone, which apply once a silent payment
/// output is present: no input spends a SegWit version above 1, and
/// every input asks for and signs with `SIGHASH_ALL` only. [`check`]
/// holds a PSBT to these and the rest.
pub fn check_inputs(psbt: &PsbtV2) -> Result<(), Error> {
    if psbt
        .outputs
        .iter()
        .any(|o| o.get_type(OUT_SP_V0_INFO).is_some())
    {
        inputs(psbt)?;
    }
    Ok(())
}

/// BIP-375's input rules once a silent payment output is present: no
/// SegWit version above 1, and `SIGHASH_ALL` only, asked for or used.
/// Returns each input's contribution.
fn inputs(psbt: &PsbtV2) -> Result<Vec<(Contribution, bool)>, Error> {
    let mut out = Vec::with_capacity(psbt.inputs.len());
    for (i, map) in psbt.inputs.iter().enumerate() {
        let spk = prevout(psbt, i)?.script_pubkey;
        let spk = spk.as_bytes();
        if bip352::is_future_segwit(spk) {
            return Err(Error::FutureSegwit { input: i });
        }
        if let Some(t) = map.get_type(v2::IN_SIGHASH_TYPE)
            && t != [1, 0, 0, 0]
        {
            return Err(Error::Sighash { input: i });
        }
        let ecdsa = map
            .all_of(v2::IN_PARTIAL_SIG)
            .all(|(_, s)| signs_all(s, false));
        let schnorr = map
            .all_of(v2::IN_TAP_KEY_SIG)
            .chain(map.all_of(v2::IN_TAP_SCRIPT_SIG))
            .all(|(_, s)| signs_all(s, true));
        if !ecdsa || !schnorr {
            return Err(Error::Sighash { input: i });
        }
        out.push((contribution(psbt, i, spk)?, is_p2tr(spk)));
    }
    Ok(out)
}

/// The sum of every eligible input's key, refusing an input whose
/// contribution cannot be told.
fn key_sum(contributions: &[(Contribution, bool)]) -> Result<PublicKey, Error> {
    let mut keys = Vec::new();
    for (i, (c, _)) in contributions.iter().enumerate() {
        match c {
            Contribution::Key(k) => keys.push(k),
            Contribution::Unknown(reason) => {
                return Err(Error::Unverifiable { input: i, reason });
            }
            Contribution::Nothing => {}
        }
    }
    PublicKey::combine_keys(&keys).map_err(|_| Error::InputHash)
}

/// The share sum for `scan`, with every share present verified: the
/// global share where there is one, else the sum of the inputs' shares
/// once every eligible input has one; `None` while one is missing.
fn share_for(
    secp: &Secp256k1<All>,
    psbt: &PsbtV2,
    contributions: &[(Contribution, bool)],
    scan: &PublicKey,
) -> Result<Option<PublicKey>, Error> {
    let mut key = alloc::vec![GLOBAL_SP_ECDH_SHARE];
    key.extend_from_slice(&scan.serialize());
    let global = match psbt.global.get(&key) {
        Some(share) => {
            let share = point(share, "PSBT_GLOBAL_SP_ECDH_SHARE")?;
            key[0] = GLOBAL_SP_DLEQ;
            let p = psbt
                .global
                .get(&key)
                .ok_or(Error::MissingProof { input: None })?;
            let p = proof(p, "PSBT_GLOBAL_SP_DLEQ")?;
            let sum = key_sum(contributions)?;
            if !dleq::verify_proof(secp, &sum, scan, &share, &p, None) {
                return Err(Error::BadProof { input: None });
            }
            Some(share)
        }
        None => None,
    };
    let mut shares = Vec::new();
    let mut missing = false;
    for (i, (c, _)) in contributions.iter().enumerate() {
        key[0] = IN_SP_ECDH_SHARE;
        let share = psbt.inputs[i].get(&key);
        match (c, share) {
            (Contribution::Nothing, _) => {}
            (Contribution::Unknown(reason), Some(_)) => {
                return Err(Error::Unverifiable { input: i, reason });
            }
            (_, None) => missing = true,
            (Contribution::Key(a), Some(share)) => {
                let share = point(share, "PSBT_IN_SP_ECDH_SHARE")?;
                key[0] = IN_SP_DLEQ;
                let p = psbt.inputs[i]
                    .get(&key)
                    .ok_or(Error::MissingProof { input: Some(i) })?;
                let p = proof(p, "PSBT_IN_SP_DLEQ")?;
                if !dleq::verify_proof(secp, a, scan, &share, &p, None) {
                    return Err(Error::BadProof { input: Some(i) });
                }
                shares.push(share);
            }
        }
    }
    if global.is_some() {
        return Ok(global);
    }
    if missing || shares.is_empty() {
        return Ok(None);
    }
    let refs: Vec<&PublicKey> = shares.iter().collect();
    PublicKey::combine_keys(&refs)
        .map(Some)
        .map_err(|_| Error::InputHash)
}

/// The P2TR scripts of one scan key's outputs, `k` running over them
/// in `order`: `B_spend + hash(shared ‖ k)·G` each.
fn group_scripts(
    secp: &Secp256k1<All>,
    shared: &[u8; 33],
    outputs: &[SpOutput],
    order: &[usize],
) -> Result<Vec<[u8; 34]>, Error> {
    let mut scripts = Vec::with_capacity(order.len());
    for (k, o) in order.iter().enumerate() {
        let t_k = tagged(
            b"BIP0352/SharedSecret",
            &[shared, &(k as u32).to_be_bytes()],
        );
        let t_k = Scalar::from_be_bytes(t_k).map_err(|_| Error::InputHash)?;
        let p_k = outputs[*o]
            .spend
            .add_exp_tweak(secp, &t_k)
            .map_err(|_| Error::InputHash)?;
        let mut script = [0u8; 34];
        script[0] = 0x51;
        script[1] = 0x20;
        script[2..].copy_from_slice(&p_k.x_only_public_key().0.serialize());
        scripts.push(script);
    }
    Ok(scripts)
}

/// The checks without the token, and each input's contribution.
fn check_inner(
    secp: &Secp256k1<All>,
    psbt: &PsbtV2,
) -> Result<(Report, Vec<(Contribution, bool)>), Error> {
    let mut outputs = structure(psbt)?;
    if outputs.is_empty() {
        return Ok((Report::default(), Vec::new()));
    }
    let contributions = inputs(psbt)?;
    let smallest = (0..psbt.inputs.len())
        .map(|i| psbt.outpoint_bytes(i))
        .min()
        .ok_or(Error::InputHash)?;
    // The outputs grouped by scan key, each group in output order.
    let mut scans: Vec<PublicKey> = Vec::new();
    for o in &outputs {
        if !scans.contains(&o.scan) {
            scans.push(o.scan);
        }
    }
    for scan in scans {
        let group: Vec<usize> = (0..outputs.len())
            .filter(|o| outputs[*o].scan == scan)
            .collect();
        let Some(share) = share_for(secp, psbt, &contributions, &scan)? else {
            if let Some(o) = group.iter().find(|o| outputs[**o].stated) {
                return Err(Error::NoShare {
                    output: outputs[*o].index,
                });
            }
            continue;
        };
        let sum = key_sum(&contributions)?;
        let input_hash = tagged(b"BIP0352/Inputs", &[&smallest, &sum.serialize()]);
        let input_hash = Scalar::from_be_bytes(input_hash).map_err(|_| Error::InputHash)?;
        let shared = share
            .mul_tweak(secp, &input_hash)
            .map_err(|_| Error::InputHash)?
            .serialize();
        // BIP-375's text gives `k` in the order of the spend keys sorted,
        // output order among equal ones; its published vectors and
        // reference validator give it in output order, and so do the
        // scripts set here. Both are payments BIP-352's receiver finds,
        // so a group whose stated scripts follow either is taken.
        let mut sorted = group.clone();
        sorted.sort_by_key(|o| (outputs[*o].spend.serialize(), outputs[*o].index));
        let mut chosen = None;
        let mut first_mismatch = None;
        for order in [&group, &sorted] {
            let scripts = group_scripts(secp, &shared, &outputs, order)?;
            let mismatch = order
                .iter()
                .zip(scripts.iter())
                .find(|(o, script)| {
                    outputs[**o].stated
                        && psbt.output_script(outputs[**o].index) != Some(&script[..])
                })
                .map(|(o, _)| outputs[*o].index);
            match mismatch {
                None => {
                    chosen = Some((order.clone(), scripts));
                    break;
                }
                Some(m) => {
                    first_mismatch.get_or_insert(m);
                }
            }
        }
        let Some((order, scripts)) = chosen else {
            return Err(Error::ScriptMismatch {
                output: first_mismatch.expect("a mismatch was found"),
            });
        };
        for (k, (o, script)) in order.iter().zip(scripts).enumerate() {
            outputs[*o].k = k as u32;
            outputs[*o].script = Some(script);
        }
    }
    Ok((Report { outputs }, contributions))
}

/// Costs a verification of every share's proof (four elliptic-curve
/// multiplications each) and the outputs' scripts: reads `psbt` against
/// BIP-375 and returns its silent payment outputs, with the script each
/// has once its scan key's shares are all there.
pub fn check(c: &Compute, secp: &Secp256k1<All>, psbt: &PsbtV2) -> Result<Report, Error> {
    c.count(Work::PsbtInspect);
    check_inner(secp, psbt).map(|(report, _)| report)
}

/// Sets every script the shares determine that the PSBT does not state,
/// and marks the transaction no longer modifiable when it sets one.
fn fill_scripts(secp: &Secp256k1<All>, psbt: &mut PsbtV2) -> Result<Report, Error> {
    let (report, _) = check_inner(secp, psbt)?;
    let mut set = false;
    for out in &report.outputs {
        if let (Some(script), false) = (out.script, out.stated) {
            psbt.outputs[out.index].set(alloc::vec![v2::OUT_SCRIPT], script.to_vec());
            set = true;
        }
    }
    if !set {
        return Ok(report);
    }
    psbt.global
        .set(alloc::vec![v2::GLOBAL_TX_MODIFIABLE], alloc::vec![0]);
    check_inner(secp, psbt).map(|(report, _)| report)
}

/// Costs what [`check`] costs: sets every silent payment script the
/// shares already determine, for a participant that adds no share.
pub fn compute_scripts(
    c: &Compute,
    secp: &Secp256k1<All>,
    psbt: &mut PsbtV2,
) -> Result<Report, Error> {
    c.count(Work::PsbtInspect);
    fill_scripts(secp, psbt)
}

/// Adds input `i`'s share and proof for every scan key, from its key
/// `key` as it signs (BIP-352 negates a taproot key whose point has
/// odd `y`). An input that already has the same share is left as it
/// is.
fn add_input(
    secp: &Secp256k1<All>,
    psbt: &mut PsbtV2,
    contributions: &[(Contribution, bool)],
    scans: &[PublicKey],
    i: usize,
    key: &SecretKey,
    aux: &[u8; 32],
) -> Result<usize, Error> {
    let (contribution, taproot) = contributions
        .get(i)
        .ok_or(Error::NotEligible { input: i })?;
    let a_point = match contribution {
        Contribution::Key(k) => *k,
        Contribution::Nothing => return Err(Error::NotEligible { input: i }),
        Contribution::Unknown(reason) => return Err(Error::Unverifiable { input: i, reason }),
    };
    let ours = key.public_key(secp);
    let mut a = *key;
    if *taproot {
        let (x, parity) = ours.x_only_public_key();
        if x != a_point.x_only_public_key().0 {
            return Err(Error::KeyMismatch { input: i });
        }
        if parity == Parity::Odd {
            a = a.negate();
        }
    } else if ours != a_point {
        return Err(Error::KeyMismatch { input: i });
    }
    let mut added = 0;
    let mut result = Ok(());
    for scan in scans {
        let share = match dleq::secret_mul(scan, &a) {
            Ok(share) => share,
            Err(_) => {
                result = Err(Error::InputHash);
                break;
            }
        };
        let mut key = alloc::vec![IN_SP_ECDH_SHARE];
        key.extend_from_slice(&scan.serialize());
        match psbt.inputs[i].get(&key) {
            Some(existing) if existing == share.serialize() => continue,
            Some(_) => {
                result = Err(Error::ShareConflict { input: i });
                break;
            }
            None => {}
        }
        let proof = match dleq::generate_proof(secp, &a, scan, aux, None) {
            Ok(proof) => proof,
            Err(_) => {
                result = Err(Error::InputHash);
                break;
            }
        };
        psbt.inputs[i].set(key.clone(), share.serialize().to_vec());
        key[0] = IN_SP_DLEQ;
        psbt.inputs[i].set(key, proof.to_vec());
        added += 1;
    }
    a.non_secure_erase();
    result.map(|()| added)
}

/// The scan keys the outputs name, each once.
fn scan_keys(report: &Report) -> Vec<PublicKey> {
    let mut scans: Vec<PublicKey> = Vec::new();
    for o in &report.outputs {
        if !scans.contains(&o.scan) {
            scans.push(o.scan);
        }
    }
    scans
}

/// Costs an elliptic-curve multiplication and a proof (eight more) per
/// input and scan key, and what [`check`] costs: adds the share and
/// proof of each input in `keys` (its index and the private key that
/// spends it, untweaked for P2WPKH and P2PKH, the output key's for
/// taproot), then sets every script the shares now determine.
///
/// `aux` is BIP-374's auxiliary randomness. Refuses, before adding
/// anything, a PSBT [`check`] refuses; refuses a key that is not its
/// input's and an input that contributes no key.
pub fn add_shares_for_keys(
    c: &Compute,
    secp: &Secp256k1<All>,
    psbt: &mut PsbtV2,
    keys: &[(usize, &SecretKey)],
    aux: &[u8; 32],
) -> Result<Report, Error> {
    c.count(Work::Sign);
    let (report, contributions) = check_inner(secp, psbt)?;
    let scans = scan_keys(&report);
    if scans.is_empty() {
        return Ok(report);
    }
    for (i, key) in keys {
        add_input(secp, psbt, &contributions, &scans, *i, key, aux)?;
    }
    fill_scripts(secp, psbt)
}

/// A key origin's value: the fingerprint and the path.
fn origin(value: &[u8]) -> Option<([u8; 4], DerivationPath)> {
    if value.len() < 4 || !value.len().is_multiple_of(4) {
        return None;
    }
    let fp: [u8; 4] = value[..4].try_into().ok()?;
    let path: Vec<ChildNumber> = value[4..]
        .chunks(4)
        .map(|c| ChildNumber::from(u32::from_le_bytes(c.try_into().expect("four bytes"))))
        .collect();
    Some((fp, DerivationPath::from(path)))
}

/// A taproot key origin's value with no leaf hashes: the fingerprint
/// and the path of a key-path key.
fn tap_origin(value: &[u8]) -> Option<([u8; 4], DerivationPath)> {
    match value.split_first() {
        Some((0, rest)) => origin(rest),
        _ => None,
    }
}

/// Costs a key derivation per input `master` holds, and what
/// [`add_shares_for_keys`] costs: adds the share and proof of every
/// eligible input whose key origin names `master`, deriving the key at
/// the stated path and refusing one that is not the input's, then sets
/// every script the shares now determine.
pub fn add_shares(
    c: &Compute,
    psbt: &mut PsbtV2,
    master: &MasterKey,
    aux: &[u8; 32],
) -> Result<Report, Error> {
    c.count(Work::Sign);
    let secp = master.secp();
    let (report, contributions) = check_inner(secp, psbt)?;
    let scans = scan_keys(&report);
    if scans.is_empty() {
        return Ok(report);
    }
    let fp = master.fingerprint().0;
    for i in 0..psbt.inputs.len() {
        let (Contribution::Key(_), taproot) = contributions[i] else {
            continue;
        };
        let map = &psbt.inputs[i];
        if taproot {
            let claim = map
                .all_of(v2::IN_TAP_BIP32_DERIVATION)
                .find_map(|(k, v)| tap_origin(v).filter(|(f, _)| *f == fp).map(|o| (k, o)));
            let Some((internal, (_, path))) = claim else {
                continue;
            };
            let internal = XOnlyPublicKey::from_slice(internal)
                .map_err(|_| Error::Field("PSBT_IN_TAP_BIP32_DERIVATION"))?;
            if map
                .get_type(v2::IN_TAP_INTERNAL_KEY)
                .is_some_and(|k| k != internal.serialize())
            {
                return Err(Error::KeyMismatch { input: i });
            }
            let merkle = match map.get_type(v2::IN_TAP_MERKLE_ROOT) {
                Some(m) => Some(TapNodeHash::from_byte_array(
                    m.try_into()
                        .map_err(|_| Error::Field("PSBT_IN_TAP_MERKLE_ROOT"))?,
                )),
                None => None,
            };
            let derived = master.derive(c, &path);
            // `bitcoin` forces two secret copies here: the untweaked
            // keypair and the tweaked one. Both are erased before the
            // input is done.
            let mut keypair = Keypair::from_secret_key(secp, derived.secret_key());
            let mut tweaked = keypair.tap_tweak(secp, merkle).to_keypair();
            let matches = keypair.x_only_public_key().0 == internal;
            let mut key = tweaked.secret_key();
            keypair.non_secure_erase();
            tweaked.non_secure_erase();
            let added = if matches {
                add_input(secp, psbt, &contributions, &scans, i, &key, aux)
            } else {
                Err(Error::KeyMismatch { input: i })
            };
            key.non_secure_erase();
            added?;
        } else {
            let claim = map
                .all_of(v2::IN_BIP32_DERIVATION)
                .find_map(|(k, v)| origin(v).filter(|(f, _)| *f == fp).map(|o| (k, o)));
            let Some((stated, (_, path))) = claim else {
                continue;
            };
            let stated = point(stated, "PSBT_IN_BIP32_DERIVATION")?;
            let derived = master.derive(c, &path);
            if derived.secret_key().public_key(secp) != stated {
                return Err(Error::KeyMismatch { input: i });
            }
            add_input(
                secp,
                psbt,
                &contributions,
                &scans,
                i,
                derived.secret_key(),
                aux,
            )?;
        }
    }
    fill_scripts(secp, psbt)
}

/// Costs what [`check`] and a PSBT decode cost: the version 0 PSBT
/// [`crate::sign`] signs, given only when [`check`] passes and every
/// silent payment output has its script.
pub fn signable(c: &Compute, secp: &Secp256k1<All>, psbt: &PsbtV2) -> Result<crate::Psbt, Error> {
    let report = check(c, secp, psbt)?;
    if let Some(o) = report.outputs.iter().find(|o| !o.stated) {
        return Err(Error::NotComplete { output: o.index });
    }
    Ok(psbt.to_v0(c)?)
}
