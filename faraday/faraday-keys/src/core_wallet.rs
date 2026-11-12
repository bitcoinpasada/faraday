//! The private keys of a Bitcoin Core descriptor wallet, checked and
//! wiped, never loaded (`docs/decisions/F229.md`).
//!
//! A `wallet.dat` holds a private key for each descriptor, as a
//! `walletdescriptorkey` record (the key in DER with the hash of the
//! public key and the DER, in the clear) or, in an encrypted wallet, a
//! `walletdescriptorckey` record (the 32-byte key under AES-256-CBC with
//! the wallet's master key). The functions here take those records'
//! bytes, check that the private key gives the public key the record
//! names (and, in the clear, that the stored hash is right), wipe what
//! they held, and say pass or fail for each key. The key is never given
//! back, shown, loaded as a key of the session or written.
//!
//! The master key is Bitcoin Core's: SHA-512 of the passphrase and the
//! record's 8-byte salt, then that hash hashed again for the rounds
//! less one; the first 32 bytes are an AES-256 key and the next 16 the
//! IV that open the `mkey` record's 48 bytes under CBC with PKCS#7
//! padding (`BytesToKeySHA512AES` and `CCrypter::Decrypt` in Bitcoin
//! Core's `crypter.cpp`). Each key's IV is the first 16 bytes of the
//! double SHA-256 of its public key (`DecryptSecret`). Written here over
//! RustCrypto's `aes` and `cbc` and `faraday-crypto`'s SHA-512.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use aes::Aes256;
use aes::cipher::inout::InOutBuf;
use aes::cipher::{BlockModeDecrypt, KeyIvInit};
use faraday_bip::bitcoin::NetworkKind;
use faraday_bip::bitcoin::bip32::{ChainCode, ChildNumber, Fingerprint, Xpriv, Xpub};
use faraday_bip::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, sha256, sha512_again, sha512_into};
use faraday_intake::secret_buf::SecretText;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

/// The master key's derivation method Bitcoin Core writes (0): SHA-512
/// rounds, then AES-256-CBC.
pub const METHOD_SHA512_AES: u32 = 0;
/// The most rounds taken. Bitcoin Core calibrates a wallet's to about a
/// tenth of a second on the machine that encrypts it (hundreds of
/// thousands); a record past this is refused rather than waited for.
pub const MAX_ROUNDS: u32 = 10_000_000;

/// The DER of a compressed private key as Bitcoin Core writes it: 214
/// bytes, the 32-byte key at 8 and the compressed public key at 181.
const DER_LEN: usize = 214;
const DER_BEGIN: [u8; 8] = [0x30, 0x81, 0xD3, 0x02, 0x01, 0x01, 0x04, 0x20];
const DER_MIDDLE: [u8; 141] = [
    0xA0, 0x81, 0x85, 0x30, 0x81, 0x82, 0x02, 0x01, 0x01, 0x30, 0x2C, 0x06, 0x07, 0x2A, 0x86, 0x48,
    0xCE, 0x3D, 0x01, 0x01, 0x02, 0x21, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFC, 0x2F, 0x30, 0x06, 0x04, 0x01, 0x00, 0x04, 0x01, 0x07, 0x04,
    0x21, 0x02, 0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87,
    0x0B, 0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B, 0x16, 0xF8,
    0x17, 0x98, 0x02, 0x21, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B, 0xBF, 0xD2, 0x5E,
    0x8C, 0xD0, 0x36, 0x41, 0x41, 0x02, 0x01, 0x01, 0xA1, 0x24, 0x03, 0x22, 0x00,
];
/// The DER of a private key of an uncompressed public key: 279 bytes, the
/// 32-byte key at 9 and the 65-byte public key at 214.
const DER_LEN_UNCOMPRESSED: usize = 279;
const DER_BEGIN_UNCOMPRESSED: [u8; 9] = [0x30, 0x82, 0x01, 0x13, 0x02, 0x01, 0x01, 0x04, 0x20];
const DER_MIDDLE_UNCOMPRESSED: [u8; 173] = [
    0xA0, 0x81, 0xA5, 0x30, 0x81, 0xA2, 0x02, 0x01, 0x01, 0x30, 0x2C, 0x06, 0x07, 0x2A, 0x86, 0x48,
    0xCE, 0x3D, 0x01, 0x01, 0x02, 0x21, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFE, 0xFF, 0xFF, 0xFC, 0x2F, 0x30, 0x06, 0x04, 0x01, 0x00, 0x04, 0x01, 0x07, 0x04,
    0x41, 0x04, 0x79, 0xBE, 0x66, 0x7E, 0xF9, 0xDC, 0xBB, 0xAC, 0x55, 0xA0, 0x62, 0x95, 0xCE, 0x87,
    0x0B, 0x07, 0x02, 0x9B, 0xFC, 0xDB, 0x2D, 0xCE, 0x28, 0xD9, 0x59, 0xF2, 0x81, 0x5B, 0x16, 0xF8,
    0x17, 0x98, 0x48, 0x3A, 0xDA, 0x77, 0x26, 0xA3, 0xC4, 0x65, 0x5D, 0xA4, 0xFB, 0xFC, 0x0E, 0x11,
    0x08, 0xA8, 0xFD, 0x17, 0xB4, 0x48, 0xA6, 0x85, 0x54, 0x19, 0x9C, 0x47, 0xD0, 0x8F, 0xFB, 0x10,
    0xD4, 0xB8, 0x02, 0x21, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B, 0xBF, 0xD2, 0x5E,
    0x8C, 0xD0, 0x36, 0x41, 0x41, 0x02, 0x01, 0x01, 0xA1, 0x44, 0x03, 0x42, 0x00,
];
/// A compressed public key as the record names it.
const PUBKEY_LEN: usize = 33;
/// An uncompressed one.
const PUBKEY_LEN_UNCOMPRESSED: usize = 65;
/// A key under AES-256-CBC: 32 bytes and a block of padding.
const CRYPTED_LEN: usize = 48;

