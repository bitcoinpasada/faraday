//! SLIP-39: Shamir's secret sharing over a 1024-word list.
//!
//! A SLIP-39 backup splits one master secret into groups of shares. Each
//! share is a mnemonic of 20 or 33 words carrying an identifier, the
//! group and member indices and thresholds, the share value and an
//! RS1024 checksum. Recovery takes a threshold of groups, and within each
//! group a threshold of members, and returns the master secret.
//!
//! **The master secret is the BIP-32 seed, not BIP-39 entropy.** A
//! recovered master secret is 16 or 32 bytes and goes straight into
//! `bitcoin::bip32::Xpriv::new_master`, as Trezor does it. It is not
//! entropy to be turned into BIP-39 words: the same bytes read as BIP-39
//! entropy give a different seed and therefore a different wallet. A
//! person who expects their SLIP-39 backup to open the wallet Trezor
//! shows them gets it through this path and through no other.
//!
//! The passphrase is not a check. Every passphrase decrypts to some
//! master secret, so a wrong one opens a different wallet rather than
//! failing. What the checksum and the digest share catch is a mistyped
//! or mismatched set of words.
//!
//! ```
//! use osk_bip::slip39::{Share, recover};
//!
//! let share = Share::parse(
//!     "duckling enlarge academic academic agency result length solution \
//!      fridge kidney coal piece deal husband erode duke ajar critical \
//!      decision keyboard",
//! )
//! .unwrap();
//! let secret = recover(&[share], b"TREZOR").unwrap();
//! assert_eq!(secret.expose().as_bytes().len(), 16);
//! ```

use core::fmt;

use osk_crypto::{Secret, Zeroize, ZeroizeOnDrop, hmac_sha256, pbkdf2_hmac_sha256};

use crate::wordlists::slip39 as wordlist;

/// Words in the shortest share (a 128-bit master secret).
pub const MIN_WORDS: usize = 20;
/// Words in the longest share (a 256-bit master secret).
pub const MAX_WORDS: usize = 33;
/// Words in the list a share is spelled from.
pub const WORDLIST_LEN: usize = 1024;
/// Bits each word carries.
pub const RADIX_BITS: usize = 10;

/// Characters in the longest word of the list, which is what a panel of
/// shares is sized for.
pub const MAX_DISPLAY_CHARS: usize = wordlist::MAX_DISPLAY_CHARS;
/// Shortest master secret, in bytes.
pub const MIN_STRENGTH_BYTES: usize = 16;
/// Longest master secret this implementation holds, in bytes.
pub const MAX_STRENGTH_BYTES: usize = 32;
/// Most groups a backup may have, and most members a group may have.
pub const MAX_COUNT: usize = 16;
/// Longest accepted passphrase, in bytes.
pub const MAX_PASSPHRASE_BYTES: usize = 256;
/// Largest iteration exponent the four-bit field holds.
pub const MAX_ITERATION_EXPONENT: u8 = 15;
/// PBKDF2 rounds at iteration exponent 0, across all four Feistel rounds.
pub const BASE_ITERATION_COUNT: u32 = 10_000;

/// Words of header before the share value: identifier, extendable flag,
/// iteration exponent, group index, group threshold, group count, member
/// index and member threshold, forty bits in all.
const HEADER_WORDS: usize = 4;
/// Words of RS1024 checksum after the share value.
const CHECKSUM_WORDS: usize = 3;
/// Feistel rounds in the master secret's encryption.
const ROUND_COUNT: u32 = 4;
/// `x` of the point that carries the digest of the shared secret.
const DIGEST_INDEX: u8 = 254;
/// `x` of the point that carries the shared secret itself.
const SECRET_INDEX: u8 = 255;
/// Bytes of the digest share that are the digest; the rest is random. A
/// threshold of `t` therefore draws `t - 2` whole random values and one
/// shorter than a value by this.
pub const DIGEST_LENGTH: usize = 4;

const CUSTOMIZATION: &[u8] = b"shamir";
const CUSTOMIZATION_EXTENDABLE: &[u8] = b"shamir_extendable";

