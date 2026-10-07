//! FROST threshold signing (BIP 445) and the trusted dealer that makes
//! the shares.
//!
//! Two layers:
//!
//! - [`nonce_gen`], [`nonce_agg`], [`session_values`], [`sign`],
//!   [`partial_sig_verify`], [`partial_sig_agg`] and
//!   [`deterministic_sign`] are BIP 445's algorithms, written over the
//!   curve primitives the same way [`crate::musig`] writes BIP-327's and
//!   sharing its scalar arithmetic, its tagged hash and its point
//!   helpers. The nonce, its aggregate, a partial signature and a tweak
//!   are byte for byte BIP-327's, so [`PubNonce`], [`AggNonce`],
//!   [`PartialSig`] and [`Tweak`] are that module's types, and the tweak
//!   context `(Q, gacc, tacc)` is an [`AggregateKey`].
//! - [`deal`], [`recover_share`] and [`recover_info`] are the trusted
//!   dealer of `docs/PLANNING.md` §16.103: `t` shares chosen freely fix
//!   the polynomial, the other `n - t` shares and the group key follow,
//!   and any `t` shares rebuild a lost share or the public record. The
//!   group secret is never returned and never computed: the group key
//!   comes from the public shares.
//!
//! Identifiers are 0-based and at most [`MAX_PARTICIPANTS`] of them. A
//! participant with identifier `id` holds the polynomial's value at
//! `x = id + 1`, and the group secret is its value at `x = 0`; the
//! interpolation here works in identifier coordinates, where the secret
//! sits at the identifier `-1`.
//!
//! A [`SecShare`] and a [`SecNonce`] are not `Clone`, are wiped on drop,
//! and [`sign`] takes the nonce by value, so one nonce cannot sign twice.

use alloc::vec::Vec;
use core::fmt;

use bitcoin::secp256k1::{Parity, PublicKey, Secp256k1, Verification};
use osk_crypto::Zeroize;

use crate::musig::{
    self, CURVE_ORDER, MaybePoint, add_mod_order, at_least_order, base_mul, cbytes_ext, cpoint_ext,
    generator, mul_mod_order, negate_mod_order, one, parity_factor, point_add, point_mul,
    point_negate, reduce, tagged_hash,
};

pub use crate::musig::{AggNonce, AggregateKey, PartialSig, PubNonce, Tweak};

/// The largest participant count BIP 445 allows.
pub const MAX_PARTICIPANTS: usize = 128;

const TAG_AUX: &[u8] = b"BIP0445/aux";
const TAG_NONCE: &[u8] = b"BIP0445/nonce";
const TAG_NONCECOEF: &[u8] = b"BIP0445/noncecoef";
const TAG_DETERMINISTIC_NONCE: &[u8] = b"BIP0445/deterministic/nonce";
const TAG_CHALLENGE: &[u8] = b"BIP0340/challenge";

// ---------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------

/// Which contribution a participant or the coordinator got wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contrib {
    /// One signer's public nonce.
    PubNonce,
    /// The coordinator's aggregate nonce.
    AggNonce,
    /// The aggregate of the other signers' nonces, which a
    /// deterministic signer signs against.
    AggOtherNonce,
    /// One signer's partial signature.
    PartialSig,
}

/// Why a session, a contribution or a set of shares was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The threshold is not between one and the participant count.
    ThresholdRange,
    /// There are more participants than BIP 445 allows.
    TooManyParticipants,
    /// The signer count is below the threshold or above the participant
    /// count.
    SignerCount,
    /// The public share list is not as long as the identifier list.
    PubShareCount,
    /// The identifier at this position is not below the participant
    /// count.
    IdOutOfRange {
        /// Its position in the identifier list.
        index: usize,
    },
    /// The identifier list names one participant twice.
    DuplicateIds,
    /// The public share at this position is no point on the curve.
    InvalidPubShare {
        /// Its position in the public share list.
        index: usize,
    },
    /// The public shares do not interpolate to the group key.
    ThreshPkMismatch,
    /// The public shares interpolate to the point at infinity, which is
    /// no group key.
    ThreshPkInfinity,
    /// Fewer than `t` public shares are present, so the record states no
    /// key.
    TooFewPubShares,
    /// The public shares do not lie on one polynomial of degree `t - 1`.
    PubSharesNotOnOnePolynomial,
    /// A tweak is not a scalar below the curve order.
    TweakOutOfRange,
    /// Tweaking the group key gives the point at infinity.
    TweakInfinity,
    /// The first half of the secret nonce is zero or at or above the
    /// curve order.
    FirstSecNonceOutOfRange,
    /// The second half of the secret nonce is zero or at or above the
    /// curve order.
    SecondSecNonceOutOfRange,
    /// The secret share is zero or at or above the curve order.
    SecShareOutOfRange,
    /// The signer's identifier is not one of the session's.
    NotASigner,
    /// The signer's public share is not the one the session lists for
    /// it.
    PubShareMismatch,
    /// The partial signature count differs from the signer count.
    PartialSigCount,
    /// The nonce, share and identifier lists have different lengths.
    VerifyInputCount,
    /// The signer position to verify is past the end of the signer set.
    SignerIndexOutOfRange,
    /// A participant or the coordinator sent something this session
    /// cannot read.
    InvalidContribution {
        /// The signer's position in the list it contributed to, or
        /// `None` for the coordinator.
        signer: Option<usize>,
        /// What it contributed.
        contrib: Contrib,
    },
    /// The partial signature is not the one this signer's nonce and
    /// share make over this message.
    VerifyFailed,
    /// A point operation gave the point at infinity where a point was
    /// needed.
    Infinity,
    /// Nonce aggregation was given no nonces.
    NoSigners,
    /// The dealer was given a number of chosen shares other than `t`.
    ChosenCount,
    /// The dealer was given one identifier twice.
    ChosenDuplicate,
    /// The dealer was given an identifier that is not below `n`.
    ChosenIdOutOfRange,
    /// A share the dealer computed is zero, which is no share.
    ZeroShare,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::ThresholdRange => "the threshold must be between 1 and the participant count",
            Error::TooManyParticipants => "there may be at most 128 participants",
            Error::SignerCount => "the signer count must be between the threshold and n",
            Error::PubShareCount => "there must be one public share for each identifier",
            Error::IdOutOfRange { .. } => "an identifier is not below the participant count",
            Error::DuplicateIds => "the identifier list names one participant twice",
            Error::InvalidPubShare { .. } => "a public share is no point on the curve",
            Error::ThreshPkMismatch => "the public shares do not give this group key",
            Error::ThreshPkInfinity => "the public shares give the point at infinity",
            Error::TooFewPubShares => "fewer public shares are present than the threshold",
            Error::PubSharesNotOnOnePolynomial => "the public shares are not of one polynomial",
            Error::TweakOutOfRange => "the tweak must be less than the curve order",
            Error::TweakInfinity => "tweaking the group key gives the point at infinity",
            Error::FirstSecNonceOutOfRange => "the first half of the secret nonce is out of range",
            Error::SecondSecNonceOutOfRange => {
                "the second half of the secret nonce is out of range"
            }
            Error::SecShareOutOfRange => "the secret share is out of range",
            Error::NotASigner => "the signer's identifier is not one of the session's",
            Error::PubShareMismatch => "the session lists another public share for this signer",
            Error::PartialSigCount => "there must be one partial signature for each signer",
            Error::VerifyInputCount => "the nonce, share and identifier lists differ in length",
            Error::SignerIndexOutOfRange => "the signer position is past the end of the signer set",
            Error::InvalidContribution { .. } => "a contribution to this session cannot be read",
            Error::VerifyFailed => "partial signature does not verify",
            Error::Infinity => "a point operation gave the point at infinity",
            Error::NoSigners => "nonce aggregation needs at least one nonce",
            Error::ChosenCount => "exactly t shares are chosen and the rest are computed",
            Error::ChosenDuplicate => "two chosen shares carry the same identifier",
            Error::ChosenIdOutOfRange => "a chosen identifier is not below the participant count",
            Error::ZeroShare => "a computed share is zero",
        })
    }
}

