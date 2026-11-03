//! Known-answer tests for the hash wrappers and the `Secret` type.

use core::sync::atomic::{AtomicUsize, Ordering};

use faraday_crypto::{
    HKDF_SHA256_MAX, Pinned, Secret, Zeroize, ZeroizeOnDrop, hkdf_sha256_expand, hmac_sha256,
    hmac_sha512, pbkdf2_hmac_sha256, pbkdf2_hmac_sha512, sha256, work::Compute,
};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn sha256_known_answers() {
    // FIPS 180-4 examples.
    assert_eq!(
        sha256(b"").to_vec(),
        unhex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    );
    assert_eq!(
        sha256(b"abc").to_vec(),
        unhex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
    );
}

#[test]
fn hmac_sha512_rfc4231_case_1() {
    let key = [0x0bu8; 20];
    let out = hmac_sha512(&key, b"Hi There");
    assert_eq!(
        out.to_vec(),
        unhex(
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde\
             daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
        )
    );
}

#[test]
fn hmac_sha512_rfc4231_case_2() {
    // Key shorter than the block size, text longer than one word.
    let out = hmac_sha512(b"Jefe", b"what do ya want for nothing?");
    assert_eq!(
        out.to_vec(),
        unhex(
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea250554\
             9758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        )
    );
}

#[test]
fn pbkdf2_hmac_sha512_known_answer() {
    // password "password", salt "salt", 1 iteration, 64-byte output. This is
    // the widely published SHA-512 counterpart of the RFC 6070 vectors and
    // agrees with Python's hashlib.pbkdf2_hmac.
    let mut out = [0u8; 64];
    pbkdf2_hmac_sha512(&Compute::new(|_| {}), b"password", b"salt", 1, &mut out);
    assert_eq!(
        out.to_vec(),
        unhex(
            "867f70cf1ade02cff3752599a3a53dc4af34c7a669815ae5d513554e1c8cf252\
             c02d470a285a0501bad999bfe943c08f050235d7d68b1da55e63f73b60a57fce"
        )
    );

    // Two iterations, to exercise the round loop.
    pbkdf2_hmac_sha512(&Compute::new(|_| {}), b"password", b"salt", 2, &mut out);
    assert_eq!(
        out.to_vec(),
        unhex(
            "e1d9c16aa681708a45f5c7c4e215ceb66e011a2e9f0040713f18aefdb866d53c\
             f76cab2868a39b9f7840edce4fef5a82be67335c77a6068e04112754f27ccf4e"
        )
    );
}

#[test]
fn equal_secrets_compare_equal_and_different_ones_do_not() {
    let a = Secret::new([1u8, 2, 3, 4]);
    let b = Secret::new([1u8, 2, 3, 4]);
    let c = Secret::new([1u8, 2, 3, 5]);
    assert!(a == b);
    assert!(a != c);
}

#[test]
fn secret_zeroizes_on_drop() {
    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
    assert_zeroize_on_drop::<Secret<[u8; 32]>>();
    assert_zeroize_on_drop::<Secret<[u16; 24]>>();
    assert_zeroize_on_drop::<Pinned<[u8; 32]>>();
    assert_zeroize_on_drop::<Pinned<[u16; 24]>>();
}

#[test]
fn pinned_holds_what_was_put_in_it() {
    let mut key = Pinned::new([0u8; 32]);
    key.expose_mut()[..4].copy_from_slice(b"seed");
    assert_eq!(&key.expose()[..4], b"seed");
    assert_eq!(key.with(|k| k[31]), 0);

    // A value that has moved still holds what it held, and compares
    // equal to the same bytes on another page.
    let moved = key;
    assert!(moved == Pinned::new(*moved.expose()));
    assert!(moved != Pinned::new([0u8; 32]));
}

/// A secret that reports every time it is wiped, so that a test can see
/// what putting one on a pinned page leaves behind.
#[derive(Default)]
struct Counted([u8; 32]);

static ZEROIZED: AtomicUsize = AtomicUsize::new(0);

impl Zeroize for Counted {
    fn zeroize(&mut self) {
        self.0.zeroize();
        ZEROIZED.fetch_add(1, Ordering::SeqCst);
    }
}

