//! `tools/vault/open.py` reads the file vectors with pykeepass and the QR
//! vectors with Sequoia's `sq`, outside programs, and prints every item under §7's names with the values
//! pykeepass reads back; `--field KIND.FIELD --out FILE` writes one value's
//! bytes and nothing else, to a new file only its owner can read
//! (`docs/VAULT.md` §10). The file vectors are skipped, and it is said, where
//! Python or pykeepass is missing; a missing Python or `sq` fails the QR tests
//! (DEVELOPING.md).

#[path = "../examples/vectors.rs"]
mod vectors;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn vector(name: &str) -> PathBuf {
    repo(&format!("tools/vectors/vault/{name}"))
}

/// open.py on `file` with the passphrase typed at its prompt (one line of
/// stdin); `None` where python3 does not start.
fn run_open_py(file: &Path, passphrase: &str, extra: &[&str]) -> Option<Output> {
    use std::io::Write;
    use std::process::Stdio;
    let mut child = Command::new("python3")
        .arg("-B")
        .arg(repo("tools/vault/open.py"))
        .arg(file)
        .args(extra)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let _ = child
        .stdin
        .take()?
        .write_all(format!("{passphrase}\n").as_bytes());
    child.wait_with_output().ok()
}

/// open.py on a file vault; `None` where pykeepass is not installed.
fn open_py_on(file: &Path, passphrase: &str, extra: &[&str]) -> Option<Output> {
    let out = run_open_py(file, passphrase, extra)?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success()
        && (err.contains("ModuleNotFoundError")
            || err.contains("needs the distribution's pykeepass"))
    {
        eprintln!("pykeepass is not installed: skipped");
        return None;
    }
    Some(out)
}

fn open_py(vault: &str, passphrase: &str, extra: &[&str]) -> Option<Output> {
    open_py_on(&vector(vault), passphrase, extra)
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
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

fn armored() -> String {
    // Built so that no whole armor line of a secret key is in the source.
    let secret = "PRIVATE";
    format!(
        "-----BEGIN PGP {secret} KEY BLOCK-----\n\nxVgEZ0\n-----END PGP {secret} KEY BLOCK-----\n"
    )
}

#[test]
fn open_py_prints_every_item_under_its_name_with_the_values_pykeepass_reads() {
    let Some(out) = open_py(vectors::ALL, vectors::PASSPHRASE, &[]) else {
        return;
    };
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    for want in [
        "Vault: Everything",
        "  id: 01010101010101010101010101010101\n",
        "  faraday.format: 1\n",
        "  faraday.seals: 1\n",
        "  faraday.sealed: 2023-11-14T22:13:20Z\n",
        "  faraday.signed.abababababababababababababababababababababababababababababababab: 5000,0\n",
        "  faraday.plan.1: wallet = \"wpkh([73c5da0a/84h/0h/0h]xpub6CatWdiZiodm",
        "seed (Bitcoin keys) 01010101010101010101010101010101\n  Title: Savings\n  Password: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\n  passphrase: TREZOR\n  faraday.load: false\n  faraday.held: true\n",
        "  faraday.codex32: {k = 2, id = \"name\", lengths = [48], unknown = 1, shares = [{index = \"a\", state = \"placed\"}, {index = \"c\", state = \"made\"}]}\n",
        "  faraday.child: {parent = \"01020304\", application = 39, language = 0, words = 12, index = 7}\n",
        "  Title: Key 01020304\n  hex: 5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a\n",
        "wallet (Wallets) ",
        "  policy: wpkh(@0/**)\n[73c5da0a/84h/0h/0h]xpub6Cat",
        "  descriptor: wpkh([73c5da0a/84h/0h/0h]xpub6Cat",
        "note (Notes) ",
        "  Notes: The safe's code is in the drawer.\n",
        "sheet (Recovery sheets) ",
        "  Notes: in the safe\n",
        "gpg (GPG keys) ",
        "  Title: Test <test@example.com>\n",
        "  revoked: 2026-10-28T14:02:00Z\n",
        "secure_boot (Secure Boot) ",
        "  owner: 11111111-1111-1111-1111-111111111111\n",
        "  db_cert: -----BEGIN CERTIFICATE-----\nMAMCAQE=\n-----END CERTIFICATE-----\n",
        "wallet_share (Wallet shares) ",
        "  Title: Family\n  Password: share text\n  index: 1\n  of: 3\n",
        "passphrase (Passphrases) ",
        "  Title: Passphrase for b4e3f5ed\n  Password: TREZOR\n  for: b4e3f5ed\n",
        "share (Codex32 shares) ",
        "  Password: ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn\n  for: fab6868a\n",
        "entry 0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c\n",
        "  UserName: you@example.com\n  Password: hunter2\n  URL: https://mail.example.com\n",
        "  otp: otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP&issuer=Example\n",
    ] {
        assert!(text.contains(want), "missing {want:?} in:\n{text}");
    }
    let secret = "PRIVATE";
    assert!(text.contains(&format!(
        "  pk_key: -----BEGIN {secret} KEY-----\nMAMCAQE=\n-----END {secret} KEY-----\n"
    )));
    assert!(text.contains(&armored()));
    // `faraday.kind` names the kind line; it is not printed as a field.
    assert!(!text.contains("faraday.kind"));
}

#[test]
fn open_py_reads_the_small_the_codex32_and_the_sealed_vaults() {
    let Some(out) = open_py(vectors::SMALL, vectors::PASSPHRASE, &[]) else {
        return;
    };
    let text = stdout(&out);
    assert!(text.starts_with("Vault: Small\n"), "{text}");
    assert!(text.contains("  Notes: A note in the small vault.\n"));
    let out = open_py(vectors::CODEX32, vectors::PASSPHRASE, &[]).unwrap();
    let text = stdout(&out);
    assert!(
        text.contains("  Title: Codex32 key\n") || text.contains("  Title: Key fab6868a\n"),
        "{text}"
    );
    assert!(text.contains(
        "  faraday.codex32: {k = 2, id = \"name\", lengths = [48], unknown = 0, shares = [{index = \"a\", state = \"placed\"}, {index = \"c\", state = \"placed\"}, {index = \"d\", state = \"made\"}]}\n"
    ), "{text}");
    assert!(text.contains("  Password: ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn\n"));
    // The file `file::seal` wrote, at the cost a seal writes.
    let out = open_py(vectors::SEALED, vectors::PASSPHRASE, &[]).unwrap();
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.starts_with("Vault: Sealed again\n"), "{text}");
    assert!(text.contains("  faraday.seals: 2\n  faraday.sealed: 2023-11-14T22:14:20Z\n"));
    assert!(text.contains("  Notes: Added at a seal.\n"));
    assert!(text.contains("  UserName: you@example.com\n"));
}

