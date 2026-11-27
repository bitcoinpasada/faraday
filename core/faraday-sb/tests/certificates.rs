//! What reads as a certificate: only one certificate in DER, whole and
//! with nothing after it. Everything else reads as no certificate: it
//! names no one, verifies nothing and signs nothing.

use faraday_sb::x509;
use faraday_sb::{common_name, make};

const NOW: u64 = 1_700_000_000;

fn cert() -> Vec<u8> {
    make(&[3u8; 32], "Reader", NOW).unwrap().db.cert
}

#[test]
fn bytes_that_are_not_one_whole_der_certificate_read_as_none() {
    let good = cert();
    let mut trailing = good.clone();
    trailing.push(0);
    // The outer length written with a needless leading zero octet: BER,
    // not DER.
    let mut padded = vec![good[0], 0x83, 0x00];
    padded.extend_from_slice(&good[2..]);
    // The first validity time, 2023-11-14 22:13:20 UTC, with its hour made 24.
    let mut late_hour = good.clone();
    let at = late_hour
        .windows(13)
        .position(|w| w == b"231114221320Z")
        .expect("the validity start is in the certificate");
    late_hour[at + 6..at + 8].copy_from_slice(b"24");
    let cases: [(&str, Vec<u8>); 8] = [
        ("empty", vec![]),
        ("zeros", vec![0; 64]),
        ("cut short", good[..good.len() - 1].to_vec()),
        ("bytes after it", trailing),
        ("a length that is not minimal", padded),
        ("a sequence with no fields", vec![0x30, 0x00]),
        (
            "a sequence with one field",
            vec![0x30, 0x03, 0x02, 0x01, 0x00],
        ),
        ("a validity time with an hour of 24", late_hour),
    ];
    for (what, bytes) in cases {
        assert!(x509::read(&bytes).is_none(), "{what}");
        assert_eq!(common_name(&bytes), None, "{what}");
        assert!(!x509::verify(&bytes, b"message", &[0; 256]), "{what}");
        assert!(
            x509::signer_info(&bytes, None, &[0; 256]).is_err(),
            "{what}"
        );
    }
}

#[test]
fn a_signature_by_another_key_does_not_verify() {
    let keys = make(&[3u8; 32], "Reader", NOW).unwrap();
    let mut rng = faraday_sb::rng::stream(&[5u8; 32], 0);
    let sig = x509::sign(&mut rng, &keys.db.key, b"message").unwrap();
    assert!(x509::verify(&keys.db.cert, b"message", &sig));
    assert!(!x509::verify(&keys.kek.cert, b"message", &sig));
    assert!(!x509::verify(&keys.db.cert, b"another message", &sig));
}

#[test]
fn a_certificate_whose_key_is_not_an_rsa_public_key_verifies_nothing() {
    let keys = make(&[3u8; 32], "Reader", NOW).unwrap();
    let mut rng = faraday_sb::rng::stream(&[5u8; 32], 0);
    let sig = x509::sign(&mut rng, &keys.db.key, b"message").unwrap();
    assert!(x509::verify(&keys.db.cert, b"message", &sig));
    // The key's algorithm is rsaEncryption (1.2.840.113549.1.1.1); name
    // another one (1.2.840.113549.1.1.2) and the certificate still reads.
    let rsa_encryption = [
        0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01,
    ];
    let mut other = keys.db.cert.clone();
    let at = other
        .windows(rsa_encryption.len())
        .position(|w| w == rsa_encryption)
        .expect("the key's algorithm is named");
    other[at + rsa_encryption.len() - 1] = 0x02;
    assert!(x509::read(&other).is_some());
    assert!(!x509::verify(&other, b"message", &sig));
}
