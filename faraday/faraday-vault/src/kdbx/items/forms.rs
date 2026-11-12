//! The published text forms of the secret values an item holds
//! (`docs/VAULT.md` §7), each to the bytes of its record and back:
//! BIP-39 words of a list, a BIP-32 seed in hex, a PEM block, and the
//! small public forms beside them (a fingerprint, a GUID, a decimal).
//!
//! Every value is read by a parser the tree already has (`faraday-bip`'s
//! BIP-39 reader, `pem-rfc7468`) or is a fixed shape checked byte by byte;
//! every secret is in a wiping buffer. Listed in `tools/lint-secrets.sh`:
//! no heap text.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use alloc::vec::Vec;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_crypto::work::Compute;
use pem_rfc7468::LineEnding;
use zeroize::Zeroizing;

use crate::layouts::{self, Opened};

/// The `list` names of §7, every list but English, by its place in
/// [`Language::ALL`].
const LISTS: [(Language, &str); 9] = [
    (Language::Japanese, "japanese"),
    (Language::Korean, "korean"),
    (Language::Spanish, "spanish"),
    (Language::ChineseSimplified, "chinese-simplified"),
    (Language::ChineseTraditional, "chinese-traditional"),
    (Language::French, "french"),
    (Language::Italian, "italian"),
    (Language::Czech, "czech"),
    (Language::Portuguese, "portuguese"),
];

/// The list a `list` field names; `None` for a name §7 does not give
/// (English is the field's absence, not a name).
pub(super) fn list_of(name: &str) -> Option<Language> {
    LISTS.iter().find(|(_, n)| *n == name).map(|(l, _)| *l)
}

/// The `list` field a language is written with; `None` for English.
pub(super) fn list_name(lang: Language) -> Option<&'static str> {
    LISTS.iter().find(|(l, _)| *l == lang).map(|(_, n)| *n)
}

/// Costs a parse of the words: the words payload of a BIP-39 phrase of
/// `lang`, or `None` where it is not one.
pub(super) fn words_payload(
    c: &Compute,
    lang: Language,
    words: &str,
) -> Option<Zeroizing<Vec<u8>>> {
    Mnemonic::parse(c, lang, words)
        .ok()
        .map(|m| layouts::words_payload(&m))
}

/// The words a words payload holds, one space between, and their list.
pub(super) fn words_text(payload: &[u8]) -> Option<(Zeroizing<Vec<u8>>, Language)> {
    let Some(Opened::Words(m)) = layouts::payload_of(layouts::KIND_WORDS, payload) else {
        return None;
    };
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    out.try_reserve_exact(24 * 40).ok()?;
    for (i, idx) in m.indices().iter().enumerate() {
        if i > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(m.language().word(*idx).as_bytes());
    }
    Some((out, m.language()))
}

/// The seed payload of a `hex` field: 16 to 64 bytes in lower-case hex.
pub(super) fn seed_payload(text: &str) -> Option<Zeroizing<Vec<u8>>> {
    let seed = unhex(text)?;
    layouts::seed_payload(&seed)
}

/// A seed payload's seed in lower-case hex.
pub(super) fn seed_hex(payload: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let Some(Opened::Seed(seed)) = layouts::payload_of(layouts::KIND_SEED, payload) else {
        return None;
    };
    Some(hex(&seed))
}

/// Bytes as lower-case hex.
pub(super) fn hex(bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; bytes.len() * 2]);
    faraday_crypto::hex::encode_lower(bytes, &mut out);
    out
}

/// Lower-case hex as bytes; `None` for an odd length or another
/// character.
pub(super) fn unhex(text: &str) -> Option<Zeroizing<Vec<u8>>> {
    if text.bytes().any(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; text.len() / 2]);
    (faraday_crypto::hex::decode_into(text, &mut out)? == out.len()).then_some(out)
}

/// A key's fingerprint: 8 lower-case hex digits.
pub(super) fn fingerprint(text: &str) -> Option<[u8; 4]> {
    let b = unhex(text)?;
    <[u8; 4]>::try_from(&b[..]).ok()
}