/// §3: pykeepass does not put a passphrase in NFC, open.py does: the NFD
/// vector opens under either form.
#[test]
fn open_py_opens_the_nfd_vector_under_either_form() {
    for p in [vectors::NFD_PASSPHRASE, vectors::NFD_AS_NFC] {
        let Some(out) = open_py(vectors::NFD, p, &[]) else {
            return;
        };
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(stdout(&out).starts_with("Vault: Accents\n"));
    }
}

#[test]
fn field_gpg_key_writes_the_armored_key_and_nothing_else() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("field");
    let to = dir.0.join("gpg-key.asc");
    let to_arg = to.to_str().unwrap();
    let Some(out) = open_py(
        vectors::ALL,
        vectors::PASSPHRASE,
        &["--field", "gpg.key", "--out", to_arg],
    ) else {
        return;
    };
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(out.stdout.is_empty(), "nothing of the vault is printed");
    assert_eq!(std::fs::read_to_string(&to).unwrap(), armored());
    assert_eq!(
        std::fs::metadata(&to).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn index_takes_one_of_several_and_a_missing_one_is_refused() {
    let dir = scratch("index");
    let to = dir.0.join("title");
    let to_arg = to.to_str().unwrap();
    // The vault holds two seeds: one needs --index.
    let Some(out) = open_py(
        vectors::ALL,
        vectors::PASSPHRASE,
        &["--field", "seed.Title", "--out", to_arg],
    ) else {
        return;
    };
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("holds 2 of kind seed"),
        "{}",
        stderr(&out)
    );
    assert!(!to.exists());
    for (index, want) in [("1", "Savings"), ("2", "Key 01020304")] {
        let out = open_py(
            vectors::ALL,
            vectors::PASSPHRASE,
            &["--field", "seed.Title", "--index", index, "--out", to_arg],
        )
        .unwrap();
        assert!(out.status.success(), "{}", stderr(&out));
        assert_eq!(std::fs::read_to_string(&to).unwrap(), want);
        std::fs::remove_file(&to).unwrap();
    }
    let out = open_py(
        vectors::ALL,
        vectors::PASSPHRASE,
        &["--field", "seed.Title", "--index", "3", "--out", to_arg],
    )
    .unwrap();
    assert!(!out.status.success());
    assert!(!to.exists());
}

#[test]
fn a_passphrase_on_the_command_line_is_refused_for_a_field() {
    let dir = scratch("argv");
    let to = dir.0.join("out");
    let Some(out) = open_py(
        vectors::ALL,
        vectors::PASSPHRASE,
        &[
            "--passphrase",
            "x",
            "--field",
            "gpg.key",
            "--out",
            to.to_str().unwrap(),
        ],
    ) else {
        return;
    };
    assert!(!out.status.success());
    assert!(stderr(&out).contains("--passphrase is refused with --field"));
    assert!(!to.exists());
}

#[test]
fn an_existing_file_is_refused_and_left_alone() {
    let dir = scratch("exists");
    let to = dir.0.join("out");
    std::fs::write(&to, b"keep").unwrap();
    let Some(out) = open_py(
        vectors::ALL,
        vectors::PASSPHRASE,
        &["--field", "gpg.key", "--out", to.to_str().unwrap()],
    ) else {
        return;
    };
    assert!(!out.status.success());
    assert!(stderr(&out).contains("exists; nothing was written"));
    assert_eq!(std::fs::read(&to).unwrap(), b"keep");
}

