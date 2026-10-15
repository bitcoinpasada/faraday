//! A certificate as PEM: one `CERTIFICATE` block, which is what an APK
//! signing key's public side is written as and what Files reads as a
//! public file (`docs/APK-SIGNING.md` §4.3).

use alloc::string::String;
use alloc::vec::Vec;

const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const END: &str = "-----END CERTIFICATE-----";
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A certificate's DER as a PEM block, lines of 64 characters.
pub fn encode(der: &[u8]) -> String {
    let mut b64 = String::new();
    for c in der.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                b64.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                b64.push('=');
            }
        }
    }
    let mut out = String::from(BEGIN);
    out.push('\n');
    for line in b64.as_bytes().chunks(64) {
        out.push_str(core::str::from_utf8(line).unwrap_or(""));
        out.push('\n');
    }
    out.push_str(END);
    out.push('\n');
    out
}

fn value(c: u8) -> Option<u32> {
    Some(match c {
        b'A'..=b'Z' => u32::from(c - b'A'),
        b'a'..=b'z' => u32::from(c - b'a') + 26,
        b'0'..=b'9' => u32::from(c - b'0') + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    })
}

/// The DER in a file that is exactly one PEM `CERTIFICATE` block and
/// nothing else but whitespace around it, and which reads as a
/// certificate. `None` for anything else.
pub fn read(file: &[u8]) -> Option<Vec<u8>> {
    let text = core::str::from_utf8(file).ok()?.trim();
    let body = text.strip_prefix(BEGIN)?.strip_suffix(END)?;
    let mut bits = 0u32;
    let mut n = 0;
    let mut pad = 0;
    let mut der = Vec::new();
    for c in body.bytes().filter(|c| !c.is_ascii_whitespace()) {
        if c == b'=' {
            pad += 1;
            continue;
        }
        if pad > 0 {
            return None;
        }
        bits = (bits << 6) | value(c)?;
        n += 1;
        if n == 4 {
            der.extend_from_slice(&[(bits >> 16) as u8, (bits >> 8) as u8, bits as u8]);
            bits = 0;
            n = 0;
        }
    }
    match (n, pad) {
        (0, 0) => {}
        (2, 2) => der.push((bits >> 4) as u8),
        (3, 1) => der.extend_from_slice(&[(bits >> 10) as u8, (bits >> 2) as u8]),
        _ => return None,
    }
    // It has to read as a certificate, and fill its own length.
    let (_, outer, next) = faraday_sb::der::read(&der, 0)?;
    (next == der.len() && faraday_sb::x509::Cert::read(&der).is_some() && outer.end <= der.len())
        .then_some(der)
}
