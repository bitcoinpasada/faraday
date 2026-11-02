//! The age file format (age-encryption.org/v1, C2SP), the passphrase
//! recipient only: a file whose one stanza is `scrypt`, in its binary
//! form.
//!
//! Written here for F178's reason "only the part used is written": age
//! has many recipient types, an armor and a plugin protocol, and Faraday
//! takes one file shape, a passphrase over scrypt, so what is written is
//! that shape's header, its header MAC and its 64 KiB STREAM payload, over
//! primitives that are established or already in the tree (scrypt and
//! HKDF-SHA-256 from `faraday-crypto`, ChaCha20-Poly1305 from RustCrypto).
//! The tests open the C2SP testkit's scrypt vectors
//! (`tools/vectors/age/`), made by the age authors' own generator.
//!
//! - [`recognise`] reads a file's header text and says what kind of file
//!   it is, with no key and no work.
//! - [`seal`] writes a file from a passphrase, the bytes of randomness
//!   the caller draws, and the plaintext.
//! - [`open`] reads one: every refusal the specification names for the
//!   scrypt stanza, the header and the payload is its own [`Error`], and a
//!   work factor above the caller's limit is refused before any work.
//!
//! The passphrase arrives in Unicode NFC, which is the caller's job as it
//! is for the vault's (age leaves the form to the application). Nothing
//! here draws randomness. ASCII armor and every other recipient type are
//! recognised and refused, never read.
//!
//! The salt, the file key and the payload nonce are 16 bytes each, taken
//! in that order from the 48 bytes the caller passes to [`seal`].

use alloc::vec::Vec;

use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, ChaCha20Poly1305, KeyInit, Nonce, Tag};
use faraday_crypto::work::Compute;
use faraday_crypto::{hkdf_sha256_expand, hmac_sha256, scrypt};
use zeroize::Zeroizing;

/// The first line of every v1 file.
const MAGIC: &[u8] = b"age-encryption.org/v1\n";
/// What an armored file begins with (RFC 7468's label for age).
const ARMOR_BEGIN: &[u8] = b"-----BEGIN AGE ENCRYPTED FILE-----";
/// What scrypt's salt is prefixed with.
const SCRYPT_LABEL: &[u8] = b"age-encryption.org/v1/scrypt";
/// The plaintext of one payload chunk.
const CHUNK: usize = 64 * 1024;
/// The Poly1305 tag after each chunk.
const TAG: usize = 16;
/// The payload's nonce, which follows the header.
const PAYLOAD_NONCE: usize = 16;
/// The longest base64 line of a stanza body.
const LINE: usize = 64;
/// The characters of a 16-byte value and of a 32-byte one in unpadded base64.
const B64_16: usize = 22;
const B64_32: usize = 43;

/// The least work factor (`log_n`) [`seal`] writes.
pub const MIN_LOG_N: u8 = 16;
/// The most [`seal`] writes: 1 GiB of memory.
pub const MAX_LOG_N: u8 = 20;

/// What [`recognise`] makes of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An age v1 file whose one stanza is a well-formed `scrypt` stanza,
    /// with the work factor it states (not yet checked against any limit).
    Scrypt {
        /// The base-two logarithm of scrypt's `N`.
        log_n: u8,
    },
    /// An age v1 header whose stanzas are not one well-formed scrypt
    /// stanza: another recipient type, several stanzas, or a scrypt stanza
    /// that breaks the specification.
    OtherRecipient,
    /// ASCII armor, which is not read.
    Armored,
    /// Not an age v1 file, or a header that does not parse.
    NotAge,
}

