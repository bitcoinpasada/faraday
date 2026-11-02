//! Passphrase files in the age format (`faraday_vault::age`): the C2SP
//! testkit's scrypt vectors (`tools/vectors/age/`), a file sealed here
//! opening here, and each way a file is refused.

use std::cell::Cell;
use std::path::PathBuf;

use chacha20poly1305::aead::inout::InOutBuf;
use chacha20poly1305::{AeadInOut, ChaCha20Poly1305, KeyInit, Nonce};
use faraday_crypto::work::{Compute, Work};
use faraday_crypto::{hkdf_sha256_expand, hmac_sha256, sha256};
use faraday_vault::age::{Error, Kind, open, recognise, seal};

const PASSPHRASE: &[u8] = b"correct horse battery staple";
/// Salt, file key and payload nonce.
const RANDOM: [u8; 48] = *b"0123456789abcdef-the-file-key-!!-payload-nonce-!";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(count)
}

thread_local! {
    /// The key-stretching runs this test's thread has counted.
    static STRETCHES: Cell<usize> = const { Cell::new(0) };
}

fn count(work: Work) {
    if matches!(work, Work::Kdf) {
        STRETCHES.set(STRETCHES.get() + 1);
    }
}

fn sealed(plaintext: &[u8]) -> Vec<u8> {
    seal(&compute(), PASSPHRASE, 16, &RANDOM, plaintext)
}

fn opened(file: &[u8]) -> Result<Vec<u8>, Error> {
    open(&compute(), PASSPHRASE, file, 20).map(|p| p.to_vec())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// --- The testkit's vectors ------------------------------------------------

#[test]
fn the_testkit_scrypt_vectors_give_their_results() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/age/testdata");
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .expect("the vendored vectors")
        .map(|e| e.expect("entry").file_name().into_string().expect("name"))
        .collect();
    names.sort();
    assert!(names.len() >= 20, "the vectors are there");
    for name in names {
        let bytes = std::fs::read(dir.join(&name)).expect("a vector");
        let split = bytes
            .windows(2)
            .position(|w| w == b"\n\n")
            .expect("a vector has a header");
        let keys = std::str::from_utf8(&bytes[..split]).expect("text");
        let file = &bytes[split + 2..];
        let value = |key: &str| {
            keys.lines()
                .find_map(|l| l.strip_prefix(&format!("{key}: ")))
                .map(str::to_owned)
        };
        let expect = value("expect").expect("expect");
        let passphrase = value("passphrase").expect("passphrase");
        let result = open(&compute(), passphrase.as_bytes(), file, 20);
        let ok = match expect.as_str() {
            "success" if value("armored").is_some() => {
                result.as_ref().err() == Some(&Error::Armored)
            }
            "success" => result
                .as_ref()
                .is_ok_and(|p| hex(&sha256(p)) == value("payload").expect("payload")),
            "no match" => matches!(result, Err(Error::WrongPassphrase | Error::OtherRecipient)),
            "header failure" => matches!(
                result,
                Err(Error::Header
                    | Error::NotAge
                    | Error::ScryptNotAlone
                    | Error::ScryptStanza
                    | Error::TooCostly { .. })
            ),
            other => panic!("{name}: unknown expectation {other}"),
        };
        assert!(ok, "{name}: expected {expect}, got {:?}", result.err());
    }
}

#[test]
fn the_testkit_files_are_told_apart() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/vectors/age/testdata");
    let kind = |name: &str| {
        let bytes = std::fs::read(dir.join(name)).expect("a vector");
        let split = bytes.windows(2).position(|w| w == b"\n\n").expect("header");
        recognise(&bytes[split + 2..])
    };
    assert_eq!(kind("scrypt"), Kind::Scrypt { log_n: 10 });
    assert_eq!(kind("scrypt_and_x25519"), Kind::OtherRecipient);
    assert_eq!(kind("scrypt_double"), Kind::OtherRecipient);
    assert_eq!(kind("scrypt_uppercase"), Kind::OtherRecipient);
    assert_eq!(kind("armor_scrypt"), Kind::Armored);
    assert_eq!(kind("scrypt_work_factor_23"), Kind::Scrypt { log_n: 23 });
    assert_eq!(kind("scrypt_salt_short"), Kind::OtherRecipient);
}

