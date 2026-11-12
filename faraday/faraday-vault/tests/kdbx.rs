//! The KDBX 4 container (`faraday_vault::kdbx`, `docs/VAULT.md` §1.1,
//! §7.1): files pykeepass wrote (`tools/vectors/kdbx/`), the KeePass
//! export's vector read and written again byte for byte, a file sealed
//! here opening here, and each way a file is refused.
#![allow(clippy::disallowed_methods)]

use std::cell::Cell;
use std::path::PathBuf;

use faraday_crypto::sha256;
use faraday_crypto::work::{Compute, Work};
use faraday_vault::Cost;
use faraday_vault::kdbx::{
    Binary, Cipher, Compression, Contents, Error, InnerStream, Kdf, MAX_PLAIN, MAX_STORED,
    RANDOM_LEN, Unsupported, inspect, open, recognise, seal,
};

const PASSPHRASE: &str = "kdbx vector passphrase";
/// Argon2 at the least a reader accepts: 8 MiB, one pass, one lane.
const CHEAP: Cost = Cost {
    memory_kib: 8 * 1024,
    passes: 1,
    lanes: 1,
};

thread_local! {
    /// The key-stretching runs this test's thread has counted.
    static STRETCHES: Cell<usize> = const { Cell::new(0) };
}

fn count(work: Work) {
    if matches!(work, Work::Kdf) {
        STRETCHES.set(STRETCHES.get() + 1);
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-vault's tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(count)
}

fn stretches() -> usize {
    STRETCHES.get()
}

fn vector(dir: &str, name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/vectors")
        .join(dir)
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// 140 distinct bytes, counted up.
fn random() -> [u8; RANDOM_LEN] {
    std::array::from_fn(|i| i as u8)
}

fn contents<'a>(document: &'a [u8]) -> Contents<'a> {
    Contents {
        document,
        binaries: faraday_vault::kdbx::Binaries::all(&[]),
        public_custom_data: None,
    }
}

fn sealed(document: &[u8], plain_len: usize) -> Vec<u8> {
    seal(
        &compute(),
        PASSPHRASE,
        CHEAP,
        &random(),
        &contents(document),
        plain_len,
    )
    .expect("sealed")
}

// --- Base64 and the protected values -------------------------------------