/// Why [`open`] refused a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The file is in ASCII armor, which is not read.
    Armored,
    /// The file does not begin with an age v1 header.
    NotAge,
    /// The header text breaks the grammar: a stanza, a line's length or
    /// characters, the end line or its MAC's encoding.
    Header,
    /// There is no scrypt stanza: the file is for another kind of recipient.
    OtherRecipient,
    /// A scrypt stanza is present beside another stanza.
    ScryptNotAlone,
    /// The scrypt stanza does not have three arguments, a canonical
    /// 16-byte salt, a work factor of plain decimal digits without a
    /// leading zero, and a body of exactly 32 bytes.
    ScryptStanza,
    /// The work factor is above the limit the caller set; no work was done.
    TooCostly {
        /// The work factor the file states, 255 where it states more.
        log_n: u8,
    },
    /// The passphrase does not open the file key.
    WrongPassphrase,
    /// The file key opened and the header MAC does not match.
    HeaderMac,
    /// The payload ends before a chunk: no nonce, or a chunk without its tag.
    Truncated,
    /// A chunk does not authenticate: changed, cut, reordered, or the
    /// payload has bytes after its final chunk.
    Corrupt,
    /// The final chunk is empty and is not the only one.
    EmptyFinalChunk,
    /// The plaintext does not fit in memory.
    Memory,
}

/// What kind of file this is, from its header text alone: no key, no
/// work, and nothing allocated.
#[must_use]
pub fn recognise(file: &[u8]) -> Kind {
    if is_armored(file) {
        return Kind::Armored;
    }
    let Ok(header) = parse_header(file) else {
        return Kind::NotAge;
    };
    if header.stanzas == 1
        && let Some(stanza) = &header.scrypt
        && let Ok(params) = stanza.params()
    {
        return Kind::Scrypt {
            log_n: params.log_n.min(255) as u8,
        };
    }
    Kind::OtherRecipient
}

/// Costs `1024 · 2^log_n` bytes of memory and one scrypt run (64 MiB at
/// `log_n` 16, 1 GiB at 20), then the plaintext's ChaCha20-Poly1305: an
/// age file under `passphrase`.
///
/// `random` is the 48 bytes the caller draws: the scrypt salt, the file
/// key and the payload nonce, 16 bytes each in that order. `passphrase` is
/// already in Unicode NFC. The file is binary, never armored.
///
/// # Panics
///
/// When `log_n` is not in [`MIN_LOG_N`]`..=`[`MAX_LOG_N`].
#[must_use]
pub fn seal(
    c: &Compute,
    passphrase: &[u8],
    log_n: u8,
    random: &[u8; 48],
    plaintext: &[u8],
) -> Vec<u8> {
    assert!(
        (MIN_LOG_N..=MAX_LOG_N).contains(&log_n),
        "age work factor out of range"
    );
    let salt: [u8; 16] = random[0..16].try_into().expect("16 bytes");
    let file_key: Zeroizing<[u8; 16]> =
        Zeroizing::new(random[16..32].try_into().expect("16 bytes"));
    let nonce: [u8; 16] = random[32..48].try_into().expect("16 bytes");

    let wrap = wrap_key(c, passphrase, &salt, u32::from(log_n)).expect("a valid work factor");
    let mut body = Zeroizing::new([0u8; 16]);
    body.copy_from_slice(&*file_key);
    let tag = wrap_cipher(&wrap)
        .encrypt_inout_detached(&Nonce::from([0u8; 12]), b"", InOutBuf::from(&mut body[..]))
        .expect("a file key is far below the cipher's limit");
    let mut wrapped = [0u8; 32];
    wrapped[..16].copy_from_slice(&*body);
    wrapped[16..].copy_from_slice(&tag);

    let mut out = Vec::with_capacity(
        260 + PAYLOAD_NONCE + plaintext.len() + (plaintext.len() / CHUNK + 1) * TAG,
    );
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(b"-> scrypt ");
    b64_push(&mut out, &salt);
    out.push(b' ');
    out.push(b'0' + log_n / 10);
    out.push(b'0' + log_n % 10);
    out.push(b'\n');
    b64_push(&mut out, &wrapped);
    out.extend_from_slice(b"\n---");
    let mac = header_mac(&file_key, &out);
    out.push(b' ');
    b64_push(&mut out, &mac);
    out.push(b'\n');

    out.extend_from_slice(&nonce);
    let cipher = payload_cipher(&file_key, &nonce);
    let mut chunks = plaintext.chunks(CHUNK).peekable();
    let mut counter = 0u64;
    // An empty payload is one empty final chunk.
    if chunks.peek().is_none() {
        seal_chunk(&cipher, counter, true, &[], &mut out);
    }
    while let Some(chunk) = chunks.next() {
        seal_chunk(&cipher, counter, chunks.peek().is_none(), chunk, &mut out);
        counter += 1;
    }
    out
}

