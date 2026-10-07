//! BIP 93: codex32, a checksummed base-32 encoding of a BIP-32 master
//! seed and of Shamir shares of one.
//!
//! A codex32 string is `ms1`, a threshold digit, a four-character
//! identifier, a share index, a payload and a checksum, all in the
//! bech32 alphabet and all one case. The share index `s` marks the
//! secret itself; any other index marks a share of a `k`-of-`n` split.
//! Any `k` shares of a set rebuild the secret, and any `k` strings of a
//! set (the secret counts as one) derive a further share, both by
//! Lagrange interpolation over GF(32) of each character position at
//! once — the checksum interpolates with the rest, so a derived string
//! is checksummed without recomputing it.
//!
//! ```
//! use osk_bip::codex32::Codex32;
//!
//! let a = Codex32::parse("ms13casha320zyxwvutsrqpnmlkjhgfedca2a8d0zehn8a0t").unwrap();
//! let c = Codex32::parse("ms13cashcacdefghjklmnpqrstuvwxyz023949xq35my48dr").unwrap();
//! let d = Codex32::parse("ms13cashd0wsedstcdcts64cd7wvy4m90lm28w4ffupqs7rm").unwrap();
//! let secret = Codex32::recover(&[a, c, d]).unwrap();
//! assert_eq!(
//!     secret.encode().as_str(),
//!     "ms13cashsllhdmn9m42vcsamx24zrxgs3qqjzqud4m0d6nln"
//! );
//! assert_eq!(secret.to_seed().unwrap().expose().bytes()[0], 0xff);
//! ```
//!
//! Conventions follow [`crate::bip39`]: a character is a `u8` symbol in
//! `0..32`, never text; the string, its payload and the seed live in
//! fixed-size arrays that are wiped on drop; text appears only at the
//! edges, as `&str` in and [`Codex32Ascii`] out.
//!
//! The checksum is a BCH code that detects any error touching at most 8
//! characters and could correct up to 4 substitutions. This module
//! detects; it does not correct. A string whose checksum fails is
//! refused with [`Error::BadChecksum`] and nothing is offered in its
//! place.

use alloc::vec::Vec;
use core::fmt;

use osk_crypto::{Secret, Zeroize, ZeroizeOnDrop};

/// The bech32 alphabet, symbol value by position (BIP-0173).
const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
/// What `bech32_hrp_expand("ms")` contributes to the codeword length.
const HRP_EXPANDED_LEN: usize = 5;
/// Characters of regular checksum.
const CHECKSUM_LEN: usize = 13;
/// Characters of long checksum.
const LONG_CHECKSUM_LEN: usize = 15;
/// Threshold, identifier and share index, before the payload.
const HEADER_LEN: usize = 6;
/// The symbol value of the share index `s`, the secret's own index.
const SECRET_INDEX: u8 = 16;

/// The only string lengths a master seed encodes to, shortest first.
const VALID_LENGTHS: [usize; 6] = [48, 54, 61, 67, 74, 127];
/// The master seed sizes BIP 93 allows, in bytes, in the same order.
pub const SEED_LENGTHS: [usize; 6] = [16, 20, 24, 28, 32, 64];

/// Longest codex32 string this module reads or writes, in characters.
pub const MAX_STRING_LEN: usize = 127;
/// Longest data part: the string without `ms1`.
const MAX_DATA_LEN: usize = MAX_STRING_LEN - 3;
/// Longest master seed, in bytes.
pub const MAX_SEED_BYTES: usize = 64;
/// Largest threshold BIP 93 allows.
pub const MAX_THRESHOLD: u8 = 9;
/// Most shares one set can hold: every bech32 character but `s`.
pub const MAX_SHARES: u8 = 31;

/// Share indices in the order BIP 93 hands them out: the letters of the
/// alphabet that bech32 has, `s` excluded, then the digits.
const INDEX_ORDER: &[u8; MAX_SHARES as usize] = b"acdefghjklmnpqrtuvwxyz023456789";

