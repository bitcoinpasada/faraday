//! MuSig2 key aggregation (BIP-327), the synthetic aggregate extended
//! public key (BIP-328), and the `musig()` key expression (BIP-390).
//!
//! Four layers, each usable on its own:
//!
//! - [`key_agg`] is BIP-327's `KeyAgg`, over plain compressed public keys
//!   in the order given. [`sort_keys`] is `KeySort`. The result is an
//!   [`AggregateKey`], which carries the accumulators `gacc` and `tacc`
//!   so that [`AggregateKey::apply_tweak`] is BIP-327's `ApplyTweak` and
//!   a signer can later pick the state up where this leaves it.
//! - [`aggregate_xpub`] is BIP-328: the aggregate point published as an
//!   extended public key with the fixed chain code, so that ordinary
//!   unhardened BIP-32 derivation gives the per-address keys.
//! - [`parse_musig_expr`], [`parse_tr_musig`] and [`script_pubkey_at`]
//!   are BIP-390: the `musig(KEY,…)` expression, with or without a
//!   derivation of its own, inside `tr()`.
//! - [`nonce_gen`], [`nonce_agg`], [`session_values`], [`sign`],
//!   [`partial_sig_verify`], [`partial_sig_agg`] and
//!   [`deterministic_sign`] are BIP-327's signing side, written over the
//!   curve primitives the same way. A [`SecNonce`] is not `Clone`, is
//!   wiped on drop and is consumed by [`sign`], so one nonce cannot sign
//!   twice.
//!
//! The keys inside a `musig()` are sorted with `KeySort` after
//! derivation and before aggregation, which BIP-390 requires; [`key_agg`]
//! itself sorts nothing, because BIP-327's own test vectors depend on the
//! order they are given in.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use bitcoin::bip32::{ChainCode, ChildNumber, DerivationPath, Fingerprint, Xpub};
use bitcoin::secp256k1::{Parity, PublicKey, Scalar, Secp256k1, Verification, XOnlyPublicKey};
use bitcoin::{NetworkKind, ScriptBuf};
use osk_crypto::sha256;

use crate::keys::Network;

/// The order of the secp256k1 group, big-endian.
pub(crate) const CURVE_ORDER: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36, 0x41, 0x41,
];

/// The chain code BIP-328 gives every aggregate extended public key:
/// `sha256("MuSig2 deterministic chaincode")` as the BIP publishes it.
pub const AGGREGATE_CHAIN_CODE: [u8; 32] = [
    0x86, 0x80, 0x87, 0xca, 0x02, 0xa6, 0xf9, 0x74, 0xc4, 0x59, 0x89, 0x24, 0xc3, 0x6b, 0x57, 0x76,
    0x2d, 0x32, 0xcb, 0x45, 0x71, 0x71, 0x67, 0xe3, 0x00, 0x62, 0x2c, 0x71, 0x67, 0xe3, 0x89, 0x65,
];

/// Why a key could not be aggregated, or a `musig()` expression read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// `KeyAgg` was given no keys.
    NoKeys,
    /// The aggregate point, or the result of a tweak, is the point at
    /// infinity, which is no public key.
    Infinity,
    /// A tweak is not a scalar below the curve order.
    TweakOutOfRange,
    /// The expression is not `musig(…)`, or it sits somewhere BIP-390
    /// does not allow a `musig()` — anything but `tr()` here — or one
    /// `musig()` is nested in another.
    Expression,
    /// A participant key is not a compressed public key or an extended
    /// public key, with an optional `[fingerprint/path]` origin.
    Key,
    /// A derivation step is hardened, a wildcard stands somewhere other
    /// than last, more than one step is multipath, or a participant
    /// carries a derivation of its own while the `musig()` also has one.
    Derivation,
    /// An address index is 2³¹ or larger.
    IndexOutOfRange,
    /// A public nonce is not two compressed points.
    PubNonce,
    /// An aggregate nonce is not two points or extended-encoded
    /// infinities.
    AggNonce,
    /// A secret nonce is malformed, zero, at or above the curve order,
    /// or was drawn for another public key.
    SecNonce,
    /// A partial signature is not a scalar below the curve order.
    PartialSig,
    /// The signer's public key is not one of the session's.
    NotAParticipant,
    /// A partial signature is not the one this signer's nonce and key
    /// make over this message.
    VerifyFailed,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::NoKeys => "key aggregation needs at least one key",
            Error::Infinity => "the aggregate key is the point at infinity",
            Error::TweakOutOfRange => "the tweak must be less than the curve order",
            Error::Expression => "not a musig() expression a tr() descriptor can hold",
            Error::Key => "participant key is not a public key or an extended public key",
            Error::Derivation => "derivation must be unhardened, with the wildcard last",
            Error::IndexOutOfRange => "index must be below 2^31",
            Error::PubNonce => "public nonce is not two compressed points",
            Error::AggNonce => "aggregate nonce is not two points",
            Error::SecNonce => "secret nonce is not a usable pair of scalars for this key",
            Error::PartialSig => "partial signature must be less than the curve order",
            Error::NotAParticipant => "the signer's key is not one of the session's keys",
            Error::VerifyFailed => "partial signature does not verify",
        })
    }
}

impl core::error::Error for Error {}

// ---------------------------------------------------------------------
// Scalars mod n
// ---------------------------------------------------------------------

fn limbs(a: &[u8; 32]) -> [u64; 4] {
    let mut out = [0u64; 4];
    for (i, l) in out.iter_mut().enumerate() {
        *l = u64::from_be_bytes(a[i * 8..i * 8 + 8].try_into().expect("eight bytes"));
    }
    out
}

fn bytes(l: &[u64; 4]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, limb) in l.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&limb.to_be_bytes());
    }
    out
}

/// Whether `a` is at or above the curve order.
pub(crate) fn at_least_order(a: &[u8; 32]) -> bool {
    *a >= CURVE_ORDER
}

