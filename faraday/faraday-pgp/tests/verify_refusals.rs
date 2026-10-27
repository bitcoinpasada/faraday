//! What `faraday_pgp::verify_detached` refuses (`docs/decisions/F134.md`):
//! signatures of shapes Faraday never writes, made here byte by byte with
//! a valid Ed25519 signature over them, so that each is refused for its
//! shape and not for a bad signature; armour that is not one whole block
//! with its checksum; a GnuPG signature by another key with its issuer
//! rewritten; and certificates whose subkey is bound by another primary
//! key or carries no back signature.

use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};
use faraday_pgp::{Armor, Certificate, Hash, Key, Unverified, armor, verify_detached};
use sha2::{Digest, Sha512};

/// When the test keys were made.
const MADE: u32 = 1_780_000_000;
const DATA: &[u8] = b"# Faraday 0.9.0, commit 0\n";

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/release-sums")
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn key(primary: u8, subkey: u8) -> Key {
    Key {
        primary_seed: [primary; 32],
        created: MADE,
        subkey_seed: [subkey; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Release <release@example.com>".to_string()],
        expiry: 0,
    }
}

/// A packet with a new-format header.
fn packet(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![0xC0 | tag];
    let n = body.len();
    if n < 192 {
        out.push(n as u8);
    } else {
        let m = n - 192;
        out.extend([((m >> 8) + 192) as u8, (m & 0xff) as u8]);
    }
    out.extend_from_slice(body);
    out
}

/// The packets Faraday's `Key::certificate` writes: new-format headers.
fn packets(bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let tag = bytes[at] & 0x3f;
        let first = usize::from(bytes[at + 1]);
        let (len, head) = if first < 192 {
            (first, 2)
        } else {
            (((first - 192) << 8) + usize::from(bytes[at + 2]) + 192, 3)
        };
        out.push((tag, bytes[at + head..at + head + len].to_vec()));
        at += head + len;
    }
    out
}

fn sub(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut out = vec![(data.len() + 1) as u8, kind];
    out.extend_from_slice(data);
    out
}

fn mpi(bytes: &[u8]) -> Vec<u8> {
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    let b = &bytes[start..];
    let bits = if b.is_empty() {
        0
    } else {
        (b.len() as u16 - 1) * 8 + (8 - b[0].leading_zeros() as u16)
    };
    let mut out = bits.to_be_bytes().to_vec();
    out.extend_from_slice(b);
    out
}

/// A signature body of `version` and `kind` over SHA-512, signed with
/// `seed` over `prefix`, its fields and trailer, as RFC 9580 §5.2.4 has
/// it for version 4.
fn signature(
    seed: u8,
    version: u8,
    kind: u8,
    prefix: &[u8],
    hashed: &[u8],
    unhashed: &[u8],
) -> Vec<u8> {
    let mut fields = vec![version, kind, 22, 10];
    fields.extend_from_slice(&(hashed.len() as u16).to_be_bytes());
    fields.extend_from_slice(hashed);
    let mut data = prefix.to_vec();
    data.extend_from_slice(&fields);
    data.extend_from_slice(&[0x04, 0xff]);
    data.extend_from_slice(&(fields.len() as u32).to_be_bytes());
    let digest = Sha512::digest(&data);
    let sig = SigningKey::from_bytes(&[seed; 32]).sign(&digest).to_bytes();
    let mut body = fields;
    body.extend_from_slice(&(unhashed.len() as u16).to_be_bytes());
    body.extend_from_slice(unhashed);
    body.extend_from_slice(&digest[..2]);
    body.extend(mpi(&sig[..32]));
    body.extend(mpi(&sig[32..]));
    body
}

fn issuer_fp(fp: &[u8; 20]) -> Vec<u8> {
    let mut d = vec![4];
    d.extend_from_slice(fp);
    sub(33, &d)
}

/// The test key's certificate, read.
fn cert_of(k: &Key) -> Certificate {
    Certificate::read(armor(Armor::PublicKey, &k.certificate(MADE)).as_bytes()).expect("reads")
}

