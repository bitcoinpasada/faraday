//! The KDBX 4 container a file vault is (`docs/VAULT.md` §1.1, §7.1):
//! KeePass's file format, read and written as its specification says
//! (`tools/reference/kdbx4/`), the container only. What the XML document
//! holds is the meaning of entries, which is not here.
//!
//! Written here for F178's reason, only the part used: the header, the
//! HMAC block stream, ChaCha20 and AES-256-CBC, gzip read, the inner
//! header and the inner random stream, over the primitives the tree
//! already trusts.
//!
//! - [`recognise`] says from the first twelve bytes whether a file is KDBX
//!   and of which version, with no work.
//! - [`inspect`] reads a file's header and checks its cost against §3's
//!   limits, with no passphrase and no work, so the memory a vault needs
//!   is known before the passphrase is asked for.
//! - [`open`] opens a file Faraday or a KeePass app saved (4.0 or 4.1,
//!   ChaCha20 or AES-256, Argon2id or Argon2d, gzip or none), giving the
//!   XML document's bytes, the inner stream's key and what the header
//!   states. Every block is authenticated before any of its bytes is used.
//! - [`seal`] writes a file as Faraday writes one (4.0, ChaCha20,
//!   Argon2id, no compression), every random value from the one buffer
//!   the caller passes (F190 point 8).
//! - [`InnerStream`] encrypts and decrypts a protected value in document
//!   order.
//!
//! A file with an unknown header field, a KDBX 3.1 file, AES-KDF,
//! Twofish, Salsa20 as the inner stream, a cost outside the limits or any
//! damage is refused, each as its own [`Error`]. A key file or a
//! challenge-response key is not named anywhere in a KDBX 4 header, so a
//! file under one cannot be told from a wrong passphrase
//! ([`Error::Passphrase`]); the message for a KeePass file says that a file
//! which also needs a key file or a security key cannot be opened here
//! (owner, 2026-11-02; `docs/VAULT.md` §7.1).

mod blocks;
mod gzip;
mod header;
mod inner;
pub mod items;
mod keys;
pub mod xml;

use alloc::vec::Vec;

use aes::cipher::inout::InOutBuf;
use aes::cipher::{BlockModeDecrypt, KeyIvInit};
use chacha20::ChaCha20;
use chacha20::cipher::StreamCipher;
use faraday_crypto::work::Compute;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::Cost;

pub use header::{Cipher, Compression, Header, Kdf, LANES, MEMORY_KIB, PASSES};
pub use inner::{Binaries, Binary, InnerStream, Shared};
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub use items::fuzz_items;
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub use xml::fuzz_document as fuzz_xml;

/// The random bytes a sealed file needs: the master seed (32), the
/// encryption IV (12), the Argon2 salt (32) and the inner stream's key
/// (64), in that order (§5).
pub const RANDOM_LEN: usize = 140;

/// The most a file's plaintext, the inner header, the XML document and the
/// attachments, may be: 128 MiB, the ceiling of a vault (§2, F206).
pub const MAX_PLAIN: usize = 128 * 1024 * 1024;

/// The most the stored body may be, with its framing, before decryption:
/// the plaintext as a compressing program may have left it, and an eighth
/// of slack that covers the frames (36 bytes for each 1 MiB block).
pub const MAX_STORED: usize = MAX_PLAIN + MAX_PLAIN / 8;

/// The most attachments a file may hold. Each costs 32 bytes once read,
/// and a field is only 6 bytes of a file, so the count is limited
/// apart from the length: a million is far more than a vault holds.
pub const MAX_BINARIES: usize = 1 << 20;

/// Where the inner stream's key stands in the random bytes.
const INNER_KEY_AT: usize = 76;

/// What a reader refuses by name (§7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unsupported {
    /// Twofish as the cipher.
    Twofish,
    /// A cipher other than ChaCha20 and AES-256.
    OtherCipher,
    /// AES-KDF as the key derivation.
    AesKdf,
    /// A key derivation other than Argon2id and Argon2d.
    OtherKdf,
    /// A compression other than none and gzip.
    Compression,
    /// Salsa20 as the inner random stream.
    Salsa20,
    /// An inner random stream other than ChaCha20.
    OtherInnerStream,
}