/// The extended public key Bitcoin Core caches for a descriptor's last
/// hardened step (`walletdescriptorlhcache`), to be checked against the
/// master key it comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::disallowed_methods,
    reason = "a derived Clone: no secret type derives it"
)]
pub struct AccountCheck {
    /// The master key's depth, parent fingerprint and child number, as
    /// the descriptor's extended key states them (9 bytes).
    pub meta: [u8; 9],
    /// The master key's chain code, from the descriptor's extended key.
    pub chain_code: [u8; 32],
    /// The hardened steps from the master key to the cached key, each
    /// with bit 31 set.
    pub path: Vec<u32>,
    /// The cached key as Core serializes it: depth, parent fingerprint,
    /// child number, chain code and compressed public key (74 bytes).
    pub cached: [u8; 74],
}

/// What one key record comes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyVerdict {
    /// The private key gives the public key the record names, and where
    /// the record stores a hash of the two it is right.
    Pass,
    /// The record is not the shape Bitcoin Core writes: a length, the
    /// DER's fixed parts, the padding, a private key past the curve's
    /// order, or a public key that is not 33 bytes (65 for the DER of an
    /// uncompressed key).
    Malformed,
    /// The stored hash of the public key and the private key's DER is not
    /// the one the bytes give.
    HashDiffers,
    /// The private key gives another public key than the one named.
    PublicDiffers,
}

/// Why no key was checked in an encrypted wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    /// The `mkey` record is not the shape Bitcoin Core writes.
    Malformed,
    /// A derivation method other than SHA-512 with AES-256-CBC (the
    /// number is the method).
    Method(u32),
    /// No rounds, or more than [`MAX_ROUNDS`].
    Rounds(u32),
    /// The passphrase does not open the master key.
    WrongPassphrase,
}

/// Bitcoin Core's `CompactSize` at the start of `bytes`: its value and
/// the bytes it took. Only the shortest form is read (as Core's
/// `ReadCompactSize` demands).
fn compact_size(bytes: &[u8]) -> Option<(u64, usize)> {
    let (&first, rest) = bytes.split_first()?;
    let (value, width, least) = match first {
        0..=252 => return Some((u64::from(first), 1)),
        253 => (
            u64::from(u16::from_le_bytes(rest.get(..2)?.try_into().ok()?)),
            3,
            253,
        ),
        254 => (
            u64::from(u32::from_le_bytes(rest.get(..4)?.try_into().ok()?)),
            5,
            0x1_0000,
        ),
        255 => (
            u64::from_le_bytes(rest.get(..8)?.try_into().ok()?),
            9,
            0x1_0000_0000,
        ),
    };
    (value >= least).then_some((value, width))
}

