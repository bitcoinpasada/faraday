//! KEF, Krux's encrypted code, read and written in full (`docs/KEF.md`):
//! the twelve versions both ways, tolerant in what it reads (§4.2) and
//! strict in what it writes (§4.1), with compression that stops at 100 KiB
//! (§7.1), the defaults of §10 and the choice of a version.
//!
//! The envelope and the text forms are `faraday-codec`'s `kef` module
//! (nothing there is cryptographic). What is here derives the key, which
//! costs a PBKDF2 of the envelope's count that can be stopped, and runs the
//! ciphers. Every function that derives a key takes `c: &Compute`, says
//! what it costs, takes bytes and keeps no key: the key is worked out,
//! used and wiped inside the call. Plaintext and the working copies are
//! [`Zeroizing`], as `faraday-pgp`'s are.
//!
//! `no_std` + `alloc`.

#![no_std]
// clippy.toml has the refusal of a copy, text or second buffer of a secret
// (docs/SECRET-LEAKS.md §6); it is on for the whole crate outside tests. A
// legitimate use carries `#[expect(clippy::disallowed_methods, reason = "...")]`.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]
#![cfg_attr(test, allow(clippy::disallowed_methods))]

extern crate alloc;

use alloc::vec::Vec;

use aes::Aes256;
use aes::cipher::array::Array;
use aes::cipher::consts::{U4, U12};
use aes::cipher::inout::InOutBuf;
use aes::cipher::{
    BlockCipherDecrypt, BlockCipherEncrypt, BlockModeDecrypt, BlockModeEncrypt, KeyInit, KeyIvInit,
    StreamCipher,
};
use aes_gcm::{AeadInOut, AesGcm};
use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, pbkdf2_hmac_sha256_block};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

pub use faraday_codec::kef::{
    Envelope, Form, Mode, Padding, Recognized, Refusal, Spec, VERSIONS, recognize, spec_of, unwrap,
};

/// The most bytes a compressed version's contents inflate to: Krux's own
/// limit (`DEFLATEIO_MAX_DECOMPRESSED_SIZE`), 100 KiB.
pub const INFLATE_LIMIT: usize = 102_400;

/// The most bytes a compressed version is written for, since below it a
/// match cannot reach beyond the 1,024-byte window Krux reads
/// (`docs/KEF.md` §4.1, §10).
pub const COMPRESS_MAX: usize = 1_024;

/// The default version: AES-GCM (`docs/KEF.md` §10).
pub const DEFAULT_VERSION: u8 = 20;
/// The default id: 8 random bytes.
pub const DEFAULT_ID_LEN: usize = 8;
/// The id one level down, which gives NIST SP 800-132's 128 bits.
pub const LONG_ID_LEN: usize = 16;
/// The default PBKDF2 count, Krux's own.
pub const DEFAULT_COUNT: u32 = 100_000;
/// The lowest count a code states and the lowest Faraday offers.
pub const COUNT_MIN: u32 = 10_000;
/// The highest of Krux's setting, and of what Faraday offers.
pub const COUNT_SETTING_MAX: u32 = 500_000;
/// The most Krux itself writes: a count typed for one code.
pub const COUNT_KRUX_MAX: u32 = 550_000;
/// The most characters of a passphrase Faraday writes under.
pub const PASSPHRASE_MAX: usize = 200;
/// A count as Krux adds its change to a setting: the setting and a random
/// number under a tenth of it (`KEFEnvelope.__init__`), so a reused id or
/// label does not mean a reused key. At most `COUNT_KRUX_MAX - 1` for a
/// setting up to 500,000.
pub fn count_with_change(setting: u32, random: u32) -> u32 {
    match setting / 10 {
        0 => setting,
        span => setting + random % span,
    }
}

/// Whether `text` is a passphrase Faraday writes a code under: printable
/// ASCII (0x20 to 0x7E), 1 to 200 characters (`docs/KEF.md` §9).
pub fn writable_passphrase(text: &str) -> bool {
    writable_passphrase_bytes(text.as_bytes())
}

fn writable_passphrase_bytes(bytes: &[u8]) -> bool {
    (1..=PASSPHRASE_MAX).contains(&bytes.len()) && bytes.iter().all(|b| (0x20..=0x7e).contains(b))
}

