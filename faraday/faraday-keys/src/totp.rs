//! Time-based one-time codes from a vault entry's TOTP field (RFC 6238).
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

pub use faraday_vault::totp::NotTotp;
use zeroize::Zeroizing;

/// A code as it is shown.
pub struct Shown {
    /// The digits, grouped for reading; wiped when dropped, as a code
    /// held while it shows is.
    pub grouped: Zeroizing<String>,
    /// Seconds until it changes.
    pub left: u32,
    /// The period it lasts, in seconds.
    pub period: u64,
}

/// The code `field` gives at `now` (Unix seconds); `Ok(None)` when the
/// field reads as a TOTP key but there is no `now` yet (the clock is not
/// checked). Cheap: one HMAC.
pub fn shown(field: &str, now: Option<u64>) -> Result<Option<Shown>, NotTotp> {
    match now {
        Some(now) => code_at(field, now).map(Some),
        None => faraday_vault::totp::parse(field).map(|_| None),
    }
}

/// Cheap: one HMAC. The code a TOTP the vault crate read gives at `now`
/// (Unix seconds); `None` when there is no `now` yet.
pub fn shown_of(t: &faraday_vault::totp::Totp, now: Option<u64>) -> Option<Shown> {
    let now = now?;
    Some(Shown {
        grouped: faraday_vault::totp::grouped(&t.code(now)).into(),
        left: t.left(now),
        period: t.period(),
    })
}

/// The code `field` gives at `now` (Unix seconds). Cheap: one HMAC.
pub fn code_at(field: &str, now: u64) -> Result<Shown, NotTotp> {
    let code = faraday_vault::totp::parse(field)?;
    Ok(Shown {
        grouped: faraday_vault::totp::grouped(&code.code(now)).into(),
        left: code.left(now),
        period: code.period(),
    })
}

/// The field as it is stored, for a key or URI typed in as `title` and
/// `account`; `None` when it is no TOTP key.
pub fn normalise(field: &str, title: &str, account: &str) -> Option<Zeroizing<String>> {
    faraday_vault::totp::normalise(field, title, account)
}