/// Why a share or a set of shares was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The share is not 20 to 33 words, or its length leaves the share
    /// value short of a whole number of bytes.
    InvalidWordCount,
    /// The word at `position` (0-based) is not in the SLIP-39 wordlist.
    UnknownWord {
        /// 0-based position of the offending word.
        position: u8,
    },
    /// The RS1024 checksum does not match the rest of the words.
    BadChecksum,
    /// The bits padding the share value up to a whole number of words are
    /// not zero.
    NonZeroPadding,
    /// The share value is shorter than 16 bytes or is an odd length.
    InvalidSecretLength,
    /// No shares were given.
    NoShares,
    /// The shares do not all carry the same identifier.
    MismatchedIdentifiers,
    /// The shares do not all carry the same extendable flag.
    MismatchedExtendable,
    /// The shares do not all carry the same iteration exponent.
    MismatchedIterationExponents,
    /// The shares do not all carry the same group threshold.
    MismatchedGroupThresholds,
    /// The shares do not all carry the same group count.
    MismatchedGroupCounts,
    /// The shares do not all carry the same share value length.
    MismatchedValueLengths,
    /// Two shares of one group disagree about the member threshold.
    MismatchedMemberThresholds,
    /// The group threshold is greater than the group count.
    GroupThresholdExceedsCount,
    /// A group index is at or above the group count.
    GroupIndexOutOfRange,
    /// Two shares of one group carry the same member index.
    DuplicateMemberIndex,
    /// Groups were given for `given` of the `needed` the backup requires.
    WrongGroupCount {
        /// Groups the backup requires.
        needed: u8,
        /// Distinct groups among the shares given.
        given: u8,
    },
    /// A group has the wrong number of shares.
    WrongMemberCount {
        /// The group's index.
        group_index: u8,
        /// Shares the group requires.
        needed: u8,
        /// Shares of that group among those given.
        given: u8,
    },
    /// The shares interpolate, but the digest share does not match the
    /// secret they give: they are from different backups or one is
    /// mistyped in a way the checksum does not catch.
    BadDigest,
    /// The passphrase contains a byte outside printable ASCII
    /// (0x20..=0x7E).
    PassphraseNotAscii,
    /// The passphrase is longer than [`MAX_PASSPHRASE_BYTES`].
    PassphraseTooLong,
    /// A threshold, a count, an identifier or an iteration exponent is
    /// outside what the share format holds.
    InvalidParameters,
    /// The output slice is too short for the shares asked for.
    OutputTooSmall,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidWordCount => f.write_str("a share is 20 or 33 words"),
            Error::UnknownWord { position } => {
                write!(f, "word {} is not in the wordlist", position + 1)
            }
            Error::BadChecksum => f.write_str("checksum does not match"),
            Error::NonZeroPadding => f.write_str("padding bits are not zero"),
            Error::InvalidSecretLength => {
                f.write_str("the share value must be an even number of bytes, at least 16")
            }
            Error::NoShares => f.write_str("no shares given"),
            Error::MismatchedIdentifiers => f.write_str("the shares are from different backups"),
            Error::MismatchedExtendable => {
                f.write_str("the shares disagree about the extendable flag")
            }
            Error::MismatchedIterationExponents => {
                f.write_str("the shares disagree about the iteration exponent")
            }
            Error::MismatchedGroupThresholds => {
                f.write_str("the shares disagree about how many groups must be present")
            }
            Error::MismatchedGroupCounts => {
                f.write_str("the shares disagree about how many groups there are")
            }
            Error::MismatchedValueLengths => {
                f.write_str("the shares disagree about the secret's length")
            }
            Error::MismatchedMemberThresholds => {
                f.write_str("one group's shares disagree about how many of them must be present")
            }
            Error::GroupThresholdExceedsCount => {
                f.write_str("more groups must be present than the backup has")
            }
            Error::GroupIndexOutOfRange => f.write_str("a group index is outside the group count"),
            Error::DuplicateMemberIndex => {
                f.write_str("two shares of one group are the same share")
            }
            Error::WrongGroupCount { needed, given } => {
                write!(f, "{needed} groups must be present, {given} given")
            }
            Error::WrongMemberCount {
                group_index,
                needed,
                given,
            } => write!(
                f,
                "group {} needs {needed} shares, {given} given",
                group_index + 1
            ),
            Error::BadDigest => f.write_str("the shares do not belong together"),
            Error::PassphraseNotAscii => f.write_str("passphrase must be printable ASCII"),
            Error::PassphraseTooLong => {
                write!(f, "passphrase exceeds {MAX_PASSPHRASE_BYTES} bytes")
            }
            Error::InvalidParameters => f.write_str("thresholds and counts are 1 to 16"),
            Error::OutputTooSmall => f.write_str("not enough room for the shares"),
        }
    }
}

impl core::error::Error for Error {}

// --- the wordlist ----------------------------------------------------------

/// The word at `idx`, which must be below [`WORDLIST_LEN`].
pub fn word(idx: u16) -> &'static str {
    wordlist::WORDS[idx as usize]
}