/// Why a file could not be read or written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not a KDBX file.
    NotKdbx,
    /// A KDBX file of an older major version (3.1 and before).
    Older {
        /// The major version.
        major: u16,
    },
    /// A KDBX file of a version after 4.1.
    Newer {
        /// The major version.
        major: u16,
        /// The minor version.
        minor: u16,
    },
    /// A header or inner header field this reader does not know.
    UnknownField(u8),
    /// A cipher, key derivation, compression or inner stream this reader
    /// does not take.
    Unsupported(Unsupported),
    /// The stated Argon2 cost is outside §3's limits, or carries a secret
    /// key or associated data, or is not Argon2 version 1.3 (a missing version is [`Error::Damaged`]).
    Cost,
    /// The Argon2 memory could not be allocated: the MiB it needs.
    Memory(u32),
    /// The header's authentication fails: the passphrase is wrong, or the
    /// file also needs a key file or a challenge-response key, which this
    /// reader does not take and a KDBX 4 header does not record, so the
    /// two cannot be told apart.
    Passphrase,
    /// The passphrase is empty.
    EmptyPassphrase,
    /// The file is damaged: a hash or an HMAC does not match, a block is
    /// missing or follows the last, or a structure is malformed.
    Damaged,
    /// The plaintext is longer than [`MAX_PLAIN`], or the file is too long
    /// to hold one.
    TooLarge,
    /// The contents do not fit the plaintext length asked for: the bytes
    /// they take.
    Full(usize),
    /// A file just made would not read: the document written does not read
    /// back as it was, or the container does not open under the keys it
    /// was made with.
    ReadBack,
}

/// A file opened.
pub struct Opened {
    /// What the header stated.
    pub header: Header,
    /// The XML document and whatever followed it in the plaintext (the line
    /// feeds a vault is padded with), in a buffer of its own: the
    /// attachments share the body, which does not hold it.
    pub document: Shared,
    /// The inner random stream's key, for [`InnerStream::new`].
    pub inner_key: Zeroizing<Vec<u8>>,
    /// The attachments of the inner header, in order.
    pub binaries: Vec<Binary>,
}

/// What a file is made from.
pub struct Contents<'a> {
    /// The XML document, its protected values encrypted under
    /// [`inner_key_of`] the random bytes sealed with.
    pub document: &'a [u8],
    /// The inner header's attachments.
    pub binaries: Binaries<'a>,
    /// The outer header's public custom data, to be written as it was
    /// read.
    pub public_custom_data: Option<&'a [u8]>,
}

/// The minor version (0 or 1) of a KDBX 4 file, from its first twelve
/// bytes: [`Error::NotKdbx`] for a file that is not KDBX, and
/// [`Error::Older`] for a KDBX 3.1 file, which is named and not opened.
pub fn recognise(bytes: &[u8]) -> Result<u16, Error> {
    header::minor_of(bytes)
}

/// The inner stream's key a file sealed with `random` carries, which the
/// document's protected values are encrypted under before sealing.
pub fn inner_key_of(random: &[u8; RANDOM_LEN]) -> &[u8] {
    &random[INNER_KEY_AT..]
}

/// What a file's header states, with the cost checked against §3's limits
/// and the length against the largest vault, before any work.
pub fn inspect(bytes: &[u8]) -> Result<Header, Error> {
    checked_header(bytes).map(|parsed| parsed.header)
}

/// Costs a SHA-256 of the header (the check [`inspect`] makes): the length
/// of a file's header with its hash and its HMAC, which the plaintext's
/// length and the blocks' framing follow.
pub fn head_len(bytes: &[u8]) -> Result<usize, Error> {
    checked_header(bytes).map(|parsed| parsed.len + 64)
}

