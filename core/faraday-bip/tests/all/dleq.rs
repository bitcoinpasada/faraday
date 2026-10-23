//! BIP-374 discrete log equality proofs against the BIP's published
//! vectors (`tools/vectors/bip374/`): every generation row gives the
//! proof the BIP expects or fails where it fails, and every verification
//! row holds or fails as the file says.

use crate::common::unhex;
use faraday_bip::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
use faraday_bip::dleq;

const GENERATE: &str =
    include_str!("../../../../tools/vectors/bip374/test_vectors_generate_proof.csv");
const VERIFY: &str = include_str!("../../../../tools/vectors/bip374/test_vectors_verify_proof.csv");

/// A point as the file writes it; `None` for `INFINITY`, which no
/// `PublicKey` can hold.
fn point(hex: &str) -> Option<PublicKey> {
    if hex == "INFINITY" {
        return None;
    }
    Some(PublicKey::from_slice(&unhex(hex)).expect("a point"))
}

/// The optional 32-byte message; an empty field is none.
fn message(hex: &str) -> Option<[u8; 32]> {
    if hex.is_empty() {
        return None;
    }
    Some(unhex(hex).try_into().expect("32 bytes"))
}

fn rows(file: &str) -> impl Iterator<Item = Vec<&str>> {
    file.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim_end_matches('\r').split(',').collect())
}

#[test]
fn generation_vectors() {
    let secp = Secp256k1::new();
    let mut count = 0;
    for row in rows(GENERATE) {
        let [index, g, a, b, r, m, expected, comment] = row[..] else {
            panic!("eight columns");
        };
        let r: [u8; 32] = unhex(r).try_into().expect("32 bytes");
        let m = message(m);
        let made = match (SecretKey::from_slice(&unhex(a)), point(b)) {
            // A secret of zero or the curve order, and `B` at infinity,
            // are refused before a proof is attempted.
            (Err(_), _) | (_, None) => None,
            (Ok(a), Some(b)) => {
                let g = point(g).expect("a generator");
                dleq::generate_proof_with(&secp, &a, &b, &r, &g, m.as_ref()).ok()
            }
        };
        let expected = (expected != "INVALID").then(|| unhex(expected));
        assert_eq!(made.map(|p| p.to_vec()), expected, "row {index}: {comment}");
        count += 1;
    }
    assert_eq!(count, 11);
}

#[test]
fn verification_vectors() {
    let secp = Secp256k1::verification_only();
    let mut count = 0;
    for row in rows(VERIFY) {
        let [index, g, a, b, c, proof, m, expected, comment] = row[..] else {
            panic!("nine columns");
        };
        let points = [point(g), point(a), point(b), point(c)];
        let [Some(g), Some(a), Some(b), Some(c)] = points else {
            panic!("row {index}: the file has no point at infinity here");
        };
        let proof: [u8; 64] = unhex(proof).try_into().expect("64 bytes");
        let holds = dleq::verify_proof_with(&secp, &a, &b, &c, &proof, &g, message(m).as_ref());
        assert_eq!(holds, expected == "TRUE", "row {index}: {comment}");
        count += 1;
    }
    assert_eq!(count, 17);
}

#[test]
fn a_proof_made_under_the_standard_generator_verifies_and_binds_its_message() {
    let secp = Secp256k1::new();
    let a = SecretKey::from_slice(&[0x11; 32]).expect("a key");
    let b = SecretKey::from_slice(&[0x22; 32])
        .expect("a key")
        .public_key(&secp);
    let big_a = a.public_key(&secp);
    let big_c = dleq::secret_mul(&b, &a).expect("a point");
    let m = [7u8; 32];
    let proof = dleq::generate_proof(&secp, &a, &b, &[0u8; 32], Some(&m)).expect("a proof");
    assert!(dleq::verify_proof(
        &secp,
        &big_a,
        &b,
        &big_c,
        &proof,
        Some(&m)
    ));
    assert!(!dleq::verify_proof(&secp, &big_a, &b, &big_c, &proof, None));
    // A share from another key does not pass under this key's proof.
    let other =
        dleq::secret_mul(&b, &SecretKey::from_slice(&[0x33; 32]).expect("a key")).expect("a point");
    assert!(!dleq::verify_proof(
        &secp,
        &big_a,
        &b,
        &other,
        &proof,
        Some(&m)
    ));
}
