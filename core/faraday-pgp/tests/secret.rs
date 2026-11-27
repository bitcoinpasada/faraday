//! The armoured secret key Faraday keeps in a KDBX vault's GPG entry
//! (`docs/VAULT.md` §7): what `armor_secret` writes reads back as the same
//! key, and anything else (another key's packets, a passphrase on the key,
//! a changed byte, an added packet, a block that is not a secret key)
//! does not read.
#![allow(clippy::disallowed_methods)]

use faraday_pgp::{Armor, Hash, Key, armor, armor_secret, read_secret, read_secret_at};

const MADE: u32 = 1_780_000_000;
const YEAR: u32 = 31_536_000;

fn key() -> Key {
    Key {
        primary_seed: [0x11; 32],
        created: MADE,
        subkey_seed: [0x22; 32],
        subkey_created: MADE + 60,
        user_ids: vec![faraday_text::Text::of("Test Person <test@example.com>")],
        expiry: 2 * YEAR,
    }
}

fn same(a: &Key, b: &Key) -> bool {
    a.primary_seed == b.primary_seed
        && a.created == b.created
        && a.subkey_seed == b.subkey_seed
        && a.subkey_created == b.subkey_created
        && a.user_ids == b.user_ids
        && a.expiry == b.expiry
}

/// The text of a private key block holding `packets`.
fn private_block(packets: &[u8]) -> String {
    armor(Armor::PublicKey, packets).replace("PUBLIC KEY BLOCK", "PRIVATE KEY BLOCK")
}

fn text_of(k: &Key, at: u32) -> String {
    String::from_utf8(armor_secret(k, at).to_vec()).unwrap()
}

#[test]
fn a_key_reads_back_as_written() {
    let mut zero_seeds = key();
    zero_seeds.primary_seed[0] = 0;
    zero_seeds.subkey_seed[0] = 0;
    zero_seeds.subkey_seed[1] = 0;
    let mut never = key();
    never.expiry = 0;
    let mut two_ids = key();
    two_ids
        .user_ids
        .push(faraday_text::Text::of("Second <second@example.com>"));
    let mut long_id = key();
    long_id.user_ids = vec![faraday_text::Text::of(&"x".repeat(300))];
    for k in [key(), zero_seeds, never, two_ids, long_id] {
        let at = MADE + 3600;
        let text = armor_secret(&k, at);
        let (back, back_at) = read_secret_at(&text).expect("reads");
        assert!(same(&back, &k));
        assert_eq!(back_at, at);
        assert_eq!(*back.secret_key(at), *k.secret_key(at));
    }
}

#[test]
fn the_block_is_gnupgs_private_key_block() {
    let text = text_of(&key(), MADE);
    assert!(text.starts_with("-----BEGIN PGP PRIVATE KEY BLOCK-----\n\n"));
    assert!(text.ends_with("\n-----END PGP PRIVATE KEY BLOCK-----\n"));
    let crc = text.lines().rev().nth(1).unwrap();
    assert!(crc.starts_with('=') && crc.len() == 5);
    assert!(
        text.lines()
            .all(|l| l.len() <= 64 || l.starts_with("-----"))
    );
}

#[test]
fn line_ends_and_headers_do_not_matter() {
    let text = text_of(&key(), MADE);
    assert!(read_secret(text.replace('\n', "\r\n").as_bytes()).is_some());
    let headed = text.replacen("\n\n", "\nComment: kept\n\n", 1);
    assert!(read_secret(headed.as_bytes()).is_some());
    assert!(read_secret(format!("\n{text}\n\n").as_bytes()).is_some());
}