/// Why a codex32 string, or a set of them, was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A character outside printable ASCII.
    NotPrintable,
    /// Upper and lower case in one string.
    MixedCase,
    /// The string does not start with `ms1`.
    BadPrefix,
    /// The string's length is not one of a master seed's.
    BadLength,
    /// The character at `position` (0-based, in the whole string) is not
    /// in the bech32 alphabet.
    BadCharacter {
        /// 0-based position of the offending character.
        position: u8,
    },
    /// The threshold is not a digit.
    BadThreshold,
    /// The threshold is `0`, which only the secret's own index may carry.
    SecretIndexRequired,
    /// The checksum does not match the rest of the string.
    BadChecksum,
    /// The string is a share where a secret was wanted, or a secret where
    /// a share was wanted.
    WrongKind,
    /// Two strings of the set carry the same share index.
    DuplicateIndex,
    /// The strings do not all carry the same identifier.
    IdentifierMismatch,
    /// The strings do not all carry the same threshold.
    ThresholdMismatch,
    /// The strings are not all the same length.
    LengthMismatch,
    /// The number of strings is not the threshold.
    WrongShareCount,
    /// The share index is not a bech32 character, or is already in the set.
    BadShareIndex,
    /// The identifier is not four bech32 characters.
    BadIdentifier,
    /// The seed is not 16, 20, 24, 28, 32 or 64 bytes.
    BadSeedLength,
    /// The threshold is not `0` or 2 to 9, or the share count is not
    /// between the threshold and [`MAX_SHARES`].
    BadSplit,
    /// The caller supplied too few random bytes for the shares asked for.
    NotEnoughRandomness,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotPrintable => f.write_str("string holds a character outside printable ASCII"),
            Error::MixedCase => f.write_str("string mixes upper and lower case"),
            Error::BadPrefix => f.write_str("string does not start with ms1"),
            Error::BadLength => f.write_str("string length does not encode a master seed"),
            Error::BadCharacter { position } => {
                write!(f, "character {} is not a bech32 character", position + 1)
            }
            Error::BadThreshold => f.write_str("threshold is not a digit"),
            Error::SecretIndexRequired => f.write_str("threshold 0 needs the share index s"),
            Error::BadChecksum => f.write_str("checksum does not match"),
            Error::WrongKind => f.write_str("string is the wrong kind for this operation"),
            Error::DuplicateIndex => f.write_str("two strings carry the same share index"),
            Error::IdentifierMismatch => f.write_str("identifiers differ across the set"),
            Error::ThresholdMismatch => f.write_str("thresholds differ across the set"),
            Error::LengthMismatch => f.write_str("lengths differ across the set"),
            Error::WrongShareCount => f.write_str("the number of shares is not the threshold"),
            Error::BadShareIndex => f.write_str("share index is not free and in the alphabet"),
            Error::BadIdentifier => f.write_str("identifier is not four bech32 characters"),
            Error::BadSeedLength => {
                f.write_str("master seed must be 16, 20, 24, 28, 32 or 64 bytes")
            }
            Error::BadSplit => f.write_str("threshold must be 2 to 9 and at most the share count"),
            Error::NotEnoughRandomness => f.write_str("too few random bytes for the shares asked"),
        }
    }
}

impl core::error::Error for Error {}

// ---------------------------------------------------------------------
// The bech32 alphabet
// ---------------------------------------------------------------------

/// Symbol value of each ASCII byte, or 32 where the byte is not a bech32
/// character. Uppercase maps as its lowercase does.
const CHARVAL: [u8; 128] = {
    let mut table = [32u8; 128];
    let mut i = 0;
    while i < 32 {
        let c = CHARSET[i] as usize;
        table[c] = i as u8;
        table[c - 32] = i as u8; // the uppercase form
        i += 1;
    }
    table
};