/// A copy of `bytes` in a buffer wiped on drop: the only copy made of them.
#[expect(
    clippy::disallowed_methods,
    reason = "the copy goes straight into Zeroizing, which wipes it"
)]
fn wiped(bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(bytes.to_vec())
}

/// The byte strings a typed passphrase is tried as when reading: as
/// typed, and, if it holds anything outside ASCII, its NFC and NFD forms
/// where they differ (`docs/KEF.md` §9). At most three; each is a full
/// PBKDF2.
pub fn reading_passphrases(text: &str) -> Vec<Zeroizing<Vec<u8>>> {
    let mut out = alloc::vec![wiped(text.as_bytes())];
    if !text.is_ascii() {
        // `char` has no wipe: the forms are held in Zeroizing, with room
        // for the longest (four bytes a char), so they never reallocate.
        // unicode-normalization's own buffers are not wiped.
        let room = text.len() * 4;
        let mut nfc = Zeroizing::new(Vec::<char>::with_capacity(room));
        nfc.extend(text.nfc());
        let mut nfd = Zeroizing::new(Vec::<char>::with_capacity(room));
        nfd.extend(text.nfd());
        for chars in [nfc, nfd] {
            // Its final size is known, so it never reallocates.
            let size: usize = chars.iter().map(|ch| ch.len_utf8()).sum();
            let mut form = Zeroizing::new(Vec::with_capacity(size));
            for &ch in chars.iter() {
                form.extend_from_slice(ch.encode_utf8(&mut [0; 4]).as_bytes());
            }
            if !out.iter().any(|seen| **seen == *form) {
                out.push(form);
            }
        }
    }
    out
}

/// Why a code could not be made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SealError {
    /// The version, id, count or payload cannot be written.
    Refused(Refusal),
    /// Nothing to seal.
    Empty,
    /// The IV is not the length the version carries (none for ECB).
    Iv,
    /// ECB would write two equal blocks, which show; `instead` is the
    /// version of the same shape in CBC.
    Duplicate {
        /// The version that would work.
        instead: u8,
    },
    /// NUL padding cannot tell the end of this plaintext (or of its check)
    /// from its padding; `instead` is the version with PKCS #7 padding.
    NulEnd {
        /// The version that would work.
        instead: u8,
    },
    /// A compressed version is written only up to [`COMPRESS_MAX`] bytes.
    TooLong,
    /// The passphrase is not one Faraday writes under
    /// ([`writable_passphrase`]).
    Passphrase,
    /// Told to stop while the key was worked out.
    Cancelled,
}

/// A code that did not open. Every failure after the passphrase is
/// entered reads the same (`docs/KEF.md` §7.4): the wrong passphrase, a
/// check that fails, padding, a stream that will not inflate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenError {
    /// The code did not open.
    Failed,
    /// Told to stop while the key was worked out.
    Cancelled,
}

/// A code opened. The check is short, so this is only probably the
/// person's: a wrong passphrase passes about 1 time in 16,777,216 for
/// version 5 and 1 in 4,294,967,296 for the 4-byte versions
/// (`docs/KEF.md` §7.3), and what comes out is then random bytes.
pub struct Opened {
    /// The contents.
    pub plaintext: Zeroizing<Vec<u8>>,
    /// Whether the check passed only after putting NULs back (§4.2), which
    /// a writer that skipped §4.1 causes; the person gets a note.
    pub retried: bool,
}

/// The key, 32 bytes, for as long as one call.
type Key = Secret<[u8; 32]>;

/// Costs a PBKDF2 of `count` rounds: the key `PBKDF2-HMAC-SHA-256(
/// passphrase, id, count)`. `between` is called with the rounds done so
/// far and stops the work when it returns `false`, so a count of
/// 100,000,000 can be cancelled at once.
fn derive(
    c: &Compute,
    passphrase: &[u8],
    id: &[u8],
    count: u32,
    between: &mut dyn FnMut(u32) -> bool,
) -> Option<Key> {
    pbkdf2_hmac_sha256_block(c, passphrase, id, count, between)
}

fn sha256_of(parts: &[&[u8]]) -> Zeroizing<[u8; 32]> {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    let mut out = Zeroizing::new([0u8; 32]);
    h.finalize_into((&mut *out).into());
    out
}

type Gcm4 = AesGcm<Aes256, U12, U4>;

