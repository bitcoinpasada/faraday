//! An image's signature is checked over the signed attributes as they are
//! written in the file (RFC 5652 5.4), and the first signer is the first
//! one written, whatever order DER would put them in.

use faraday_sb::{Check, make, pe, rng, x509};

const NOW: u64 = 1_700_000_000;

/// One DER element: `tag`, the length and `content`.
fn tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    match content.len() {
        n if n < 0x80 => out.push(n as u8),
        n if n < 0x100 => out.extend([0x81, n as u8]),
        n => out.extend([0x82, (n >> 8) as u8, n as u8]),
    }
    out.extend_from_slice(content);
    out
}

/// The elements one after another in `bytes`, each whole.
fn elements(mut bytes: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        let (head, len) = match bytes[1] {
            n if n < 0x80 => (2, n as usize),
            0x81 => (3, bytes[2] as usize),
            _ => (4, usize::from(bytes[2]) << 8 | usize::from(bytes[3])),
        };
        out.push(&bytes[..head + len]);
        bytes = &bytes[head + len..];
    }
    out
}

/// What is inside one element.
fn inside(element: &[u8]) -> &[u8] {
    let head = match element[1] {
        n if n < 0x80 => 2,
        0x81 => 3,
        _ => 4,
    };
    &element[head..]
}

/// An image signed by the db key, then written again with its signed
/// attributes in DER order or `reversed`, signed as written, and
/// with `signers` the SignerInfos of the SET OF, in the order given.
fn rewritten(reversed: bool, signers: impl Fn(&[u8]) -> Vec<Vec<u8>>) -> (Vec<u8>, Vec<u8>) {
    let keys = make(&[3u8; 32], "Attributes", NOW).unwrap();
    let tiny = include_bytes!("vectors/tiny.efi");
    let mut blind = rng::stream(&[5u8; 32], 0);
    let signed = pe::sign(&mut blind, tiny, &keys.db).unwrap();
    let at = tiny.len().div_ceil(8) * 8;
    let table = &signed[at..];
    let content_info = &table[8..];
    // ContentInfo { OID, [0] { SignedData } }.
    let explicit = elements(inside(content_info))[1];
    let signed_data = elements(inside(explicit))[0];
    let parts = elements(inside(signed_data));
    let info = elements(inside(parts[4]))[0];
    let fields = elements(inside(info));
    let attrs = elements(inside(fields[3]));
    assert!(attrs.len() > 1);
    let mut ordered = attrs.clone();
    if reversed {
        ordered.reverse();
    }
    let written: Vec<u8> = ordered.concat();
    let as_signed = tlv(0x31, &written);
    let sig = x509::sign(&mut blind, &keys.db.key, &as_signed).unwrap();
    let new_info = tlv(
        0x30,
        &[
            fields[0],
            fields[1],
            fields[2],
            &tlv(0xa0, &written),
            fields[4],
            &tlv(0x04, &sig),
        ]
        .concat(),
    );
    let new_set = tlv(0x31, &signers(&new_info).concat());
    let new_data = tlv(
        0x30,
        &[parts[0], parts[1], parts[2], parts[3], &new_set].concat(),
    );
    let new_ci = tlv(
        0x30,
        &[elements(inside(content_info))[0], &tlv(0xa0, &new_data)].concat(),
    );
    let length = (8 + new_ci.len()).div_ceil(8) * 8;
    let mut image = signed[..at].to_vec();
    image.extend_from_slice(&(length as u32).to_le_bytes());
    image.extend_from_slice(&[0x00, 0x02, 0x02, 0x00]);
    image.extend_from_slice(&new_ci);
    image.resize(at + length, 0);
    // The certificate-table entry: offset and length.
    let entry: Vec<u8> = [
        (at as u32).to_le_bytes(),
        (table.len() as u32).to_le_bytes(),
    ]
    .concat();
    let place = signed[..at]
        .windows(8)
        .position(|w| w == entry)
        .expect("the entry names the table");
    image[place + 4..place + 8].copy_from_slice(&(length as u32).to_le_bytes());
    assert_eq!(
        pe::image_hash(&image).unwrap(),
        pe::image_hash(&signed).unwrap(),
        "hash"
    );
    assert_eq!(
        pe::check(&signed, &keys.db.cert).unwrap(),
        Check::Valid,
        "orig"
    );
    (image, keys.db.cert)
}

#[test]
fn a_signature_over_attributes_written_out_of_der_order_is_valid() {
    // In DER order, as Faraday writes them.
    let (image, cert) = rewritten(false, |i| vec![i.to_vec()]);
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::Valid);
    // In reverse, and signed as written.
    let (image, cert) = rewritten(true, |i| vec![i.to_vec()]);
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::Valid);
}

#[test]
fn the_first_signer_is_the_first_one_written() {
    // A second signer that DER order would put first: the same
    // SignerInfo with another signature, one that sorts lower.
    let (image, cert) = rewritten(false, |good| {
        let mut other = good.to_vec();
        let last = other.len() - 1;
        other[last] = other[last].wrapping_add(1);
        if other.as_slice() > good {
            other[last] = good[last].wrapping_sub(1);
        }
        assert!(other.as_slice() < good);
        vec![good.to_vec(), other]
    });
    assert_eq!(pe::check(&image, &cert).unwrap(), Check::Valid);
}
