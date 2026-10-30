//! A single private key written as WIF (Wallet Import Format): what
//! Bitcoin Core's `dumpprivkey` writes and a paper wallet prints, read
//! by the rule Core's `DecodeSecret` reads it with (`key_io.cpp`).
//!
//! 1. base58check, so a bad checksum is a rejection;
//! 2. a version byte of `0x80` (mainnet) or `0xef` (the test networks);
//! 3. then 32 bytes of key, and either nothing more (an uncompressed key)
//!    or one byte that is exactly `0x01` (a compressed key);
//! 4. a scalar in `1..n`.
//!
//! A WIF names no script. The key pays to three: a legacy address
//! (P2PKH) of its public key in the form the WIF states, and, for a
//! compressed key only, native SegWit (P2WPKH) and nested SegWit
//! (P2SH-P2WPKH). [`SingleKey::scripts`] gives those, in that order.
//!
//! Private material passes through the decoded buffer, which is erased
//! before this module returns, and the key is held on a pinned page
//! ([`faraday_crypto::Pinned`]) and erased when it is dropped, with the
//! same best-effort rule as [`crate::keys::MasterKey`]. Like it, the key
//! carries a signing context of its own, blinded from the key itself
//! when it is read.

use bitcoin::NetworkKind;
use bitcoin::base58;
use bitcoin::secp256k1::{All, PublicKey, Secp256k1, SecretKey};
use bitcoin::{CompressedPublicKey, ScriptBuf};
use faraday_crypto::work::{Compute, Work};
use faraday_crypto::{Pinned, Zeroize, hmac_sha512};

/// The version byte of a mainnet WIF.
pub const MAINNET: u8 = 0x80;
/// The version byte of a WIF for the test networks (testnet, signet,
/// regtest).
pub const TEST: u8 = 0xef;

/// Why text is not a WIF private key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not valid base58, or the checksum does not hold.
    Base58,
    /// Decoded to a length that is neither 33 nor 34 bytes.
    Length(usize),
    /// The version byte is neither [`MAINNET`] nor [`TEST`].
    Version(u8),
    /// A 34-byte WIF whose last byte, the compression flag, is not `0x01`.
    Flag(u8),
    /// The key is zero or at or above the curve order.
    Key,
}

/// The domain separation of the blind a key derives for its context.
const BLIND_LABEL: &[u8] = b"faraday-wif-blind";

/// A `SecretKey` that a [`Pinned`] page can hold: erasing it is
/// `non_secure_erase`, a volatile write.
struct KeyCell(SecretKey);

impl Zeroize for KeyCell {
    fn zeroize(&mut self) {
        self.0.non_secure_erase();
    }
}

/// One private key read from a WIF, with its public key and the form
/// it is in.
///
/// Implements none of `Debug`, `Display` or `Clone`; what leaves is
/// public (the public key, the form, the network, the scripts it pays)
/// or the key itself to a signer in this repository's key boundary
/// ([`SingleKey::secret_key`]).
pub struct SingleKey {
    secret: Pinned<KeyCell>,
    public: bitcoin::PublicKey,
    network: NetworkKind,
    secp: Secp256k1<All>,
}

impl SingleKey {
    /// Costs a base58check decode and an elliptic-curve multiplication
    /// for the public key (and one for the context's blind). Reads
    /// `text` as a WIF, strictly (see the module's rules); surrounding
    /// white space is not part of it.
    pub fn decode(c: &Compute, text: &str) -> Result<Self, Error> {
        c.count(Work::KeyDerive);
        let mut bytes = base58::decode_check(text).map_err(|_| Error::Base58)?;
        let held = Self::from_payload(&bytes);
        bytes.zeroize();
        held
    }

    /// The key a decoded payload holds, by rules 2 to 4.
    fn from_payload(bytes: &[u8]) -> Result<Self, Error> {
        let compressed = match bytes.len() {
            33 => false,
            34 if bytes[33] == 0x01 => true,
            34 => return Err(Error::Flag(bytes[33])),
            n => return Err(Error::Length(n)),
        };
        let network = match bytes[0] {
            MAINNET => NetworkKind::Main,
            TEST => NetworkKind::Test,
            v => return Err(Error::Version(v)),
        };
        let mut key = SecretKey::from_slice(&bytes[1..33]).map_err(|_| Error::Key)?;
        let secp = blinded(&key);
        let public = bitcoin::PublicKey {
            inner: PublicKey::from_secret_key(&secp, &key),
            compressed,
        };
        let secret = Pinned::new(KeyCell(key));
        key.non_secure_erase();
        Ok(SingleKey {
            secret,
            public,
            network,
            secp,
        })
    }

    /// The public key, in the form the WIF states.
    pub fn public_key(&self) -> bitcoin::PublicKey {
        self.public
    }

    /// Whether the WIF states a compressed public key.
    pub fn compressed(&self) -> bool {
        self.public.compressed
    }

    /// Whether the WIF is mainnet's ([`MAINNET`]) or the test networks'
    /// ([`TEST`]).
    pub fn network(&self) -> NetworkKind {
        self.network
    }

    /// The scripts the key pays to: P2PKH, and for a compressed key
    /// P2WPKH and P2SH-P2WPKH, in that order. A lookup and two hashes
    /// each.
    pub fn scripts(&self) -> alloc::vec::Vec<ScriptBuf> {
        let mut out = alloc::vec![ScriptBuf::new_p2pkh(&self.public.pubkey_hash())];
        if let Ok(cpk) = CompressedPublicKey::try_from(self.public) {
            let wpkh = ScriptBuf::new_p2wpkh(&cpk.wpubkey_hash());
            out.push(wpkh.clone());
            out.push(ScriptBuf::new_p2sh(&wpkh.script_hash()));
        }
        out
    }

    /// The first four bytes of HASH160 of the public key as the WIF
    /// states it: what names a key with no BIP-32 origin, as Core's
    /// wallet writes it for an imported key.
    pub fn id(&self) -> [u8; 4] {
        let h = self.public.pubkey_hash();
        let b: &[u8] = h.as_ref();
        [b[0], b[1], b[2], b[3]]
    }

    /// The private key, for the signer (`faraday_psbt::sweep`).
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret.expose().0
    }

    /// The signing context this key's operations run in, blinded for
    /// it.
    pub fn secp(&self) -> &Secp256k1<All> {
        &self.secp
    }

    /// Costs an elliptic-curve multiplication: randomizes the signing
    /// context from `blind`, as [`crate::keys::MasterKey::reblind`] does.
    /// Blinding changes no result.
    pub fn reblind(&mut self, c: &Compute, blind: &[u8; 32]) {
        c.count(Work::Blind);
        self.secp.seeded_randomize(blind);
    }
}

/// A signing context blinded for `key`: the blind is HMAC-SHA512 of the
/// key under a label of its own, as [`crate::keys::MasterKey`]'s is of
/// its chain code. An attacker who could predict it would hold the key.
fn blinded(key: &SecretKey) -> Secp256k1<All> {
    let mut secp = Secp256k1::new();
    let mut bytes = key.secret_bytes();
    let mut derived = hmac_sha512(&bytes, BLIND_LABEL);
    let mut blind = [0u8; 32];
    blind.copy_from_slice(&derived[..32]);
    secp.seeded_randomize(&blind);
    blind.zeroize();
    derived.zeroize();
    bytes.zeroize();
    secp
}
