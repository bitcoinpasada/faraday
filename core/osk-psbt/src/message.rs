//! Signed messages: BIP-137 (the legacy compact signature) and BIP-322
//! "simple" (a witness over the two virtual transactions the BIP builds).
//!
//! A message signature proves that whoever holds the key behind one
//! address was willing to sign that text. It moves no coins and spends
//! nothing: the BIP-322 transactions are never broadcastable, because
//! `to_spend` spends an output that cannot exist.
//!
//! Which format goes with which address (`docs/PLANNING.md` §9):
//!
//! | Address | Formats |
//! |---|---|
//! | p2pkh (`1…`) | BIP-137 |
//! | p2sh-p2wpkh (`3…`) | BIP-137 |
//! | p2wpkh (`bc1q…`) | BIP-137, BIP-322 |
//!
//! "Simple" carries a witness and no scriptSig, so a nested-segwit
//! address, which is spent through a redeem script, has no simple form.
//! | p2tr (`bc1p…`) | BIP-322 |
//!
//! BIP-137's header byte carries the recovery id and, in the ranges
//! Electrum and Trezor settled on, the address form: 27–34 for p2pkh,
//! 35–38 for p2sh-p2wpkh, 39–42 for p2wpkh. [`sign`] writes the range
//! for the script type it signed for. [`verify`] reads the recovery id
//! from the header and then asks the address itself which form to
//! rebuild, which is what Electrum and Sparrow do, so a signature from a
//! wallet that always writes 31–34 still verifies against the segwit
//! address it was made for.
//!
//! Both message formats sign with ECDSA, so both take the RFC 6979
//! nonce choice [`crate::Nonce`] carries (§16.38). BIP-137's compact
//! signature has no low-R signer in libsecp256k1, so the grind is done
//! here, in [`sign_recoverable_low_r`]. A BIP-322 signature for a
//! taproot address is Schnorr, and takes the auxiliary randomness
//! [`crate::Aux`] carries, exactly as a taproot input does.
//!
//! # Secrets
//!
//! [`sign`] borrows a `SecretKey`, and the curve context it runs in,
//! for the moment it signs: the context belongs to the master key the
//! secret was derived from and is blinded by the session, so nothing
//! here makes an unblinded one (security review M1). Taproot
//! forces two copies of it inside `bitcoin` (the untweaked keypair and
//! the BIP-341-tweaked one); both are erased before the function
//! returns, as in [`crate::sign`].

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use bitcoin::address::{Address, AddressType};
use bitcoin::hashes::{Hash, HashEngine, sha256};
use bitcoin::key::TapTweak;
use bitcoin::opcodes::OP_0;
use bitcoin::opcodes::all::OP_RETURN;
use bitcoin::script::PushBytes;
use bitcoin::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use bitcoin::secp256k1::{All, Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::sign_message::{MessageSignature, signed_msg_hash};
use bitcoin::{
    Amount, EcdsaSighashType, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut,
    Txid, Witness, absolute, consensus, transaction,
};
use osk_bip::keys::{Network, ScriptType};

use crate::base64;
use crate::sign::{Aux, Nonce};

/// The two message formats this crate signs and verifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// The legacy compact signature with a recovery header, base64.
    Bip137,
    /// BIP-322 "simple": the witness of `to_sign`, consensus-encoded and
    /// base64. "Full" is not needed for a single-key address.
    Bip322,
}

impl Format {
    /// The formats an address of `script` can carry, in the order a
    /// chooser offers them: the one more verifiers read first.
    pub fn for_script(script: ScriptType) -> &'static [Format] {
        match script {
            ScriptType::Legacy => &[Format::Bip137],
            // BIP-322 "simple" carries a witness and no scriptSig, so a
            // nested-segwit address, which is spent through its redeem
            // script, has no simple form.
            ScriptType::NestedSegwit => &[Format::Bip137],
            ScriptType::NativeSegwit => &[Format::Bip137, Format::Bip322],
            ScriptType::Taproot => &[Format::Bip322],
        }
    }
}

/// What [`sign`] produced: the address the signature is for, the
/// signature itself, and which format it is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    /// The address the message was signed for.
    pub address: String,
    /// The signature, base64.
    pub signature: String,
    /// The format it is in.
    pub format: Format,
}

/// What [`verify`] found: the signature holds for this address, in this
/// format, checked as this address form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checked {
    /// The format the signature was read as.
    pub format: Format,
    /// The address form the check rebuilt from the signature.
    pub script: ScriptType,
}

