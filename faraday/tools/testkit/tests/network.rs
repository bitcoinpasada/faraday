//! The test stick made for Mainnet: what the website's tour and the user
//! guide render from must read as a Mainnet stick holds it.

use std::path::PathBuf;
use std::process::Command;

/// A directory of the test's own, removed when it ends.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every file `faraday-testkit` writes into a fresh directory with `args`.
fn written(name: &str, args: &[&str]) -> (Scratch, Vec<(String, Vec<u8>)>) {
    let dir = Scratch(
        std::env::temp_dir().join(format!("faraday-testkit-{name}-{}", std::process::id())),
    );
    let out = Command::new(env!("CARGO_BIN_EXE_faraday-testkit"))
        .args(args)
        .arg(&dir.0)
        .output()
        .expect("faraday-testkit runs");
    assert!(
        out.status.success(),
        "faraday-testkit {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files = std::fs::read_dir(&dir.0)
        .expect("written")
        .flatten()
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).expect("read"),
            )
        })
        .collect();
    (dir, files)
}

fn holds(files: &[(String, Vec<u8>)], text: &str) -> bool {
    files
        .iter()
        .any(|(_, b)| b.windows(text.len()).any(|w| w == text.as_bytes()))
}

#[test]
fn mainnet_backup_stick_has_xpubs_at_coin_type_zero() {
    let (_dir, files) = written("mainnet", &["--network", "mainnet", "--backup"]);
    assert!(!files.is_empty());
    assert!(holds(&files, "xpub6"), "no xpub in the Mainnet kit");
    assert!(holds(&files, "/48h/0h/0h/3h]"), "no coin type 0 path");
    assert!(!holds(&files, "tpubD"), "a tpub in the Mainnet kit");
}

#[test]
fn default_backup_stick_stays_on_testnet() {
    let (_dir, files) = written("testnet", &["--backup"]);
    assert!(holds(&files, "tpubD"), "no tpub in the default kit");
    assert!(holds(&files, "/48h/1h/0h/3h]"), "no coin type 1 path");
    assert!(!holds(&files, "xpub6"), "an xpub in the default kit");
}

#[test]
fn unknown_network_is_refused() {
    let out = Command::new(env!("CARGO_BIN_EXE_faraday-testkit"))
        .args(["--network", "moon", "unused"])
        .output()
        .expect("faraday-testkit runs");
    assert_eq!(out.status.code(), Some(2));
}
