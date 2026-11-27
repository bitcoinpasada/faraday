//! What a packet's or a subpacket's length may be in a file Faraday reads
//! (RFC 9580 §4.2 packet lengths, §5.2.3.7 subpacket lengths): the forms
//! that are read, and the lengths that are refused because they run past
//! the data, are cut short or are not allowed there.
#![allow(clippy::disallowed_methods)]

use faraday_pgp::{Armor, Key, armor, read_public, signature_time};

const MADE: u32 = 1_780_000_000;

fn key() -> Key {
    Key {
        primary_seed: [0x11; 32],
        created: MADE,
        subkey_seed: [0x22; 32],
        subkey_created: MADE,
        user_ids: vec![faraday_text::Text::of("Test Person <test@example.com>")],
        expiry: 0,
    }
}

/// The body of the certificate's public key packet.
fn public_key_body() -> Vec<u8> {
    let cert = key().certificate(MADE);
    let n = usize::from(cert[1]);
    cert[2..2 + n].to_vec()
}

fn five(n: u32) -> Vec<u8> {
    let mut out = vec![0xFF];
    out.extend_from_slice(&n.to_be_bytes());
    out
}

/// A key block of the public key and then `rest`, as `read_public` takes it.
fn block(rest: &[u8]) -> Vec<u8> {
    let body = public_key_body();
    let mut data = vec![0xC6, body.len() as u8];
    data.extend(body);
    data.extend_from_slice(rest);
    armor(Armor::PublicKey, &data).as_bytes().to_vec()
}

/// The user ID text a block of the key and one user ID packet with `header`
/// reads as, when it reads.
fn user_id(header: &[u8], text: &[u8]) -> Option<String> {
    let mut rest = header.to_vec();
    rest.extend_from_slice(text);
    let read = read_public(&block(&rest))?;
    assert_eq!(read.tags, vec![6, 13]);
    assert_eq!(read.texts.len(), 1);
    Some(String::from(&*read.texts[0]))
}

fn text(n: usize) -> Vec<u8> {
    vec![b'u'; n]
}

fn same(n: usize) -> Option<String> {
    Some("u".repeat(n))
}

#[test]
fn a_new_format_length_is_read_in_the_one_two_and_five_octet_forms() {
    assert_eq!(user_id(&[0xCD, 1], &text(1)), same(1));
    assert_eq!(user_id(&[0xCD, 191], &text(191)), same(191));
    assert_eq!(user_id(&[0xCD, 192, 0], &text(192)), same(192));
    assert_eq!(user_id(&[0xCD, 193, 5], &text(453)), same(453));
    assert_eq!(user_id(&[0xCD, 223, 255], &text(8383)), same(8383));
    // The five-octet form need not be the shortest.
    assert_eq!(user_id(&[0xCD, 255, 0, 0, 0, 5], &text(5)), same(5));
    assert_eq!(user_id(&[0xCD, 255, 0, 0, 0, 200], &text(200)), same(200));
}

#[test]
fn a_new_format_length_that_is_partial_or_cut_short_is_refused() {
    // 224 to 254 are partial lengths: no key block has one.
    for first in [224u8, 225, 233, 254] {
        assert_eq!(user_id(&[0xCD, first], &text(512)), None, "{first}");
    }
    // The length runs past the data, or the length itself does.
    assert_eq!(user_id(&[0xCD, 10], &text(9)), None);
    assert_eq!(user_id(&[0xCD, 192, 0], &text(191)), None);
    assert_eq!(user_id(&[0xCD, 192], &[]), None);
    assert_eq!(user_id(&[0xCD], &[]), None);
    assert_eq!(user_id(&[0xCD, 255, 0, 0], &[]), None);
    assert_eq!(user_id(&[0xCD, 255, 0, 0, 0], &[]), None);
    assert_eq!(user_id(&[0xCD, 255, 0, 0, 0, 6], &text(5)), None);
    assert_eq!(
        user_id(&[0xCD, 255, 0xFF, 0xFF, 0xFF, 0xFF], &text(5)),
        None
    );
    assert_eq!(user_id(&[0xCD, 255, 0x80, 0, 0, 0], &text(5)), None);
}

#[test]
fn an_old_format_length_is_read_in_the_one_two_and_four_octet_forms() {
    // Tag 13 in the old format: 0x80 | 13 << 2, plus the length type.
    assert_eq!(user_id(&[0xB4, 5], &text(5)), same(5));
    assert_eq!(user_id(&[0xB4, 255], &text(255)), same(255));
    assert_eq!(user_id(&[0xB5, 0, 5], &text(5)), same(5));
    assert_eq!(user_id(&[0xB5, 1, 0x2C], &text(300)), same(300));
    assert_eq!(user_id(&[0xB6, 0, 0, 0, 5], &text(5)), same(5));
    assert_eq!(user_id(&[0xB6, 0, 0, 1, 0], &text(256)), same(256));
}

#[test]
fn an_old_format_length_that_is_indeterminate_or_cut_short_is_refused() {
    assert_eq!(user_id(&[0xB7], &text(5)), None);
    assert_eq!(user_id(&[0xB4, 6], &text(5)), None);
    assert_eq!(user_id(&[0xB4], &[]), None);
    assert_eq!(user_id(&[0xB5, 0], &[]), None);
    assert_eq!(user_id(&[0xB5, 1, 0x2C], &text(299)), None);
    assert_eq!(user_id(&[0xB6, 0, 0, 0], &[]), None);
    assert_eq!(user_id(&[0xB6, 0, 0, 0, 6], &text(5)), None);
    assert_eq!(user_id(&[0xB6, 0xFF, 0xFF, 0xFF, 0xFF], &text(5)), None);
    // A header without its top bit is no packet.
    assert_eq!(user_id(&[0x4D, 5], &text(5)), None);
}

