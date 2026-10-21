//! `read_public_armor` (`docs/history/PLAN.md` §4.4, F22, F32): a file that is one
//! armoured block of public OpenPGP packets, each read to its exact end,
//! reads as a public key or a signature, and anything else does not.

use faraday_pgp::{
    Armor, Hash, Key, Reason, armor, armor_with, crc24, read_block, read_public, read_public_armor,
};

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

fn revocation_packets() -> Vec<u8> {
    key().revocation(MADE + 60, Reason::Retired, "Not used")
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

/// The packets of `data` as (tag, start, end), for new-format headers.
fn packets(data: &[u8]) -> Vec<(u8, usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let tag = data[at] & 0x3f;
        let first = data[at + 1] as usize;
        let (len, head) = if first < 192 {
            (first, 2)
        } else {
            (((first - 192) << 8) + data[at + 2] as usize + 192, 3)
        };
        out.push((tag, at, at + head + len));
        at += head + len;
    }
    out
}

/// A packet with a new-format header.
fn packet(tag: u8, body: &[u8]) -> Vec<u8> {
    assert!(body.len() < 192);
    let mut out = vec![0xC0 | tag, body.len() as u8];
    out.extend_from_slice(body);
    out
}

#[test]
fn the_certificate_and_the_signature_read_as_public() {
    assert_eq!(read(&certificate()), Some(Armor::PublicKey));
    assert_eq!(read(&signature()), Some(Armor::Signature));
    let block = read_public(certificate().as_bytes()).unwrap();
    assert_eq!(block.tags, vec![6, 13, 2, 14, 2]);
    assert_eq!(block.texts, vec!["Test Person <test@example.com>"]);
}

#[test]
fn a_revocation_certificate_is_not_public() {
    // Whoever has it can revoke the key (F32): in a key block it holds no
    // public key packet to start with, and the file GnuPG lays out around
    // it has a colon before its first dashes.
    let rev = armor(Armor::PublicKey, &revocation_packets());
    assert_eq!(read(&rev), None);
    let file = key().revocation_file(&revocation_packets());
    assert_eq!(read(&file), None);
    let guarded = format!(":{}", armor(Armor::PublicKey, &key().certificate(MADE)));
    assert_eq!(read(&guarded), None);
    assert_eq!(read(&guarded.replacen(':', "", 1)), Some(Armor::PublicKey));
}