/// A detached signature over `DATA` by key 0x31/0x32's subkey, with the
/// hashed and unhashed areas given, of `kind` and `version`.
fn made(version: u8, kind: u8, hashed: &[u8], unhashed: &[u8]) -> String {
    armor(
        Armor::Signature,
        &packet(2, &signature(0x32, version, kind, DATA, hashed, unhashed)),
    )
}

/// The areas Faraday writes: the time and the issuer's fingerprint
/// hashed, the issuer's key ID not.
fn areas() -> (Vec<u8>, Vec<u8>) {
    let fp = key(0x31, 0x32).subkey_fingerprint();
    let mut hashed = sub(2, &MADE.to_be_bytes());
    hashed.extend(issuer_fp(&fp));
    (hashed, sub(16, &fp[12..]))
}

fn verdict(asc: &str) -> Result<u32, Unverified> {
    verify_detached(&cert_of(&key(0x31, 0x32)), DATA, asc.as_bytes())
}

#[test]
fn a_signature_made_here_of_faradays_shape_verifies() {
    // What every refusal below changes one thing of.
    let (hashed, unhashed) = areas();
    assert_eq!(verdict(&made(4, 0x00, &hashed, &unhashed)), Ok(MADE));
    // An issuer marked critical names the key all the same.
    let fp = key(0x31, 0x32).subkey_fingerprint();
    let mut critical = sub(2, &MADE.to_be_bytes());
    let mut d = vec![4];
    d.extend_from_slice(&fp);
    critical.extend(sub(0x80 | 33, &d));
    assert_eq!(verdict(&made(4, 0x00, &critical, &unhashed)), Ok(MADE));
}

#[test]
fn a_text_mode_signature_is_refused() {
    let (hashed, unhashed) = areas();
    assert_eq!(
        verdict(&made(4, 0x01, &hashed, &unhashed)),
        Err(Unverified::NotSignature)
    );
    // And GnuPG's, over the fixture's sums.
    let release = Certificate::read(&read("release-key.asc")).expect("reads");
    assert_eq!(
        verify_detached(
            &release,
            &read("SHA256SUMS"),
            &read("SHA256SUMS.textmode.asc")
        ),
        Err(Unverified::NotSignature)
    );
}

#[test]
fn version_5_and_6_signatures_are_refused() {
    let (hashed, unhashed) = areas();
    for version in [5, 6] {
        assert_eq!(
            verdict(&made(version, 0x00, &hashed, &unhashed)),
            Err(Unverified::NotSignature),
            "{version}"
        );
    }
}

#[test]
fn a_hashed_critical_subpacket_not_understood_is_refused() {
    let (mut hashed, unhashed) = areas();
    hashed.extend(sub(0x80 | 101, b"x"));
    assert_eq!(
        verdict(&made(4, 0x00, &hashed, &unhashed)),
        Err(Unverified::NotSignature)
    );
}

#[test]
fn a_hashed_signature_expiry_is_refused() {
    let (mut hashed, unhashed) = areas();
    hashed.extend(sub(3, &1000u32.to_be_bytes()));
    assert_eq!(
        verdict(&made(4, 0x00, &hashed, &unhashed)),
        Err(Unverified::NotSignature)
    );
}

#[test]
fn a_creation_time_only_in_the_unhashed_area_is_refused() {
    let fp = key(0x31, 0x32).subkey_fingerprint();
    let hashed = issuer_fp(&fp);
    let mut unhashed = sub(2, &MADE.to_be_bytes());
    unhashed.extend(sub(16, &fp[12..]));
    assert_eq!(
        verdict(&made(4, 0x00, &hashed, &unhashed)),
        Err(Unverified::NotSignature)
    );
}

#[test]
fn a_wrong_16_bit_hash_prefix_is_refused() {
    let (hashed, unhashed) = areas();
    let mut body = signature(0x32, 4, 0x00, DATA, &hashed, &unhashed);
    let at = 6 + hashed.len() + 2 + unhashed.len();
    body[at] ^= 0xff;
    assert_eq!(
        verdict(&armor(Armor::Signature, &packet(2, &body))),
        Err(Unverified::Bad)
    );
}

/// The fixture's signature by the release key, as text.
fn gnupg_asc() -> String {
    String::from_utf8(read("SHA256SUMS.asc")).expect("text")
}

