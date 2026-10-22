//! The sample files open the way a person opens them: the encrypted
//! backup and the vault under `faraday`, the signed message against its
//! address, the setup code as the vault takes it.

use crate::common::{PASSPHRASE, key};
use faraday_backup::oskb::{self, Opened};
use faraday_bip::keys::Network;
use faraday_core::testkit::{self, compute};
use faraday_core::wallet::{Origin, Session};
use faraday_samples::SAMPLES;
use faraday_vault::records::{field, kind};

#[test]
fn the_backup_opens_under_the_passphrase_to_the_master_seed_of_every_byte_0x42() {
    let c = compute();
    let sample = key("sample_backup");
    match oskb::open_payload(&c, sample.file, PASSPHRASE.as_bytes()).expect("it opens") {
        Opened::Seed(seed) => assert_eq!(seed.as_slice(), [0x42u8; 32]),
        _ => panic!("a master seed"),
    }
    // Another passphrase does not open it.
    assert!(oskb::open_payload(&c, sample.file, b"password").is_err());
    assert!(oskb::open_payload(&c, sample.file, b"Faraday").is_err());
    // It is the vectors' payload: the same seed their backup holds.
    assert_eq!(
        sample.plain,
        Some(crate::common::fingerprint_of_seed(&[0x42u8; 32]))
    );
}

/// The other implementation of the format, `tools/backup/decrypt.py`,
/// prints the same master seed. `None` where python3 or its libraries are
/// not installed, which skips the check.
fn decrypt_with_script(bytes: &[u8]) -> Option<(String, String)> {
    use std::process::Command;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let probe = Command::new("python3")
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
    let path = std::env::temp_dir().join(format!("faraday-samples-{}.oskb", std::process::id()));
    std::fs::write(&path, bytes).expect("write the backup");
    let out = Command::new("python3")
        .arg(root.join("tools/backup/decrypt.py"))
        .arg(&path)
        .arg("--passphrase")
        .arg(PASSPHRASE)
        .current_dir(&root)
        .output()
        .expect("run the script");
    std::fs::remove_file(&path).ok();
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

#[test]
fn the_backup_opens_in_the_python_reader_to_the_same_seed() {
    let Some((kind, printed)) = decrypt_with_script(key("sample_backup").file) else {
        eprintln!(
            "skipped: python3 with argon2-cffi and pynacl, or cryptography 44+, is not installed"
        );
        return;
    };
    assert_eq!(kind, "seed");
    assert_eq!(printed, "42".repeat(32));
}

#[test]
fn the_vault_opens_under_the_passphrase_to_sample_key_1_and_the_savings_wallet() {
    let c = compute();
    let vault = SAMPLES.inputs.vault;
    assert!(faraday_vault::is_vault(vault));
    assert!(faraday_vault::open(&c, vault, b"password").is_err());
    let (_, contents) = faraday_vault::open(&c, vault, PASSPHRASE.as_bytes()).expect("it opens");
    let of = |k: u8| -> Vec<&faraday_vault::Record> {
        contents.records.iter().filter(|r| r.kind == k).collect()
    };
    let keys = of(kind::KEY);
    assert_eq!(keys.len(), 1);
    let wallets = of(kind::WALLET);
    assert_eq!(wallets.len(), 1);
    assert_eq!(
        wallets[0].field(field::WALLET),
        Some(testkit::savings().as_bytes())
    );
    assert_eq!(wallets[0].field(field::WALLET_NAME), Some(&b"Savings"[..]));
    // The key is the first test seed's words.
    let mut session = Session::on(Network::Testnet);
    let fp = session
        .keys
        .add_words(&c, &faraday_core::samples::repeated("bacon", 24), "", None)
        .expect("bacon");
    assert_eq!(Some(fp.0), key("sample_words").plain);
    // Another passphrase does not open it.
    assert!(faraday_vault::open(&c, vault, b"decoy").is_err());
}

/// Sample Spending in a session on Testnet, with bacon loaded.
fn spending() -> (Session, usize) {
    let c = compute();
    let mut session = Session::on(Network::Testnet);
    let spending = SAMPLES
        .wallets
        .iter()
        .find(|w| w.name == "sample_spending")
        .expect("spending");
    session
        .add_wallet(&c, "Spending", spending.text, "Sample", Origin::Statement)
        .expect("wallet");
    (session, 0)
}

#[test]
fn the_address_is_spendings_first_receive_address() {
    let c = compute();
    let (session, w) = spending();
    let address = session.address(&c, &session.wallets[w], false, 0);
    assert_eq!(SAMPLES.inputs.address, address);
}

#[test]
fn the_signed_message_checks_against_that_address_and_belongs_to_spending() {
    let c = compute();
    let (session, w) = spending();
    let signed = session
        .check_message(&c, SAMPLES.inputs.signed_message)
        .expect("a signed message");
    assert!(signed.verdict.is_ok(), "{:?}", signed.verdict);
    assert_eq!(signed.signed.address, SAMPLES.inputs.address);
    assert_eq!(signed.wallet, Some((w, false, 0)));
    // A message changed by a letter does not check.
    let altered = SAMPLES.inputs.signed_message.replace("sample", "Sample");
    let altered = session
        .check_message(&c, &altered)
        .expect("still a message");
    assert!(altered.verdict.is_err());
}

#[test]
fn the_setup_code_is_the_testkits_first_line_and_a_totp_uri() {
    let first = testkit::ENTRIES_FILE.lines().next().expect("a line");
    assert_eq!(SAMPLES.inputs.totp, first);
    assert!(SAMPLES.inputs.totp.starts_with("otpauth://totp/"));
    assert!(SAMPLES.inputs.totp.contains("secret=JBSWY3DPEHPK3PXP"));
}
