//! A QR vault's packets (`docs/VAULT.md` §1.2), from any file or code the
//! scanners and Files test: `faraday_pgp::message::parse` runs on every
//! file Files lists and every code a scanner reads, with no passphrase.
//! It never panics, and the Argon2 cost it states is read back within the
//! limits, before anything is derived.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(message) = faraday_pgp::message::parse(data) {
        let cost = message.cost();
        let _ = cost.memory_kib();
        assert!(faraday_pgp::message::EXPONENTS.contains(&cost.exponent));
        assert!((1..=faraday_pgp::message::MAX_PASSES).contains(&cost.passes));
        assert!((1..=faraday_pgp::message::MAX_LANES).contains(&cost.lanes));
    }
});
