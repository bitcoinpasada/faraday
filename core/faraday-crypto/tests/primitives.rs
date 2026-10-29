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
fn secret_constant_time_eq() {
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
