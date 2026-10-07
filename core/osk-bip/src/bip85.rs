//! BIP-85: deterministic entropy from a BIP-32 master key.
//!
//! A master key, an application number and its parameters make a fully
//! hardened path `m/83696968'/{app}'/{params…}'/{index}'`; the derived
//! private key is run through `HMAC-SHA512("bip-entropy-from-k", k)`,
//! and the 64 bytes that come out are what the application reads.
//!
//! Six applications are here:
//!
//! | Application | Function | Path after the purpose |
//! |---|---|---|
//! | 2' HD-seed WIF | [`child_wif`] | `2'/{index}'` |
//! | 32' XPRV | [`child_xprv`] | `32'/{index}'` |
//! | 39' BIP-39 words | [`child_mnemonic`] | `39'/{language}'/{words}'/{index}'` |
//! | 128169' hex | [`child_hex`] | `128169'/{bytes}'/{index}'` |
//! | 707764' base64 password | [`child_password_base64`] | `707764'/{length}'/{index}'` |
//! | 707785' base85 password | [`child_password_base85`] | `707785'/{length}'/{index}'` |
//!
//! Every function zeroizes the derived private key and the 64 bytes
//! before it returns, and every value it returns is a fixed-size buffer
//! that zeroizes on drop. Nothing here allocates: a derived password or
//! WIF is as secret as the key it came from, and a heap string is not a
//! place a secret can be erased from.
//!
//! The vectors are in `tools/vectors/bip85/`, read by
//! `core/osk-bip/tests/bip85.rs`.

use osk_crypto::Zeroize;

use crate::bip39::{Language, Mnemonic};
use crate::keys::{ChildNumber, DerivationPath, MasterKey, SecretKey, WifAscii, XprivAscii};

/// BIP-85's own purpose, `83696968'`.
const PURPOSE: u32 = 83_696_968;
/// The HD-seed WIF application, `2'`.
const APP_WIF: u32 = 2;
/// The extended-private-key application, `32'`.
const APP_XPRV: u32 = 32;
/// The BIP-39 application, `39'`.
const APP_BIP39: u32 = 39;
/// The hex application, `128169'`.
const APP_HEX: u32 = 128_169;
/// The base64 password application, `707764'`.
const APP_BASE64: u32 = 707_764;
/// The base85 password application, `707785'`.
const APP_BASE85: u32 = 707_785;

/// The largest hardened index, `2^31 - 1`.
pub const MAX_INDEX: u32 = 0x7fff_ffff;
/// The word counts BIP-39, and so application 39', defines.
pub const WORD_COUNTS: [usize; 5] = [12, 15, 18, 21, 24];

/// The byte counts application 128169' takes, which is `16 <= n <= 64`.
pub const HEX_BYTES: (usize, usize) = (16, 64);
/// The password lengths application 707764' takes.
pub const BASE64_LENGTHS: (usize, usize) = (20, 86);
/// The password lengths application 707785' takes.
pub const BASE85_LENGTHS: (usize, usize) = (10, 80);

/// The 64 bytes one BIP-85 path yields, the longest any application
/// here reads, which is also the longest a password can be.
const HMAC_BYTES: usize = 64;

/// The longest password either application writes, which is base64's 86
/// characters.
const PASSWORD_MAX: usize = 86;

/// RFC 1924's base85 alphabet, which BIP-85's application 707785' does
/// not name and its reference implementation fixes: the BIP says only
/// "Base85 encode all 64 bytes of entropy", and its published password
/// `_s`{TW89)i4`` is in this alphabet and in no other — Ascii85's runs
/// from `!` to `u` and holds neither `{` nor `` ` ``.
const BASE85_ALPHABET: &[u8; 85] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";

