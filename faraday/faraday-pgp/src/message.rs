//! The QR vault's OpenPGP message (`docs/VAULT.md` §1.2): an RFC 9580
//! message under a passphrase, binary, three packets and nothing else.
//!
//! - a version 6 symmetric-key encrypted session key (tag 3): AES-256,
//!   OCB, the Argon2 S2K (salt 16, passes 3, lanes 4, a memory exponent),
//!   the OCB IV, the encrypted session key and its tag (RFC 9580 §5.3);
//! - a version 2 symmetrically encrypted and integrity protected data
//!   packet (tag 18): AES-256, OCB, chunk size octet 10, a 32-byte salt,
//!   the chunks and the final tag (§5.13.2);
//! - which holds one literal data packet (tag 11): format `b`, no file
//!   name, date 0, then the QR TOML (§5.9).
//!
//! [`seal`] writes the message from one caller-supplied buffer of
//! random bytes. [`parse`] reads the key packet first and checks the
//! stated Argon2 cost against the limits of `docs/VAULT.md` §3 before it
//! reads the data packet (the key packet has a fixed length, so nothing
//! is joined or copied until the cost is accepted); [`Message::open`]
//! then runs Argon2id and checks every tag before the plaintext is
//! returned. Padding packets (tag 21) after the data packet and after the
//! literal data are ignored, as RFC 9580 §5.14 requires. The passphrase
//! is given as the bytes of its NFC form.
//!
//! Only the part used is written (F178): AES-256 with OCB, the Argon2
//! S2K, the length forms RFC 9580 §4.2 allows. Everything else is
//! refused, named [`Unsupported`], "An OpenPGP message Faraday does not
//! open".
//!
//! The Argon2id and the HKDF are the vault's: the `argon2` crate as
//! `faraday-vault` runs it and Faraday's own HKDF (`faraday-crypto`).

use alloc::vec::Vec;

#[cfg(test)]
use aes::Aes128;
use aes::Aes256;
use argon2::Algorithm;
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, hkdf_sha256_expand, hmac_sha256};
use ocb3::AeadInOut;
use ocb3::aead::inout::InOutBuf;
use ocb3::consts::{U15, U16};
use ocb3::{Array, KeyInit, Ocb3};
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

/// The random bytes [`seal`] takes: the S2K salt (16), the OCB IV (15),
/// the session key (32) and the data packet's salt (32).
pub const RANDOM_LEN: usize = 95;
/// The Argon2 passes Faraday writes (`docs/VAULT.md` §3).
pub const PASSES: u8 = 3;
/// The Argon2 lanes Faraday writes.
pub const LANES: u8 = 4;
/// The memory exponents a reader accepts: 2^13 KiB (8 MiB) to 2^20 KiB
/// (1 GiB).
pub const EXPONENTS: core::ops::RangeInclusive<u8> = 13..=20;
/// The memory exponents [`seal`] writes, the steps of `docs/VAULT.md` §3:
/// 2^16 KiB (64 MiB) to 2^20 KiB (1 GiB).
pub const SEAL_EXPONENTS: core::ops::RangeInclusive<u8> = 16..=20;
/// The most passes a reader accepts.
pub const MAX_PASSES: u8 = 100;
/// The most lanes a reader accepts.
pub const MAX_LANES: u8 = 64;
/// The chunk size octet Faraday writes: chunks of 2^(10 + 6) = 64 KiB.
pub const CHUNK_OCTET: u8 = 10;
/// The longest plaintext [`seal`] takes, so that the message is one
/// chunk: what a chunk holds after the literal data packet's header
/// (6 octets at this length) and fields (6).
pub const MAX_PLAINTEXT: usize = (1 << (CHUNK_OCTET as usize + 6)) - 12;

const TAG_PKESK: u8 = 1;
const TAG_SIGNATURE: u8 = 2;
const TAG_SKESK: u8 = 3;
const TAG_ONE_PASS: u8 = 4;
const TAG_COMPRESSED: u8 = 8;
const TAG_ENCRYPTED: u8 = 9;
const TAG_LITERAL: u8 = 11;
const TAG_SEIPD: u8 = 18;
const TAG_PADDING: u8 = 21;