/// Costs `1024 · 2^log_n` bytes of memory and one scrypt run, `log_n`
/// being the file's own and refused above `max_log_n` before it starts,
/// then the payload's ChaCha20-Poly1305: the plaintext of an age file
/// sealed under `passphrase`.
///
/// `passphrase` is already in Unicode NFC. Everything the specification
/// says an identity MUST refuse is refused, each as its own [`Error`]; no
/// plaintext is returned unless every chunk authenticates and the final
/// chunk is the last.
pub fn open(
    c: &Compute,
    passphrase: &[u8],
    file: &[u8],
    max_log_n: u8,
) -> Result<Zeroizing<Vec<u8>>, Error> {
    if is_armored(file) {
        return Err(Error::Armored);
    }
    let header = parse_header(file)?;
    let Some(stanza) = &header.scrypt else {
        return Err(Error::OtherRecipient);
    };
    if header.stanzas != 1 {
        return Err(Error::ScryptNotAlone);
    }
    let params = stanza.params()?;
    if params.log_n > u64::from(max_log_n) {
        return Err(Error::TooCostly {
            log_n: params.log_n.min(255) as u8,
        });
    }
    let wrap =
        wrap_key(c, passphrase, &params.salt, params.log_n as u32).map_err(|()| Error::Header)?;
    let mut key = Zeroizing::new([0u8; 16]);
    let (ct, tag) = params.body.split_at(16);
    key.copy_from_slice(ct);
    let tag = Tag::from(<[u8; 16]>::try_from(tag).expect("16 bytes"));
    wrap_cipher(&wrap)
        .decrypt_inout_detached(
            &Nonce::from([0u8; 12]),
            b"",
            InOutBuf::from(&mut key[..]),
            &tag,
        )
        .map_err(|_| Error::WrongPassphrase)?;

    let mac = header_mac(&key, &file[..header.mac_input_end]);
    if !equal(&mac, &header.mac) {
        return Err(Error::HeaderMac);
    }
    open_payload(&key, &file[header.payload_at..])
}

/// The payload after the header: its nonce, then chunks to the final one.
fn open_payload(file_key: &[u8; 16], payload: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    let (nonce, mut rest) = payload
        .split_at_checked(PAYLOAD_NONCE)
        .ok_or(Error::Truncated)?;
    let cipher = payload_cipher(file_key, nonce.try_into().expect("16 bytes"));
    let mut out = Zeroizing::new(Vec::new());
    out.try_reserve_exact(rest.len())
        .map_err(|_| Error::Memory)?;
    let mut counter = 0u64;
    loop {
        if rest.len() < TAG {
            return Err(Error::Truncated);
        }
        // A chunk is final when no full chunk and tag can follow it.
        let last = rest.len() <= CHUNK + TAG;
        let take = if last { rest.len() } else { CHUNK + TAG };
        let (chunk, tag) = rest[..take].split_at(take - TAG);
        let start = out.len();
        out.extend_from_slice(chunk);
        let tag = Tag::from(<[u8; 16]>::try_from(tag).expect("16 bytes"));
        cipher
            .decrypt_inout_detached(
                &chunk_nonce(counter, last),
                b"",
                InOutBuf::from(&mut out[start..]),
                &tag,
            )
            .map_err(|_| Error::Corrupt)?;
        if last {
            if chunk.is_empty() && counter > 0 {
                return Err(Error::EmptyFinalChunk);
            }
            return Ok(out);
        }
        counter += 1;
        rest = &rest[take..];
    }
}

/// Encrypts one chunk and appends it and its tag to `out`.
fn seal_chunk(
    cipher: &ChaCha20Poly1305,
    counter: u64,
    last: bool,
    plain: &[u8],
    out: &mut Vec<u8>,
) {
    let start = out.len();
    out.extend_from_slice(plain);
    let tag = cipher
        .encrypt_inout_detached(
            &chunk_nonce(counter, last),
            b"",
            InOutBuf::from(&mut out[start..]),
        )
        .expect("a chunk is far below the cipher's limit");
    out.extend_from_slice(&tag);
}