fn charval(c: u8) -> Option<u8> {
    if c >= 128 {
        return None;
    }
    let v = CHARVAL[c as usize];
    if v == 32 { None } else { Some(v) }
}

// ---------------------------------------------------------------------
// Checksums
// ---------------------------------------------------------------------

const MS32_CONST: u128 = 0x1_0ce0_795c_2fd1_e62a;
const MS32_LONG_CONST: u128 = 0x433_81e5_70bf_4798_ab26;

fn polymod(values: &[u8], zeros: usize) -> u128 {
    const GEN: [u128; 5] = [
        0x1_9dc5_00ce_73fd_e210,
        0x1_bfae_00de_f77f_e529,
        0x1_fbd9_20ff_fe7b_ee52,
        0x1_7396_40bd_eee3_fdad,
        0x0_7729_a039_cfc7_5f5a,
    ];
    let mut residue: u128 = 0x23181b3;
    for v in values
        .iter()
        .copied()
        .chain(core::iter::repeat_n(0u8, zeros))
    {
        let b = residue >> 60;
        residue = ((residue & 0x0fff_ffff_ffff_ffff) << 5) ^ u128::from(v);
        for (i, g) in GEN.iter().enumerate() {
            if (b >> i) & 1 == 1 {
                residue ^= g;
            }
        }
    }
    residue
}

fn long_polymod(values: &[u8], zeros: usize) -> u128 {
    const GEN: [u128; 5] = [
        0x3d5_9d27_3535_ea62_d897,
        0x7a9_becb_6361_c6c5_1507,
        0x543_f9b7_e6c3_8d8a_2a0e,
        0x0c5_77ea_eccf_1990_d13c,
        0x188_7f74_f8dc_71b1_0651,
    ];
    let mut residue: u128 = 0x23181b3;
    for v in values
        .iter()
        .copied()
        .chain(core::iter::repeat_n(0u8, zeros))
    {
        let b = residue >> 70;
        residue = ((residue & 0x3f_ffff_ffff_ffff_ffff) << 5) ^ u128::from(v);
        for (i, g) in GEN.iter().enumerate() {
            if (b >> i) & 1 == 1 {
                residue ^= g;
            }
        }
    }
    residue
}

/// Whether the data part (checksum included) carries a valid checksum of
/// the variant its length calls for.
fn verify_checksum(data: &[u8]) -> bool {
    let expanded = HRP_EXPANDED_LEN + data.len();
    if expanded >= 96 {
        expanded <= 1023 && long_polymod(data, 0) == MS32_LONG_CONST
    } else {
        expanded <= 93 && polymod(data, 0) == MS32_CONST
    }
}

/// Appends the checksum the data part's length calls for, and returns the
/// new length.
fn append_checksum(data: &mut [u8], len: usize) -> usize {
    if HRP_EXPANDED_LEN + len + CHECKSUM_LEN > 93 {
        let residue = long_polymod(&data[..len], LONG_CHECKSUM_LEN) ^ MS32_LONG_CONST;
        for (i, slot) in data[len..len + LONG_CHECKSUM_LEN].iter_mut().enumerate() {
            *slot = ((residue >> (5 * (LONG_CHECKSUM_LEN - 1 - i))) & 31) as u8;
        }
        len + LONG_CHECKSUM_LEN
    } else {
        let residue = polymod(&data[..len], CHECKSUM_LEN) ^ MS32_CONST;
        for (i, slot) in data[len..len + CHECKSUM_LEN].iter_mut().enumerate() {
            *slot = ((residue >> (5 * (CHECKSUM_LEN - 1 - i))) & 31) as u8;
        }
        len + CHECKSUM_LEN
    }
}

// ---------------------------------------------------------------------
// GF(32)
// ---------------------------------------------------------------------

