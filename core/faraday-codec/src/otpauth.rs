//! The `otpauth://` scheme of a TOTP entry.

const OTPAUTH: &str = "otpauth://";

/// Whether a line begins with the `otpauth://` scheme, in either case: the
/// spelling the TOTP reader accepts and an entry stores.
pub fn is_otpauth(line: &str) -> bool {
    line.get(..OTPAUTH.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(OTPAUTH))
}