/// A secret handed to a pinned page does not stay behind in the frame it
/// came from: the page keeps it and the source is wiped.
#[test]
fn pinning_a_value_wipes_where_it_came_from() {
    let before = ZEROIZED.load(Ordering::SeqCst);
    let held = Pinned::new(Counted([0x5au8; 32]));
    assert_eq!(
        ZEROIZED.load(Ordering::SeqCst),
        before + 1,
        "the value handed in is wiped once"
    );
    assert_eq!(held.expose().0, [0x5au8; 32], "and the page holds it");
    drop(held);
    assert_eq!(
        ZEROIZED.load(Ordering::SeqCst),
        before + 2,
        "and the page is wiped when it goes"
    );
}

/// A page filled in place holds what was written into it, and the writer
/// never had a copy of its own to leave anywhere.
#[test]
fn a_pinned_page_can_be_filled_in_place() {
    let source = *b"entropy from the shell, 32 bytes";
    let key: Pinned<[u8; 32]> = Pinned::new_with(|k: &mut [u8; 32]| k.copy_from_slice(&source));
    assert_eq!(key.expose(), &source);
    assert_eq!(Pinned::<[u8; 32]>::new_with(|_| {}).expose(), &[0u8; 32]);
}

#[test]
fn hmac_sha256_rfc4231_case_1() {
    let key = [0x0bu8; 20];
    let out = hmac_sha256(&key, b"Hi There");
    assert_eq!(
        out.to_vec(),
        unhex("b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7")
    );
}