/// A chunk's nonce: 11 bytes of big-endian counter, then 1 if final.
fn chunk_nonce(counter: u64, last: bool) -> Nonce {
    let mut n = [0u8; 12];
    n[3..11].copy_from_slice(&counter.to_be_bytes());
    n[11] = u8::from(last);
    Nonce::from(n)
}

/// Costs `1024 · 2^log_n` bytes of memory and one scrypt run: the wrap
/// key, `N = 2^log_n`, `r = 8`, `p = 1`, 32 bytes, over
/// the label and the salt. `Err` where `N` does not fit.
fn wrap_key(
    c: &Compute,
    passphrase: &[u8],
    salt: &[u8; 16],
    log_n: u32,
) -> Result<Zeroizing<[u8; 32]>, ()> {
    let mut s = [0u8; SCRYPT_LABEL.len() + 16];
    s[..SCRYPT_LABEL.len()].copy_from_slice(SCRYPT_LABEL);
    s[SCRYPT_LABEL.len()..].copy_from_slice(salt);
    let n = 1u32.checked_shl(log_n).ok_or(())?;
    let mut key = Zeroizing::new([0u8; 32]);
    scrypt(c, passphrase, &s, n, 8, 1, &mut *key).map_err(|_| ())?;
    Ok(key)
}

fn wrap_cipher(key: &[u8; 32]) -> ChaCha20Poly1305 {
    ChaCha20Poly1305::new_from_slice(key).expect("32-byte key")
}

/// HKDF-SHA-256 with `salt`, 32 bytes of output.
fn hkdf(salt: &[u8], ikm: &[u8], info: &[u8]) -> Zeroizing<[u8; 32]> {
    let prk = Zeroizing::new(hmac_sha256(salt, ikm));
    let mut out = Zeroizing::new([0u8; 32]);
    hkdf_sha256_expand(&*prk, info, &mut *out).expect("32 bytes");
    out
}

fn payload_cipher(file_key: &[u8; 16], nonce: &[u8; 16]) -> ChaCha20Poly1305 {
    wrap_cipher(&hkdf(nonce, file_key, b"payload"))
}

/// HMAC-SHA-256 of the header up to and including `---`.
fn header_mac(file_key: &[u8; 16], header: &[u8]) -> [u8; 32] {
    let key = hkdf(&[], file_key, b"header");
    hmac_sha256(&*key, header)
}

/// Whether two MACs are equal, in time that does not depend on where
/// they differ.
fn equal(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// --- The header ---------------------------------------------------------

/// What a header parse found, borrowing the file.
struct Header<'a> {
    /// The stanzas in all.
    stanzas: usize,
    /// The last stanza whose first argument is `scrypt`.
    scrypt: Option<Stanza<'a>>,
    /// Where the MAC's input ends: after `---`.
    mac_input_end: usize,
    mac: [u8; 32],
    /// Where the payload begins.
    payload_at: usize,
}

/// A stanza's arguments and the shape of its body.
struct Stanza<'a> {
    args: [&'a [u8]; 3],
    arg_count: usize,
    /// The first body line, and how many lines the body has.
    body_line: &'a [u8],
    body_lines: usize,
}

/// A scrypt stanza that follows the specification.
struct ScryptParams {
    salt: [u8; 16],
    /// Saturated at 1000 for a longer number.
    log_n: u64,
    body: [u8; 32],
}

impl Stanza<'_> {
    fn params(&self) -> Result<ScryptParams, Error> {
        if self.arg_count != 3 {
            return Err(Error::ScryptStanza);
        }
        let salt = b64_decode::<16, B64_16>(self.args[1]).ok_or(Error::ScryptStanza)?;
        let log_n = decimal(self.args[2]).ok_or(Error::ScryptStanza)?;
        // One line of 43 characters is the only way to write 32 bytes.
        if self.body_lines != 1 {
            return Err(Error::ScryptStanza);
        }
        let body = b64_decode::<32, B64_32>(self.body_line).ok_or(Error::ScryptStanza)?;
        Ok(ScryptParams { salt, log_n, body })
    }
}

/// `%x31-39 *DIGIT`, saturating at 1000.
fn decimal(text: &[u8]) -> Option<u64> {
    let (first, _) = text.split_first()?;
    if !(b'1'..=b'9').contains(first) || !text.iter().all(u8::is_ascii_digit) {
        return None;
    }
    Some(
        text.iter()
            .fold(0u64, |v, d| (v * 10 + u64::from(d - b'0')).min(1000)),
    )
}

