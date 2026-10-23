//! BIP-374 discrete log equality proofs over secp256k1.
//!
//! A proof shows that `A = a·G` and `C = a·B` share one secret `a`
//! without giving `a` away. BIP-375 uses it for silent payments: a
//! signer that adds an ECDH share `C = a·B_scan` to a PSBT adds a proof
//! with it, so that whoever computes the output scripts from the shares
//! can check every share came from the key that spends its input.
//!
//! [`generate_proof`] and [`verify_proof`] use secp256k1's own
//! generator; the `_with` forms take another generator, as the BIP
//! allows and its vectors exercise.
//!
//! Every multiplication by a secret scalar (`a` and the nonce `k`) goes
//! through `secp256k1_ecdh`, which takes the same time for every
//! scalar. Multiplications by the public challenge and response go
//! through `mul_tweak`, which is right for public values only. The
//! secret's bytes, the nonce and the hashes they pass through are
//! zeroized before return.

use bitcoin::secp256k1::{All, PublicKey, Secp256k1, SecretKey, Verification};
use faraday_crypto::Zeroize;

use crate::musig::{
    add_mod_order, at_least_order, generator, mul_mod_order, point_add, point_mul, point_negate,
    reduce, tagged_hash,
};

/// Why a proof could not be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The nonce BIP-374 derives is zero, which the BIP says to fail on.
    Nonce,
    /// A product is the point at infinity, which is no public key.
    Infinity,
    /// The proof made does not verify, which the BIP says to abort on.
    SelfCheck,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Error::Nonce => "the proof's nonce is zero",
            Error::Infinity => "a product is the point at infinity",
            Error::SelfCheck => "the proof made does not verify",
        })
    }
}

impl core::error::Error for Error {}

/// A 64-byte proof: `e` then `s`, each 32 bytes big-endian.
pub type Proof = [u8; 64];

/// Costs one elliptic-curve multiplication by a secret scalar:
/// `secret · point`, in a time that does not depend on the scalar.
pub fn secret_mul(point: &PublicKey, secret: &SecretKey) -> Result<PublicKey, Error> {
    let mut xy = bitcoin::secp256k1::ecdh::shared_secret_point(point, secret);
    let mut uncompressed = [0u8; 65];
    uncompressed[0] = 0x04;
    uncompressed[1..].copy_from_slice(&xy);
    let product = PublicKey::from_slice(&uncompressed).map_err(|_| Error::Infinity);
    xy.zeroize();
    uncompressed.zeroize();
    product
}

/// BIP-374's challenge: `hash_BIP0374/challenge(A‖B‖C‖G‖R1‖R2‖m′)`
/// reduced modulo the curve order.
fn challenge(points: [&PublicKey; 6], m: Option<&[u8; 32]>) -> [u8; 32] {
    let bytes = points.map(|p| p.serialize());
    let m: &[u8] = m.map_or(&[], |m| &m[..]);
    reduce(tagged_hash(
        b"BIP0374/challenge",
        &[
            &bytes[0], &bytes[1], &bytes[2], &bytes[3], &bytes[4], &bytes[5], m,
        ],
    ))
}

/// Costs four elliptic-curve multiplications by a secret and the four
/// of a verification: BIP-374's `GenerateProof(a, B, r, G, m)` with
/// secp256k1's generator.
///
/// `r` is auxiliary randomness. The nonce is derived from `a`, `A`, `C`
/// and `m` as well, so a fixed `r` still gives a nonce no other proof
/// under another secret or statement shares; fresh randomness is what
/// the BIP recommends, as BIP-340 does.
///
/// The BIP's first two failures, `a` zero or not below the curve order
/// and `B` the point at infinity, are held by the types: no
/// [`SecretKey`] is either, and no [`PublicKey`] is the point at
/// infinity.
pub fn generate_proof(
    secp: &Secp256k1<All>,
    a: &SecretKey,
    b: &PublicKey,
    r: &[u8; 32],
    m: Option<&[u8; 32]>,
) -> Result<Proof, Error> {
    generate_proof_with(secp, a, b, r, &generator(), m)
}

