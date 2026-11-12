//! TOTP codes (`docs/TOTP.md` §4, §8): reading a type 6 field 6 value
//! and computing its code against the published RFC vectors.
#![allow(clippy::disallowed_methods)]

use faraday_vault::totp::{NotTotp, from_fields, grouped, normalise, parse};

const SHA1_SEED: &[u8] = b"12345678901234567890";

fn code_of(fields: &[(&str, &str)], t: u64) -> String {
    from_fields(fields)
        .expect("holds a TOTP form")
        .expect("reads")
        .code(t)
}

/// RFC 6238's SHA-1 vector read through each of KeePass 2's secret
/// fields, `otp` and older KeePassXC's pair.
#[test]
fn every_form_an_entry_holds_gives_rfc_6238_codes() {
    let b32 = base32(SHA1_SEED);
    let hex: String = SHA1_SEED.iter().map(|b| format!("{b:02x}")).collect();
    let b64 = "MTIzNDU2Nzg5MDEyMzQ1Njc4OTA=";
    let otp = format!("otpauth://totp/x?secret={b32}&digits=8");
    let eight = ("TimeOtp-Length", "8");
    let want = "94287082";
    assert_eq!(code_of(&[("otp", &otp)], 59), want);
    assert_eq!(code_of(&[("TimeOtp-Secret-Base32", &b32), eight], 59), want);
    assert_eq!(code_of(&[("TimeOtp-Secret-Hex", &hex), eight], 59), want);
    assert_eq!(code_of(&[("TimeOtp-Secret-Base64", b64), eight], 59), want);
    assert_eq!(
        code_of(&[("TimeOtp-Secret", "12345678901234567890"), eight], 59),
        want
    );
    assert_eq!(
        code_of(&[("TOTP Seed", &b32), ("TOTP Settings", "30;8")], 59),
        want
    );
    // Defaults: 30 s, 6 digits, SHA-1.
    assert_eq!(code_of(&[("TOTP Seed", &b32)], 59), "287082");
}

/// KeePass 2's algorithm and period fields, against RFC 6238's SHA-256
/// and SHA-512 vectors.
#[test]
fn keepass_two_algorithm_and_period_fields() {
    let s256 = "12345678901234567890123456789012";
    let mut s512 = "1234567890".repeat(6);
    s512.push_str("1234");
    let f256 = [
        ("TimeOtp-Secret", s256),
        ("TimeOtp-Algorithm", "HMAC-SHA-256"),
        ("TimeOtp-Length", "8"),
        ("TimeOtp-Period", "30"),
    ];
    assert_eq!(code_of(&f256, 1_111_111_109), "68084774");
    let f512 = [
        ("TimeOtp-Secret", s512.as_str()),
        ("TimeOtp-Algorithm", "HMAC-SHA-512"),
        ("TimeOtp-Length", "8"),
    ];
    assert_eq!(code_of(&f512, 1_111_111_109), "25091201");
    // RFC 4226 counter 1 at a 1-second period.
    let f = [
        ("TimeOtp-Secret", "12345678901234567890"),
        ("TimeOtp-Period", "1"),
    ];
    assert_eq!(code_of(&f, 1), "287082");
}

/// An entry with several forms is read by `otp`, then KeePass 2's, then
/// the old pair; one with none gives `None`.
#[test]
fn the_first_form_present_decides() {
    let b32 = base32(SHA1_SEED);
    let other = base32(b"abcdefghijklmnopqrst");
    let otp = format!("otpauth://totp/x?secret={b32}");
    let f = [
        ("TOTP Seed", other.as_str()),
        ("TimeOtp-Secret-Base32", other.as_str()),
        ("otp", otp.as_str()),
    ];
    assert_eq!(code_of(&f, 59), "287082");
    let f = [
        ("TOTP Seed", other.as_str()),
        ("TimeOtp-Secret-Base32", b32.as_str()),
    ];
    assert_eq!(code_of(&f, 59), "287082");
    // An `otp` that does not read is not passed over for a later form.
    let f = [("otp", "garbage"), ("TimeOtp-Secret-Base32", b32.as_str())];
    assert_eq!(from_fields(&f).unwrap().unwrap_err(), NotTotp::Unreadable);
    assert!(from_fields(&[("Title", "x"), ("otp", "  ")]).is_none());
    assert!(from_fields(&[]).is_none());
}

