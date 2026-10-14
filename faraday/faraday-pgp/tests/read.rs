//! `read_public_armor` (`PLAN.md` §4.4): a file that is one armoured block
//! of public OpenPGP packets reads as a public key or a signature, and
//! anything else does not.

use faraday_pgp::{Armor, Hash, Key, Reason, armor, crc24, read_public_armor};

const MADE: u32 = 1_780_000_000;

fn key() -> Key {
    Key {
        primary_seed: [0x11; 32],
        created: MADE,
        subkey_seed: [0x22; 32],
        subkey_created: MADE,
        user_ids: vec!["Test Person <test@example.com>".to_string()],
        expiry: 0,
    }
}

fn certificate() -> String {
    armor(Armor::PublicKey, &key().certificate(MADE))
}

fn revocation() -> String {
    armor(
        Armor::PublicKey,
        &key().revocation(MADE + 60, Reason::Retired, "Not used"),
    )
}

fn signature() -> String {
    armor(
        Armor::Signature,
        &key().sign(b"A file Faraday signs.\n", MADE + 60, Hash::Sha256),
    )
}

fn read(text: &str) -> Option<Armor> {
    read_public_armor(text.as_bytes())
}

#[test]
fn the_certificate_the_revocation_and_the_signature_read_as_public() {
    assert_eq!(read(&certificate()), Some(Armor::PublicKey));
    assert_eq!(read(&revocation()), Some(Armor::PublicKey));
    assert_eq!(read(&signature()), Some(Armor::Signature));
}

#[test]
fn crlf_line_ends_and_blank_lines_around_the_block_read() {
    for text in [certificate(), revocation(), signature()] {
        let expect = read(&text);
        assert!(expect.is_some());
        assert_eq!(read(&text.replace('\n', "\r\n")), expect);
        assert_eq!(read(&format!("\n\n{text}\n\n")), expect);
        assert_eq!(
            read(&format!("\r\n{}\r\n\r\n", text.replace('\n', "\r\n"))),
            expect
        );
    }
}

#[test]
fn armour_headers_and_a_missing_checksum_read() {
    let text = certificate().replacen(
        "BLOCK-----\n",
        "BLOCK-----\nVersion: GnuPG v2\nComment: made elsewhere\n",
        1,
    );
    assert_eq!(read(&text), Some(Armor::PublicKey));
    let without: String = certificate()
        .lines()
        .filter(|l| !l.starts_with('='))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(read(&without), Some(Armor::PublicKey));
}

#[test]
fn a_private_key_block_does_not_read() {
    let text = certificate().replace("PUBLIC KEY BLOCK", "PRIVATE KEY BLOCK");
    assert_eq!(read(&text), None);
}

#[test]
fn a_secret_key_packet_inside_a_public_block_does_not_read() {
    let k = key();
    let secret = k.secret_key(MADE);
    assert_eq!(read(&armor(Armor::PublicKey, &secret)), None);
}

#[test]
fn text_around_the_block_does_not_read() {
    assert_eq!(read(&format!("My key:\n{}", certificate())), None);
    assert_eq!(read(&format!("{}Thanks\n", certificate())), None);
    assert_eq!(read(&format!("{}{}", certificate(), signature())), None);
}

#[test]
fn a_wrong_checksum_does_not_read() {
    let data = key().certificate(MADE);
    let good = armor(Armor::PublicKey, &data);
    let sum = crc24(&data).to_be_bytes();
    let bad = armor(Armor::PublicKey, &data).replace(
        &format!("\n={}\n", &good.lines().rev().nth(1).unwrap()[1..]),
        "\n=AAAA\n",
    );
    assert_ne!(sum, [0, 0, 0, 0]);
    assert_ne!(good, bad);
    assert_eq!(read(&bad), None);
}

#[test]
fn a_truncated_packet_does_not_read() {
    let data = key().certificate(MADE);
    assert_eq!(
        read(&armor(Armor::PublicKey, &data[..data.len() - 1])),
        None
    );
    assert_eq!(read(&armor(Armor::PublicKey, &data[..1])), None);
    assert_eq!(read(&armor(Armor::PublicKey, &[])), None);
}

#[test]
fn a_signature_block_holding_a_public_key_does_not_read() {
    let data = key().certificate(MADE);
    assert_eq!(read(&armor(Armor::Signature, &data)), None);
}

#[test]
fn lengths_that_are_not_given_do_not_read() {
    // A partial body length (new format) and an indeterminate length
    // (old format).
    assert_eq!(read(&armor(Armor::PublicKey, &[0xC6, 0xE0, 1, 2])), None);
    assert_eq!(read(&armor(Armor::PublicKey, &[0x9B, 1, 2])), None);
    // Old-format headers with a length given read.
    assert_eq!(
        read(&armor(Armor::PublicKey, &[0x98, 2, 1, 2])),
        Some(Armor::PublicKey)
    );
    assert_eq!(
        read(&armor(Armor::PublicKey, &[0x99, 0, 2, 1, 2])),
        Some(Armor::PublicKey)
    );
}

#[test]
fn bytes_that_are_not_text_do_not_read() {
    assert_eq!(read_public_armor(&[]), None);
    assert_eq!(read_public_armor(&[0xFF; 64]), None);
    let mut bytes = certificate().into_bytes();
    bytes[40] = 0xC3;
    assert_eq!(read_public_armor(&bytes), None);
}