/// A byte vector as Core serializes it: its length, then the bytes; and
/// what follows.
fn vector(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (len, used) = compact_size(bytes)?;
    let rest = &bytes[used..];
    let len = usize::try_from(len).ok()?;
    (len <= rest.len()).then(|| rest.split_at(len))
}

/// The double SHA-256 of `parts` joined, as Core's `Hash`.
fn hash256(parts: &[&[u8]]) -> Zeroizing<[u8; 32]> {
    let joined: Zeroizing<Vec<u8>> = Zeroizing::new(parts.concat());
    let once = Zeroizing::new(sha256(&joined));
    Zeroizing::new(sha256(&*once))
}

/// Costs an elliptic-curve multiplication: whether the 32 bytes are a
/// secret key giving `pubkey`.
#[expect(
    clippy::disallowed_methods,
    reason = "the multiplication is counted by the token this function takes"
)]
fn gives(c: &Compute, secret: &[u8], pubkey: &[u8]) -> KeyVerdict {
    let uncompressed = pubkey.len() == PUBKEY_LEN_UNCOMPRESSED;
    c.count(faraday_crypto::work::Work::KeyDerive);
    let Ok(mut key) = SecretKey::from_slice(secret) else {
        return KeyVerdict::Malformed;
    };
    let secp = Secp256k1::signing_only();
    let public = PublicKey::from_secret_key(&secp, &key);
    key.non_secure_erase();
    let public = if uncompressed {
        public.serialize_uncompressed().to_vec()
    } else {
        public.serialize().to_vec()
    };
    if bool::from(public[..].ct_eq(pubkey)) {
        KeyVerdict::Pass
    } else {
        KeyVerdict::PublicDiffers
    }
}

/// Overwrites an `Xpriv`'s private key and chain code in place. rust-bitcoin's
/// own derivation leaves its HMAC-SHA-512 state and the intermediate keys
/// it works through on its own stack, out of reach here; this wipes what
/// Faraday holds.
fn erase(node: &mut Xpriv) {
    node.private_key.non_secure_erase();
    AsMut::<[u8; 32]>::as_mut(&mut node.chain_code).zeroize();
}

/// Costs a derivation along its path for each account check (counted once for each), and
/// an elliptic-curve multiplication for the public key: whether the 32
/// bytes are a secret key giving `pubkey`, and for each of `accounts`
/// whether the key derived along its path is the cached one. Only the
/// verdicts leave; every key derived is wiped.
#[expect(
    clippy::disallowed_methods,
    reason = "the derivations are counted by the token this function takes"
)]
fn gives_accounts(
    c: &Compute,
    secret: &[u8],
    pubkey: &[u8],
    accounts: &[AccountCheck],
) -> (KeyVerdict, Vec<bool>) {
    let verdict = gives(c, secret, pubkey);
    if verdict != KeyVerdict::Pass || accounts.is_empty() {
        return (verdict, Vec::new());
    }
    let Ok(mut key) = SecretKey::from_slice(secret) else {
        return (KeyVerdict::Malformed, Vec::new());
    };
    let secp = Secp256k1::signing_only();
    let matches = accounts
        .iter()
        .map(|a| {
            c.count(faraday_crypto::work::Work::KeyDerive);
            let [depth, f0, f1, f2, f3, n0, n1, n2, n3] = a.meta;
            let mut node = Xpriv {
                network: NetworkKind::Test,
                depth,
                parent_fingerprint: Fingerprint::from([f0, f1, f2, f3]),
                child_number: ChildNumber::from(u32::from_be_bytes([n0, n1, n2, n3])),
                private_key: key,
                chain_code: ChainCode::from(a.chain_code),
            };
            let mut ok = true;
            for &step in &a.path {
                match node.derive_priv(&secp, &[ChildNumber::from(step)]) {
                    Ok(next) => {
                        erase(&mut node);
                        node = next;
                    }
                    Err(_) => {
                        ok = false;
                        break;
                    }
                }
            }
            let same = ok && {
                let public = Xpub::from_priv(&secp, &node).encode();
                bool::from(public[4..].ct_eq(&a.cached[..]))
            };
            erase(&mut node);
            same
        })
        .collect();
    key.non_secure_erase();
    (KeyVerdict::Pass, matches)
}

