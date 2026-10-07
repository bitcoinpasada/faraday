//! The `osk-backup` and KDBX 4 vectors in `tools/vectors/backup/`: what
//! this crate writes from the inputs `tools/vectors/backup/README.md`
//! names, what it reads back, and what a second implementation reads —
//! `tools/backup/decrypt.py` and `tools/backup/read-kdbx.py`, run where
//! their Python libraries are installed.

use std::io::Write;
use std::process::Command as Process;

use osk_backup::DEVICE_PARAMS;
use osk_bip::bip39::{Language, Mnemonic};

/// The version-2 vectors, which this build still opens: a 12-word key
/// and a 24-word one.
const VECTOR: &str = include_str!("../../../tools/vectors/backup/backup-12-english.hex");
const VECTOR_24: &str = include_str!("../../../tools/vectors/backup/backup-24-japanese.hex");

/// The version-3 vectors, one per payload kind, which this build writes
/// and `tools/backup/decrypt.py` reads.
const V3_WORDS: &str = include_str!("../../../tools/vectors/backup/backup-v3-words-12-english.hex");
const V3_SEED: &str = include_str!("../../../tools/vectors/backup/backup-v3-seed-32.hex");
const V3_NOTE: &str = include_str!("../../../tools/vectors/backup/backup-v3-note.hex");
const V3_SHEET: &str = include_str!("../../../tools/vectors/backup/backup-v3-sheet.hex");

/// The same bytes as files.
const V3_WORDS_FILE: &[u8] =
    include_bytes!("../../../tools/vectors/backup/backup-v3-words-12-english.oskb");
const V3_SEED_FILE: &[u8] = include_bytes!("../../../tools/vectors/backup/backup-v3-seed-32.oskb");
const V3_NOTE_FILE: &[u8] = include_bytes!("../../../tools/vectors/backup/backup-v3-note.oskb");
const V3_SHEET_FILE: &[u8] = include_bytes!("../../../tools/vectors/backup/backup-v3-sheet.oskb");
const VECTOR_FILE: &[u8] = include_bytes!("../../../tools/vectors/backup/backup-12-english.oskb");
const VECTOR_24_FILE: &[u8] =
    include_bytes!("../../../tools/vectors/backup/backup-24-japanese.oskb");

/// The KDBX vector: the same 12-word English key, written at the
/// device's costs under the vector passphrase.
const KDBX_VECTOR: &[u8] =
    include_bytes!("../../../tools/vectors/backup/backup-kdbx-words-12-english.kdbx");

/// The note the version-3 note vector holds.
const VECTOR_NOTE: &str = "Keys in the safe.\nWords with the notary.\n";

/// The passphrase the vectors were written under.
const VECTOR_PASSPHRASE: &str = "correct horse battery staple";

/// A passphrase for the files these tests seal themselves.
const PASSPHRASE: &str = "a long enough one";

/// The words of the all-zero 12-word English key every key vector holds.
const ABANDON: [&str; 12] = [
    "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon", "abandon",
    "abandon", "abandon", "abandon", "about",
];

fn unhex(text: &str) -> Vec<u8> {
    let text = text.trim();
    (0..text.len() / 2)
        .map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

/// The 56 bytes the 12-word vector's salt and nonce were taken from.
fn vector_seed() -> [u8; osk_backup::oskb::SEED_LEN] {
    let mut seed = [0u8; osk_backup::oskb::SEED_LEN];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8;
    }
    seed
}

/// The 24-word vector's, which is a different one because the two
/// share a passphrase.
fn vector_seed_24() -> [u8; osk_backup::oskb::SEED_LEN] {
    let mut seed = [0u8; osk_backup::oskb::SEED_LEN];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = 0x80 + i as u8;
    }
    seed
}

/// The 140 bytes the KDBX vector's master seed, IV, salt and inner key
/// were taken from.
fn kdbx_vector_seed() -> [u8; osk_backup::kdbx::SEED_LEN] {
    let mut seed = [0u8; osk_backup::kdbx::SEED_LEN];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = i as u8;
    }
    seed
}

