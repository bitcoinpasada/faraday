//! Reading and computing TOTP codes (`docs/TOTP.md` §4) from a vault
//! entry's field 6: an `otpauth://totp/…` URI, or a bare Base32 key as
//! services show when a setup QR cannot be scanned.
//!
//! [`from_fields`] reads an entry's TOTP secret from whichever field
//! forms it holds; [`parse`] reads field 6; [`Totp::code`] and [`Totp::left`] are RFC
//! 6238 over a Unix time; [`normalise`] is what the entry form and the
//! Inbox import (`faraday-vault::entries`) store back into field 6.

use alloc::vec::Vec;

use faraday_text::{Text, text};
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

impl Totp {
    /// What the secret takes beyond the struct: its buffer, with an
    /// allocation's overhead.
    pub(crate) fn held(&self) -> usize {
        self.secret.capacity() + 16
    }
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
        if !c.is_ascii() {
            return None;
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
fn clean_base32_key(s: &str) -> Text {
    // Sized for the whole of `s`, so the key never outgrows its buffer and
    // leaves a block behind.
    let mut key = Text::with_room(s.len());
    for c in s.chars() {
        if !c.is_whitespace() && c != '-' && c != '=' {
            key.push(c.to_ascii_uppercase());
        }
    }
    key
}

/// Whether `s` starts with the `otpauth://` scheme, in any case.
fn has_otpauth_scheme(s: &str) -> bool {
    s.get(.."otpauth://".len())
        .is_some_and(|p| p.eq_ignore_ascii_case("otpauth://"))
}

/// Percent-encodes everything outside RFC 3986's unreserved characters,
/// byte by byte (`docs/TOTP.md` §4.3).
fn percent_encode(s: &str) -> Text {
    let mut out = Text::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&text!("%{b:02X}"));
        }
    }
    out
}

/// The query parameters of the key URI format; one written twice is refused.
const KNOWN_PARAMETERS: [&str; 8] = [
    "secret",
    "issuer",
    "algorithm",
    "digits",
    "period",
    "counter",
    "image",
    "encoder",
];