impl core::error::Error for Error {}

/// The tweak helpers report in [`crate::musig`]'s terms; these are the
/// same two refusals.
fn tweak_error(e: musig::Error) -> Error {
    match e {
        musig::Error::TweakOutOfRange => Error::TweakOutOfRange,
        _ => Error::TweakInfinity,
    }
}

// ---------------------------------------------------------------------
// Scalars: division mod n, and small signed values
// ---------------------------------------------------------------------

/// `a - b` for `a >= b`, over 256-bit big-endian values.
fn sub_256(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut borrow = 0i16;
    for i in (0..32).rev() {
        let d = i16::from(a[i]) - i16::from(b[i]) - borrow;
        if d < 0 {
            out[i] = (d + 256) as u8;
            borrow = 1;
        } else {
            out[i] = d as u8;
            borrow = 0;
        }
    }
    out
}

/// `a + b` over 256-bit big-endian values, with the bit that carried out.
fn add_256(a: &[u8; 32], b: &[u8; 32]) -> ([u8; 32], u8) {
    let mut out = [0u8; 32];
    let mut carry = 0u16;
    for i in (0..32).rev() {
        let s = u16::from(a[i]) + u16::from(b[i]) + carry;
        out[i] = s as u8;
        carry = s >> 8;
    }
    (out, carry as u8)
}

/// `a` shifted right one bit, with `top` shifted in as the new high bit.
fn shift_right_one(a: &[u8; 32], top: u8) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut carry = top & 1;
    for i in 0..32 {
        out[i] = (carry << 7) | (a[i] >> 1);
        carry = a[i] & 1;
    }
    out
}

fn is_odd(a: &[u8; 32]) -> bool {
    a[31] & 1 == 1
}

/// `a / 2 mod n`: halving an odd value adds `n` first, which is odd, so
/// the sum is even and one bit may carry out.
fn half_mod_order(a: &[u8; 32]) -> [u8; 32] {
    if is_odd(a) {
        let (sum, carry) = add_256(a, &CURVE_ORDER);
        shift_right_one(&sum, carry)
    } else {
        shift_right_one(a, 0)
    }
}

/// `(a - b) mod n`, for `a` and `b` already reduced.
fn sub_mod_order(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    add_mod_order(a, &negate_mod_order(b))
}

/// `a⁻¹ mod n` by the binary extended Euclidean algorithm, for `a` in
/// `1..n`.
///
/// This runs in a time that depends on `a`. Every value inverted here is
/// a product of the participant identifiers, which are public numbers
/// below 128 that the signer set states in the clear, so what the timing
/// reveals is already known. The secret share is only ever multiplied,
/// by `mul_mod_order`, which does not branch on its input.
fn invert_mod_order(a: &[u8; 32]) -> [u8; 32] {
    // Zero has no inverse and would loop below forever. No caller can
    // pass it: every divisor is a product of differences of distinct
    // identifiers, each below the order. The answer for it is zero, so
    // that a caller that somehow did gets a wrong scalar and not a hang.
    if *a == [0u8; 32] {
        return [0u8; 32];
    }
    let (mut u, mut v) = (*a, CURVE_ORDER);
    let (mut x1, mut x2) = (one(), [0u8; 32]);
    let unit = one();
    while u != unit && v != unit {
        while !is_odd(&u) {
            u = shift_right_one(&u, 0);
            x1 = half_mod_order(&x1);
        }
        while !is_odd(&v) {
            v = shift_right_one(&v, 0);
            x2 = half_mod_order(&x2);
        }
        if u >= v {
            u = sub_256(&u, &v);
            x1 = sub_mod_order(&x1, &x2);
        } else {
            v = sub_256(&v, &u);
            x2 = sub_mod_order(&x2, &x1);
        }
    }
    if u == unit { x1 } else { x2 }
}