/// A positive decimal integer without a leading zero, at most `max`.
pub(super) fn decimal(text: &str, max: u64) -> Option<u64> {
    let b = text.as_bytes();
    if b.is_empty() || b.len() > 20 || b[0] == b'0' || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    text.parse::<u64>().ok().filter(|&n| n <= max)
}

/// A decimal integer, `0` or one without a leading zero.
pub(super) fn amount(text: &str) -> Option<u64> {
    if text == "0" {
        return Some(0);
    }
    decimal(text, u64::MAX)
}

/// An RFC 3339 date-time in UTC to the second, as Faraday writes one:
/// `2026-10-28T14:02:00Z`.
pub(super) fn is_utc_time(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |r: core::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    b.len() == 20
        && digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19)
        && b[19] == b'Z'
}

/// A GUID's 16 bytes from its text form (`8-4-4-4-12` hex digits, the
/// first three groups little-endian, as UEFI and Faraday's Secure Boot
/// screen write it); either case is read.
pub(super) fn guid(text: &str) -> Option<[u8; 16]> {
    let b = text.as_bytes();
    if b.len() != 36 || [8, 13, 18, 23].iter().any(|&i| b[i] != b'-') {
        return None;
    }
    let mut plain = [0u8; 32];
    for (n, &c) in b.iter().filter(|&&c| c != b'-').enumerate() {
        plain[n] = c.to_ascii_lowercase();
    }
    let raw = unhex(core::str::from_utf8(&plain).ok()?)?;
    let mut out = [0u8; 16];
    out.copy_from_slice(&raw);
    out[0..4].reverse();
    out[4..6].reverse();
    out[6..8].reverse();
    Some(out)
}

/// A GUID's text form, lower case.
pub(super) fn guid_text(g: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let mut g: [u8; 16] = g.try_into().ok()?;
    g[0..4].reverse();
    g[4..6].reverse();
    g[6..8].reverse();
    let h = hex(&g);
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(36));
    for (i, c) in h.iter().enumerate() {
        if [8, 12, 16, 20].contains(&i) {
            out.push(b'-');
        }
        out.push(*c);
    }
    Some(out)
}

/// The DER bytes of one PEM block (RFC 7468) whose label is `label`;
/// `None` for another label, damage, or text after the block.
pub(super) fn pem_der(text: &str, label: &str) -> Option<Zeroizing<Vec<u8>>> {
    let mut buf: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; text.len()]);
    let (got, der) = pem_rfc7468::decode(text.as_bytes(), &mut buf).ok()?;
    if got != label || der.is_empty() {
        return None;
    }
    let len = der.len();
    buf.truncate(len);
    Some(buf)
}

/// DER bytes as one PEM block with this label, lines of 64 and a line
/// feed after each.
pub(super) fn der_pem(der: &[u8], label: &str) -> Option<Zeroizing<Vec<u8>>> {
    let len = pem_rfc7468::encoded_len(label, LineEnding::LF, der).ok()?;
    let mut out: Zeroizing<Vec<u8>> = Zeroizing::new(alloc::vec![0u8; len]);
    let n = pem_rfc7468::encode(label, LineEnding::LF, der, &mut out)
        .ok()?
        .len();
    out.truncate(n);
    Some(out)
}

/// Whether `text` is one ASCII-armored OpenPGP block (RFC 9580 §6.2) of
/// this kind (`PRIVATE KEY BLOCK`, `PUBLIC KEY BLOCK`), by its armor
/// lines; what the packets hold is not read here.
pub(super) fn is_armor(text: &str, kind: &str) -> bool {
    let body = text.trim_end_matches(['\n', '\r']);
    let mut lines = body.lines();
    let first = lines.next().unwrap_or("");
    let last = body.lines().next_back().unwrap_or("");
    let names = |line: &str, start: &str| {
        line.strip_prefix(start)
            .and_then(|r| r.strip_suffix("-----"))
            .is_some_and(|k| k == kind)
    };
    body.lines().count() >= 3 && names(first, "-----BEGIN PGP ") && names(last, "-----END PGP ")
}
