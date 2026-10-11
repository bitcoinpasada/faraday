//! Reading and computing TOTP codes (`docs/TOTP.md` §4) from a vault
//! entry's field 6: an `otpauth://totp/…` URI, or a bare Base32 key as
//! services show when a setup QR cannot be scanned.
//!
//! [`parse`] reads field 6; [`Totp::code`] and [`Totp::left`] are RFC
//! 6238 over a Unix time; [`normalise`] is what the entry form and the
//! Inbox import (`faraday-vault::entries`) store back into field 6.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use zeroize::Zeroizing;

/// The HMAC a TOTP secret's code is drawn from (§4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

/// Why a field 6 value does not [`parse`] as a TOTP secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotTotp {
    /// An `otpauth://hotp/…` URI: a HOTP code, not read here (§7).
    Hotp,
    /// Anything else: not a URI or bare key this reads, or a parameter
    /// outside what §4.1 allows.
    Unreadable,
}

/// A decoded TOTP secret and its parameters (§4.1). `Debug` prints no
/// secret bytes.
pub struct Totp {
    secret: Zeroizing<Vec<u8>>,
    algorithm: Algorithm,
    digits: u32,
    period: u64,
}

impl core::fmt::Debug for Totp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Totp")
            .field("algorithm", &self.algorithm)
            .field("digits", &self.digits)
            .field("period", &self.period)
            .finish()
    }
}