/// The plaintext a file takes to hold an XML document of `document_len`
/// bytes and these attachments: the inner header (with an inner key as
/// long as Faraday writes) and the document, before any padding.
pub fn plain_needs(binaries: &Binaries<'_>, document_len: usize) -> usize {
    inner::written_len(&[0u8; RANDOM_LEN - INNER_KEY_AT], binaries).saturating_add(document_len)
}

/// Costs a read of `document`, linear in its size (at most the vault's largest, [`MAX_PLAIN`]), and
/// nothing when there is no attachment: the attachments a seal of
/// `document` writes (VAULT §7.1). An attachment no entry, history entry or
/// recycle bin entry refers to is emptied in place and the trailing ones
/// are left off; when a reference cannot be understood every attachment is
/// kept.
pub fn kept_binaries<'a>(binaries: &'a [Binary], document: &[u8]) -> Binaries<'a> {
    if binaries.is_empty() {
        return Binaries::all(binaries);
    }
    match xml::referenced(document, binaries.len()) {
        Some(used) => Binaries::used(binaries, &used),
        None => Binaries::all(binaries),
    }
}

/// Costs an Argon2 run at the cost and function `header` states, for a
/// passphrase and salt of no meaning, the keys discarded: the time an
/// unlock of this file takes, measured on this computer. The cost is
/// checked against §3's limits first.
pub fn measure(c: &Compute, header: &Header) -> Result<(), Error> {
    keys::file_keys(
        c,
        b"measure",
        &[0u8; 32],
        header.kdf,
        &header.cost,
        &[0u8; 32],
    )
    .map(|_| ())
}

/// The header read, its hash and the file's length checked.
fn checked_header(bytes: &[u8]) -> Result<header::Parsed, Error> {
    let parsed = header::read_fields(bytes)?;
    let after = parsed.len + 64;
    if bytes.len() < after {
        return Err(Error::Damaged);
    }
    if Sha256::digest(&bytes[..parsed.len]).as_slice() != &bytes[parsed.len..parsed.len + 32] {
        return Err(Error::Damaged);
    }
    stored_fits(bytes.len() - after)?;
    Ok(parsed)
}

/// Whether a body of `stored` bytes after the header is no longer than the
/// largest vault's ([`MAX_STORED`]): the check made before any work.
fn stored_fits(stored: usize) -> Result<(), Error> {
    if stored > MAX_STORED {
        return Err(Error::TooLarge);
    }
    Ok(())
}

/// A passphrase's NFC form as bytes, in a buffer that wipes.
pub fn passphrase_bytes(passphrase: &str) -> Result<Zeroizing<Vec<u8>>, Error> {
    let nfc = crate::nfc(passphrase).map_err(Error::Memory)?;
    if nfc.is_empty() {
        return Err(Error::EmptyPassphrase);
    }
    let mut out = Zeroizing::new(Vec::new());
    out.try_reserve_exact(nfc.len())
        .map_err(|_| Error::Memory(1))?;
    out.extend_from_slice(nfc.as_bytes());
    Ok(out)
}

/// Costs an Argon2 run at the cost the header states. Opens the KDBX file
/// `bytes` under `passphrase` (put in NFC here): the header and the cost
/// checked first, then the keys, the header's HMAC, every block's HMAC,
/// the decryption, the inflation and the inner header.
pub fn open(c: &Compute, bytes: &[u8], passphrase: &str) -> Result<Opened, Error> {
    open_within(c, bytes, passphrase, &mut |_| Ok(()), &mut |_, _| Ok(()))
}

