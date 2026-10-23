//! GPG keys in a vault (`docs/history/PLAN.md` §7, F32): making one, and what it makes —
//! the public certificate, detached signatures over Inbox files, a
//! revocation certificate kept with the key, a renewed expiry, and the
//! paperkey text. The packets are `faraday-pgp`'s; the key lives in an open
//! vault as a type-7 record (`docs/VAULT.md` §7) and only its public
//! results reach the Outbox. What GnuPG does, this does: F32 lists each
//! behaviour copied and how it was checked against GnuPG 2.4.
//!
//! Its presses and what they change are the vault area's glue, in
//! `vault/mod.rs`.

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;

use faraday_keys::vault::{GpgFacts as Facts, RecordView, gpg_facts, gpg_field as f};

/// GnuPG's year: 365 days, as `--quick-set-expire 2y` counts it.
pub const YEAR: u32 = 31_536_000;

/// A new key's term in years, as `gpg --quick-generate-key` makes it with
/// no expiry given (GnuPG 2.4: `3y`).
pub const DEFAULT_YEARS: u32 = 3;

/// How far the clock may be before the key's last self-signature for a
/// renewal to go on, in seconds: GnuPG makes the new signature one second
/// after the last when the clock is within 4 seconds of it, and refuses
/// ("Time conflict") from 5 on.
pub(crate) const SKEW: u32 = 5;

/// The longest text a signing page shows of a file.
pub(crate) const TEXT_MAX: usize = 2048;

pub(crate) fn u32_of(r: RecordView<'_>, n: u8) -> Option<u32> {
    Some(u32::from_le_bytes(r.public(n)?.try_into().ok()?))
}

/// When the certificate in a record was last signed: the time of its
/// self-signatures.
pub fn certified_at(r: RecordView<'_>) -> Option<u32> {
    faraday_pgp::signature_time(r.public(f::CERTIFICATE)?)
}

/// When a record's key was marked revoked.
pub(crate) fn revoked_at(r: RecordView<'_>) -> Option<u32> {
    u32_of(r, f::REVOKED)
}

/// Costs a hash of a whole file (through `sha256_of`). SHA-256 of a
/// file, as the signing page shows and ticks it.
pub fn sha256(c: &Compute, bytes: &[u8]) -> [u8; 32] {
    crate::sha256_of(c, bytes)
}