/// Why a BIP-85 child could not be derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The word count is not one of [`WORD_COUNTS`].
    WordCount,
    /// The index is 2³¹ or larger, which no hardened path can hold.
    IndexOutOfRange,
    /// This build does not carry the wordlist the child would be in.
    Wordlist,
    /// The byte count is outside [`HEX_BYTES`].
    ByteCount,
    /// The password length is outside the application's range.
    PasswordLength,
    /// The 32 bytes the application would use as a private key are not
    /// one. BIP-32 puts this below 1 in 2¹²⁷ and BIP-85 says to fail on
    /// it rather than to fix it up, so the person derives the next
    /// index instead.
    InvalidKey,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::WordCount => f.write_str("word count must be 12, 15, 18, 21 or 24"),
            Error::IndexOutOfRange => f.write_str("index must be below 2^31"),
            Error::Wordlist => f.write_str("this build does not carry that wordlist"),
            Error::ByteCount => f.write_str("byte count must be 16 to 64"),
            Error::PasswordLength => f.write_str("password length is outside the range"),
            Error::InvalidKey => f.write_str("the derived key is not valid; use the next index"),
        }
    }
}

impl core::error::Error for Error {}

/// The code application 39' gives `language`, which is its position in
/// [`Language::ALL`]: English 0, Japanese 1, Korean 2, Spanish 3,
/// Chinese (Simplified) 4, Chinese (Traditional) 5, French 6, Italian 7,
/// Czech 8, Portuguese 9.
fn language_code(language: Language) -> u32 {
    Language::ALL
        .iter()
        .position(|l| *l == language)
        .unwrap_or(0) as u32
}

/// The 64 bytes BIP-85 derives at one path, zeroized on drop.
struct Bytes([u8; HMAC_BYTES]);

impl Drop for Bytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// `HMAC-SHA512("bip-entropy-from-k", k)` over the private key at
/// `m/83696968'` followed by `steps`, all hardened. The derived private
/// key is zeroized before this returns.
fn derive(master: &MasterKey, steps: &[u32]) -> Bytes {
    let hardened = |n: u32| ChildNumber::from_hardened_idx(n).expect("an index below 2^31");
    let mut children = alloc::vec::Vec::with_capacity(steps.len() + 1);
    children.push(hardened(PURPOSE));
    children.extend(steps.iter().copied().map(hardened));
    let path = DerivationPath::from(children);
    let derived = master.derive(&path);
    let mut k = derived.secret_key().secret_bytes();
    let mut hash = osk_crypto::hmac_sha512(b"bip-entropy-from-k", &k);
    k.zeroize();
    let mut out = Bytes([0; HMAC_BYTES]);
    out.0.copy_from_slice(&hash);
    hash.zeroize();
    out
}

/// Whether `index` is one a hardened path can hold.
fn check_index(index: u32) -> Result<(), Error> {
    if index > MAX_INDEX {
        return Err(Error::IndexOutOfRange);
    }
    Ok(())
}

/// The child mnemonic of `words` words at `index` in `language`
/// (application 39').
pub fn child_mnemonic(
    master: &MasterKey,
    language: Language,
    words: usize,
    index: u32,
) -> Result<Mnemonic, Error> {
    if !WORD_COUNTS.contains(&words) {
        return Err(Error::WordCount);
    }
    check_index(index)?;
    let hash = derive(
        master,
        &[APP_BIP39, language_code(language), words as u32, index],
    );
    let n = words * 4 / 3;
    // The entropy length is one BIP-39 accepts, so the only way this
    // fails is a wordlist this build does not carry.
    Mnemonic::from_entropy(language, &hash.0[..n]).map_err(|_| Error::Wordlist)
}

/// The child HD-seed WIF at `index` (application 2'), on the network the
/// master key is for.
///
/// BIP-85 takes the most significant 256 bits of the entropy as the
/// secret exponent and writes it as a compressed WIF, which is the form
/// Bitcoin Core's `sethdseed` takes.
pub fn child_wif(master: &MasterKey, index: u32) -> Result<WifAscii, Error> {
    check_index(index)?;
    let hash = derive(master, &[APP_WIF, index]);
    let mut secret = SecretKey::from_slice(&hash.0[..32]).map_err(|_| Error::InvalidKey)?;
    let wif = WifAscii::new(&secret, master.network());
    secret.non_secure_erase();
    Ok(wif)
}

