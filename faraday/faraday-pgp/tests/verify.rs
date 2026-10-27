//! Detached signatures checked against a certificate
//! (`faraday_pgp::verify_detached`, `docs/decisions/F134.md`), against
//! what GnuPG made: the fixtures in `tools/vectors/release-sums/` were
//! made by GnuPG 2.4 (`make.sh` there, its commands in its README), and
//! `faraday-0.1.7/` is Faraday 0.1.7's published `SHA256SUMS` and its
//! signature by the release key.

use std::path::{Path, PathBuf};

use faraday_pgp::{Certificate, Hash, Key, Unverified, armor, verify_detached};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/release-sums")
}

fn read(name: &str) -> Vec<u8> {
    std::fs::read(dir().join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn cert(name: &str) -> Certificate {
    Certificate::read(&read(name)).expect("a certificate of Faraday's shape")
}

/// When make.sh's clock said the signatures were made.
const MADE: u32 = 1_780_000_000;

#[test]
fn a_gnupg_signature_over_sha512_and_over_sha256_verifies() {
    let release = cert("release-key.asc");
    let sums = read("SHA256SUMS");
    for asc in ["SHA256SUMS.asc", "SHA256SUMS.sha256.asc"] {
        assert_eq!(
            verify_detached(&release, &sums, &read(asc)),
            Ok(MADE),
            "{asc}"
        );
    }
}

#[test]
fn faraday_0_1_7s_sums_verify_with_the_release_key_in_the_repository() {
    let key = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../release-key.asc"))
        .expect("faraday/release-key.asc");
    let release = Certificate::read(&key).expect("the release key");
    let sums = read("faraday-0.1.7/SHA256SUMS");
    let asc = read("faraday-0.1.7/SHA256SUMS.asc");
    // 2026-10-25, as `gpg --verify` reports it.
    assert_eq!(verify_detached(&release, &sums, &asc), Ok(1_792_921_584));
    // The same signature over the test release's sums does not.
    assert_eq!(
        verify_detached(&release, &read("SHA256SUMS"), &asc),
        Err(Unverified::Bad)
    );
}

#[test]
fn a_signature_by_another_key_is_refused_as_another_keys() {
    let release = cert("release-key.asc");
    let sums = read("SHA256SUMS");
    assert_eq!(
        verify_detached(&release, &sums, &read("SHA256SUMS.other.asc")),
        Err(Unverified::OtherKey)
    );
    // And the other key's own certificate takes it.
    assert_eq!(
        verify_detached(&cert("other-key.asc"), &sums, &read("SHA256SUMS.other.asc")),
        Ok(MADE)
    );
}

#[test]
fn sums_changed_by_one_byte_or_one_line_are_refused() {
    let release = cert("release-key.asc");
    let asc = read("SHA256SUMS.asc");
    let sums = read("SHA256SUMS");
    let mut flipped = sums.clone();
    flipped[10] ^= 1;
    assert_eq!(
        verify_detached(&release, &flipped, &asc),
        Err(Unverified::Bad)
    );
    let mut added = sums.clone();
    added.extend_from_slice(format!("{}  BOOTX64.EFI\n", "ab".repeat(32)).as_bytes());
    assert_eq!(
        verify_detached(&release, &added, &asc),
        Err(Unverified::Bad)
    );
}

#[test]
fn what_is_not_one_signature_is_refused() {
    let release = cert("release-key.asc");
    let sums = read("SHA256SUMS");
    for not in [
        read("release-key.asc"),
        sums.clone(),
        Vec::new(),
        // Two signatures in one block.
        {
            let one = read("SHA256SUMS.asc");
            let text = String::from_utf8(one).expect("text");
            let body: String = text
                .lines()
                .skip(2)
                .take_while(|l| !l.starts_with('='))
                .collect();
            let bytes = base64_decode(&body);
            let mut two = bytes.clone();
            two.extend_from_slice(&bytes);
            armor(faraday_pgp::Armor::Signature, &two).into_bytes()
        },
    ] {
        assert_eq!(
            verify_detached(&release, &sums, &not),
            Err(Unverified::NotSignature)
        );
    }
}

fn faraday_key(expiry: u32) -> Key {
    Key {
        primary_seed: [0x31; 32],
        created: MADE,
        subkey_seed: [0x32; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Release <release@example.com>".to_string()],
        expiry,
    }
}

#[test]
fn a_signature_faraday_makes_verifies_within_the_keys_life_only() {
    let year = 31_536_000;
    let key = faraday_key(2 * year);
    let cert =
        Certificate::read(armor(faraday_pgp::Armor::PublicKey, &key.certificate(MADE)).as_bytes())
            .expect("the certificate");
    let data = b"# Faraday 0.9.0, commit 0\n";
    for hash in [Hash::Sha256, Hash::Sha512] {
        let at = MADE + year;
        let asc = armor(faraday_pgp::Armor::Signature, &key.sign(data, at, hash));
        assert_eq!(verify_detached(&cert, data, asc.as_bytes()), Ok(at));
    }
    // Made after the key expired, or before it was made.
    for at in [MADE + 2 * year, MADE - 1] {
        let asc = armor(
            faraday_pgp::Armor::Signature,
            &key.sign(data, at, Hash::Sha512),
        );
        assert_eq!(
            verify_detached(&cert, data, asc.as_bytes()),
            Err(Unverified::OutOfTime),
            "{at}"
        );
    }
}

#[test]
fn a_revoked_key_verifies_nothing() {
    let key = faraday_key(0);
    let mut packets = key.certificate(MADE);
    packets.extend(key.revocation(MADE + 10, faraday_pgp::Reason::Superseded, ""));
    let Ok(cert) = Certificate::read(armor(faraday_pgp::Armor::PublicKey, &packets).as_bytes())
    else {
        panic!("a revoked certificate still reads");
    };
    let data = b"sums";
    let asc = armor(
        faraday_pgp::Armor::Signature,
        &key.sign(data, MADE + 5, Hash::Sha512),
    );
    assert_eq!(
        verify_detached(&cert, data, asc.as_bytes()),
        Err(Unverified::Revoked)
    );
}

#[test]
fn a_binding_signature_changed_by_one_byte_verifies_nothing() {
    // The certificate with its binding signature's last byte changed: the
    // binding no longer verifies. (A subkey bound by another primary key
    // is in verify_refusals.rs.)
    let key = faraday_key(0);
    let packets = key.certificate(MADE);
    // The binding is the last packet but the back signature inside it;
    // change the last byte of the packets, which is in its signature.
    let mut broken = packets.clone();
    let n = broken.len();
    broken[n - 1] ^= 0x01;
    let cert = Certificate::read(armor(faraday_pgp::Armor::PublicKey, &broken).as_bytes())
        .expect("it still reads");
    let data = b"sums";
    let asc = armor(
        faraday_pgp::Armor::Signature,
        &key.sign(data, MADE, Hash::Sha512),
    );
    assert_eq!(
        verify_detached(&cert, data, asc.as_bytes()),
        Err(Unverified::Bad)
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