/// `(a / b) mod n`, for `b` in `1..n`.
fn div_mod_order(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    mul_mod_order(a, &invert_mod_order(b))
}

/// A small signed number as a scalar mod `n`.
fn scalar_of(value: i64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&value.unsigned_abs().to_be_bytes());
    if value < 0 {
        negate_mod_order(&out)
    } else {
        out
    }
}

// ---------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------

fn has_duplicates(ids: &[u32]) -> bool {
    ids.iter()
        .enumerate()
        .any(|(i, id)| ids[i + 1..].contains(id))
}

/// BIP 445's `serialize_ids`: the identifiers sorted, each four bytes
/// big-endian, so that the binding value does not depend on the order
/// the signer set is given in.
fn serialize_ids(ids: &[u32]) -> Vec<u8> {
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    let mut out = Vec::with_capacity(4 * sorted.len());
    for id in sorted {
        out.extend_from_slice(&id.to_be_bytes());
    }
    out
}

/// BIP 445's `DeriveInterpolatingValue`: the Lagrange coefficient of
/// `my_id` at the group secret, in identifier coordinates.
pub fn derive_interpolating_value(ids: &[u32], my_id: u32) -> Result<[u8; 32], Error> {
    if has_duplicates(ids) {
        return Err(Error::DuplicateIds);
    }
    if !ids.contains(&my_id) {
        return Err(Error::NotASigner);
    }
    let (mut num, mut deno) = (one(), one());
    for &curr in ids {
        if curr == my_id {
            continue;
        }
        num = mul_mod_order(&num, &scalar_of(i64::from(curr) + 1));
        deno = mul_mod_order(&deno, &scalar_of(i64::from(curr) - i64::from(my_id)));
    }
    Ok(div_mod_order(&num, &deno))
}

/// BIP 445's `DerivePubshareAt`: the public share the polynomial through
/// `ids`' public shares takes at the identifier `x`, which may be the
/// point at infinity.
pub fn derive_pubshare_at<C: Verification>(
    secp: &Secp256k1<C>,
    ids: &[u32],
    pubshares: &[PublicKey],
    x: i64,
) -> Result<Option<PublicKey>, Error> {
    if ids.len() != pubshares.len() {
        return Err(Error::PubShareCount);
    }
    if has_duplicates(ids) {
        return Err(Error::DuplicateIds);
    }
    let mut sum: MaybePoint = None;
    for (&my_id, point) in ids.iter().zip(pubshares) {
        let (mut num, mut deno) = (one(), one());
        for &curr in ids {
            if curr == my_id {
                continue;
            }
            num = mul_mod_order(&num, &scalar_of(x - i64::from(curr)));
            deno = mul_mod_order(&deno, &scalar_of(i64::from(my_id) - i64::from(curr)));
        }
        let term = point_mul(secp, Some(*point), &div_mod_order(&num, &deno))
            .map_err(|_| Error::Infinity)?;
        sum = point_add(secp, sum, term).map_err(|_| Error::Infinity)?;
    }
    Ok(sum)
}

/// BIP 445's `DeriveThreshPubkey`: the group key, which is the
/// polynomial at the identifier `-1`.
pub fn derive_thresh_pubkey<C: Verification>(
    secp: &Secp256k1<C>,
    ids: &[u32],
    pubshares: &[PublicKey],
) -> Result<PublicKey, Error> {
    derive_pubshare_at(secp, ids, pubshares, -1)?.ok_or(Error::ThreshPkInfinity)
}

/// The scalar the polynomial through `ids`' secret shares takes at the
/// identifier `x`.
fn interpolate_secret_at(ids: &[u32], shares: &[[u8; 32]], x: i64) -> [u8; 32] {
    let mut sum = [0u8; 32];
    for (&my_id, value) in ids.iter().zip(shares) {
        let (mut num, mut deno) = (one(), one());
        for &curr in ids {
            if curr == my_id {
                continue;
            }
            num = mul_mod_order(&num, &scalar_of(x - i64::from(curr)));
            deno = mul_mod_order(&deno, &scalar_of(i64::from(my_id) - i64::from(curr)));
        }
        sum = add_mod_order(&sum, &mul_mod_order(value, &div_mod_order(&num, &deno)));
    }
    sum
}

// ---------------------------------------------------------------------
// The public record
// ---------------------------------------------------------------------

/// What a group publishes about itself: the threshold, the group key and
/// one public share per participant, by identifier, absent where the
/// record does not carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThresholdInfo {
    /// How many shares sign.
    pub t: usize,
    /// The group key, before any tweak.
    pub thresh_pk: PublicKey,
    /// The public shares, entry `i` belonging to identifier `i`.
    pub pubshares: Vec<Option<PublicKey>>,
}

impl ThresholdInfo {
    /// The participant count.
    pub fn n(&self) -> usize {
        self.pubshares.len()
    }

