//! The sample files open the way a person opens them: the vault under
//! `faraday`, the signed message against its address, the setup code as
//! the vault takes it.

use crate::common::{PASSPHRASE, key};
use faraday_bip::keys::Network;
use faraday_core::testkit::{self, compute};
use faraday_core::wallet::{Origin, Session};
use faraday_samples::SAMPLES;
use faraday_vault::records::{field, kind};

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