/// Costs what [`open`] costs. [`open`] for a caller that has a budget:
/// `budget` is asked, for a gzip body, with the length the member states
/// once the HMACs have verified, before the buffer of that length is
/// allocated; `Err(mib)` is the memory that was not found
/// ([`Error::Memory`]) and nothing is allocated (F206 point 5). `parsed` is
/// asked once the inner header is read, with the XML document's length and
/// the attachments' together, before the document is copied out of the
/// body: a file it refuses takes no copy (F206 point 4).
pub fn open_within(
    c: &Compute,
    bytes: &[u8],
    passphrase: &str,
    budget: &mut dyn FnMut(usize) -> Result<(), u32>,
    parsed_fn: &mut dyn FnMut(usize, usize) -> Result<(), u32>,
) -> Result<Opened, Error> {
    let nfc = passphrase_bytes(passphrase)?;
    let parsed = checked_header(bytes)?;
    let keys = keys::file_keys(
        c,
        &nfc,
        &parsed.master_seed,
        parsed.header.kdf,
        &parsed.header.cost,
        &parsed.salt,
    )?;
    let body = decrypted(bytes, &parsed, &keys)?;
    let (document, inner) = read_body(body, parsed.header.compression, budget, parsed_fn)?;
    Ok(Opened {
        header: parsed.header,
        document,
        inner_key: inner.key,
        binaries: inner.binaries,
    })
}

/// The body of `bytes`, whose header `parsed` is, under the keys `keys`
/// (derived from the passphrase): the header's HMAC, every block's HMAC
/// and the decryption checked, and the padding taken off.
fn decrypted(
    bytes: &[u8],
    parsed: &header::Parsed,
    keys: &keys::Keys,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    let header_end = parsed.len;
    let header_key = keys::block_key(&keys.hmac_base, keys::HEADER_BLOCK);
    if !keys::hmac_ok(
        &header_key,
        &[&bytes[..header_end]],
        &bytes[header_end + 32..header_end + 64],
    ) {
        return Err(Error::Passphrase);
    }
    // A body that is not compressed is the plaintext and a cipher's padding.
    let limit = match parsed.header.compression {
        Compression::None => MAX_PLAIN + 16,
        Compression::Gzip => MAX_STORED,
    };
    let mut body = blocks::join(&bytes[header_end + 64..], &keys.hmac_base, limit)?;
    match parsed.header.cipher {
        Cipher::ChaCha20 => {
            let mut cipher = ChaCha20::new_from_slices(keys.encryption.expose(), &parsed.iv)
                .map_err(|_| Error::Damaged)?;
            cipher.apply_keystream(&mut body);
        }
        Cipher::Aes256 => {
            let cipher = cbc::Decryptor::<aes::Aes256>::new_from_slices(
                keys.encryption.expose(),
                &parsed.iv,
            )
            .map_err(|_| Error::Damaged)?;
            let plain = unpad(&mut body, cipher)?;
            // The padding's bytes are wiped with the rest on drop.
            body.truncate(plain);
        }
    }
    Ok(body)
}

/// A decrypted, unpadded body read: inflated when it is gzip, its inner
/// header read, and the XML document (with whatever follows it) left, in a
/// buffer of its own; the attachments are ranges of the body.
fn read_body(
    body: Zeroizing<Vec<u8>>,
    compression: Compression,
    budget: &mut dyn FnMut(usize) -> Result<(), u32>,
    parsed: &mut dyn FnMut(usize, usize) -> Result<(), u32>,
) -> Result<(Shared, inner::Inner), Error> {
    let plain = match compression {
        Compression::None => body,
        Compression::Gzip => gzip::inflate(&body, MAX_PLAIN, budget)?,
    };
    if plain.len() > MAX_PLAIN {
        return Err(Error::TooLarge);
    }
    let mut plain = plain;
    let scanned = inner::scan(&plain)?;
    // What the document and the attachments take is checked before any copy
    // is made of them.
    let attachments = scanned
        .binaries
        .iter()
        .fold(0usize, |n, (_, r)| n.saturating_add(r.len()));
    parsed(plain.len() - scanned.document_at, attachments).map_err(Error::Memory)?;
    // The XML text goes into a buffer of its own and is wiped from the
    // body, so the attachments, which live as long as the vault is open, do
    // not keep every unprotected field of the document in plaintext with
    // them: the document's buffer is wiped and freed once it is parsed.
    let at = scanned.document_at;
    let mut text = Zeroizing::new(Vec::new());
    text.try_reserve_exact(plain.len() - at)
        .map_err(|_| Error::TooLarge)?;
    text.extend_from_slice(&plain[at..]);
    plain[at..].zeroize();
    plain.truncate(at);
    plain.shrink_to_fit();
    let plain = Shared::new(plain);
    let inner = scanned.into_inner(&plain)?;
    debug_assert_eq!(inner.document_at, at);
    Ok((Shared::new(text), inner))
}

