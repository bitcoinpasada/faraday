//! `tools/vault/open.py` reads the vectors, every record type among them
//! (a GPG key, a wallet share, a passphrase), and `--field
//! TYPE.FIELD --out FILE` writes one field's bytes and nothing else, to a
//! new file only its owner can read
//! (`docs/VAULT.md` §10). Skipped, and said, where Python or the
//! libraries the reader needs are missing.

#[path = "../examples/vectors.rs"]
mod vectors;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn open_py(vault: &str, passphrase: &str, extra: &[&str]) -> Option<Output> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("python3")
        .arg("-B")
        .arg(repo("tools/vault/open.py"))
        .arg(repo(&format!("tools/vectors/vault/{vault}")))
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    // The passphrase is typed at the prompt, here one line of stdin.
    let _ = child
        .stdin
        .take()?
        .write_all(format!("{passphrase}\n").as_bytes());
    let out = child.wait_with_output().ok()?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() && (err.contains("ModuleNotFoundError") || err.contains("ImportError"))
    {
        eprintln!("the Python libraries open.py needs are not installed: skipped");
        return None;
    }
    Some(out)
}

/// A test's own scratch directory; removed when it drops.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let dir = std::env::temp_dir().join(format!("faraday-open-py-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}

#[test]
fn open_py_opens_a_slot_holding_a_wallet_share_and_a_passphrase() {
    let Some(out) = open_py(vectors::ONE.0, vectors::ONE.1[0], &[]) else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("wallet name: Savings"), "{text}");
    assert!(text.contains("key fingerprint: 9a6a2580"), "{text}");
    assert!(text.contains("passphrase: TREZOR"), "{text}");
}

#[test]
fn field_7_1_writes_the_primary_seed_bytes_and_nothing_else() {
    let dir = scratch("field");
    let file = dir.0.join("seed.bin");
    let Some(out) = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "7.1", "--out", file.to_str().unwrap()],
    ) else {
        return;
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "nothing of the slot is printed");
    assert_eq!(std::fs::read(&file).unwrap(), vec![0x11; 32]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // Field 3 is the subkey seed, in a file of its own.
    let cert = dir.0.join("subkey.bin");
    let out = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "7.3", "--out", cert.to_str().unwrap()],
    )
    .unwrap();
    assert!(out.status.success());
    assert_eq!(std::fs::read(&cert).unwrap(), vec![0x22; 32]);
}

#[test]
fn a_passphrase_on_the_command_line_is_refused_for_a_field() {
    let dir = scratch("argv");
    let file = dir.0.join("seed.bin");
    let out = Command::new("python3")
        .arg("-B")
        .arg(repo("tools/vault/open.py"))
        .arg(repo("tools/vectors/vault/vault-one.fvlt"))
        .args(["--passphrase", vectors::ONE.1[0]])
        .args(["--field", "7.1", "--out", file.to_str().unwrap()])
        .output();
    let Ok(out) = out else {
        return;
    };
    assert!(!out.status.success());
    assert!(!file.exists());
}

#[test]
fn an_existing_file_is_refused_and_left_alone() {
    let dir = scratch("exists");
    let file = dir.0.join("seed.bin");
    std::fs::write(&file, b"mine").unwrap();
    let Some(out) = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "7.1", "--out", file.to_str().unwrap()],
    ) else {
        return;
    };
    assert!(!out.status.success());
    assert_eq!(std::fs::read(&file).unwrap(), b"mine");
}

#[test]
fn a_type_the_slot_does_not_hold_and_a_wrong_passphrase_write_nothing() {
    let dir = scratch("none");
    let file = dir.0.join("out.bin");
    let Some(out) = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "9.9", "--out", file.to_str().unwrap()],
    ) else {
        return;
    };
    assert!(!out.status.success());
    let out = open_py(
        vectors::ONE.0,
        "not the passphrase",
        &["--field", "7.1", "--out", file.to_str().unwrap()],
    )
    .unwrap();
    assert!(!out.status.success());
    assert!(!file.exists());
}

/// The vector made with "café" in NFD opens in the reader under the
/// passphrase in NFD and in NFC, its seal count read.
#[test]
fn open_py_opens_the_nfd_vector_under_either_form() {
    for p in [vectors::NFD.1[0], vectors::NFD_AS_NFC] {
        let Some(out) = open_py(vectors::NFD.0, p, &[]) else {
            return;
        };
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(text.contains("label: Accents"), "{text}");
        assert!(text.contains("Seal count\n  count: 1"), "{text}");
    }
}

/// The slot that verifies under two passphrases' keys is refused by the
/// reader under each, for its commitment block.
#[test]
fn open_py_refuses_the_slot_two_passphrases_open() {
    for p in vectors::TWO_KEYS.1 {
        let Some(out) = open_py(vectors::TWO_KEYS.0, p, &[]) else {
            return;
        };
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert!(err.contains("not committed"), "{err}");
    }
}

#[test]
fn open_py_reads_a_codex32_keys_set_and_a_codex32_share() {
    let Some(out) = open_py(vectors::CODEX32.0, vectors::CODEX32.1[0], &[]) else {
        return;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.contains(
            "codex32 set: k 2 · identifier name · 48 characters · a placed, c placed, d made"
        ),
        "{text}"
    );
    assert!(
        text.contains(&format!("string: {}", vectors::VECTOR_2.1)),
        "{text}"
    );
    assert!(text.contains("key fingerprint: fab6868a"), "{text}");
}
