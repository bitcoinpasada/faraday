//! Time-based one-time codes from a vault entry's TOTP field (RFC 6238).

pub use faraday_vault::totp::NotTotp;

/// A code as it is shown.
pub struct Shown {
    /// The digits, grouped for reading.
    pub grouped: String,
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

/// The code `field` gives at `now` (Unix seconds). Cheap: one HMAC.
pub fn code_at(field: &str, now: u64) -> Result<Shown, NotTotp> {
    let code = faraday_vault::totp::parse(field)?;
    Ok(Shown {
        grouped: faraday_vault::totp::grouped(&code.code(now)),
        left: code.left(now),
        period: code.period(),
    })
}

/// The field as it is stored, for a key or URI typed in as `title` and
/// `account`; `None` when it is no TOTP key.
pub fn normalise(field: &str, title: &str, account: &str) -> Option<String> {
    faraday_vault::totp::normalise(field, title, account)
}