// Subpackets, read in the signature of a signature block.

/// A signature packet whose areas are `hashed` and `unhashed`, the rest as
/// the reader wants it (Ed448, 114 bytes).
fn signature_block(hashed: &[u8], unhashed: &[u8]) -> Vec<u8> {
    let mut body = vec![4, 0, 28, 10];
    body.extend_from_slice(&(hashed.len() as u16).to_be_bytes());
    body.extend_from_slice(hashed);
    body.extend_from_slice(&(unhashed.len() as u16).to_be_bytes());
    body.extend_from_slice(unhashed);
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(&[7; 114]);
    let mut data = vec![0xC2];
    data.extend(five(body.len() as u32));
    data.extend(body);
    armor(Armor::Signature, &data).as_bytes().to_vec()
}

/// A policy URI subpacket (type 26) of `n` bytes of text, its length
/// octets given.
fn policy(length: &[u8], n: usize) -> Vec<u8> {
    let mut out = length.to_vec();
    out.push(26);
    out.extend(text(n));
    out
}

/// The length of each text the block holds, when it reads.
fn texts(hashed: &[u8], unhashed: &[u8]) -> Option<Vec<usize>> {
    let read = read_public(&signature_block(hashed, unhashed))?;
    Some(read.texts.iter().map(|t| t.len()).collect())
}

#[test]
fn a_subpacket_length_is_read_in_the_one_two_and_five_octet_forms() {
    assert_eq!(texts(&policy(&[1], 0), &[]), Some(vec![0]));
    assert_eq!(texts(&policy(&[191], 190), &[]), Some(vec![190]));
    assert_eq!(texts(&policy(&[192, 0], 191), &[]), Some(vec![191]));
    // 192 to 254 are all the first octet of two: there is no partial form.
    assert_eq!(texts(&policy(&[223, 0], 8127), &[]), Some(vec![8127]));
    assert_eq!(texts(&policy(&[224, 0], 8383), &[]), Some(vec![8383]));
    assert_eq!(texts(&policy(&[254, 36], 16099), &[]), Some(vec![16099]));
    assert_eq!(texts(&policy(&[255, 0, 0, 0, 6], 5), &[]), Some(vec![5]));
    assert_eq!(
        texts(&policy(&[255, 0, 0, 1, 0], 255), &[]),
        Some(vec![255])
    );
    // Both areas are read.
    let (a, b) = (policy(&[3], 2), policy(&[255, 0, 0, 0, 4], 3));
    assert_eq!(texts(&a, &b), Some(vec![2, 3]));
}

#[test]
fn a_subpacket_length_that_runs_past_its_area_or_is_cut_short_is_refused() {
    assert_eq!(texts(&[0], &[]), None);
    assert_eq!(texts(&policy(&[10], 8), &[]), None);
    assert_eq!(texts(&policy(&[192, 0], 190), &[]), None);
    assert_eq!(texts(&[192], &[]), None);
    assert_eq!(texts(&[224], &[]), None);
    assert_eq!(texts(&[254], &[]), None);
    assert_eq!(texts(&[255], &[]), None);
    assert_eq!(texts(&[255, 0, 0, 0], &[]), None);
    assert_eq!(texts(&policy(&[255, 0, 0, 0, 7], 5), &[]), None);
    assert_eq!(texts(&policy(&[255, 0xFF, 0xFF, 0xFF, 0xFF], 5), &[]), None);
    assert_eq!(texts(&[], &policy(&[255, 0x80, 0, 0, 0], 5)), None);
    // A cut length in the unhashed area is refused as one in the hashed.
    assert_eq!(texts(&policy(&[3], 2), &[200]), None);
}

// The time of the first signature packet.

fn signature_packet(header: &[u8], pad: usize) -> Vec<u8> {
    let mut body = vec![4, 0x13, 22, 10, 0, 6, 5, 2];
    body.extend_from_slice(&MADE.to_be_bytes());
    body.extend(vec![0; pad]);
    let mut out = header.to_vec();
    out.extend(body);
    out
}

#[test]
fn the_time_of_a_signature_is_read_past_any_new_format_length_of_two_octets_or_fewer() {
    let time = |h: &[u8], pad| signature_time(&signature_packet(h, pad));
    assert_eq!(time(&[0xC2, 12], 0), Some(MADE));
    assert_eq!(time(&[0xC2, 191], 179), Some(MADE));
    assert_eq!(time(&[0xC2, 192, 0], 180), Some(MADE));
    assert_eq!(time(&[0xC2, 193, 4], 440), Some(MADE));
    // After a packet of another tag with a two-octet length.
    let mut two = vec![0xCD, 192, 0];
    two.extend(vec![b'u'; 192]);
    two.extend(signature_packet(&[0xC2, 12], 0));
    assert_eq!(signature_time(&two), Some(MADE));
    // Partial and five-octet lengths, a cut packet and a short one: none.
    assert_eq!(time(&[0xC2, 224], 0), None);
    assert_eq!(time(&[0xC2, 0xE9], 0), None);
    assert_eq!(time(&[0xC2, 254], 0), None);
    assert_eq!(time(&[0xC2, 255, 0, 0, 0, 12], 0), None);
    assert_eq!(time(&[0xC2, 13], 0), None);
    assert_eq!(time(&[0xC2, 192, 0], 0), None);
    assert_eq!(signature_time(&[0xC2, 192]), None);
    assert_eq!(signature_time(&[0xC2]), None);
    assert_eq!(signature_time(&[]), None);
    // Old-format headers are not read here.
    assert_eq!(time(&[0x88, 12], 0), None);
}
