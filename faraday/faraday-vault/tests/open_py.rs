//! `tools/vault/open.py` reads the vectors, including an APK signing key
//! (type 12), and `--field TYPE.FIELD --out FILE` writes one field's bytes
//! and nothing else, to a new file only its owner can read
//! (`docs/APK-SIGNING.md` §3, §7). Skipped, and said, where Python or the
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
fn open_py_prints_the_apk_signing_key_record_of_the_vectors() {
    let Some(out) = open_py(vectors::ONE.0, vectors::ONE.1[0], &[]) else {
        return;
    };
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("APK signing key"), "{text}");
    assert!(
        text.contains("package: app.faradayoffline.fixture"),
        "{text}"
    );
}

#[test]
fn field_12_1_writes_the_private_key_bytes_and_nothing_else() {
    let dir = scratch("field");
    let file = dir.0.join("key.pk8");
    let Some(out) = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "12.1", "--out", file.to_str().unwrap()],
    ) else {
        return;
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty(), "nothing of the slot is printed");
    assert_eq!(std::fs::read(&file).unwrap(), vec![0x66; 64]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // Field 2 is the certificate, in a file of its own.
    let cert = dir.0.join("cert.der");
    let out = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "12.2", "--out", cert.to_str().unwrap()],
    )
    .unwrap();
    assert!(out.status.success());
    assert_eq!(std::fs::read(&cert).unwrap(), vec![0x77; 48]);
}

#[test]
fn a_passphrase_on_the_command_line_is_refused_for_a_field() {
    let dir = scratch("argv");
    let file = dir.0.join("key.pk8");
    let out = Command::new("python3")
        .arg("-B")
        .arg(repo("tools/vault/open.py"))
        .arg(repo("tools/vectors/vault/vault-one.ofv"))
        .args(["--passphrase", vectors::ONE.1[0]])
        .args(["--field", "12.1", "--out", file.to_str().unwrap()])
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
    let file = dir.0.join("key.pk8");
    std::fs::write(&file, b"mine").unwrap();
    let Some(out) = open_py(
        vectors::ONE.0,
        vectors::ONE.1[0],
        &["--field", "12.1", "--out", file.to_str().unwrap()],
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
        &["--field", "12.1", "--out", file.to_str().unwrap()],
    )
    .unwrap();
    assert!(!out.status.success());
    assert!(!file.exists());
}
