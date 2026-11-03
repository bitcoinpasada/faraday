//! The parts of a KDBX body that only a file whose passphrase is known
//! reaches: gzip and the inner header, read from bytes made here.

use alloc::vec;
use alloc::vec::Vec;

use super::header::put_field;
use super::{Error, Unsupported, gzip, inner};

/// A gzip member of one stored block holding `data`.
fn member(data: &[u8], crc: [u8; 4], size: u32) -> Vec<u8> {
    let mut out = vec![0x1F, 0x8B, 8, 0, 0, 0, 0, 0, 0, 255];
    out.push(1);
    out.extend_from_slice(&(data.len() as u16).to_le_bytes());
    out.extend_from_slice(&(!(data.len() as u16)).to_le_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&crc);
    out.extend_from_slice(&size.to_le_bytes());
    out
}

/// The CRC-32 of "hello".
const HELLO: [u8; 4] = [0x86, 0xA6, 0x10, 0x36];

#[test]
fn a_gzip_member_is_inflated_and_checked() {
    let good = member(b"hello", HELLO, 5);
    assert_eq!(&gzip::inflate(&good, 5).unwrap()[..], b"hello");
    let bad_crc = member(b"hello", [0, 0, 0, 0], 5);
    assert_eq!(gzip::inflate(&bad_crc, 5).err(), Some(Error::Damaged));
    let long = member(b"hello", HELLO, 6);
    assert_eq!(gzip::inflate(&long, 8).err(), Some(Error::Damaged));
    let short = member(b"hello", HELLO, 4);
    assert_eq!(gzip::inflate(&short, 8).err(), Some(Error::Damaged));
    // The size it states is over the limit: refused as soon as it is read.
    assert_eq!(gzip::inflate(&good, 4).err(), Some(Error::TooLarge));
    let huge = member(b"hello", HELLO, u32::MAX);
    assert_eq!(
        gzip::inflate(&huge, 4 * 1024 * 1024).err(),
        Some(Error::TooLarge)
    );
    let mut more = good.clone();
    more.push(0);
    assert_eq!(gzip::inflate(&more, 8).err(), Some(Error::Damaged));
    let mut not = good.clone();
    not[1] = 0x8C;
    assert_eq!(gzip::inflate(&not, 8).err(), Some(Error::Damaged));
    for cut in 0..good.len() {
        assert!(gzip::inflate(&good[..cut], 8).is_err(), "cut at {cut}");
    }
}

#[test]
fn a_gzip_header_with_a_name_is_skipped() {
    let mut named = member(b"hello", HELLO, 5);
    named[3] = 8;
    named.splice(10..10, *b"name\0");
    assert_eq!(&gzip::inflate(&named, 5).unwrap()[..], b"hello");
}

fn inner_header(fields: &[(u8, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    for (id, value) in fields {
        put_field(*id, value, &mut out);
    }
    out.extend_from_slice(b"<doc/>");
    out
}

const CHACHA20: [u8; 4] = [3, 0, 0, 0];

#[test]
fn an_inner_header_is_read_to_where_the_document_begins() {
    let body = inner_header(&[(1, &CHACHA20), (2, &[5; 64]), (3, &[1, 9, 9]), (0, &[])]);
    let read = inner::read_inner(&body).unwrap();
    assert_eq!(&body[read.document_at..], b"<doc/>");
    assert_eq!(&read.key[..], &[5; 64][..]);
    assert_eq!(read.binaries.len(), 1);
    assert!(read.binaries[0].protected);
    assert_eq!(&read.binaries[0].data[..], &[9, 9]);
}

#[test]
fn an_inner_header_that_is_not_chacha20_is_refused_by_name() {
    let salsa = inner_header(&[(1, &[2, 0, 0, 0]), (2, &[5; 32]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&salsa).err(),
        Some(Error::Unsupported(Unsupported::Salsa20))
    );
    let none = inner_header(&[(1, &[0, 0, 0, 0]), (2, &[5; 32]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&none).err(),
        Some(Error::Unsupported(Unsupported::OtherInnerStream))
    );
    let unknown = inner_header(&[(1, &CHACHA20), (2, &[5; 32]), (9, &[1]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&unknown).err(),
        Some(Error::UnknownField(9))
    );
    let no_key = inner_header(&[(1, &CHACHA20), (0, &[])]);
    assert_eq!(inner::read_inner(&no_key).err(), Some(Error::Damaged));
    let empty_key = inner_header(&[(1, &CHACHA20), (2, &[]), (0, &[])]);
    assert_eq!(inner::read_inner(&empty_key).err(), Some(Error::Damaged));
    let no_end = inner_header(&[(1, &CHACHA20), (2, &[5; 32])]);
    assert_eq!(inner::read_inner(&no_end).err(), Some(Error::Damaged));
    let twice = inner_header(&[(1, &CHACHA20), (1, &CHACHA20), (2, &[5; 32]), (0, &[])]);
    assert_eq!(inner::read_inner(&twice).err(), Some(Error::Damaged));
}
