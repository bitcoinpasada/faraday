//! The QR vault's message against RFC 9580's own vectors (Appendix A,
//! `tools/vectors/openpgp/`), a write read back, and every refusal.

use alloc::vec;
use alloc::vec::Vec;

use faraday_crypto::work::Compute;

use super::*;

const VECTORS: &str = include_str!("../../../../tools/vectors/openpgp/rfc9580.txt");

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-pgp's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn vector(name: &str) -> Vec<u8> {
    let line = VECTORS
        .lines()
        .find_map(|l| l.strip_prefix(name)?.strip_prefix(" = "))
        .expect("vector present");
    (0..line.len() / 2)
        .map(|i| u8::from_str_radix(&line[2 * i..2 * i + 2], 16).expect("hex"))
        .collect()
}

/// A buffer of distinct bytes standing for the session's randomness.
fn random() -> [u8; RANDOM_LEN] {
    core::array::from_fn(|i| (i as u8).wrapping_mul(7).wrapping_add(3))
}

/// 226 bytes of QR TOML as a vault holds them: a 12-word seed padded
/// with line feeds.
fn toml() -> Vec<u8> {
    let mut t = b"words = \"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\"\n".to_vec();
    t.resize(226, b'\n');
    t
}

const PASSPHRASE: &[u8] = b"correct horse";
/// The least memory a reader accepts, to keep the tests quick.
const FAST: u8 = 13;

fn sealed() -> Vec<u8> {
    seal_at(&compute(), PASSPHRASE, FAST, &random(), &toml()).expect("sealed")
}

