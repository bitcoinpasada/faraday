//! Android signing keys and APK signatures (faraday-apk).

use faraday_apk::{Apk, Source};
use faraday_sb::rng::Stream;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

use crate::vault::MadeKey;

/// A key made here ([`create`]): its record, and its certificate.
pub struct Made {
    /// The record every vault of the plan takes: the private key, the
    /// certificate and the package.
    pub key: MadeKey,
    /// The certificate (DER), which is public.
    pub cert: Vec<u8>,
}

/// Costs an RSA key generation. A signing key and its certificate from
/// `seed` (fresh bytes of the system's), named `name`, valid from `now`,
/// for `package`.
pub fn create(
    seed: &[u8; 32],
    name: &str,
    package: &str,
    now: u64,
) -> Result<Made, faraday_apk::Error> {
    let pair = faraday_apk::key::make(seed, name, now)?;
    let record = Record::new(kind::APK_KEY)
        .with(field::APK_KEY, &pair.key)
        .with(field::APK_CERT, &pair.cert)
        .with(field::APK_PACKAGE, package.as_bytes());
    Ok(Made {
        key: MadeKey::of(record),
        cert: pair.cert.clone(),
    })
}

/// Signs the APK `src` (read as `apk`) with the PKCS #8 key `pkcs8` and
/// its certificate `cert`, the signing blinded from `seed` (fresh bytes
/// of the system's); the signed file. Costs RSA signatures over the whole
/// file.
pub(crate) fn sign(
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