/// Costs an elliptic-curve multiplication and two hashes. A
/// `walletdescriptorkey` record's `value` (the private key's DER as a
/// vector, then the 32-byte hash of the public key and that DER) against
/// the public key its record's key names (`pubkey`, the 33 or 65 bytes
/// without their length), wiped when this returns.
pub fn check_private_key(c: &Compute, pubkey: &[u8], value: Zeroizing<Vec<u8>>) -> KeyVerdict {
    check_private_key_accounts(c, pubkey, value, &[]).0
}

/// Costs what [`check_private_key`] costs and a hardened derivation for
/// each step of each account check: the verdict, and where it is a pass,
/// whether each of `accounts` (cached account keys, derived from this
/// key as the master key of the descriptor) is the key derived from it.
pub fn check_private_key_accounts(
    c: &Compute,
    pubkey: &[u8],
    value: Zeroizing<Vec<u8>>,
    accounts: &[AccountCheck],
) -> (KeyVerdict, Vec<bool>) {
    let Some((der, hash)) = vector(&value) else {
        return (KeyVerdict::Malformed, Vec::new());
    };
    // Where the key sits in the DER, by the public key's form.
    let key_at = match (pubkey.len(), der.len()) {
        (PUBKEY_LEN, DER_LEN)
            if der[..8] == DER_BEGIN && der[40..181] == DER_MIDDLE && der[181..] == *pubkey =>
        {
            8
        }
        (PUBKEY_LEN_UNCOMPRESSED, DER_LEN_UNCOMPRESSED)
            if der[..9] == DER_BEGIN_UNCOMPRESSED
                && der[41..214] == DER_MIDDLE_UNCOMPRESSED
                && der[214..] == *pubkey =>
        {
            9
        }
        _ => return (KeyVerdict::Malformed, Vec::new()),
    };
    if hash.len() != 32 {
        return (KeyVerdict::Malformed, Vec::new());
    }
    let stored = hash256(&[pubkey, der]);
    if !bool::from(stored[..].ct_eq(hash)) {
        return (KeyVerdict::HashDiffers, Vec::new());
    }
    gives_accounts(c, &der[key_at..key_at + 32], pubkey, accounts)
}

/// Costs the master key's SHA-512 rounds (up to [`MAX_ROUNDS`]) and a
/// decryption and an elliptic-curve multiplication for each key. The
/// passphrase opens the `mkey` record's master key (`mkey`, the record's
/// value), and each `(pubkey, value)` of `keys`, a
/// `walletdescriptorckey` record's public key and its value (the 48
/// encrypted bytes as a vector), both borrowed from the caller, which
/// holds the only copy; each is opened with it and checked against
/// its public key. A verdict for each key, in order; the wrong
/// passphrase is the [`OpenError`], not a verdict. Everything held is
/// wiped when this returns.
pub fn check_encrypted_keys<'a>(
    c: &Compute,
    mkey: &[u8],
    passphrase: &SecretText,
    keys: impl IntoIterator<Item = (&'a [u8], &'a [u8])>,
) -> Result<Vec<KeyVerdict>, OpenError> {
    let keys = keys
        .into_iter()
        .map(|(pubkey, value)| (pubkey, value, &[][..]));
    Ok(check_encrypted_keys_accounts(c, mkey, passphrase, keys)?
        .into_iter()
        .map(|(verdict, _)| verdict)
        .collect())
}

/// Costs what [`check_encrypted_keys`] costs and a hardened derivation
/// for each step of each account check: each key also carries the cached
/// account keys (`&[AccountCheck]`) to check against it as the master
/// key of their descriptor, and its answer is the verdict with whether
/// each matched.
pub fn check_encrypted_keys_accounts<'a>(
    c: &Compute,
    mkey: &[u8],
    passphrase: &SecretText,
    keys: impl IntoIterator<Item = (&'a [u8], &'a [u8], &'a [AccountCheck])>,
) -> Result<Vec<(KeyVerdict, Vec<bool>)>, OpenError> {
    c.count(faraday_crypto::work::Work::Kdf);
    let master = open_master_key(mkey, passphrase.as_str().as_bytes())?;
    Ok(keys
        .into_iter()
        .map(|(pubkey, value, accounts)| {
            check_encrypted_key(c, master.expose(), pubkey, value, accounts)
        })
        .collect())
}

