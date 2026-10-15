//! The APK signing key and its certificate (`docs/APK-SIGNING.md` §2):
//! RSA-3072 with a self-signed certificate naming the project, valid from
//! midnight of the day it was made until 9999-12-31.

use alloc::string::String;
use alloc::vec::Vec;

use faraday_sb::Pair;
use faraday_sb::rng::Stream;
use faraday_sb::x509::{self, Cert, Profile};
use sha2::{Digest, Sha256};

use crate::Error;

/// The key's size in bits.
pub const BITS: usize = 3072;
/// The most characters X.520 allows a common name.
pub const NAME_MAX: usize = 64;
/// The longest an Android package name is.
pub const PACKAGE_MAX: usize = 255;

/// Why a name or a package was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// The certificate's name is empty once trimmed.
    NoName,
    /// The package is not Android's form.
    Package,
}

/// The certificate's name, trimmed, or why not.
pub fn check_name(name: &str) -> Result<String, Invalid> {
    let n = name.trim();
    if n.is_empty() {
        return Err(Invalid::NoName);
    }
    Ok(n.chars().take(NAME_MAX).collect())
}

/// Whether `p` is an Android package name: at least two parts joined by
/// dots, each a letter followed by letters, digits or underscores, ASCII
/// only, at most 255 bytes.
pub fn valid_package(p: &str) -> bool {
    if p.len() > PACKAGE_MAX {
        return false;
    }
    let mut parts = 0;
    for part in p.split('.') {
        let mut c = part.chars();
        if !c.next().is_some_and(|f| f.is_ascii_alphabetic())
            || !c.all(|x| x.is_ascii_alphanumeric() || x == '_')
        {
            return false;
        }
        parts += 1;
    }
    parts >= 2
}

/// Makes the key and its certificate from 32 bytes of the system's
/// entropy, which are not kept (`docs/APK-SIGNING.md` §2.3). `now` is
/// truncated to midnight UTC. The prime search belongs to the `rsa` crate
/// and may change between its versions: the key itself is what a vault
/// keeps, never the seed.
pub fn make(seed: &[u8; 32], name: &str, now: u64) -> Result<Pair, Error> {
    let name = check_name(name).map_err(|_| Error::Key)?;
    let mut ids = Stream::new(seed, 0);
    let mut serial = [0u8; 16];
    ids.fill(&mut serial);
    let mut keys = Stream::new(seed, 1);
    let mut blind = Stream::new(seed, 2);
    let midnight = now - now % 86_400;
    x509::generate(
        &mut keys,
        &mut blind,
        BITS,
        &name,
        &serial,
        midnight,
        Profile::NoExpiry,
    )
    .map_err(|_| Error::Key)
}

/// What a certificate states, as Faraday shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertInfo {
    /// Its common name.
    pub name: String,
    /// SHA-256 of its DER.
    pub fingerprint: [u8; 32],
    /// The start of its validity: midnight UTC of the day it was made.
    pub made: u64,
}

/// A certificate's name, fingerprint and date; `None` when it does not
/// read as one.
pub fn describe(cert: &[u8]) -> Option<CertInfo> {
    let c = Cert::read(cert)?;
    Some(CertInfo {
        name: c.cn?,
        fingerprint: fingerprint(cert),
        made: c.not_before?,
    })
}

/// SHA-256 of the certificate's DER: what `apksigner verify
/// --print-certs` prints, and F-Droid's `AllowedAPKSigningKeys` takes.
pub fn fingerprint(cert: &[u8]) -> [u8; 32] {
    Sha256::digest(cert).into()
}

/// A fingerprint in groups of four hex digits, lower case.
pub fn grouped(fp: &[u8; 32]) -> String {
    let hex = crate::hex(fp);
    hex.as_bytes()
        .chunks(4)
        .map(|c| core::str::from_utf8(c).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first `n` hex digits of a fingerprint, in groups of four.
pub fn grouped_prefix(fp: &[u8; 32], n: usize) -> String {
    let hex = crate::hex(fp);
    hex.as_bytes()[..n.min(hex.len())]
        .chunks(4)
        .map(|c| core::str::from_utf8(c).unwrap_or(""))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The public key (SubjectPublicKeyInfo DER) in a certificate.
pub fn public_key(cert: &[u8]) -> Option<&[u8]> {
    let c = Cert::read(cert)?;
    cert.get(c.spki)
}