fn aes(key: &[u8; 32]) -> Aes256 {
    Aes256::new_from_slice(key).expect("a 32-byte key")
}

/// ECB in place over whole blocks.
fn ecb(key: &[u8; 32], data: &mut [u8], encrypt: bool) {
    let cipher = aes(key);
    let (blocks, tail) = InOutBuf::from(data).into_chunks();
    debug_assert!(tail.is_empty());
    if encrypt {
        cipher.encrypt_blocks_inout(blocks);
    } else {
        cipher.decrypt_blocks_inout(blocks);
    }
}

/// CBC in place over whole blocks.
fn cbc(key: &[u8; 32], iv: &[u8], data: &mut [u8], encrypt: bool) {
    let (blocks, tail) = InOutBuf::from(data).into_chunks();
    debug_assert!(tail.is_empty());
    if encrypt {
        cbc::Encryptor::<Aes256>::new_from_slices(key, iv)
            .expect("a 32-byte key and a 16-byte IV")
            .encrypt_blocks_inout(blocks);
    } else {
        cbc::Decryptor::<Aes256>::new_from_slices(key, iv)
            .expect("a 32-byte key and a 16-byte IV")
            .decrypt_blocks_inout(blocks);
    }
}

/// CTR in place: the 12-byte IV and a 4-byte big-endian counter from 0.
fn ctr(key: &[u8; 32], iv: &[u8], data: &mut [u8]) {
    let mut block = [0u8; 16];
    block[..12].copy_from_slice(iv);
    ctr::Ctr32BE::<Aes256>::new_from_slices(key, &block)
        .expect("a 32-byte key and a 16-byte counter block")
        .apply_keystream(data);
}

/// Raw deflate of `data`.
fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(data, 9)
}

/// Raw inflate of `data` to at most [`INFLATE_LIMIT`] bytes; any window up
/// to 32 KiB (`docs/KEF.md` §7.1).
fn inflate(data: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    miniz_oxide::inflate::decompress_to_vec_with_limit(data, INFLATE_LIMIT)
        .ok()
        .map(Zeroizing::new)
}

/// The version to try instead when a refusal of §4.1 names one.
fn pkcs_counterpart(v: u8) -> u8 {
    match v {
        5 => 6,
        10 => 11,
        0 => 6,
        _ => 11,
    }
}

fn cbc_counterpart(v: u8) -> u8 {
    match v {
        0 => 1,
        5 => 10,
        6 => 11,
        _ => 12,
    }
}

/// What `plaintext` becomes before it is encrypted under `s`: compressed
/// if the version is, its hidden check on the end, padded; and how many of
/// its bytes come before the padding (what an exposed check hashes). Every
/// refusal of §4.1 that needs no key is here.
fn prepare(s: &Spec, plaintext: &[u8]) -> Result<(Zeroizing<Vec<u8>>, usize), SealError> {
    let version = s.version;
    let nul = s.padding == Padding::Nul;
    let mut plain: Zeroizing<Vec<u8>> = if s.compressed {
        if plaintext.len() > COMPRESS_MAX {
            return Err(SealError::TooLong);
        }
        Zeroizing::new(deflate(plaintext))
    } else {
        wiped(plaintext)
    };
    if s.auth > 0 && s.mode != Mode::Gcm {
        if nul && plain.last() == Some(&0) {
            return Err(SealError::NulEnd {
                instead: pkcs_counterpart(version),
            });
        }
    } else if s.auth < 0 {
        // The hidden check is encrypted after the plaintext.
        let n = (-s.auth) as usize;
        let h = sha256_of(&[&plain]);
        if nul && h[n - 1] == 0 {
            return Err(SealError::NulEnd {
                instead: pkcs_counterpart(version),
            });
        }
        plain.reserve(n + 16);
        plain.extend_from_slice(&h[..n]);
    }
    let unpadded = plain.len();
    match s.padding {
        Padding::None => {}
        Padding::Nul | Padding::Pkcs7 => {
            let short = (16 - plain.len() % 16) % 16;
            let pad = if s.padding == Padding::Pkcs7 {
                if short == 0 { 16 } else { short }
            } else {
                short
            };
            let byte = if s.padding == Padding::Pkcs7 {
                pad as u8
            } else {
                0
            };
            plain.reserve(pad);
            plain.extend(core::iter::repeat_n(byte, pad));
        }
    }
    if s.mode == Mode::Ecb {
        let mut blocks: Vec<&[u8]> = plain.chunks(16).collect();
        blocks.sort_unstable();
        if blocks.windows(2).any(|w| w[0] == w[1]) {
            return Err(SealError::Duplicate {
                instead: cbc_counterpart(version),
            });
        }
    }
    Ok((plain, unpadded))
}

