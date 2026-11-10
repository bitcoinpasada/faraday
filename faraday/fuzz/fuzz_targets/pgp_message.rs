//! A QR vault's packets (`docs/VAULT.md` §1.2), from any file or code the
//! scanners and Files test: `faraday_pgp::message::parse` runs on every
//! file Files lists and every code a scanner reads, with no passphrase.
//! It never panics, and the Argon2 cost it states is read back within the
//! limits, before anything is derived.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(message) = faraday_pgp::message::parse(data) {
        let _ = message.cost().memory_kib();
    }
});