/// Costs what `sha256` costs. [`sha256`] in lower-case hex.
pub fn sha256_hex(c: &Compute, bytes: &[u8]) -> String {
    sha256(c, bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A Unix time as a date, `YYYY-MM-DD`.
pub fn date(unix: u64) -> String {
    faraday_pgp::utc_date(unix)
}

/// A key's ID as GnuPG prints it: the last 16 hex digits of its
/// fingerprint.
pub fn key_id(fp: &[u8; 20]) -> String {
    fp[12..].iter().map(|b| format!("{b:02X}")).collect()
}

/// A short name for files: the user ID's name part, lower-case, and the
/// fingerprint's last eight digits.
pub(crate) fn file_stem(key: &Facts) -> String {
    let name = name_part(key);
    let fp = key.fingerprint;
    let tail: String = fp[16..].iter().map(|b| format!("{b:02X}")).collect();
    if name.is_empty() {
        tail
    } else {
        format!("{name}-{tail}")
    }
}

/// The user ID's name, as a file name part: lower-case letters and digits
/// with dashes between.
pub(crate) fn name_part(key: &Facts) -> String {
    let name = key
        .user_ids
        .first()
        .map(|u| u.split('<').next().unwrap_or(u).trim().to_string())
        .unwrap_or_default();
    let name: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    name.trim_matches('-').to_string()
}

/// A revocation file's name, `{name}-{FPR}.rev`: the name GnuPG gives
/// `openpgp-revocs.d/<FPR>.rev`, with the name before it.
pub(crate) fn rev_name(key: &Facts) -> String {
    let fpr: String = key.fingerprint.iter().map(|b| format!("{b:02X}")).collect();
    match name_part(key) {
        n if n.is_empty() => format!("{fpr}.rev"),
        n => format!("{n}-{fpr}.rev"),
    }
}

/// How long ago, as GnuPG says it: seconds below a day, then days.
pub(crate) fn span(secs: u32) -> String {
    match secs {
        1 => fill(EN.gpg_second, &["1"]),
        s if s < 86_400 => fill(EN.gpg_seconds, &[&s.to_string()]),
        s if s / 86_400 == 1 => fill(EN.gpg_day, &["1"]),
        s => fill(EN.gpg_days, &[&(s / 86_400).to_string()]),
    }
}

/// Whether a key can sign at `now`: not revoked, not expired, and made
/// before now. GnuPG's refusals: a key it cannot use is "Unusable secret
/// key"; one made in the future says so, with its ID.
pub(crate) fn usable(key: &Facts, revoked: Option<u32>, now: u32) -> Result<(), String> {
    if let Some(t) = revoked {
        return Err(fill(EN.gpg_revoked, &[&date(u64::from(t))]));
    }
    if key.expiry > 0 {
        let end = key.created.saturating_add(key.expiry);
        if now >= end {
            return Err(fill(EN.gpg_expired, &[&date(u64::from(end))]));
        }
    }
    for (created, fp) in [
        (key.created, key.fingerprint),
        (key.subkey_created, key.subkey_fingerprint),
    ] {
        if created > now {
            return Err(fill(EN.gpg_future, &[&key_id(&fp), &span(created - now)]));
        }
    }
    Ok(())
}

/// An Inbox file the signing page lists: what is signed is what is shown.
#[derive(Debug, Clone)]
pub struct GpgFile {
    /// Its place in the Inbox.
    pub index: usize,
    /// Its name.
    pub name: String,
    /// Its size.
    pub size: usize,
    /// SHA-256 of its bytes.
    pub hash: [u8; 32],
    /// Its text, when it is a small text file.
    pub text: Option<String>,
}

/// What the signing page remembers of a file so that it is hashed once: the
/// file's name, size, place in memory and both ends. A file that a stick
/// visit replaced differs in one of them; the hash is taken again at
/// signing in any case.
#[derive(Debug, Clone)]
pub struct Seen {
    pub(crate) name: String,
    pub(crate) probe: (usize, usize, Vec<u8>),
    pub(crate) hash: [u8; 32],
    pub(crate) text: Option<String>,
}

pub(crate) fn probe_of(bytes: &[u8]) -> (usize, usize, Vec<u8>) {
    let mut ends = bytes[..bytes.len().min(32)].to_vec();
    ends.extend_from_slice(&bytes[bytes.len().saturating_sub(32)..]);
    (bytes.len(), bytes.as_ptr() as usize, ends)
}

/// A small text file's text: UTF-8, printable, a few lines.
pub(crate) fn small_text(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() || bytes.len() > TEXT_MAX {
        return None;
    }
    let text = std::str::from_utf8(bytes).ok()?;
    text.chars()
        .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
        .then(|| text.to_string())
}

/// What the detail column shows of a GPG key: its fingerprints, user IDs,
/// dates and revocation. The paperkey text is not here: it is made when
/// it is shown ([`paperkey`]).
pub struct Shown {
    /// The primary fingerprint, as GnuPG prints it.
    pub fingerprint: String,
    /// The signing subkey's.
    pub subkey: String,
    /// The signing subkey's ID.
    pub subkey_id: String,
    /// The primary key's ID.
    pub key_id: String,
    /// The user IDs.
    pub user_ids: Vec<String>,
    /// Created, as a date.
    pub created: String,
    /// Expires, as a date, or "Never".
    pub expires: String,
    /// The date a revocation certificate kept with the key was made.
    pub revocation_kept: Option<String>,
    /// The date the key was marked revoked.
    pub revoked: Option<String>,
}

/// What a type-7 record shows. The dates are local (`docs/TOTP.md`
/// §2.3); the key's own creation and expiry stay UTC (`key.created`).
pub fn shown(r: RecordView<'_>, tz_min: i16) -> Option<Shown> {
    let key = gpg_facts(r)?;
    let local = |t: u64| date(crate::vault::local_secs(t, tz_min));
    Some(Shown {
        fingerprint: faraday_pgp::fingerprint_text(&key.fingerprint),
        subkey: faraday_pgp::fingerprint_text(&key.subkey_fingerprint),
        subkey_id: key_id(&key.subkey_fingerprint),
        key_id: key_id(&key.fingerprint),
        user_ids: key.user_ids.clone(),
        created: local(u64::from(key.created)),
        expires: if key.expiry == 0 {
            EN.gpg_never.to_string()
        } else {
            local(u64::from(key.created) + u64::from(key.expiry))
        },
        revocation_kept: faraday_keys::vault::gpg_revocation_kept_at(r)
            .map(|t| local(u64::from(t))),
        revoked: revoked_at(r).map(|t| local(u64::from(t))),
    })
}
