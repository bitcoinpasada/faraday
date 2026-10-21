//! What arrives, and whether it is a secret that stays on the device.
//!
//! Every scan, paste and file that reaches a Faraday app is read here
//! first: the seed a code holds ([`seedqr_words`], [`seed_in_code`]), the
//! secret a line of text is ([`secret_text`]), what a whole transfer
//! becomes ([`arrival`]).
//!
//! Nothing here holds a word a person reads. Each answer is an enum, and
//! the app that asked says it in its own strings; the one exception is
//! [`wallet`], the description reader, and the readers of other
//! wallets' files it calls ([`caravan`], [`electrum`], [`single_key`],
//! [`slip132`]), whose refusals are Faraday's own lines (`faraday-strings`);
//! the JSON those readers read is `osk_bip::json`. [`core_before_29`]
//! writes Bitcoin Core's import file for Core before 29.
//!
//! [`secret_buf`] holds text a person types that may be a secret, wiped
//! wherever it has been.

pub mod caravan;
pub mod core_before_29;
pub mod electrum;
pub mod secret_buf;
pub mod single_key;
pub mod slip132;
pub mod wallet;

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::codex32::Codex32;
use osk_bip::slip39;
use zeroize::Zeroizing;

/// The room a seed phrase is built in, bytes: more than 24 words of the
/// longest BIP-39 list, so a seed phrase never moves.
pub const ROOM: usize = 512;

/// An empty `Zeroizing<String>` with room for a whole seed phrase, for
/// building one a word at a time: a string that has to grow leaves its
/// old buffer behind unwiped.
pub fn room() -> Zeroizing<String> {
    Zeroizing::new(String::with_capacity(ROOM))
}

/// The words a SeedQR holds, standard (digits) or compact (entropy), in
/// English; `None` for any other code.
pub fn seedqr_words(payload: &[u8]) -> Option<Zeroizing<String>> {
    let lang = Language::English;
    let digits = payload.iter().all(u8::is_ascii_digit);
    let m = if digits && matches!(payload.len(), 48 | 96) {
        osk_codec::seedqr::from_digits(payload, lang).ok()?
    } else if matches!(payload.len(), 16 | 32) && std::str::from_utf8(payload).is_err() {
        osk_codec::seedqr::from_entropy(payload, lang).ok()?
    } else {
        return None;
    };
    let words = lang.words();
    let mut out = room();
    for (k, &i) in m.indices().iter().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(words[usize::from(i)]);
    }
    Some(out)
}

/// The seed a code holds: a SeedQR, a CompactSeedQR, or a seed's words in
/// English, as words; `None` for anything else.
pub fn seed_in_code(payload: &[u8]) -> Option<Zeroizing<String>> {
    if let Some(words) = seedqr_words(payload) {
        return Some(words);
    }
    let text = Zeroizing::new(String::from_utf8_lossy(payload).into_owned());
    let mut out = room();
    for (k, w) in text.split_whitespace().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(w);
    }
    let words = out.split(' ').count();
    (matches!(words, 12 | 15 | 18 | 21 | 24)
        && Mnemonic::parse(Language::English, out.as_str()).is_ok())
    .then_some(out)
}

/// The secret a line of scanned text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSecret {
    /// A seed's words: English, a valid checksum.
    Words,
    /// An extended private key, `xprv` or a SLIP-132 form of one.
    ExtendedPrivateKey,
}

/// Whether scanned text is a secret, and which: a seed's words or an
/// extended private key, with or without its origin.
pub fn secret_text(text: &str) -> Option<TextSecret> {
    let t = text.trim();
    let words = t.split_whitespace().count();
    if matches!(words, 12 | 15 | 18 | 21 | 24) && Mnemonic::parse(Language::English, t).is_ok() {
        return Some(TextSecret::Words);
    }
    let key = t.split(']').next_back().unwrap_or(t);
    if [
        "xprv", "tprv", "yprv", "zprv", "Yprv", "Zprv", "uprv", "vprv", "Uprv", "Vprv",
    ]
    .iter()
    .any(|p| key.starts_with(p))
    {
        return Some(TextSecret::ExtendedPrivateKey);
    }
    None
}

/// What a whole transfer becomes when it is kept: a name when it brought
/// one, an extension, and the bytes.
pub fn arrival(a: faraday_qr::Arrived) -> (Option<String>, &'static str, Vec<u8>) {
    use faraday_qr::Arrived;
    match a {
        Arrived::Psbt(p) => (None, "psbt", p),
        // A finished transaction is kept as hex, as one written here is.
        Arrived::Transaction(t) => (
            None,
            "txn",
            t.iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
                .into_bytes(),
        ),
        Arrived::File(name, data, _) => (Some(name), "", data),
        Arrived::Keys(t) | Arrived::Descriptor(t) | Arrived::Text(t) => {
            (None, "txt", t.into_bytes())
        }
    }
}

/// The words in a file of them, as a person or a backup tool writes one:
/// lines starting `#` are notes, and a number before a word (`1.`, `2)`,
/// `03`) is its place, not a word.
pub fn file_words(text: &str) -> Zeroizing<String> {
    let mut out = room();
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        for t in line.split_whitespace() {
            let place = t
                .trim_end_matches(['.', ')', ':'])
                .bytes()
                .all(|b| b.is_ascii_digit());
            if place {
                continue;
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(t);
        }
    }
    out
}

/// A text file's SLIP-39 share or codex32 string, read.
pub enum Part {
    /// A SLIP-39 share.
    Slip39(slip39::Share),
    /// A codex32 string.
    Codex32(Codex32),
}

/// Reads a text file as one part of a split seed.
pub fn part_of(text: &str) -> Option<Part> {
    let t = Zeroizing::new(text.trim().to_lowercase());
    if let Ok(s) = slip39::Share::parse(&t) {
        return Some(Part::Slip39(s));
    }
    Codex32::parse(&t).ok().map(Part::Codex32)
}

/// The first four bytes of every Faraday vault (`docs/VAULT.md`).
pub const VAULT_MAGIC: [u8; 4] = *b"OFVT";

/// Whether the bytes begin as a vault does. Says nothing of whether the
/// rest is sound.
pub fn is_vault(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == VAULT_MAGIC
}

/// KeePass's two signatures, little-endian, as KDBX 2 to 4 begin.
const KDBX_SIGNATURE: [u8; 8] = [0x03, 0xD9, 0xA2, 0x9A, 0x67, 0xFB, 0x4B, 0xB5];

/// Whether the bytes begin as a KeePass database (KDBX) does.
pub fn is_kdbx(bytes: &[u8]) -> bool {
    bytes.starts_with(&KDBX_SIGNATURE)
}

/// Whether text looks like entries for a vault: an `otpauth://` line, or
/// a `title:` line.
pub fn looks_like_entries(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .any(|l| l.starts_with("otpauth://") || l.to_ascii_lowercase().starts_with("title:"))
}