/// A parameter written twice, or in another spelling than lower case, is not read.
#[test]
fn a_parameter_written_twice_is_not_read() {
    let b32 = base32(SHA1_SEED);
    for twice in [
        format!("secret={b32}&secret={b32}"),
        format!("secret={b32}&issuer=A&issuer=B"),
        format!("secret={b32}&issuer=A&issuer=A"),
        format!("secret={b32}&algorithm=SHA1&algorithm=SHA1"),
        format!("secret={b32}&digits=7&digits=7"),
        format!("secret={b32}&period=60&period=30"),
        format!("secret={b32}&counter=1&counter=1"),
        format!("secret={b32}&image=a&image=b"),
        format!("secret={b32}&encoder=a&encoder=a"),
        format!("secret={b32}&s%65cret={b32}"),
        format!("secret={b32}&digits=6&d%69gits=8"),
        format!("secret={b32}&s%2565cret={b32}"),
        format!("secret={b32}&SECRET=GEZDGNBVGY3TQOJQ"),
        format!("secret={b32}&digits=6&DIGITS=8"),
        format!("secret={b32}&sec\tret=GEZDGNBVGY3TQOJQ"),
        format!("secret={b32}&d%69gits=8"),
        format!("secret={b32}&alg%6Frithm=SHA256"),
        format!("secret={b32}&p%65riod=60"),
        format!("secret={b32}&%65ncoder=steam"),
        format!("secret={b32}&ENCODER=steam"),
        format!("s%65cret={b32}"),
        format!("secret={b32}&foo bar=1"),
        format!("secret={b32}&Issuer=Foo"),
        format!("secret={b32}&Period=30"),
    ] {
        let uri = format!("otpauth://totp/x?{twice}");
        assert_eq!(
            from_fields(&[("otp", &uri)])
                .expect("a form is held")
                .unwrap_err(),
            NotTotp::Unreadable,
            "{uri}"
        );
    }
}

/// HOTP, a Steam seed and each malformed field are told apart.
#[test]
fn hotp_steam_and_malformed_forms_are_not_read() {
    let b32 = base32(SHA1_SEED);
    let err = |f: &[(&str, &str)]| from_fields(f).expect("a form is held").unwrap_err();
    let steam = format!("otpauth://totp/x?secret={b32}&encoder=steam");
    assert_eq!(err(&[("otp", &steam)]), NotTotp::Unreadable);
    let steam = format!("otpauth://totp/x?secret={b32}&encoder=STEAM");
    assert_eq!(err(&[("otp", &steam)]), NotTotp::Unreadable);
    let other = format!("otpauth://totp/x?secret={b32}&encoder=other");
    assert!(
        from_fields(&[("otp", &other)])
            .expect("a form is held")
            .is_ok()
    );
    let hotp = "otpauth://hotp/x?secret=JBSWY3DPEHPK3PXP&counter=1";
    assert_eq!(err(&[("otp", hotp)]), NotTotp::Hotp);
    assert_eq!(err(&[("HmacOtp-Secret-Base32", &b32)]), NotTotp::Hotp);
    let bad: &[&[(&str, &str)]] = &[
        &[("TOTP Seed", &b32), ("TOTP Settings", "30;S")],
        &[("TOTP Seed", "not base32!")],
        &[("TimeOtp-Secret-Hex", "abc")],
        &[("TimeOtp-Secret-Hex", "zz")],
        &[("TimeOtp-Secret-Base64", "!!!")],
        &[("TimeOtp-Secret", "x"), ("TimeOtp-Algorithm", "HMAC-MD5")],
        &[("TimeOtp-Secret", "x"), ("TimeOtp-Length", "9")],
        &[("TimeOtp-Secret", "x"), ("TimeOtp-Period", "0")],
        &[("TimeOtp-Secret", "x"), ("TimeOtp-Period", "ab")],
    ];
    for f in bad {
        assert_eq!(err(f), NotTotp::Unreadable, "{f:?}");
    }
    // KeePassXC reads settings of two parts or more: a period alone is the
    // defaults, and a third part is ignored.
    let one = [("TOTP Seed", b32.as_str()), ("TOTP Settings", "60")];
    assert_eq!(code_of(&one, 59), "287082");
    let three = [("TOTP Seed", b32.as_str()), ("TOTP Settings", "30;8;x")];
    let eight = [("TOTP Seed", b32.as_str()), ("TOTP Settings", "30;8")];
    assert_eq!(code_of(&three, 59), code_of(&eight, 59));
    // KeePass 2's plain secret is its bytes as written, spaces included.
    assert_eq!(
        code_of(
            &[("TimeOtp-Secret", "  "), ("TimeOtp-Secret-Hex", "00")],
            59
        )
        .len(),
        6
    );
    assert_eq!(
        err(&[("TimeOtp-Secret-Base32", "   ")]),
        NotTotp::Unreadable
    );
}

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