fn unbase64(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let (mut acc, mut bits) = (0u32, 0);
    for &c in text {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// Every protected value of the document, decrypted in document order.
fn protected_values(document: &[u8], inner_key: &[u8]) -> Vec<Vec<u8>> {
    const OPEN: &[u8] = b"Protected=\"True\">";
    let mut stream = InnerStream::new(inner_key);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(found) = document[at..].windows(OPEN.len()).position(|w| w == OPEN) {
        let start = at + found + OPEN.len();
        let end = start + document[start..].iter().position(|&b| b == b'<').unwrap();
        let mut value = unbase64(&document[start..end]);
        stream.apply(&mut value);
        out.push(value);
        at = end;
    }
    out
}

fn has(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

// --- Files an outside writer made ----------------------------------------

#[test]
fn files_pykeepass_wrote_open_with_what_their_headers_state() {
    let files = [
        (
            "aes-argon2d-gzip",
            Cipher::Aes256,
            Kdf::Argon2d,
            Compression::Gzip,
            0,
        ),
        (
            "aes-argon2id-gzip",
            Cipher::Aes256,
            Kdf::Argon2id,
            Compression::Gzip,
            0,
        ),
        (
            "chacha20-argon2d-gzip",
            Cipher::ChaCha20,
            Kdf::Argon2d,
            Compression::Gzip,
            0,
        ),
        (
            "chacha20-argon2id-gzip",
            Cipher::ChaCha20,
            Kdf::Argon2id,
            Compression::Gzip,
            0,
        ),
        (
            "chacha20-argon2id-plain",
            Cipher::ChaCha20,
            Kdf::Argon2id,
            Compression::None,
            0,
        ),
        (
            "aes-argon2id-gzip-v41",
            Cipher::Aes256,
            Kdf::Argon2id,
            Compression::Gzip,
            1,
        ),
    ];
    for (name, cipher, kdf, compression, minor) in files {
        let file = vector("kdbx", &format!("pykeepass-{name}.kdbx"));
        let header = inspect(&file).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            (header.cipher, header.kdf, header.compression, header.minor),
            (cipher, kdf, compression, minor),
            "{name}"
        );
        assert_eq!(
            header.cost,
            Cost {
                memory_kib: 8 * 1024,
                passes: 2,
                lanes: 2
            },
            "{name}"
        );
        let opened =
            open(&compute(), &file, PASSPHRASE).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(opened.header, header, "{name}");
        assert!(has(&opened.document, b"<Value>Example</Value>"), "{name}");
        assert!(has(&opened.document, b"alice"), "{name}");
        let secrets = protected_values(&opened.document, &opened.inner_key);
        assert!(secrets.iter().any(|v| v == b"hunter2"), "{name}");
    }
}

#[test]
fn a_file_that_needs_another_passphrase_is_the_wrong_passphrase() {
    let file = vector("kdbx", "pykeepass-chacha20-argon2id-gzip.kdbx");
    assert_eq!(
        open(&compute(), &file, "not it").err(),
        Some(Error::Passphrase)
    );
    assert_eq!(
        open(&compute(), &file, "").err(),
        Some(Error::EmptyPassphrase)
    );
}

#[test]
fn what_is_refused_is_named() {
    let twofish = vector("kdbx", "pykeepass-twofish-argon2d-gzip.kdbx");
    assert_eq!(recognise(&twofish), Ok(0));
    assert_eq!(
        open(&compute(), &twofish, PASSPHRASE).err(),
        Some(Error::Unsupported(Unsupported::Twofish))
    );
    // KeePassXC's own KDBX 3.1 file, under AES-KDF.
    let old = vector("kdbx", "keepassxc-3.1-aeskdf.kdbx");
    assert_eq!(recognise(&old), Err(Error::Older { major: 3 }));
    assert_eq!(
        open(&compute(), &old, "pw").err(),
        Some(Error::Older { major: 3 })
    );
    assert_eq!(recognise(b"FVLT and more bytes"), Err(Error::NotKdbx));
    assert_eq!(recognise(b""), Err(Error::NotKdbx));
    assert_eq!(stretches(), 0);
}

// --- The KeePass export's vector -----------------------------------------

#[test]
fn the_export_vector_opens_and_is_written_again_byte_for_byte() {
    let file = vector("backup", "backup-kdbx-words-12-english.kdbx");
    let c = compute();
    let opened = open(&c, &file, "correct horse battery staple").expect("opens");
    assert_eq!(
        (
            opened.header.cipher,
            opened.header.kdf,
            opened.header.compression
        ),
        (Cipher::ChaCha20, Kdf::Argon2id, Compression::None)
    );
    // The entry's title is the key's fingerprint and its notes name the words.
    assert!(has(&opened.document, b"<Value>73c5da0a</Value>"));
    assert!(has(&opened.document, b"<Value>12 words, English</Value>"));
    let secrets = protected_values(&opened.document, &opened.inner_key);
    assert_eq!(
        secrets,
        [b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about".to_vec()]
    );
    // The export drew its seeds counted up from zero: master seed, IV,
    // salt, inner key.
    let random: [u8; RANDOM_LEN] = std::array::from_fn(|i| i as u8);
    assert_eq!(&opened.inner_key[..], &random[76..]);
    // The inner header is 83 bytes.
    let again = seal(
        &c,
        "correct horse battery staple",
        opened.header.cost,
        &random,
        &contents(&opened.document),
        83 + opened.document.len(),
    )
    .expect("sealed");
    assert!(again == file, "the file is written as the export wrote it");
}

// --- Sealing and opening --------------------------------------------------

#[test]
fn a_sealed_file_opens_with_everything_it_was_given() {
    let document = b"<KeePassFile>stand-in</KeePassFile>";
    let binaries = [
        Binary {
            protected: true,
            data: b"an attachment".to_vec().into(),
        },
        Binary {
            protected: false,
            data: Vec::new().into(),
        },
    ];
    let custom = [1u8, 2, 3, 4];
    let file = seal(
        &compute(),
        PASSPHRASE,
        CHEAP,
        &random(),
        &Contents {
            document,
            binaries: faraday_vault::kdbx::Binaries::all(&binaries),
            public_custom_data: Some(&custom),
        },
        65_536,
    )
    .expect("sealed");
    let header = inspect(&file).expect("header");
    assert_eq!(
        (
            header.minor,
            header.cipher,
            header.kdf,
            header.compression,
            header.cost
        ),
        (0, Cipher::ChaCha20, Kdf::Argon2id, Compression::None, CHEAP)
    );
    assert_eq!(header.public_custom_data.as_deref(), Some(&custom[..]));
    let opened = open(&compute(), &file, PASSPHRASE).expect("opens");
    assert!(opened.document.starts_with(document));
    assert!(
        opened.document[document.len()..]
            .iter()
            .all(|&b| b == b'\n')
    );
    assert_eq!(opened.binaries.len(), 2);
    assert!(opened.binaries[0].protected && !opened.binaries[1].protected);
    assert_eq!(&opened.binaries[0].data[..], b"an attachment");
    assert!(opened.binaries[1].data.is_empty());
    assert_eq!(&opened.inner_key[..], &random()[76..]);
}

#[test]
fn a_file_is_its_size_and_the_framing() {
    // The header's 249 bytes, its hash and HMAC, and 36 bytes for each block
    // and the empty one after (§2's table).
    let document = b"<KeePassFile/>";
    for (plain_len, file_len) in [(32_768, 33_153), (65_536, 65_921), (4_194_304, 4_194_797)] {
        assert_eq!(sealed(document, plain_len).len(), file_len);
    }
}

#[test]
fn a_passphrase_opens_in_either_form_of_its_text() {
    let nfd = "cafe\u{301}";
    let nfc = "caf\u{e9}";
    let file = seal(&compute(), nfd, CHEAP, &random(), &contents(b"<x/>"), 4096).expect("sealed");
    assert!(open(&compute(), &file, nfc).is_ok());
    assert!(open(&compute(), &file, nfd).is_ok());
    assert_eq!(
        open(&compute(), &file, "cafe").err(),
        Some(Error::Passphrase)
    );
}

#[test]
fn every_seal_takes_all_its_random_bytes() {
    let base = sealed(b"<x/>", 4096);
    // The master seed, the IV, the salt and the inner stream's key.
    for i in [0, 31, 32, 43, 44, 75, 76, 139] {
        let mut changed = random();
        changed[i] ^= 1;
        let other = seal(
            &compute(),
            PASSPHRASE,
            CHEAP,
            &changed,
            &contents(b"<x/>"),
            4096,
        )
        .expect("sealed");
        assert!(
            other != base,
            "byte {i} of the random bytes changes the file"
        );
    }
    assert!(
        sealed(b"<x/>", 4096) == base,
        "the same bytes make the same file"
    );
}

#[test]
fn what_does_not_fit_is_not_sealed() {
    let long = vec![b'x'; 5000];
    assert_eq!(
        seal(
            &compute(),
            PASSPHRASE,
            CHEAP,
            &random(),
            &contents(&long),
            4096
        )
        .err(),
        Some(Error::Full(83 + 5000))
    );
    assert_eq!(
        seal(
            &compute(),
            PASSPHRASE,
            CHEAP,
            &random(),
            &contents(b"x"),
            MAX_PLAIN + 1
        )
        .err(),
        Some(Error::TooLarge)
    );
    assert_eq!(
        seal(&compute(), "", CHEAP, &random(), &contents(b"x"), 4096).err(),
        Some(Error::EmptyPassphrase)
    );
    let dear = Cost {
        memory_kib: 2 * 1024 * 1024,
        ..CHEAP
    };
    assert_eq!(
        seal(
            &compute(),
            PASSPHRASE,
            dear,
            &random(),
            &contents(b"x"),
            4096
        )
        .err(),
        Some(Error::Cost)
    );
}

#[test]
fn protected_values_are_encrypted_in_document_order() {
    let key = [7u8; 64];
    let mut one = InnerStream::new(&key);
    let (mut a, mut b) = (b"first".to_vec(), b"second".to_vec());
    one.apply(&mut a);
    one.apply(&mut b);
    assert!(a != b"first" && b != b"second");
    let mut two = InnerStream::new(&key);
    two.apply(&mut a);
    two.apply(&mut b);
    assert_eq!((&a[..], &b[..]), (&b"first"[..], &b"second"[..]));
}

// --- Headers ---------------------------------------------------------------

const CHACHA20: [u8; 16] = [
    0xD6, 0x03, 0x8A, 0x2B, 0x8B, 0x6F, 0x4C, 0xB5, 0xA5, 0x24, 0x33, 0x9A, 0x31, 0xDB, 0xB5, 0x9A,
];
const ARGON2ID: [u8; 16] = [
    0x9E, 0x29, 0x8B, 0x19, 0x56, 0xDB, 0x47, 0x73, 0xB2, 0x3D, 0xFC, 0x3E, 0xC6, 0xF0, 0xA1, 0xE6,
];
const AES_KDF: [u8; 16] = [
    0xC9, 0xD9, 0xF3, 0x9A, 0x62, 0x8A, 0x44, 0x60, 0xBF, 0x74, 0x0D, 0x08, 0xC1, 0x8A, 0x4F, 0xEA,
];

/// A header with each part set, for the refusals.
#[derive(Clone)]
struct Head {
    version: u32,
    cipher: [u8; 16],
    compression: u32,
    kdf: [u8; 16],
    salt_len: usize,
    memory: u64,
    passes: u64,
    lanes: u32,
    argon_version: Option<u32>,
    secret_key: Option<Vec<u8>>,
    extra: Option<u8>,
}

fn head() -> Head {
    Head {
        version: 0x0004_0000,
        cipher: CHACHA20,
        compression: 0,
        kdf: ARGON2ID,
        salt_len: 32,
        memory: 8 * 1024 * 1024,
        passes: 1,
        lanes: 1,
        argon_version: Some(0x13),
        secret_key: None,
        extra: None,
    }
}

fn field(id: u8, value: &[u8], out: &mut Vec<u8>) {
    out.push(id);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

fn item(kind: u8, name: &[u8], value: &[u8], out: &mut Vec<u8>) {
    out.push(kind);
    out.extend_from_slice(&(name.len() as u32).to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

/// A file of the header, its hash and a stand-in HMAC, nothing more.
fn file(h: &Head) -> Vec<u8> {
    let mut out = vec![];
    out.extend_from_slice(&0x9AA2_D903u32.to_le_bytes());
    out.extend_from_slice(&0xB54B_FB67u32.to_le_bytes());
    out.extend_from_slice(&h.version.to_le_bytes());
    field(2, &h.cipher, &mut out);
    field(3, &h.compression.to_le_bytes(), &mut out);
    field(4, &[1; 32], &mut out);
    field(7, &[2; 12], &mut out);
    let mut kdf = vec![0, 1];
    item(0x42, b"$UUID", &h.kdf, &mut kdf);
    item(0x42, b"S", &vec![3; h.salt_len], &mut kdf);
    item(0x05, b"I", &h.passes.to_le_bytes(), &mut kdf);
    item(0x05, b"M", &h.memory.to_le_bytes(), &mut kdf);
    item(0x04, b"P", &h.lanes.to_le_bytes(), &mut kdf);
    if let Some(v) = h.argon_version {
        item(0x04, b"V", &v.to_le_bytes(), &mut kdf);
    }
    if let Some(k) = &h.secret_key {
        item(0x42, b"K", k, &mut kdf);
    }
    kdf.push(0);
    field(11, &kdf, &mut out);
    if let Some(id) = h.extra {
        field(id, b"x", &mut out);
    }
    field(0, &[0x0D, 0x0A, 0x0D, 0x0A], &mut out);
    let hash = sha256(&out);
    out.extend_from_slice(&hash);
    out.extend_from_slice(&[0; 32]);
    out
}

#[test]
fn a_header_within_the_limits_is_read_and_asks_for_the_passphrase() {
    let f = file(&head());
    let header = inspect(&f).expect("read");
    assert_eq!(header.cost, CHEAP);
    assert_eq!(
        open(&compute(), &f, PASSPHRASE).err(),
        Some(Error::Passphrase)
    );
    // The limits' edges, each read without any work.
    for (memory, passes, lanes) in [
        (8u64 << 20, 1u64, 1u32),
        (1 << 30, 100, 64),
        (8 << 20, 1, 64),
    ] {
        let h = Head {
            memory,
            passes,
            lanes,
            ..head()
        };
        assert!(inspect(&file(&h)).is_ok(), "{memory} {passes} {lanes}");
    }
}

#[test]
fn a_cost_outside_the_limits_is_refused_before_any_work() {
    let before = stretches();
    let cases = [
        Head {
            memory: (8 << 20) - 1024,
            ..head()
        },
        Head {
            memory: (1 << 30) + 1024,
            ..head()
        },
        Head {
            memory: 1 << 40,
            ..head()
        },
        Head {
            memory: (8 << 20) + 1,
            ..head()
        },
        Head {
            passes: 0,
            ..head()
        },
        Head {
            passes: 101,
            ..head()
        },
        Head {
            passes: 1 << 33,
            ..head()
        },
        Head { lanes: 0, ..head() },
        Head {
            lanes: 65,
            ..head()
        },
        Head {
            argon_version: Some(0x10),
            ..head()
        },
        Head {
            secret_key: Some(vec![1]),
            ..head()
        },
    ];
    for (i, h) in cases.iter().enumerate() {
        let f = file(h);
        assert_eq!(inspect(&f).err(), Some(Error::Cost), "case {i}");
        assert_eq!(
            open(&compute(), &f, PASSPHRASE).err(),
            Some(Error::Cost),
            "case {i}"
        );
    }
    assert_eq!(stretches(), before);
}

#[test]
fn a_secret_key_that_is_empty_is_none() {
    let h = Head {
        secret_key: Some(vec![]),
        ..head()
    };
    assert!(inspect(&file(&h)).is_ok());
}

/// A header with one change.
fn with(change: impl FnOnce(&mut Head)) -> Head {
    let mut h = head();
    change(&mut h);
    h
}

/// Every header is refused as its error, by `inspect` and by `open`, with no
/// key derivation run.
fn assert_refused(cases: &[(Head, Error)]) {
    let before = stretches();
    for (i, (h, error)) in cases.iter().enumerate() {
        assert_eq!(inspect(&file(h)).err(), Some(*error), "case {i}");
        assert_eq!(
            open(&compute(), &file(h), PASSPHRASE).err(),
            Some(*error),
            "case {i}"
        );
    }
    assert_eq!(stretches(), before);
}

#[test]
fn a_cipher_kdf_or_compression_this_reader_does_not_take_is_named() {
    assert_refused(&[
        (
            with(|h| h.kdf = AES_KDF),
            Error::Unsupported(Unsupported::AesKdf),
        ),
        (
            with(|h| h.kdf = [9; 16]),
            Error::Unsupported(Unsupported::OtherKdf),
        ),
        (
            with(|h| h.cipher = [9; 16]),
            Error::Unsupported(Unsupported::OtherCipher),
        ),
        (
            with(|h| h.compression = 2),
            Error::Unsupported(Unsupported::Compression),
        ),
    ]);
}

#[test]
fn a_version_or_field_this_reader_does_not_know_is_refused() {
    assert_refused(&[
        (
            with(|h| h.version = 0x0004_0002),
            Error::Newer { major: 4, minor: 2 },
        ),
        (
            with(|h| h.version = 0x0005_0000),
            Error::Newer { major: 5, minor: 0 },
        ),
        (with(|h| h.version = 0x0003_0001), Error::Older { major: 3 }),
        (with(|h| h.extra = Some(99)), Error::UnknownField(99)),
        // KDBX 3 fields in a version 4 file.
        (with(|h| h.extra = Some(5)), Error::UnknownField(5)),
        (with(|h| h.extra = Some(10)), Error::UnknownField(10)),
        // KeePass and KeePassXC refuse a dictionary without its version.
        (with(|h| h.argon_version = None), Error::Damaged),
        (with(|h| h.salt_len = 7), Error::Damaged),
        (with(|h| h.salt_len = 65), Error::Damaged),
    ]);
    // 4.1 is read.
    let minor = inspect(&file(&with(|h| h.version = 0x0004_0001))).map(|h| h.minor);
    assert_eq!(minor, Ok(1));
}

#[test]
fn a_header_cut_short_or_changed_is_damaged() {
    let f = file(&head());
    // Every cut of the header and its hash.
    for cut in 0..f.len() {
        let result = inspect(&f[..cut]);
        assert!(result.is_err(), "cut at {cut}");
    }
    // One changed byte of the header: its hash no longer matches, or a
    // field is read differently.
    let mut hash_bad = f.clone();
    let at = f.len() - 40;
    hash_bad[at] ^= 1;
    assert_eq!(inspect(&hash_bad).err(), Some(Error::Damaged));
}

// --- Damage ------------------------------------------------------------------

#[test]
fn a_file_with_one_byte_changed_or_cut_is_not_opened() {
    let file = sealed(b"<KeePassFile>secrets</KeePassFile>", 4096);
    let header_len = 249;
    let ok = open(&compute(), &file, PASSPHRASE);
    assert!(ok.is_ok());
    // One byte of each part, and the error that part gives: a header byte
    // fails the header's hash, and the HMAC after it is the first check
    // that depends on the passphrase.
    let seed_at = file
        .windows(32)
        .position(|w| w == &random()[..32])
        .expect("the master seed stands in the header");
    for (what, at, expected) in [
        ("master seed", seed_at + 5, Error::Damaged),
        ("header hash", header_len, Error::Damaged),
        ("header HMAC", header_len + 32, Error::Passphrase),
        ("block HMAC", header_len + 64, Error::Damaged),
        ("block length", header_len + 64 + 32, Error::Damaged),
        ("block data", header_len + 64 + 36, Error::Damaged),
        ("last block's HMAC", file.len() - 36, Error::Damaged),
    ] {
        let mut bad = file.clone();
        bad[at] ^= 0x01;
        assert_eq!(
            open(&compute(), &bad, PASSPHRASE).err(),
            Some(expected),
            "{what} (byte {at})"
        );
    }
    // Cut after the last block, in the empty block, and in the middle of one.
    for cut in [
        file.len() - 36,
        file.len() - 1,
        file.len() - 36 - 100,
        header_len + 70,
    ] {
        assert_eq!(
            open(&compute(), &file[..cut], PASSPHRASE).err(),
            Some(Error::Damaged),
            "cut at {cut}"
        );
    }
    // Something after the empty block.
    let mut more = file.clone();
    more.push(0);
    assert_eq!(
        open(&compute(), &more, PASSPHRASE).err(),
        Some(Error::Damaged)
    );
}

#[test]
fn every_byte_of_a_small_file_changed_in_turn_is_not_opened() {
    // The smallest vault there is, at the cheapest cost: the header's bytes
    // are refused by its hash before any derivation, the rest by the HMACs
    // after one. Every header byte is tried; of the rest every third, since
    // each costs an 8 MiB derivation and all of them took about two seconds
    // in a debug build, which is too long for a test of the same refusal.
    const HEADER: usize = 249;
    let file = sealed(b"<x/>", 128);
    assert!(open(&compute(), &file, PASSPHRASE).is_ok());
    for at in (0..file.len()).filter(|&at| at < HEADER || at % 3 == 0) {
        let mut bad = file.clone();
        bad[at] ^= 0x01;
        assert!(
            open(&compute(), &bad, PASSPHRASE).is_err(),
            "byte {at} changed"
        );
    }
}

#[test]
fn a_file_too_long_for_the_largest_vault_is_refused_before_any_work() {
    let before = stretches();
    let head = file(&head());
    let mut f = vec![0u8; head.len() + MAX_STORED + 1];
    f[..head.len()].copy_from_slice(&head);
    assert_eq!(inspect(&f).err(), Some(Error::TooLarge));
    assert_eq!(
        open(&compute(), &f, PASSPHRASE).err(),
        Some(Error::TooLarge)
    );
    assert_eq!(stretches(), before);
}

#[test]
fn the_derivation_runs_once_for_a_file_and_not_for_a_refusal() {
    let file = sealed(b"<x/>", 4096);
    let before = stretches();
    open(&compute(), &file, PASSPHRASE).expect("opens");
    assert_eq!(stretches(), before + 1);
    // A refusal the header's hash gives, a changed cost within the limits
    // among them, runs no derivation.
    let before = stretches();
    for bad in [with_passes(&file, 2), with_memory(&file, 16 * 1024)] {
        assert_eq!(
            open(&compute(), &bad, PASSPHRASE).err(),
            Some(Error::Damaged)
        );
    }
    assert_eq!(stretches(), before);
    // A wrong passphrase is the one refusal that costs the run.
    assert_eq!(
        open(&compute(), &file, "another passphrase").err(),
        Some(Error::Passphrase)
    );
    assert_eq!(stretches(), before + 1);
}

/// The Argon2 parameter dictionary's entry `key` (a one-letter name, its type
/// byte before it), the offset of its value in `file`.
fn kdf_value_at(file: &[u8], key: u8, kind: u8, len: u8) -> usize {
    let head = [kind, 1, 0, 0, 0, key, len, 0, 0, 0];
    file.windows(head.len())
        .position(|w| w == head)
        .expect("the parameter stands in the header")
        + head.len()
}

/// `file` with the Argon2 passes written as `passes`, the header's hash left as it was.
fn with_passes(file: &[u8], passes: u64) -> Vec<u8> {
    let at = kdf_value_at(file, b'I', 5, 8);
    let mut f = file.to_vec();
    f[at..at + 8].copy_from_slice(&passes.to_le_bytes());
    f
}

/// `file` with the Argon2 memory written as `kib`, the header's hash left as it was.
fn with_memory(file: &[u8], kib: u64) -> Vec<u8> {
    let at = kdf_value_at(file, b'M', 5, 8);
    let mut f = file.to_vec();
    f[at..at + 8].copy_from_slice(&(kib * 1024).to_le_bytes());
    f
}

#[test]
fn a_changed_cost_within_the_limits_is_refused_without_a_derivation() {
    let file = vector("backup", "backup-kdbx-words-12-english.kdbx");
    let before = stretches();
    let c = compute();
    // The passes the export states are 3; two and four are within the limits.
    for passes in [2, 4] {
        assert_eq!(
            open(
                &c,
                &with_passes(&file, passes),
                "correct horse battery staple"
            )
            .err(),
            Some(Error::Damaged),
            "passes {passes}"
        );
    }
    assert_eq!(stretches(), before);
}

#[test]
fn every_bit_of_the_version_changed_is_refused_and_a_major_of_132_by_name() {
    let file = vector("backup", "backup-kdbx-words-12-english.kdbx");
    let before = stretches();
    let c = compute();
    for bit in 0..32 {
        let mut bad = file.clone();
        bad[8 + bit / 8] ^= 1 << (bit % 8);
        assert!(
            open(&c, &bad, "correct horse battery staple").is_err(),
            "version bit {bit}"
        );
    }
    // The case the cross-check once found: the major's high bit set, 4 read
    // as 132.
    let mut bad = file.clone();
    bad[10] |= 0x80;
    assert_eq!(
        open(&c, &bad, "correct horse battery staple").err(),
        Some(Error::Newer {
            major: 132,
            minor: 0
        })
    );
    assert_eq!(stretches(), before);
}

#[test]
fn a_vault_of_several_blocks_is_sealed_and_opened() {
    // Three data blocks of a megabyte, a partial one and the empty one after
    // them; the ceiling itself is measured by `examples/large.rs`.
    let document = b"<KeePassFile>secrets</KeePassFile>";
    let plain_len = 3 * 1024 * 1024 + 32_768;
    let file = sealed(document, plain_len);
    assert_eq!(file.len(), 313 + plain_len + 36 * 5);
    let opened = open(&compute(), &file, PASSPHRASE).expect("opened");
    // The inner header (stream id, key, end) comes off the front.
    let inner = (5 + 4) + (5 + 64) + 5;
    assert_eq!(opened.document.len(), plain_len - inner);
    assert!(opened.document.starts_with(document));
    assert!(
        opened.document[document.len()..]
            .iter()
            .all(|&b| b == b'\n')
    );
}

#[test]
fn attachments_are_carried_without_a_copy_of_the_body_for_each() {
    let binaries = [
        Binary {
            protected: false,
            data: vec![7u8; 100_000].into(),
        },
        Binary {
            protected: true,
            data: vec![8u8; 50_000].into(),
        },
    ];
    let file = seal(
        &compute(),
        PASSPHRASE,
        CHEAP,
        &random(),
        &Contents {
            document: b"<KeePassFile/>",
            binaries: faraday_vault::kdbx::Binaries::all(&binaries),
            public_custom_data: None,
        },
        262_144,
    )
    .expect("sealed");
    let opened = open(&compute(), &file, PASSPHRASE).expect("opened");
    assert!(opened.binaries[0].data.iter().all(|&b| b == 7));
    assert!(opened.binaries[1].data.iter().all(|&b| b == 8));
    // Sealing what was opened gives the same attachments back.
    let again = seal(
        &compute(),
        PASSPHRASE,
        CHEAP,
        &random(),
        &Contents {
            document: &opened.document[..14],
            binaries: faraday_vault::kdbx::Binaries::all(&opened.binaries),
            public_custom_data: None,
        },
        262_144,
    )
    .expect("sealed again");
    let back = open(&compute(), &again, PASSPHRASE).expect("opened again");
    assert_eq!(&back.binaries[0].data[..], &binaries[0].data[..]);
    assert_eq!(&back.binaries[1].data[..], &binaries[1].data[..]);
}

#[test]
fn a_file_pykeepass_wrote_with_a_megabyte_attachment_opens() {
    // The stream is more than one block: the attachment is 1.2 MiB.
    let file = vector("kdbx", "pykeepass-chacha20-argon2id-plain-large.kdbx");
    assert!(file.len() > 1024 * 1024);
    let opened = open(&compute(), &file, PASSPHRASE).expect("opened");
    assert!(has(&opened.document, b"<Value>Example</Value>"));
    assert_eq!(opened.binaries.len(), 1);
    let data = &opened.binaries[0].data;
    assert_eq!(data.len(), 1200 * 1024);
    let digest: String = sha256(data).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        digest,
        "7e36854b87ab32660282cf3a40f7c4f2f555c5102facb9120d880dcc82d2bef5"
    );
}

#[test]
fn a_sealed_file_is_built_in_exactly_the_room_reserved_for_it() {
    // The room is reserved once and nothing grows it, whatever the length
    // stands near a whole number of blocks. This stays as a test of an
    // allocation: the memory check at the ceiling budgets one buffer for the
    // sealed file, and a growing one would peak at twice that.
    const MIB: usize = 1024 * 1024;
    for plain_len in [
        0,
        32_768,
        MIB - 32_768,
        MIB,
        MIB + 32_768,
        2 * MIB - 1,
        2 * MIB,
        2 * MIB + 32_768,
    ] {
        let file = sealed(b"<x/>", plain_len.max(4096));
        assert_eq!(file.capacity(), file.len(), "{plain_len}");
    }
}