#[cfg(test)]
const AES128: u8 = 7;
const AES256: u8 = 9;
const OCB: u8 = 2;
const S2K_ARGON2: u8 = 4;
/// The S2K specifier: type, salt, passes, lanes, exponent.
const S2K_LEN: usize = 20;
const IV_LEN: usize = 15;
const TAG_LEN: usize = 16;
const SALT_LEN: usize = 32;
const OCB_PREFIX: usize = 7;

type Ocb256 = Ocb3<Aes256, U15, U16>;
#[cfg(test)]
type Ocb128 = Ocb3<Aes128, U15, U16>;

/// What a message is refused for, being an OpenPGP message that is not
/// a QR vault: "An OpenPGP message Faraday does not open".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsupported {
    /// A version 4 symmetric-key encrypted session key (what GnuPG writes).
    Skesk4,
    /// A version 1 symmetrically encrypted and integrity protected data
    /// packet.
    Seipd1,
    /// A public-key encrypted session key.
    Pkesk,
    /// A second symmetric-key encrypted session key.
    SecondSkesk,
    /// A compressed data packet.
    Compressed,
    /// A signature or a one-pass signature.
    Signature,
    /// Anything but padding after the data packet, or after the literal
    /// data.
    Trailing,
    /// Any other packet, version, algorithm, S2K or AEAD mode.
    Other,
}

/// Why a message was not made or opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// An OpenPGP message that is not a QR vault.
    Unsupported(Unsupported),
    /// Not the packets of an OpenPGP message, or a length that does not fit.
    Malformed,
    /// The stated Argon2 cost is outside the limits (`docs/VAULT.md` §3).
    Cost,
    /// The Argon2 memory could not be allocated: the MiB it needs.
    Memory(u32),
    /// The passphrase is wrong: the session key's tag did not check.
    WrongPassphrase,
    /// The passphrase is right and a data tag did not check.
    Damaged,
    /// The literal data packet's format is not `b`, or it has a file name.
    Literal,
    /// The plaintext does not fit one chunk.
    TooLong,
}

/// The Argon2id cost a message states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cost {
    /// The memory is 2^`exponent` KiB.
    pub exponent: u8,
    /// The passes over the memory.
    pub passes: u8,
    /// The lanes.
    pub lanes: u8,
}

impl Cost {
    /// The memory, in KiB.
    #[must_use]
    pub fn memory_kib(&self) -> u32 {
        1 << self.exponent
    }

    fn within_limits(&self) -> bool {
        EXPONENTS.contains(&self.exponent)
            && (1..=MAX_PASSES).contains(&self.passes)
            && (1..=MAX_LANES).contains(&self.lanes)
    }
}

/// The two block ciphers the primitives know; a message is read and
/// written with AES-256 alone, AES-128 is RFC 9580's own test vectors'.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cipher {
    #[cfg(test)]
    Aes128,
    Aes256,
}

impl Cipher {
    fn id(self) -> u8 {
        match self {
            #[cfg(test)]
            Cipher::Aes128 => AES128,
            Cipher::Aes256 => AES256,
        }
    }

    fn key_len(self) -> usize {
        match self {
            #[cfg(test)]
            Cipher::Aes128 => 16,
            Cipher::Aes256 => 32,
        }
    }
}

/// A message read to its packets, its cost checked, nothing run.
#[derive(Debug)]
pub struct Message<'a> {
    cost: Cost,
    s2k_salt: [u8; 16],
    iv: [u8; IV_LEN],
    session: [u8; 32],
    session_tag: [u8; TAG_LEN],
    chunk_octet: u8,
    salt: [u8; SALT_LEN],
    /// The chunks and the final tag.
    data: Body<'a>,
}