fn is_armored(file: &[u8]) -> bool {
    let start = file
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(file.len());
    file[start..].starts_with(ARMOR_BEGIN)
}

/// The next line of `file` from `at`, without its line feed.
fn line(file: &[u8], at: usize) -> Result<&[u8], Error> {
    let rest = file.get(at..).ok_or(Error::Header)?;
    let end = rest.iter().position(|&b| b == b'\n').ok_or(Error::Header)?;
    Ok(&rest[..end])
}

fn is_b64_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'+' || b == b'/'
}

/// Reads the header's grammar: the version line, one or more stanzas, the
/// end line. No stanza body is decoded and nothing is allocated.
fn parse_header(file: &[u8]) -> Result<Header<'_>, Error> {
    if !file.starts_with(MAGIC) {
        return Err(Error::NotAge);
    }
    let mut at = MAGIC.len();
    let mut stanzas = 0;
    let mut scrypt = None;
    loop {
        let l = line(file, at)?;
        if let Some(args) = l.strip_prefix(b"-> ") {
            at += l.len() + 1;
            let mut parts = [&b""[..]; 3];
            let mut arg_count = 0;
            for arg in args.split(|&b| b == b' ') {
                if arg.is_empty() || !arg.iter().all(|b| (0x21..=0x7e).contains(b)) {
                    return Err(Error::Header);
                }
                if let Some(slot) = parts.get_mut(arg_count) {
                    *slot = arg;
                }
                arg_count += 1;
            }
            let (body_line, body_lines) = body(file, &mut at)?;
            stanzas += 1;
            if parts[0] == b"scrypt" {
                scrypt = Some(Stanza {
                    args: parts,
                    arg_count,
                    body_line,
                    body_lines,
                });
            }
        } else if let Some(mac) = l.strip_prefix(b"--- ") {
            if stanzas == 0 {
                return Err(Error::Header);
            }
            let mac = b64_decode::<32, B64_32>(mac).ok_or(Error::Header)?;
            return Ok(Header {
                stanzas,
                scrypt,
                mac_input_end: at + 3,
                mac,
                payload_at: at + l.len() + 1,
            });
        } else {
            return Err(Error::Header);
        }
    }
}

/// A stanza's body from `at`: full lines of 64 characters and one shorter
/// line, which may be empty. Gives the first line and the count of lines.
fn body<'a>(file: &'a [u8], at: &mut usize) -> Result<(&'a [u8], usize), Error> {
    let mut first = None;
    let mut lines = 0;
    loop {
        let l = line(file, *at)?;
        if l.len() > LINE || !l.iter().all(|&b| is_b64_char(b)) {
            return Err(Error::Header);
        }
        *at += l.len() + 1;
        lines += 1;
        first.get_or_insert(l);
        if l.len() < LINE {
            return Ok((first.unwrap_or(l), lines));
        }
    }
}

// --- Base64 -------------------------------------------------------------

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Appends `bytes` as unpadded base64.
fn b64_push(out: &mut Vec<u8>, bytes: &[u8]) {
    for group in bytes.chunks(3) {
        let mut v = 0u32;
        for (i, b) in group.iter().enumerate() {
            v |= u32::from(*b) << (16 - 8 * i);
        }
        for i in 0..=group.len() {
            out.push(ALPHABET[((v >> (18 - 6 * i)) & 63) as usize]);
        }
    }
}

/// Canonical unpadded base64 of exactly `N` bytes, which take `CHARS`
/// characters: `None` for any other length or character, `=`, or nonzero
/// bits after the last byte.
fn b64_decode<const N: usize, const CHARS: usize>(text: &[u8]) -> Option<[u8; N]> {
    if text.len() != CHARS {
        return None;
    }
    let mut out = [0u8; N];
    let mut acc = 0u32;
    let mut bits = 0;
    let mut at = 0;
    for &c in text {
        let v = ALPHABET.iter().position(|&a| a == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out[at] = (acc >> bits) as u8;
            at += 1;
            acc &= (1 << bits) - 1;
        }
    }
    (at == N && acc == 0).then_some(out)
}