/// The master key `mkey` holds, under `passphrase`.
fn open_master_key(mkey: &[u8], passphrase: &[u8]) -> Result<Secret<[u8; 32]>, OpenError> {
    let (crypted, rest) = vector(mkey).ok_or(OpenError::Malformed)?;
    let (salt, rest) = vector(rest).ok_or(OpenError::Malformed)?;
    let (method, rest) = rest.split_first_chunk::<4>().ok_or(OpenError::Malformed)?;
    let (rounds, rest) = rest.split_first_chunk::<4>().ok_or(OpenError::Malformed)?;
    let (_other, rest) = vector(rest).ok_or(OpenError::Malformed)?;
    if crypted.len() != CRYPTED_LEN || salt.len() != 8 || !rest.is_empty() {
        return Err(OpenError::Malformed);
    }
    let method = u32::from_le_bytes(*method);
    let rounds = u32::from_le_bytes(*rounds);
    if method != METHOD_SHA512_AES {
        return Err(OpenError::Method(method));
    }
    if rounds == 0 || rounds > MAX_ROUNDS {
        return Err(OpenError::Rounds(rounds));
    }
    let mut chain = Zeroizing::new([passphrase, salt].concat());
    // Each round's digest is written over the last, in one wiping array.
    let mut digest = Zeroizing::new([0u8; 64]);
    sha512_into(&chain, &mut digest);
    for _ in 1..rounds {
        sha512_again(&mut digest);
    }
    chain.fill(0);
    let plain =
        decrypt(&digest[..32], &digest[32..48], crypted).ok_or(OpenError::WrongPassphrase)?;
    let mut master = Secret::new([0u8; 32]);
    master.expose_mut().copy_from_slice(&plain[..]);
    Ok(master)
}

/// Costs an elliptic-curve multiplication and two hashes: one
/// `walletdescriptorckey` against its public key, under the master key.
fn check_encrypted_key(
    c: &Compute,
    master: &[u8; 32],
    pubkey: &[u8],
    value: &[u8],
    accounts: &[AccountCheck],
) -> (KeyVerdict, Vec<bool>) {
    let Some((crypted, rest)) = vector(value) else {
        return (KeyVerdict::Malformed, Vec::new());
    };
    if !matches!(pubkey.len(), PUBKEY_LEN | PUBKEY_LEN_UNCOMPRESSED)
        || crypted.len() != CRYPTED_LEN
        || !rest.is_empty()
    {
        return (KeyVerdict::Malformed, Vec::new());
    }
    let iv = hash256(&[pubkey]);
    match decrypt(master, &iv[..16], crypted) {
        Some(plain) => gives_accounts(c, &plain[..], pubkey, accounts),
        None => (KeyVerdict::Malformed, Vec::new()),
    }
}

/// 48 bytes of AES-256-CBC opened to 32 and the PKCS#7 padding checked
/// (a block of sixteen 0x10), wiped by the caller's drop.
fn decrypt(key: &[u8], iv: &[u8], crypted: &[u8]) -> Option<Zeroizing<[u8; 32]>> {
    let mut buf = Zeroizing::new([0u8; CRYPTED_LEN]);
    buf.copy_from_slice(crypted);
    let (blocks, _) = InOutBuf::from(&mut buf[..]).into_chunks();
    cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
        .ok()?
        .decrypt_blocks_inout(blocks);
    // The padding is a whole block of 0x10; Core's key and master key
    // are 32 bytes, and a record that unpads to another length is not one.
    let padding = buf[32..].iter().fold(0u8, |bad, &b| bad | (b ^ 0x10));
    if padding != 0 {
        return None;
    }
    let mut plain = Zeroizing::new([0u8; 32]);
    plain.copy_from_slice(&buf[..32]);
    Some(plain)
}
