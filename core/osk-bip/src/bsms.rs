//! BIP 129's two records: the Bitcoin Secure Multisig Setup files
//! Coldcard, Sparrow, Nunchuk and Keystone exchange a multisig wallet
//! in.
//!
//! Round 1 is the signer's key record: the version, the session token,
//! the key with its origin, a description and a signature over the four
//! lines above it.
//!
//! ```text
//! BSMS 1.0
//! 00
//! [1cf0bf7e/48'/0'/0'/2']xpub6FL8Fhx…
//! Signer 1 key
//! IB7v+qi1b+Xrwm/3bF+Rjl8QbIJ/FMQ40kUsOOQo1SqU…
//! ```
//!
//! Round 2 is the coordinator's descriptor record: the version, the
//! descriptor template whose keys end in `/**`, the derivation paths the
//! template expands over, and the wallet's first address.
//!
//! ```text
//! BSMS 1.0
//! wsh(sortedmulti(2,[1cf0bf7e/48'/0'/0'/2']xpub…/**,[4fc1dd4a/48'/0'/0'/2']xpub…/**))
//! /0/*,/1/*
//! bc1qrgc6p3kylfztu06ysl752gwwuekhvtfh9vr7zg43jvu60mutamcsv948ej
//! ```
//!
//! The descriptor record is the same wallet as the coordinator config
//! [`crate::multisig_config`] reads and the same wallet policy
//! [`crate::policy`] holds; what BSMS adds is the signature on the key
//! record and the first address on the descriptor record, so that each
//! side can check the other's work without trusting the channel.
//!
//! # Encryption
//!
//! The BIP also defines an encrypted form: the token as a PBKDF2
//! password, AES-256-CTR under that key, and an HMAC-SHA256 over the
//! plaintext whose first sixteen bytes are the IV, the whole written as
//! hex. This tree has no AES and no HMAC-SHA256, and adding a
//! dependency for them is not this pass's to make, so an encrypted
//! record is refused by name ([`Error::Encrypted`]) rather than read
//! wrongly. Every unencrypted record — token `00`, and also a record
//! whose token is a real nonce but which arrives as text — is read in
//! full.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use bitcoin::address::{Address, NetworkUnchecked};
use bitcoin::hashes::Hash;
use bitcoin::secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
use bitcoin::secp256k1::{Message, PublicKey, Secp256k1, SecretKey, Signing, Verification};
use bitcoin::sign_message::signed_msg_hash;
use core::fmt::{self, Write as _};
use core::str::FromStr;

use crate::account::MultisigAccountXpub;
use crate::base64;
use crate::keys::Network;
use crate::policy::{self, PolicyKey, TEMPLATE_SUFFIX, WalletPolicy};

/// The specification version both records carry on their first line.
pub const VERSION: &str = "BSMS 1.0";

/// The token that says the session is not encrypted.
pub const NO_ENCRYPTION: &str = "00";

/// The paths line a record writes when the descriptor is not a
/// template.
pub const NO_PATH_RESTRICTIONS: &str = "No path restrictions";