/// Costs a hash of the plaintext, a compression of up to 1,024 bytes and
/// the cipher over it: `plaintext` sealed under a key already worked out,
/// as `Cipher.encrypt` does with `fail_unsafe` on, wrapped in its envelope
/// (`docs/KEF.md` §4, §4.1). `iv` is fresh bytes of the session's, 12 or
/// 16 as the version needs, none for ECB. This is the entry the fuzz
/// target and the tests use, with a key of their own; it takes any count
/// the envelope can state, where [`seal`] writes only what §10 allows.
pub fn seal_with_key(
    key: &[u8; 32],
    plaintext: &[u8],
    version: u8,
    id: &[u8],
    count: u32,
    iv: &[u8],
) -> Result<Vec<u8>, SealError> {
    let s = spec_of(version).ok_or(SealError::Refused(Refusal::Version))?;
    if id.len() > faraday_codec::kef::ID_MAX {
        return Err(SealError::Refused(Refusal::IdLength));
    }
    if faraday_codec::kef::quotient_of(count).is_none() {
        return Err(SealError::Refused(Refusal::Count));
    }
    if plaintext.is_empty() {
        return Err(SealError::Empty);
    }
    if iv.len() != s.iv_len() {
        return Err(SealError::Iv);
    }
    let (mut plain, unpadded) = prepare(s, plaintext)?;
    // The exposed check of 5 and 10 hashes the plaintext with the key.
    let mut exposed: Option<[u8; 4]> = None;
    if s.auth > 0 && s.mode != Mode::Gcm {
        let h = sha256_of(&[&[version], iv, &plain[..unpadded], key]);
        let mut a = [0u8; 4];
        a[..s.auth as usize].copy_from_slice(&h[..s.auth as usize]);
        exposed = Some(a);
    }
    let mut tag = [0u8; 4];
    match s.mode {
        Mode::Ecb => ecb(key, &mut plain, true),
        Mode::Cbc => cbc(key, iv, &mut plain, true),
        Mode::Ctr => ctr(key, iv, &mut plain),
        Mode::Gcm => {
            let nonce = Array::try_from(iv).expect("a 12-byte IV");
            let t = Gcm4::new_from_slice(key)
                .expect("a 32-byte key")
                .encrypt_inout_detached(&nonce, &[], InOutBuf::from(&mut plain[..]))
                .expect("a buffer within GCM's bound");
            tag.copy_from_slice(&t);
        }
    }
    let mut payload = Vec::with_capacity(iv.len() + plain.len() + 4);
    payload.extend_from_slice(iv);
    payload.extend_from_slice(&plain);
    if s.mode == Mode::Gcm {
        payload.extend_from_slice(&tag);
    } else if let Some(a) = exposed {
        payload.extend_from_slice(&a[..s.auth as usize]);
    }
    faraday_codec::kef::wrap(id, version, count, &payload).map_err(SealError::Refused)
}

/// What a code is made as: the version, the id (the PBKDF2 salt), the count
/// and the IV.
#[derive(Clone, Copy, Debug)]
pub struct Sealing<'a> {
    /// The version byte.
    pub version: u8,
    /// The id.
    pub id: &'a [u8],
    /// The PBKDF2 count.
    pub count: u32,
    /// Fresh bytes of the session's: none for ECB, else 12 or 16.
    pub iv: &'a [u8],
}