/// Why a message could not be signed or a signature could not be
/// checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The format cannot express this address form.
    UnsupportedFormat,
    /// The address is of a kind this crate cannot check a message for:
    /// a script address (multisig, p2wsh) or a future witness version.
    UnsupportedAddress,
    /// The signature is not base64, or not a signature of its format.
    Malformed,
    /// The signature is well formed and does not hold for this address
    /// and this message.
    Invalid,
}

/// The BIP-340 tag BIP-322 hashes a message under.
const TAG: &[u8] = b"BIP0322-signed-message";

/// The header byte a BIP-137 signature starts at, per address form.
const P2PKH_BASE: u8 = 31;
const P2SH_P2WPKH_BASE: u8 = 35;
const P2WPKH_BASE: u8 = 39;

/// Signs `message` for the address `secret` has of `script` on
/// `network`, with the RFC 6979 nonce `nonce` names and, for a taproot
/// BIP-322 signature, the auxiliary randomness `aux` carries.
///
/// The address is derived here from the key that signs it, so the
/// address in [`Signed`] is exactly the one the signature verifies
/// against.
///
/// `secp` is the context the key's own operations run in — the master
/// key's, blinded by the session (`osk_bip::keys`, security review M1) —
/// and not one made here, so that no curve operation on a private key
/// runs in an unblinded context.
#[allow(clippy::too_many_arguments)]
pub fn sign(
    secp: &Secp256k1<All>,
    secret: &SecretKey,
    script: ScriptType,
    network: Network,
    message: &str,
    format: Format,
    nonce: Nonce,
    aux: Aux,
) -> Result<Signed, Error> {
    if !Format::for_script(script).contains(&format) {
        return Err(Error::UnsupportedFormat);
    }
    let net = bitcoin::Network::from(network);
    let public = PublicKey::new(secret.public_key(secp));
    let address = match script {
        ScriptType::Legacy => Address::p2pkh(public, net),
        ScriptType::NestedSegwit => Address::p2shwpkh(
            &public.try_into().map_err(|_| Error::UnsupportedAddress)?,
            net,
        ),
        ScriptType::NativeSegwit => Address::p2wpkh(
            &public.try_into().map_err(|_| Error::UnsupportedAddress)?,
            net,
        ),
        ScriptType::Taproot => {
            let (xonly, _) = public.inner.x_only_public_key();
            Address::p2tr(secp, xonly, None, net)
        }
    };
    let signature = match format {
        Format::Bip137 => sign_bip137(secp, secret, script, message, nonce),
        Format::Bip322 => sign_bip322(secp, secret, &address, message, nonce, aux)?,
    };
    Ok(Signed {
        address: address.to_string(),
        signature,
        format,
    })
}

/// Checks `signature` against `address` and `message`. The format is
/// read from the signature: 65 bytes is BIP-137, anything else is a
/// BIP-322 witness.
pub fn verify(address: &Address, message: &str, signature: &str) -> Result<Checked, Error> {
    let (format, bytes) = decode(signature)?;
    match format {
        Format::Bip137 => verify_bip137(address, message, &bytes),
        Format::Bip322 => verify_bip322(address, message, &bytes),
    }
}

/// The bytes behind a signature and the format they are in.
///
/// BIP-322 gained variant prefixes in 2025: `smp` for simple, `ful` and
/// `pof` for the two full variants. Wallets in the field write the
/// witness alone, so a prefix is read where there is one and [`sign`]
/// writes the plain form. Without a prefix, 65 bytes is BIP-137's
/// compact signature and anything else is a witness.
fn decode(signature: &str) -> Result<(Format, Vec<u8>), Error> {
    let text = signature.trim();
    let (prefixed, body) = match text.get(..3) {
        Some("smp") => (true, &text[3..]),
        Some("ful") | Some("pof") => return Err(Error::UnsupportedFormat),
        _ => (false, text),
    };
    let bytes = base64::decode(body).map_err(|_| Error::Malformed)?;
    let format = if !prefixed && bytes.len() == 65 {
        Format::Bip137
    } else {
        Format::Bip322
    };
    Ok((format, bytes))
}

// ----- BIP-137 -----

fn sign_bip137(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    secret: &SecretKey,
    script: ScriptType,
    message: &str,
    nonce: Nonce,
) -> String {
    let hash = signed_msg_hash(message);
    let msg = Message::from_digest(hash.to_byte_array());
    let recoverable = match nonce {
        Nonce::LowR => sign_recoverable_low_r(secp, &msg, secret),
        Nonce::First => secp.sign_ecdsa_recoverable(&msg, secret),
    };
    let mut bytes = MessageSignature::new(recoverable, true).serialize();
    // `MessageSignature` writes the p2pkh range; the segwit ranges are
    // the same recovery id counted from a different base.
    let recid = bytes[0] - P2PKH_BASE;
    bytes[0] = base_of(script) + recid;
    base64::encode(&bytes)
}