/// The index of `w` in the wordlist, if it is there. A word is identified
/// by its first four letters, so a prefix of four or more letters finds
/// the word it begins.
pub fn index_of(w: &str) -> Option<u16> {
    if w.len() >= 4 {
        let key = &w.as_bytes()[..4];
        wordlist::WORDS
            .iter()
            .position(|&candidate| {
                candidate.as_bytes().starts_with(key) && candidate.starts_with(w)
            })
            .map(|i| i as u16)
    } else {
        wordlist::WORDS
            .iter()
            .position(|&c| c == w)
            .map(|i| i as u16)
    }
}

/// Every word that begins with `prefix`, in list order.
pub fn candidates<'a>(prefix: &'a str) -> impl Iterator<Item = u16> + 'a {
    wordlist::WORDS
        .iter()
        .enumerate()
        .filter(move |(_, w)| w.starts_with(prefix))
        .map(|(i, _)| i as u16)
}

/// SHA-256 (hex) of the published wordlist file.
pub fn wordlist_sha256() -> &'static str {
    wordlist::SHA256
}

// --- secret bytes ----------------------------------------------------------

/// Secret bytes of a length SLIP-39 works in: 16 to 32, always even. A
/// master secret, an encrypted master secret, a group secret and a share
/// value are all one of these. Erased on drop; it has no `Debug` and no
/// text form.
#[derive(Clone)]
pub struct SecretBytes {
    buf: [u8; MAX_STRENGTH_BYTES],
    len: u8,
}

impl SecretBytes {
    /// Wraps `bytes`, which must be 16 to 32 bytes and an even length.
    pub fn new(bytes: &[u8]) -> Result<Self, Error> {
        if !(MIN_STRENGTH_BYTES..=MAX_STRENGTH_BYTES).contains(&bytes.len())
            || !bytes.len().is_multiple_of(2)
        {
            return Err(Error::InvalidSecretLength);
        }
        let mut buf = [0u8; MAX_STRENGTH_BYTES];
        buf[..bytes.len()].copy_from_slice(bytes);
        Ok(Self {
            buf,
            len: bytes.len() as u8,
        })
    }

    /// The bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
}

impl Default for SecretBytes {
    fn default() -> Self {
        Self {
            buf: [0; MAX_STRENGTH_BYTES],
            len: 0,
        }
    }
}