/// Why a BSMS record could not be read or did not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The first line is not `BSMS 1.0`.
    Version,
    /// The record does not have the lines its round calls for.
    Lines,
    /// The token is not `00`, a 64-bit nonce or a 128-bit nonce in hex.
    Token,
    /// The record is the encrypted `.dat` form, which this device does
    /// not decrypt.
    Encrypted,
    /// The key is not `[fingerprint/path]xpub`. A record whose key is a
    /// plain public key rather than an extended one lands here too: a
    /// wallet of single public keys derives no addresses.
    Key,
    /// The description is longer than the eighty characters the BIP
    /// allows.
    Description,
    /// The signature is not base64, not sixty-five bytes, or not a
    /// signature of these four lines by this key.
    Signature,
    /// The paths line is neither `No path restrictions` nor the receive
    /// and change chains this device derives.
    Paths,
    /// The fourth line is not an address, or not an address of the
    /// network the descriptor's keys are on.
    Address,
    /// The address on the record is not the wallet's own first receive
    /// address.
    FirstAddress,
    /// The descriptor is not a wallet this device can read.
    Policy(policy::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Version => f.write_str("the first line is not BSMS 1.0"),
            Error::Lines => f.write_str("the record does not have the lines it needs"),
            Error::Token => f.write_str("the token is not 00 or a hex nonce"),
            Error::Encrypted => f.write_str("this record is encrypted"),
            Error::Key => f.write_str("key is not [fingerprint/path]xpub"),
            Error::Description => f.write_str("the description is longer than 80 characters"),
            Error::Signature => f.write_str("the signature is not this key's"),
            Error::Paths => f.write_str("the derivation paths are not /0/*,/1/*"),
            Error::Address => f.write_str("the fourth line is not an address"),
            Error::FirstAddress => f.write_str("this is not the wallet's first address"),
            Error::Policy(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<policy::Error> for Error {
    fn from(e: policy::Error) -> Self {
        Error::Policy(e)
    }
}

/// Round 1: what a signer hands the coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerRecord {
    /// The session token, hex, `00` when the session is not encrypted.
    pub token: String,
    /// The key this signer brings to the wallet.
    pub key: PolicyKey,
    /// The signer's description of the key, at most eighty characters.
    pub description: String,
    /// The signature over the first four lines, base64.
    pub signature: String,
}

impl SignerRecord {
    /// Reads a key record. The signature is not checked here;
    /// [`verify`](Self::verify) checks it.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let lines = record_lines(text)?;
        if lines.len() != 5 {
            return Err(Error::Lines);
        }
        let token = lines[1].trim();
        if !is_token(token) {
            return Err(Error::Token);
        }
        let key = PolicyKey::parse(lines[2].trim()).map_err(|_| Error::Key)?;
        let description = lines[3];
        if description.chars().count() > 80 {
            return Err(Error::Description);
        }
        Ok(SignerRecord {
            token: String::from(token),
            key,
            description: String::from(description),
            signature: String::from(lines[4].trim()),
        })
    }

    /// The record as its five lines.
    pub fn to_text(&self) -> String {
        format!("{}\n{}", self.signed_text(), self.signature)
    }

    /// The four lines the signature is over: everything but the
    /// signature itself, with no trailing newline.
    pub fn signed_text(&self) -> String {
        format!(
            "{VERSION}\n{}\n{}\n{}",
            self.token,
            self.key.key_text(),
            self.description
        )
    }

    /// Whether the signature is this key's signature over those four
    /// lines.
    ///
    /// The BIP asks for BIP-322 and accepts the legacy format; every
    /// signer in the field, and every vector in the BIP itself, writes
    /// the legacy compact signature, which is what is read here. The
    /// key that signs is the record's own key — the extended public
    /// key's node — so what the check does is recover the public key
    /// from the signature and compare it to the record's.
    pub fn verify(&self) -> Result<(), Error> {
        let secp = Secp256k1::verification_only();
        self.verify_with(&secp)
    }

    /// The same check, with a caller's secp context.
    pub fn verify_with<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<(), Error> {
        let bytes = base64::decode(&self.signature).map_err(|_| Error::Signature)?;
        if bytes.len() != 65 {
            return Err(Error::Signature);
        }
        // The header byte is 27 plus the recovery id, plus 4 when the
        // public key is compressed; Electrum and Trezor put the segwit
        // address forms in the ranges above that, and the recovery id
        // is the same count from each base. Every signer BSMS names
        // signs for a key, not for an address, so only the id matters.
        if !(27..=42).contains(&bytes[0]) {
            return Err(Error::Signature);
        }
        let recid =
            RecoveryId::from_i32(i32::from((bytes[0] - 27) & 3)).map_err(|_| Error::Signature)?;
        let signature =
            RecoverableSignature::from_compact(&bytes[1..], recid).map_err(|_| Error::Signature)?;
        let hash = signed_msg_hash(&self.signed_text());
        let message = Message::from_digest(hash.to_byte_array());
        let recovered = secp
            .recover_ecdsa(&message, &signature)
            .map_err(|_| Error::Signature)?;
        if recovered == self.key.xpub().public_key {
            Ok(())
        } else {
            Err(Error::Signature)
        }
    }
}

/// Writes the key record for one of this device's multisig account
/// keys, signed by that key.
///
/// `secret` is the private key of the account node itself — the key
/// `account.xpub()` is the public half of — which is what the BIP's own
/// vectors sign with. A secret whose public key is not the account's is
/// refused rather than written into a record nobody could verify.
pub fn signer_record<C: Signing + Verification>(
    secp: &Secp256k1<C>,
    account: &MultisigAccountXpub,
    secret: &SecretKey,
    token: &str,
    description: &str,
) -> Result<SignerRecord, Error> {
    if !is_token(token) {
        return Err(Error::Token);
    }
    if description.chars().count() > 80 {
        return Err(Error::Description);
    }
    if PublicKey::from_secret_key(secp, secret) != account.xpub().public_key {
        return Err(Error::Key);
    }
    let mut origin = String::new();
    write!(origin, "[{}", account.master_fingerprint()).expect("string write");
    for child in account.path() {
        write!(origin, "/{child}").expect("string write");
    }
    write!(origin, "]{}", account.xpub()).expect("string write");
    let key = PolicyKey::parse(&origin).map_err(|_| Error::Key)?;
    let mut record = SignerRecord {
        token: String::from(token),
        key,
        description: String::from(description),
        signature: String::new(),
    };
    let hash = signed_msg_hash(&record.signed_text());
    let message = Message::from_digest(hash.to_byte_array());
    let (recid, compact) = secp
        .sign_ecdsa_recoverable(&message, secret)
        .serialize_compact();
    let mut bytes = [0u8; 65];
    bytes[0] = 27 + 4 + recid.to_i32() as u8;
    bytes[1..].copy_from_slice(&compact);
    record.signature = base64::encode(&bytes);
    Ok(record)
}