/// RFC 6979 ground for a low `r`, in the recoverable form BIP-137 needs.
///
/// The first attempt is plain RFC 6979; while the signature's `r` is at
/// least 2^255, the counter goes up by one and is written little-endian
/// into the first four bytes of 32 bytes of RFC 6979 "additional data".
/// This is the scheme Bitcoin Core's `CKey::Sign` uses and the one
/// libsecp256k1's `sign_ecdsa_low_r` implements for the plain form,
/// which has no recoverable counterpart to call (§16.38).
fn sign_recoverable_low_r(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    msg: &Message,
    secret: &SecretKey,
) -> RecoverableSignature {
    let mut signature = secp.sign_ecdsa_recoverable(msg, secret);
    let mut counter: u32 = 0;
    while signature.serialize_compact().1[0] >= 0x80 {
        counter += 1;
        let mut noncedata = [0u8; 32];
        noncedata[..4].copy_from_slice(&counter.to_le_bytes());
        signature = secp.sign_ecdsa_recoverable_with_noncedata(msg, secret, &noncedata);
    }
    signature
}

/// The header base for the address form a BIP-137 signature is for.
fn base_of(script: ScriptType) -> u8 {
    match script {
        ScriptType::NestedSegwit => P2SH_P2WPKH_BASE,
        ScriptType::NativeSegwit => P2WPKH_BASE,
        _ => P2PKH_BASE,
    }
}

fn verify_bip137(address: &Address, message: &str, bytes: &[u8]) -> Result<Checked, Error> {
    let header = bytes[0];
    if !(27..=42).contains(&header) {
        return Err(Error::Malformed);
    }
    // Below 35 the header also says whether the key was compressed;
    // in the segwit ranges it always was.
    let (recid, compressed) = match header {
        27..=34 => ((header - 27) & 3, header >= P2PKH_BASE),
        35..=38 => (header - P2SH_P2WPKH_BASE, true),
        _ => (header - P2WPKH_BASE, true),
    };
    let recid = RecoveryId::from_i32(i32::from(recid)).map_err(|_| Error::Malformed)?;
    let signature =
        RecoverableSignature::from_compact(&bytes[1..], recid).map_err(|_| Error::Malformed)?;
    let secp = Secp256k1::verification_only();
    let hash = signed_msg_hash(message);
    let msg = Message::from_digest(hash.to_byte_array());
    let inner = secp
        .recover_ecdsa(&msg, &signature)
        .map_err(|_| Error::Malformed)?;
    // The address says which form to rebuild, and the rebuilt output
    // script is what is compared, so the check is the same on every
    // network. A p2pkh address is tried in both key encodings: the
    // header's compression bit is what an old uncompressed signature
    // carries, and a segwit-ranged header does not carry it at all.
    let spk = address.script_pubkey();
    let compressed_key = bitcoin::CompressedPublicKey(inner);
    let script = match address.address_type() {
        Some(AddressType::P2pkh) => {
            let candidates = [
                PublicKey::new(inner),
                PublicKey::new_uncompressed(inner),
                PublicKey { inner, compressed },
            ];
            if !candidates
                .iter()
                .any(|pk| ScriptBuf::new_p2pkh(&pk.pubkey_hash()) == spk)
            {
                return Err(Error::Invalid);
            }
            ScriptType::Legacy
        }
        Some(AddressType::P2sh) => {
            let redeem = ScriptBuf::new_p2wpkh(&compressed_key.wpubkey_hash());
            if ScriptBuf::new_p2sh(&redeem.script_hash()) != spk {
                return Err(Error::Invalid);
            }
            ScriptType::NestedSegwit
        }
        Some(AddressType::P2wpkh) => {
            if ScriptBuf::new_p2wpkh(&compressed_key.wpubkey_hash()) != spk {
                return Err(Error::Invalid);
            }
            ScriptType::NativeSegwit
        }
        _ => return Err(Error::UnsupportedAddress),
    };
    Ok(Checked {
        format: Format::Bip137,
        script,
    })
}

// ----- BIP-322 -----

