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
    assert_eq!(
        &gzip::inflate(&good, 5, &mut |_| Ok(())).unwrap()[..],
        b"hello"
    );
    let bad_crc = member(b"hello", [0, 0, 0, 0], 5);
    assert_eq!(
        gzip::inflate(&bad_crc, 5, &mut |_| Ok(())).err(),
        Some(Error::Damaged)
    );
    let long = member(b"hello", HELLO, 6);
    assert_eq!(
        gzip::inflate(&long, 8, &mut |_| Ok(())).err(),
        Some(Error::Damaged)
    );
    let short = member(b"hello", HELLO, 4);
    assert_eq!(
        gzip::inflate(&short, 8, &mut |_| Ok(())).err(),
        Some(Error::Damaged)
    );
    // The size it states is over the limit: refused as soon as it is read.
    assert_eq!(
        gzip::inflate(&good, 4, &mut |_| Ok(())).err(),
        Some(Error::TooLarge)
    );
    let huge = member(b"hello", HELLO, u32::MAX);
    assert_eq!(
        gzip::inflate(&huge, 4 * 1024 * 1024, &mut |_| Ok(())).err(),
        Some(Error::TooLarge)
    );
    let mut more = good.clone();
    more.push(0);
    assert_eq!(
        gzip::inflate(&more, 8, &mut |_| Ok(())).err(),
        Some(Error::Damaged)
    );
    let mut not = good.clone();
    not[1] = 0x8C;
    assert_eq!(
        gzip::inflate(&not, 8, &mut |_| Ok(())).err(),
        Some(Error::Damaged)
    );
    for cut in 0..good.len() {
        assert!(
            gzip::inflate(&good[..cut], 8, &mut |_| Ok(())).is_err(),
            "cut at {cut}"
        );
    }
}

#[test]
fn a_gzip_header_with_a_name_is_skipped() {
    let mut named = member(b"hello", HELLO, 5);
    named[3] = 8;
    named.splice(10..10, *b"name\0");
    assert_eq!(
        &gzip::inflate(&named, 5, &mut |_| Ok(())).unwrap()[..],
        b"hello"
    );
}