/// Reads a body as a file whose HMACs verified would hold it, for the
/// fuzzer: `decrypted` is what the cipher gave (with AES-256's padding
/// still on it), so the padding, the inflation and the inner header are
/// reached without a passphrase. The length of the XML document read.
#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub fn fuzz_body(decrypted: &[u8], aes256: bool, gzip: bool) -> Result<usize, Error> {
    let mut body = Zeroizing::new(decrypted.to_vec());
    if aes256 {
        let plain = strip_padding(&body)?;
        body.truncate(plain);
    }
    let compression = if gzip {
        Compression::Gzip
    } else {
        Compression::None
    };
    // A run allocates at most this much for an inflation, so a mutated
    // trailer cannot ask for the ceiling on every run.
    const FUZZ_INFLATE: usize = 64 * 1024 * 1024;
    let mut budget = |n: usize| {
        if n > FUZZ_INFLATE {
            Err((n >> 20) as u32)
        } else {
            Ok(())
        }
    };
    read_body(body, compression, &mut budget, &mut |_, _| Ok(()))
        .map(|(document, _)| document.len())
}

/// Decrypts `body` in place under AES-CBC and returns the length of the
/// plaintext once PKCS #7 padding is removed. The padding is read only of
/// a body whose HMAC verified, so it is not an oracle.
fn unpad(body: &mut [u8], mut cipher: cbc::Decryptor<aes::Aes256>) -> Result<usize, Error> {
    let (blocks, tail) = InOutBuf::from(&mut *body).into_chunks();
    if !tail.is_empty() {
        return Err(Error::Damaged);
    }
    cipher.decrypt_blocks_inout(blocks);
    strip_padding(body)
}

/// The length of `body` without its PKCS #7 padding: a whole number of
/// blocks, ending in 1 to 16 bytes that each state their own count.
fn strip_padding(body: &[u8]) -> Result<usize, Error> {
    if !body.len().is_multiple_of(16) {
        return Err(Error::Damaged);
    }
    let pad = usize::from(*body.last().ok_or(Error::Damaged)?);
    let start = body.len().checked_sub(pad).ok_or(Error::Damaged)?;
    if !(1..=16).contains(&pad) || body[start..].iter().any(|&b| usize::from(b) != pad) {
        return Err(Error::Damaged);
    }
    Ok(start)
}

/// The keys of a seal, worked out before anything is built: Argon2id has
/// run and its memory is freed (F206 point 6).
pub struct SealKeys {
    keys: keys::Keys,
    cost: Cost,
}

/// Costs an Argon2id run at `cost`. The keys [`seal_keyed`] seals under:
/// the passphrase (put in NFC here) and `cost` are checked, then the key is
/// derived from the seed and salt `random` carries.
pub fn derive(
    c: &Compute,
    passphrase: &str,
    cost: Cost,
    random: &[u8; RANDOM_LEN],
) -> Result<SealKeys, Error> {
    let nfc = passphrase_bytes(passphrase)?;
    header::cost_in_limits(&cost)?;
    let (master_seed, rest) = random.split_at(32);
    let salt = &rest[12..44];
    let keys = keys::file_keys(c, &nfc, master_seed, Kdf::Argon2id, &cost, salt)?;
    Ok(SealKeys { keys, cost })
}

/// Costs an Argon2id run at `cost`, and a read of the file made without
/// one (its HMACs, its decryption and its inner header). [`derive`] and
/// [`seal_keyed`] in one.
pub fn seal(
    c: &Compute,
    passphrase: &str,
    cost: Cost,
    random: &[u8; RANDOM_LEN],
    contents: &Contents<'_>,
    plain_len: usize,
) -> Result<Vec<u8>, Error> {
    let keys = derive(c, passphrase, cost, random)?;
    seal_keyed(keys, random, contents, plain_len)
}