#[test]
fn hmac_sha256_rfc4231_case_2() {
    let out = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
    assert_eq!(
        out.to_vec(),
        unhex("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
    );
}

#[test]
fn pbkdf2_hmac_sha256_rfc7914_known_answers() {
    // RFC 7914 §11, the PBKDF2-HMAC-SHA-256 test vectors.
    let mut out = [0u8; 64];
    pbkdf2_hmac_sha256(&Compute::new(|_| {}), b"passwd", b"salt", 1, &mut out);
    assert_eq!(
        out.to_vec(),
        unhex(
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc\
             49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
        )
    );

    let mut out = [0u8; 64];
    pbkdf2_hmac_sha256(
        &Compute::new(|_| {}),
        b"Password",
        b"NaCl",
        80_000,
        &mut out,
    );
    assert_eq!(
        out.to_vec(),
        unhex(
            "4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56\
             a1d425a1225833549adb841b51c9b3176a272bdebba1d078478f62b397f33c8d"
        )
    );
}

/// RFC 5869 Appendix A, test cases 1 to 3: HKDF-Expand from the PRK each
/// case states, with its info, gives the case's OKM.
#[test]
fn hkdf_sha256_expand_rfc5869_cases() {
    let info2: Vec<u8> = (0xb0..=0xff).collect();
    let cases: [(&str, &[u8], &str); 3] = [
        (
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
            &[0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9],
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf\
             34007208d5b887185865",
        ),
        (
            "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244",
            &info2,
            "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c\
             59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71\
             cc30c58179ec3e87c14c01d5c1f3434f1d87",
        ),
        (
            "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
            b"",
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d\
             9d201395faa4b61a96c8",
        ),
    ];
    for (prk, info, okm) in cases {
        let want = unhex(okm);
        let mut got = vec![0u8; want.len()];
        assert_eq!(hkdf_sha256_expand(&unhex(prk), info, &mut got), Some(()));
        assert_eq!(got, want);
    }
}

/// RFC 5869 §2.3 bounds the output at 255 blocks: one byte more is
/// refused and nothing is written.
#[test]
fn hkdf_sha256_expand_refuses_more_than_255_blocks() {
    let prk = [7u8; 32];
    let mut most = vec![0u8; HKDF_SHA256_MAX];
    assert_eq!(hkdf_sha256_expand(&prk, b"x", &mut most), Some(()));
    let mut over = vec![0u8; HKDF_SHA256_MAX + 1];
    assert_eq!(hkdf_sha256_expand(&prk, b"x", &mut over), None);
    assert!(over.iter().all(|&b| b == 0));
}

fn blake2b_vec(data: &[u8], len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    faraday_crypto::blake2b(data, &mut out);
    out
}

#[test]
fn blake2b_rfc_7693_vectors() {
    // RFC 7693 appendix A: BLAKE2b-512 of "abc".
    assert_eq!(
        blake2b_vec(b"abc", 64),
        unhex(
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
             7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
        )
    );
    // BLAKE2b-512 of the empty message.
    assert_eq!(
        blake2b_vec(b"", 64),
        unhex(
            "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419\
             d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce"
        )
    );
}

#[test]
fn blake2b_multi_block_input() {
    let data: Vec<u8> = (0..300).map(|i| (i % 251) as u8).collect();
    assert_eq!(
        blake2b_vec(&data, 48),
        unhex(
            "8babfe78d19949e35b6d53afca26e4e3020a44dcf304670af4c47579d01af707b922cbb9cd85078470e31e264f9f9726"
        )
    );
    assert_eq!(
        blake2b_vec(&data, 64),
        unhex(
            "3a482b7748b0bdc43c3d00c080890c10e57a9aa5618f78b86067eb7eaae4942a\
             cd96d827accbc16958364ae5b0df6105bbd3b15445092eba1137b5f69c1070f1"
        )
    );
}

#[test]
fn blake2b_input_of_whole_blocks() {
    // A last block that is full is compressed once, as the final block:
    // values from CPython's `hashlib.blake2b(digest_size=48)`.
    let data: Vec<u8> = (0..256).map(|i| (i % 251) as u8).collect();
    assert_eq!(
        blake2b_vec(&data[..128], 48),
        unhex(
            "a2c2acf7ce4079c02b7f38e2ef33bff531a31a7c7effe712c5348b4d616c0cba9b152679317984ec632d0c70eb11eece"
        )
    );
    assert_eq!(
        blake2b_vec(&data, 48),
        unhex(
            "21adb9e49580c4a246a658c1fc866177ea298b2962898c9a1639d57a275eee4f779994f101508b5b9f41646a06654777"
        )
    );
}

#[test]
fn blake2b_every_length_from_1_to_64() {
    // The digests of "abc" at lengths 1 to 64, end to end, hash to the
    // value CPython's `hashlib.blake2b(digest_size=n)` gives.
    let mut all = Vec::new();
    for len in 1..=64 {
        all.extend(blake2b_vec(b"abc", len));
    }
    assert_eq!(
        sha256(&all).to_vec(),
        unhex("9d1cdef0b728145136303c3d27212397502c10036c04beee860a12d3bdc6c8d0")
    );
}

#[test]
#[should_panic(expected = "1 to 64")]
fn blake2b_refuses_a_digest_of_65_bytes() {
    blake2b_vec(b"abc", 65);
}

#[test]
#[should_panic(expected = "1 to 64")]
fn blake2b_refuses_an_empty_digest() {
    blake2b_vec(b"abc", 0);
}

#[test]
fn hex_matches_rfc_4648_base16_vectors() {
    // RFC 4648 section 10, upper case there; Faraday writes lower case.
    let vectors: [(&[u8], &str); 7] = [
        (b"", ""),
        (b"f", "66"),
        (b"fo", "666f"),
        (b"foo", "666f6f"),
        (b"foob", "666f6f62"),
        (b"fooba", "666f6f6261"),
        (b"foobar", "666f6f626172"),
    ];
    for (bytes, text) in vectors {
        let mut buf = [0u8; 12];
        assert_eq!(faraday_crypto::hex::encode_lower(bytes, &mut buf), text);
        assert_eq!(faraday_crypto::hex::encode_secret(bytes).as_str(), text);
        let mut out = [0u8; 8];
        let n = faraday_crypto::hex::decode_into(text, &mut out).unwrap();
        assert_eq!(&out[..n], bytes);
        let upper = text.to_ascii_uppercase();
        let n = faraday_crypto::hex::decode_into(&upper, &mut out).unwrap();
        assert_eq!(&out[..n], bytes);
    }
}

#[test]
fn hex_refuses_odd_long_and_non_hex_text() {
    let mut out = [0u8; 2];
    assert_eq!(faraday_crypto::hex::decode_into("abc", &mut out), None);
    assert_eq!(faraday_crypto::hex::decode_into("aabbcc", &mut out), None);
    assert_eq!(faraday_crypto::hex::decode_into("zz", &mut out), None);
    assert_eq!(faraday_crypto::hex::decode_into("+f", &mut out), None);
    assert_eq!(faraday_crypto::hex::decode_into("aA", &mut out), Some(1));
}
