//! KEF as a person or a Krux device would notice it (`docs/KEF.md` §14):
//! Krux's own codes open, every version makes a code it opens again, what
//! Krux refuses to write is refused, what is not a code is not taken for
//! one, and a count of 100,000,000 stops when told.

use faraday_crypto::work::Compute;
use faraday_kef::*;

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-kef's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(|_| {})
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../tools/vectors/kef/vectors.json")).unwrap()
}

fn open_vector(v: &serde_json::Value, passphrase: &str) -> Result<Opened, OpenError> {
    let env = unwrap(&unhex(v["envelope"].as_str().unwrap())).unwrap();
    assert_eq!(env.count, v["count"].as_u64().unwrap() as u32);
    assert_eq!(u64::from(env.version), v["version"].as_u64().unwrap());
    open(&compute(), passphrase.as_bytes(), &env, &mut |_| true)
}

#[test]
fn krux_ctr_code_opens_to_its_entropy() {
    let v = vectors();
    let got = open_vector(&v["ctr"], v["ctr"]["passphrase"].as_str().unwrap()).unwrap();
    assert_eq!(
        *got.plaintext,
        unhex(v["ctr"]["plaintext"].as_str().unwrap())
    );
    assert!(!got.retried);
}

#[test]
fn krux_gcm_code_opens_to_its_entropy() {
    let v = vectors();
    let got = open_vector(&v["gcm"], v["gcm"]["passphrase"].as_str().unwrap()).unwrap();
    assert_eq!(
        *got.plaintext,
        unhex(v["gcm"]["plaintext"].as_str().unwrap())
    );
}

#[test]
fn krux_gcm_code_with_a_wrong_passphrase_or_a_changed_byte_does_not_open() {
    let v = vectors();
    assert_eq!(
        open_vector(&v["gcm"], "test kez").err(),
        Some(OpenError::Failed)
    );
    let mut bytes = unhex(v["gcm"]["envelope"].as_str().unwrap());
    bytes[20] ^= 1;
    let env = unwrap(&bytes).unwrap();
    assert!(open(&compute(), b"test key", &env, &mut |_| true).is_err());
}

#[test]
fn krux_version_5_code_opens_and_a_wrong_passphrase_that_passes_gives_other_bytes() {
    let v = vectors();
    let g = &v["grind"];
    let right = open_vector(g, g["right"]["passphrase"].as_str().unwrap()).unwrap();
    assert_eq!(*right.plaintext, b"I'm 16 raw bytes");
    let wrong = open_vector(g, g["wrong_that_passes"]["passphrase"].as_str().unwrap()).unwrap();
    assert_eq!(
        *wrong.plaintext,
        unhex(g["wrong_that_passes"]["plaintext"].as_str().unwrap())
    );
    assert_ne!(*wrong.plaintext, *right.plaintext);
    assert_eq!(
        open_vector(g, "abd").err(),
        Some(OpenError::Failed),
        "most wrong passphrases do not pass"
    );
}

const KEY: [u8; 32] = [7; 32];

fn iv_for(v: u8) -> Vec<u8> {
    (0..spec_of(v).unwrap().iv_len() as u8)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect()
}

fn seal_k(plain: &[u8], v: u8) -> Result<Vec<u8>, SealError> {
    seal_with_key(&KEY, plain, v, b"an id", 10_000, &iv_for(v))
}

fn open_k(bytes: &[u8]) -> Result<Opened, OpenError> {
    open_with_key(&KEY, &unwrap(bytes).unwrap())
}

#[test]
fn every_version_opens_what_it_seals() {
    let entropy16: Vec<u8> = (1..=16).collect();
    let entropy32: Vec<u8> = (100..132).collect();
    let text = b"a longer note, with words in it, that goes past one block of sixteen bytes and a bit more";
    let wallet: Vec<u8> =
        b"wsh(sortedmulti(2,[aabbccdd/48h/0h/0h/2h]xpub1,[11223344/48h/0h/0h/2h]xpub2))"
            .iter()
            .cycle()
            .take(700)
            .copied()
            .collect();
    for v in VERSIONS {
        for plain in [&entropy16[..], &entropy32[..], &text[..], &wallet[..]] {
            let code = match seal_k(plain, v) {
                Ok(c) => c,
                Err(SealError::Duplicate { instead }) => {
                    assert!(spec_of(instead).is_some());
                    continue;
                }
                Err(SealError::TooLong) => {
                    assert!(plain.len() > COMPRESS_MAX);
                    continue;
                }
                Err(e) => panic!("version {v}: {e:?}"),
            };
            let got = open_k(&code).unwrap();
            assert_eq!(*got.plaintext, plain, "version {v}, {} bytes", plain.len());
            assert!(!got.retried);
        }
    }
}