/// A file in the system temporary directory, removed when it drops.
struct Temp {
    path: std::path::PathBuf,
    file: std::fs::File,
}

impl Write for Temp {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn tempfile() -> Temp {
    let path = std::env::temp_dir().join(format!(
        "osk-backup-vector-{}-{}.oskb",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock")
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("a temporary file");
    Temp { path, file }
}

/// Two keys of different lengths back up to files of the same length,
/// and each opens to its own words. Nothing about a backup — the file
/// on the card or the QR on the screen — says how long the seed is.
#[test]
fn a_12_word_and_a_24_word_backup_are_the_same_size() {
    let short = Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("a key");
    let long = Mnemonic::from_entropy(Language::English, &[0u8; 32]).expect("a key");
    let a = osk_backup::oskb::seal(&short, PASSPHRASE.as_bytes(), DEVICE_PARAMS, &vector_seed())
        .expect("a backup");
    let b = osk_backup::oskb::seal(
        &long,
        PASSPHRASE.as_bytes(),
        DEVICE_PARAMS,
        &vector_seed_24(),
    )
    .expect("a backup");
    assert_eq!(a.len(), b.len());

    let out_short = osk_backup::oskb::open(&a, PASSPHRASE.as_bytes()).expect("it opens");
    let out_long = osk_backup::oskb::open(&b, PASSPHRASE.as_bytes()).expect("it opens");
    assert_eq!(out_short.indices(), short.indices());
    assert_eq!(out_long.indices(), long.indices());
}

/// The version-2 vectors still open, whatever else the format has
/// grown: a file someone wrote before this build shipped is still their
/// backup.
#[test]
fn a_version_2_vector_still_opens() {
    let bytes = unhex(VECTOR);
    assert_eq!(bytes, VECTOR_FILE, "the hex and the file are one backup");
    let words: Vec<&str> = osk_backup::oskb::open(&bytes, VECTOR_PASSPHRASE.as_bytes())
        .expect("it opens")
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    assert_eq!(words, ABANDON);

    let long = unhex(VECTOR_24);
    assert_eq!(long, VECTOR_24_FILE, "the hex and the file are one backup");
    assert_eq!(long.len(), bytes.len(), "both vectors are one length");
    assert_eq!(
        osk_backup::oskb::open(&long, VECTOR_PASSPHRASE.as_bytes())
            .expect("it opens")
            .indices()
            .len(),
        24
    );

    let Some((kind, printed)) = decrypt_with_script(&bytes) else {
        eprintln!(
            "skipped: python3 with argon2-cffi and pynacl, or cryptography 44+, \
             is not installed; tools/backup/decrypt.py was not run"
        );
        return;
    };
    assert_eq!(kind, "words");
    assert_eq!(printed.split_whitespace().collect::<Vec<_>>(), ABANDON);
}

/// The published version-3 vectors are what this code writes and what a
/// second implementation reads, one per payload kind. The script is run
/// only where its dependencies are installed; where they are not, the
/// test says so and stops, because nothing is installed to make a test
/// pass.
#[test]
fn the_version_3_vectors_read_the_same_in_both_implementations() {
    let words = unhex(V3_WORDS);
    let seed = unhex(V3_SEED);
    let note = unhex(V3_NOTE);
    let sheet = unhex(V3_SHEET);
    assert_eq!(words, V3_WORDS_FILE, "the hex and the file are one backup");
    assert_eq!(seed, V3_SEED_FILE, "the hex and the file are one backup");
    assert_eq!(note, V3_NOTE_FILE, "the hex and the file are one backup");
    assert_eq!(sheet, V3_SHEET_FILE, "the hex and the file are one backup");

    // A key's two doors — its words and its master seed — make files of
    // one length, so neither says which it is.
    assert_eq!(words.len(), seed.len());
    assert_eq!(words.len(), osk_backup::oskb::LEN);

    let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("a key");
    assert_eq!(
        osk_backup::oskb::seal(
            &m,
            VECTOR_PASSPHRASE.as_bytes(),
            DEVICE_PARAMS,
            &vector_seed()
        ),
        Some(words.clone()),
        "the vector is what these inputs produce; see tools/vectors/backup/README.md"
    );
    let opened: Vec<&str> = osk_backup::oskb::open(&words, VECTOR_PASSPHRASE.as_bytes())
        .expect("it opens")
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    assert_eq!(opened, ABANDON);

    match osk_backup::oskb::open_payload(&seed, VECTOR_PASSPHRASE.as_bytes()).expect("it opens") {
        osk_backup::oskb::Opened::Seed(bytes) => assert_eq!(bytes.as_slice(), &[0x42u8; 32]),
        _ => panic!("a master seed"),
    }
    match osk_backup::oskb::open_payload(&note, VECTOR_PASSPHRASE.as_bytes()).expect("it opens") {
        osk_backup::oskb::Opened::Note(bytes) => {
            assert_eq!(
                String::from_utf8(bytes.to_vec()).expect("text"),
                VECTOR_NOTE
            );
        }
        _ => panic!("a note"),
    }
    match osk_backup::oskb::open_payload(&sheet, VECTOR_PASSPHRASE.as_bytes()).expect("it opens") {
        osk_backup::oskb::Opened::Sheet(parts) => {
            assert_eq!(parts.name.as_slice(), b"Cold storage");
            assert!(parts.descriptor.starts_with(b"wsh(sortedmulti(2,"));
            assert!(parts.note.starts_with(b"Two of three."));
        }
        _ => panic!("a recovery sheet"),
    }

    let Some((kind, printed)) = decrypt_with_script(&words) else {
        eprintln!(
            "skipped: python3 with argon2-cffi and pynacl, or cryptography 44+, \
             is not installed; tools/backup/decrypt.py was not run"
        );
        return;
    };
    assert_eq!(kind, "words");
    assert_eq!(printed.split_whitespace().collect::<Vec<_>>(), ABANDON);

    let (kind, printed) = decrypt_with_script(&seed).expect("the script ran once already");
    assert_eq!(kind, "seed");
    assert_eq!(printed, "42".repeat(32));

    let (kind, printed) = decrypt_with_script(&note).expect("the script ran once already");
    assert_eq!(kind, "note");
    assert_eq!(printed, VECTOR_NOTE.trim_end());

    let (kind, printed) = decrypt_with_script(&sheet).expect("the script ran once already");
    assert_eq!(kind, "sheet");
    assert!(printed.contains("name: Cold storage"), "{printed}");
}

/// A file made at 256 MiB opens at that cost, and one that claims
/// 64 GiB is refused by what it asks for, before any work is done.
#[test]
fn a_file_is_opened_at_the_cost_it_states() {
    let m = Mnemonic::from_entropy(Language::English, &[1u8; 32]).expect("a key");
    let cost = osk_backup::Cost {
        memory_kib: 262_144,
        passes: DEVICE_PARAMS.passes,
        lanes: DEVICE_PARAMS.lanes,
    };
    let bytes =
        osk_backup::oskb::seal(&m, PASSPHRASE.as_bytes(), cost, &vector_seed()).expect("a backup");
    assert_eq!(osk_backup::oskb::cost_of(&bytes), Some(cost));
    assert_eq!(
        osk_backup::oskb::open(&bytes, PASSPHRASE.as_bytes())
            .expect("it opens here")
            .indices(),
        m.indices()
    );

    let mut huge = bytes;
    huge[5..9].copy_from_slice(&(64u32 * 1024 * 1024).to_le_bytes());
    assert_eq!(
        osk_backup::oskb::read_header(&huge).err(),
        Some(osk_backup::oskb::Error::Memory(65_536)),
        "the file says how much memory it needs"
    );
}

/// The published KDBX vector is what this code writes, it authenticates
/// under its passphrase, and a KeePass reader outside this repository
/// gets the words back out of it.
#[test]
fn the_kdbx_vector_is_what_this_code_writes_and_reads_back_elsewhere() {
    let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("a key");
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let words = words.join(" ");
    let bytes = osk_backup::kdbx::write(
        &[osk_backup::kdbx::Item {
            title: b"73c5da0a",
            notes: b"12 words, English",
            secret: words.as_bytes(),
        }],
        VECTOR_PASSPHRASE.as_bytes(),
        DEVICE_PARAMS,
        &kdbx_vector_seed(),
    )
    .expect("a database");
    assert_eq!(
        bytes, KDBX_VECTOR,
        "the vector is what these inputs produce; see tools/vectors/backup/README.md"
    );
    assert!(osk_backup::kdbx::verify(
        &bytes,
        VECTOR_PASSPHRASE.as_bytes()
    ));
    assert_eq!(osk_backup::kdbx::cost_of(&bytes), Some(DEVICE_PARAMS));

    let Some(printed) = read_kdbx_with_script(&bytes, VECTOR_PASSPHRASE) else {
        eprintln!(
            "skipped: python3 with pykeepass, or with cryptography 44+, is not \
             installed; tools/backup/read-kdbx.py was not run"
        );
        return;
    };
    assert!(printed.contains("title: 73c5da0a"), "{printed}");
    assert!(printed.contains(&words), "{printed}");
    assert!(printed.contains("12 words, English"), "{printed}");
}

/// Runs `tools/backup/decrypt.py` over `bytes`: the kind it printed on
/// standard error and what it printed on standard output. `None` where
/// the script cannot run here.
fn decrypt_with_script(bytes: &[u8]) -> Option<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tools/backup/decrypt.py");
    // The two libraries the script needs, asked for without decrypting
    // anything: a machine with neither skips rather than fails.
    let probe = Process::new("python3")
        .arg("-c")
        .arg(
            "import sys, pathlib; sys.path.insert(0, str(pathlib.Path('tools/backup')));\n\
             import decrypt;\n\
             decrypt.argon2id(b'x', bytes(32), 8, 1, 1);\n\
             decrypt.xchacha20poly1305_open(bytes(32), bytes(24), b'', bytes(16))",
        )
        .current_dir(&root)
        .output()
        .ok()?;
    if !probe.status.success() {
        return None;
    }

    let mut file = tempfile();
    file.write_all(bytes).expect("write the backup");
    file.flush().expect("flush");
    let path = file.path.clone();
    let out = Process::new("python3")
        .arg(&script)
        .arg(&path)
        .arg("--passphrase")
        .arg(VECTOR_PASSPHRASE)
        .current_dir(&root)
        .output()
        .expect("run the script");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let kind = String::from_utf8_lossy(&out.stderr)
        .lines()
        .next_back()
        .unwrap_or_default()
        .trim()
        .to_string();
    Some((
        kind,
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

/// Runs `tools/backup/read-kdbx.py` over `bytes` and returns what it
/// printed. `None` where the script cannot run here, which is a machine
/// with neither `pykeepass` nor `cryptography`.
fn read_kdbx_with_script(bytes: &[u8], passphrase: &str) -> Option<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tools/backup/read-kdbx.py");
    let probe = Process::new("python3")
        .arg("-c")
        .arg(
            "import sys, pathlib; sys.path.insert(0, str(pathlib.Path('tools/backup')));\n\
             import decrypt;\n\
             decrypt.argon2id(b'x', bytes(32), 8, 1, 1);\n\
             from cryptography.hazmat.primitives.ciphers import Cipher, algorithms;\n\
             Cipher(algorithms.ChaCha20(bytes(32), bytes(16)), mode=None).encryptor()",
        )
        .current_dir(&root)
        .output()
        .ok()?;
    if !probe.status.success() {
        return None;
    }
    let mut file = tempfile();
    file.write_all(bytes).expect("write the database");
    file.flush().expect("flush");
    let path = file.path.clone();
    let out = Process::new("python3")
        .arg(&script)
        .arg(&path)
        .arg("--passphrase")
        .arg(passphrase)
        .current_dir(&root)
        .output()
        .expect("run the script");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}