#[test]
fn recognise_names_what_is_not_an_age_file() {
    assert_eq!(recognise(b""), Kind::NotAge);
    assert_eq!(recognise(b"FVLT and then some bytes"), Kind::NotAge);
    assert_eq!(recognise(b"age-encryption.org/v1\n-> scrypt"), Kind::NotAge);
    assert_eq!(recognise(&sealed(b"x")), Kind::Scrypt { log_n: 16 });
}

// --- A file sealed here ---------------------------------------------------

#[test]
fn a_sealed_file_opens_to_what_was_sealed() {
    for len in [0usize, 1, 65535, 65536, 65537, 131072, 131077] {
        let plain: Vec<u8> = (0..len).map(|i| (i * 7 % 251) as u8).collect();
        assert_eq!(opened(&sealed(&plain)).as_deref(), Ok(&plain[..]), "{len}");
    }
}

#[test]
fn a_sealed_file_is_binary_and_the_same_each_time() {
    let file = sealed(b"hello");
    assert!(file.starts_with(b"age-encryption.org/v1\n-> scrypt "));
    assert_eq!(file, sealed(b"hello"));
}

#[test]
fn a_work_factor_outside_sixteen_to_twenty_is_not_sealed() {
    for log_n in [0u8, 15, 21] {
        let r = std::panic::catch_unwind(|| seal(&compute(), PASSPHRASE, log_n, &RANDOM, b""));
        assert!(r.is_err(), "{log_n}");
    }
}

// --- Refusals -------------------------------------------------------------

#[test]
fn a_wrong_passphrase_is_its_own_refusal() {
    let file = sealed(b"secret");
    assert_eq!(
        open(&compute(), b"correct horse battery stapl", &file, 20).err(),
        Some(Error::WrongPassphrase)
    );
}

#[test]
fn a_changed_header_byte_is_refused() {
    let file = sealed(b"secret");
    let mac_at = file.windows(5).position(|w| w == b"\n--- ").expect("mac") + 8;
    let body_at = file.windows(2).position(|w| w == b"6\n").expect("log_n") + 5;
    let version_at = 3;
    let cases = [
        (mac_at, Error::HeaderMac),
        (body_at, Error::WrongPassphrase),
        (version_at, Error::NotAge),
    ];
    for (at, want) in cases {
        let mut f = file.clone();
        f[at] = if f[at] == b'A' { b'B' } else { b'A' };
        assert_eq!(opened(&f), Err(want), "byte {at}");
    }
}

#[test]
fn a_changed_payload_byte_is_refused() {
    let file = sealed(&vec![9u8; 70_000]);
    let header_end = header_len(&file);
    for at in [header_end + 3, header_end + 20, file.len() - 1] {
        let mut f = file.clone();
        f[at] ^= 1;
        assert_eq!(opened(&f), Err(Error::Corrupt), "byte {at}");
    }
}

#[test]
fn a_truncated_file_is_refused() {
    let file = sealed(&vec![9u8; 70_000]);
    let header_end = header_len(&file);
    assert_eq!(opened(&file[..file.len() - 1]), Err(Error::Corrupt));
    // Cut at the first chunk's end, leaving a chunk that was not final.
    assert_eq!(
        opened(&file[..header_end + 16 + 65536 + 16]),
        Err(Error::Corrupt)
    );
    assert_eq!(opened(&file[..header_end + 16]), Err(Error::Truncated));
    assert_eq!(opened(&file[..header_end + 7]), Err(Error::Truncated));
    assert!(opened(&file[..header_end - 1]).is_err());
    assert!(opened(&file[..10]).is_err());
}

#[test]
fn a_second_stanza_is_refused() {
    let file = sealed(b"secret");
    let at = file.windows(5).position(|w| w == b"\n--- ").expect("mac") + 1;
    for extra in [
        &b"-> scrypt AAAAAAAAAAAAAAAAAAAAAA 16\n\n"[..],
        b"-> grease x\n\n",
    ] {
        let mut f = file[..at].to_vec();
        f.extend_from_slice(extra);
        f.extend_from_slice(&file[at..]);
        assert_eq!(opened(&f), Err(Error::ScryptNotAlone));
        assert_eq!(recognise(&f), Kind::OtherRecipient);
    }
}

#[test]
fn another_recipient_alone_is_refused() {
    let file = sealed(b"secret");
    let mut f = file.clone();
    f[25..31].copy_from_slice(b"X25519");
    assert_eq!(opened(&f), Err(Error::OtherRecipient));
    assert_eq!(recognise(&f), Kind::OtherRecipient);
}

