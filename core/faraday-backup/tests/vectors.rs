//! The KDBX 4 vector in `tools/vectors/backup/`: what this crate writes
//! from the inputs `tools/vectors/backup/README.md` names, and what a
//! second implementation reads, `tools/backup/read-kdbx.py`, run where
//! its Python libraries are installed.

use std::io::Write;
use std::process::Command as Process;

use faraday_backup::DEVICE_PARAMS;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;

/// The KDBX vector: a 12-word English key, written at the device's
/// costs under the vector passphrase.
const KDBX_VECTOR: &[u8] =
    include_bytes!("../../../tools/vectors/backup/backup-kdbx-words-12-english.kdbx");

/// The passphrase the vector was written under.
const VECTOR_PASSPHRASE: &str = "correct horse battery staple";

/// The 140 bytes the KDBX vector's master seed, IV, salt and inner key
/// were taken from.
fn kdbx_vector_seed() -> [u8; faraday_backup::kdbx::SEED_LEN] {
    let mut seed = [0u8; faraday_backup::kdbx::SEED_LEN];
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
        "faraday-backup-vector-{}-{}.kdbx",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock")
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).expect("a temporary file");
    Temp { path, file }
}

/// The published KDBX vector is what this code writes, it authenticates
/// under its passphrase, and a KeePass reader outside this repository
/// gets the words back out of it.
#[test]
fn the_kdbx_vector_is_what_this_code_writes_and_reads_back_elsewhere() {
    let c = Compute::unchecked();
    let m = Mnemonic::from_entropy(Language::English, &[0u8; 16]).expect("a key");
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::English.word(i))
        .collect();
    let words = words.join(" ");
    let bytes = faraday_backup::kdbx::write(
        &c,
        &[faraday_backup::kdbx::Item {
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
    assert!(faraday_backup::kdbx::verify(
        &c,
        &bytes,
        VECTOR_PASSPHRASE.as_bytes()
    ));
    assert_eq!(faraday_backup::kdbx::cost_of(&bytes), Some(DEVICE_PARAMS));

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

/// Runs `tools/backup/read-kdbx.py` over `bytes` and returns what it
/// printed. `None` where the script cannot run here, which is a machine
/// with neither `pykeepass` nor `cryptography`.
fn read_kdbx_with_script(bytes: &[u8], passphrase: &str) -> Option<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = root.join("tools/backup/read-kdbx.py");
    let probe = Process::new("python3")
        .arg("-c")
        .arg(
            "try:\n    from argon2.low_level import Type\nexcept ImportError:\n    from cryptography.hazmat.primitives.kdf.argon2 import Argon2id\n\
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