fn opened(bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>, Error> {
    parse(bytes)?.open(&compute(), PASSPHRASE)
}

fn refused(bytes: &[u8]) -> Error {
    match parse(bytes) {
        Err(e) => e,
        Ok(m) => m.open(&compute(), PASSPHRASE).expect_err("refused"),
    }
}

fn other(u: Unsupported) -> Error {
    Error::Unsupported(u)
}

// RFC 9580 A.10: v6 SKESK and v2 SEIPD, AES-128 and OCB.

fn a10_skesk_fields() -> ([u8; IV_LEN], Vec<u8>, [u8; TAG_LEN]) {
    let p = vector("a10_skesk");
    // Header (2), then the body: version, count, cipher, AEAD, S2K length,
    // the 11-octet S2K, the IV, the encrypted session key and its tag.
    let iv = p[2 + 5 + 11..2 + 5 + 11 + IV_LEN].try_into().unwrap();
    let esk = p[2 + 5 + 11 + IV_LEN..p.len() - TAG_LEN].to_vec();
    let tag = p[p.len() - TAG_LEN..].try_into().unwrap();
    (iv, esk, tag)
}

#[test]
fn the_rfc_session_key_is_unwrapped_and_wrapped_again() {
    let (iv, esk, tag) = a10_skesk_fields();
    let derived = vector("a10_derived_key");
    let session = vector("a10_session_key");
    let got = unwrap_session_key(Cipher::Aes128, &derived, &iv, &esk, &tag).expect("unwrapped");
    assert_eq!(&**got, &session[..]);
    let (wrapped, wrapped_tag) = wrap_session_key(Cipher::Aes128, &derived, &iv, &session);
    assert_eq!(wrapped, esk);
    assert_eq!(wrapped_tag, tag);
}

#[test]
fn the_rfc_data_packet_is_decrypted_and_encrypted_again() {
    let p = vector("a10_seipd");
    let session = vector("a10_session_key");
    // Header (2), version, cipher, AEAD, chunk octet, salt (32).
    let salt: [u8; SALT_LEN] = p[6..38].try_into().unwrap();
    let data = &p[38..];
    let mut expected = vector("a10_plain_literal");
    expected.extend(vector("a10_plain_padding"));
    let got = decrypt_chunks(Cipher::Aes128, &session, 6, &salt, data).expect("decrypted");
    assert_eq!(&**got, &expected[..]);
    assert_eq!(
        encrypt_chunks(Cipher::Aes128, &session, 6, &salt, &expected),
        data
    );
}

/// RFC 9580 A.12.3: Argon2 with t = 1, p = 4 and m = 2^21 KiB (2 GiB)
/// under "password"; the v4 SKESK's encrypted session key is AES-256 CFB
/// with a zero IV under the S2K output. Run with `--run-ignored all`.
#[test]
#[ignore = "takes 2 GiB of memory"]
fn the_rfc_argon2_s2k_gives_its_session_key() {
    use aes::cipher::{BlockCipherEncrypt, KeyInit};

    let m = vector("a12_3_message");
    let body = &m[2..];
    let salt: [u8; 16] = body[3..19].try_into().unwrap();
    assert_eq!(&body[19..22], &[1, 4, 21]);
    let key = s2k(&compute(), b"password", &salt, 1 << 21, 1, 4).expect("s2k");
    let cipher = Aes256::new_from_slice(key.expose()).unwrap();
    let mut previous = [0u8; 16];
    let mut plain = Vec::new();
    for block in body[22..55].chunks(16) {
        let mut stream = previous;
        cipher.encrypt_block((&mut stream).into());
        plain.extend(block.iter().zip(stream).map(|(c, k)| c ^ k));
        if block.len() == 16 {
            previous.copy_from_slice(block);
        }
    }
    assert_eq!(plain[0], AES256);
    assert_eq!(&plain[1..], &vector("a12_3_session_key")[..]);
}

// The message Faraday writes.

#[test]
fn a_vault_is_396_bytes_and_opens_to_what_it_holds() {
    let bytes = sealed();
    assert_eq!(bytes.len(), 396);
    let message = parse(&bytes).expect("parsed");
    assert_eq!(
        message.cost(),
        Cost {
            exponent: FAST,
            passes: 3,
            lanes: 4
        }
    );
    assert_eq!(&**opened(&bytes).expect("opened"), &toml()[..]);
    // The same randomness and passphrase make the same bytes.
    assert_eq!(bytes, sealed());
}

#[test]
fn the_packets_are_the_ones_the_spec_names() {
    let b = sealed();
    // SKESK: tag 3, 88 bytes; version 6, AES-256, OCB, the Argon2 S2K.
    assert_eq!(&b[..2], &[0xC3, 88]);
    assert_eq!(&b[2..8], &[6, 38, 9, 2, 20, 4]);
    assert_eq!(&b[8..24], &random()[..16]);
    assert_eq!(&b[24..27], &[3, 4, FAST]);
    assert_eq!(&b[27..42], &random()[16..31]);
    // SEIPD: tag 18, 303 bytes (two-octet length); version 2, AES-256,
    // OCB, chunk octet 10, then the salt.
    assert_eq!(&b[90..97], &[0xD2, 0xC0, 0x6F, 2, 9, 2, 10]);
    assert_eq!(&b[97..129], &random()[63..]);
}

#[test]
fn a_wrong_passphrase_is_named() {
    let bytes = sealed();
    let got = parse(&bytes).unwrap().open(&compute(), b"correct horsf");
    assert_eq!(got.unwrap_err(), Error::WrongPassphrase);
}

#[test]
fn a_changed_byte_in_the_data_is_refused() {
    let bytes = sealed();
    // The first ciphertext byte, the last of the chunk's tag, the final tag.
    for at in [129, 129 + 235 + 15, bytes.len() - 1] {
        let mut b = bytes.clone();
        b[at] ^= 1;
        assert_eq!(opened(&b).unwrap_err(), Error::Damaged, "byte {at}");
    }
}

#[test]
fn a_changed_byte_in_the_key_packet_is_a_wrong_passphrase() {
    let bytes = sealed();
    // The S2K salt, the IV, the encrypted session key, its tag.
    for at in [8, 27, 50, 85] {
        let mut b = bytes.clone();
        b[at] ^= 1;
        assert_eq!(opened(&b).unwrap_err(), Error::WrongPassphrase, "byte {at}");
    }
}

#[test]
fn a_changed_header_of_the_data_packet_is_refused() {
    let bytes = sealed();
    // The chunk size octet and the salt feed the keys and the associated data.
    for at in [96, 97] {
        let mut b = bytes.clone();
        b[at] ^= 1;
        assert!(opened(&b).is_err(), "byte {at}");
    }
}

#[test]
fn other_plaintext_lengths_are_sealed_and_opened() {
    for len in [0, 1, 191, 192, 600, MAX_PLAINTEXT] {
        let text = vec![b'x'; len];
        let b = seal_at(&compute(), PASSPHRASE, FAST, &random(), &text).expect("sealed");
        assert_eq!(&**opened(&b).expect("opened"), &text[..], "length {len}");
    }
    let text = vec![b'x'; MAX_PLAINTEXT + 1];
    assert_eq!(
        seal_at(&compute(), PASSPHRASE, FAST, &random(), &text),
        Err(Error::TooLong)
    );
}

#[test]
fn chunks_of_every_size_are_read_back() {
    let session = [9u8; 32];
    let salt = [4u8; SALT_LEN];
    // Chunk octet 0 is 64-byte chunks.
    for len in [0, 1, 63, 64, 65, 128, 200] {
        let text: Vec<u8> = (0..len).map(|i| i as u8).collect();
        let data = encrypt_chunks(Cipher::Aes256, &session, 0, &salt, &text);
        let back = decrypt_chunks(Cipher::Aes256, &session, 0, &salt, &data).expect("read");
        assert_eq!(&**back, &text[..], "length {len}");
        // A chunk dropped is refused by the final tag.
        if len > 64 {
            let mut cut = data[..64 + TAG_LEN].to_vec();
            cut.extend_from_slice(&data[data.len() - TAG_LEN..]);
            assert!(decrypt_chunks(Cipher::Aes256, &session, 0, &salt, &cut).is_err());
        }
    }
}

// What is refused.

#[test]
fn a_cost_outside_the_limits_is_refused_before_anything_runs() {
    let bytes = sealed();
    // Passes, lanes and the exponent are bytes 24, 25 and 26.
    for (at, value) in [
        (24, 0),
        (24, 101),
        (25, 0),
        (25, 65),
        (26, 12),
        (26, 21),
        (26, 255),
    ] {
        let mut b = bytes.clone();
        b[at] = value;
        assert_eq!(parse(&b).unwrap_err(), Error::Cost, "byte {at} = {value}");
    }
    // The widest the readers take.
    for (at, value) in [(24, 100), (25, 64), (26, 20), (26, 13)] {
        let mut b = bytes.clone();
        b[at] = value;
        assert!(parse(&b).is_ok(), "byte {at} = {value}");
    }
    // A writer takes the memory steps only (16 to 20), a reader 13 to 20.
    for exponent in [12, 13, 15, 21] {
        assert_eq!(
            seal(&compute(), PASSPHRASE, exponent, &random(), &toml()),
            Err(Error::Cost)
        );
    }
}

fn join(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat() // not text: bytes
}

#[test]
fn another_openpgp_message_is_named() {
    let b = sealed();
    let skesk = &b[..90];
    let seipd = &b[90..];

    let skesk4 = crate::packet(TAG_SKESK, &[4, AES256, 4, 0, 1, 2, 3]);
    assert_eq!(
        refused(&join(&[&skesk4, seipd])),
        other(Unsupported::Skesk4)
    );

    let seipd1 = crate::packet(TAG_SEIPD, &[1, 0, 0, 0]);
    assert_eq!(
        refused(&join(&[skesk, &seipd1])),
        other(Unsupported::Seipd1)
    );

    let pkesk = crate::packet(TAG_PKESK, &[6, 0, 0]);
    assert_eq!(
        refused(&join(&[&pkesk, skesk, seipd])),
        other(Unsupported::Pkesk)
    );
    assert_eq!(
        refused(&join(&[skesk, &pkesk, seipd])),
        other(Unsupported::Pkesk)
    );

    assert_eq!(
        refused(&join(&[skesk, skesk, seipd])),
        other(Unsupported::SecondSkesk)
    );

    let compressed = crate::packet(TAG_COMPRESSED, &[0, 1, 2]);
    assert_eq!(
        refused(&join(&[skesk, &compressed])),
        other(Unsupported::Compressed)
    );

    let signature = crate::packet(TAG_SIGNATURE, &[4, 0]);
    assert_eq!(
        refused(&join(&[skesk, &signature])),
        other(Unsupported::Signature)
    );
    let one_pass = crate::packet(TAG_ONE_PASS, &[3, 0]);
    assert_eq!(
        refused(&join(&[&one_pass, skesk, seipd])),
        other(Unsupported::Signature)
    );

    assert_eq!(refused(&join(&[&b, &[0]])), other(Unsupported::Trailing));
    let pad = crate::packet(TAG_PADDING, &[7; 20]);
    assert_eq!(
        refused(&join(&[&b, &pad, &crate::packet(TAG_LITERAL, b"b")])),
        other(Unsupported::Trailing)
    );
    assert_eq!(refused(&join(&[&b, skesk])), other(Unsupported::Trailing));
}

#[test]
fn another_algorithm_is_named() {
    let b = sealed();
    // AES-128 and another AEAD mode in the key packet, the S2K type, a
    // version 5 key packet, and the data packet's cipher.
    for (at, value) in [(4, AES128), (5, 1), (7, 3), (2, 5), (94, AES128)] {
        let mut m = b.clone();
        m[at] = value;
        assert_eq!(
            refused(&m),
            other(Unsupported::Other),
            "byte {at} = {value}"
        );
    }
}

#[test]
fn text_that_is_not_a_message_is_malformed() {
    assert_eq!(refused(b""), Error::Malformed);
    assert_eq!(
        refused(b"-----BEGIN PGP MESSAGE-----\n\nwz8G\n-----END PGP MESSAGE-----\n"),
        Error::Malformed
    );
    let b = sealed();
    for n in [1, 2, 50, 89, 90, 91, 120, b.len() - 1] {
        assert_eq!(parse(&b[..n]).unwrap_err(), Error::Malformed, "cut at {n}");
    }
}

/// A message whose data packet holds `inner` as its plaintext.
fn with_inner(inner: &[u8]) -> Vec<u8> {
    let cost = Cost {
        exponent: FAST,
        passes: PASSES,
        lanes: LANES,
    };
    seal_packets(&compute(), PASSPHRASE, cost, &random(), inner).expect("sealed")
}

fn literal(format: u8, name: &[u8], data: &[u8]) -> Vec<u8> {
    let mut body = vec![format, name.len() as u8];
    body.extend_from_slice(name);
    body.extend_from_slice(&[0; 4]);
    body.extend_from_slice(data);
    crate::packet(TAG_LITERAL, &body)
}

#[test]
fn the_literal_data_packet_must_be_binary_without_a_name() {
    let ok = literal(b'b', b"", b"hello");
    assert_eq!(&**opened(&with_inner(&ok)).unwrap(), b"hello");
    for (format, name) in [(b't', &b""[..]), (b'u', b""), (b'b', b"vault.txt")] {
        let got = opened(&with_inner(&literal(format, name, b"hello")));
        assert_eq!(got.unwrap_err(), Error::Literal);
    }
}

#[test]
fn a_compressed_first_packet_in_partial_lengths_is_named() {
    // A new-format header, tag 8, a partial length of 512, then the end.
    let mut m = vec![0xC0 | TAG_COMPRESSED, 224 + 9];
    m.extend([0u8; 512]);
    m.extend([0]);
    assert_eq!(refused(&m), other(Unsupported::Compressed));
}

#[test]
fn what_else_is_in_the_plaintext_is_named() {
    let ok = literal(b'b', b"", b"hello");
    let trailing = join(&[&ok, &crate::packet(TAG_LITERAL, b"b\0\0\0\0\0")]);
    assert_eq!(
        opened(&with_inner(&trailing)).unwrap_err(),
        other(Unsupported::Trailing)
    );
    let trailing = join(&[&ok, &crate::packet(TAG_PADDING, &[0; 16]), &[0]]);
    assert_eq!(
        opened(&with_inner(&trailing)).unwrap_err(),
        other(Unsupported::Trailing)
    );
    let compressed = crate::packet(TAG_COMPRESSED, &[0, 3, 4]);
    assert_eq!(
        opened(&with_inner(&compressed)).unwrap_err(),
        other(Unsupported::Compressed)
    );
    let signed = join(&[&crate::packet(TAG_ONE_PASS, &[3, 0, 8, 22]), &ok]);
    assert_eq!(
        opened(&with_inner(&signed)).unwrap_err(),
        other(Unsupported::Signature)
    );
    assert_eq!(opened(&with_inner(b"")).unwrap_err(), Error::Malformed);
    assert_eq!(
        opened(&with_inner(&crate::packet(TAG_LITERAL, b"b"))).unwrap_err(),
        Error::Malformed
    );
}

// Every length form RFC 9580 §4.2 allows.

fn old_format(tag: u8, body: &[u8]) -> Vec<u8> {
    let n = body.len();
    let mut out = if n < 256 {
        vec![0x80 | (tag << 2), n as u8]
    } else if n < 65536 {
        let mut v = vec![0x80 | (tag << 2) | 1];
        v.extend_from_slice(&(n as u16).to_be_bytes());
        v
    } else {
        let mut v = vec![0x80 | (tag << 2) | 2];
        v.extend_from_slice(&(n as u32).to_be_bytes());
        v
    };
    out.extend_from_slice(body);
    out
}

fn five_octet(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![0xC0 | tag, 0xFF];
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(body);
    out
}

/// The body in partial pieces: the first of 512, then pieces of
/// 2^`piece_exponent`, the rest a fixed length.
fn partial(tag: u8, body: &[u8], piece_exponent: u8) -> Vec<u8> {
    let mut out = vec![0xC0 | tag, 0xE0 | 9];
    out.extend_from_slice(&body[..512]);
    let mut rest = &body[512..];
    let piece = 1usize << piece_exponent;
    while rest.len() > piece {
        out.push(0xE0 | piece_exponent);
        out.extend_from_slice(&rest[..piece]);
        rest = &rest[piece..];
    }
    out.extend(crate::length(rest.len()));
    out.extend_from_slice(rest);
    out
}

fn bodies(bytes: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (a, rest) = read_packet(bytes).unwrap();
    let (b, _) = read_packet(rest).unwrap();
    (a.body.as_slice().to_vec(), b.body.as_slice().to_vec())
}

#[test]
fn every_length_form_is_read() {
    let text = vec![b'q'; 900];
    let plain = seal_at(&compute(), PASSPHRASE, FAST, &random(), &text).unwrap();
    let (skesk, seipd) = bodies(&plain);
    let forms = [
        (old_format(TAG_SKESK, &skesk), five_octet(TAG_SEIPD, &seipd)),
        (five_octet(TAG_SKESK, &skesk), five_octet(TAG_SEIPD, &seipd)),
        (
            crate::packet(TAG_SKESK, &skesk),
            partial(TAG_SEIPD, &seipd, 9),
        ),
        (old_format(TAG_SKESK, &skesk), partial(TAG_SEIPD, &seipd, 6)),
    ];
    for (i, (a, b)) in forms.iter().enumerate() {
        let m = join(&[a, b]);
        assert_eq!(&**opened(&m).expect("opened"), &text[..], "form {i}");
    }
    // The same inside the encryption: the literal in partial pieces.
    let inner = literal(b'b', b"", &text);
    let (literal_body, _) = bodies(&join(&[&inner, &inner]));
    let pieces = partial(TAG_LITERAL, &literal_body, 8);
    assert_eq!(&**opened(&with_inner(&pieces)).expect("opened"), &text[..]);
    // The literal in the old format, with a two-octet length and with
    // none (to the end of the plaintext).
    let old = old_format(TAG_LITERAL, &literal_body);
    assert_eq!(&**opened(&with_inner(&old)).expect("opened"), &text[..]);
    let mut open_ended = vec![0x80 | (TAG_LITERAL << 2) | 3];
    open_ended.extend_from_slice(&literal_body);
    assert_eq!(
        &**opened(&with_inner(&open_ended)).expect("opened"),
        &text[..]
    );
}

#[test]
fn a_length_that_is_not_allowed_is_malformed() {
    let text = vec![b'q'; 900];
    let plain = seal_at(&compute(), PASSPHRASE, FAST, &random(), &text).unwrap();
    let (skesk, seipd) = bodies(&plain);
    // A partial first piece under 512 octets.
    let mut small = vec![0xC0 | TAG_SEIPD, 0xE0 | 8];
    small.extend_from_slice(&seipd[..256]);
    small.extend(crate::length(seipd.len() - 256));
    small.extend_from_slice(&seipd[256..]);
    let m = join(&[&crate::packet(TAG_SKESK, &skesk), &small]);
    assert_eq!(refused(&m), Error::Malformed);
    // A partial length on the key packet.
    let long_skesk = join(&[&skesk, &[0; 600]]);
    let m = join(&[&partial(TAG_SKESK, &long_skesk, 9), &plain[90..]]);
    assert_eq!(refused(&m), Error::Malformed);
}

// Padding (RFC 9580 §5.14), and a vault from outside.

#[test]
fn padding_packets_are_ignored() {
    let ok = literal(b'b', b"", b"hello");
    let pad = crate::packet(TAG_PADDING, &[0xAA; 40]);
    // Inside the data, after the literal data.
    let inner = join(&[&ok, &pad, &pad]);
    assert_eq!(&**opened(&with_inner(&inner)).unwrap(), b"hello");
    // After the data packet.
    let b = join(&[&with_inner(&ok), &pad, &pad]);
    assert_eq!(&**opened(&b).unwrap(), b"hello");
    // Both, and an empty padding packet.
    let b = join(&[&with_inner(&inner), &crate::packet(TAG_PADDING, &[])]);
    assert_eq!(&**opened(&b).unwrap(), b"hello");
}

#[test]
fn the_rfc_plaintext_with_its_padding_is_read() {
    // A.10 is AES-128 with an iterated S2K, which a message is not opened
    // with; its data packet's whole plaintext, the literal data and then
    // the padding packet, is read as a message's is.
    let seipd = vector("a10_seipd");
    let salt: [u8; SALT_LEN] = seipd[6..38].try_into().unwrap();
    let plain = decrypt_chunks(
        Cipher::Aes128,
        &vector("a10_session_key"),
        6,
        &salt,
        &seipd[38..],
    )
    .expect("decrypted");
    assert_eq!(&**read_literal(&plain).expect("read"), b"Hello, world!");
}

#[test]
fn faradays_own_vault_at_exponent_13_is_a_known_answer() {
    // Faraday's own seal output, kept as a known answer
    // (tools/vectors/openpgp/README.md); it is not an outside check.
    let bytes = vector("vault_13");
    assert_eq!(parse(&bytes).unwrap().cost().exponent, 13);
    let got = opened(&bytes).expect("opened");
    assert_eq!(&**got, toml().as_slice());
}

// Where a packet's length ends and its body begins, at the edges of each
// form: a padding packet and then one more octet.

/// What reading `header`, a body of `n` octets and one more octet gives:
/// the body's length and what is left, or the error.
fn read_header(header: &[u8], n: usize) -> Result<(usize, usize), Error> {
    let bytes = join(&[header, &vec![0u8; n], &[9]]);
    let (packet, rest) = read_packet(&bytes)?;
    Ok((packet.body.as_slice().len(), rest.len()))
}

#[test]
fn a_packet_length_is_read_at_the_edge_of_each_form() {
    let pad = 0xC0 | TAG_PADDING;
    assert_eq!(read_header(&[pad, 0], 0), Ok((0, 1)));
    assert_eq!(read_header(&[pad, 191], 191), Ok((191, 1)));
    assert_eq!(read_header(&[pad, 192, 0], 192), Ok((192, 1)));
    assert_eq!(read_header(&[pad, 223, 255], 8383), Ok((8383, 1)));
    assert_eq!(read_header(&[pad, 255, 0, 0, 0, 0], 0), Ok((0, 1)));
    assert_eq!(read_header(&[pad, 255, 0, 0, 0, 192], 192), Ok((192, 1)));
    // The old format: one, two and four octets.
    let old = 0x80 | (TAG_SKESK << 2);
    assert_eq!(read_header(&[old, 255], 255), Ok((255, 1)));
    assert_eq!(read_header(&[old | 1, 1, 0], 256), Ok((256, 1)));
    assert_eq!(read_header(&[old | 2, 0, 0, 0, 3], 3), Ok((3, 1)));
}

#[test]
fn a_packet_length_that_is_cut_short_or_past_the_data_is_malformed() {
    let pad = 0xC0 | TAG_PADDING;
    let old = 0x80 | (TAG_SKESK << 2);
    for header in [
        vec![pad],
        vec![pad, 192],
        vec![pad, 255],
        vec![pad, 255, 0, 0, 0],
        vec![pad, 255, 0xFF, 0xFF, 0xFF, 0xFF],
        vec![pad, 255, 0x80, 0, 0, 0],
        vec![old],
        vec![old | 1, 0],
        vec![old | 2, 0, 0, 0],
        vec![old | 2, 0xFF, 0xFF, 0xFF, 0xFF],
        // Partial and indeterminate lengths on a packet that is not data.
        vec![pad, 224],
        vec![pad, 233],
        vec![pad, 254],
        vec![old | 3],
        // No top bit.
        vec![0x40 | TAG_PADDING, 0],
    ] {
        assert_eq!(read_header(&header, 0), Err(Error::Malformed), "{header:?}");
    }
    assert_eq!(read_header(&[pad, 193, 0], 193), Err(Error::Malformed));
    assert_eq!(read_header(&[pad, 191], 192), Ok((191, 2)));
}
