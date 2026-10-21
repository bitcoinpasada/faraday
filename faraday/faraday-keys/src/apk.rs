//! Android signing keys and APK signatures (faraday-apk).

use faraday_apk::{Apk, Source};
use faraday_sb::rng::Stream;

/// A key and its certificate.
pub use faraday_sb::Pair;

/// Makes a signing key and its certificate from `seed`, named `name`,
/// valid from `now`. Costs an RSA key generation.
pub fn make(seed: &[u8; 32], name: &str, now: u64) -> Result<Pair, faraday_apk::Error> {
    faraday_apk::key::make(seed, name, now)
}

/// Signs the APK `src` (read as `apk`) with the PKCS #8 key `pkcs8` and
/// its certificate `cert`, the signing blinded from `seed` (fresh bytes
/// of the system's); the signed file. Costs RSA signatures over the whole
/// file.
pub fn sign(
    seed: &[u8; 32],
    src: &mut dyn Source,
    apk: &Apk,
    pkcs8: &[u8],
    cert: &[u8],
) -> Result<Vec<u8>, faraday_apk::Error> {
    let mut blind = Stream::new(seed, 0);
    faraday_apk::sign::sign(&mut blind, src, apk, pkcs8, cert)
        .and_then(|s| faraday_apk::sign::assemble(src, apk, &s))
}
