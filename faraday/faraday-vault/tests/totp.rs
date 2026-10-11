//! TOTP codes (`docs/TOTP.md` §4, §8): reading a type 6 field 6 value
//! and computing its code against the published RFC vectors.

use faraday_vault::totp::{NotTotp, grouped, normalise, parse};

/// A Base32 encoding of an ASCII seed, built here rather than pulled
/// from a crate (`docs/TOTP.md` §4 names no Base32 dependency).
fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut bits: u32 = 0;
    let mut bit_count = 0u32;
    for &b in bytes {
        bits = (bits << 8) | u32::from(b);
        bit_count += 8;
        while bit_count >= 5 {
            bit_count -= 5;
            out.push(ALPHABET[((bits >> bit_count) & 0x1f) as usize] as char);
        }
    }
    if bit_count > 0 {
        out.push(ALPHABET[((bits << (5 - bit_count)) & 0x1f) as usize] as char);
    }
    out
}

fn uri(secret: &[u8], algorithm: &str, digits: u32) -> String {
    format!(
        "otpauth://totp/x?secret={}&algorithm={algorithm}&digits={digits}",
        base32(secret)
    )
}

/// RFC 6238 Appendix B, 8 digits, 30-second period.
#[test]
fn rfc_6238_appendix_b_vectors() {
    let sha1_seed = b"12345678901234567890".to_vec();
    let sha256_seed = b"12345678901234567890123456789012".to_vec();
    let mut sha512_seed = b"1234567890".repeat(6);
    sha512_seed.extend_from_slice(b"1234");

    let cases: &[(u64, &str, &str, &str)] = &[
        (59, "94287082", "46119246", "90693936"),
        (1_111_111_109, "07081804", "68084774", "25091201"),
        (1_111_111_111, "14050471", "67062674", "99943326"),
        (1_234_567_890, "89005924", "91819424", "93441116"),
        (2_000_000_000, "69279037", "90698825", "38618901"),
        (20_000_000_000, "65353130", "77737706", "47863826"),
    ];

    let sha1 = parse(&uri(&sha1_seed, "SHA1", 8)).expect("reads");
    let sha256 = parse(&uri(&sha256_seed, "SHA256", 8)).expect("reads");
    let sha512 = parse(&uri(&sha512_seed, "SHA512", 8)).expect("reads");

    for (t, s1, s256, s512) in cases {
        assert_eq!(sha1.code(*t), *s1, "SHA1 at {t}");
        assert_eq!(sha256.code(*t), *s256, "SHA256 at {t}");
        assert_eq!(sha512.code(*t), *s512, "SHA512 at {t}");
    }
}

/// RFC 4226 Appendix D, SHA1, 6 digits: counters 0-9 read directly from
/// times 0-9 at a 1-second period.
#[test]
fn rfc_4226_appendix_d_vectors() {
    let seed = b"12345678901234567890".to_vec();
    let t = parse(&format!(
        "otpauth://totp/x?secret={}&algorithm=SHA1&digits=6&period=1",
        base32(&seed)
    ))
    .expect("reads");
    let expected = [
        "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583", "399871",
        "520489",
    ];
    for (i, code) in expected.iter().enumerate() {
        assert_eq!(t.code(i as u64), *code, "counter {i}");
    }
}

/// The key-URI example from the spec: typed as a full URI, and as the
/// same secret typed with spaces, give the same code.
#[test]
fn the_key_uri_example_and_its_typed_form_agree() {
    let full = "otpauth://totp/ACME%20Co:john.doe@email.com?secret=HXDMVJECJJWSRB3HWIZR4IFUGFTMXBOZ&issuer=ACME%20Co&algorithm=SHA1&digits=6&period=30";
    let typed = "hxdm vjec jjws rb3h wizr 4ifu gftm xboz";
    let a = parse(full).expect("the full URI reads");
    let b = parse(typed).expect("the typed key reads");
    assert_eq!(a.code(1_234_567_890), b.code(1_234_567_890));
}

/// HOTP, an unknown digit count, an unknown algorithm, an out-of-range
/// period, and a key with digits in it are all refused.
#[test]
fn unreadable_and_hotp_values_are_told_apart() {
    assert_eq!(
        parse("otpauth://hotp/x?secret=JBSWY3DPEHPK3PXP").unwrap_err(),
        NotTotp::Hotp
    );
    assert_eq!(
        parse("otpauth://totp/x?secret=JBSWY3DPEHPK3PXP&digits=9").unwrap_err(),
        NotTotp::Unreadable
    );
    assert_eq!(
        parse("otpauth://totp/x?secret=JBSWY3DPEHPK3PXP&algorithm=MD5").unwrap_err(),
        NotTotp::Unreadable
    );
    assert_eq!(
        parse("otpauth://totp/x?secret=JBSWY3DPEHPK3PXP&period=0").unwrap_err(),
        NotTotp::Unreadable
    );
    assert_eq!(parse("JBSWY3DP18PK3PXP").unwrap_err(), NotTotp::Unreadable);
}

/// `normalise` of a bare key reads back with the same code as the key
/// typed straight into `parse`; of garbage it is `None`.
#[test]
fn normalise_of_a_bare_key_and_of_garbage() {
    let bare = "JBSW Y3DP EHPK 3PXP";
    let direct = parse(bare).expect("reads");
    let stored = normalise(bare, "Example", "octo").expect("a bare key normalises");
    let via_store = parse(&stored).expect("the stored URI reads");
    assert_eq!(direct.code(1_000_000), via_store.code(1_000_000));
    assert!(stored.starts_with("otpauth://totp/Example:octo?secret=JBSWY3DPEHPK3PXP"));

    assert_eq!(normalise("JBSWY3DP18PK3PXP", "Example", ""), None);
    // A word or a password typed into the field is not a key, though its
    // letters are all Base32.
    assert_eq!(normalise("hello", "Example", ""), None);
    assert_eq!(normalise("correcthorse", "Example", ""), None);
}

/// `grouped` matches the digit count (§4.2).
#[test]
fn grouped_by_digit_count() {
    assert_eq!(grouped("492039"), "492 039");
    assert_eq!(grouped("1234567"), "1 234 567");
    assert_eq!(grouped("12345678"), "1234 5678");
}