/// `a - n`, which the callers only ask for when `a >= n`.
fn subtract_order(a: &[u8; 32]) -> [u8; 32] {
    let (a, n) = (limbs(a), limbs(&CURVE_ORDER));
    let mut out = [0u64; 4];
    let mut borrow = 0u64;
    for i in (0..4).rev() {
        let (d, b1) = a[i].overflowing_sub(n[i]);
        let (d, b2) = d.overflowing_sub(borrow);
        out[i] = d;
        borrow = u64::from(b1 || b2);
    }
    bytes(&out)
}

/// `a mod n` for a 256-bit `a`. One subtraction is enough: `n` is above
/// `2²⁵⁵`, so `a - n < n` whenever `a < 2²⁵⁶`.
pub(crate) fn reduce(a: [u8; 32]) -> [u8; 32] {
    if at_least_order(&a) {
        subtract_order(&a)
    } else {
        a
    }
}

/// `(a + b) mod n`, for `a` and `b` already reduced.
pub(crate) fn add_mod_order(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let (a, b) = (limbs(a), limbs(b));
    let mut sum = [0u64; 4];
    let mut carry = 0u64;
    for i in (0..4).rev() {
        let (s, c1) = a[i].overflowing_add(b[i]);
        let (s, c2) = s.overflowing_add(carry);
        sum[i] = s;
        carry = u64::from(c1 || c2);
    }
    let sum = bytes(&sum);
    if carry == 1 || at_least_order(&sum) {
        subtract_order(&sum)
    } else {
        sum
    }
}

/// `-a mod n`.
pub(crate) fn negate_mod_order(a: &[u8; 32]) -> [u8; 32] {
    if *a == [0u8; 32] {
        return [0u8; 32];
    }
    let (n, a) = (limbs(&CURVE_ORDER), limbs(a));
    let mut out = [0u64; 4];
    let mut borrow = 0u64;
    for i in (0..4).rev() {
        let (d, b1) = n[i].overflowing_sub(a[i]);
        let (d, b2) = d.overflowing_sub(borrow);
        out[i] = d;
        borrow = u64::from(b1 || b2);
    }
    bytes(&out)
}

pub(crate) fn one() -> [u8; 32] {
    let mut v = [0u8; 32];
    v[31] = 1;
    v
}

// ---------------------------------------------------------------------
// Tagged hashes
// ---------------------------------------------------------------------

/// BIP-340's `hash_tag(m) = sha256(sha256(tag) ‖ sha256(tag) ‖ m)`.
pub(crate) fn tagged_hash(tag: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let prefix = sha256(tag);
    let mut data = Vec::with_capacity(64 + parts.iter().map(|p| p.len()).sum::<usize>());
    data.extend_from_slice(&prefix);
    data.extend_from_slice(&prefix);
    for part in parts {
        data.extend_from_slice(part);
    }
    sha256(&data)
}

// ---------------------------------------------------------------------
// BIP-327 KeyAgg
// ---------------------------------------------------------------------

/// An aggregate public key and the state a tweak carries forward.
///
/// `gacc` and `tacc` are BIP-327's accumulators: the product of the
/// negation factors every tweak applied, and the sum of the tweaks, both
/// mod `n`. A signer needs them; a wallet that only derives addresses
/// does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateKey {
    point: PublicKey,
    gacc: [u8; 32],
    tacc: [u8; 32],
}

impl AggregateKey {
    /// The tweak state of a key no aggregation stands behind: `gacc` 1
    /// and `tacc` 0, so that [`AggregateKey::apply_tweak`] tweaks the key
    /// itself. BIP 445's `TweakCtxInit` over a FROST group key is this.
    pub fn untweaked(point: PublicKey) -> AggregateKey {
        AggregateKey {
            point,
            gacc: one(),
            tacc: [0u8; 32],
        }
    }

    /// The aggregate point, `Q`.
    pub fn public_key(&self) -> PublicKey {
        self.point
    }

    /// `Q` as an x-only key with the parity of the point it came from.
    pub fn x_only(&self) -> (XOnlyPublicKey, Parity) {
        self.point.x_only_public_key()
    }

    /// The 32 bytes a BIP-340 verifier reads as the aggregate key.
    pub fn serialize_x_only(&self) -> [u8; 32] {
        self.x_only().0.serialize()
    }

    /// BIP-327's `gacc`.
    pub fn gacc(&self) -> [u8; 32] {
        self.gacc
    }

    /// BIP-327's `tacc`.
    pub fn tacc(&self) -> [u8; 32] {
        self.tacc
    }

    /// BIP-327's `ApplyTweak`. `x_only` selects the x-only form, which
    /// negates `Q` first when `Q` has an odd y — the form BIP-341's
    /// taproot tweak needs; the plain form is `false`.
    pub fn apply_tweak<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        tweak: &[u8; 32],
        x_only: bool,
    ) -> Result<AggregateKey, Error> {
        if at_least_order(tweak) {
            return Err(Error::TweakOutOfRange);
        }
        let negate = x_only && self.point.x_only_public_key().1 == Parity::Odd;
        let base = if negate {
            self.point.negate(secp)
        } else {
            self.point
        };
        let point = if *tweak == [0u8; 32] {
            base
        } else {
            let scalar = Scalar::from_be_bytes(*tweak).map_err(|_| Error::TweakOutOfRange)?;
            base.add_exp_tweak(secp, &scalar)
                .map_err(|_| Error::Infinity)?
        };
        let (gacc, tacc) = if negate {
            (negate_mod_order(&self.gacc), negate_mod_order(&self.tacc))
        } else {
            (self.gacc, self.tacc)
        };
        Ok(AggregateKey {
            point,
            gacc,
            tacc: add_mod_order(tweak, &tacc),
        })
    }
}

/// BIP-327's `KeySort`: the keys in ascending order of their compressed
/// serialisation.
pub fn sort_keys(pubkeys: &[PublicKey]) -> Vec<PublicKey> {
    let mut sorted = pubkeys.to_vec();
    sorted.sort_by_key(|k| k.serialize());
    sorted
}

/// BIP-327's `HashKeys`: the tagged hash of every key, concatenated.
fn hash_keys(pubkeys: &[PublicKey]) -> [u8; 32] {
    let mut data = Vec::with_capacity(33 * pubkeys.len());
    for key in pubkeys {
        data.extend_from_slice(&key.serialize());
    }
    tagged_hash(b"KeyAgg list", &[&data])
}

