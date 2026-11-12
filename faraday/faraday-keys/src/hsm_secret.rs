//! Core Lightning's `hsm_secret` as a source of Lightning node key
//! (`docs/decisions/F228.md`): its 32 bytes plain, its 73 bytes encrypted
//! under a passphrase, the mnemonic form (a 32-byte passphrase hash, then
//! English BIP-39 words as text), or BIP-39 words made into one as
//! `lightning-hsmtool generatehsm` makes it. The node key is the one
//! Core Lightning's `hsmd` derives, so one secret is one node id on every
//! network.
//!
//! The encrypted form is a libsodium `secretstream`
//! (`crypto_secretstream_xchacha20poly1305`) of one message, under a key
//! Argon2id makes from the passphrase; the opening is written here over
//! RustCrypto's ChaCha20 (with HChaCha20) and Poly1305.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use argon2::Algorithm;
use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use chacha20::{ChaCha20, R20, hchacha};
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::SecretKey;
use faraday_crypto::work::Compute;
use faraday_crypto::{Argon2Cost, Argon2Error, Secret};
use faraday_intake::secret_buf::SecretText;
use poly1305::Poly1305;
use poly1305::universal_hash::KeyInit;
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, Zeroizing};

use crate::lightning::{Node, node_of_secret_key};

/// Bytes of a plain `hsm_secret`.
pub const PLAIN_LEN: usize = 32;
/// Bytes of an encrypted `hsm_secret`: the secretstream's header, a tag
/// byte, the secret and the message's 16-byte authenticator.
pub const ENCRYPTED_LEN: usize = 73;

/// The most bytes of a mnemonic-form file read: the 32-byte passphrase
/// hash and room for the longest 24 English words (8 letters each) with
/// their spaces. Core Lightning has no bound, and libwally takes 27 and
/// 30 words too; Faraday's BIP-39 takes 12 to 24, so a longer file is
/// refused, as `Length` past this bound and as `Words` below it.
pub const MNEMONIC_MAX_LEN: usize = PLAIN_LEN + 256;

/// The secretstream header (`crypto_secretstream_xchacha20poly1305_HEADERBYTES`).
const HEADER_LEN: usize = 24;
/// The Argon2id salt Core Lightning fixes: `c-lightning` and five zero
/// bytes (`get_encryption_key` in `common/hsm_secret.c`).
const SALT: [u8; 16] = *b"c-lightning\0\0\0\0\0";
/// libsodium's `crypto_pwhash_argon2id_MEMLIMIT_MODERATE`, 256 MiB, in KiB.
const MEMORY_KIB: u32 = 262_144;
/// libsodium's `crypto_pwhash_argon2id_OPSLIMIT_MODERATE`: three passes.
const PASSES: u32 = 3;
/// The info of the node key's HKDF (`node_key` in `hsmd/libhsmd.c`).
const NODE_INFO: &[u8] = b"nodeid";

/// Why no node came from the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The file is shorter than 32 bytes or longer than the mnemonic form
    /// can be.
    Length,
    /// The mnemonic form's text is not English BIP-39 words as Core
    /// Lightning writes them (valid UTF-8, no NUL, one space between
    /// words, a valid checksum).
    Words,
    /// The mnemonic form's passphrase is wrong.
    Passphrase,
    /// The passphrase is not printable ASCII, which Faraday's BIP-39 takes
    /// alone, though Core Lightning takes any bytes: the file is not said
    /// to have another passphrase.
    PassphraseNotAscii,
    /// The passphrase is longer than Faraday's BIP-39 takes.
    PassphraseTooLong,
    /// The mnemonic form has no passphrase (its hash is zero) and one was
    /// given; Core Lightning refuses that too.
    PassphraseNotNeeded,
    /// The tag failed: the passphrase is wrong or the file is changed.
    Opening,
    /// The 256 MiB the passphrase's stretch needs could not be had.
    Memory,
}

/// Costs an elliptic-curve multiplication. The node key Core Lightning
/// derives from the 32-byte `secret`: HKDF-SHA256 with the secret as
/// input key material, the info `nodeid` and a 4-byte salt that counts up
/// from 0 (in the byte order of the machine Core Lightning runs on: little
/// endian), taken again with the next salt while the 32 bytes are not a
/// secp256k1 private key.
pub fn node_from_secret(c: &Compute, secret: &[u8; PLAIN_LEN]) -> Node {
    let mut salt: u32 = 0;
    loop {
        let mut okm = Zeroizing::new([0u8; 32]);
        faraday_crypto::hkdf_sha256(&salt.to_le_bytes(), secret, NODE_INFO, &mut okm[..])
            .expect("32 bytes is within HKDF's output");
        if let Ok(mut key) = SecretKey::from_slice(&okm[..]) {
            let node = node_of_secret_key(c, &key);
            // libsecp256k1's own overwrite (volatile writes), which the
            // library calls best effort; the bytes in `okm` are wiped
            // by `Zeroizing`.
            key.non_secure_erase();
            return node;
        }
        salt += 1;
    }
}