#[test]
fn what_is_not_exactly_the_key_faraday_writes_is_not_read() {
    let k = key();
    let packets = k.secret_key(MADE).to_vec();
    assert!(read_secret(private_block(&packets).as_bytes()).is_some());

    // A byte of a seed changed: the signatures no longer match the key.
    let mut bad = packets.clone();
    bad[2 + 1 + 4 + 1 + 10 + 2 + 33 + 1 + 2 + 5] ^= 1;
    assert!(read_secret(private_block(&bad).as_bytes()).is_none());

    // A byte of a signature changed.
    let mut bad = packets.clone();
    let last = bad.len() - 10;
    bad[last] ^= 1;
    assert!(read_secret(private_block(&bad).as_bytes()).is_none());

    // Packets cut short, or more after them.
    assert!(read_secret(private_block(&packets[..packets.len() - 1]).as_bytes()).is_none());
    let mut more = packets.clone();
    more.extend_from_slice(&packets[packets.len() - 20..]);
    assert!(read_secret(private_block(&more).as_bytes()).is_none());
    let mut twice = packets.clone();
    twice.extend_from_slice(&packets);
    assert!(read_secret(private_block(&twice).as_bytes()).is_none());

    // The public certificate or a signature in a private block, a public
    // block, nothing.
    assert!(read_secret(private_block(&k.certificate(MADE)).as_bytes()).is_none());
    let sig = k.sign(b"x", MADE, Hash::Sha256);
    assert!(read_secret(private_block(&sig).as_bytes()).is_none());
    assert!(read_secret(armor(Armor::PublicKey, &k.certificate(MADE)).as_bytes()).is_none());
    assert!(read_secret(b"").is_none());
}

#[test]
fn a_secret_key_with_a_passphrase_or_a_wrong_checksum_is_not_read() {
    let packets = key().secret_key(MADE).to_vec();
    // The secret part follows the packet header (2 bytes) and the public
    // body (1 + 4 + 1 + 10 + 2 + 33).
    let secret = 2 + 1 + 4 + 1 + 10 + 2 + 33;
    // String-to-key usage 254, as GnuPG writes a key with a passphrase.
    let mut bad = packets.clone();
    assert_eq!(bad[secret], 0);
    bad[secret] = 254;
    assert!(read_secret(private_block(&bad).as_bytes()).is_none());
    // The checksum of the secret part wrong, the armour's CRC right.
    let mut bad = packets.clone();
    bad[secret + 1 + 2 + 32] ^= 1;
    assert!(read_secret(private_block(&bad).as_bytes()).is_none());
}

#[test]
fn armour_that_is_off_is_not_read() {
    let text = text_of(&key(), MADE);
    let lines: Vec<&str> = text.lines().collect();
    // The checksum line gone, or wrong.
    let without: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| !l.starts_with('='))
        .collect();
    assert!(read_secret(without.join("\n").as_bytes()).is_none());
    let wrong = text.replace(lines[lines.len() - 2], "=AAAA");
    assert!(read_secret(wrong.as_bytes()).is_none());
    // Text after the end line, an empty line in the body, non-ASCII.
    assert!(read_secret(format!("{text}x\n").as_bytes()).is_none());
    assert!(read_secret(text.replacen("\n\n", "\n\n\n", 1).as_bytes()).is_none());
    assert!(read_secret(text.replace('Q', "\u{e9}").as_bytes()).is_none());
    // Not a private key block.
    assert!(read_secret(text.replace("PRIVATE", "PUBLIC").as_bytes()).is_none());
}

#[test]
fn a_packet_header_in_another_length_form_or_cut_short_is_not_read() {
    let packets = key().secret_key(MADE).to_vec();
    let n = usize::from(packets[1]);
    assert!(packets[1] < 192);
    let read = |p: &[u8]| read_secret(private_block(p).as_bytes());
    assert!(read(&packets).is_some());
    let rest = &packets[2..];
    let with = |head: &[u8]| [head, rest].concat();
    // The same length in the five-octet form, a partial length, an
    // old-format header, a header without its top bits.
    assert!(read(&with(&[0xC5, 255, 0, 0, 0, n as u8])).is_none());
    assert!(read(&with(&[0xC5, 224])).is_none());
    assert!(read(&with(&[0xC5, 254])).is_none());
    assert!(read(&with(&[0x94, n as u8])).is_none());
    assert!(read(&with(&[0x45, n as u8])).is_none());
    // A length past the data, and a header with no length.
    assert!(read(&with(&[0xC5, n as u8 + 1])).is_none());
    assert!(read(&with(&[0xC5, 255, 0xff, 0xff, 0xff, 0xff])).is_none());
    assert!(read(&with(&[0xC5, 192, 0])).is_none());
    assert!(read(&[0xC5]).is_none());
    assert!(read(&[0xC5, 255, 0, 0]).is_none());
}