/// BIP-327's `GetSecondKey`: the first key that differs from the first
/// one, or none when every key is the same.
fn second_key(pubkeys: &[PublicKey]) -> Option<PublicKey> {
    let first = pubkeys.first()?;
    pubkeys[1..].iter().find(|k| *k != first).copied()
}

/// BIP-327's `KeyAggCoeff`: 1 for the second distinct key, and the
/// tagged hash of the key list and the key otherwise.
fn key_agg_coeff(list: &[u8; 32], key: &PublicKey, second: Option<PublicKey>) -> [u8; 32] {
    if second == Some(*key) {
        return one();
    }
    reduce(tagged_hash(
        b"KeyAgg coefficient",
        &[list, &key.serialize()],
    ))
}

/// BIP-327's `KeyAgg`, over the keys in the order given.
///
/// Nothing is sorted here. A caller that wants the order not to matter —
/// a BIP-390 `musig()` does — passes [`sort_keys`]'s result.
pub fn key_agg(pubkeys: &[PublicKey]) -> Result<AggregateKey, Error> {
    let secp = Secp256k1::verification_only();
    key_agg_with(&secp, pubkeys)
}

/// The same aggregation, with a caller's secp context.
pub fn key_agg_with<C: Verification>(
    secp: &Secp256k1<C>,
    pubkeys: &[PublicKey],
) -> Result<AggregateKey, Error> {
    if pubkeys.is_empty() {
        return Err(Error::NoKeys);
    }
    let list = hash_keys(pubkeys);
    let second = second_key(pubkeys);
    let mut terms = Vec::with_capacity(pubkeys.len());
    for key in pubkeys {
        let coeff = key_agg_coeff(&list, key, second);
        let scalar = Scalar::from_be_bytes(coeff).map_err(|_| Error::Infinity)?;
        terms.push(key.mul_tweak(secp, &scalar).map_err(|_| Error::Infinity)?);
    }
    let refs: Vec<&PublicKey> = terms.iter().collect();
    let point = if refs.len() == 1 {
        *refs[0]
    } else {
        PublicKey::combine_keys(&refs).map_err(|_| Error::Infinity)?
    };
    Ok(AggregateKey {
        point,
        gacc: one(),
        tacc: [0u8; 32],
    })
}

// ---------------------------------------------------------------------
// BIP-328: the synthetic extended public key
// ---------------------------------------------------------------------

/// The aggregate of `pubkeys` as an extended public key: depth 0, no
/// parent, index 0, and BIP-328's fixed chain code.
///
/// The keys are aggregated in the order given, so a BIP-390 caller sorts
/// first. Unhardened BIP-32 derivation from the result gives the
/// per-address keys.
pub fn aggregate_xpub(pubkeys: &[PublicKey], network: Network) -> Result<Xpub, Error> {
    Ok(synthetic_xpub(
        key_agg(pubkeys)?.public_key(),
        network.kind(),
    ))
}

/// One plain public key as an extended public key of the same shape:
/// depth 0, no parent, index 0, and BIP-328's fixed chain code.
///
/// BIP-328 gives a MuSig2 aggregate key this form so that a wallet can
/// derive addresses from it; a FROST group key is a plain key in the
/// same position and takes the same construction
/// (`docs/PLANNING.md` §16.103).
pub fn synthetic_xpub(public_key: PublicKey, network: NetworkKind) -> Xpub {
    Xpub {
        network,
        depth: 0,
        parent_fingerprint: Fingerprint::default(),
        child_number: ChildNumber::from_normal_idx(0).expect("zero is a normal index"),
        public_key,
        chain_code: ChainCode::from(AGGREGATE_CHAIN_CODE),
    }
}

// ---------------------------------------------------------------------
// BIP-390: the musig() key expression
// ---------------------------------------------------------------------

/// One step of a descriptor derivation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    /// A fixed unhardened index.
    Fixed(u32),
    /// BIP-389's `<a;b>`: the receive chain then the change chain.
    Multi(u32, u32),
}

/// A derivation path as a descriptor writes it: fixed and multipath
/// steps, and whether it ends in `/*`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Path {
    steps: Vec<Step>,
    wildcard: bool,
}

impl Path {
    fn is_empty(&self) -> bool {
        self.steps.is_empty() && !self.wildcard
    }

    fn is_ranged(&self) -> bool {
        self.wildcard || self.steps.iter().any(|s| matches!(s, Step::Multi(..)))
    }

    /// The concrete BIP-32 steps at `change`/`index`.
    fn at(&self, change: bool, index: u32) -> Result<Vec<ChildNumber>, Error> {
        let mut out = Vec::with_capacity(self.steps.len() + 1);
        for step in &self.steps {
            let n = match *step {
                Step::Fixed(n) => n,
                Step::Multi(a, b) => {
                    if change {
                        b
                    } else {
                        a
                    }
                }
            };
            out.push(ChildNumber::from_normal_idx(n).map_err(|_| Error::IndexOutOfRange)?);
        }
        if self.wildcard {
            out.push(ChildNumber::from_normal_idx(index).map_err(|_| Error::IndexOutOfRange)?);
        }
        Ok(out)
    }
}

/// A participant's key: a plain compressed public key, or an extended
/// public key that the participant's own path derives from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ParticipantKey {
    Plain(PublicKey),
    Extended(Xpub),
}

/// One key inside a `musig()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    origin: Option<String>,
    key: ParticipantKey,
    path: Path,
}

impl Participant {
    /// The `[fingerprint/path]` origin, if the expression gave one.
    pub fn origin(&self) -> Option<&str> {
        self.origin.as_deref()
    }

    /// The participant's extended public key, when it has one.
    pub fn xpub(&self) -> Option<&Xpub> {
        match &self.key {
            ParticipantKey::Extended(xpub) => Some(xpub),
            ParticipantKey::Plain(_) => None,
        }
    }