impl Message<'_> {
    /// The Argon2id cost the message states, for showing before the
    /// passphrase is asked for.
    #[must_use]
    pub fn cost(&self) -> Cost {
        self.cost
    }

    /// Costs an Argon2id run at the stated cost (`docs/VAULT.md` §4):
    /// opens the message under `passphrase`, the bytes of its NFC form,
    /// and returns the literal data. Every tag is checked before any
    /// plaintext is returned.
    pub fn open(&self, c: &Compute, passphrase: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
        let key = s2k(
            c,
            passphrase,
            &self.s2k_salt,
            self.cost.memory_kib(),
            self.cost.passes,
            self.cost.lanes,
        )?;
        let session = unwrap_session_key(
            Cipher::Aes256,
            key.expose(),
            &self.iv,
            &self.session,
            &self.session_tag,
        )
        .map_err(|()| Error::WrongPassphrase)?;
        let plain = decrypt_chunks(
            Cipher::Aes256,
            &session,
            self.chunk_octet,
            &self.salt,
            self.data.as_slice(),
        )?;
        read_literal(&plain)
    }
}

/// Parses the three packets of a QR vault and checks the Argon2 cost
/// against the limits of `docs/VAULT.md` §3, before anything is
/// allocated. Any other message is refused, named.
pub fn parse(bytes: &[u8]) -> Result<Message<'_>, Error> {
    // A first packet that is a data packet (which may be in partial
    // lengths) is named by its header's tag before any of it is joined.
    if let Some(&b0) = bytes.first() {
        let tag = if b0 & 0x40 != 0 {
            b0 & 0x3f
        } else {
            (b0 >> 2) & 0x0f
        };
        if b0 & 0x80 != 0 && tag != TAG_SKESK && is_data_packet(tag) {
            return Err(Error::Unsupported(classify(tag)));
        }
    }
    let (first, rest) = read_packet(bytes)?;
    if first.tag != TAG_SKESK {
        return Err(Error::Unsupported(classify(first.tag)));
    }
    let skesk = first.body.as_slice();
    match skesk.first() {
        Some(4) => return Err(Error::Unsupported(Unsupported::Skesk4)),
        Some(6) => {}
        Some(_) => return Err(Error::Unsupported(Unsupported::Other)),
        None => return Err(Error::Malformed),
    }

    // SKESK v6: version, count, cipher, AEAD, S2K length, S2K, IV, the
    // encrypted session key and its tag. Its cost is checked before the
    // data packet is read.
    let [_, count, cipher, aead, s2k_len, tail @ ..] = skesk else {
        return Err(Error::Malformed);
    };
    if *cipher != AES256 || *aead != OCB || usize::from(*s2k_len) != S2K_LEN {
        return Err(Error::Unsupported(Unsupported::Other));
    }
    if usize::from(*count) != 3 + S2K_LEN + IV_LEN || tail.len() != S2K_LEN + IV_LEN + 32 + TAG_LEN
    {
        return Err(Error::Malformed);
    }
    let (s2k, tail) = tail.split_at(S2K_LEN);
    if s2k[0] != S2K_ARGON2 {
        return Err(Error::Unsupported(Unsupported::Other));
    }
    let cost = Cost {
        passes: s2k[17],
        lanes: s2k[18],
        exponent: s2k[19],
    };
    if !cost.within_limits() {
        return Err(Error::Cost);
    }
    let (iv, tail) = tail.split_at(IV_LEN);
    let (session, session_tag) = tail.split_at(32);

    let (second, rest) = read_packet(rest)?;
    if second.tag != TAG_SEIPD {
        return Err(Error::Unsupported(classify(second.tag)));
    }
    let seipd = second.body.as_slice();
    match seipd.first() {
        Some(1) => return Err(Error::Unsupported(Unsupported::Seipd1)),
        Some(2) => {}
        Some(_) => return Err(Error::Unsupported(Unsupported::Other)),
        None => return Err(Error::Malformed),
    }
    skip_padding(rest)?;

    // SEIPD v2: version, cipher, AEAD, chunk size octet, salt, then the
    // chunks and the final tag.
    let [_, cipher, aead, chunk_octet, rest @ ..] = seipd else {
        return Err(Error::Malformed);
    };
    if *cipher != AES256 || *aead != OCB {
        return Err(Error::Unsupported(Unsupported::Other));
    }
    if *chunk_octet > 16 || rest.len() < SALT_LEN + TAG_LEN {
        return Err(Error::Malformed);
    }
    let (salt, _) = rest.split_at(SALT_LEN);
    Ok(Message {
        cost,
        s2k_salt: s2k[1..17].try_into().expect("16 bytes"),
        iv: iv.try_into().expect("15 bytes"),
        session: session.try_into().expect("32 bytes"),
        session_tag: session_tag.try_into().expect("16 bytes"),
        chunk_octet: *chunk_octet,
        salt: salt.try_into().expect("32 bytes"),
        data: second.body.tail(1 + 3 + SALT_LEN),
    })
}

