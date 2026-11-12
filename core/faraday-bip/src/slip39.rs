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
//! use faraday_bip::slip39::{Share, recover};
//!
//! let share = Share::parse(
//!     "duckling enlarge academic academic agency result length solution \
//!      fridge kidney coal piece deal husband erode duke ajar critical \
//!      decision keyboard",
//! )
//! .unwrap();
//! let c = faraday_crypto::work::Compute::unchecked();
//! let secret = recover(&c, &[share], b"TREZOR").unwrap();
//! assert_eq!(secret.expose().as_bytes().len(), 16);
//! ```

use faraday_crypto::work::Compute;
use faraday_crypto::{Secret, SeedBytes, Zeroize, ZeroizeOnDrop, hmac_sha256, pbkdf2_hmac_sha256};

use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::keys::{MasterKey, Network as KeysNetwork};
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
/// Largest iteration exponent this implementation computes: 320,000
/// PBKDF2-HMAC-SHA256 rounds, a few seconds on a Raspberry Pi 3. The
/// field allows 15, which is 327,680,000 rounds, close to an hour there,
/// and a set of shares is read from outside, so a higher exponent is
/// refused by [`Error::IterationExponentTooHigh`] before any round runs.
///
/// Every wallet's sets fall under it: Trezor writes 1, the reference
/// implementation (`python-shamir-mnemonic`) writes 1 by default and 0
/// from its command line, Faraday writes 1, and the published vectors
/// use 0, 2 and 3. Two steps above the vectors cover a set someone made
/// by hand with a raised exponent.
pub const MAX_READ_ITERATION_EXPONENT: u8 = 5;
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
    /// The iteration exponent is above [`MAX_READ_ITERATION_EXPONENT`]:
    /// the shares ask for more PBKDF2 rounds than this implementation
    /// runs.
    IterationExponentTooHigh {
        /// The exponent the shares carry, or the one asked for.
        exponent: u8,
    },
    /// The output slice is too short for the shares asked for.
    OutputTooSmall,
    /// Shares of one group were needed, and more than one group's were
    /// given.
    MixedGroups,
    /// A member index above 15 was asked for.
    MemberIndexOutOfRange,
    /// The member index asked for is one of the shares given: that share
    /// exists.
    MemberIndexTaken,
    /// A share was asked of a group whose threshold is 1, which SLIP-39
    /// allows one share only.
    SingleMemberGroup,
    /// A new identifier was asked for a set whose extendable flag is
    /// clear: its identifier is in the encryption's salt, so the new set
    /// would recover another master secret.
    NotExtendable,
}

// --- the wordlist ----------------------------------------------------------

/// The word at `idx`, which must be below [`WORDLIST_LEN`].
pub fn word(idx: u16) -> &'static str {
    wordlist::WORDS[idx as usize]
}