/// Multiplication in GF(32) as BIP 93 defines it over the bech32 alphabet.
fn gf_mul(a: u8, b: u8) -> u8 {
    let mut a = a;
    let mut res = 0u8;
    for i in 0..5 {
        if (b >> i) & 1 == 1 {
            res ^= a;
        }
        a <<= 1;
        if a >= 32 {
            a ^= 41;
        }
    }
    res
}

/// Multiplicative inverse in GF(32); `INV[0]` is 0, which never divides.
const INV: [u8; 32] = [
    0, 1, 20, 24, 10, 8, 12, 29, 5, 11, 4, 9, 6, 28, 26, 31, 22, 18, 17, 23, 2, 25, 16, 19, 3, 21,
    14, 30, 13, 7, 27, 15,
];

/// The Lagrange weights of the points `xs` evaluated at `x`.
fn lagrange(xs: &[u8], x: u8, out: &mut [u8]) {
    let mut n = 1u8;
    for &i in xs {
        n = gf_mul(n, i ^ x);
    }
    for (slot, &i) in out.iter_mut().zip(xs) {
        let mut m = 1u8;
        for &j in xs {
            m = gf_mul(m, if i == j { x } else { i } ^ j);
        }
        *slot = gf_mul(n, INV[m as usize]);
    }
}

// ---------------------------------------------------------------------
// A master seed
// ---------------------------------------------------------------------

/// A BIP-32 master seed of 16, 20, 24, 28, 32 or 64 bytes, wiped on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterSeed {
    bytes: [u8; MAX_SEED_BYTES],
    len: u8,
}

impl MasterSeed {
    /// The seed itself.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

// ---------------------------------------------------------------------
// A codex32 string
// ---------------------------------------------------------------------

/// A valid codex32 string: a secret or one share of a set, held as the
/// data part's symbols with its checksum, and wiped on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Codex32 {
    data: [u8; MAX_DATA_LEN],
    len: u8,
}

/// A codex32 string as lowercase ASCII, wiped on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Codex32Ascii {
    buf: [u8; MAX_STRING_LEN],
    len: u8,
}

impl Codex32Ascii {
    /// The string.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..usize::from(self.len)]).unwrap_or("")
    }
}

impl Default for Codex32 {
    /// The empty slot a fixed-size set starts from, as
    /// [`crate::slip39::Share`] has one: no string, length zero, and
    /// never handed to any of the operations below.
    fn default() -> Self {
        Codex32 {
            data: [0u8; MAX_DATA_LEN],
            len: 0,
        }
    }
}

impl Codex32 {
    /// Reads a codex32 string, checking every rule BIP 93 states.
    pub fn parse(s: &str) -> Result<Self, Error> {
        let b = s.as_bytes();
        if b.iter().any(|&c| !(33..=126).contains(&c)) {
            return Err(Error::NotPrintable);
        }
        if b.iter().any(u8::is_ascii_lowercase) && b.iter().any(u8::is_ascii_uppercase) {
            return Err(Error::MixedCase);
        }
        if b.len() < 3
            || !b[0].eq_ignore_ascii_case(&b'm')
            || !b[1].eq_ignore_ascii_case(&b's')
            || b[2] != b'1'
        {
            return Err(Error::BadPrefix);
        }
        if !VALID_LENGTHS.contains(&b.len()) {
            return Err(Error::BadLength);
        }

        let mut data = [0u8; MAX_DATA_LEN];
        for (i, &c) in b[3..].iter().enumerate() {
            data[i] = charval(c).ok_or(Error::BadCharacter {
                position: (i + 3) as u8,
            })?;
        }
        let len = b.len() - 3;

        if !b[3].is_ascii_digit() {
            return Err(Error::BadThreshold);
        }
        if b[3] == b'0' && data[5] != SECRET_INDEX {
            return Err(Error::SecretIndexRequired);
        }
        if !verify_checksum(&data[..len]) {
            return Err(Error::BadChecksum);
        }
        Ok(Codex32 {
            data,
            len: len as u8,
        })
    }