impl AsRef<[u8]> for SecretBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Zeroize for SecretBytes {
    fn zeroize(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SecretBytes {}

// --- GF(256) ---------------------------------------------------------------

const TABLES: ([u8; 255], [u8; 256]) = build_tables();
const EXP: [u8; 255] = TABLES.0;
const LOG: [u8; 256] = TABLES.1;

const fn build_tables() -> ([u8; 255], [u8; 256]) {
    let mut exp = [0u8; 255];
    let mut log = [0u8; 256];
    let mut poly: u16 = 1;
    let mut i = 0;
    while i < 255 {
        exp[i] = poly as u8;
        log[poly as usize] = i as u8;
        // Multiply by 3, the generator, modulo x^8 + x^4 + x^3 + x + 1.
        poly = (poly << 1) ^ poly;
        if poly & 0x100 != 0 {
            poly ^= 0x11B;
        }
        i += 1;
    }
    (exp, log)
}

/// Up to sixteen points of one polynomial over GF(256), one byte at a
/// time: `x` is the share index, `y` the share value.
struct Points {
    x: [u8; MAX_COUNT + 2],
    y: [[u8; MAX_STRENGTH_BYTES]; MAX_COUNT + 2],
    n: usize,
    len: usize,
}

impl Points {
    fn new(len: usize) -> Self {
        Self {
            x: [0; MAX_COUNT + 2],
            y: [[0; MAX_STRENGTH_BYTES]; MAX_COUNT + 2],
            n: 0,
            len,
        }
    }

    fn push(&mut self, x: u8, y: &[u8]) {
        self.x[self.n] = x;
        self.y[self.n][..y.len()].copy_from_slice(y);
        self.n += 1;
    }

    /// The polynomial's value at `x`, Lagrange interpolation over
    /// GF(256).
    fn at(&self, x: u8) -> SecretBytes {
        let mut out = [0u8; MAX_STRENGTH_BYTES];
        for i in 0..self.n {
            if self.x[i] == x {
                out.copy_from_slice(&self.y[i]);
                return SecretBytes {
                    buf: out,
                    len: self.len as u8,
                };
            }
        }
        let mut log_prod: i32 = 0;
        for i in 0..self.n {
            log_prod += i32::from(LOG[(self.x[i] ^ x) as usize]);
        }
        for i in 0..self.n {
            let mut others: i32 = 0;
            for j in 0..self.n {
                others += i32::from(LOG[(self.x[i] ^ self.x[j]) as usize]);
            }
            let basis =
                (log_prod - i32::from(LOG[(self.x[i] ^ x) as usize]) - others).rem_euclid(255);
            for (k, byte) in out.iter_mut().take(self.len).enumerate() {
                let v = self.y[i][k];
                if v != 0 {
                    *byte ^= EXP[((i32::from(LOG[v as usize]) + basis) % 255) as usize];
                }
            }
        }
        SecretBytes {
            buf: out,
            len: self.len as u8,
        }
    }
}

impl Zeroize for Points {
    fn zeroize(&mut self) {
        self.x.zeroize();
        for y in &mut self.y {
            y.zeroize();
        }
        self.n = 0;
    }
}

impl Drop for Points {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Points {}

fn digest_of(random_part: &[u8], shared_secret: &[u8]) -> [u8; DIGEST_LENGTH] {
    let full = hmac_sha256(random_part, shared_secret);
    let mut out = [0u8; DIGEST_LENGTH];
    out.copy_from_slice(&full[..DIGEST_LENGTH]);
    out
}

/// The shared secret behind `points`, checked against its digest share.
fn recover_secret(threshold: u8, points: &Points) -> Result<SecretBytes, Error> {
    if threshold == 1 {
        return Ok(SecretBytes {
            buf: points.y[0],
            len: points.len as u8,
        });
    }
    let shared = points.at(SECRET_INDEX);
    let digest_share = points.at(DIGEST_INDEX);
    let (digest, random_part) = digest_share.as_bytes().split_at(DIGEST_LENGTH);
    if digest_of(random_part, shared.as_bytes()) != digest {
        return Err(Error::BadDigest);
    }
    Ok(shared)
}

// --- RS1024 ----------------------------------------------------------------

const GEN: [u32; 10] = [
    0x00E0_E040,
    0x01C1_C080,
    0x0383_8100,
    0x0707_0200,
    0x0E0E_0009,
    0x1C0C_2412,
    0x3808_6C24,
    0x3090_FC48,
    0x21B1_F890,
    0x03F3_F120,
];

fn polymod(customization: &[u8], words: &[u16], trailing_zeros: usize) -> u32 {
    let mut chk: u32 = 1;
    let mut step = |v: u32| {
        let b = chk >> 20;
        chk = ((chk & 0x000F_FFFF) << 10) ^ v;
        for (i, g) in GEN.iter().enumerate() {
            if (b >> i) & 1 != 0 {
                chk ^= *g;
            }
        }
    };
    for &c in customization {
        step(u32::from(c));
    }
    for &w in words {
        step(u32::from(w));
    }
    for _ in 0..trailing_zeros {
        step(0);
    }
    chk
}

fn customization(extendable: bool) -> &'static [u8] {
    if extendable {
        CUSTOMIZATION_EXTENDABLE
    } else {
        CUSTOMIZATION
    }
}

/// The three checksum words for `data`.
fn checksum(extendable: bool, data: &[u16]) -> [u16; CHECKSUM_WORDS] {
    let chk = polymod(customization(extendable), data, CHECKSUM_WORDS) ^ 1;
    [
        ((chk >> 20) & 1023) as u16,
        ((chk >> 10) & 1023) as u16,
        (chk & 1023) as u16,
    ]
}

// --- the share -------------------------------------------------------------

/// One SLIP-39 share: its words and what they say.
///
/// The share value is a secret. `Share` has no `Debug` and no text form,
/// and its words and value are erased when it is dropped.
#[derive(Clone)]
pub struct Share {
    indices: [u16; MAX_WORDS],
    word_count: u8,
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
    group_index: u8,
    group_threshold: u8,
    group_count: u8,
    member_index: u8,
    member_threshold: u8,
    value: SecretBytes,
}

impl Default for Share {
    fn default() -> Self {
        Self {
            indices: [0; MAX_WORDS],
            word_count: 0,
            identifier: 0,
            extendable: false,
            iteration_exponent: 0,
            group_index: 0,
            group_threshold: 1,
            group_count: 1,
            member_index: 0,
            member_threshold: 1,
            value: SecretBytes::default(),
        }
    }
}

impl Zeroize for Share {
    fn zeroize(&mut self) {
        self.indices.zeroize();
        self.word_count = 0;
        self.value.zeroize();
    }
}

impl Drop for Share {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for Share {}

impl Share {
    /// Reads a share from its words, separated by any ASCII whitespace.
    pub fn parse(mnemonic: &str) -> Result<Self, Error> {
        let mut indices = [0u16; MAX_WORDS];
        let mut n = 0;
        for w in mnemonic.split_ascii_whitespace() {
            if n == MAX_WORDS {
                return Err(Error::InvalidWordCount);
            }
            indices[n] = index_of(w).ok_or(Error::UnknownWord { position: n as u8 })?;
            n += 1;
        }
        Share::from_indices(&indices[..n])
    }

    /// Reads a share from its word indices.
    pub fn from_indices(indices: &[u16]) -> Result<Self, Error> {
        if indices.len() < MIN_WORDS || indices.len() > MAX_WORDS {
            return Err(Error::InvalidWordCount);
        }
        for (i, &w) in indices.iter().enumerate() {
            if w as usize >= WORDLIST_LEN {
                return Err(Error::UnknownWord { position: i as u8 });
            }
        }
        let head = u64::from(indices[0]) << 30
            | u64::from(indices[1]) << 20
            | u64::from(indices[2]) << 10
            | u64::from(indices[3]);
        let extendable = (head >> 24) & 1 != 0;
        if polymod(customization(extendable), indices, 0) != 1 {
            return Err(Error::BadChecksum);
        }

        let value_words = &indices[HEADER_WORDS..indices.len() - CHECKSUM_WORDS];
        let padding = (RADIX_BITS * value_words.len()) % 16;
        if padding > 8 {
            return Err(Error::InvalidWordCount);
        }
        let value_len = (RADIX_BITS * value_words.len() - padding) / 8;
        let mut value = [0u8; MAX_STRENGTH_BYTES + 8];
        let mut acc: u64 = 0;
        let mut bits = 0usize;
        let mut out = 0usize;
        for (i, &w) in value_words.iter().enumerate() {
            acc = (acc << 10) | u64::from(w);
            bits += 10;
            if i == 0 {
                if acc >> (bits - padding) != 0 {
                    return Err(Error::NonZeroPadding);
                }
                bits -= padding;
                acc &= (1u64 << bits) - 1;
            }
            while bits >= 8 {
                bits -= 8;
                value[out] = (acc >> bits) as u8;
                out += 1;
            }
        }
        if !(MIN_STRENGTH_BYTES..=MAX_STRENGTH_BYTES).contains(&value_len)
            || !value_len.is_multiple_of(2)
        {
            return Err(Error::InvalidSecretLength);
        }

        let mut words = [0u16; MAX_WORDS];
        words[..indices.len()].copy_from_slice(indices);
        Ok(Self {
            indices: words,
            word_count: indices.len() as u8,
            identifier: (head >> 25) as u16,
            extendable,
            iteration_exponent: ((head >> 20) & 0xF) as u8,
            group_index: ((head >> 16) & 0xF) as u8,
            group_threshold: (((head >> 12) & 0xF) + 1) as u8,
            group_count: (((head >> 8) & 0xF) + 1) as u8,
            member_index: ((head >> 4) & 0xF) as u8,
            member_threshold: ((head & 0xF) + 1) as u8,
            value: SecretBytes::new(&value[..value_len])?,
        })
    }

    /// Builds a share from its fields and value.
    #[allow(clippy::too_many_arguments)]
    fn build(
        identifier: u16,
        extendable: bool,
        iteration_exponent: u8,
        group_index: u8,
        group_threshold: u8,
        group_count: u8,
        member_index: u8,
        member_threshold: u8,
        value: &SecretBytes,
    ) -> Self {
        let head = u64::from(identifier) << 25
            | u64::from(extendable) << 24
            | u64::from(iteration_exponent) << 20
            | u64::from(group_index) << 16
            | u64::from(group_threshold - 1) << 12
            | u64::from(group_count - 1) << 8
            | u64::from(member_index) << 4
            | u64::from(member_threshold - 1);
        let mut indices = [0u16; MAX_WORDS];
        indices[0] = ((head >> 30) & 1023) as u16;
        indices[1] = ((head >> 20) & 1023) as u16;
        indices[2] = ((head >> 10) & 1023) as u16;
        indices[3] = (head & 1023) as u16;

        let bytes = value.as_bytes();
        let value_words = (bytes.len() * 8).div_ceil(RADIX_BITS);
        let padding = value_words * RADIX_BITS - bytes.len() * 8;
        let mut acc: u64 = 0;
        let mut bits = padding;
        let mut n = HEADER_WORDS;
        for &b in bytes {
            acc = (acc << 8) | u64::from(b);
            bits += 8;
            while bits >= 10 {
                bits -= 10;
                indices[n] = ((acc >> bits) & 1023) as u16;
                n += 1;
            }
        }
        let word_count = n + CHECKSUM_WORDS;
        let sum = checksum(extendable, &indices[..n]);
        indices[n..word_count].copy_from_slice(&sum);
        Self {
            indices,
            word_count: word_count as u8,
            identifier,
            extendable,
            iteration_exponent,
            group_index,
            group_threshold,
            group_count,
            member_index,
            member_threshold,
            value: value.clone(),
        }
    }

    /// The share's words, as indices into the SLIP-39 wordlist.
    pub fn indices(&self) -> &[u16] {
        &self.indices[..self.word_count as usize]
    }

    /// How many words the share has.
    pub fn word_count(&self) -> usize {
        self.word_count as usize
    }

    /// The word at `position`.
    pub fn word(&self, position: usize) -> &'static str {
        word(self.indices[position])
    }

    /// The fifteen-bit identifier every share of one backup carries.
    pub fn identifier(&self) -> u16 {
        self.identifier
    }

    /// Whether the backup is extendable: its master secret's encryption
    /// does not depend on the identifier, so more groups can be added
    /// later.
    pub fn is_extendable(&self) -> bool {
        self.extendable
    }

    /// The iteration exponent: PBKDF2 does `10000 << e` rounds in all.
    pub fn iteration_exponent(&self) -> u8 {
        self.iteration_exponent
    }

    /// Which group, 0-based, the share belongs to.
    pub fn group_index(&self) -> u8 {
        self.group_index
    }

    /// How many groups must be present to recover the master secret.
    pub fn group_threshold(&self) -> u8 {
        self.group_threshold
    }

    /// How many groups the backup has.
    pub fn group_count(&self) -> u8 {
        self.group_count
    }

    /// Which member, 0-based, of its group the share is.
    pub fn member_index(&self) -> u8 {
        self.member_index
    }

    /// How many shares of this group must be present.
    pub fn member_threshold(&self) -> u8 {
        self.member_threshold
    }

    /// Length in bytes of the master secret behind this share.
    pub fn secret_len(&self) -> usize {
        self.value.as_bytes().len()
    }
}

// --- the master secret's encryption ----------------------------------------

fn check_passphrase(passphrase: &[u8]) -> Result<(), Error> {
    if passphrase.len() > MAX_PASSPHRASE_BYTES {
        return Err(Error::PassphraseTooLong);
    }
    if passphrase.iter().any(|&b| !(0x20..=0x7E).contains(&b)) {
        return Err(Error::PassphraseNotAscii);
    }
    Ok(())
}

/// The four-round Feistel network SLIP-39 encrypts and decrypts the
/// master secret with. `forward` runs the rounds 0, 1, 2, 3 (encryption)
/// and its inverse runs 3, 2, 1, 0.
fn feistel(
    value: &SecretBytes,
    passphrase: &[u8],
    iteration_exponent: u8,
    identifier: u16,
    extendable: bool,
    forward: bool,
) -> SecretBytes {
    let bytes = value.as_bytes();
    let half = bytes.len() / 2;
    let rounds = (BASE_ITERATION_COUNT << iteration_exponent) / ROUND_COUNT;

    let mut salt = [0u8; CUSTOMIZATION.len() + 2 + MAX_STRENGTH_BYTES / 2];
    let salt_prefix = if extendable {
        0
    } else {
        salt[..CUSTOMIZATION.len()].copy_from_slice(CUSTOMIZATION);
        salt[CUSTOMIZATION.len()..CUSTOMIZATION.len() + 2]
            .copy_from_slice(&identifier.to_be_bytes());
        CUSTOMIZATION.len() + 2
    };

    let mut password = [0u8; 1 + MAX_PASSPHRASE_BYTES];
    password[1..1 + passphrase.len()].copy_from_slice(passphrase);

    let mut left = [0u8; MAX_STRENGTH_BYTES / 2];
    let mut right = [0u8; MAX_STRENGTH_BYTES / 2];
    left[..half].copy_from_slice(&bytes[..half]);
    right[..half].copy_from_slice(&bytes[half..]);

    let mut f = [0u8; MAX_STRENGTH_BYTES / 2];
    for step in 0..ROUND_COUNT {
        let i = if forward {
            step as u8
        } else {
            (ROUND_COUNT - 1 - step) as u8
        };
        password[0] = i;
        salt[salt_prefix..salt_prefix + half].copy_from_slice(&right[..half]);
        pbkdf2_hmac_sha256(
            &password[..1 + passphrase.len()],
            &salt[..salt_prefix + half],
            rounds,
            &mut f[..half],
        );
        for k in 0..half {
            left[k] ^= f[k];
        }
        core::mem::swap(&mut left, &mut right);
    }

    let mut out = [0u8; MAX_STRENGTH_BYTES];
    out[..half].copy_from_slice(&right[..half]);
    out[half..bytes.len()].copy_from_slice(&left[..half]);

    left.zeroize();
    right.zeroize();
    f.zeroize();
    password.zeroize();
    salt.zeroize();

    SecretBytes {
        buf: out,
        len: bytes.len() as u8,
    }
}

/// The encrypted master secret a split carries, which is `secret` under
/// `passphrase`.
///
/// A device that writes shares needs this on its own, before any share
/// exists: the two fingerprints it shows for a passphrase are what
/// reading the shares back with and without it would give.
pub fn encrypt(
    secret: &[u8],
    passphrase: &[u8],
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
) -> Result<Secret<SecretBytes>, Error> {
    feistel_call(
        secret,
        passphrase,
        identifier,
        extendable,
        iteration_exponent,
        true,
    )
}

/// The master secret behind `encrypted` under `passphrase`: the inverse
/// of [`encrypt`].
pub fn decrypt(
    encrypted: &[u8],
    passphrase: &[u8],
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
) -> Result<Secret<SecretBytes>, Error> {
    feistel_call(
        encrypted,
        passphrase,
        identifier,
        extendable,
        iteration_exponent,
        false,
    )
}

fn feistel_call(
    value: &[u8],
    passphrase: &[u8],
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
    forward: bool,
) -> Result<Secret<SecretBytes>, Error> {
    check_passphrase(passphrase)?;
    if identifier >= 1 << 15 || iteration_exponent > MAX_ITERATION_EXPONENT {
        return Err(Error::InvalidParameters);
    }
    let value = SecretBytes::new(value)?;
    Ok(Secret::new(feistel(
        &value,
        passphrase,
        iteration_exponent,
        identifier,
        extendable,
        forward,
    )))
}

// --- recovery --------------------------------------------------------------

/// The master secret behind `shares`, under `passphrase`.
///
/// The result is the BIP-32 seed: hand it to
/// `bitcoin::bip32::Xpriv::new_master`. It is not BIP-39 entropy.
///
/// Every refusal names what is wrong with the set: a share from another
/// backup, a group short of shares, a digest that does not match. A wrong
/// passphrase is not one of them — it gives a different master secret,
/// and so a different wallet.
pub fn recover(shares: &[Share], passphrase: &[u8]) -> Result<Secret<SecretBytes>, Error> {
    check_passphrase(passphrase)?;
    let first = shares.first().ok_or(Error::NoShares)?;
    for s in shares {
        if s.identifier != first.identifier {
            return Err(Error::MismatchedIdentifiers);
        }
        if s.extendable != first.extendable {
            return Err(Error::MismatchedExtendable);
        }
        if s.iteration_exponent != first.iteration_exponent {
            return Err(Error::MismatchedIterationExponents);
        }
        if s.group_threshold != first.group_threshold {
            return Err(Error::MismatchedGroupThresholds);
        }
        if s.group_count != first.group_count {
            return Err(Error::MismatchedGroupCounts);
        }
        if s.secret_len() != first.secret_len() {
            return Err(Error::MismatchedValueLengths);
        }
        if s.group_index >= first.group_count {
            return Err(Error::GroupIndexOutOfRange);
        }
    }
    if first.group_threshold > first.group_count {
        return Err(Error::GroupThresholdExceedsCount);
    }

    // One entry per group index present, in the order first met.
    let mut order = [0u8; MAX_COUNT];
    let mut groups = 0usize;
    for s in shares {
        if !order[..groups].contains(&s.group_index) {
            order[groups] = s.group_index;
            groups += 1;
        }
    }
    if groups != first.group_threshold as usize {
        return Err(Error::WrongGroupCount {
            needed: first.group_threshold,
            given: groups as u8,
        });
    }

    let len = first.secret_len();
    let mut group_points = Points::new(len);
    for &gi in &order[..groups] {
        let mut members = Points::new(len);
        let mut threshold: Option<u8> = None;
        let mut seen = [false; MAX_COUNT];
        for s in shares.iter().filter(|s| s.group_index == gi) {
            match threshold {
                None => threshold = Some(s.member_threshold),
                Some(t) if t != s.member_threshold => {
                    return Err(Error::MismatchedMemberThresholds);
                }
                Some(_) => {}
            }
            if s.member_index as usize >= MAX_COUNT || seen[s.member_index as usize] {
                return Err(Error::DuplicateMemberIndex);
            }
            seen[s.member_index as usize] = true;
            members.push(s.member_index, s.value.as_bytes());
        }
        let threshold = threshold.expect("the group has at least one share");
        if members.n != threshold as usize {
            return Err(Error::WrongMemberCount {
                group_index: gi,
                needed: threshold,
                given: members.n as u8,
            });
        }
        let group_secret = recover_secret(threshold, &members)?;
        group_points.push(gi, group_secret.as_bytes());
    }

    let encrypted = recover_secret(first.group_threshold, &group_points)?;
    Ok(Secret::new(feistel(
        &encrypted,
        passphrase,
        first.iteration_exponent,
        first.identifier,
        first.extendable,
        false,
    )))
}

// --- splitting -------------------------------------------------------------

/// One group of a backup being made: how many of its shares must be
/// present, and how many it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupSpec {
    /// Shares of this group that must be present.
    pub threshold: u8,
    /// Shares this group has.
    pub count: u8,
}

/// Splits `secret` into the shares `groups` describes.
///
/// `secret` is the master secret: 16 to 32 bytes of even length, and the
/// BIP-32 seed the wallet comes from. `random` fills the slice it is
/// given with fresh randomness; on a device that is `osk_entropy`'s
/// pool, and in a test it is a fixed stream so the shares are
/// reproducible. Shares are written to `out` group by group, and the
/// count written is returned.
///
/// A share of a group whose threshold is 1 is the whole group secret, so
/// SLIP-39 allows such a group only one share.
#[allow(clippy::too_many_arguments)]
pub fn split(
    secret: &[u8],
    passphrase: &[u8],
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
    group_threshold: u8,
    groups: &[GroupSpec],
    random: &mut dyn FnMut(&mut [u8]),
    out: &mut [Share],
) -> Result<usize, Error> {
    check_passphrase(passphrase)?;
    let master = SecretBytes::new(secret)?;
    if identifier >= 1 << 15
        || iteration_exponent > MAX_ITERATION_EXPONENT
        || groups.is_empty()
        || groups.len() > MAX_COUNT
        || group_threshold == 0
        || group_threshold as usize > groups.len()
        || (group_threshold == 1 && groups.len() != 1)
    {
        return Err(Error::InvalidParameters);
    }
    for g in groups {
        if g.threshold == 0
            || g.count == 0
            || g.count as usize > MAX_COUNT
            || g.threshold > g.count
            || (g.threshold == 1 && g.count != 1)
        {
            return Err(Error::InvalidParameters);
        }
    }
    let total: usize = groups.iter().map(|g| g.count as usize).sum();
    if out.len() < total {
        return Err(Error::OutputTooSmall);
    }

    let encrypted = feistel(
        &master,
        passphrase,
        iteration_exponent,
        identifier,
        extendable,
        true,
    );
    let mut group_secrets = [const { None }; MAX_COUNT];
    split_secret(
        group_threshold,
        groups.len() as u8,
        &encrypted,
        random,
        &mut group_secrets,
    );

    let mut n = 0;
    for (gi, spec) in groups.iter().enumerate() {
        let group_secret = group_secrets[gi].take().expect("one secret per group");
        let mut members = [const { None }; MAX_COUNT];
        split_secret(
            spec.threshold,
            spec.count,
            &group_secret,
            random,
            &mut members,
        );
        for (mi, member) in members.iter_mut().take(spec.count as usize).enumerate() {
            let value = member.take().expect("one secret per member");
            out[n] = Share::build(
                identifier,
                extendable,
                iteration_exponent,
                gi as u8,
                group_threshold,
                groups.len() as u8,
                mi as u8,
                spec.threshold,
                &value,
            );
            n += 1;
        }
    }
    Ok(n)
}

/// Splits one secret into `count` shares, `threshold` of which recover
/// it. Writes share `i` to `out[i]`.
fn split_secret(
    threshold: u8,
    count: u8,
    secret: &SecretBytes,
    random: &mut dyn FnMut(&mut [u8]),
    out: &mut [Option<SecretBytes>; MAX_COUNT],
) {
    let len = secret.as_bytes().len();
    if threshold == 1 {
        for slot in out.iter_mut().take(count as usize) {
            *slot = Some(secret.clone());
        }
        return;
    }

    let mut base = Points::new(len);
    let random_shares = threshold as usize - 2;
    let mut buf = [0u8; MAX_STRENGTH_BYTES];
    for (i, slot) in out.iter_mut().take(random_shares).enumerate() {
        random(&mut buf[..len]);
        base.push(i as u8, &buf[..len]);
        *slot = Some(SecretBytes {
            buf,
            len: len as u8,
        });
    }

    let mut digest_share = [0u8; MAX_STRENGTH_BYTES];
    random(&mut digest_share[DIGEST_LENGTH..len]);
    let digest = digest_of(&digest_share[DIGEST_LENGTH..len], secret.as_bytes());
    digest_share[..DIGEST_LENGTH].copy_from_slice(&digest);
    base.push(DIGEST_INDEX, &digest_share[..len]);
    base.push(SECRET_INDEX, secret.as_bytes());

    for (i, slot) in out
        .iter_mut()
        .enumerate()
        .take(count as usize)
        .skip(random_shares)
    {
        *slot = Some(base.at(i as u8));
    }
    buf.zeroize();
    digest_share.zeroize();
}