#[test]
fn a_kind_the_vault_does_not_hold_and_a_wrong_passphrase_write_nothing() {
    let dir = scratch("none");
    let to = dir.0.join("out");
    let to_arg = to.to_str().unwrap();
    let Some(out) = open_py(
        vectors::SMALL,
        vectors::PASSPHRASE,
        &["--field", "gpg.key", "--out", to_arg],
    ) else {
        return;
    };
    assert!(!out.status.success());
    assert!(stderr(&out).contains("holds no gpg"));
    let out = open_py(
        vectors::ALL,
        "not the passphrase",
        &["--field", "gpg.key", "--out", to_arg],
    )
    .unwrap();
    assert!(!out.status.success());
    assert!(stderr(&out).contains("Wrong passphrase. A file that also needs a key file or a security key cannot be opened here."));
    assert!(!to.exists());
    assert!(out.stdout.is_empty());
}

/// What this reader names, as the Spec does: a damaged file and a KDBX 3.1
/// file.
#[test]
fn open_py_refuses_damage_and_an_older_version() {
    for name in [
        "cut-stream.kdbx",
        "refused-block-data.kdbx",
        "refused-cut-block.kdbx",
    ] {
        let Some(out) = open_py(name, vectors::PASSPHRASE, &[]) else {
            return;
        };
        assert!(!out.status.success(), "{name}");
        assert!(out.stdout.is_empty(), "{name}");
    }
    let out = open_py("refused-header-hmac.kdbx", vectors::PASSPHRASE, &[]).unwrap();
    assert!(
        stderr(&out).contains("Wrong passphrase."),
        "{}",
        stderr(&out)
    );
    let old = repo("tools/vectors/kdbx/keepassxc-3.1-aeskdf.kdbx");
    let out = open_py_on(&old, "pw", &[]).unwrap();
    assert!(
        stderr(&out).contains("A KeePass file of an older version"),
        "{}",
        stderr(&out)
    );
}

fn qr_vector(name: &str) -> PathBuf {
    repo(&format!("tools/vectors/vault/qr/{name}"))
}

/// open.py on a QR vault: it needs `sq`, not pykeepass, and a missing `sq`
/// fails the test (DEVELOPING.md).
fn open_py_qr(name: &str, passphrase: &str, extra: &[&str]) -> Output {
    let out = run_open_py(&qr_vector(name), passphrase, extra).expect("python3 starts");
    assert!(
        !stderr(&out).contains("needs Sequoia's sq"),
        "sq is not installed"
    );
    out
}

#[test]
fn open_py_opens_a_qr_vault_through_sq_and_prints_its_item() {
    let out = open_py_qr("qr-note.pgp", "correct horse battery staple", &[]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "QR vault\n  note: The safe's code is in the drawer.\n"
    );
    let out = open_py_qr("qr-codex32.pgp", "correct horse battery staple", &[]);
    assert_eq!(
        stdout(&out),
        "QR vault\n  codex32: ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn\n  for: fab6868a\n"
    );
    let out = open_py_qr("qr-seed-12.pgp", "correct horse battery staple", &[]);
    assert!(stdout(&out).starts_with("QR vault\n  words: abandon abandon "));
    let out = open_py_qr("qr-empty.pgp", "correct horse battery staple", &[]);
    assert_eq!(stdout(&out), "QR vault: empty\n");
}

#[test]
fn open_py_opens_the_qr_nfd_vault_under_either_form() {
    for p in ["cafe\u{301}", "caf\u{e9}"] {
        let out = open_py_qr("qr-nfd.pgp", p, &[]);
        assert_eq!(stdout(&out), "QR vault\n  note: caf\u{e9}\n");
    }
}

#[test]
fn open_py_writes_a_qr_vaults_value_and_refuses_a_wrong_passphrase_or_a_message_sq_refuses() {
    let dir = scratch("qr-field");
    let to = dir.0.join("note");
    let out = open_py_qr(
        "qr-note.pgp",
        "correct horse battery staple",
        &["--field", "note", "--out", to.to_str().unwrap()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(out.stdout.is_empty());
    assert_eq!(
        std::fs::read_to_string(&to).unwrap(),
        "The safe's code is in the drawer."
    );
    let out = open_py_qr("qr-note.pgp", "not it", &[]);
    assert!(!out.status.success());
    assert!(out.stdout.is_empty());
    // What `sq` itself cannot open: a compressed packet where the SEIPD goes
    // and data after it. open.py reads whatever `sq` opens, so it prints
    // nothing for these two; the other refused files are Faraday's Rust's to
    // refuse (`tests/qr.rs`), and `sq` opens a second SKESK.
    for name in ["refused-compressed.pgp", "refused-trailing.pgp"] {
        let out = open_py_qr(name, "correct horse battery staple", &[]);
        assert!(!out.status.success(), "{name}");
        assert!(out.stdout.is_empty(), "{name}");
    }
}