    /// The threshold: 0 for a secret that was never split, else 2 to 9.
    pub fn threshold(&self) -> u8 {
        CHARSET[self.data[0] as usize] - b'0'
    }

    /// The four-character identifier, lowercase.
    pub fn identifier(&self) -> [u8; 4] {
        let mut out = [0u8; 4];
        for (slot, &v) in out.iter_mut().zip(&self.data[1..5]) {
            *slot = CHARSET[v as usize];
        }
        out
    }

    /// The share index, lowercase; `s` for the secret.
    pub fn share_index(&self) -> u8 {
        CHARSET[self.data[5] as usize]
    }

    /// Whether this string is the secret rather than a share of it.
    pub fn is_secret(&self) -> bool {
        self.data[5] == SECRET_INDEX
    }

    fn checksum_len(&self) -> usize {
        if HRP_EXPANDED_LEN + usize::from(self.len) >= 96 {
            LONG_CHECKSUM_LEN
        } else {
            CHECKSUM_LEN
        }
    }

    /// The payload as 5-bit symbols, without the header or the checksum.
    pub fn payload(&self) -> &[u8] {
        &self.data[HEADER_LEN..usize::from(self.len) - self.checksum_len()]
    }

    /// The master seed this string carries, which only the secret does.
    pub fn to_seed(&self) -> Result<Secret<MasterSeed>, Error> {
        if !self.is_secret() {
            return Err(Error::WrongKind);
        }
        let payload = self.payload();
        let len = payload.len() * 5 / 8;
        if !SEED_LENGTHS.contains(&len) {
            return Err(Error::BadSeedLength);
        }
        let mut seed = MasterSeed {
            bytes: [0u8; MAX_SEED_BYTES],
            len: len as u8,
        };
        let mut acc: u16 = 0;
        let mut bits = 0u32;
        let mut out = 0usize;
        for &v in payload {
            acc = (acc << 5) | u16::from(v);
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                seed.bytes[out] = (acc >> bits) as u8;
                out += 1;
            }
        }
        Ok(Secret::new(seed))
    }

    /// Encodes a master seed as the codex32 secret of a set with this
    /// threshold and identifier, padding the last group with zero bits.
    ///
    /// The threshold is 0 for a seed that will not be split, else 2 to 9;
    /// the identifier is four bech32 characters in either case.
    pub fn from_seed(threshold: u8, identifier: [u8; 4], seed: &[u8]) -> Result<Self, Error> {
        if threshold == 1 || threshold > MAX_THRESHOLD {
            return Err(Error::BadSplit);
        }
        if !SEED_LENGTHS.contains(&seed.len()) {
            return Err(Error::BadSeedLength);
        }
        let mut data = [0u8; MAX_DATA_LEN];
        data[0] = charval(b'0' + threshold).ok_or(Error::BadThreshold)?;
        for (slot, &c) in data[1..5].iter_mut().zip(&identifier) {
            *slot = charval(c).ok_or(Error::BadIdentifier)?;
        }
        data[5] = SECRET_INDEX;

        let len = HEADER_LEN + seed_symbols(seed, &mut data[HEADER_LEN..]);
        let len = append_checksum(&mut data, len);
        Ok(Codex32 {
            data,
            len: len as u8,
        })
    }

    /// Writes the string in lowercase.
    pub fn encode(&self) -> Codex32Ascii {
        let mut out = Codex32Ascii {
            buf: [0u8; MAX_STRING_LEN],
            len: usize::from(self.len) as u8 + 3,
        };
        out.buf[..3].copy_from_slice(b"ms1");
        for (slot, &v) in out.buf[3..]
            .iter_mut()
            .zip(&self.data[..usize::from(self.len)])
        {
            *slot = CHARSET[v as usize];
        }
        out
    }

    /// Recovers the secret from exactly `k` shares of one `k`-of-`n` set.
    pub fn recover(shares: &[Self]) -> Result<Self, Error> {
        check_set(shares, false)?;
        if shares.len() != usize::from(shares[0].threshold()) {
            return Err(Error::WrongShareCount);
        }
        Ok(interpolate(shares, SECRET_INDEX))
    }

    /// Derives a further share of a set from exactly `k` of its strings,
    /// the secret counting as one, at a share index not already in use.
    pub fn derive_share(strings: &[Self], index: u8) -> Result<Self, Error> {
        check_set(strings, true)?;
        if strings.len() != usize::from(strings[0].threshold()) {
            return Err(Error::WrongShareCount);
        }
        let x = charval(index).ok_or(Error::BadShareIndex)?;
        if x == SECRET_INDEX || strings.iter().any(|s| s.data[5] == x) {
            return Err(Error::BadShareIndex);
        }
        Ok(interpolate(strings, x))
    }

    /// Splits this secret into `shares` shares of which `threshold` are
    /// needed, using `fresh` as the randomness of the first
    /// `threshold - 1` of them.
    ///
    /// `fresh` needs `(threshold - 1) * payload length` bytes, of which
    /// the low five bits of each are used; the rest are ignored. The
    /// secret keeps its own identifier and takes the new threshold; its
    /// padding bits are rewritten as zeroes, so the string the shares
    /// recover carries the same master seed but need not be this one
    /// character for character.
    pub fn split(&self, threshold: u8, shares: u8, fresh: &[u8]) -> Result<Vec<Self>, Error> {
        if !self.is_secret() {
            return Err(Error::WrongKind);
        }
        if !(2..=MAX_THRESHOLD).contains(&threshold) || shares < threshold || shares > MAX_SHARES {
            return Err(Error::BadSplit);
        }
        let payload_len = self.payload().len();
        if fresh.len() < usize::from(threshold - 1) * payload_len {
            return Err(Error::NotEnoughRandomness);
        }

        let secret = {
            let seed = self.to_seed()?;
            Self::from_seed(threshold, self.identifier(), seed.expose().bytes())?
        };

        let mut set = Vec::with_capacity(usize::from(threshold));
        set.push(secret);
        let mut random = fresh;
        for &index in &INDEX_ORDER[..usize::from(threshold) - 1] {
            let mut data = [0u8; MAX_DATA_LEN];
            data[..HEADER_LEN].copy_from_slice(&set[0].data[..HEADER_LEN]);
            data[5] = charval(index).ok_or(Error::BadShareIndex)?;
            for (slot, &byte) in data[HEADER_LEN..HEADER_LEN + payload_len]
                .iter_mut()
                .zip(random)
            {
                *slot = byte & 31;
            }
            random = &random[payload_len..];
            let len = append_checksum(&mut data, HEADER_LEN + payload_len);
            set.push(Codex32 {
                data,
                len: len as u8,
            });
        }

        let mut out = Vec::with_capacity(usize::from(shares));
        for &index in &INDEX_ORDER[..usize::from(shares)] {
            let x = charval(index).ok_or(Error::BadShareIndex)?;
            match set.iter().position(|s| s.data[5] == x) {
                Some(i) => out.push(Codex32 {
                    data: set[i].data,
                    len: set[i].len,
                }),
                None => out.push(interpolate(&set, x)),
            }
        }
        Ok(out)
    }
}