/// The child extended private key at `index` (application 32'), at depth
/// 0 on the network the master key is for.
///
/// BIP-85 reverses BIP-32's order: the first 32 bytes of the entropy are
/// the chain code and the second 32 are the private key. The child
/// number, the depth and the parent fingerprint are zero.
pub fn child_xprv(master: &MasterKey, index: u32) -> Result<XprivAscii, Error> {
    check_index(index)?;
    let hash = derive(master, &[APP_XPRV, index]);
    let mut chain_code = [0u8; 32];
    let mut key = [0u8; 32];
    chain_code.copy_from_slice(&hash.0[..32]);
    key.copy_from_slice(&hash.0[32..]);
    let child = MasterKey::from_chain_code_and_key(&chain_code, &key, master.network());
    chain_code.zeroize();
    key.zeroize();
    Ok(child.ok_or(Error::InvalidKey)?.xpriv_ascii())
}

/// `bytes` bytes of entropy at `index` (application 128169'), which is
/// the derived entropy with its trailing bytes dropped.
pub fn child_hex(master: &MasterKey, bytes: usize, index: u32) -> Result<HexBytes, Error> {
    if bytes < HEX_BYTES.0 || bytes > HEX_BYTES.1 {
        return Err(Error::ByteCount);
    }
    check_index(index)?;
    let hash = derive(master, &[APP_HEX, bytes as u32, index]);
    let mut out = HexBytes {
        buf: [0; HMAC_BYTES],
        len: bytes as u8,
    };
    out.buf[..bytes].copy_from_slice(&hash.0[..bytes]);
    Ok(out)
}

/// The base64 password of `length` characters at `index` (application
/// 707764'): all 64 bytes base64-encoded, cut to `length`.
pub fn child_password_base64(
    master: &MasterKey,
    length: usize,
    index: u32,
) -> Result<Password, Error> {
    if length < BASE64_LENGTHS.0 || length > BASE64_LENGTHS.1 {
        return Err(Error::PasswordLength);
    }
    check_index(index)?;
    let hash = derive(master, &[APP_BASE64, length as u32, index]);
    let mut out = Password {
        buf: [0; PASSWORD_MAX],
        len: 0,
    };
    // 64 bytes encode to 86 characters and two pad characters, so the
    // whole buffer is filled and the cut is never past the end.
    let written = crate::base64::encode_into(&hash.0, &mut out.buf);
    debug_assert_eq!(written, PASSWORD_MAX);
    out.len = length as u8;
    Ok(out)
}

/// The base85 password of `length` characters at `index` (application
/// 707785'): all 64 bytes base85-encoded, cut to `length`.
pub fn child_password_base85(
    master: &MasterKey,
    length: usize,
    index: u32,
) -> Result<Password, Error> {
    if length < BASE85_LENGTHS.0 || length > BASE85_LENGTHS.1 {
        return Err(Error::PasswordLength);
    }
    check_index(index)?;
    let hash = derive(master, &[APP_BASE85, length as u32, index]);
    let mut out = Password {
        buf: [0; PASSWORD_MAX],
        len: 0,
    };
    // 64 bytes are sixteen four-byte groups, each five characters, so
    // there is no partial group and the result is exactly 80.
    for (group, chars) in hash.0.chunks(4).zip(out.buf.chunks_mut(5)) {
        let mut n = u32::from_be_bytes([group[0], group[1], group[2], group[3]]);
        for slot in chars.iter_mut().rev() {
            *slot = BASE85_ALPHABET[(n % 85) as usize];
            n /= 85;
        }
    }
    out.len = length as u8;
    Ok(out)
}

/// Raw entropy from application 128169', zeroized on drop.
pub struct HexBytes {
    buf: [u8; HMAC_BYTES],
    len: u8,
}

impl HexBytes {
    /// The bytes, `16` to `64` of them.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }
}

impl Drop for HexBytes {
    fn drop(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}

/// A password from application 707764' or 707785', zeroized on drop.
pub struct Password {
    buf: [u8; PASSWORD_MAX],
    len: u8,
}

impl Password {
    /// The password.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).expect("both alphabets are ascii")
    }
}

impl Drop for Password {
    fn drop(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}