fn release_verdict(asc: &str) -> Result<u32, Unverified> {
    let release = Certificate::read(&read("release-key.asc")).expect("reads");
    verify_detached(&release, &read("SHA256SUMS"), asc.as_bytes())
}

#[test]
fn armour_without_its_checksum_or_with_a_wrong_one_is_refused() {
    let asc = gnupg_asc();
    assert!(release_verdict(&asc).is_ok());
    let without: String = asc
        .lines()
        .filter(|l| !(l.len() == 5 && l.starts_with('=')))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(release_verdict(&without), Err(Unverified::NotSignature));
    let crc = asc
        .lines()
        .find(|l| l.len() == 5 && l.starts_with('='))
        .expect("a checksum");
    let other = if crc.ends_with('A') { "B" } else { "A" };
    let wrong = asc.replace(crc, &format!("{}{other}", &crc[..4]));
    assert_eq!(release_verdict(&wrong), Err(Unverified::NotSignature));
}

#[test]
fn anything_after_the_armour_or_a_second_block_is_refused() {
    let asc = gnupg_asc();
    assert_eq!(
        release_verdict(&format!("{asc}trailing\n")),
        Err(Unverified::NotSignature)
    );
    assert_eq!(
        release_verdict(&format!("{asc}{asc}")),
        Err(Unverified::NotSignature)
    );
}

#[test]
fn another_keys_signature_with_its_issuer_rewritten_to_the_release_subkey_is_bad() {
    let release = Certificate::read(&read("release-key.asc")).expect("reads");
    let other = Certificate::read(&read("other-key.asc")).expect("reads");
    let text = String::from_utf8(read("SHA256SUMS.other.asc")).expect("text");
    let body: String = text
        .lines()
        .skip(2)
        .take_while(|l| !l.starts_with('=') && !l.starts_with("-----"))
        .collect();
    let mut bytes = base64_decode(&body);
    let (from, to) = (other.subkey_fingerprint(), release.subkey_fingerprint());
    let mut rewritten = 0;
    let mut at = 0;
    while at + 20 <= bytes.len() {
        if bytes[at..at + 20] == from {
            bytes[at..at + 20].copy_from_slice(&to);
            rewritten += 1;
        }
        if bytes[at..at + 8] == from[12..] && bytes[at.saturating_sub(1)] == 16 {
            bytes[at..at + 8].copy_from_slice(&to[12..]);
            rewritten += 1;
        }
        at += 1;
    }
    assert!(rewritten >= 1, "the issuer is in the signature");
    let asc = armor(Armor::Signature, &bytes);
    assert_eq!(
        verify_detached(&release, &read("SHA256SUMS"), asc.as_bytes()),
        Err(Unverified::Bad)
    );
}

/// Key A's primary key and user ID with key B's subkey and a binding of
/// `binding`'s making.
fn cross(binding: impl Fn(&[u8], &[u8], &[u8; 20]) -> Vec<u8>) -> Certificate {
    let a = packets(&key(0x31, 0x32).certificate(MADE));
    let b = packets(&key(0x41, 0x42).certificate(MADE));
    let primary = &a[0].1;
    let subkey = &b.iter().find(|(t, _)| *t == 14).expect("a subkey").1;
    let fp = key(0x31, 0x32).fingerprint();
    let mut out = Vec::new();
    for (t, body) in a.iter().take_while(|(t, _)| *t != 14) {
        out.extend(packet(*t, body));
    }
    out.extend(packet(14, subkey));
    out.extend(packet(2, &binding(primary, subkey, &fp)));
    Certificate::read(armor(Armor::PublicKey, &out).as_bytes()).expect("it reads")
}

/// What a binding covers: both keys, as keys are hashed.
fn keys_hashed(primary: &[u8], subkey: &[u8]) -> Vec<u8> {
    let mut out = vec![0x99];
    out.extend_from_slice(&(primary.len() as u16).to_be_bytes());
    out.extend_from_slice(primary);
    out.push(0x99);
    out.extend_from_slice(&(subkey.len() as u16).to_be_bytes());
    out.extend_from_slice(subkey);
    out
}