/// Whether reading the `hsm_secret` file `bytes` asks for a passphrase, as
/// Core Lightning's `hsm_secret_needs_passphrase`: an encrypted file, or
/// a mnemonic-form file whose first 32 bytes are not all zero. The
/// screen asks before it starts the job.
pub fn needs_passphrase(bytes: &[u8]) -> bool {
    match bytes.len() {
        0..=PLAIN_LEN => false,
        ENCRYPTED_LEN => true,
        _ => bytes[..PLAIN_LEN].iter().any(|&b| b != 0),
    }
}

/// Costs an Argon2id run of 256 MiB (when the file is encrypted), a
/// PBKDF2 (when it is the mnemonic form) and an elliptic-curve
/// multiplication. The node of an `hsm_secret` file's `bytes`, told apart
/// by length as Core Lightning's `detect_hsm_secret_type` does: 32 bytes
/// are the secret, 73 are opened under `passphrase`, every other length
/// over 32 is the mnemonic form ([`mnemonic_secret`]), and a shorter file
/// is refused. The passphrase is not read for a plain file.
pub fn node_from_file(c: &Compute, bytes: &[u8], passphrase: &SecretText) -> Result<Node, Refusal> {
    match bytes.len() {
        PLAIN_LEN => {
            let mut secret = Zeroizing::new([0u8; PLAIN_LEN]);
            secret.copy_from_slice(bytes);
            Ok(node_from_secret(c, &secret))
        }
        ENCRYPTED_LEN => {
            let mut file = [0u8; ENCRYPTED_LEN];
            file.copy_from_slice(bytes);
            let secret = open(c, &file, passphrase.as_str().as_bytes())?;
            Ok(node_from_secret(c, secret.expose()))
        }
        0..PLAIN_LEN => Err(Refusal::Length),
        _ => {
            let secret = mnemonic_secret(c, bytes, passphrase)?;
            Ok(node_from_secret(c, secret.expose()))
        }
    }
}

/// Costs a PBKDF2 and a checksum. The secret of a mnemonic-form `file`,
/// read as Core Lightning's `extract_mnemonic_secret` reads it: the first
/// 32 bytes are all zero when the words have no passphrase, else the
/// SHA-256 of the 64-byte BIP-39 seed the words and passphrase give; the
/// rest is the words as text. The secret is the first 32 bytes of that
/// seed, as `hsmd` takes it.
///
/// Refused where Core Lightning refuses, and where it would read the text
/// in a way Faraday does not: a text that is not UTF-8, holds a NUL
/// (`tal_strndup` would stop there), is not English BIP-39 words with a
/// valid checksum, or is not the words in single-space form (a newline or
/// extra space is not accepted); a passphrase on a zero hash (`PASSPHRASE_NOT_NEEDED`);
/// a passphrase whose seed hashes to another value (`WRONG_PASSPHRASE`).
/// A passphrase Faraday's BIP-39 cannot take (not printable ASCII, or too
/// long) has a refusal of its own, not the wrong-passphrase one.
fn mnemonic_secret(
    c: &Compute,
    file: &[u8],
    passphrase: &SecretText,
) -> Result<Secret<[u8; PLAIN_LEN]>, Refusal> {
    if file.len() > MNEMONIC_MAX_LEN {
        return Err(Refusal::Length);
    }
    let (hash, text) = file.split_at(PLAIN_LEN);
    let text = core::str::from_utf8(text).map_err(|_| Refusal::Words)?;
    if text.contains('\0') {
        return Err(Refusal::Words);
    }
    let words = Mnemonic::parse(c, Language::English, text).map_err(|_| Refusal::Words)?;
    let mut single = Zeroizing::new(Vec::with_capacity(MNEMONIC_MAX_LEN));
    for (i, &w) in words.indices().iter().enumerate() {
        if i > 0 {
            single.push(b' ');
        }
        single.extend_from_slice(Language::English.word(w).as_bytes());
    }
    if single.as_slice() != text.as_bytes() {
        return Err(Refusal::Words);
    }

    let none = hash.iter().all(|&b| b == 0);
    if none && !passphrase.as_str().is_empty() {
        return Err(Refusal::PassphraseNotNeeded);
    }
    let seed = words
        .to_seed(c, passphrase.as_str().as_bytes())
        .map_err(|e| match e {
            faraday_bip::bip39::Error::PassphraseTooLong => Refusal::PassphraseTooLong,
            _ => Refusal::PassphraseNotAscii,
        })?;
    if !none {
        let digest = Zeroizing::new(faraday_crypto::sha256(seed.expose()));
        if !bool::from(digest[..].ct_eq(hash)) {
            return Err(Refusal::Passphrase);
        }
    }
    let mut secret = Secret::new([0u8; PLAIN_LEN]);
    secret
        .expose_mut()
        .copy_from_slice(&seed.expose()[..PLAIN_LEN]);
    Ok(secret)
}