/// Costs what [`generate_proof`] costs: the same, under the generator
/// `g`.
pub fn generate_proof_with(
    secp: &Secp256k1<All>,
    a: &SecretKey,
    b: &PublicKey,
    r: &[u8; 32],
    g: &PublicKey,
    m: Option<&[u8; 32]>,
) -> Result<Proof, Error> {
    let big_a = secret_mul(g, a)?;
    let big_c = secret_mul(b, a)?;
    let mut a_bytes = a.secret_bytes();
    let mut t = tagged_hash(b"BIP0374/aux", &[r]);
    for (t, a) in t.iter_mut().zip(a_bytes.iter()) {
        *t ^= *a;
    }
    let m_bytes: &[u8] = m.map_or(&[], |m| &m[..]);
    let mut rand = tagged_hash(
        b"BIP0374/nonce",
        &[&t, &big_a.serialize(), &big_c.serialize(), m_bytes],
    );
    t.zeroize();
    let mut k_bytes = reduce(rand);
    rand.zeroize();
    // `reduce` leaves zero only for zero, the one value `SecretKey`
    // refuses that a reduced scalar can take.
    let Ok(mut k) = SecretKey::from_slice(&k_bytes) else {
        a_bytes.zeroize();
        k_bytes.zeroize();
        return Err(Error::Nonce);
    };
    let points = secret_mul(g, &k).and_then(|r1| Ok((r1, secret_mul(b, &k)?)));
    k.non_secure_erase();
    let (r1, r2) = match points {
        Ok(points) => points,
        Err(e) => {
            a_bytes.zeroize();
            k_bytes.zeroize();
            return Err(e);
        }
    };
    let e = challenge([&big_a, b, &big_c, g, &r1, &r2], m);
    let mut ea = mul_mod_order(&e, &a_bytes);
    a_bytes.zeroize();
    let s = add_mod_order(&k_bytes, &ea);
    ea.zeroize();
    k_bytes.zeroize();
    let mut proof = [0u8; 64];
    proof[..32].copy_from_slice(&e);
    proof[32..].copy_from_slice(&s);
    if !verify_proof_with(secp, &big_a, b, &big_c, &proof, g, m) {
        return Err(Error::SelfCheck);
    }
    Ok(proof)
}

/// Costs four elliptic-curve multiplications: BIP-374's
/// `VerifyProof(A, B, C, proof, G, m)` with secp256k1's generator.
pub fn verify_proof<C: Verification>(
    secp: &Secp256k1<C>,
    a: &PublicKey,
    b: &PublicKey,
    c: &PublicKey,
    proof: &Proof,
    m: Option<&[u8; 32]>,
) -> bool {
    verify_proof_with(secp, a, b, c, proof, &generator(), m)
}

/// Costs what [`verify_proof`] costs: the same, under the generator
/// `g`.
///
/// The BIP's first failure, a point at infinity among the inputs, is
/// held by the type: no [`PublicKey`] is the point at infinity.
pub fn verify_proof_with<C: Verification>(
    secp: &Secp256k1<C>,
    a: &PublicKey,
    b: &PublicKey,
    c: &PublicKey,
    proof: &Proof,
    g: &PublicKey,
    m: Option<&[u8; 32]>,
) -> bool {
    let mut e = [0u8; 32];
    let mut s = [0u8; 32];
    e.copy_from_slice(&proof[..32]);
    s.copy_from_slice(&proof[32..]);
    if at_least_order(&e) || at_least_order(&s) {
        return false;
    }
    // `s·P − e·Q`, or `None` for the point at infinity.
    let combine = |p: &PublicKey, q: &PublicKey| -> Option<PublicKey> {
        let sp = point_mul(secp, Some(*p), &s).ok()?;
        let eq = point_mul(secp, Some(*q), &e).ok()?;
        point_add(secp, sp, point_negate(secp, eq)).ok()?
    };
    let Some(r1) = combine(g, a) else {
        return false;
    };
    let Some(r2) = combine(b, c) else {
        return false;
    };
    challenge([a, b, c, g, &r1, &r2], m) == e
}
