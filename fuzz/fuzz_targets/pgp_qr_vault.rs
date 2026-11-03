//! `faraday_pgp::message` (the QR vault's OpenPGP message, F190): `parse`
//! on the input, which never panics and never accepts a cost outside the
//! limits of `docs/VAULT.md` §3; and, where the stated cost is the least
//! one (8 MiB, so a run is quick), `open` under the seed's passphrase,
//! which gives the plaintext only for a message whose every tag checks,
//! and then the plaintext is the seed's.

#![no_main]

use faraday_crypto::work::Compute;
use faraday_pgp::message::{EXPONENTS, MAX_LANES, MAX_PASSES, parse};
use libfuzzer_sys::fuzz_target;

/// The passphrase of `corpus/pgp_qr_vault/vault-13.pgp`.
const PASSPHRASE: &[u8] = b"correct horse";
/// The plaintext of that file: a 12-word seed padded to 226 bytes.
const PLAIN_START: &[u8] = b"words = \"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about\"\n";

fuzz_target!(|data: &[u8]| {
    let Ok(message) = parse(data) else { return };
    let cost = message.cost();
    assert!(EXPONENTS.contains(&cost.exponent));
    assert!((1..=MAX_PASSES).contains(&cost.passes));
    assert!((1..=MAX_LANES).contains(&cost.lanes));
    if cost.exponent == *EXPONENTS.start()
        && cost.passes <= 3
        && cost.lanes <= 4
        && let Ok(plain) = message.open(&Compute::unchecked(), PASSPHRASE)
    {
        assert!(plain.starts_with(PLAIN_START));
        assert!(plain[PLAIN_START.len()..].iter().all(|&b| b == b'\n'));
    }
});