    /// The participant's plain public key, when it has one.
    pub fn plain_key(&self) -> Option<PublicKey> {
        match &self.key {
            ParticipantKey::Plain(key) => Some(*key),
            ParticipantKey::Extended(_) => None,
        }
    }

    fn key_at<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        change: bool,
        index: u32,
    ) -> Result<PublicKey, Error> {
        match &self.key {
            ParticipantKey::Plain(key) => Ok(*key),
            ParticipantKey::Extended(xpub) => {
                let steps = self.path.at(change, index)?;
                Ok(xpub
                    .derive_pub(secp, &steps)
                    .map_err(|_| Error::Derivation)?
                    .public_key)
            }
        }
    }
}

/// A parsed `musig(KEY,…)` expression, with the derivation that follows
/// it when there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusigExpr {
    participants: Vec<Participant>,
    path: Path,
}

impl MusigExpr {
    /// `musig(xpub,…)/<0;1>/*`: the expression a wallet of `xpubs` is,
    /// with the two chains derived from the aggregate key and no
    /// participant carrying a derivation of its own.
    ///
    /// The origins the wallet states are not part of what this derives,
    /// so they are not carried here.
    pub fn from_xpubs(xpubs: &[Xpub]) -> Result<MusigExpr, Error> {
        if xpubs.len() < 2 {
            return Err(Error::Expression);
        }
        Ok(MusigExpr {
            participants: xpubs
                .iter()
                .map(|xpub| Participant {
                    origin: None,
                    key: ParticipantKey::Extended(*xpub),
                    path: Path::default(),
                })
                .collect(),
            path: Path {
                steps: alloc::vec![Step::Multi(0, 1)],
                wildcard: true,
            },
        })
    }

    /// The participants, in the order the expression wrote them.
    pub fn participants(&self) -> &[Participant] {
        &self.participants
    }

    /// Whether the expression derives from the aggregate key, which is
    /// BIP-328 derivation and needs every participant to be an xpub.
    pub fn derives_from_aggregate(&self) -> bool {
        !self.path.is_empty()
    }

    /// Whether the expression names one key or a range of them.
    pub fn is_ranged(&self) -> bool {
        self.path.is_ranged() || self.participants.iter().any(|p| p.path.is_ranged())
    }

    /// The aggregate key of the participants at `change`/`index`, before
    /// any derivation from the aggregate. The participants are sorted
    /// with `KeySort` first, which BIP-390 requires.
    pub fn aggregate_at<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        change: bool,
        index: u32,
    ) -> Result<AggregateKey, Error> {
        let mut keys = Vec::with_capacity(self.participants.len());
        for participant in &self.participants {
            keys.push(participant.key_at(secp, change, index)?);
        }
        key_agg_with(secp, &sort_keys(&keys))
    }

    /// The aggregate as a BIP-328 extended public key at `change`/`index`,
    /// before the expression's own derivation.
    pub fn aggregate_xpub_at<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        network: Network,
        change: bool,
        index: u32,
    ) -> Result<Xpub, Error> {
        Ok(synthetic_xpub(
            self.aggregate_at(secp, change, index)?.public_key(),
            network.kind(),
        ))
    }

    /// The public key this expression names at `change`/`index`: the
    /// aggregate, then the expression's own BIP-328 derivation.
    pub fn key_at<C: Verification>(
        &self,
        secp: &Secp256k1<C>,
        change: bool,
        index: u32,
    ) -> Result<PublicKey, Error> {
        let aggregate = self.aggregate_at(secp, change, index)?;
        if self.path.is_empty() {
            return Ok(aggregate.public_key());
        }
        let steps = self.path.at(change, index)?;
        Ok(synthetic_xpub(aggregate.public_key(), NetworkKind::Main)
            .derive_pub(secp, &steps)
            .map_err(|_| Error::Derivation)?
            .public_key)
    }
}

/// Reads `musig(KEY,…)`, with or without a derivation of its own.
pub fn parse_musig_expr(text: &str) -> Result<MusigExpr, Error> {
    let text = text.trim();
    let rest = text.strip_prefix("musig(").ok_or(Error::Expression)?;
    let close = rest.find(')').ok_or(Error::Expression)?;
    let (args, tail) = rest.split_at(close);
    let tail = &tail[1..];
    if args.contains('(') || tail.contains(['(', ')']) {
        return Err(Error::Expression);
    }
    let path = parse_path(tail)?;
    let mut participants = Vec::new();
    for arg in args.split(',') {
        participants.push(parse_participant(arg.trim())?);
    }
    if participants.len() < 2 {
        return Err(Error::Expression);
    }
    // BIP-390: a derivation on the musig() needs every participant to be
    // an xpub with no range of its own. Without one, any number of the
    // participants may be ranged, and every one is derived at the same
    // chain and index before the keys are sorted and aggregated.
    if !path.is_empty() {
        for participant in &participants {
            if participant.xpub().is_none() || participant.path.is_ranged() {
                return Err(Error::Derivation);
            }
        }
    }
    Ok(MusigExpr { participants, path })
}

/// Reads `tr(musig(…))`, the only place BIP-390 lets a `musig()` stand
/// that this crate supports. A script tree is not read here.
pub fn parse_tr_musig(text: &str) -> Result<MusigExpr, Error> {
    let text = text.trim();
    let body = text
        .strip_prefix("tr(")
        .and_then(|t| t.strip_suffix(')'))
        .ok_or(Error::Expression)?;
    if body.contains(['{', '}']) {
        return Err(Error::Expression);
    }
    parse_musig_expr(body)
}

/// The `tr(musig(…))` output script at `change`/`index`: the aggregate
/// key, BIP-341-tweaked with no script path.
pub fn script_pubkey_at(expr: &MusigExpr, change: bool, index: u32) -> Result<ScriptBuf, Error> {
    let secp = Secp256k1::verification_only();
    script_pubkey_at_with(&secp, expr, change, index)
}