/// Costs a PBKDF2 of `count` rounds: `plaintext` sealed under
/// `passphrase` as version `version` with `id` and `count`, wrapped in its
/// envelope's bytes. `iv` is fresh bytes of the session's (none for ECB,
/// else 12 or 16). The passphrase is one Faraday writes under
/// ([`writable_passphrase`], `docs/KEF.md` §9) and the count at most
/// [`COUNT_KRUX_MAX`] (§10); `Enter as hex` is for reading only.
/// `between` is called with
/// the rounds done and stops the work when it returns `false`.
pub fn seal(
    c: &Compute,
    passphrase: &[u8],
    plaintext: &[u8],
    how: &Sealing<'_>,
    between: &mut dyn FnMut(u32) -> bool,
) -> Result<Vec<u8>, SealError> {
    let Sealing {
        version,
        id,
        count,
        iv,
    } = *how;
    // Refuse what is refused before the work, not after it: everything
    // that needs no key.
    if !writable_passphrase_bytes(passphrase) {
        return Err(SealError::Passphrase);
    }
    let s = spec_of(version).ok_or(SealError::Refused(Refusal::Version))?;
    if id.len() > faraday_codec::kef::ID_MAX {
        return Err(SealError::Refused(Refusal::IdLength));
    }
    if faraday_codec::kef::quotient_of(count).is_none() || count > COUNT_KRUX_MAX {
        return Err(SealError::Refused(Refusal::Count));
    }
    if plaintext.is_empty() {
        return Err(SealError::Empty);
    }
    if iv.len() != s.iv_len() {
        return Err(SealError::Iv);
    }
    prepare(s, plaintext)?;
    let key = derive(c, passphrase, id, count, between).ok_or(SealError::Cancelled)?;
    seal_with_key(key.expose(), plaintext, version, id, count, iv)
}

/// Whether the hidden or exposed check passes for the NUL- or
/// PKCS-stripped `decrypted`, with Krux's retries (`_authenticate`,
/// `docs/KEF.md` §4.2). Gives the plaintext and whether a retry was
/// needed.
fn authenticate(
    s: &Spec,
    key: &[u8; 32],
    iv: &[u8],
    decrypted: &[u8],
    exposed: &[u8],
) -> Option<(Zeroizing<Vec<u8>>, bool)> {
    let mut end = decrypted.len();
    match s.padding {
        Padding::Nul => {
            while end > 0 && decrypted[end - 1] == 0 {
                end -= 1;
            }
        }
        Padding::Pkcs7 => {
            let n = usize::from(*decrypted.last()?);
            if n == 0 || n > decrypted.len() {
                return None;
            }
            end -= n;
        }
        Padding::None => {}
    }
    let removed = decrypted.len() - end;
    let stripped = &decrypted[..end];
    let width = s.auth.unsigned_abs() as usize;
    // A hidden check is the last bytes of what is left.
    let (body, mut hidden) = if s.auth < 0 {
        let at = stripped.len().checked_sub(width)?;
        (&stripped[..at], wiped(&stripped[at..]))
    } else {
        (stripped, Zeroizing::new(Vec::new()))
    };
    let tries = if s.padding == Padding::Nul {
        removed.min(width + 1) + 1
    } else {
        1
    };
    let mut plain = Zeroizing::new(Vec::with_capacity(body.len() + width + 2));
    plain.extend_from_slice(body);
    for attempt in 0..tries {
        let sum = if s.auth > 0 {
            sha256_of(&[&[s.version], iv, &plain, key])
        } else {
            sha256_of(&[&plain])
        };
        let want: &[u8] = if s.auth > 0 { exposed } else { &hidden };
        if bool::from(sum[..width].ct_eq(want)) {
            return Some((plain, attempt > 0));
        }
        if s.auth < 0 {
            // The check had a NUL stripped: move a byte of it to the
            // plaintext and put a NUL on its end.
            plain.push(hidden[0]);
            hidden.remove(0);
            hidden.push(0);
        } else {
            plain.push(0);
        }
    }
    None
}