fn shared(bytes: Vec<u8>) -> super::Shared {
    super::Shared::new(zeroize::Zeroizing::new(bytes))
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
    let read = inner::read_inner(&shared(body.clone())).unwrap();
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
        inner::read_inner(&shared(salsa.clone())).err(),
        Some(Error::Unsupported(Unsupported::Salsa20))
    );
    let none = inner_header(&[(1, &[0, 0, 0, 0]), (2, &[5; 32]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&shared(none.clone())).err(),
        Some(Error::Unsupported(Unsupported::OtherInnerStream))
    );
    let unknown = inner_header(&[(1, &CHACHA20), (2, &[5; 32]), (9, &[1]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&shared(unknown.clone())).err(),
        Some(Error::UnknownField(9))
    );
    let no_key = inner_header(&[(1, &CHACHA20), (0, &[])]);
    assert_eq!(
        inner::read_inner(&shared(no_key.clone())).err(),
        Some(Error::Damaged)
    );
    let empty_key = inner_header(&[(1, &CHACHA20), (2, &[]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&shared(empty_key.clone())).err(),
        Some(Error::Damaged)
    );
    let no_end = inner_header(&[(1, &CHACHA20), (2, &[5; 32])]);
    assert_eq!(
        inner::read_inner(&shared(no_end.clone())).err(),
        Some(Error::Damaged)
    );
    let twice = inner_header(&[(1, &CHACHA20), (1, &CHACHA20), (2, &[5; 32]), (0, &[])]);
    assert_eq!(
        inner::read_inner(&shared(twice.clone())).err(),
        Some(Error::Damaged)
    );
}

/// `plain` encrypted under AES-256-CBC with a fixed key and IV, as a body
/// whose HMAC verified would hold it, and run through `unpad`.
fn unpadded(plain: &[u8]) -> Result<Vec<u8>, Error> {
    use aes::cipher::inout::InOutBuf;
    use aes::cipher::{BlockModeEncrypt, KeyIvInit};

    let (key, iv) = ([7u8; 32], [9u8; 16]);
    let mut body = plain.to_vec();
    let (blocks, tail) = InOutBuf::from(&mut body[..]).into_chunks();
    assert!(tail.is_empty(), "the test body is whole blocks");
    cbc::Encryptor::<aes::Aes256>::new_from_slices(&key, &iv)
        .unwrap()
        .encrypt_blocks_inout(blocks);
    let cipher = cbc::Decryptor::<aes::Aes256>::new_from_slices(&key, &iv).unwrap();
    let len = super::unpad(&mut body, cipher)?;
    body.truncate(len);
    Ok(body)
}

/// Two blocks ending in `tail`.
fn blocks_ending(tail: &[u8]) -> Vec<u8> {
    let mut body = vec![b'x'; 32 - tail.len()];
    body.extend_from_slice(tail);
    body
}

#[test]
fn aes_padding_is_taken_off_only_when_it_is_whole() {
    // One to sixteen bytes, each stating its own count.
    assert_eq!(unpadded(&blocks_ending(&[1])).unwrap().len(), 31);
    assert_eq!(unpadded(&blocks_ending(&[3, 3, 3])).unwrap().len(), 29);
    assert_eq!(unpadded(&blocks_ending(&[16; 16])).unwrap().len(), 16);
    // A whole block of padding and nothing before it.
    assert_eq!(unpadded(&[16; 16]).unwrap().len(), 0);
    // Padding of 0, over 16, bytes that do not match.
    for tail in [&[0u8][..], &[17], &[255], &[2, 2, 3], &[1, 2], &[3, 3]] {
        assert_eq!(
            unpadded(&blocks_ending(tail)).err(),
            Some(Error::Damaged),
            "{tail:?}"
        );
    }
    // Empty is no body: padding always adds a block.
    assert_eq!(unpadded(&[]).err(), Some(Error::Damaged));
    // Not a whole number of blocks.
    let mut body = [7u8; 31];
    let cipher = <cbc::Decryptor<aes::Aes256> as aes::cipher::KeyIvInit>::new_from_slices(
        &[7; 32], &[9; 16],
    )
    .unwrap();
    assert_eq!(super::unpad(&mut body, cipher).err(), Some(Error::Damaged));
    assert_eq!(super::strip_padding(&[1u8; 17]).err(), Some(Error::Damaged));
}

#[test]
fn the_inner_header_length_is_worked_out_as_it_is_written() {
    let binaries = [
        inner::Binary {
            protected: true,
            data: shared(vec![1, 2, 3]),
        },
        inner::Binary {
            protected: false,
            data: shared(vec![]),
        },
    ];
    for set in [&binaries[..0], &binaries[..1], &binaries[..]] {
        let mut out = Vec::new();
        let set = inner::Binaries::all(set);
        inner::write(&[5; 64], &set, &mut |piece| out.extend_from_slice(piece));
        assert_eq!(inner::written_len(&[5; 64], &set), out.len());
    }
}

#[test]
fn a_gzip_member_states_its_length_to_the_budget_before_anything_is_allocated() {
    let good = member(b"hello", HELLO, 5);
    let mut asked = Vec::new();
    let got = gzip::inflate(&good, 5, &mut |n| {
        asked.push(n);
        Err(7)
    });
    assert_eq!(got.err(), Some(Error::Memory(7)));
    assert_eq!(asked, [5]);
    // A length past the limit never reaches the budget.
    let huge = member(b"hello", HELLO, u32::MAX);
    let got = gzip::inflate(&huge, super::MAX_PLAIN, &mut |_| panic!("asked"));
    assert_eq!(got.err(), Some(Error::TooLarge));
}

#[test]
fn a_body_longer_than_the_largest_vault_is_refused_by_its_stated_length() {
    // The file itself would be about 151 MiB: the check takes the length.
    assert_eq!(
        super::stored_fits(super::MAX_STORED + 1),
        Err(Error::TooLarge)
    );
    assert_eq!(super::stored_fits(super::MAX_STORED), Ok(()));
}

#[test]
fn an_inner_header_with_more_attachments_than_the_limit_is_refused() {
    let mut fields = vec![(1u8, &CHACHA20[..]), (2, &[5; 64][..])];
    let one = [0u8];
    fields.extend(core::iter::repeat_n(
        (3u8, &one[..]),
        super::MAX_BINARIES + 1,
    ));
    fields.push((0, &[]));
    assert_eq!(
        inner::read_inner(&shared(inner_header(&fields))).err(),
        Some(Error::TooLarge)
    );
    fields.remove(2);
    assert_eq!(
        inner::read_inner(&shared(inner_header(&fields)))
            .unwrap()
            .binaries
            .len(),
        super::MAX_BINARIES
    );
}

#[test]
fn an_opened_file_keeps_no_copy_of_the_document_with_its_attachments() {
    let c = crate::tests::compute();
    let document = vec![b'x'; 40_000];
    let attached = vec![7u8; 100];
    let binaries = [inner::Binary {
        protected: false,
        data: shared(attached.clone()),
    }];
    let random: [u8; super::RANDOM_LEN] = core::array::from_fn(|i| i as u8);
    let contents = super::Contents {
        document: &document,
        binaries: inner::Binaries::all(&binaries),
        public_custom_data: None,
    };
    let cost = crate::Cost {
        memory_kib: 8 * 1024,
        passes: 1,
        lanes: 1,
    };
    let file = super::seal(&c, "pass", cost, &random, &contents, 65_536).unwrap();

    let opened = super::open(&c, &file, "pass").unwrap();
    assert_eq!(&opened.document[..document.len()], &document[..]);
    assert_eq!(&opened.binaries[0].data[..], &attached[..]);
    // What the attachment keeps alive is the inner header, not the document.
    let kept = opened.binaries[0].data.backing_len();
    assert!(
        kept < document.len() / 10,
        "the attachment keeps {kept} bytes alive"
    );
}

/// An inner header as a file writes it, with `end` as the end field's value.
fn body_with_end_field(end: &[u8], document: &[u8]) -> Vec<u8> {
    let field = |id: u8, value: &[u8], out: &mut Vec<u8>| {
        out.push(id);
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
    };
    let mut body = Vec::new();
    field(1, &3u32.to_le_bytes(), &mut body);
    field(2, &[9u8; 64], &mut body);
    field(3, &[0, 5, 6, 7], &mut body);
    field(0, end, &mut body);
    body.extend_from_slice(document);
    body
}

#[test]
fn an_end_field_with_bytes_opens_the_same_and_costs_no_more_to_hold() {
    let document = b"<KeePassFile/>";
    let read = |end: &[u8]| {
        let body = body_with_end_field(end, document);
        super::read_body(
            zeroize::Zeroizing::new(body),
            super::Compression::None,
            &mut |_| Ok(()),
            &mut |_, _| Ok(()),
        )
        .ok()
        .unwrap()
    };
    let (empty_doc, empty) = read(&[]);
    let (big_doc, big) = read(&vec![0xAB; 1 << 20]);
    assert_eq!(&empty_doc[..], &big_doc[..]);
    assert_eq!(&empty.binaries[0].data[..], &big.binaries[0].data[..]);
    assert_eq!(
        empty.binaries[0].data.backing_len(),
        big.binaries[0].data.backing_len()
    );
}