/// The RFC 4648 §6 Base32 alphabet.
const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Base32 decodes `s`. Spaces and `=` are always ignored; hyphens are
/// ignored only when `allow_hyphen` (the bare-key form, §4.1). Any other
/// character refuses, as does decoding to nothing.
fn base32_decode(s: &str, allow_hyphen: bool) -> Option<Vec<u8>> {
    let mut bits: u32 = 0;
    let mut bit_count: u32 = 0;
    let mut out = Vec::new();
    for c in s.chars() {
        if c == ' ' || c == '=' || (allow_hyphen && c == '-') {
            continue;
        }
        let up = c.to_ascii_uppercase();
        let v = ALPHABET.iter().position(|&b| b == up as u8)? as u32;
        bits = (bits << 5) | v;
        bit_count += 5;
        if bit_count >= 8 {
            bit_count -= 8;
            out.push(((bits >> bit_count) & 0xFF) as u8);
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

/// The fewest characters a bare key may have: 16 is 80 bits, the
/// shortest key services hand out. A shorter word typed into the field,
/// a password say, is not taken for a key.
const MIN_BARE_KEY_CHARS: usize = 16;

/// `s`, upper-cased, with spaces, hyphens and padding removed: the form
/// [`normalise`] writes a bare key's `secret` param in.
fn clean_base32_key(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Percent-encodes everything outside RFC 3986's unreserved characters,
/// byte by byte (`docs/TOTP.md` §4.3).
fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `parse(field)` from a type 6 field 6 value (§4.1).
pub fn parse(field: &str) -> Result<Totp, NotTotp> {
    let field = field.trim();
    let lower = field.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("otpauth://") {
        // Keep original-case bytes aligned with `lower`'s ASCII prefix
        // so the query string below is read from `field`, not `lower`.
        let rest = &field[field.len() - rest.len()..];
        let (otp_kind, rest) = rest.split_once('/').ok_or(NotTotp::Unreadable)?;
        if otp_kind.eq_ignore_ascii_case("hotp") {
            return Err(NotTotp::Hotp);
        }
        if !otp_kind.eq_ignore_ascii_case("totp") {
            return Err(NotTotp::Unreadable);
        }
        let query = rest.split_once('?').map_or("", |(_, q)| q);
        let mut secret: Option<&str> = None;
        let mut algorithm = Algorithm::Sha1;
        let mut digits: u32 = 6;
        let mut period: u64 = 30;
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = pair.split_once('=').ok_or(NotTotp::Unreadable)?;
            match k {
                "secret" => secret = Some(v),
                "algorithm" => {
                    algorithm = match v.to_ascii_uppercase().as_str() {
                        "SHA1" => Algorithm::Sha1,
                        "SHA256" => Algorithm::Sha256,
                        "SHA512" => Algorithm::Sha512,
                        _ => return Err(NotTotp::Unreadable),
                    };
                }
                "digits" => {
                    digits = v.parse().map_err(|_| NotTotp::Unreadable)?;
                    if !(6..=8).contains(&digits) {
                        return Err(NotTotp::Unreadable);
                    }
                }
                "period" => {
                    period = v.parse().map_err(|_| NotTotp::Unreadable)?;
                    if !(1..=3600).contains(&period) {
                        return Err(NotTotp::Unreadable);
                    }
                }
                _ => {}
            }
        }
        let secret = secret.ok_or(NotTotp::Unreadable)?;
        let decoded = base32_decode(secret, false).ok_or(NotTotp::Unreadable)?;
        Ok(Totp {
            secret: Zeroizing::new(decoded),
            algorithm,
            digits,
            period,
        })
    } else {
        if clean_base32_key(field).len() < MIN_BARE_KEY_CHARS {
            return Err(NotTotp::Unreadable);
        }
        let decoded = base32_decode(field, true).ok_or(NotTotp::Unreadable)?;
        Ok(Totp {
            secret: Zeroizing::new(decoded),
            algorithm: Algorithm::Sha1,
            digits: 6,
            period: 30,
        })
    }
}

/// RFC 4226 dynamic truncation of an HMAC over `counter`, modulo
/// `10^digits`.
fn hotp(secret: &[u8], counter: u64, algorithm: Algorithm, digits: u32) -> u32 {
    let msg = counter.to_be_bytes();
    let hash: Vec<u8> = match algorithm {
        Algorithm::Sha1 => {
            let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("any key length");
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha512 => {
            let mut mac = Hmac::<Sha512>::new_from_slice(secret).expect("any key length");
            mac.update(&msg);
            mac.finalize().into_bytes().to_vec()
        }
    };
    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let bin = ((u32::from(hash[offset]) & 0x7f) << 24)
        | (u32::from(hash[offset + 1]) << 16)
        | (u32::from(hash[offset + 2]) << 8)
        | u32::from(hash[offset + 3]);
    bin % 10u32.pow(digits)
}

impl Totp {
    /// RFC 6238 with T0 = 0 (§4.2): the code at Unix time `unix`,
    /// zero-padded to its digit count.
    pub fn code(&self, unix: u64) -> String {
        let counter = unix / self.period;
        let value = hotp(&self.secret, counter, self.algorithm, self.digits);
        format!("{value:0width$}", width = self.digits as usize)
    }

    /// Seconds until the next code (§4.2).
    pub fn left(&self, unix: u64) -> u32 {
        (self.period - unix % self.period) as u32
    }

    /// The period a code is valid for, seconds: for a progress bar's
    /// fraction.
    pub fn period(&self) -> u64 {
        self.period
    }
}

/// `code` grouped for display (§4.2): six digits as `492 039`, seven as
/// `1 234 567`, eight as `1234 5678`.
pub fn grouped(code: &str) -> String {
    match code.len() {
        6 => format!("{} {}", &code[0..3], &code[3..6]),
        7 => format!("{} {} {}", &code[0..1], &code[1..4], &code[4..7]),
        8 => format!("{} {}", &code[0..4], &code[4..8]),
        _ => code.to_string(),
    }
}

/// What the entry form and the Inbox import store into field 6 (§4.3): a
/// URI [`parse`] reads, or reads as HOTP, unchanged (trimmed); a bare
/// Base32 key turned into a URI naming `title` and `account`; anything
/// else, `None`.
pub fn normalise(field: &str, title: &str, account: &str) -> Option<String> {
    let trimmed = field.trim();
    if trimmed.to_ascii_lowercase().starts_with("otpauth://") {
        return match parse(trimmed) {
            Ok(_) | Err(NotTotp::Hotp) => Some(trimmed.to_string()),
            Err(NotTotp::Unreadable) => None,
        };
    }
    parse(trimmed).ok()?;
    let key = clean_base32_key(trimmed);
    let mut uri = String::from("otpauth://totp/");
    uri.push_str(&percent_encode(title));
    if !account.is_empty() {
        uri.push(':');
        uri.push_str(&percent_encode(account));
    }
    uri.push_str("?secret=");
    uri.push_str(&key);
    uri.push_str("&issuer=");
    uri.push_str(&percent_encode(title));
    Some(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_key_decodes_as_sha1_6_digits_30_seconds() {
        let t = parse("JBSW Y3DP EHPK 3PXP").expect("reads");
        assert_eq!(t.code(0).len(), 6);
    }

    #[test]
    fn unknown_query_parameters_are_ignored() {
        let uri = "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP&issuer=X&image=https://x";
        assert!(parse(uri).is_ok());
    }

    #[test]
    fn grouping_matches_digit_count() {
        assert_eq!(grouped("492039"), "492 039");
        assert_eq!(grouped("1234567"), "1 234 567");
        assert_eq!(grouped("12345678"), "1234 5678");
    }
}