/// Costs an Argon2id run at `exponent`, 3 passes and 4 lanes
/// (`docs/VAULT.md` §3): seals `plaintext`, the QR TOML, as a QR vault
/// under `passphrase`, the bytes of its NFC form. Every random value
/// comes from `random`. An `exponent` outside the memory steps
/// ([`SEAL_EXPONENTS`]) is refused.
pub fn seal(
    c: &Compute,
    passphrase: &[u8],
    exponent: u8,
    random: &[u8; RANDOM_LEN],
    plaintext: &[u8],
) -> Result<Vec<u8>, Error> {
    if !SEAL_EXPONENTS.contains(&exponent) {
        return Err(Error::Cost);
    }
    seal_at(c, passphrase, exponent, random, plaintext)
}

/// Costs an Argon2id run: [`seal`] without the check of the memory steps,
/// so that the tests can seal at the reader's least cost.
fn seal_at(
    c: &Compute,
    passphrase: &[u8],
    exponent: u8,
    random: &[u8; RANDOM_LEN],
    plaintext: &[u8],
) -> Result<Vec<u8>, Error> {
    let cost = Cost {
        exponent,
        passes: PASSES,
        lanes: LANES,
    };
    if !cost.within_limits() {
        return Err(Error::Cost);
    }
    if plaintext.len() > MAX_PLAINTEXT {
        return Err(Error::TooLong);
    }
    // The literal data packet: format b, no name, date 0.
    let mut literal_body = Zeroizing::new(Vec::with_capacity(6 + plaintext.len()));
    literal_body.extend_from_slice(&[b'b', 0, 0, 0, 0, 0]);
    literal_body.extend_from_slice(plaintext);
    let literal = Zeroizing::new(crate::packet(TAG_LITERAL, &literal_body));
    seal_packets(c, passphrase, cost, random, &literal)
}

/// Costs an Argon2id run: the two packets around `inner`, the plaintext
/// of the data packet.
fn seal_packets(
    c: &Compute,
    passphrase: &[u8],
    cost: Cost,
    random: &[u8; RANDOM_LEN],
    inner: &[u8],
) -> Result<Vec<u8>, Error> {
    let s2k_salt: &[u8; 16] = random[..16].try_into().expect("16 bytes");
    let iv: &[u8; IV_LEN] = random[16..31].try_into().expect("15 bytes");
    let session: Zeroizing<[u8; 32]> = Zeroizing::new(random[31..63].try_into().expect("32 bytes"));
    let salt: &[u8; SALT_LEN] = random[63..].try_into().expect("32 bytes");

    let key = s2k(
        c,
        passphrase,
        s2k_salt,
        cost.memory_kib(),
        cost.passes,
        cost.lanes,
    )?;
    let (wrapped, tag) = wrap_session_key(Cipher::Aes256, key.expose(), iv, &*session);

    let mut skesk = Vec::with_capacity(88);
    skesk.extend_from_slice(&[6, (3 + S2K_LEN + IV_LEN) as u8, AES256, OCB, S2K_LEN as u8]);
    skesk.push(S2K_ARGON2);
    skesk.extend_from_slice(s2k_salt);
    skesk.extend_from_slice(&[cost.passes, cost.lanes, cost.exponent]);
    skesk.extend_from_slice(iv);
    skesk.extend_from_slice(&wrapped);
    skesk.extend_from_slice(&tag);

    let mut seipd = Vec::new();
    seipd.extend_from_slice(&[2, AES256, OCB, CHUNK_OCTET]);
    seipd.extend_from_slice(salt);
    seipd.extend(encrypt_chunks(
        Cipher::Aes256,
        &*session,
        CHUNK_OCTET,
        salt,
        inner,
    ));

    let mut out = crate::packet(TAG_SKESK, &skesk);
    out.extend(crate::packet(TAG_SEIPD, &seipd));
    Ok(out)
}

