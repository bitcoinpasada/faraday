//! An EFI image the Secure Boot screen checks and signs, and a
//! certificate it checks against. The input's first byte splits it: that
//! many bytes (times 8) at the front are a certificate, the rest is a PE
//! image. `pe::image_hash` and `pe::check` run on the image against that
//! certificate and against a fixed one made by this crate's own key
//! maker; `x509::Cert::read` and `der::children` run on the certificate
//! bytes. Signing is `pe::sign` on the image with the fixed key.

#![no_main]

use std::sync::OnceLock;

use faraday_sb::{der, pe, rng, x509};
use libfuzzer_sys::fuzz_target;

/// A db key made once from a fixed seed: RSA key generation takes
/// seconds.
fn db() -> &'static x509::Pair {
    static DB: OnceLock<x509::Pair> = OnceLock::new();
    DB.get_or_init(|| faraday_sb::make(&[7u8; 32], "Fuzz", 1_700_000_000).expect("keys").db)
}

fuzz_target!(|data: &[u8]| {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let cut = (usize::from(first) * 8).min(rest.len());
    let (cert, image) = rest.split_at(cut);
    let _ = x509::Cert::read(cert);
    let _ = der::children(cert);
    let _ = der::read(cert, 0);
    let _ = pe::image_hash(image);
    let _ = pe::check(image, cert);
    let _ = pe::check(image, &db().cert);
    let mut stream = rng::Stream::new(&[1u8; 32], 0);
    let _ = pe::sign(&mut stream, image, db());
});