    /// BIP 445's `ValidateThresholdInfo`: at least `t` public shares are
    /// present, they lie on one polynomial of degree `t - 1`, and that
    /// polynomial gives this group key.
    pub fn validate<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<(), Error> {
        let n = self.n();
        if self.t < 1 || self.t > n {
            return Err(Error::ThresholdRange);
        }
        if n > MAX_PARTICIPANTS {
            return Err(Error::TooManyParticipants);
        }
        let present: Vec<(u32, PublicKey)> = self
            .pubshares
            .iter()
            .enumerate()
            .filter_map(|(i, share)| share.map(|s| (i as u32, s)))
            .collect();
        if present.len() < self.t {
            return Err(Error::TooFewPubShares);
        }
        let base_ids: Vec<u32> = present[..self.t].iter().map(|(i, _)| *i).collect();
        let base_points: Vec<PublicKey> = present[..self.t].iter().map(|(_, p)| *p).collect();
        for (id, point) in &present[self.t..] {
            if derive_pubshare_at(secp, &base_ids, &base_points, i64::from(*id))? != Some(*point) {
                return Err(Error::PubSharesNotOnOnePolynomial);
            }
        }
        if derive_thresh_pubkey(secp, &base_ids, &base_points)? != self.thresh_pk {
            return Err(Error::ThreshPkMismatch);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------
// Secrets
// ---------------------------------------------------------------------

/// One participant's secret share: a scalar in `1..n`.
///
/// Not `Clone`, wiped on drop, and never printed.
pub struct SecShare([u8; 32]);

impl SecShare {
    /// Reads the 32 bytes, which must be a scalar in `1..n`. This is the
    /// door every share source comes through: the shell's entropy, dice,
    /// or the bytes of a 24-word phrase.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, Error> {
        if *bytes == [0u8; 32] || at_least_order(bytes) {
            return Err(Error::SecShareOutOfRange);
        }
        Ok(SecShare(*bytes))
    }

    /// The public share, `secshare · G`.
    pub fn public_share<C: Verification>(&self, secp: &Secp256k1<C>) -> PublicKey {
        base_mul(secp, &self.0)
            .ok()
            .flatten()
            .expect("a scalar in 1..n has a point")
    }

    /// The 32 bytes, for the 24-word encoding that carries a share on
    /// paper and for nothing else.
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for SecShare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecShare(..)")
    }
}

impl Zeroize for SecShare {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for SecShare {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl osk_crypto::ZeroizeOnDrop for SecShare {}

/// A secret nonce: two scalars in `1..n`, 64 bytes.
///
/// Not `Clone`, wiped on drop, and consumed by value by [`sign`], so one
/// nonce cannot sign twice.
pub struct SecNonce {
    k1: [u8; 32],
    k2: [u8; 32],
}

impl SecNonce {
    /// Reads BIP 445's 64-byte serialisation, both halves of which must
    /// be scalars in `1..n`.
    ///
    /// This and [`SecNonce::serialize`] exist for the vectors and for
    /// §16.103's bound nonce file, which travels on the stick between
    /// the two locations of one signing. Nothing else the device keeps
    /// holds a secret nonce.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 64 {
            return Err(Error::FirstSecNonceOutOfRange);
        }
        let k1: [u8; 32] = bytes[..32].try_into().expect("32 bytes");
        let k2: [u8; 32] = bytes[32..].try_into().expect("32 bytes");
        if k1 == [0u8; 32] || at_least_order(&k1) {
            return Err(Error::FirstSecNonceOutOfRange);
        }
        if k2 == [0u8; 32] || at_least_order(&k2) {
            return Err(Error::SecondSecNonceOutOfRange);
        }
        Ok(SecNonce { k1, k2 })
    }

    /// The 64 bytes.
    pub fn serialize(&self) -> [u8; 64] {
        let mut out = [0u8; 64];
        out[..32].copy_from_slice(&self.k1);
        out[32..].copy_from_slice(&self.k2);
        out
    }

    /// The public nonce this secret nonce belongs to.
    pub fn public_nonce<C: Verification>(&self, secp: &Secp256k1<C>) -> Result<PubNonce, Error> {
        pubnonce_of(secp, &self.k1, &self.k2)
    }
}

impl fmt::Debug for SecNonce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecNonce(..)")
    }
}

impl Zeroize for SecNonce {
    fn zeroize(&mut self) {
        self.k1.zeroize();
        self.k2.zeroize();
    }
}