/// What a packet tag in the wrong place is named.
fn classify(tag: u8) -> Unsupported {
    match tag {
        TAG_PKESK => Unsupported::Pkesk,
        TAG_SKESK => Unsupported::SecondSkesk,
        TAG_SIGNATURE | TAG_ONE_PASS => Unsupported::Signature,
        TAG_COMPRESSED => Unsupported::Compressed,
        _ => Unsupported::Other,
    }
}

/// Costs an Argon2id run: `memory_kib` of memory filled `passes` times,
/// asked for so that a cost this machine cannot meet is an error. The
/// RFC 9580 Argon2 S2K (§3.7.1.4): Argon2id version 1.3, 32 bytes.
fn s2k(
    c: &Compute,
    passphrase: &[u8],
    salt: &[u8; 16],
    memory_kib: u32,
    passes: u8,
    lanes: u8,
) -> Result<Secret<[u8; 32]>, Error> {
    let mut derived = Secret::new([0u8; 32]);
    faraday_crypto::argon2(
        c,
        Algorithm::Argon2id,
        passphrase,
        salt,
        faraday_crypto::Argon2Cost {
            memory_kib,
            passes: u32::from(passes),
            lanes: u32::from(lanes),
        },
        &mut derived,
    )
    .map_err(|e| match e {
        faraday_crypto::Argon2Error::Memory => Error::Memory(memory_kib / 1024),
        faraday_crypto::Argon2Error::Params => Error::Cost,
    })?;
    Ok(derived)
}

/// HKDF-SHA-256 (RFC 5869) into `out`.
fn hkdf(salt: &[u8], ikm: &[u8], info: &[u8], out: &mut [u8]) {
    let prk = Zeroizing::new(hmac_sha256(salt, ikm));
    hkdf_sha256_expand(&*prk, info, out).expect("output within HKDF's bound");
}

/// The info the session key's key is derived with, and its associated data.
fn skesk_info(cipher: Cipher) -> [u8; 4] {
    [0xC0 | TAG_SKESK, 6, cipher.id(), OCB]
}

/// The key that wraps the session key (§5.3): HKDF of the S2K output.
fn wrapping_key(cipher: Cipher, s2k_output: &[u8]) -> Zeroizing<[u8; 32]> {
    let mut kek = Zeroizing::new([0u8; 32]);
    hkdf(
        &[],
        s2k_output,
        &skesk_info(cipher),
        &mut kek[..cipher.key_len()],
    );
    kek
}

/// Encrypts `buf` in place under `key`, AES-OCB with a 15-byte nonce and
/// 16-byte tag, and returns the tag.
fn ocb_seal(
    cipher: Cipher,
    key: &[u8],
    nonce: &[u8; IV_LEN],
    ad: &[u8],
    buf: &mut [u8],
) -> [u8; TAG_LEN] {
    let nonce = Array::from(*nonce);
    let tag = match cipher {
        Cipher::Aes256 => Ocb256::new_from_slice(key)
            .expect("32-byte key")
            .encrypt_inout_detached(&nonce, ad, InOutBuf::from(buf)),
        #[cfg(test)]
        Cipher::Aes128 => Ocb128::new_from_slice(key)
            .expect("16-byte key")
            .encrypt_inout_detached(&nonce, ad, InOutBuf::from(buf)),
    };
    tag.expect("a buffer within OCB's bound").into()
}

/// Decrypts `buf` in place and checks `tag`. On `Err` the buffer holds
/// plaintext that must not be used.
fn ocb_open(
    cipher: Cipher,
    key: &[u8],
    nonce: &[u8; IV_LEN],
    ad: &[u8],
    buf: &mut [u8],
    tag: &[u8; TAG_LEN],
) -> Result<(), ()> {
    let nonce = Array::from(*nonce);
    let tag = Array::from(*tag);
    match cipher {
        Cipher::Aes256 => Ocb256::new_from_slice(key)
            .expect("32-byte key")
            .decrypt_inout_detached(&nonce, ad, InOutBuf::from(buf), &tag),
        #[cfg(test)]
        Cipher::Aes128 => Ocb128::new_from_slice(key)
            .expect("16-byte key")
            .decrypt_inout_detached(&nonce, ad, InOutBuf::from(buf), &tag),
    }
    .map_err(|_| ())
}