#[test]
fn a_seed_sized_code_is_as_small_as_krux_writes_it() {
    // Version 20, an 8-byte id, 12 words (16 bytes): `docs/KEF.md` §5.
    let code = seal_with_key(&KEY, &[9; 16], 20, &[1; 8], 100_000, &[2; 12]).unwrap();
    assert_eq!(code.len(), 45);
    let code = seal_with_key(&KEY, &[9; 32], 20, &[1; 8], 100_000, &[2; 12]).unwrap();
    assert_eq!(code.len(), 61);
    let code = seal_with_key(
        &KEY,
        &(1..=32).collect::<Vec<u8>>(),
        5,
        &[1; 8],
        100_000,
        &[],
    )
    .unwrap();
    assert_eq!(code.len(), 48);
}

#[test]
fn what_krux_refuses_to_write_is_refused_and_names_a_version_that_works() {
    // Two equal blocks in ECB.
    assert!(matches!(
        seal_k(&[5; 32], 5),
        Err(SealError::Duplicate { instead: 10 })
    ));
    assert!(matches!(
        seal_k(&[5; 48], 6),
        Err(SealError::Duplicate { instead: 11 })
    ));
    // NUL padding and a plaintext ending in NUL, with an exposed check.
    assert!(matches!(
        seal_k(&[1, 2, 3, 0], 5),
        Err(SealError::NulEnd { instead: 6 })
    ));
    assert!(matches!(
        seal_k(&[1, 2, 3, 0], 10),
        Err(SealError::NulEnd { instead: 11 })
    ));
    assert_eq!(seal_k(&[], 20), Err(SealError::Empty));
    assert_eq!(
        seal_k(&vec![1; COMPRESS_MAX + 1], 21),
        Err(SealError::TooLong)
    );
    assert_eq!(
        seal_with_key(&KEY, &[1; 16], 20, b"id", 10_000, &[0; 16]),
        Err(SealError::Iv)
    );
    assert_eq!(
        seal_with_key(&KEY, &[1; 16], 5, b"id", 10_000, &[0; 12]),
        Err(SealError::Iv)
    );
}

#[test]
fn version_0_refuses_the_one_in_256_whose_check_ends_in_nul() {
    let mut refused = 0;
    for n in 0u16..1024 {
        let plain = [n as u8, (n >> 8) as u8, 7, 7];
        match seal_k(&plain, 0) {
            Ok(code) => assert_eq!(*open_k(&code).unwrap().plaintext, plain),
            Err(SealError::NulEnd { .. }) => refused += 1,
            Err(e) => panic!("{e:?}"),
        }
    }
    assert!((1..=10).contains(&refused), "{refused} of 1024");
}

#[test]
fn ids_and_counts_are_written_as_krux_writes_them() {
    for id in [&[][..], &[3u8; 8][..], &[4u8; 252][..]] {
        let code = seal_with_key(&KEY, &[1; 16], 20, id, 100_000, &[0; 12]).unwrap();
        let env = unwrap(&code).unwrap();
        assert_eq!(env.id, id);
        assert_eq!(env.count, 100_000);
    }
    assert_eq!(
        seal_with_key(&KEY, &[1; 16], 20, &[0; 253], 100_000, &[0; 12]),
        Err(SealError::Refused(Refusal::IdLength))
    );
    for count in [10_000, 20_000, 100_000_000, 10_001, 550_000, 16_777_215] {
        let code = seal_with_key(&KEY, &[1; 16], 20, b"id", count, &[0; 12]).unwrap();
        assert_eq!(unwrap(&code).unwrap().count, count);
    }
    for count in [0, 9_999, 16_777_216] {
        assert_eq!(
            seal_with_key(&KEY, &[1; 16], 20, b"id", count, &[0; 12]),
            Err(SealError::Refused(Refusal::Count)),
            "{count}"
        );
    }
}