impl Drop for SecNonce {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl osk_crypto::ZeroizeOnDrop for SecNonce {}

fn pubnonce_of<C: Verification>(
    secp: &Secp256k1<C>,
    k1: &[u8; 32],
    k2: &[u8; 32],
) -> Result<PubNonce, Error> {
    let mut out = [0u8; 66];
    for (half, k) in [k1, k2].iter().enumerate() {
        let point = base_mul(secp, k)
            .map_err(|_| Error::Infinity)?
            .ok_or(Error::Infinity)?;
        out[half * 33..half * 33 + 33].copy_from_slice(&point.serialize());
    }
    PubNonce::from_bytes(&out).map_err(|_| Error::Infinity)
}

// ---------------------------------------------------------------------
// Reading what others send
// ---------------------------------------------------------------------

/// Reads the public share the participant at `index` published. A
/// participant that publishes something else is named.
pub fn parse_pubshare(bytes: &[u8], index: usize) -> Result<PublicKey, Error> {
    PublicKey::from_slice(bytes).map_err(|_| Error::InvalidPubShare { index })
}

/// Reads the public nonce the signer at `index` sent.
pub fn parse_pubnonce(bytes: &[u8], index: usize) -> Result<PubNonce, Error> {
    PubNonce::from_bytes(bytes).map_err(|_| Error::InvalidContribution {
        signer: Some(index),
        contrib: Contrib::PubNonce,
    })
}

/// Reads the partial signature the signer at `index` sent.
pub fn parse_psig(bytes: &[u8], index: usize) -> Result<PartialSig, Error> {
    PartialSig::from_bytes(bytes).map_err(|_| Error::InvalidContribution {
        signer: Some(index),
        contrib: Contrib::PartialSig,
    })
}

/// Reads an aggregate nonce, which the coordinator sends: either the
/// session's, or the aggregate of the other signers' nonces that a
/// deterministic signer signs against.
pub fn parse_aggnonce(bytes: &[u8], contrib: Contrib) -> Result<AggNonce, Error> {
    AggNonce::from_bytes(bytes).map_err(|_| Error::InvalidContribution {
        signer: None,
        contrib,
    })
}

// ---------------------------------------------------------------------
// Nonces
// ---------------------------------------------------------------------

/// BIP 445's `NonceGen`, with the `rand` the caller draws.
///
/// The caller draws `rand` afresh for every call; the vectors fix it.
/// Every other input is optional, and each one that is given binds the
/// nonce to it. `extra_in` is the caller's: this pass fixes no use for
/// it.
pub fn nonce_gen<C: Verification>(
    secp: &Secp256k1<C>,
    rand: &[u8; 32],
    secshare: Option<&SecShare>,
    pubshare: Option<&PublicKey>,
    thresh_pk_xonly: Option<&[u8; 32]>,
    msg: Option<&[u8]>,
    extra_in: Option<&[u8]>,
) -> Result<(SecNonce, PubNonce), Error> {
    let mut seed = *rand;
    if let Some(share) = secshare {
        let masked = tagged_hash(TAG_AUX, &[rand]);
        for (s, m) in seed.iter_mut().zip(share.0.iter().zip(masked)) {
            *s = m.0 ^ m.1;
        }
    }
    let pubshare_bytes = pubshare.map(PublicKey::serialize);
    let pubshare_bytes = pubshare_bytes.as_ref().map_or(&[][..], |p| &p[..]);
    let thresh_pk_xonly = thresh_pk_xonly.map_or(&[][..], |k| &k[..]);
    let mut prefixed_msg = Vec::new();
    match msg {
        None => prefixed_msg.push(0u8),
        Some(m) => {
            prefixed_msg.push(1u8);
            prefixed_msg.extend_from_slice(&(m.len() as u64).to_be_bytes());
            prefixed_msg.extend_from_slice(m);
        }
    }
    let extra_in = extra_in.unwrap_or(&[]);
    let mut k = [[0u8; 32]; 2];
    for (i, k) in k.iter_mut().enumerate() {
        *k = reduce(tagged_hash(
            TAG_NONCE,
            &[
                &seed,
                &[pubshare_bytes.len() as u8],
                pubshare_bytes,
                &[thresh_pk_xonly.len() as u8],
                thresh_pk_xonly,
                &prefixed_msg,
                &(extra_in.len() as u32).to_be_bytes(),
                extra_in,
                &[i as u8],
            ],
        ));
        if *k == [0u8; 32] {
            return Err(Error::FirstSecNonceOutOfRange);
        }
    }
    seed.zeroize();
    let pubnonce = pubnonce_of(secp, &k[0], &k[1])?;
    Ok((SecNonce { k1: k[0], k2: k[1] }, pubnonce))
}

/// BIP 445's `NonceAgg`, which is BIP-327's sum.
pub fn nonce_agg(pubnonces: &[PubNonce]) -> Result<AggNonce, Error> {
    let secp = Secp256k1::verification_only();
    nonce_agg_with(&secp, pubnonces)
}

/// The same aggregation, with a caller's secp context.
pub fn nonce_agg_with<C: Verification>(
    secp: &Secp256k1<C>,
    pubnonces: &[PubNonce],
) -> Result<AggNonce, Error> {
    musig::nonce_agg_with(secp, pubnonces).map_err(|_| Error::NoSigners)
}

// ---------------------------------------------------------------------
// The session
// ---------------------------------------------------------------------

/// Everything a signer needs to know about the session it signs in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    /// The participant count of the group.
    pub n: usize,
    /// The threshold.
    pub t: usize,
    /// The identifiers of the signers, in the order their nonces and
    /// shares are listed.
    pub ids: Vec<u32>,
    /// The signers' public shares, when the session knows them. Signing
    /// and aggregation work without them; verification does not.
    pub pubshares: Option<Vec<PublicKey>>,
    /// The group key, before any tweak.
    pub thresh_pk: PublicKey,
    /// The aggregate of every signer's public nonce.
    pub aggnonce: AggNonce,
    /// The tweaks, in the order they apply.
    pub tweaks: Vec<Tweak>,
    /// The message, which for a spend is the sighash.
    pub msg: Vec<u8>,
}

/// BIP 445's `GetSessionValues`: the tweaked group key, the binding
/// value `b`, the final nonce `R` and the challenge `e`.
///
/// Every value here is public; the secret of a session is the
/// [`SecNonce`], which this does not hold.
#[derive(Debug, Clone)]
pub struct SessionValues {
    /// The tweaked group key `Q` with its accumulators, whose x-only
    /// form is the key a BIP-340 verifier checks the finished signature
    /// against.
    pub aggregate: AggregateKey,
    /// The binding value `b`.
    pub b: [u8; 32],
    /// The final nonce `R`.
    pub r: PublicKey,
    /// The challenge `e`.
    pub e: [u8; 32],
    /// The signers' identifiers, in the order the session gave them.
    pub ids: Vec<u32>,
    /// The signers' public shares, when the session knows them.
    pub pubshares: Option<Vec<PublicKey>>,
}

/// BIP 445's `ThreshPubkeyAndTweak`: the group key with the tweaks
/// applied in order, carrying `gacc` and `tacc` forward.
pub fn thresh_pubkey_and_tweak<C: Verification>(
    secp: &Secp256k1<C>,
    thresh_pk: &PublicKey,
    tweaks: &[Tweak],
) -> Result<AggregateKey, Error> {
    let mut state = AggregateKey::untweaked(*thresh_pk);
    for tweak in tweaks {
        state = state
            .apply_tweak(secp, &tweak.bytes, tweak.x_only)
            .map_err(tweak_error)?;
    }
    Ok(state)
}

