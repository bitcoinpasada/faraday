//! A boot program is signed only when it is a Faraday release's own
//! (`docs/decisions/F134.md`): the release's `SHA256SUMS` and
//! `SHA256SUMS.asc` are beside it, the signature verifies with a release
//! key built into Faraday, and `SHA256SUMS` lists the file's SHA-256 under
//! the boot program's name.
//!
//! [`check`] gives a [`Release`] only then; [`sign`] signs an image only
//! with one, and only the image whose SHA-256 it names. A release
//! publishes its boot program as `faraday-X.Y.Z-BOOTX64.EFI`
//! ([`boot_program_file`]) and lists it in `SHA256SUMS` under that name,
//! as `faraday/tools/stage-release.sh` writes it: the plain SHA-256 of the
//! unsigned file, so `sha256sum -c` checks the same line Faraday does, and
//! a file already signed does not match. Faraday finds the line by the
//! release's version, not by the name the file has on the stick.

use alloc::string::String;

use faraday_pgp::{Certificate, verify_detached};
use sha2::{Digest, Sha256};

use crate::{Error, x509};

/// The name of a release's sums, beside the boot program.
pub const SUMS: &str = "SHA256SUMS";
/// The name of their detached signature.
pub const SUMS_SIGNATURE: &str = "SHA256SUMS.asc";
/// The boot program's name on a stick's boot partition.
pub const BOOT_PROGRAM: &str = "BOOTX64.EFI";

/// The boot program's name as release `version` publishes it and its
/// `SHA256SUMS` lists it.
pub fn boot_program_file(version: &str) -> String {
    let mut name = String::from("faraday-");
    name.push_str(version);
    name.push('-');
    name.push_str(BOOT_PROGRAM);
    name
}

/// A release key: its armoured certificate and the fingerprint its
/// primary key must have.
#[derive(Debug, Clone, Copy)]
pub struct ReleaseKey<'a> {
    /// The armoured public certificate.
    pub certificate: &'a [u8],
    /// Its primary key's fingerprint.
    pub fingerprint: [u8; 20],
}

/// The release keys built into Faraday: today one, `faraday/release-key.asc`,
/// D1AF A712 ABEF 9010 9A1B 587F F153 6BE0 7E23 4CFD (`README.md`,
/// "Verifying a release"). Across a key rotation the list holds the old
/// key and the new one, and the old one leaves it in a later release
/// (F134).
pub const KEYS: &[ReleaseKey<'static>] = &[ReleaseKey {
    certificate: include_bytes!("../../release-key.asc"),
    fingerprint: [
        0xD1, 0xAF, 0xA7, 0x12, 0xAB, 0xEF, 0x90, 0x10, 0x9A, 0x1B, 0x58, 0x7F, 0xF1, 0x53, 0x6B,
        0xE0, 0x7E, 0x23, 0x4C, 0xFD,
    ],
}];

/// Why a boot program is not taken as a release's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not an EFI image: the file does not read as a PE image, so it has
    /// no SHA-256 taken for it to compare.
    NotImage,
    /// `SHA256SUMS` or `SHA256SUMS.asc` is not beside it, or the sums
    /// are not a release's (their first line does not name a version).
    NoSums,
    /// The signature does not verify with any release key.
    NotSigned,
    /// The release is older than the Faraday checking it: a genuine
    /// older boot program is not signed (no downgrade, F134).
    Older {
        /// The release the sums are for.
        version: String,
        /// The Faraday checking it.
        running: String,
    },
    /// The release's sums do not list this file as its boot program,
    /// exactly once.
    NotListed {
        /// The release the sums are for.
        version: String,
    },
}

/// A boot program checked as a release's own: made only by [`check`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    version: String,
    sha256: [u8; 32],
}