/// Costs a read of the file made (its HMACs, its decryption and its inner
/// header), no Argon2id. Writes a file as
/// Faraday writes one: 4.0, ChaCha20, Argon2id, no compression, the passphrase (put in NFC
/// here) alone, 1 MiB blocks, and a plaintext of exactly `plain_len`
/// bytes: the inner header, the document and line feeds after it. Every
/// random value comes from `random`, the same as [`derive`] was given.
pub fn seal_keyed(
    sealing: SealKeys,
    random: &[u8; RANDOM_LEN],
    contents: &Contents<'_>,
    plain_len: usize,
) -> Result<Vec<u8>, Error> {
    let SealKeys { keys, cost } = sealing;
    if plain_len > MAX_PLAIN {
        return Err(Error::TooLarge);
    }
    // The length first, so a refusal leaves no copy of the inner key in a
    // buffer that grew.
    let needs = inner::written_len(inner_key_of(random), &contents.binaries)
        .saturating_add(contents.document.len());
    if needs > plain_len {
        return Err(Error::Full(needs));
    }

    let (master_seed, rest) = random.split_at(32);
    let (iv, rest) = rest.split_at(12);
    let salt = &rest[..32];
    let cipher =
        ChaCha20::new_from_slices(keys.encryption.expose(), iv).map_err(|_| Error::Damaged)?;

    let head = header::write(master_seed, iv, salt, cost, contents.public_custom_data);
    let mut out = Vec::new();
    let frames = plain_len.div_ceil(blocks::BLOCK_LEN) + 1;
    let file_len = head.len() + 64 + plain_len + frames * blocks::BLOCK_OVERHEAD;
    out.try_reserve_exact(file_len)
        .map_err(|_| no_memory(file_len))?;
    out.extend_from_slice(&head);
    out.extend_from_slice(&Sha256::digest(&head));
    out.extend_from_slice(&keys::hmac(
        &keys::block_key(&keys.hmac_base, keys::HEADER_BLOCK),
        &[&head],
    ));
    // The plaintext is encrypted and framed where it lands in the file: no
    // other copy of it is made (F206 point 6).
    let mut framer = blocks::Framer::new(&mut out, cipher, &keys.hmac_base);
    let mut digest = Sha256::new();
    let mut put = |piece: &[u8]| {
        digest.update(piece);
        framer.put(piece);
    };
    inner::write(inner_key_of(random), &contents.binaries, &mut put);
    put(contents.document);
    let mut pad = plain_len - needs;
    let lines = [b'\n'; 4096];
    while pad > 0 {
        let n = pad.min(lines.len());
        put(&lines[..n]);
        pad -= n;
    }
    framer.finish();
    read_back(&out, &keys, &digest.finalize())?;
    Ok(out)
}

/// Costs a hash of the file, its blocks' HMACs, one decryption and one
/// inner header read, no Argon2id: opens the file `out` just made with the
/// keys it was made under, and requires the plaintext to hash to `written`.
/// [`Error::ReadBack`] when the header, a block, the cipher or the inner
/// header does not read, or the plaintext is not the one written;
/// [`Error::Memory`] when the buffer to read into cannot be had, which says
/// nothing of the file.
fn read_back(out: &[u8], keys: &keys::Keys, written: &[u8]) -> Result<(), Error> {
    let read = || -> Result<(), Error> {
        let parsed = checked_header(out)?;
        let body = decrypted(out, &parsed, keys)?;
        if Sha256::digest(&body).as_slice() != written {
            return Err(Error::Damaged);
        }
        read_body(
            body,
            parsed.header.compression,
            &mut |_| Ok(()),
            &mut |_, _| Ok(()),
        )
        .map(|_| ())
    };
    read().map_err(|e| match e {
        Error::Memory(mib) => Error::Memory(mib),
        _ => Error::ReadBack,
    })
}

/// A buffer of `bytes` that cannot be had: the computer's memory, not a file
/// too long. The MiB asked for, rounded up.
fn no_memory(bytes: usize) -> Error {
    Error::Memory(u32::try_from(bytes.div_ceil(1 << 20)).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests;