/// BIP 445's `ValidateSessionParams`.
fn validate_session_params<C: Verification>(
    secp: &Secp256k1<C>,
    n: usize,
    t: usize,
    ids: &[u32],
    pubshares: Option<&[PublicKey]>,
    thresh_pk: &PublicKey,
) -> Result<(), Error> {
    if t < 1 || t > n {
        return Err(Error::ThresholdRange);
    }
    if n > MAX_PARTICIPANTS {
        return Err(Error::TooManyParticipants);
    }
    if ids.len() < t || ids.len() > n {
        return Err(Error::SignerCount);
    }
    if let Some(shares) = pubshares
        && shares.len() != ids.len()
    {
        return Err(Error::PubShareCount);
    }
    for (index, &id) in ids.iter().enumerate() {
        if id as usize >= n {
            return Err(Error::IdOutOfRange { index });
        }
    }
    if has_duplicates(ids) {
        return Err(Error::DuplicateIds);
    }
    if let Some(shares) = pubshares
        && derive_thresh_pubkey(secp, ids, shares)? != *thresh_pk
    {
        return Err(Error::ThreshPkMismatch);
    }
    Ok(())
}

/// BIP 445's `GetSessionValues` over a session context.
pub fn session_values<C: Verification>(
    secp: &Secp256k1<C>,
    ctx: &SessionContext,
) -> Result<SessionValues, Error> {
    validate_session_params(
        secp,
        ctx.n,
        ctx.t,
        &ctx.ids,
        ctx.pubshares.as_deref(),
        &ctx.thresh_pk,
    )?;
    let aggregate = thresh_pubkey_and_tweak(secp, &ctx.thresh_pk, &ctx.tweaks)?;
    let q = aggregate.serialize_x_only();
    let aggnonce = ctx.aggnonce.serialize();
    let b = reduce(tagged_hash(
        TAG_NONCECOEF,
        &[
            &(ctx.ids.len() as u32).to_be_bytes(),
            &serialize_ids(&ctx.ids),
            &aggnonce,
            &q,
            &ctx.msg,
        ],
    ));
    let r1 = cpoint_ext(&aggnonce[..33]).map_err(|()| Error::InvalidContribution {
        signer: None,
        contrib: Contrib::AggNonce,
    })?;
    let r2 = cpoint_ext(&aggnonce[33..]).map_err(|()| Error::InvalidContribution {
        signer: None,
        contrib: Contrib::AggNonce,
    })?;
    let scaled = point_mul(secp, r2, &b).map_err(|_| Error::Infinity)?;
    let r = match point_add(secp, r1, scaled).map_err(|_| Error::Infinity)? {
        Some(r) => r,
        // An aggregate nonce of infinity signs under the generator, so
        // that the session still has a nonce.
        None => generator(),
    };
    let e = reduce(tagged_hash(
        TAG_CHALLENGE,
        &[&r.x_only_public_key().0.serialize(), &q, &ctx.msg],
    ));
    Ok(SessionValues {
        aggregate,
        b,
        r,
        e,
        ids: ctx.ids.clone(),
        pubshares: ctx.pubshares.clone(),
    })
}

// ---------------------------------------------------------------------
// Signing
// ---------------------------------------------------------------------

/// BIP 445's `Sign`. The nonce is consumed, so it cannot sign twice, and
/// the partial signature is verified before it is returned.
pub fn sign<C: Verification>(
    secp: &Secp256k1<C>,
    secnonce: SecNonce,
    secshare: &SecShare,
    my_id: u32,
    ctx: &SessionContext,
) -> Result<PartialSig, Error> {
    let session = session_values(secp, ctx)?;
    let (k1_, k2_) = (secnonce.k1, secnonce.k2);
    drop(secnonce);
    let even = session.r.x_only_public_key().1 == Parity::Even;
    let (k1, k2) = if even {
        (k1_, k2_)
    } else {
        (negate_mod_order(&k1_), negate_mod_order(&k2_))
    };
    let my_pubshare = secshare.public_share(secp);
    if !session.ids.contains(&my_id) {
        return Err(Error::NotASigner);
    }
    if let Some(shares) = &session.pubshares {
        let position = session
            .ids
            .iter()
            .position(|id| *id == my_id)
            .expect("the identifier is in the list");
        if shares[position] != my_pubshare {
            return Err(Error::PubShareMismatch);
        }
    }
    let a = derive_interpolating_value(&session.ids, my_id)?;
    let g = parity_factor(&session.aggregate.public_key());
    let d = mul_mod_order(
        &mul_mod_order(&g, &session.aggregate.gacc()),
        &secshare.secret_bytes(),
    );
    let s = add_mod_order(
        &add_mod_order(&k1, &mul_mod_order(&session.b, &k2)),
        &mul_mod_order(&mul_mod_order(&session.e, &a), &d),
    );
    let psig = PartialSig::from_bytes(&s).expect("a reduced scalar is below the order");
    let pubnonce = pubnonce_of(secp, &k1_, &k2_)?;
    partial_sig_verify_internal(secp, &psig, my_id, &pubnonce, &my_pubshare, &session)?;
    Ok(psig)
}

/// BIP 445's `PartialSigVerifyInternal`: the session values are already
/// in hand.
pub fn partial_sig_verify_internal<C: Verification>(
    secp: &Secp256k1<C>,
    psig: &PartialSig,
    my_id: u32,
    pubnonce: &PubNonce,
    pubshare: &PublicKey,
    session: &SessionValues,
) -> Result<(), Error> {
    let bytes = pubnonce.serialize();
    let r1 = PublicKey::from_slice(&bytes[..33]).map_err(|_| Error::InvalidContribution {
        signer: None,
        contrib: Contrib::PubNonce,
    })?;
    let r2 = PublicKey::from_slice(&bytes[33..]).map_err(|_| Error::InvalidContribution {
        signer: None,
        contrib: Contrib::PubNonce,
    })?;
    let scaled = point_mul(secp, Some(r2), &session.b).map_err(|_| Error::Infinity)?;
    let effective = point_add(secp, Some(r1), scaled).map_err(|_| Error::Infinity)?;
    let effective = if session.r.x_only_public_key().1 == Parity::Even {
        effective
    } else {
        point_negate(secp, effective)
    };
    let a = derive_interpolating_value(&session.ids, my_id)?;
    let g = parity_factor(&session.aggregate.public_key());
    let gg = mul_mod_order(&g, &session.aggregate.gacc());
    let scalar = mul_mod_order(&mul_mod_order(&session.e, &a), &gg);
    let term = point_mul(secp, Some(*pubshare), &scalar).map_err(|_| Error::Infinity)?;
    let right = point_add(secp, effective, term).map_err(|_| Error::Infinity)?;
    let left = base_mul(secp, &psig.serialize()).map_err(|_| Error::Infinity)?;
    if left == right {
        Ok(())
    } else {
        Err(Error::VerifyFailed)
    }
}