/// A subkey binding with key flags 0x02 and the issuer `fp`, signed with
/// `seed`, embedding `back` (a back signature) when there is one.
fn binding(
    seed: u8,
    primary: &[u8],
    subkey: &[u8],
    fp: &[u8; 20],
    back: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut hashed = sub(2, &MADE.to_be_bytes());
    hashed.extend(sub(27, &[0x02]));
    hashed.extend(issuer_fp(fp));
    if let Some(back) = back {
        hashed.extend([0xff]);
        hashed.extend(((back.len() + 1) as u32).to_be_bytes());
        hashed.push(32);
        hashed.extend(back);
    }
    signature(seed, 4, 0x18, &keys_hashed(primary, subkey), &hashed, &[])
}

/// Key B's subkey's back signature over A's primary key and itself.
fn back(primary: &[u8], subkey: &[u8]) -> Vec<u8> {
    signature(
        0x42,
        4,
        0x19,
        &keys_hashed(primary, subkey),
        &sub(2, &MADE.to_be_bytes()),
        &[],
    )
}

/// A signature over `DATA` by key B's subkey, naming it.
fn by_b_subkey() -> String {
    let fp = key(0x41, 0x42).subkey_fingerprint();
    let mut hashed = sub(2, &MADE.to_be_bytes());
    hashed.extend(issuer_fp(&fp));
    armor(
        Armor::Signature,
        &packet(2, &signature(0x42, 4, 0x00, DATA, &hashed, &[])),
    )
}

#[test]
fn a_subkey_bound_by_another_primary_key_verifies_nothing() {
    // Key A's certificate carrying key B's subkey, its binding naming A
    // but made by B's primary key: A never bound it.
    let cert = cross(|p, s, fp| binding(0x41, p, s, fp, Some(back(p, s))));
    assert_eq!(
        verify_detached(&cert, DATA, by_b_subkey().as_bytes()),
        Err(Unverified::Bad)
    );
    // The same made by A's primary key verifies: the check is the
    // binding's, nothing else.
    let cert = cross(|p, s, fp| binding(0x31, p, s, fp, Some(back(p, s))));
    assert_eq!(
        verify_detached(&cert, DATA, by_b_subkey().as_bytes()),
        Ok(MADE)
    );
}

#[test]
fn a_signing_subkey_without_its_back_signature_or_with_a_wrong_one_verifies_nothing() {
    let cert = cross(|p, s, fp| binding(0x31, p, s, fp, None));
    assert_eq!(
        verify_detached(&cert, DATA, by_b_subkey().as_bytes()),
        Err(Unverified::Bad)
    );
    // A back signature by another key than the subkey.
    let cert = cross(|p, s, fp| {
        let wrong = signature(
            0x77,
            4,
            0x19,
            &keys_hashed(p, s),
            &sub(2, &MADE.to_be_bytes()),
            &[],
        );
        binding(0x31, p, s, fp, Some(wrong))
    });
    assert_eq!(
        verify_detached(&cert, DATA, by_b_subkey().as_bytes()),
        Err(Unverified::Bad)
    );
}

#[test]
fn a_gnupg_certificate_carries_its_back_signature_unhashed_and_verifies() {
    // GnuPG writes the back signature in the binding's unhashed area.
    let release = Certificate::read(&read("release-key.asc")).expect("reads");
    assert!(verify_detached(&release, &read("SHA256SUMS"), &read("SHA256SUMS.asc")).is_ok());
}

/// The test's own SHA-512-signed Ed25519 key, for `Hash` to be used.
#[test]
fn faradays_own_signature_still_verifies_with_the_back_signature_checked() {
    let k = key(0x31, 0x32);
    let asc = armor(Armor::Signature, &k.sign(DATA, MADE, Hash::Sha512));
    assert_eq!(
        verify_detached(&cert_of(&k), DATA, asc.as_bytes()),
        Ok(MADE)
    );
}

fn base64_decode(text: &str) -> Vec<u8> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for c in text.bytes().filter(|&c| c != b'=') {
        let v = ALPHABET.iter().position(|&a| a == c).expect("base64") as u32;
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}