/// The BIP-340 tagged hash of the message, which is what `to_spend`
/// commits to. BIP-322's own test vectors name it, beside the two
/// transaction ids [`virtual_txids`] returns.
pub fn message_hash(message: &str) -> [u8; 32] {
    let tag = sha256::Hash::hash(TAG);
    let mut engine = sha256::Hash::engine();
    engine.input(tag.as_byte_array());
    engine.input(tag.as_byte_array());
    engine.input(message.as_bytes());
    sha256::Hash::from_engine(engine).to_byte_array()
}

/// The ids of the two transactions a BIP-322 signature is made over:
/// `to_spend`, which pays the message to the address, and `to_sign`,
/// which spends it. Neither can be broadcast.
pub fn virtual_txids(address: &Address, message: &str) -> (Txid, Txid) {
    let spend = to_spend(&address.script_pubkey(), message);
    let sign = to_sign(&spend);
    (spend.compute_txid(), sign.compute_txid())
}

/// The three-line text form a signed message travels in, built: the
/// address on the first line, the signature on the second, the message
/// from the third to the end with its newlines kept. A QR or a file in
/// this form is checked with nothing typed.
///
/// ```text
/// bc1q9vza2e8x573nczrlzms0wvx3gsqjx7vavgkx0l
/// AkcwRAIgZRfIY3p7/DoVTty6YZbWS71bc5Vct9p9Fia83eRmw2QC…
/// Hello World
/// ```
pub fn signed_text(address: &str, signature: &str, message: &str) -> String {
    let mut out = String::with_capacity(address.len() + signature.len() + message.len() + 2);
    out.push_str(address);
    out.push('\n');
    out.push_str(signature);
    out.push('\n');
    out.push_str(message);
    out
}

/// A signed message read from a QR or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedText {
    /// The address the signature is for.
    pub address: String,
    /// The signature.
    pub signature: String,
    /// The message.
    pub message: String,
}

/// Reads the three-line form. A text with fewer than three lines, or
/// with nothing on one of the first two, is not one.
///
/// A file that ends with a newline carries the same message as one that
/// does not, so a single trailing newline is dropped; every other
/// newline in the message is kept.
pub fn parse_signed(text: &str) -> Option<SignedText> {
    let text = text.strip_suffix('\n').unwrap_or(text);
    let (address, rest) = text.split_once('\n')?;
    let (signature, message) = rest.split_once('\n')?;
    let address = address.trim();
    let signature = signature.trim();
    if address.is_empty() || signature.is_empty() {
        return None;
    }
    Some(SignedText {
        address: String::from(address),
        signature: String::from(signature),
        message: String::from(message),
    })
}

/// `to_spend`: one input spending an output that cannot exist, paying
/// nothing to the address being signed for.
fn to_spend(script_pubkey: &ScriptBuf, message: &str) -> Transaction {
    let hash = message_hash(message);
    let push: &PushBytes = (&hash).into();
    Transaction {
        version: transaction::Version(0),
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: Txid::from_byte_array([0u8; 32]),
                vout: 0xFFFF_FFFF,
            },
            script_sig: ScriptBuf::builder()
                .push_opcode(OP_0)
                .push_slice(push)
                .into_script(),
            sequence: Sequence(0),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::ZERO,
            script_pubkey: script_pubkey.clone(),
        }],
    }
}

/// `to_sign`: one input spending `to_spend`, paying nothing to nobody.
fn to_sign(spend: &Transaction) -> Transaction {
    Transaction {
        version: transaction::Version(0),
        lock_time: absolute::LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: spend.compute_txid(),
                vout: 0,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence(0),
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::ZERO,
            script_pubkey: ScriptBuf::builder().push_opcode(OP_RETURN).into_script(),
        }],
    }
}