/// The same script, with a caller's secp context.
pub fn script_pubkey_at_with<C: Verification>(
    secp: &Secp256k1<C>,
    expr: &MusigExpr,
    change: bool,
    index: u32,
) -> Result<ScriptBuf, Error> {
    let internal = AggregateKey::untweaked(expr.key_at(secp, change, index)?);
    let (x_only, _) = internal.x_only();
    let tweak = tagged_hash(b"TapTweak", &[&x_only.serialize()]);
    let output = internal.apply_tweak(secp, &tweak, true)?;
    Ok(ScriptBuf::new_p2tr_tweaked(
        bitcoin::key::TweakedPublicKey::dangerous_assume_tweaked(output.x_only().0),
    ))
}

/// `[fingerprint/path]key/path` — the BIP-380 key expression, restricted
/// to what a `musig()` participant may be.
fn parse_participant(text: &str) -> Result<Participant, Error> {
    let (origin, rest) = match text.strip_prefix('[') {
        Some(rest) => {
            let (origin, rest) = rest.split_once(']').ok_or(Error::Key)?;
            check_origin(origin)?;
            (Some(String::from(origin)), rest)
        }
        None => (None, text),
    };
    let (key, path) = match rest.find('/') {
        Some(i) => (&rest[..i], parse_path(&rest[i..])?),
        None => (rest, Path::default()),
    };
    let key = if key.len() == 66 && key.bytes().all(|b| b.is_ascii_hexdigit()) {
        let mut raw = [0u8; 33];
        for (i, b) in raw.iter_mut().enumerate() {
            *b = u8::from_str_radix(&key[2 * i..2 * i + 2], 16).map_err(|_| Error::Key)?;
        }
        ParticipantKey::Plain(PublicKey::from_slice(&raw).map_err(|_| Error::Key)?)
    } else {
        ParticipantKey::Extended(crate::xkey::decode_xpub(key).map_err(|_| Error::Key)?)
    };
    if matches!(key, ParticipantKey::Plain(_)) && !path.is_empty() {
        return Err(Error::Derivation);
    }
    Ok(Participant { origin, key, path })
}

/// `fingerprint` then an optional `/path`, both of which BIP-380 fixes
/// the shape of.
fn check_origin(origin: &str) -> Result<(), Error> {
    let fingerprint = origin.get(..8).ok_or(Error::Key)?;
    if fingerprint.len() != 8 || !fingerprint.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Key);
    }
    let path = &origin[8..];
    if path.is_empty() {
        return Ok(());
    }
    DerivationPath::from_str(path.strip_prefix('/').ok_or(Error::Key)?)
        .map(|_| ())
        .map_err(|_| Error::Key)
}

/// A derivation as a descriptor writes it: `""`, `/**`, or a run of
/// `/NUM`, `/<a;b>` and a final `/*`, all unhardened.
fn parse_path(text: &str) -> Result<Path, Error> {
    if text.is_empty() {
        return Ok(Path::default());
    }
    if text == "/**" {
        return Ok(Path {
            steps: alloc::vec![Step::Multi(0, 1)],
            wildcard: true,
        });
    }
    let body = text.strip_prefix('/').ok_or(Error::Derivation)?;
    let mut path = Path::default();
    let mut multipaths = 0;
    for part in body.split('/') {
        if path.wildcard {
            return Err(Error::Derivation);
        }
        if part == "*" {
            path.wildcard = true;
        } else if let Some(inner) = part.strip_prefix('<').and_then(|p| p.strip_suffix('>')) {
            let (a, b) = inner.split_once(';').ok_or(Error::Derivation)?;
            multipaths += 1;
            path.steps.push(Step::Multi(index_of(a)?, index_of(b)?));
        } else {
            path.steps.push(Step::Fixed(index_of(part)?));
        }
    }
    if multipaths > 1 {
        return Err(Error::Derivation);
    }
    Ok(path)
}

/// One unhardened index. `NUMh` and `NUM'` are hardened, which no
/// derivation from an aggregate public key can be.
fn index_of(text: &str) -> Result<u32, Error> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Derivation);
    }
    let n: u32 = text.parse().map_err(|_| Error::Derivation)?;
    if n >= 0x8000_0000 {
        return Err(Error::IndexOutOfRange);
    }
    Ok(n)
}

// ---------------------------------------------------------------------
// BIP-327: the signing side
// ---------------------------------------------------------------------

/// `(a * b) mod n`, for `a` and `b` already reduced.
///
/// Double-and-add over the bits of `b`, selecting without a branch, so
/// that a secret scalar's bits do not steer the control flow.
pub(crate) fn mul_mod_order(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut acc = [0u8; 32];
    for i in 0..256 {
        acc = add_mod_order(&acc, &acc);
        let bit = (b[i / 8] >> (7 - (i % 8))) & 1;
        let added = add_mod_order(&acc, a);
        let mask = 0u8.wrapping_sub(bit);
        for (out, add) in acc.iter_mut().zip(added.iter()) {
            *out ^= mask & (*out ^ *add);
        }
    }
    acc
}

/// A point that may be the point at infinity, which `PublicKey` cannot
/// hold: BIP-327's aggregate nonce halves and the sums this module takes
/// of them.
pub(crate) type MaybePoint = Option<PublicKey>;

pub(crate) fn point_add<C: Verification>(
    secp: &Secp256k1<C>,
    a: MaybePoint,
    b: MaybePoint,
) -> Result<MaybePoint, Error> {
    let _ = secp;
    match (a, b) {
        (None, other) | (other, None) => Ok(other),
        (Some(a), Some(b)) => match a.combine(&b) {
            Ok(sum) => Ok(Some(sum)),
            // `combine` fails only when the sum is the point at infinity.
            Err(_) => Ok(None),
        },
    }
}

pub(crate) fn point_mul<C: Verification>(
    secp: &Secp256k1<C>,
    point: MaybePoint,
    scalar: &[u8; 32],
) -> Result<MaybePoint, Error> {
    let Some(point) = point else { return Ok(None) };
    if *scalar == [0u8; 32] {
        return Ok(None);
    }
    let scalar = Scalar::from_be_bytes(*scalar).map_err(|_| Error::TweakOutOfRange)?;
    Ok(Some(
        point
            .mul_tweak(secp, &scalar)
            .map_err(|_| Error::Infinity)?,
    ))
}