#[test]
fn armored_input_is_refused() {
    let armored = b"-----BEGIN AGE ENCRYPTED FILE-----\nYWdlLWVuY3J5cHRpb24ub3JnL3YxCg==\n-----END AGE ENCRYPTED FILE-----\n";
    assert_eq!(opened(armored), Err(Error::Armored));
    assert_eq!(recognise(armored), Kind::Armored);
    assert_eq!(
        opened(b"\n  -----BEGIN AGE ENCRYPTED FILE-----\n"),
        Err(Error::Armored)
    );
}

#[test]
fn a_work_factor_above_the_limit_is_refused_before_any_work() {
    let file = sealed(b"secret");
    let at = file.windows(4).position(|w| w == b" 16\n").expect("log_n") + 1;
    let mut f = file.clone();
    f[at..at + 2].copy_from_slice(b"21");
    let before = STRETCHES.get();
    assert_eq!(
        open(&compute(), PASSPHRASE, &f, 20).err(),
        Some(Error::TooCostly { log_n: 21 })
    );
    assert_eq!(STRETCHES.get(), before);
    assert_eq!(recognise(&f), Kind::Scrypt { log_n: 21 });
    // The file's own work factor is within a limit of 16.
    assert!(open(&compute(), PASSPHRASE, &file, 16).is_ok());
    assert_eq!(
        open(&compute(), PASSPHRASE, &file, 15).err(),
        Some(Error::TooCostly { log_n: 16 })
    );
}

// --- The payload's shape, crafted under the sealed file's header ----------

fn header_len(file: &[u8]) -> usize {
    let mac = file.windows(5).position(|w| w == b"\n--- ").expect("mac") + 1;
    mac + file[mac..].iter().position(|&b| b == b'\n').expect("end") + 1
}

/// The sealed file's header with a payload of the given chunks, each with
/// its final flag, encrypted under that file's key and nonce.
fn with_chunks(chunks: &[(&[u8], bool)]) -> Vec<u8> {
    let file = sealed(b"");
    let mut out = file[..header_len(&file)].to_vec();
    out.extend_from_slice(&RANDOM[32..48]);
    let prk = hmac_sha256(&RANDOM[32..48], &RANDOM[16..32]);
    let mut key = [0u8; 32];
    hkdf_sha256_expand(&prk, b"payload", &mut key).expect("32 bytes");
    let cipher = ChaCha20Poly1305::new_from_slice(&key).expect("key");
    for (counter, (plain, last)) in chunks.iter().enumerate() {
        let mut nonce = [0u8; 12];
        nonce[3..11].copy_from_slice(&(counter as u64).to_be_bytes());
        nonce[11] = u8::from(*last);
        let mut buf = plain.to_vec();
        let tag = cipher
            .encrypt_inout_detached(&Nonce::from(nonce), b"", InOutBuf::from(&mut buf[..]))
            .expect("encrypts");
        out.extend_from_slice(&buf);
        out.extend_from_slice(&tag);
    }
    out
}

#[test]
fn crafted_payloads_open_when_they_follow_the_format() {
    let full = vec![1u8; 65536];
    assert_eq!(opened(&with_chunks(&[(b"", true)])), Ok(vec![]));
    assert_eq!(opened(&with_chunks(&[(&full, true)])), Ok(full.clone()));
    let two = [full.clone(), b"end".to_vec()].concat();
    assert_eq!(
        opened(&with_chunks(&[(&full, false), (b"end", true)])),
        Ok(two)
    );
}

#[test]
fn a_payload_that_breaks_the_format_is_refused() {
    let full = vec![1u8; 65536];
    // No final chunk.
    assert_eq!(
        opened(&with_chunks(&[(b"abc", false)])),
        Err(Error::Corrupt)
    );
    assert_eq!(opened(&with_chunks(&[(&full, false)])), Err(Error::Corrupt));
    // An empty final chunk after a full one.
    assert_eq!(
        opened(&with_chunks(&[(&full, false), (b"", true)])),
        Err(Error::EmptyFinalChunk)
    );
    // Two chunks marked final, and a chunk after the final one.
    assert_eq!(
        opened(&with_chunks(&[(&full, true), (b"x", true)])),
        Err(Error::Corrupt)
    );
    assert_eq!(
        opened(&with_chunks(&[(b"a", true), (b"b", true)])),
        Err(Error::Corrupt)
    );
    // Bytes after the final chunk.
    let mut f = with_chunks(&[(b"abc", true)]);
    f.push(0);
    assert_eq!(opened(&f), Err(Error::Corrupt));
}