/// A query parameter's name as a URI parser reads it: percent escapes
/// decoded (again while one is left, as a name escaped twice is) and
/// lowercased, for comparing it with the names of the format.
#[expect(
    clippy::disallowed_methods,
    reason = "the name of a parameter of an otpauth address, which is not secret"
)]
fn parameter_name(raw: &str) -> Text {
    let mut name = raw.as_bytes().to_vec();
    for _ in 0..4 {
        let mut out = Vec::with_capacity(name.len());
        let mut i = 0;
        let mut changed = false;
        while i < name.len() {
            let hex = |b: u8| (b as char).to_digit(16);
            if name[i] == b'%'
                && i + 2 < name.len()
                && let (Some(h), Some(l)) = (hex(name[i + 1]), hex(name[i + 2]))
            {
                out.push((h * 16 + l) as u8);
                i += 3;
                changed = true;
            } else {
                out.push(name[i]);
                i += 1;
            }
        }
        name = out;
        if !changed {
            break;
        }
    }
    Text::from_utf8_lossy(&name)
        .chars()
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// `parse(field)` from a type 6 field 6 value (§4.1).
pub fn parse(field: &str) -> Result<Totp, NotTotp> {
    let field = field.trim();
    if has_otpauth_scheme(field) {
        let rest = &field["otpauth://".len()..];
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
        let mut seen: Vec<Text> = Vec::new();
        for pair in query.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (k, v) = pair.split_once('=').ok_or(NotTotp::Unreadable)?;
            // A parameter of the key URI format is read only as written in
            // lower case: its name in another case or percent-encoded is
            // refused, as is one written twice, since a parser that decodes
            // the name reads another value than the one used here. A name
            // with a space or a control character in it is refused too: a
            // parser may strip it and read a parameter.
            let name = parameter_name(k);
            if name.contains(|c: char| c.is_ascii_whitespace() || c.is_ascii_control()) {
                return Err(NotTotp::Unreadable);
            }
            if KNOWN_PARAMETERS.contains(&name.as_str()) {
                if name != k || seen.contains(&name) {
                    return Err(NotTotp::Unreadable);
                }
                seen.push(name);
            }
            match k {
                "secret" => secret = Some(v),
                "algorithm" => {
                    algorithm = if v.eq_ignore_ascii_case("SHA1") {
                        Algorithm::Sha1
                    } else if v.eq_ignore_ascii_case("SHA256") {
                        Algorithm::Sha256
                    } else if v.eq_ignore_ascii_case("SHA512") {
                        Algorithm::Sha512
                    } else {
                        return Err(NotTotp::Unreadable);
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
                // Steam's codes are five characters of its own alphabet: a
                // plain code from its key is one Steam will not take.
                "encoder" if v.eq_ignore_ascii_case("steam") => return Err(NotTotp::Unreadable),
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

/// The field value named `name` in `fields`: the first with that name
/// (names are case-sensitive, as KeePass's are), absent when its value
/// is empty.
fn field<'a>(fields: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .filter(|v| !v.trim().is_empty())
}

/// A secret field's value: the first with that name, absent only when it
/// is empty (KeePass takes the value as it is, spaces included).
fn secret_field<'a>(fields: &[(&str, &'a str)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| *v)
        .filter(|v| !v.is_empty())
}

/// Hex decodes `s` (either case); an odd length, any other character or
/// nothing at all refuses.
fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    faraday_crypto::hex::decode_vec(s)
}

/// Reads KeePass 2's own `TimeOtp-*` fields (`docs/TOTP.md` §4.1). The
/// secret is the first of `TimeOtp-Secret` (UTF-8, as it is), `-Secret-Hex`,
/// `-Secret-Base32` and `-Secret-Base64` that is not empty, the order
/// KeePass's help lists them in (it says exactly one is present and names
/// no other order); `None` when none is. A value of spaces alone is a
/// secret in the plain form and reads as nothing in the others.
fn keepass_time_otp(fields: &[(&str, &str)]) -> Option<Result<Totp, NotTotp>> {
    let secret: Option<Zeroizing<Vec<u8>>> = if let Some(v) = secret_field(fields, "TimeOtp-Secret")
    {
        Some(crate::wiped_copy(v.as_bytes()))
    } else if let Some(v) = secret_field(fields, "TimeOtp-Secret-Hex") {
        hex_decode(v).map(Zeroizing::new)
    } else if let Some(v) = secret_field(fields, "TimeOtp-Secret-Base32") {
        base32_decode(v, false).map(Zeroizing::new)
    } else {
        let v = secret_field(fields, "TimeOtp-Secret-Base64")?;
        use base64ct::{Base64, Encoding};
        Base64::decode_vec(v.trim())
            .ok()
            .filter(|b| !b.is_empty())
            .map(Zeroizing::new)
    };
    Some(keepass_time_otp_params(fields, secret))
}

fn keepass_time_otp_params(
    fields: &[(&str, &str)],
    secret: Option<Zeroizing<Vec<u8>>>,
) -> Result<Totp, NotTotp> {
    let secret = secret.ok_or(NotTotp::Unreadable)?;
    let digits: u32 = match field(fields, "TimeOtp-Length") {
        None => 6,
        Some(v) => v.trim().parse().map_err(|_| NotTotp::Unreadable)?,
    };
    let period: u64 = match field(fields, "TimeOtp-Period") {
        None => 30,
        Some(v) => v.trim().parse().map_err(|_| NotTotp::Unreadable)?,
    };
    let algorithm = match field(fields, "TimeOtp-Algorithm").map(str::trim) {
        None | Some("HMAC-SHA-1") => Algorithm::Sha1,
        Some("HMAC-SHA-256") => Algorithm::Sha256,
        Some("HMAC-SHA-512") => Algorithm::Sha512,
        Some(_) => return Err(NotTotp::Unreadable),
    };
    if !(6..=8).contains(&digits) || !(1..=3600).contains(&period) {
        return Err(NotTotp::Unreadable);
    }
    Ok(Totp {
        secret,
        algorithm,
        digits,
        period,
    })
}

/// Reads older KeePassXC's `TOTP Seed` (Base32) with `TOTP Settings`
/// (`period;digits`, `30;6` when absent or when it is one part; a third part
/// and after are ignored; a Steam `S` for digits is unreadable) (`docs/TOTP.md` §4.1).
fn keepassxc_seed(fields: &[(&str, &str)]) -> Option<Result<Totp, NotTotp>> {
    let seed = secret_field(fields, "TOTP Seed")?;
    Some(keepassxc_seed_params(fields, seed))
}

fn keepassxc_seed_params(fields: &[(&str, &str)], seed: &str) -> Result<Totp, NotTotp> {
    let secret = base32_decode(seed, false).ok_or(NotTotp::Unreadable)?;
    let (mut period, mut digits) = (30u64, 6u32);
    if let Some(settings) = field(fields, "TOTP Settings") {
        // KeePassXC (src/core/Totp.cpp, parseSettings) splits the value at
        // `;` and, when it has two parts or more (`vars.size() >= 2`),
        // reads the first as the period and the second as the digits; a
        // period alone leaves both at their defaults.
        let mut parts = settings.trim().split(';');
        if let (Some(p), Some(d)) = (parts.next(), parts.next()) {
            period = p.trim().parse().map_err(|_| NotTotp::Unreadable)?;
            digits = d.trim().parse().map_err(|_| NotTotp::Unreadable)?;
        }
    }
    if !(6..=8).contains(&digits) || !(1..=3600).contains(&period) {
        return Err(NotTotp::Unreadable);
    }
    Ok(Totp {
        secret: Zeroizing::new(secret),
        algorithm: Algorithm::Sha1,
        digits,
        period,
    })
}

/// The TOTP secret of an entry, read for display from its `fields` (name
/// and value pairs, `docs/TOTP.md` §4.1): `otp` (the Key URI Format),
/// else KeePass 2's `TimeOtp-*` fields, else older KeePassXC's `TOTP
/// Seed` and `TOTP Settings`; the first form the entry holds decides,
/// whether it reads or not. `None` when the entry holds none of them;
/// `Err` for HOTP (`otpauth://hotp`, KeePass 2's `HmacOtp-*` fields) or a
/// form that does not read. Reading changes nothing in the entry.
pub fn from_fields(fields: &[(&str, &str)]) -> Option<Result<Totp, NotTotp>> {
    if let Some(v) = field(fields, "otp") {
        return Some(parse(v));
    }
    if let Some(r) = keepass_time_otp(fields) {
        return Some(r);
    }
    if let Some(r) = keepassxc_seed(fields) {
        return Some(r);
    }
    field(fields, "HmacOtp-Secret")
        .or_else(|| field(fields, "HmacOtp-Secret-Base32"))
        .or_else(|| field(fields, "HmacOtp-Secret-Hex"))
        .or_else(|| field(fields, "HmacOtp-Secret-Base64"))
        .map(|_| Err(NotTotp::Hotp))
}

/// RFC 4226 dynamic truncation of an HMAC over `counter`, modulo
/// `10^digits`.
fn hotp(secret: &[u8], counter: u64, algorithm: Algorithm, digits: u32) -> u32 {
    let msg = counter.to_be_bytes();
    let hash: Zeroizing<Vec<u8>> = match algorithm {
        Algorithm::Sha1 => {
            let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("any key length");
            mac.update(&msg);
            crate::wiped_copy(&mac.finalize().into_bytes())
        }
        Algorithm::Sha256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("any key length");
            mac.update(&msg);
            crate::wiped_copy(&mac.finalize().into_bytes())
        }
        Algorithm::Sha512 => {
            let mut mac = Hmac::<Sha512>::new_from_slice(secret).expect("any key length");
            mac.update(&msg);
            crate::wiped_copy(&mac.finalize().into_bytes())
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
    pub fn code(&self, unix: u64) -> Text {
        let counter = unix / self.period;
        let value = hotp(&self.secret, counter, self.algorithm, self.digits);
        text!("{value:0width$}", width = self.digits as usize)
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
pub fn grouped(code: &str) -> Text {
    match code.len() {
        6 => text!("{} {}", &code[0..3], &code[3..6]),
        7 => text!("{} {} {}", &code[0..1], &code[1..4], &code[4..7]),
        8 => text!("{} {}", &code[0..4], &code[4..8]),
        _ => Text::of(code),
    }
}

/// What the entry form and the Inbox import store into field 6 (§4.3): a
/// URI [`parse`] reads, or reads as HOTP, unchanged (trimmed); a bare
/// Base32 key turned into a URI naming `title` and `account`; anything
/// else, `None`.
///
/// The result is wiping text sized up front, so the secret is never
/// copied into a larger buffer on the way.
pub fn normalise(field: &str, title: &str, account: &str) -> Option<Text> {
    let trimmed = field.trim();
    if has_otpauth_scheme(trimmed) {
        return match parse(trimmed) {
            Ok(_) | Err(NotTotp::Hotp) => {
                let mut uri = Text::with_room(trimmed.len());
                uri.push_str(trimmed);
                Some(uri)
            }
            Err(NotTotp::Unreadable) => None,
        };
    }
    parse(trimmed).ok()?;
    let key = clean_base32_key(trimmed);
    let title = percent_encode(title);
    let account = percent_encode(account);
    let mut uri = Text::with_room(
        "otpauth://totp/".len()
            + title.len() * 2
            + account.len()
            + 1
            + "?secret=".len()
            + key.len()
            + "&issuer=".len(),
    );
    uri.push_str("otpauth://totp/");
    uri.push_str(&title);
    if !account.is_empty() {
        uri.push(':');
        uri.push_str(&account);
    }
    uri.push_str("?secret=");
    uri.push_str(&key);
    uri.push_str("&issuer=");
    uri.push_str(&title);
    Some(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_query_parameters_are_ignored() {
        let uri = "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP&issuer=X&image=https://x";
        assert!(parse(uri).is_ok());
    }

    #[test]
    fn a_non_ascii_character_is_not_base32() {
        // U+0141 truncates to 'A' as a byte.
        assert!(base32_decode("JBSWY3DPEHPK3PX\u{141}", false).is_none());
        assert!(parse("otpauth://totp/x?secret=JBSWY3DPEHPK3PX\u{141}").is_err());
    }
}