#[test]
fn things_that_are_not_codes_are_refused() {
    let good = seal_k(&[1; 16], 20).unwrap();
    assert!(unwrap(&good).is_ok());
    // A count of 0.
    let mut b = good.clone();
    let at = 1 + b[0] as usize + 1;
    b[at..at + 3].fill(0);
    assert_eq!(unwrap(&b), Err(Refusal::Count));
    // An id over 252 bytes.
    let mut b = good.clone();
    b[0] = 253;
    assert_eq!(unwrap(&b), Err(Refusal::IdLength));
    // A version KEF does not have.
    let mut b = good.clone();
    let at = 1 + b[0] as usize;
    for v in [2u8, 3, 4, 8, 9, 13, 14, 17, 18, 19, 22, 255] {
        b[at] = v;
        assert_eq!(unwrap(&b), Err(Refusal::Version), "{v}");
    }
    // A payload of the wrong length: ECB not in whole blocks, none at all.
    let code = seal_k(&(1..=16).collect::<Vec<u8>>(), 5).unwrap();
    assert!(unwrap(&code).is_ok());
    assert_eq!(unwrap(&code[..code.len() - 1]), Err(Refusal::Payload));
    let header = 1 + 5 + 1 + 3;
    assert_eq!(unwrap(&code[..header]), Err(Refusal::Payload));
    assert!(unwrap(&[]).is_err());
}

#[test]
fn a_plaintext_that_ends_in_nul_opens_from_a_writer_that_skipped_the_rule() {
    // Version 5 is NUL-padded with a check over the plaintext and the key.
    // Make the code the way such a writer would: the plaintext's trailing
    // NUL is stripped on reading, so the check fails at first and passes
    // once the NUL is put back, and the person is told.
    use aes::cipher::{BlockCipherEncrypt, KeyInit};
    use sha2::{Digest, Sha256};
    let plain = [9u8, 8, 7, 6, 5, 4, 3, 2, 1, 0];
    let mut h = Sha256::new();
    h.update([5u8]);
    h.update(plain);
    h.update(KEY);
    let check = h.finalize();
    let mut block = [0u8; 16];
    block[..plain.len()].copy_from_slice(&plain);
    let cipher = aes::Aes256::new_from_slice(&KEY).unwrap();
    let mut b = aes::Block::from(block);
    cipher.encrypt_block(&mut b);
    let mut payload = b.to_vec();
    payload.extend_from_slice(&check[..3]);
    let code = faraday_codec::kef::wrap(b"id", 5, 10_000, &payload).unwrap();
    let got = open_k(&code).unwrap();
    assert_eq!(*got.plaintext, plain);
    assert!(got.retried);
}

#[test]
fn a_repetitive_text_is_sealed_compressed_and_opens_again_in_every_compressed_version() {
    let wallet: Vec<u8> = b"0123456789abcdef"
        .iter()
        .cycle()
        .take(1000)
        .copied()
        .collect();
    for v in [7, 12, 16, 21] {
        let code = seal_k(&wallet, v).unwrap();
        assert!(code.len() < 200, "{v}: {} bytes", code.len());
        assert_eq!(*open_k(&code).unwrap().plaintext, wallet);
    }
}

#[test]
fn a_hidden_check_ending_in_nul_opens_from_a_writer_that_skipped_the_rule() {
    // Version 0 is NUL-padded with a 16-byte check hidden after the
    // plaintext. A writer that skipped the refusal of a check ending in 0x00
    // makes a code whose check loses that byte on reading; it opens once the
    // byte is put back, and the person is told.
    use aes::cipher::{BlockCipherEncrypt, KeyInit};
    use sha2::{Digest, Sha256};
    let (plain, check) = (0u16..)
        .map(|n| {
            let plain = vec![n as u8, (n >> 8) as u8, 7, 7];
            let check: [u8; 32] = Sha256::digest(&plain).into();
            (plain, check)
        })
        .find(|(_, check)| check[15] == 0)
        .unwrap();
    let mut data = plain.clone();
    data.extend_from_slice(&check[..16]);
    data.resize(data.len().div_ceil(16) * 16, 0);
    let cipher = aes::Aes256::new_from_slice(&KEY).unwrap();
    let mut payload = Vec::new();
    for chunk in data.chunks(16) {
        let mut b = aes::Block::try_from(chunk).unwrap();
        cipher.encrypt_block(&mut b);
        payload.extend_from_slice(&b);
    }
    let code = faraday_codec::kef::wrap(b"id", 0, 10_000, &payload).unwrap();
    let got = open_k(&code).unwrap();
    assert_eq!(*got.plaintext, plain);
    assert!(got.retried);
}

#[test]
fn a_code_that_inflates_past_100_kib_does_not_open() {
    // Seal the deflated bytes as an uncompressed version's plaintext and
    // read them as the compressed one: same key, same layout.
    let big = vec![0u8; INFLATE_LIMIT + 1];
    let z = miniz_oxide::deflate::compress_to_vec(&big, 9);
    let code = seal_k(&z, 15).unwrap();
    let mut b = code.clone();
    let at = 1 + b[0] as usize;
    b[at] = 16;
    // Version 16 hashes the compressed bytes the same way as 15.
    assert_eq!(open_k(&b).err(), Some(OpenError::Failed));
    let ok = vec![0u8; INFLATE_LIMIT];
    let z = miniz_oxide::deflate::compress_to_vec(&ok, 9);
    let mut b = seal_k(&z, 15).unwrap();
    b[at] = 16;
    assert_eq!(open_k(&b).unwrap().plaintext.len(), INFLATE_LIMIT);
}