/// The session key, encrypted under the wrapping key of `s2k_output`,
/// and its tag (§5.3).
fn wrap_session_key(
    cipher: Cipher,
    s2k_output: &[u8],
    iv: &[u8; IV_LEN],
    session: &[u8],
) -> (Vec<u8>, [u8; TAG_LEN]) {
    let kek = wrapping_key(cipher, s2k_output);
    let mut buf = session.to_vec();
    let tag = ocb_seal(
        cipher,
        &kek[..cipher.key_len()],
        iv,
        &skesk_info(cipher),
        &mut buf,
    );
    (buf, tag)
}

/// The session key, if the tag checks, which a wrong passphrase fails.
fn unwrap_session_key(
    cipher: Cipher,
    s2k_output: &[u8],
    iv: &[u8; IV_LEN],
    wrapped: &[u8],
    tag: &[u8; TAG_LEN],
) -> Result<Zeroizing<Vec<u8>>, ()> {
    let kek = wrapping_key(cipher, s2k_output);
    let mut buf = Zeroizing::new(wrapped.to_vec());
    ocb_open(
        cipher,
        &kek[..cipher.key_len()],
        iv,
        &skesk_info(cipher),
        &mut buf,
        tag,
    )?;
    Ok(buf)
}

/// The message key, the IV prefix and the associated data of a data
/// packet (§5.13.2).
struct DataKeys {
    key: Zeroizing<[u8; 32]>,
    key_len: usize,
    prefix: [u8; OCB_PREFIX],
    info: [u8; 5],
}

impl DataKeys {
    fn new(cipher: Cipher, session: &[u8], salt: &[u8; SALT_LEN], chunk_octet: u8) -> DataKeys {
        let info = [0xC0 | TAG_SEIPD, 2, cipher.id(), OCB, chunk_octet];
        let mut out = Zeroizing::new([0u8; 32 + OCB_PREFIX]);
        let n = cipher.key_len() + OCB_PREFIX;
        hkdf(salt, session, &info, &mut out[..n]);
        let mut key = Zeroizing::new([0u8; 32]);
        key[..cipher.key_len()].copy_from_slice(&out[..cipher.key_len()]);
        let mut prefix = [0u8; OCB_PREFIX];
        prefix.copy_from_slice(&out[cipher.key_len()..n]);
        DataKeys {
            key,
            key_len: cipher.key_len(),
            prefix,
            info,
        }
    }

    /// The nonce of chunk `index`; the final tag's is the next index.
    fn nonce(&self, index: u64) -> [u8; IV_LEN] {
        let mut nonce = [0u8; IV_LEN];
        nonce[..OCB_PREFIX].copy_from_slice(&self.prefix);
        nonce[OCB_PREFIX..].copy_from_slice(&index.to_be_bytes());
        nonce
    }

    /// The final tag's associated data: the info and the plaintext's
    /// length.
    fn final_ad(&self, plaintext_len: usize) -> [u8; 13] {
        let mut ad = [0u8; 13];
        ad[..5].copy_from_slice(&self.info);
        ad[5..].copy_from_slice(&(plaintext_len as u64).to_be_bytes());
        ad
    }
}

/// The data packet's chunks and final tag for `plaintext` (§5.13.2).
fn encrypt_chunks(
    cipher: Cipher,
    session: &[u8],
    chunk_octet: u8,
    salt: &[u8; SALT_LEN],
    plaintext: &[u8],
) -> Vec<u8> {
    let keys = DataKeys::new(cipher, session, salt, chunk_octet);
    let key = &keys.key[..keys.key_len];
    let size = 1usize << (usize::from(chunk_octet) + 6);
    let mut out = Vec::with_capacity(plaintext.len() + TAG_LEN * (plaintext.len() / size + 2));
    let mut index = 0u64;
    for chunk in plaintext.chunks(size) {
        let start = out.len();
        out.extend_from_slice(chunk);
        let tag = ocb_seal(
            cipher,
            key,
            &keys.nonce(index),
            &keys.info,
            &mut out[start..],
        );
        out.extend_from_slice(&tag);
        index += 1;
    }
    let tag = ocb_seal(
        cipher,
        key,
        &keys.nonce(index),
        &keys.final_ad(plaintext.len()),
        &mut [],
    );
    out.extend_from_slice(&tag);
    out
}