#[test]
fn crlf_line_ends_and_blank_lines_around_the_block_read() {
    for text in [certificate(), signature()] {
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
fn the_headers_gnupg_knows_read_and_others_do_not() {
    for header in [
        "Version: GnuPG v2",
        "Comment: made elsewhere",
        "Hash: SHA512",
        "Charset: utf-8",
        "MessageID: abcdefghijklmnopqrstuvwxyz012345",
    ] {
        let text = certificate().replacen("BLOCK-----\n", &format!("BLOCK-----\n{header}\n"), 1);
        assert_eq!(read(&text), Some(Armor::PublicKey), "{header}");
    }
    let text = certificate().replacen("BLOCK-----\n", "BLOCK-----\nNotes: x\n", 1);
    assert_eq!(read(&text), None);
    let text = certificate().replacen("BLOCK-----\n", "BLOCK-----\nComment:no space\n", 1);
    assert_eq!(read(&text), None);
}

#[test]
fn header_values_are_handed_back_to_be_checked_for_secrets() {
    let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let text = armor_with(Armor::PublicKey, &key().certificate(MADE), Some(words));
    let block = read_public(text.as_bytes()).unwrap();
    assert!(block.texts.iter().any(|t| t == words));
}

#[test]
fn a_missing_or_wrong_checksum_is_read_but_not_taken_for_public() {
    let data = key().certificate(MADE);
    let without: String = certificate()
        .lines()
        .filter(|l| !l.starts_with('='))
        .map(|l| format!("{l}\n"))
        .collect();
    let (block, crc_ok) = read_block(without.as_bytes()).unwrap();
    assert_eq!(block.kind, Armor::PublicKey);
    assert!(!crc_ok);
    assert_eq!(read(&without), None);

    let good = armor(Armor::PublicKey, &data);
    let bad = good.replace(
        &format!("\n={}\n", &good.lines().rev().nth(1).unwrap()[1..]),
        "\n=AAAA\n",
    );
    assert_ne!(crc24(&data), 0);
    assert_ne!(good, bad);
    let (_, crc_ok) = read_block(bad.as_bytes()).unwrap();
    assert!(!crc_ok);
    assert_eq!(read(&bad), None);
}

#[test]
fn a_private_key_block_does_not_read() {
    let text = certificate().replace("PUBLIC KEY BLOCK", "PRIVATE KEY BLOCK");
    assert_eq!(read(&text), None);
}

#[test]
fn a_secret_key_packet_inside_a_public_block_does_not_read() {
    let secret = key().secret_key(MADE);
    assert_eq!(read(&armor(Armor::PublicKey, &secret)), None);
}

#[test]
fn a_secret_packet_relabelled_as_public_does_not_read() {
    // Tag 5 changed to 6 and tag 7 to 14: the body still holds the secret
    // part after the public key, so it does not end where a public key's
    // does.
    let mut data = key().secret_key(MADE).to_vec();
    for (tag, start, _) in packets(&data) {
        match tag {
            5 => data[start] = 0xC0 | 6,
            7 => data[start] = 0xC0 | 14,
            _ => {}
        }
    }
    assert_eq!(read(&armor(Armor::PublicKey, &data)), None);
    // Only the subkey relabelled.
    let mut data = key().secret_key(MADE).to_vec();
    let pk = key().certificate(MADE);
    let (_, _, end) = packets(&pk)[0];
    data.splice(0..packets(&data)[0].2, pk[..end].iter().copied());
    for (tag, start, _) in packets(&data) {
        if tag == 7 {
            data[start] = 0xC0 | 14;
        }
    }
    assert_eq!(read(&armor(Armor::PublicKey, &data)), None);
}

#[test]
fn a_public_key_with_a_byte_more_or_less_does_not_read() {
    let data = key().certificate(MADE);
    let (tag, start, end) = packets(&data)[0];
    assert_eq!(tag, 6);
    let body = &data[start + 2..end];
    for changed in [
        [body, &[0u8][..]].concat(),
        body[..body.len() - 1].to_vec(),
        [&[3u8][..], &body[1..]].concat(),
    ] {
        let mut out = packet(6, &changed);
        out.extend_from_slice(&data[end..]);
        assert_eq!(read(&armor(Armor::PublicKey, &out)), None);
    }
}

#[test]
fn a_user_attribute_a_trust_packet_and_other_tags_do_not_read() {
    let data = key().certificate(MADE);
    for tag in [17, 12, 10, 1, 8, 9] {
        let mut out = data.clone();
        out.extend(packet(tag, b"hidden"));
        assert_eq!(read(&armor(Armor::PublicKey, &out)), None, "tag {tag}");
    }
}

#[test]
fn a_user_id_is_text() {
    let data = key().certificate(MADE);
    let (_, _, end) = packets(&data)[0];
    let mut out = data[..end].to_vec();
    out.extend(packet(13, &[0xFF, 0xFE, 0x80]));
    assert_eq!(read(&armor(Armor::PublicKey, &out)), None);
}

#[test]
fn a_key_block_starts_with_a_public_key() {
    let data = key().certificate(MADE);
    let (_, _, end) = packets(&data)[0];
    // The user ID and the rest, without the key.
    assert_eq!(read(&armor(Armor::PublicKey, &data[end..])), None);
    // A key and a signature made by it, alone, read.
    let mut out = data[..end].to_vec();
    out.extend_from_slice(&revocation_packets());
    assert_eq!(read(&armor(Armor::PublicKey, &out)), Some(Armor::PublicKey));
}

#[test]
fn a_signature_with_a_byte_more_does_not_read() {
    let sig = key().sign(b"x", MADE + 60, Hash::Sha256);
    let (_, start, end) = packets(&sig)[0];
    let mut body = sig[start + 2..end].to_vec();
    body.push(0);
    assert_eq!(read(&armor(Armor::Signature, &packet(2, &body))), None);
    let short = &sig[start + 2..end - 1];
    assert_eq!(read(&armor(Armor::Signature, &packet(2, short))), None);
}

#[test]
fn text_around_the_block_does_not_read() {
    assert_eq!(read(&format!("My key:\n{}", certificate())), None);
    assert_eq!(read(&format!("{}Thanks\n", certificate())), None);
    assert_eq!(read(&format!("{}{}", certificate(), signature())), None);
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
}

#[test]
fn old_format_headers_with_a_length_given_read() {
    let data = key().certificate(MADE);
    let (_, start, end) = packets(&data)[0];
    let body = &data[start + 2..end];
    // Tag 6, two-octet length: 0x80 | 6 << 2 | 1.
    let mut out = vec![0x80 | (6 << 2) | 1, 0, body.len() as u8];
    out.extend_from_slice(body);
    out.extend_from_slice(&data[end..]);
    assert_eq!(read(&armor(Armor::PublicKey, &out)), Some(Armor::PublicKey));
}

#[test]
fn bytes_that_are_not_text_do_not_read() {
    assert_eq!(read_public_armor(&[]), None);
    assert_eq!(read_public_armor(&[0xFF; 64]), None);
    let mut bytes = certificate().into_bytes();
    bytes[40] = 0xC3;
    assert_eq!(read_public_armor(&bytes), None);
}