#[allow(clippy::too_many_arguments)]
fn sign_bip322(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    secret: &SecretKey,
    address: &Address,
    message: &str,
    nonce: Nonce,
    aux: Aux,
) -> Result<String, Error> {
    let spk = address.script_pubkey();
    let spend = to_spend(&spk, message);
    let sign = to_sign(&spend);
    let mut cache = SighashCache::new(&sign);
    let witness = match address.address_type() {
        Some(AddressType::P2wpkh) => {
            let public = PublicKey::new(secret.public_key(secp));
            let compressed: bitcoin::CompressedPublicKey =
                public.try_into().map_err(|_| Error::UnsupportedAddress)?;
            let sighash = cache
                .p2wpkh_signature_hash(0, &spk, Amount::ZERO, EcdsaSighashType::All)
                .map_err(|_| Error::UnsupportedAddress)?;
            let msg = Message::from_digest(sighash.to_byte_array());
            let signature = bitcoin::ecdsa::Signature {
                signature: match nonce {
                    Nonce::LowR => secp.sign_ecdsa_low_r(&msg, secret),
                    Nonce::First => secp.sign_ecdsa(&msg, secret),
                },
                sighash_type: EcdsaSighashType::All,
            };
            Witness::p2wpkh(&signature, &compressed.0)
        }
        Some(AddressType::P2tr) => {
            let prevouts = [spend.output[0].clone()];
            let sighash = cache
                .taproot_key_spend_signature_hash(
                    0,
                    &Prevouts::All(&prevouts),
                    TapSighashType::Default,
                )
                .map_err(|_| Error::UnsupportedAddress)?;
            let msg = Message::from_digest(sighash.to_byte_array());
            let mut keypair = Keypair::from_secret_key(secp, secret);
            let mut tweaked = keypair.tap_tweak(secp, None).to_keypair();
            let signature = secp.sign_schnorr_with_aux_rand(&msg, &tweaked, &aux.for_input(0));
            keypair.non_secure_erase();
            tweaked.non_secure_erase();
            let mut witness = Witness::new();
            witness.push(signature.serialize());
            witness
        }
        _ => return Err(Error::UnsupportedAddress),
    };
    Ok(base64::encode(&consensus::serialize(&witness)))
}

fn verify_bip322(address: &Address, message: &str, bytes: &[u8]) -> Result<Checked, Error> {
    let witness: Witness = consensus::deserialize(bytes).map_err(|_| Error::Malformed)?;
    let spk = address.script_pubkey();
    let spend = to_spend(&spk, message);
    let mut sign = to_sign(&spend);
    sign.input[0].witness = witness.clone();
    let mut cache = SighashCache::new(&sign);
    let secp = Secp256k1::verification_only();
    let script = match address.address_type() {
        Some(AddressType::P2wpkh) => {
            let [signature, key] = stack::<2>(&witness)?;
            let public =
                bitcoin::CompressedPublicKey::from_slice(key).map_err(|_| Error::Malformed)?;
            if ScriptBuf::new_p2wpkh(&public.wpubkey_hash()) != spk {
                return Err(Error::Invalid);
            }
            let signature =
                bitcoin::ecdsa::Signature::from_slice(signature).map_err(|_| Error::Malformed)?;
            if signature.sighash_type != EcdsaSighashType::All {
                return Err(Error::Invalid);
            }
            let sighash = cache
                .p2wpkh_signature_hash(0, &spk, Amount::ZERO, EcdsaSighashType::All)
                .map_err(|_| Error::Malformed)?;
            let msg = Message::from_digest(sighash.to_byte_array());
            secp.verify_ecdsa(&msg, &signature.signature, &public.0)
                .map_err(|_| Error::Invalid)?;
            ScriptType::NativeSegwit
        }
        Some(AddressType::P2tr) => {
            let [signature] = stack::<1>(&witness)?;
            let signature =
                bitcoin::taproot::Signature::from_slice(signature).map_err(|_| Error::Malformed)?;
            if signature.sighash_type != TapSighashType::Default {
                return Err(Error::Invalid);
            }
            let output_key = output_key(&spk)?;
            let prevouts = [spend.output[0].clone()];
            let sighash = cache
                .taproot_key_spend_signature_hash(
                    0,
                    &Prevouts::All(&prevouts),
                    TapSighashType::Default,
                )
                .map_err(|_| Error::Malformed)?;
            let msg = Message::from_digest(sighash.to_byte_array());
            secp.verify_schnorr(&signature.signature, &msg, &output_key)
                .map_err(|_| Error::Invalid)?;
            ScriptType::Taproot
        }
        _ => return Err(Error::UnsupportedAddress),
    };
    Ok(Checked {
        format: Format::Bip322,
        script,
    })
}

/// The witness stack as exactly `N` elements.
fn stack<const N: usize>(witness: &Witness) -> Result<[&[u8]; N], Error> {
    if witness.len() != N {
        return Err(Error::Malformed);
    }
    let mut out = [&[] as &[u8]; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = witness.nth(i).ok_or(Error::Malformed)?;
    }
    Ok(out)
}

/// The taproot output key inside a p2tr scriptPubKey.
fn output_key(spk: &ScriptBuf) -> Result<XOnlyPublicKey, Error> {
    let program = spk.as_bytes();
    if program.len() != 34 {
        return Err(Error::UnsupportedAddress);
    }
    XOnlyPublicKey::from_slice(&program[2..]).map_err(|_| Error::UnsupportedAddress)
}