pub(crate) fn point_negate<C: Verification>(secp: &Secp256k1<C>, point: MaybePoint) -> MaybePoint {
    point.map(|p| p.negate(secp))
}

/// The generator point, compressed, as secp256k1 publishes it.
const GENERATOR: [u8; 33] = [
    0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b,
    0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17,
    0x98,
];

pub(crate) fn generator() -> PublicKey {
    PublicKey::from_slice(&GENERATOR).expect("the generator is a point")
}

/// `scalar · G`, the point at infinity for a zero scalar.
pub(crate) fn base_mul<C: Verification>(
    secp: &Secp256k1<C>,
    scalar: &[u8; 32],
) -> Result<MaybePoint, Error> {
    point_mul(secp, Some(generator()), scalar)
}

/// BIP-327's `cbytes_ext`: 33 zero bytes for the point at infinity.
pub(crate) fn cbytes_ext(point: MaybePoint) -> [u8; 33] {
    match point {
        Some(p) => p.serialize(),
        None => [0u8; 33],
    }
}

/// BIP-327's `cpoint_ext`: 33 zero bytes read back as infinity.
pub(crate) fn cpoint_ext(bytes: &[u8]) -> Result<MaybePoint, ()> {
    if bytes == [0u8; 33] {
        return Ok(None);
    }
    PublicKey::from_slice(bytes).map(Some).map_err(|_| ())
}

/// One signer's public nonce: two compressed points, 66 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PubNonce([u8; 66]);

impl PubNonce {
    /// Reads the 66 bytes, both halves of which must be points.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let bytes: [u8; 66] = bytes.try_into().map_err(|_| Error::PubNonce)?;
        for half in 0..2 {
            PublicKey::from_slice(&bytes[half * 33..half * 33 + 33])
                .map_err(|_| Error::PubNonce)?;
        }
        Ok(PubNonce(bytes))
    }

    /// The 66 bytes.
    pub fn serialize(&self) -> [u8; 66] {
        self.0
    }
}

/// The aggregate of every signer's public nonce, 66 bytes, either half
/// of which may be the extended encoding of the point at infinity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggNonce([u8; 66]);

impl AggNonce {
    /// Reads the 66 bytes, each half a point or 33 zero bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let bytes: [u8; 66] = bytes.try_into().map_err(|_| Error::AggNonce)?;
        for half in 0..2 {
            cpoint_ext(&bytes[half * 33..half * 33 + 33]).map_err(|()| Error::AggNonce)?;
        }
        Ok(AggNonce(bytes))
    }

    /// The 66 bytes.
    pub fn serialize(&self) -> [u8; 66] {
        self.0
    }
}

/// One signer's partial signature: a scalar below the curve order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialSig([u8; 32]);

impl PartialSig {
    /// Reads the 32 bytes, which must be below the curve order.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| Error::PartialSig)?;
        if at_least_order(&bytes) {
            return Err(Error::PartialSig);
        }
        Ok(PartialSig(bytes))
    }

    /// The 32 bytes.
    pub fn serialize(&self) -> [u8; 32] {
        self.0
    }
}

/// One tweak of the aggregate key, in the order the caller applies them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tweak {
    /// The tweak scalar, big-endian.
    pub bytes: [u8; 32],
    /// BIP-327's x-only form, which BIP-341's taproot tweak needs; a
    /// BIP-32 derivation tweak is the plain form.
    pub x_only: bool,
}

/// A secret nonce: two scalars and the public key they were drawn for.
///
/// Not `Clone`, wiped on drop, and consumed by value by [`sign`], so one
/// nonce cannot sign twice.
pub struct SecNonce {
    k1: [u8; 32],
    k2: [u8; 32],
    pk: [u8; 33],
}

impl SecNonce {
    /// Reads BIP-327's 97-byte serialisation, which the vectors use.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 97 {
            return Err(Error::SecNonce);
        }
        Ok(SecNonce {
            k1: bytes[..32].try_into().expect("32 bytes"),
            k2: bytes[32..64].try_into().expect("32 bytes"),
            pk: bytes[64..].try_into().expect("33 bytes"),
        })
    }

    /// The 97 bytes, for the vectors and for nothing else: a secret
    /// nonce is never written anywhere this device keeps.
    pub fn serialize(&self) -> [u8; 97] {
        let mut out = [0u8; 97];
        out[..32].copy_from_slice(&self.k1);
        out[32..64].copy_from_slice(&self.k2);
        out[64..].copy_from_slice(&self.pk);
        out
    }
}

impl core::fmt::Debug for SecNonce {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecNonce(..)")
    }
}

impl osk_crypto::Zeroize for SecNonce {
    fn zeroize(&mut self) {
        self.k1.zeroize();
        self.k2.zeroize();
        self.pk.zeroize();
    }
}

impl Drop for SecNonce {
    fn drop(&mut self) {
        use osk_crypto::Zeroize;
        self.zeroize();
    }
}

impl osk_crypto::ZeroizeOnDrop for SecNonce {}

/// BIP-327's `NonceAgg`.
pub fn nonce_agg(pubnonces: &[PubNonce]) -> Result<AggNonce, Error> {
    let secp = Secp256k1::verification_only();
    nonce_agg_with(&secp, pubnonces)
}

/// The same aggregation, with a caller's secp context.
pub fn nonce_agg_with<C: Verification>(
    secp: &Secp256k1<C>,
    pubnonces: &[PubNonce],
) -> Result<AggNonce, Error> {
    if pubnonces.is_empty() {
        return Err(Error::NoKeys);
    }
    let mut out = [0u8; 66];
    for half in 0..2 {
        let mut sum: MaybePoint = None;
        for nonce in pubnonces {
            let point = PublicKey::from_slice(&nonce.0[half * 33..half * 33 + 33])
                .map_err(|_| Error::PubNonce)?;
            sum = point_add(secp, sum, Some(point))?;
        }
        out[half * 33..half * 33 + 33].copy_from_slice(&cbytes_ext(sum));
    }
    Ok(AggNonce(out))
}