/// The plaintext of a data packet's chunks and final tag, every tag
/// checked before it is returned.
fn decrypt_chunks(
    cipher: Cipher,
    session: &[u8],
    chunk_octet: u8,
    salt: &[u8; SALT_LEN],
    data: &[u8],
) -> Result<Zeroizing<Vec<u8>>, Error> {
    let keys = DataKeys::new(cipher, session, salt, chunk_octet);
    let key = &keys.key[..keys.key_len];
    let size = 1usize << (usize::from(chunk_octet) + 6);
    let split = data.len().checked_sub(TAG_LEN).ok_or(Error::Malformed)?;
    let (mut rest, final_tag) = data.split_at(split);
    let mut plain = Zeroizing::new(Vec::with_capacity(rest.len()));
    let mut index = 0u64;
    while !rest.is_empty() {
        let n = rest.len().min(size + TAG_LEN);
        if n <= TAG_LEN {
            return Err(Error::Malformed);
        }
        let (chunk, after) = rest.split_at(n);
        rest = after;
        let (cipher_text, tag) = chunk.split_at(n - TAG_LEN);
        let start = plain.len();
        plain.extend_from_slice(cipher_text);
        ocb_open(
            cipher,
            key,
            &keys.nonce(index),
            &keys.info,
            &mut plain[start..],
            tag.try_into().expect("16 bytes"),
        )
        .map_err(|()| Error::Damaged)?;
        index += 1;
    }
    ocb_open(
        cipher,
        key,
        &keys.nonce(index),
        &keys.final_ad(plain.len()),
        &mut [],
        final_tag.try_into().expect("16 bytes"),
    )
    .map_err(|()| Error::Damaged)?;
    Ok(plain)
}

/// Whether `rest` is nothing, or padding packets (§5.14, which a reader
/// ignores) and nothing else. Their total is bounded by the input, as every
/// length is.
fn skip_padding(mut rest: &[u8]) -> Result<(), Error> {
    while !rest.is_empty() {
        match read_packet(rest) {
            Ok((packet, after)) if packet.tag == TAG_PADDING => rest = after,
            _ => return Err(Error::Unsupported(Unsupported::Trailing)),
        }
    }
    Ok(())
}

/// The data of the one literal data packet a plaintext must be (§5.9):
/// format `b` and no file name, then padding packets and nothing else.
/// The date is not read.
fn read_literal(plain: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let (packet, rest) = read_packet(plain)?;
    if packet.tag != TAG_LITERAL {
        return Err(Error::Unsupported(classify(packet.tag)));
    }
    skip_padding(rest)?;
    let body = packet.body.as_slice();
    let [format, name_len, tail @ ..] = body else {
        return Err(Error::Malformed);
    };
    if *format != b'b' || *name_len != 0 {
        return Err(Error::Literal);
    }
    let data = tail.get(4..).ok_or(Error::Malformed)?;
    Ok(Zeroizing::new(data.to_vec()))
}

/// A packet's body: borrowed, or joined from partial lengths and wiped.
#[derive(Debug)]
enum Body<'a> {
    Borrowed(&'a [u8]),
    Joined(Zeroizing<Vec<u8>>),
}

impl<'a> Body<'a> {
    fn as_slice(&self) -> &[u8] {
        match self {
            Body::Borrowed(b) => b,
            Body::Joined(v) => v,
        }
    }

    /// The body from `skip` on.
    fn tail(self, skip: usize) -> Body<'a> {
        match self {
            Body::Borrowed(b) => Body::Borrowed(&b[skip..]),
            Body::Joined(v) => Body::Joined(Zeroizing::new(v[skip..].to_vec())),
        }
    }
}

struct Packet<'a> {
    tag: u8,
    body: Body<'a>,
}

/// A length as RFC 9580 §4.2.1 writes it in the new format.
enum Len {
    Fixed(usize),
    Partial(usize),
}