/// Characters a payload of `seed_len` bytes takes: BIP 93's
/// `ceil(bitlength / 5)`, the length the "Generating Shares" section
/// asks a random share to be filled to.
pub fn symbols_for(seed_len: usize) -> usize {
    (seed_len * 8).div_ceil(5)
}

/// Writes `seed` into `out` as a payload: 5-bit groups, most
/// significant bits first, the last group padded with zero bits.
/// Returns the characters written, which is [`symbols_for`] of the
/// seed's length.
///
/// This is the payload of the codex32 secret [`Codex32::from_seed`]
/// makes, and it is also how a random share of the same length is
/// written, so a set's strings are all filled by one rule.
pub fn seed_symbols(seed: &[u8], out: &mut [u8]) -> usize {
    let mut acc: u16 = 0;
    let mut bits = 0u32;
    let mut len = 0usize;
    for &byte in seed {
        acc = (acc << 8) | u16::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out[len] = ((acc >> bits) & 31) as u8;
            len += 1;
        }
    }
    if bits > 0 {
        out[len] = ((acc << (5 - bits)) & 31) as u8;
        len += 1;
    }
    len
}

/// The four-character identifier a set takes from the master
/// fingerprint of the seed it carries: the first 20 bits of the
/// fingerprint, as four 5-bit groups through the bech32 alphabet.
///
/// BIP 93 leaves the identifier undefined and asks only that it be
/// distinct for every secret a person may need to tell apart. Taking it
/// from the fingerprint makes it distinct per seed, and a computed
/// identifier is never mistyped.
pub fn identifier_for(fingerprint: [u8; 4]) -> [u8; 4] {
    let bits = u32::from_be_bytes(fingerprint);
    let mut out = [0u8; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = CHARSET[((bits >> (27 - 5 * i)) & 31) as usize];
    }
    out
}