/// Costs an Argon2id run of 256 MiB. The secret in an encrypted
/// `hsm_secret` `file` under `passphrase`: the key Argon2id makes with
/// Core Lightning's salt and libsodium's moderate limits, then the
/// secretstream's one message.
fn open(
    c: &Compute,
    file: &[u8; ENCRYPTED_LEN],
    passphrase: &[u8],
) -> Result<Secret<[u8; PLAIN_LEN]>, Refusal> {
    let mut key = Secret::new([0u8; 32]);
    faraday_crypto::argon2(
        c,
        Algorithm::Argon2id,
        passphrase,
        &SALT,
        Argon2Cost {
            memory_kib: MEMORY_KIB,
            passes: PASSES,
            lanes: 1,
        },
        &mut key,
    )
    .map_err(|e| match e {
        Argon2Error::Memory => Refusal::Memory,
        Argon2Error::Params => Refusal::Opening,
    })?;
    pull(key.expose(), file).ok_or(Refusal::Opening)
}

/// libsodium's `crypto_secretstream_xchacha20poly1305_init_pull` and one
/// `pull` with no additional data, of a stream's first message.
///
/// The header's first 16 bytes and `key` make the subkey (HChaCha20); the
/// ChaCha20 nonce is the message counter, 1 as a 4-byte little-endian
/// number, then the header's last 8 bytes. Block 0 of the keystream is
/// Poly1305's key, block 1 hides the tag byte (the rest of it is added to
/// the authenticated data as it is), blocks 2 on hide the message. The
/// tag byte itself is not asked for, as Core Lightning does not ask for it.
fn pull(key: &[u8; 32], file: &[u8; ENCRYPTED_LEN]) -> Option<Secret<[u8; PLAIN_LEN]>> {
    let (header, body) = file.split_at(HEADER_LEN);
    let mut first = [0u8; 16];
    first.copy_from_slice(&header[..16]);
    let mut made = hchacha::<R20>(key.into(), &first.into());
    let mut subkey = Zeroizing::new([0u8; 32]);
    subkey.copy_from_slice(&made);
    made.as_mut_slice().zeroize();
    let mut nonce = [0u8; 12];
    nonce[0] = 1;
    nonce[4..].copy_from_slice(&header[16..]);
    let mut cipher = ChaCha20::new((&*subkey).into(), &nonce.into());

    let mut mac_key = Zeroizing::new([0u8; 32]);
    cipher.apply_keystream(&mut mac_key[..]);
    cipher.seek(64u64);
    let mut block = Zeroizing::new([0u8; 64]);
    cipher.apply_keystream(&mut block[..]);

    let (tag_byte, rest) = body.split_at(1);
    let (sealed, mac) = rest.split_at(PLAIN_LEN);
    // What the authenticator covers: the 64-byte block with the tag byte
    // as sent, the ciphertext, no additional data and the two lengths.
    let mut signed = Zeroizing::new([0u8; 64 + PLAIN_LEN + 16]);
    signed[..64].copy_from_slice(&block[..]);
    signed[0] = tag_byte[0];
    signed[64..64 + PLAIN_LEN].copy_from_slice(sealed);
    signed[64 + PLAIN_LEN + 8..].copy_from_slice(&((64 + PLAIN_LEN) as u64).to_le_bytes());
    let expected = Poly1305::new((&*mac_key).into()).compute_unpadded(&signed[..]);
    if !bool::from(expected.as_slice().ct_eq(mac)) {
        return None;
    }

    let mut secret = Secret::new([0u8; PLAIN_LEN]);
    secret.expose_mut().copy_from_slice(sealed);
    cipher.seek(128u64);
    cipher.apply_keystream(&mut secret.expose_mut()[..]);
    Some(secret)
}

/// A read of an `hsm_secret` file to run on another thread
/// (`docs/WORKERS.md` §6): the file's bytes and the passphrase, moved in,
/// wiped as they drop.
pub struct HsmJob {
    bytes: Zeroizing<Vec<u8>>,
    passphrase: SecretText,
}

/// What an [`HsmJob`] gives back: the node key, or why there is none.
/// It joins the screen through [`crate::Keys::hsm_node`], on the main
/// thread, and wipes as it drops.
pub struct HsmRead {
    pub(crate) found: Result<Node, Refusal>,
}

impl HsmJob {
    /// Whether the file asks for a passphrase ([`needs_passphrase`]).
    pub fn needs_passphrase(&self) -> bool {
        needs_passphrase(&self.bytes)
    }

    /// Costs what [`node_from_file`] costs. The read, with its inputs
    /// wiped as it ends.
    pub fn run(self, c: &Compute) -> HsmRead {
        HsmRead {
            found: node_from_file(c, &self.bytes, &self.passphrase),
        }
    }
}

impl crate::Keys {
    /// A read of the `hsm_secret` file `bytes` under `passphrase`, both
    /// moved in.
    pub fn hsm_job(&self, bytes: Zeroizing<Vec<u8>>, passphrase: SecretText) -> HsmJob {
        HsmJob { bytes, passphrase }
    }
}