/// BIP 445's `PartialSigVerify`: the whole check, from the nonces and
/// public shares of the session. The signer is given by its position in
/// `ids`, and the public share list is required.
#[allow(clippy::too_many_arguments)]
pub fn partial_sig_verify<C: Verification>(
    secp: &Secp256k1<C>,
    psig: &PartialSig,
    pubnonces: &[PubNonce],
    n: usize,
    t: usize,
    ids: &[u32],
    pubshares: &[PublicKey],
    thresh_pk: &PublicKey,
    tweaks: &[Tweak],
    msg: &[u8],
    index: usize,
) -> Result<(), Error> {
    if pubnonces.len() != ids.len() || pubshares.len() != ids.len() {
        return Err(Error::VerifyInputCount);
    }
    if index >= ids.len() {
        return Err(Error::SignerIndexOutOfRange);
    }
    let ctx = SessionContext {
        n,
        t,
        ids: ids.to_vec(),
        pubshares: Some(pubshares.to_vec()),
        thresh_pk: *thresh_pk,
        aggnonce: nonce_agg_with(secp, pubnonces)?,
        tweaks: tweaks.to_vec(),
        msg: msg.to_vec(),
    };
    let session = session_values(secp, &ctx)?;
    partial_sig_verify_internal(
        secp,
        psig,
        ids[index],
        &pubnonces[index],
        &pubshares[index],
        &session,
    )
}

/// BIP 445's `PartialSigAgg`: the 64-byte BIP-340 signature.
pub fn partial_sig_agg(psigs: &[PartialSig], session: &SessionValues) -> Result<[u8; 64], Error> {
    if psigs.len() != session.ids.len() {
        return Err(Error::PartialSigCount);
    }
    let g = parity_factor(&session.aggregate.public_key());
    let mut s = mul_mod_order(&mul_mod_order(&session.e, &g), &session.aggregate.tacc());
    for psig in psigs {
        s = add_mod_order(&s, &psig.serialize());
    }
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&session.r.x_only_public_key().0.serialize());
    out[32..].copy_from_slice(&s);
    Ok(out)
}

/// BIP 445's `DeterministicSign`: the nonce comes from the secret share
/// and the other signers' aggregate nonce, so the signer keeps nothing
/// between the rounds. A sole signer passes no other nonce, and its own
/// public nonce is the session's aggregate.
#[allow(clippy::too_many_arguments)]
pub fn deterministic_sign<C: Verification>(
    secp: &Secp256k1<C>,
    secshare: &SecShare,
    my_id: u32,
    aggothernonce: Option<&AggNonce>,
    n: usize,
    t: usize,
    ids: &[u32],
    pubshares: Option<&[PublicKey]>,
    thresh_pk: &PublicKey,
    tweaks: &[Tweak],
    msg: &[u8],
    aux_rand: Option<&[u8; 32]>,
) -> Result<(PubNonce, PartialSig), Error> {
    validate_session_params(secp, n, t, ids, pubshares, thresh_pk)?;
    let mut seed = secshare.secret_bytes();
    if let Some(rand) = aux_rand {
        let masked = tagged_hash(TAG_AUX, &[rand]);
        for (s, m) in seed.iter_mut().zip(masked) {
            *s ^= m;
        }
    }
    let tweaked = thresh_pubkey_and_tweak(secp, thresh_pk, tweaks)?.serialize_x_only();
    let other = aggothernonce.map(AggNonce::serialize);
    let other = other.as_ref().map_or(&[][..], |a| &a[..]);
    let mut k = [[0u8; 32]; 2];
    for (i, k) in k.iter_mut().enumerate() {
        *k = reduce(tagged_hash(
            TAG_DETERMINISTIC_NONCE,
            &[
                &seed,
                &my_id.to_be_bytes(),
                &(ids.len() as u32).to_be_bytes(),
                &serialize_ids(ids),
                other,
                &tweaked,
                &(msg.len() as u64).to_be_bytes(),
                msg,
                &[i as u8],
            ],
        ));
        if *k == [0u8; 32] {
            return Err(Error::FirstSecNonceOutOfRange);
        }
    }
    seed.zeroize();
    let pubnonce = pubnonce_of(secp, &k[0], &k[1])?;
    // `NonceAgg` is defined over public nonces, so a half of
    // `aggothernonce` that is the extended encoding of infinity is
    // refused here, even though an aggregate nonce may carry one.
    let aggnonce = match aggothernonce {
        None => AggNonce::from_bytes(&pubnonce.serialize()).map_err(|_| Error::Infinity)?,
        Some(other) => {
            let (mine, theirs) = (pubnonce.serialize(), other.serialize());
            let mut out = [0u8; 66];
            for half in 0..2 {
                let range = half * 33..half * 33 + 33;
                let mine =
                    PublicKey::from_slice(&mine[range.clone()]).map_err(|_| Error::Infinity)?;
                let theirs = PublicKey::from_slice(&theirs[range.clone()]).map_err(|_| {
                    Error::InvalidContribution {
                        signer: None,
                        contrib: Contrib::AggOtherNonce,
                    }
                })?;
                let sum = point_add(secp, Some(mine), Some(theirs)).map_err(|_| Error::Infinity)?;
                out[range].copy_from_slice(&cbytes_ext(sum));
            }
            AggNonce::from_bytes(&out).map_err(|_| Error::Infinity)?
        }
    };
    let ctx = SessionContext {
        n,
        t,
        ids: ids.to_vec(),
        pubshares: pubshares.map(<[PublicKey]>::to_vec),
        thresh_pk: *thresh_pk,
        aggnonce,
        tweaks: tweaks.to_vec(),
        msg: msg.to_vec(),
    };
    let psig = sign(secp, SecNonce { k1: k[0], k2: k[1] }, secshare, my_id, &ctx)?;
    Ok((pubnonce, psig))
}