#[test]
fn a_count_of_100_million_stops_when_told() {
    let c = compute();
    let code = seal_with_key(&KEY, &[1; 16], 20, b"id", 100_000_000, &[0; 12]).unwrap();
    let env = unwrap(&code).unwrap();
    assert_eq!(env.count, 100_000_000);
    let mut calls = 0;
    let out = open(&c, b"pw", &env, &mut |_| {
        calls += 1;
        calls < 4
    });
    assert_eq!(out.err(), Some(OpenError::Cancelled));
    assert_eq!(calls, 4);
    let out = seal(
        &c,
        b"pw",
        &[1; 16],
        &Sealing {
            version: 20,
            id: b"id",
            count: COUNT_KRUX_MAX,
            iv: &[0; 12],
        },
        &mut |_| false,
    );
    assert_eq!(out, Err(SealError::Cancelled));
}

#[test]
fn a_code_made_with_a_passphrase_opens_with_it_only() {
    let c = compute();
    for v in VERSIONS {
        let plain: Vec<u8> = (200..232).collect();
        let code = seal(
            &c,
            b"correct horse",
            &plain,
            &Sealing {
                version: v,
                id: b"id",
                count: 10_000,
                iv: &iv_for(v),
            },
            &mut |_| true,
        )
        .unwrap();
        let env = unwrap(&code).unwrap();
        let got = open(&c, b"correct horse", &env, &mut |_| true).unwrap();
        assert_eq!(*got.plaintext, plain, "version {v}");
        assert_eq!(
            open(&c, b"correct horsf", &env, &mut |_| true).err(),
            Some(OpenError::Failed),
            "version {v}"
        );
    }
}

#[test]
fn passphrases_are_printable_ascii_to_write_and_three_forms_to_read() {
    assert!(writable_passphrase("a b~"));
    assert!(!writable_passphrase(""));
    assert!(!writable_passphrase("caf\u{e9}"));
    assert!(!writable_passphrase("tab\t"));
    assert!(writable_passphrase(&"x".repeat(PASSPHRASE_MAX)));
    assert!(!writable_passphrase(&"x".repeat(PASSPHRASE_MAX + 1)));
    assert_eq!(reading_passphrases("plain").len(), 1);
    let forms = reading_passphrases("caf\u{e9}");
    assert_eq!(forms.len(), 2);
    assert_eq!(*forms[0], "caf\u{e9}".as_bytes());
    assert_eq!(*forms[1], "cafe\u{301}".as_bytes());
    assert_eq!(reading_passphrases("cafe\u{301}").len(), 2);
}

#[test]
fn versions_are_suggested_as_krux_suggests_them() {
    let seed: Vec<u8> = (1..=16).collect();
    assert_eq!(suggest_versions(&seed, Mode::Gcm), [20]);
    assert_eq!(suggest_versions(&seed, Mode::Ecb), [5]);
    assert_eq!(suggest_versions(&seed, Mode::Cbc), [10]);
    let long: Vec<u8> = (0..200u32).map(|i| (i * 7 % 251) as u8).collect();
    assert_eq!(suggest_versions(&long, Mode::Gcm), [21]);
    assert!(suggest_versions(&long, Mode::Ecb).contains(&7));
    assert!(suggest_versions(&[], Mode::Gcm).is_empty());
    for mode in [Mode::Ecb, Mode::Cbc] {
        assert!(!suggest_versions(&seed, mode).iter().any(|&v| v <= 1));
    }
}

#[test]
fn the_count_krux_adds_stays_under_a_tenth() {
    assert_eq!(count_with_change(100_000, 0), 100_000);
    assert_eq!(count_with_change(100_000, 9_999), 109_999);
    assert_eq!(count_with_change(100_000, 10_000), 100_000);
    assert!(count_with_change(COUNT_SETTING_MAX, u32::MAX) < COUNT_KRUX_MAX);
}