/// The index of `w` in the wordlist, if it is there. A word is identified
/// by its first four letters, so a prefix of four or more letters finds
/// the word it begins.
///
/// Costs all 1024 entries for every word: the first four letters are
/// compared with every entry's in constant time, and the rest of the
/// typed word with the entry found, gathered by a second scan of the
/// whole list, masked.
pub fn index_of(w: &str) -> Option<u16> {
    use subtle::{Choice, ConditionallySelectable, ConstantTimeGreater};
    let wb = w.as_bytes();
    let n = wb.len().min(4);
    let mut key = [0u8; 4];
    key[..n].copy_from_slice(&wb[..n]);
    let mut found = 0u16;
    let mut hit = Choice::from(0);
    for (i, cand) in wordlist::WORDS.iter().enumerate() {
        let cb = cand.as_bytes();
        let m = cb.len().min(4);
        let mut first = [0u8; 4];
        first[..m].copy_from_slice(&cb[..m]);
        // A word under four letters must be a whole entry.
        let eq = if wb.len() >= 4 {
            first.ct_eq(&key)
        } else {
            first.ct_eq(&key) & (m as u32).ct_eq(&(wb.len() as u32))
        };
        found.conditional_assign(&(i as u16), eq & !hit);
        hit |= eq;
    }
    // The rest of the typed word is a prefix of the entry found. The
    // entry is gathered by a scan of the whole list into a fixed slot,
    // so no load follows the word found.
    if wb.len() > 8 {
        return None;
    }
    let mut entry = Zeroizing::new([0u8; 8]);
    let mut elen = 0u32;
    for (i, cand) in wordlist::WORDS.iter().enumerate() {
        let pick = (i as u16).ct_eq(&found);
        let cb = cand.as_bytes();
        for (k, slot) in entry.iter_mut().enumerate() {
            slot.conditional_assign(&cb.get(k).copied().unwrap_or(0), pick);
        }
        elen.conditional_assign(&(cb.len() as u32), pick);
    }
    let wl = wb.len() as u32;
    let mut ok = hit & (elen.ct_gt(&wl) | elen.ct_eq(&wl));
    for (k, &b) in wb.iter().enumerate().skip(4) {
        ok &= entry[k].ct_eq(&b);
    }
    bool::from(ok).then_some(found)
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

// The arithmetic runs the same instructions for every byte: no table is
// indexed by a share's bytes and no branch is taken on them.

/// `a · b` in GF(256) modulo x⁸ + x⁴ + x³ + x + 1, SLIP-39's field
/// (and AES's): eight rounds of shift and XOR, each kept or dropped by a
/// mask.
fn gf_mul(a: u8, b: u8) -> u8 {
    let (mut a, mut b, mut product) = (a, b, 0u8);
    for _ in 0..8 {
        product ^= a & (b & 1).wrapping_neg();
        let carry = (a >> 7).wrapping_neg();
        a = (a << 1) ^ (0x1B & carry);
        b >>= 1;
    }
    product
}

/// `a⁻¹` in GF(256), as `a²⁵⁴`: seven squarings, each multiplied in.
/// Zero gives zero, which no caller divides by: share indices differ.
fn gf_inv(a: u8) -> u8 {
    let (mut square, mut out) = (a, 1u8);
    for _ in 0..7 {
        square = gf_mul(square, square);
        out = gf_mul(out, square);
    }
    out
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
        // The basis polynomial of each point at `x`: the product over the
        // other points of (x_j - x) / (x_i - x_j), subtraction being XOR.
        // It is worked out from the indices, which are public; the values
        // are multiplied in without a branch.
        for i in 0..self.n {
            let (mut num, mut den) = (1u8, 1u8);
            for j in (0..self.n).filter(|&j| j != i) {
                num = gf_mul(num, self.x[j] ^ x);
                den = gf_mul(den, self.x[i] ^ self.x[j]);
            }
            let basis = gf_mul(num, gf_inv(den));
            for (k, byte) in out.iter_mut().take(self.len).enumerate() {
                *byte ^= gf_mul(self.y[i][k], basis);
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
    if !bool::from(digest_of(random_part, shared.as_bytes()).ct_eq(digest)) {
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
            chk ^= *g & ((b >> i) & 1).wrapping_neg();
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
        let mut indices = Zeroizing::new([0u16; MAX_WORDS]);
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
        let mut value = Zeroizing::new([0u8; MAX_STRENGTH_BYTES + 8]);
        let mut acc = Zeroizing::new(0u64);
        let mut bits = 0usize;
        let mut out = 0usize;
        for (i, &w) in value_words.iter().enumerate() {
            *acc = (*acc << 10) | u64::from(w);
            bits += 10;
            if i == 0 {
                if *acc >> (bits - padding) != 0 {
                    return Err(Error::NonZeroPadding);
                }
                bits -= padding;
                *acc &= (1u64 << bits) - 1;
            }
            while bits >= 8 {
                bits -= 8;
                value[out] = (*acc >> bits) as u8;
                out += 1;
            }
        }
        if !(MIN_STRENGTH_BYTES..=MAX_STRENGTH_BYTES).contains(&value_len)
            || !value_len.is_multiple_of(2)
        {
            return Err(Error::InvalidSecretLength);
        }

        let mut words = Zeroizing::new([0u16; MAX_WORDS]);
        words[..indices.len()].copy_from_slice(indices);
        Ok(Self {
            indices: *words,
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
    #[allow(
        clippy::too_many_arguments,
        reason = "one share's header fields in SLIP-39's order, then its value"
    )]
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

/// The exponent's refusal where it is above what is computed.
fn check_iteration_exponent(exponent: u8) -> Result<(), Error> {
    if exponent > MAX_READ_ITERATION_EXPONENT {
        return Err(Error::IterationExponentTooHigh { exponent });
    }
    Ok(())
}

fn check_passphrase(passphrase: &[u8]) -> Result<(), Error> {
    if passphrase.len() > MAX_PASSPHRASE_BYTES {
        return Err(Error::PassphraseTooLong);
    }
    if passphrase.iter().any(|&b| !(0x20..=0x7E).contains(&b)) {
        return Err(Error::PassphraseNotAscii);
    }
    Ok(())
}

/// Costs a PBKDF2 (through `pbkdf2_hmac_sha256`). The four-round
/// Feistel network SLIP-39 encrypts and decrypts the master secret
/// with. `forward` runs the rounds 0, 1, 2, 3 (encryption) and its
/// inverse runs 3, 2, 1, 0.
fn feistel(
    c: &Compute,
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
            c,
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

/// Costs a PBKDF2 (through `feistel_call`). The encrypted master secret
/// a split carries, which is `secret` under `passphrase`.
///
/// A device that writes shares needs this on its own, before any share
/// exists: the two fingerprints it shows for a passphrase are what
/// reading the shares back with and without it would give.
pub fn encrypt(
    c: &Compute,
    secret: &[u8],
    passphrase: &[u8],
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
) -> Result<Secret<SecretBytes>, Error> {
    feistel_call(
        c,
        secret,
        passphrase,
        identifier,
        extendable,
        iteration_exponent,
        true,
    )
}

/// Costs a PBKDF2 (through `feistel_call`). The master secret behind
/// `encrypted` under `passphrase`: the inverse of [`encrypt`].
pub fn decrypt(
    c: &Compute,
    encrypted: &[u8],
    passphrase: &[u8],
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
) -> Result<Secret<SecretBytes>, Error> {
    feistel_call(
        c,
        encrypted,
        passphrase,
        identifier,
        extendable,
        iteration_exponent,
        false,
    )
}

/// Costs a PBKDF2 (through `feistel`).
fn feistel_call(
    c: &Compute,
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
    check_iteration_exponent(iteration_exponent)?;
    let value = SecretBytes::new(value)?;
    Ok(Secret::new(feistel(
        c,
        &value,
        passphrase,
        iteration_exponent,
        identifier,
        extendable,
        forward,
    )))
}

// --- recovery --------------------------------------------------------------

/// An encrypted master secret (EMS) and what decrypting it needs besides
/// the passphrase: the identifier, the extendable flag and the iteration
/// exponent. It is what a set of shares holds, one level below the
/// passphrase: [`recover_ems`] reads it from a quorum and [`split_ems`]
/// writes it to a new set, so a set can be rotated without the
/// passphrase being typed.
///
/// The ciphertext is a secret: with an empty passphrase it decrypts to
/// the wallet's master secret. `Ems` has no `Debug` and no text form,
/// and its bytes are erased when it is dropped.
pub struct Ems {
    identifier: u16,
    extendable: bool,
    iteration_exponent: u8,
    ciphertext: Secret<SecretBytes>,
}

impl Ems {
    /// An EMS from its parts, the ciphertext as [`encrypt`] gives it.
    /// Refused: an identifier above fifteen bits or an exponent above the
    /// four-bit field ([`Error::InvalidParameters`]), and a ciphertext
    /// that is not 16 to 32 bytes of even length. The parameters must be
    /// the set's own, since decrypting uses them: a new identifier goes
    /// through [`Ems::with_identifier`].
    pub fn new(
        identifier: u16,
        extendable: bool,
        iteration_exponent: u8,
        ciphertext: &[u8],
    ) -> Result<Self, Error> {
        if identifier >= 1 << 15 || iteration_exponent > MAX_ITERATION_EXPONENT {
            return Err(Error::InvalidParameters);
        }
        Ok(Self {
            identifier,
            extendable,
            iteration_exponent,
            ciphertext: Secret::new(SecretBytes::new(ciphertext)?),
        })
    }

    /// The same set under a new identifier, which recovers the same master
    /// secret. Refused: a set whose extendable flag is clear
    /// ([`Error::NotExtendable`]), since its identifier is in the
    /// encryption's salt, and an identifier above fifteen bits
    /// ([`Error::InvalidParameters`]).
    pub fn with_identifier(&self, identifier: u16) -> Result<Ems, Error> {
        if !self.extendable {
            return Err(Error::NotExtendable);
        }
        Ems::new(
            identifier,
            true,
            self.iteration_exponent,
            self.ciphertext.expose().as_bytes(),
        )
    }

    /// The fifteen-bit identifier every share of the set carries.
    pub fn identifier(&self) -> u16 {
        self.identifier
    }

    /// Whether the set is extendable: the identifier is not in the
    /// encryption's salt.
    pub fn is_extendable(&self) -> bool {
        self.extendable
    }

    /// The iteration exponent: decrypting does `10000 << e` PBKDF2
    /// rounds in all.
    pub fn iteration_exponent(&self) -> u8 {
        self.iteration_exponent
    }

    /// The encrypted master secret itself.
    pub fn ciphertext(&self) -> &Secret<SecretBytes> {
        &self.ciphertext
    }

    /// Costs a PBKDF2 (through `feistel`). The master secret under
    /// `passphrase`, the BIP-32 seed. A wrong passphrase gives another
    /// master secret, not a refusal.
    pub fn decrypt(&self, c: &Compute, passphrase: &[u8]) -> Result<Secret<SecretBytes>, Error> {
        check_passphrase(passphrase)?;
        check_iteration_exponent(self.iteration_exponent)?;
        Ok(Secret::new(feistel(
            c,
            self.ciphertext.expose(),
            passphrase,
            self.iteration_exponent,
            self.identifier,
            self.extendable,
            false,
        )))
    }
}

/// The checks every share of one set passes against the first: the same
/// identifier, flag, exponent, group threshold and count and value
/// length, a group index inside the count, a group threshold no greater
/// than the count, and an exponent this implementation computes.
fn check_one_set(shares: &[Share]) -> Result<&Share, Error> {
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
    check_iteration_exponent(first.iteration_exponent)?;
    Ok(first)
}

/// Whether `shares`, still being gathered, can be part of one set: what
/// [`recover_ems`] refuses of one set and of each group, short of a group
/// or a set not being complete yet. So: shares of another backup, members
/// of one group that disagree about its threshold, a member given twice,
/// and more members of a group than its threshold.
pub fn check_gathered(shares: &[Share]) -> Result<(), Error> {
    check_one_set(shares)?;
    for (at, s) in shares.iter().enumerate() {
        let mut given = 1u8;
        for e in shares[..at]
            .iter()
            .filter(|e| e.group_index == s.group_index)
        {
            if e.member_threshold != s.member_threshold {
                return Err(Error::MismatchedMemberThresholds);
            }
            if e.member_index == s.member_index {
                return Err(Error::DuplicateMemberIndex);
            }
            given = given.saturating_add(1);
        }
        if given > s.member_threshold {
            return Err(Error::WrongMemberCount {
                group_index: s.group_index,
                needed: s.member_threshold,
                given,
            });
        }
    }
    Ok(())
}

/// The points of one group's shares among `shares`, and the group's
/// member threshold. Refused: shares that disagree about the threshold,
/// a member index given twice, and a count other than the threshold.
fn group_points(shares: &[Share], group_index: u8, len: usize) -> Result<(Points, u8), Error> {
    let mut members = Points::new(len);
    let mut threshold: Option<u8> = None;
    let mut seen = [false; MAX_COUNT];
    for s in shares.iter().filter(|s| s.group_index == group_index) {
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
            group_index,
            needed: threshold,
            given: members.n as u8,
        });
    }
    Ok((members, threshold))
}

/// The encrypted master secret behind `shares`, with its identifier,
/// flag and exponent. No PBKDF2 runs: the passphrase is not needed.
///
/// Refused as [`recover`] refuses a set: shares of another backup, a
/// group short of shares or with too many, a member given twice, a
/// digest that does not match, and an iteration exponent above
/// [`MAX_READ_ITERATION_EXPONENT`].
pub fn recover_ems(shares: &[Share]) -> Result<Ems, Error> {
    let first = check_one_set(shares)?;

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
    let mut group_secrets = Points::new(len);
    for &gi in &order[..groups] {
        let (members, threshold) = group_points(shares, gi, len)?;
        let group_secret = recover_secret(threshold, &members)?;
        group_secrets.push(gi, group_secret.as_bytes());
    }

    let ciphertext = recover_secret(first.group_threshold, &group_secrets)?;
    Ok(Ems {
        identifier: first.identifier,
        extendable: first.extendable,
        iteration_exponent: first.iteration_exponent,
        ciphertext: Secret::new(ciphertext),
    })
}

/// Costs a PBKDF2 (through `Ems::decrypt`). The master secret behind
/// `shares`, under `passphrase`: [`recover_ems`], then decryption.
///
/// The result is the BIP-32 seed: hand it to
/// `bitcoin::bip32::Xpriv::new_master`. It is not BIP-39 entropy.
///
/// Every refusal names what is wrong with the set: a share from another
/// backup, a group short of shares, a digest that does not match. A wrong
/// passphrase is not one of them — it gives a different master secret,
/// and so a different wallet. A set whose iteration exponent is above
/// [`MAX_READ_ITERATION_EXPONENT`] is refused before any PBKDF2 round.
pub fn recover(
    c: &Compute,
    shares: &[Share],
    passphrase: &[u8],
) -> Result<Secret<SecretBytes>, Error> {
    check_passphrase(passphrase)?;
    recover_ems(shares)?.decrypt(c, passphrase)
}

/// A new share of one group: the group's quorum `shares` interpolated
/// at `member_index` (0 to 15). The share carries the same identifier,
/// flag, exponent, group fields and member threshold, so it joins the
/// set as one more member; at the index of a lost share it is that
/// share again, word for word. No PBKDF2 runs.
///
/// Refused, besides what [`recover_ems`] refuses of one set and of one
/// group: shares of more than one group ([`Error::MixedGroups`]), an
/// index above 15 ([`Error::MemberIndexOutOfRange`]), a group whose
/// threshold is 1 ([`Error::SingleMemberGroup`]: SLIP-39 allows it one
/// share), an index among the shares given ([`Error::MemberIndexTaken`]),
/// and a quorum whose digest does not match ([`Error::BadDigest`]): such
/// a group is never extended.
pub fn derive_member(shares: &[Share], member_index: u8) -> Result<Share, Error> {
    let first = check_one_set(shares)?;
    if shares.iter().any(|s| s.group_index != first.group_index) {
        return Err(Error::MixedGroups);
    }
    if member_index as usize >= MAX_COUNT {
        return Err(Error::MemberIndexOutOfRange);
    }
    let (members, threshold) = group_points(shares, first.group_index, first.secret_len())?;
    if threshold == 1 {
        return Err(Error::SingleMemberGroup);
    }
    if shares.iter().any(|s| s.member_index == member_index) {
        return Err(Error::MemberIndexTaken);
    }
    recover_secret(threshold, &members)?;
    Ok(Share::build(
        first.identifier,
        first.extendable,
        first.iteration_exponent,
        first.group_index,
        first.group_threshold,
        first.group_count,
        member_index,
        threshold,
        &members.at(member_index),
    ))
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

/// The random bytes [`split`] asks `random` for, in all, for a master
/// secret of `secret_len` bytes and these groups (a lookup): for the
/// groups and for each group's members, nothing at a threshold of 1, and
/// otherwise `threshold − 2` shares and the digest share's random part
/// (SLIP-39's "Splitting a secret").
pub fn random_len(secret_len: usize, group_threshold: u8, groups: &[GroupSpec]) -> usize {
    let one = |threshold: u8| match usize::from(threshold) {
        0 | 1 => 0,
        t => (t - 2) * secret_len + secret_len.saturating_sub(DIGEST_LENGTH),
    };
    one(group_threshold) + groups.iter().map(|g| one(g.threshold)).sum::<usize>()
}

/// The refusals of a split's shape: the groups, the thresholds and the
/// room for the shares, with an exponent above what is computed refused
/// among them.
fn check_split(
    iteration_exponent: u8,
    group_threshold: u8,
    groups: &[GroupSpec],
    room: usize,
) -> Result<(), Error> {
    if groups.is_empty()
        || groups.len() > MAX_COUNT
        || group_threshold == 0
        || group_threshold as usize > groups.len()
        || (group_threshold == 1 && groups.len() != 1)
    {
        return Err(Error::InvalidParameters);
    }
    check_iteration_exponent(iteration_exponent)?;
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
    if room < total {
        return Err(Error::OutputTooSmall);
    }
    Ok(())
}

/// Costs a PBKDF2 (through `feistel`). Splits `secret` into the shares
/// `groups` describes: its encryption under `passphrase`, then
/// [`split_ems`].
///
/// `secret` is the master secret: 16 to 32 bytes of even length, and the
/// BIP-32 seed the wallet comes from. `random` fills the slice it is
/// given with fresh randomness; on a device that is `faraday_entropy`'s
/// pool, and in a test it is a fixed stream so the shares are
/// reproducible. Shares are written to `out` group by group, and the
/// count written is returned.
///
/// A share of a group whose threshold is 1 is the whole group secret, so
/// SLIP-39 allows such a group only one share.
#[allow(
    clippy::too_many_arguments,
    reason = "SLIP-39's split inputs: secret, passphrase, identifier, iteration exponent, thresholds and groups"
)]
pub fn split(
    c: &Compute,
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
    if identifier >= 1 << 15 || iteration_exponent > MAX_ITERATION_EXPONENT {
        return Err(Error::InvalidParameters);
    }
    check_split(iteration_exponent, group_threshold, groups, out.len())?;
    let ems = Ems {
        identifier,
        extendable,
        iteration_exponent,
        ciphertext: Secret::new(feistel(
            c,
            &master,
            passphrase,
            iteration_exponent,
            identifier,
            extendable,
            true,
        )),
    };
    split_ems(&ems, group_threshold, groups, random, out)
}

/// Costs a PBKDF2 and a master key derivation. The fixed 1 of 1 of a
/// master secret: its one share, the same for one secret every time.
///
/// `secret` is 16 or 32 bytes, the lengths hardware wallets read; the
/// other lengths SLIP-39 allows (20, 24, 28) and anything else are
/// refused with [`Error::InvalidSecretLength`]. The identifier is the
/// first 15 bits of the master fingerprint (its two high bytes' leading
/// bits, `fingerprint >> 17`, the same end of it as codex32's
/// `identifier_for` takes). The set is extendable, one group of one share
/// (a threshold of 1 draws no randomness), iteration exponent 1 and an
/// empty passphrase, as New key and Trezor make them.
pub fn one_of_one(c: &Compute, secret: &[u8]) -> Result<Share, Error> {
    if secret.len() != 16 && secret.len() != 32 {
        return Err(Error::InvalidSecretLength);
    }
    let seed = SeedBytes::new(secret).ok_or(Error::InvalidSecretLength)?;
    let fingerprint = MasterKey::from_seed_bytes(c, &Secret::new(seed), KeysNetwork::Mainnet)
        .fingerprint()
        .0;
    let identifier = (u32::from_be_bytes(fingerprint) >> 17) as u16;
    let mut out = [Share::default()];
    split(
        c,
        secret,
        b"",
        identifier,
        true,
        1,
        1,
        &[GroupSpec {
            threshold: 1,
            count: 1,
        }],
        &mut |_| unreachable!("a threshold of 1 asks for no randomness"),
        &mut out,
    )?;
    let [share] = out;
    Ok(share)
}

/// A new set of shares for `ems` under these groups. No PBKDF2 runs:
/// the set opens under the passphrase the EMS was encrypted with, which
/// is not needed here. `random` and `out` are as [`split`]'s, and the
/// same random bytes give the same shares as [`split`] of the master
/// secret behind `ems`.
///
/// Refused as [`split`] refuses a shape: no groups or more than 16, a
/// threshold of 0 or above its count, a threshold of 1 over several
/// groups or members, an exponent above [`MAX_READ_ITERATION_EXPONENT`],
/// and too little room in `out`.
pub fn split_ems(
    ems: &Ems,
    group_threshold: u8,
    groups: &[GroupSpec],
    random: &mut dyn FnMut(&mut [u8]),
    out: &mut [Share],
) -> Result<usize, Error> {
    check_split(ems.iteration_exponent, group_threshold, groups, out.len())?;
    let mut group_secrets = [const { None }; MAX_COUNT];
    split_secret(
        group_threshold,
        groups.len() as u8,
        ems.ciphertext.expose(),
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
                ems.identifier,
                ems.extendable,
                ems.iteration_exponent,
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
