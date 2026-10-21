//! The Android app signing key Faraday makes and uses
//! (`docs/APK-SIGNING.md`): an RSA-3072 key with a self-signed
//! certificate, a reader for the APKs it signs, and APK Signature Scheme
//! v2 and v3 signatures made with it.
//!
//! [`key`] makes the key and its certificate, [`zip`] and [`axml`] read an
//! APK as untrusted input in one way only, and [`sign`] writes the APK
//! Signing Block. The private key lives in a vault; this crate writes
//! nothing to one and draws no randomness of its own: the caller passes
//! the [`faraday_sb::rng::Stream`] that makes and blinds.
//!
//! `no_std` + `alloc`.

#![no_std]

extern crate alloc;

pub mod axml;
pub mod key;
pub mod pem;
pub mod sign;
pub mod zip;

pub use key::CertInfo;
pub use sign::Signed;
pub use zip::{Apk, Info, Refusal, Slice, Source};

/// Why something could not be made or signed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The file could not be read.
    Read,
    /// The key or certificate could not be made or read.
    Key,
    /// A signature did not check against the certificate.
    Signature,
}

impl Error {
    /// The reason in a few words.
    pub fn reason(self) -> &'static str {
        match self {
            Error::Read => "The file could not be read",
            Error::Key => "The key could not be made or read",
            Error::Signature => "The signature did not check. Nothing written.",
        }
    }
}

/// Lower-case hex.
pub fn hex(bytes: &[u8]) -> alloc::string::String {
    use alloc::string::String;
    use core::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}