/// BIP-327's `NonceGen*`, with the `rand'` the caller draws.
///
/// The caller draws `rand` afresh for every call; the vectors fix it.
pub fn nonce_gen<C: Verification + bitcoin::secp256k1::Signing>(
    secp: &Secp256k1<C>,
    rand: &[u8; 32],
    sk: Option<&bitcoin::secp256k1::SecretKey>,
    pk: &PublicKey,
    aggpk: Option<&[u8; 32]>,
    msg: Option<&[u8]>,
    extra_in: Option<&[u8]>,
) -> Result<(SecNonce, PubNonce), Error> {
    let mut seed = *rand;
    if let Some(sk) = sk {
        let masked = tagged_hash(b"MuSig/aux", &[rand]);
        for (s, m) in seed.iter_mut().zip(sk.secret_bytes().iter().zip(masked)) {
            *s = m.0 ^ m.1;
        }
    }
    let pk_bytes = pk.serialize();
    let aggpk = aggpk.map_or(&[][..], |a| &a[..]);
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
            b"MuSig/nonce",
            &[
                &seed,
                &[pk_bytes.len() as u8],
                &pk_bytes,
                &[aggpk.len() as u8],
                aggpk,
                &prefixed_msg,
                &(extra_in.len() as u32).to_be_bytes(),
                extra_in,
                &[i as u8],
            ],
        ));
        if *k == [0u8; 32] {
            return Err(Error::SecNonce);
        }
    }
    let mut pubnonce = [0u8; 66];
    for (i, k) in k.iter().enumerate() {
        let point = base_mul(secp, k)?.ok_or(Error::Infinity)?;
        pubnonce[i * 33..i * 33 + 33].copy_from_slice(&point.serialize());
    }
    Ok((
        SecNonce {
            k1: k[0],
            k2: k[1],
            pk: pk_bytes,
        },
        PubNonce(pubnonce),
    ))
}

/// BIP-327's `GetSessionValues`: the tweaked aggregate key, the nonce
/// coefficient `b`, the final nonce `R` and the challenge `e`.
///
/// Every value here is public; the secret of a signing session is the
/// [`SecNonce`], which this does not hold.
#[derive(Debug, Clone)]
pub struct SessionValues {
    pubkeys: Vec<PublicKey>,
    aggregate: AggregateKey,
    b: [u8; 32],
    r: PublicKey,
    e: [u8; 32],
}

impl SessionValues {
    /// The tweaked aggregate key `Q`, whose x-only form is the key a
    /// BIP-340 verifier checks the finished signature against.
    pub fn aggregate_key(&self) -> AggregateKey {
        self.aggregate
    }
}

/// BIP-327's `GetSessionValues`: the aggregate of `pubkeys` with
/// `tweaks` applied in order, against `aggnonce` and `msg`.
pub fn session_values<C: Verification>(
    secp: &Secp256k1<C>,
    pubkeys: &[PublicKey],
    tweaks: &[Tweak],
    aggnonce: &AggNonce,
    msg: &[u8],
) -> Result<SessionValues, Error> {
    let mut aggregate = key_agg_with(secp, pubkeys)?;
    for tweak in tweaks {
        aggregate = aggregate.apply_tweak(secp, &tweak.bytes, tweak.x_only)?;
    }
    let q = aggregate.serialize_x_only();
    let b = reduce(tagged_hash(b"MuSig/noncecoef", &[&aggnonce.0, &q, msg]));
    let r1 = cpoint_ext(&aggnonce.0[..33]).map_err(|()| Error::AggNonce)?;
    let r2 = cpoint_ext(&aggnonce.0[33..]).map_err(|()| Error::AggNonce)?;
    let scaled = point_mul(secp, r2, &b)?;
    let r = match point_add(secp, r1, scaled)? {
        Some(r) => r,
        // BIP-327: an aggregate nonce of infinity signs under the
        // generator, so that the session still has a nonce.
        None => generator(),
    };
    let e = reduce(tagged_hash(
        b"BIP0340/challenge",
        &[&r.x_only_public_key().0.serialize(), &q, msg],
    ));
    Ok(SessionValues {
        pubkeys: pubkeys.to_vec(),
        aggregate,
        b,
        r,
        e,
    })
}

/// BIP-327's `GetSessionKeyAggCoeff`, which fails when the key is not a
/// participant of the session.
fn session_coeff(session: &SessionValues, pk: &PublicKey) -> Result<[u8; 32], Error> {
    if !session.pubkeys.contains(pk) {
        return Err(Error::NotAParticipant);
    }
    let list = hash_keys(&session.pubkeys);
    Ok(key_agg_coeff(&list, pk, second_key(&session.pubkeys)))
}

/// `g` for a point: 1 when its y is even, `n - 1` otherwise.
pub(crate) fn parity_factor(point: &PublicKey) -> [u8; 32] {
    if point.x_only_public_key().1 == Parity::Even {
        one()
    } else {
        negate_mod_order(&one())
    }
}

/// BIP-327's `Sign`. The nonce is consumed, so it cannot sign twice.
pub fn sign<C: Verification + bitcoin::secp256k1::Signing>(
    secp: &Secp256k1<C>,
    secnonce: SecNonce,
    sk: &bitcoin::secp256k1::SecretKey,
    session: &SessionValues,
) -> Result<PartialSig, Error> {
    let (k1, k2) = (secnonce.k1, secnonce.k2);
    for k in [&k1, &k2] {
        if *k == [0u8; 32] || at_least_order(k) {
            return Err(Error::SecNonce);
        }
    }
    let even = session.r.x_only_public_key().1 == Parity::Even;
    let (k1, k2) = if even {
        (k1, k2)
    } else {
        (negate_mod_order(&k1), negate_mod_order(&k2))
    };
    let d0 = sk.secret_bytes();
    let point = sk.public_key(secp);
    if point.serialize() != secnonce.pk {
        return Err(Error::SecNonce);
    }
    let a = session_coeff(session, &point)?;
    let g = parity_factor(&session.aggregate.point);
    let d = mul_mod_order(&mul_mod_order(&g, &session.aggregate.gacc), &d0);
    let s = add_mod_order(
        &add_mod_order(&k1, &mul_mod_order(&session.b, &k2)),
        &mul_mod_order(&mul_mod_order(&session.e, &a), &d),
    );
    Ok(PartialSig(s))
}