#[test]
fn a_scanned_code_is_known_by_its_text_or_bytes_and_other_things_are_not() {
    use faraday_codec::kef::{Form, encode};
    let code = seal_with_key(&KEY, &[9; 16], 20, &[1; 8], 100_000, &[2; 12]).unwrap();
    for form in [Form::Hex, Form::Base32, Form::Base43, Form::Base64] {
        let text = encode(form, &code).unwrap();
        let r = recognize(text.as_bytes()).unwrap_or_else(|| panic!("{form:?}: {text}"));
        assert_eq!(r.bytes, code, "{form:?}");
        assert_eq!(r.envelope.version, 20);
    }
    let r = recognize(&code).unwrap();
    assert_eq!(r.form, Form::Raw);
    // Lower-case hex is read, mixed case is not.
    let hex = encode(Form::Hex, &code).unwrap();
    assert!(recognize(hex.to_lowercase().as_bytes()).is_some());
    let mut mixed = hex.to_lowercase().into_bytes();
    let at = mixed.iter().position(|c| c.is_ascii_lowercase()).unwrap();
    mixed[at] = mixed[at].to_ascii_uppercase();
    assert!(recognize(&mixed).is_none());
    // Other kinds of codes are not KEF.
    for other in [
        &b"crush inherit small egg include title slogan mom remain blouse boost bonus"[..],
        b"bitcoin:bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        b"ur:crypto-psbt/hdcxlkahssqzwfvslofzoxwkrewngotktbmwjkwdcmnefsaaehrnkb",
        b"psbt\xff\x01\x00",
        b"010203040506070809101112131415161718192021222324",
        b"ms10testsxxxxxxxxxxxxxxxxxxxxxxxxxxqrsgwg9yvrgg",
        b"",
        b"hello",
    ] {
        assert!(
            recognize(other).is_none(),
            "{}",
            String::from_utf8_lossy(other)
        );
    }
}

fn sealing<'a>(version: u8, count: u32, iv: &'a [u8]) -> Sealing<'a> {
    Sealing {
        version,
        id: b"id",
        count,
        iv,
    }
}

#[test]
fn a_passphrase_or_count_krux_would_not_write_is_refused_before_any_work() {
    let c = compute();
    let mut calls = 0;
    let mut work = |_| {
        calls += 1;
        true
    };
    let iv = [0u8; 12];
    for (pass, count) in [
        (&b""[..], 100_000),
        (&b"caf\xc3\xa9"[..], 100_000),
        (&b"tab\t"[..], 100_000),
        (&[b'x'; 201][..], 100_000),
        (&b"fine"[..], 550_001),
        (&b"fine"[..], 100_000_000),
    ] {
        let out = seal(&c, pass, &[1; 16], &sealing(20, count, &iv), &mut work);
        assert!(
            matches!(
                out,
                Err(SealError::Passphrase | SealError::Refused(Refusal::Count))
            ),
            "{out:?}"
        );
    }
    // What needs no key is refused before the key: a compressed text too
    // long, an id too long, a plaintext whose ECB blocks repeat.
    let long = vec![7u8; COMPRESS_MAX + 1];
    assert_eq!(
        seal(&c, b"fine", &long, &sealing(21, 10_000, &iv), &mut work),
        Err(SealError::TooLong)
    );
    let mut how = sealing(20, 10_000, &iv);
    let id = [1u8; 253];
    how.id = &id;
    assert_eq!(
        seal(&c, b"fine", &[1; 16], &how, &mut work),
        Err(SealError::Refused(Refusal::IdLength))
    );
    assert!(matches!(
        seal(&c, b"fine", &[3; 32], &sealing(5, 10_000, &[]), &mut work),
        Err(SealError::Duplicate { .. })
    ));
    assert!(matches!(
        seal(&c, b"fine", &[3, 0], &sealing(5, 10_000, &[]), &mut work),
        Err(SealError::NulEnd { .. })
    ));
    assert_eq!(calls, 0, "no PBKDF2 ran for a refusal");
    // The top of what Krux writes is accepted.
    assert!(
        seal(
            &c,
            b"fine",
            &[1; 16],
            &sealing(20, COUNT_KRUX_MAX, &iv),
            &mut |_| false
        ) == Err(SealError::Cancelled)
    );
}

#[test]
fn a_text_over_the_compression_limit_is_offered_a_version_that_seals() {
    let text: Vec<u8> = (0..1100u32).map(|i| (i * 7 % 251) as u8).collect();
    for mode in [Mode::Ecb, Mode::Cbc, Mode::Ctr, Mode::Gcm] {
        let versions = suggest_versions(&text, mode);
        assert!(!versions.is_empty(), "{mode:?}");
        assert!(
            seal_k(&text, versions[0]).is_ok(),
            "{mode:?}: {:?}",
            seal_k(&text, versions[0])
        );
    }
}
