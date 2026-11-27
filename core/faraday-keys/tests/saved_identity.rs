//! The identity a save into a vault keeps for a key record
//! (`Keys::saved_identity`) is the one the vault works out from the record
//! itself: a vault never shows another fingerprint than the record's.
#![allow(clippy::disallowed_methods)]

use faraday_bip::keys::{Fingerprint, Network};
use faraday_crypto::work::Compute;
use faraday_keys::vault::{self, RecordView};
use faraday_keys::{Keys, Part, Reveal};

/// BIP-39's published vector: 128 bits of zeros.
const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[expect(
    clippy::disallowed_methods,
    reason = "a mint: faraday-keys' tests, which cannot reach faraday-core's testkit"
)]
fn compute() -> Compute {
    Compute::new(faraday_crypto::work::Hooks {
        count: ignore,
        stop: || false,
        progress: |_, _| {},
    })
}

fn ignore(_: faraday_crypto::work::Work) {}

/// What the vault works out from the record the save makes.
fn worked_out(
    c: &Compute,
    keys: &Keys,
    with_passphrase: bool,
) -> Option<([u8; 32], Fingerprint, [u8; 32])> {
    let held = keys.key_record(
        c,
        0,
        None,
        with_passphrase,
        &[],
        Reveal::VaultEntry(Part::Words),
    )?;
    let id = vault::key_record_id(RecordView::of(&held))?;
    let (fp, mid) = vault::key_identity(c, Network::Mainnet, RecordView::of(&held))?;
    Some((id, fp, mid))
}

fn agrees(c: &Compute, keys: &Keys, with_passphrase: bool) {
    let kept = keys.saved_identity(c, 0, with_passphrase).expect("kept");
    assert_eq!(Some(kept), worked_out(c, keys, with_passphrase));
}

#[test]
fn words_key_without_passphrase() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words(&c, WORDS, "Savings", None).expect("a key");
    agrees(&c, &keys, false);
}

#[test]
fn words_key_with_passphrase_saved_with_it() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, WORDS, "TREZOR", "Savings", None)
        .expect("a key");
    agrees(&c, &keys, true);
    // The passphrase makes another key than the words alone.
    let mut plain = Keys::on(Network::Mainnet);
    plain.add_words(&c, WORDS, "Savings", None).expect("a key");
    let with = keys.saved_identity(&c, 0, true).expect("kept");
    let without = plain.saved_identity(&c, 0, false).expect("kept");
    assert_ne!(with.1, without.1);
}

#[test]
fn seed_key() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_seed(&c, &[7u8; 16], "Savings").expect("a key");
    agrees(&c, &keys, false);
}

#[test]
fn words_key_with_passphrase_saved_without_it_has_none() {
    let c = compute();
    let mut keys = Keys::on(Network::Mainnet);
    keys.add_words_with(&c, WORDS, "TREZOR", "Savings", None)
        .expect("a key");
    assert_eq!(keys.saved_identity(&c, 0, false), None);
}