/// BIP-327's `PartialSigVerifyInternal`: the session values are already
/// in hand.
pub fn partial_sig_verify_internal<C: Verification>(
    secp: &Secp256k1<C>,
    psig: &PartialSig,
    pubnonce: &PubNonce,
    pk: &PublicKey,
    session: &SessionValues,
) -> Result<(), Error> {
    let r1 = PublicKey::from_slice(&pubnonce.0[..33]).map_err(|_| Error::PubNonce)?;
    let r2 = PublicKey::from_slice(&pubnonce.0[33..]).map_err(|_| Error::PubNonce)?;
    let scaled = point_mul(secp, Some(r2), &session.b)?;
    let effective = point_add(secp, Some(r1), scaled)?;
    let effective = if session.r.x_only_public_key().1 == Parity::Even {
        effective
    } else {
        point_negate(secp, effective)
    };
    let a = session_coeff(session, pk)?;
    let g = parity_factor(&session.aggregate.point);
    let gg = mul_mod_order(&g, &session.aggregate.gacc);
    let scalar = mul_mod_order(&mul_mod_order(&session.e, &a), &gg);
    let term = point_mul(secp, Some(*pk), &scalar)?;
    let right = point_add(secp, effective, term)?;
    let left = base_mul(secp, &psig.0)?;
    if left == right {
        Ok(())
    } else {
        Err(Error::VerifyFailed)
    }
}

/// BIP-327's `PartialSigVerify`: the whole check, from the nonces and
/// keys of the session.
#[allow(clippy::too_many_arguments)]
pub fn partial_sig_verify<C: Verification>(
    secp: &Secp256k1<C>,
    psig: &PartialSig,
    pubnonces: &[PubNonce],
    pubkeys: &[PublicKey],
    tweaks: &[Tweak],
    msg: &[u8],
    index: usize,
) -> Result<(), Error> {
    let aggnonce = nonce_agg_with(secp, pubnonces)?;
    let session = session_values(secp, pubkeys, tweaks, &aggnonce, msg)?;
    let pubnonce = pubnonces.get(index).ok_or(Error::NotAParticipant)?;
    let pk = pubkeys.get(index).ok_or(Error::NotAParticipant)?;
    partial_sig_verify_internal(secp, psig, pubnonce, pk, &session)
}

/// BIP-327's `PartialSigAgg`: the 64-byte BIP-340 signature.
pub fn partial_sig_agg(psigs: &[PartialSig], session: &SessionValues) -> Result<[u8; 64], Error> {
    let g = parity_factor(&session.aggregate.point);
    let mut s = mul_mod_order(&mul_mod_order(&session.e, &g), &session.aggregate.tacc);
    for psig in psigs {
        s = add_mod_order(&s, &psig.0);
    }
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&session.r.x_only_public_key().0.serialize());
    out[32..].copy_from_slice(&s);
    Ok(out)
}

/// BIP-327's `DeterministicSign` (its appendix): the nonce comes from
/// the other signers' aggregate nonce, so the signer keeps nothing
/// between the rounds and signs the same inputs the same way twice.
pub fn deterministic_sign<C: Verification + bitcoin::secp256k1::Signing>(
    secp: &Secp256k1<C>,
    sk: &bitcoin::secp256k1::SecretKey,
    aggothernonce: &AggNonce,
    pubkeys: &[PublicKey],
    tweaks: &[Tweak],
    msg: &[u8],
    rand: Option<&[u8; 32]>,
) -> Result<(PubNonce, PartialSig), Error> {
    let mut seed = sk.secret_bytes();
    if let Some(rand) = rand {
        let masked = tagged_hash(b"MuSig/aux", &[rand]);
        for (s, m) in seed.iter_mut().zip(masked) {
            *s ^= m;
        }
    }
    let mut aggregate = key_agg_with(secp, pubkeys)?;
    for tweak in tweaks {
        aggregate = aggregate.apply_tweak(secp, &tweak.bytes, tweak.x_only)?;
    }
    let aggpk = aggregate.serialize_x_only();
    let mut k = [[0u8; 32]; 2];
    for (i, k) in k.iter_mut().enumerate() {
        *k = reduce(tagged_hash(
            b"MuSig/deterministic/nonce",
            &[
                &seed,
                &aggothernonce.0,
                &aggpk,
                &(msg.len() as u64).to_be_bytes(),
                msg,
                &[i as u8],
            ],
        ));
        if *k == [0u8; 32] {
            return Err(Error::SecNonce);
        }
    }
    let pk = sk.public_key(secp);
    let mut pubnonce = [0u8; 66];
    for (i, k) in k.iter().enumerate() {
        let point = base_mul(secp, k)?.ok_or(Error::Infinity)?;
        pubnonce[i * 33..i * 33 + 33].copy_from_slice(&point.serialize());
    }
    let pubnonce = PubNonce(pubnonce);
    let secnonce = SecNonce {
        k1: k[0],
        k2: k[1],
        pk: pk.serialize(),
    };
    // `NonceAgg` of this nonce and the others', which reads both as
    // public nonces: a half of `aggothernonce` that is not a point —
    // infinity included — is refused here, as BIP-327's appendix says.
    let mut aggnonce = [0u8; 66];
    for half in 0..2 {
        let mine = PublicKey::from_slice(&pubnonce.0[half * 33..half * 33 + 33])
            .map_err(|_| Error::PubNonce)?;
        let theirs = PublicKey::from_slice(&aggothernonce.0[half * 33..half * 33 + 33])
            .map_err(|_| Error::AggNonce)?;
        let sum = point_add(secp, Some(mine), Some(theirs))?;
        aggnonce[half * 33..half * 33 + 33].copy_from_slice(&cbytes_ext(sum));
    }
    let session = session_values(secp, pubkeys, tweaks, &AggNonce(aggnonce), msg)?;
    let psig = sign(secp, secnonce, sk, &session)?;
    Ok((pubnonce, psig))
}