// ---------------------------------------------------------------------
// The trusted dealer (§16.103)
// ---------------------------------------------------------------------

/// What the dealer hands out: every share by identifier, and the public
/// record.
///
/// The shares are secrets; the record is not.
#[derive(Debug)]
pub struct Dealt {
    /// The shares, entry `i` belonging to identifier `i`.
    pub shares: Vec<SecShare>,
    /// The public record of the group.
    pub info: ThresholdInfo,
}

fn check_held(n: Option<usize>, t: usize, ids: &[u32]) -> Result<(), Error> {
    if !(1..=MAX_PARTICIPANTS).contains(&t) {
        return Err(Error::ThresholdRange);
    }
    if ids.len() != t {
        return Err(Error::ChosenCount);
    }
    if has_duplicates(ids) {
        return Err(Error::ChosenDuplicate);
    }
    if let Some(n) = n
        && ids.iter().any(|id| *id as usize >= n)
    {
        return Err(Error::ChosenIdOutOfRange);
    }
    Ok(())
}

/// §16.103's key generation: `t` shares chosen freely, the rest
/// computed.
///
/// The polynomial of degree `t - 1` through the chosen shares fixes
/// every other share and the group key. The group secret is never
/// returned: the key comes from the public shares.
pub fn deal<C: Verification>(
    secp: &Secp256k1<C>,
    n: usize,
    t: usize,
    chosen: &[(u32, SecShare)],
) -> Result<Dealt, Error> {
    if t < 1 || t > n {
        return Err(Error::ThresholdRange);
    }
    if n > MAX_PARTICIPANTS {
        return Err(Error::TooManyParticipants);
    }
    let ids: Vec<u32> = chosen.iter().map(|(id, _)| *id).collect();
    check_held(Some(n), t, &ids)?;
    let mut values: Vec<[u8; 32]> = chosen.iter().map(|(_, s)| s.secret_bytes()).collect();
    let mut shares = Vec::with_capacity(n);
    for target in 0..n {
        let target = target as u32;
        let share = match ids.iter().position(|id| *id == target) {
            Some(position) => values[position],
            None => interpolate_secret_at(&ids, &values, i64::from(target)),
        };
        shares.push(SecShare::from_bytes(&share).map_err(|_| Error::ZeroShare)?);
    }
    for value in &mut values {
        value.zeroize();
    }
    let pubshares: Vec<PublicKey> = shares.iter().map(|s| s.public_share(secp)).collect();
    let all: Vec<u32> = (0..n as u32).collect();
    let thresh_pk = derive_thresh_pubkey(secp, &all, &pubshares)?;
    Ok(Dealt {
        shares,
        info: ThresholdInfo {
            t,
            thresh_pk,
            pubshares: pubshares.into_iter().map(Some).collect(),
        },
    })
}

/// The share of `target`, rebuilt from any `t` held shares.
pub fn recover_share(t: usize, held: &[(u32, &SecShare)], target: u32) -> Result<SecShare, Error> {
    let ids: Vec<u32> = held.iter().map(|(id, _)| *id).collect();
    check_held(None, t, &ids)?;
    if let Some(position) = ids.iter().position(|id| *id == target) {
        return SecShare::from_bytes(&held[position].1.secret_bytes());
    }
    let mut values: Vec<[u8; 32]> = held.iter().map(|(_, s)| s.secret_bytes()).collect();
    let mut share = interpolate_secret_at(&ids, &values, i64::from(target));
    for value in &mut values {
        value.zeroize();
    }
    let recovered = SecShare::from_bytes(&share).map_err(|_| Error::ZeroShare);
    share.zeroize();
    recovered
}

/// The public record, rebuilt from any `t` held shares: every public
/// share and the group key, by public arithmetic from the held shares'
/// own public shares.
pub fn recover_info<C: Verification>(
    secp: &Secp256k1<C>,
    n: usize,
    t: usize,
    held: &[(u32, &SecShare)],
) -> Result<ThresholdInfo, Error> {
    if t < 1 || t > n {
        return Err(Error::ThresholdRange);
    }
    if n > MAX_PARTICIPANTS {
        return Err(Error::TooManyParticipants);
    }
    let ids: Vec<u32> = held.iter().map(|(id, _)| *id).collect();
    check_held(Some(n), t, &ids)?;
    let known: Vec<PublicKey> = held.iter().map(|(_, s)| s.public_share(secp)).collect();
    let mut pubshares = Vec::with_capacity(n);
    for target in 0..n as u32 {
        let point = match ids.iter().position(|id| *id == target) {
            Some(position) => known[position],
            None => derive_pubshare_at(secp, &ids, &known, i64::from(target))?
                .ok_or(Error::ZeroShare)?,
        };
        pubshares.push(Some(point));
    }
    Ok(ThresholdInfo {
        t,
        thresh_pk: derive_thresh_pubkey(secp, &ids, &known)?,
        pubshares,
    })
}
