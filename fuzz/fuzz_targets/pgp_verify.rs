//! `faraday_pgp::verify_detached` (F134): the check Faraday runs on a
//! release's `SHA256SUMS.asc` before it signs a boot program, on bytes
//! from the input as the signature, as the signed data and as the
//! certificate, against the GnuPG-made fixture in
//! `tools/vectors/release-sums/`. It never panics, and it never takes
//! what the fixture's key did not sign: a signature it takes over the
//! fixture's sums says it was made when the fixture's was (any other
//! would need the key), it takes the fixture's signature over no other
//! data, and a certificate that takes it carries the fixture's subkey.

#![no_main]

use std::sync::OnceLock;

use faraday_pgp::{Certificate, verify_detached};
use libfuzzer_sys::fuzz_target;

const KEY: &[u8] = include_bytes!("../../tools/vectors/release-sums/release-key.asc");
const SUMS: &[u8] = include_bytes!("../../tools/vectors/release-sums/SHA256SUMS");
const ASC: &[u8] = include_bytes!("../../tools/vectors/release-sums/SHA256SUMS.asc");
/// When make.sh's clock said the fixture's signature was made.
const MADE: u32 = 1_780_000_000;

fn release() -> &'static Certificate {
    static CERT: OnceLock<Certificate> = OnceLock::new();
    CERT.get_or_init(|| Certificate::read(KEY).expect("the fixture's key"))
}

fuzz_target!(|data: &[u8]| {
    let cert = release();
    // The input as the signature over the fixture's sums.
    if let Ok(at) = verify_detached(cert, SUMS, data) {
        assert_eq!(at, MADE);
    }
    // The input as the data the fixture's signature is over.
    if verify_detached(cert, data, ASC).is_ok() {
        assert_eq!(data, SUMS);
    }
    // The input as a certificate.
    if let Ok(other) = Certificate::read(data)
        && verify_detached(&other, SUMS, ASC).is_ok()
    {
        assert_eq!(other.subkey_fingerprint(), cert.subkey_fingerprint());
    }
});