/// Costs a hash and a cipher over the envelope's payload, and an inflate
/// of up to 100 KiB: the contents of `envelope` under a key already
/// worked out, with no work to stop: the entry the fuzz target and the tests use. Every
/// failure is [`OpenError::Failed`].
pub fn open_with_key(key: &[u8; 32], envelope: &Envelope) -> Result<Opened, OpenError> {
    let s = spec_of(envelope.version).ok_or(OpenError::Failed)?;
    if !faraday_codec::kef::payload_fits(s, &envelope.payload) {
        return Err(OpenError::Failed);
    }
    let iv_len = s.iv_len();
    let (iv, rest) = envelope.payload.split_at(iv_len);
    let exposed_len = s.auth.max(0) as usize;
    let (cipher, exposed) = rest.split_at(rest.len() - exposed_len);
    let mut buf = wiped(cipher);
    let (plain, retried) = match s.mode {
        Mode::Ecb => {
            ecb(key, &mut buf, false);
            authenticate(s, key, iv, &buf, exposed).ok_or(OpenError::Failed)?
        }
        Mode::Cbc => {
            cbc(key, iv, &mut buf, false);
            authenticate(s, key, iv, &buf, exposed).ok_or(OpenError::Failed)?
        }
        Mode::Ctr => {
            ctr(key, iv, &mut buf);
            authenticate(s, key, iv, &buf, exposed).ok_or(OpenError::Failed)?
        }
        Mode::Gcm => {
            let nonce = Array::try_from(iv).map_err(|_| OpenError::Failed)?;
            let tag = Array::try_from(exposed).map_err(|_| OpenError::Failed)?;
            Gcm4::new_from_slice(key)
                .map_err(|_| OpenError::Failed)?
                .decrypt_inout_detached(&nonce, &[], InOutBuf::from(&mut buf[..]), &tag)
                .map_err(|_| OpenError::Failed)?;
            (core::mem::take(&mut buf), false)
        }
    };
    let plaintext = if s.compressed {
        inflate(&plain).ok_or(OpenError::Failed)?
    } else {
        plain
    };
    if plaintext.is_empty() {
        return Err(OpenError::Failed);
    }
    Ok(Opened { plaintext, retried })
}

/// Costs a PBKDF2 of the envelope's `count` rounds: the contents of
/// `envelope` under `passphrase`, bytes as typed (UTF-8, or hex's bytes).
/// `between` is called with the rounds done and stops the work when it
/// returns `false`: [`OpenError::Cancelled`]. A wrong passphrase, a check
/// that fails, padding and a stream that will not inflate are all
/// [`OpenError::Failed`] (`docs/KEF.md` §7.4).
pub fn open(
    c: &Compute,
    passphrase: &[u8],
    envelope: &Envelope,
    between: &mut dyn FnMut(u32) -> bool,
) -> Result<Opened, OpenError> {
    // Refuse what is refused before the work, not after it.
    let s = spec_of(envelope.version).ok_or(OpenError::Failed)?;
    if !faraday_codec::kef::payload_fits(s, &envelope.payload) {
        return Err(OpenError::Failed);
    }
    let key =
        derive(c, passphrase, &envelope.id, envelope.count, between).ok_or(OpenError::Cancelled)?;
    open_with_key(key.expose(), envelope)
}

/// The versions of one mode that suit `plaintext`, as Krux's
/// `suggest_versions` chooses (`docs/KEF.md` §10): the first is the
/// choice. Versions 0 and 1 are never among them. Empty for an empty
/// plaintext.
pub fn suggest_versions(plaintext: &[u8], mode: Mode) -> Vec<u8> {
    const SMALL: usize = 32;
    const BIG: usize = 120;
    let len = plaintext.len();
    let Some(&last) = plaintext.last() else {
        return Vec::new();
    };
    let mut blocks: Vec<&[u8]> = plaintext.chunks(16).collect();
    blocks.sort_unstable();
    blocks.dedup();
    let duplicates = blocks.len() * 16 < len;
    let nul_end = last == 0;
    let mut out = Vec::new();
    for s in VERSIONS.iter().filter_map(|&v| spec_of(v)) {
        let pkcs = s.padding == Padding::Pkcs7;
        let nul = s.padding == Padding::Nul;
        if s.mode != mode || (s.compressed && len > COMPRESS_MAX) {
            continue;
        }
        if duplicates && mode == Mode::Ecb && !s.compressed {
            continue;
        }
        if s.auth == -16 {
            continue;
        }
        if len <= SMALL {
            if !(mode == Mode::Ecb && duplicates) {
                if pkcs && !nul_end {
                    continue;
                }
                if s.compressed {
                    continue;
                }
            }
        } else {
            if nul {
                continue;
            }
            if mode == Mode::Ecb && duplicates {
            } else if !(BIG..=COMPRESS_MAX).contains(&len) {
                // Over COMPRESS_MAX a compressed version is refused, so
                // the uncompressed one stays.
                if s.compressed {
                    continue;
                }
            } else if !s.compressed {
                continue;
            }
        }
        if nul_end && nul {
            continue;
        }
        out.push(s.version);
    }
    out
}