fn new_len(input: &[u8], pos: usize) -> Result<(Len, usize), Error> {
    let at = |i: usize| {
        input
            .get(pos + i)
            .copied()
            .map(usize::from)
            .ok_or(Error::Malformed)
    };
    let o1 = at(0)?;
    Ok(match o1 {
        0..=191 => (Len::Fixed(o1), pos + 1),
        192..=223 => (Len::Fixed(((o1 - 192) << 8) + at(1)? + 192), pos + 2),
        224..=254 => (Len::Partial(1 << (o1 & 0x1f)), pos + 1),
        _ => {
            let n = (at(1)? << 24) | (at(2)? << 16) | (at(3)? << 8) | at(4)?;
            (Len::Fixed(n), pos + 5)
        }
    })
}

/// Whether a packet of `tag` may have a partial or an indeterminate length.
fn is_data_packet(tag: u8) -> bool {
    matches!(
        tag,
        TAG_COMPRESSED | TAG_ENCRYPTED | TAG_LITERAL | TAG_SEIPD
    )
}

/// Reads one packet header and body (RFC 9580 §4.2) in any length form,
/// and returns what follows it.
fn read_packet(input: &[u8]) -> Result<(Packet<'_>, &[u8]), Error> {
    let take = |pos: usize, n: usize| -> Result<&[u8], Error> {
        let end = pos.checked_add(n).ok_or(Error::Malformed)?;
        input.get(pos..end).ok_or(Error::Malformed)
    };
    let b0 = *input.first().ok_or(Error::Malformed)?;
    if b0 & 0x80 == 0 {
        return Err(Error::Malformed);
    }
    if b0 & 0x40 != 0 {
        let tag = b0 & 0x3f;
        let (len, mut pos) = new_len(input, 1)?;
        match len {
            Len::Fixed(n) => {
                let body = take(pos, n)?;
                let packet = Packet {
                    tag,
                    body: Body::Borrowed(body),
                };
                Ok((packet, &input[pos + n..]))
            }
            Len::Partial(first) => {
                // Partial lengths: data packets only, the first at least
                // 512 octets, the last a fixed length.
                if !is_data_packet(tag) || first < 512 {
                    return Err(Error::Malformed);
                }
                // The total first, so that the join is one allocation and
                // no growth leaves a copy in freed memory.
                let total = {
                    let (mut at, mut n, mut total) = (pos, first, 0usize);
                    loop {
                        take(at, n)?;
                        total += n;
                        at += n;
                        let (len, next) = new_len(input, at)?;
                        at = next;
                        match len {
                            Len::Partial(m) => n = m,
                            Len::Fixed(m) => {
                                take(at, m)?;
                                break total + m;
                            }
                        }
                    }
                };
                let mut joined = Zeroizing::new(Vec::with_capacity(total));
                let mut n = first;
                loop {
                    joined.extend_from_slice(take(pos, n)?);
                    pos += n;
                    let (len, next) = new_len(input, pos)?;
                    pos = next;
                    match len {
                        Len::Partial(m) => n = m,
                        Len::Fixed(m) => {
                            joined.extend_from_slice(take(pos, m)?);
                            pos += m;
                            break;
                        }
                    }
                }
                let packet = Packet {
                    tag,
                    body: Body::Joined(joined),
                };
                Ok((packet, &input[pos..]))
            }
        }
    } else {
        let tag = (b0 >> 2) & 0x0f;
        let (n, pos) = match b0 & 3 {
            0 => (usize::from(*input.get(1).ok_or(Error::Malformed)?), 2),
            1 => {
                let b = take(1, 2)?;
                (usize::from(u16::from_be_bytes([b[0], b[1]])), 3)
            }
            2 => {
                let b = take(1, 4)?;
                (u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize, 5)
            }
            _ => {
                // Indeterminate: to the end of the input.
                if !is_data_packet(tag) {
                    return Err(Error::Malformed);
                }
                (input.len() - 1, 1)
            }
        };
        let body = take(pos, n)?;
        let packet = Packet {
            tag,
            body: Body::Borrowed(body),
        };
        Ok((packet, &input[pos + n..]))
    }
}