impl Release {
    /// The release, as its sums' first line names it.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The SHA-256 of the boot program the sums list.
    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// Costs a hash of the sums and three Ed25519 verifications for each key
/// tried. The boot program whose SHA-256 is `file_sha256`, checked
/// against the sums and their signature found beside it, with `keys`
/// ([`KEYS`] in Faraday), by the Faraday whose version is `running`
/// (`CARGO_PKG_VERSION`): the release must be at least as new.
///
/// Fail closed throughout: the signature must verify before anything in
/// the sums is read; the sums' first line must be `# Faraday X.Y.Z,
/// commit …`; the boot program's line, `faraday-X.Y.Z-BOOTX64.EFI` for
/// that version, must be there exactly once (two spaces or ` *` before
/// the name, LF or CRLF line ends), with this file's SHA-256.
pub fn check(
    file_sha256: &[u8; 32],
    sums: Option<&[u8]>,
    signature: Option<&[u8]>,
    keys: &[ReleaseKey<'_>],
    running: &str,
) -> Result<Release, Refusal> {
    let (Some(sums), Some(signature)) = (sums, signature) else {
        return Err(Refusal::NoSums);
    };
    let signed = keys.iter().any(|k| {
        Certificate::read(k.certificate).is_ok_and(|c| {
            c.fingerprint() == k.fingerprint && verify_detached(&c, sums, signature).is_ok()
        })
    });
    if !signed {
        return Err(Refusal::NotSigned);
    }
    let text = core::str::from_utf8(sums).map_err(|_| Refusal::NoSums)?;
    let version = version(text).ok_or(Refusal::NoSums)?;
    let at_least = match (numbers(version), numbers(running_release(running))) {
        (Some(v), Some(r)) => v >= r,
        _ => false,
    };
    if !at_least {
        return Err(Refusal::Older {
            version: String::from(version),
            running: String::from(running),
        });
    }
    let file = boot_program_file(version);
    let mut listed = text
        .lines()
        .filter_map(line)
        .filter(|(_, name)| *name == file);
    match (listed.next(), listed.next()) {
        (Some((sha256, _)), None) if &sha256 == file_sha256 => Ok(Release {
            version: String::from(version),
            sha256,
        }),
        _ => Err(Refusal::NotListed {
            version: String::from(version),
        }),
    }
}

/// The version the sums' first line names: `# Faraday X.Y.Z, commit …`.
fn version(text: &str) -> Option<&str> {
    let first = text.lines().next()?;
    let first = first.strip_suffix('\r').unwrap_or(first);
    let (version, _) = first.strip_prefix("# Faraday ")?.split_once(", commit ")?;
    numbers(version).map(|_| version)
}

/// A running build's release, its pre-release or build part left off
/// (`0.1.8-rc.1` is 0.1.8's).
fn running_release(running: &str) -> &str {
    running.split(['-', '+']).next().unwrap_or(running)
}

/// `X.Y.Z` as three numbers, each digits only.
fn numbers(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|p| {
        (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse::<u64>().ok())
            .flatten()
    });
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}

/// A line of `sha256sum`'s output: 64 hex digits, then two spaces (or a
/// space and `*`, its binary mode), then the name; a CR before the line's
/// end is left off.
fn line(l: &str) -> Option<([u8; 32], &str)> {
    let l = l.strip_suffix('\r').unwrap_or(l);
    let (hex, name) = (l.get(..64)?, l.get(64..)?);
    let name = name
        .strip_prefix("  ")
        .or_else(|| name.strip_prefix(" *"))?;
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some((out, name))
}

/// Costs a SHA-256 of `pe` and an RSA signature. The boot program `pe`
/// signed by `signer` (the db key), only when it is the file `release`
/// names. The only way this crate signs an image outside its own tests.
pub fn sign(
    rng: &mut crate::rng::Stream,
    pe: &[u8],
    signer: &x509::Pair,
    release: &Release,
) -> Result<alloc::vec::Vec<u8>, Error> {
    let sha256: [u8; 32] = Sha256::digest(pe).into();
    if &sha256 != release.sha256() {
        return Err(Error::NotRelease);
    }
    crate::pe::sign_image(rng, pe, signer)
}