/// The checks every set of codex32 strings must pass before any of them
/// is interpolated. `with_secret` says whether the secret may be among
/// them, as it may when deriving a share and may not when recovering.
fn check_set(set: &[Codex32], with_secret: bool) -> Result<(), Error> {
    let first = set.first().ok_or(Error::WrongShareCount)?;
    if first.threshold() == 0 {
        return Err(Error::ThresholdMismatch);
    }
    for (i, s) in set.iter().enumerate() {
        if s.data[0] != first.data[0] {
            return Err(Error::ThresholdMismatch);
        }
        if s.data[1..5] != first.data[1..5] {
            return Err(Error::IdentifierMismatch);
        }
        if s.len != first.len {
            return Err(Error::LengthMismatch);
        }
        if s.is_secret() && !with_secret {
            return Err(Error::WrongKind);
        }
        if set[..i].iter().any(|e| e.data[5] == s.data[5]) {
            return Err(Error::DuplicateIndex);
        }
    }
    Ok(())
}

/// The string of the set that would carry share index `x`, every
/// character position interpolated at once.
fn interpolate(set: &[Codex32], x: u8) -> Codex32 {
    let mut xs = [0u8; MAX_THRESHOLD as usize];
    for (slot, s) in xs.iter_mut().zip(set) {
        *slot = s.data[5];
    }
    let mut weights = [0u8; MAX_THRESHOLD as usize];
    lagrange(&xs[..set.len()], x, &mut weights[..set.len()]);

    let mut data = [0u8; MAX_DATA_LEN];
    let len = usize::from(set[0].len);
    for (i, slot) in data[..len].iter_mut().enumerate() {
        let mut v = 0u8;
        for (w, s) in weights.iter().zip(set) {
            v ^= gf_mul(*w, s.data[i]);
        }
        *slot = v;
    }
    Codex32 {
        data,
        len: len as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The identifier of the fingerprint `73c5da0a`, worked out by hand:
    /// its first twenty bits are 01110 01111 00010 11101, which are 14,
    /// 15, 2 and 29, and those places in the bech32 alphabet are `w`,
    /// `0`, `z` and `a`.
    #[test]
    fn an_identifier_is_the_first_twenty_bits_of_the_fingerprint() {
        assert_eq!(&identifier_for([0x73, 0xc5, 0xda, 0x0a]), b"w0za");
        // Every character is one the alphabet has, and the low twelve
        // bits of the fingerprint do not reach it.
        assert_eq!(
            identifier_for([0xff, 0xff, 0xf0, 0x00]),
            identifier_for([0xff, 0xff, 0xff, 0xff])
        );
    }
}