/// Round 2: what the coordinator hands every signer back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorRecord {
    /// The wallet the descriptor template states.
    pub policy: WalletPolicy,
    /// The derivation path restrictions, `/0/*,/1/*`, or none when the
    /// record says `No path restrictions`.
    pub paths: Vec<String>,
    /// The wallet's first receive address, on the network its keys are
    /// on.
    pub first_address: String,
    /// That network.
    pub network: Network,
}

impl DescriptorRecord {
    /// Reads a descriptor record, checking that the address on it is
    /// the wallet's own first receive address.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let lines = record_lines(text)?;
        if lines.len() != 4 {
            return Err(Error::Lines);
        }
        let descriptor = lines[1].trim();
        let paths = parse_paths(lines[2].trim())?;
        // A template is written with `/**`; a descriptor with no
        // template says so on its paths line, and the two must agree.
        if paths.is_empty() == descriptor.contains(TEMPLATE_SUFFIX) {
            return Err(Error::Paths);
        }
        let policy = WalletPolicy::from_descriptor(descriptor).map_err(Error::Policy)?;
        let stated = lines[3].trim();
        let unchecked =
            Address::<NetworkUnchecked>::from_str(stated).map_err(|_| Error::Address)?;
        let network = Network::ALL
            .into_iter()
            .find(|n| unchecked.is_valid_for_network(bitcoin::Network::from(*n)))
            .ok_or(Error::Address)?;
        let first = policy
            .address_at(network, false, 0)
            .map_err(Error::Policy)?;
        if first.to_string() != stated {
            return Err(Error::FirstAddress);
        }
        Ok(DescriptorRecord {
            policy,
            paths,
            first_address: String::from(stated),
            network,
        })
    }

    /// The record as its four lines.
    pub fn to_text(&self) -> String {
        let paths = if self.paths.is_empty() {
            String::from(NO_PATH_RESTRICTIONS)
        } else {
            self.paths.join(",")
        };
        let descriptor = if self.paths.is_empty() {
            self.policy.to_descriptor()
        } else {
            self.policy.to_descriptor_template()
        };
        format!("{VERSION}\n{descriptor}\n{paths}\n{}", self.first_address)
    }

    /// Whether this signer's key is one of the wallet's, matched whole
    /// and not by fingerprint, which the BIP says is trivial to spoof.
    pub fn holds(&self, key: &PolicyKey) -> bool {
        self.policy.keys().iter().any(|k| k == key)
    }
}

/// The record's lines, with the version checked and the encrypted form
/// refused.
fn record_lines(text: &str) -> Result<Vec<&str>, Error> {
    let text = text.trim_end_matches(['\n', '\r', ' ']);
    if looks_encrypted(text) {
        return Err(Error::Encrypted);
    }
    let lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    if lines.first().map(|l| l.trim()) != Some(VERSION) {
        return Err(Error::Version);
    }
    Ok(lines)
}

/// Whether `text` reads as a BSMS record of either round: the version
/// line is the whole of the first line.
pub fn looks_like_record(text: &str) -> bool {
    text.lines().next().map(str::trim) == Some(VERSION)
}

/// Which round a record is, by the shape of its second line: a token is
/// hex and short, a descriptor is neither.
pub fn is_signer_record(text: &str) -> bool {
    looks_like_record(text) && text.lines().nth(1).map(str::trim).is_some_and(is_token)
}

/// The encrypted `.dat` form: hex, and long enough to hold the
/// thirty-two byte MAC and something after it.
fn looks_encrypted(text: &str) -> bool {
    let text = text.trim();
    text.len() > 64 && text.len().is_multiple_of(2) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `00` for no encryption, or the 64-bit or 128-bit nonce in hex.
pub fn is_token(token: &str) -> bool {
    matches!(token.len(), 2 | 16 | 32)
        && token.bytes().all(|b| b.is_ascii_hexdigit())
        && (token.len() != 2 || token == NO_ENCRYPTION)
}

/// The paths line: `No path restrictions`, or the receive and change
/// chains, which are the only two this device derives.
fn parse_paths(line: &str) -> Result<Vec<String>, Error> {
    if line == NO_PATH_RESTRICTIONS {
        return Ok(Vec::new());
    }
    let paths: Vec<String> = line.split(',').map(|p| String::from(p.trim())).collect();
    if paths.iter().map(String::as_str).eq(["/0/*", "/1/*"]) {
        Ok(paths)
    } else {
        Err(Error::Paths)
    }
}
