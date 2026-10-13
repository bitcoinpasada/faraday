//! What arrives, and whether it is a secret that stays on the device.
//!
//! Every scan, paste and file that reaches a Faraday app is read here
//! first: the seed a code holds ([`seedqr_words`], [`seed_in_code`]), the
//! secret a line of text is ([`secret_text`]), what a whole transfer
//! becomes ([`arrival`]), and the one question the Companion asks of
//! everything (`docs/COMPANION.md` §5): is this a secret ([`secret`])?
//!
//! Nothing here holds a word a person reads. Each answer is an enum, and
//! the app that asked says it in its own strings; the one exception is
//! [`wallet`], the description reader, whose refusals are Faraday's own
//! lines (`faraday-strings`).
//!
//! [`secret_buf`] holds text a person types that may be a secret, wiped
//! wherever it has been.

pub mod secret_buf;
pub mod wallet;

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::bitcoin::base58;
use osk_bip::codex32::Codex32;
use osk_bip::slip39;
use osk_codec::{PayloadKind, classify};
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

/// KeePass 1's signatures: the first the same, the second its own.
const KDB_SIGNATURE: [u8; 8] = [0x03, 0xD9, 0xA2, 0x9A, 0x65, 0xFB, 0x4B, 0xB5];

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

/// A secret that stays on the Faraday device, by kind
/// (`docs/COMPANION.md` §5's last row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Secret {
    /// A seed's words, 12 to 24 from one BIP-39 list, in any language,
    /// as a line or as a file of words.
    Bip39Words,
    /// A SeedQR's digits.
    SeedQr,
    /// A CompactSeedQR's 16 or 32 bytes.
    CompactSeedQr,
    /// A codex32 string (BIP 93): a seed or a share of one.
    Codex32,
    /// A SLIP-39 share.
    Slip39Share,
    /// An extended private key, under any version, on its own, with an
    /// origin, or inside a descriptor.
    ExtendedPrivateKey,
    /// A private key in wallet import format.
    Wif,
    /// `ur:crypto-seed` or `ur:seed`.
    UrSeed,
    /// `ur:crypto-bip39` or `ur:bip39`.
    UrBip39,
    /// A `crypto-hdkey` (or `hdkey`) that holds a private key, alone or
    /// inside an account or an output.
    UrPrivateKey,
    /// An encrypted backup (`OSKB`).
    EncryptedBackup,
    /// A KeePass database.
    KeePass,
    /// A Faraday vault.
    Vault,
    /// A threshold carry (`OSKC`): it holds secret nonces.
    ThresholdCarry,
    /// An `otpauth://` URI: a TOTP secret.
    Totp,
    /// An `otpauth-migration://` URI: an authenticator's export.
    TotpMigration,
    /// Entries for a vault, as [`looks_like_entries`] reads them.
    VaultEntries,
}

/// Which secret the bytes are, or `None` when they are none: what the
/// Companion refuses, whether it came by camera, paste or share. A
/// Faraday file envelope is opened and its file read the same way.
pub fn secret(bytes: &[u8]) -> Option<Secret> {
    secret_in(bytes, true)
}

/// [`secret`], opening an envelope only when `open` (an envelope is not
/// read inside another).
fn secret_in(bytes: &[u8], open: bool) -> Option<Secret> {
    if is_vault(bytes) {
        return Some(Secret::Vault);
    }
    if is_kdbx(bytes) || bytes.starts_with(&KDB_SIGNATURE) {
        return Some(Secret::KeePass);
    }
    match kind_of(bytes) {
        PayloadKind::EncryptedBackup => return Some(Secret::EncryptedBackup),
        PayloadKind::ThresholdCarry => return Some(Secret::ThresholdCarry),
        PayloadKind::SeedQr { .. } => return Some(Secret::SeedQr),
        PayloadKind::Codex32 => return Some(Secret::Codex32),
        // Sixteen or thirty-two bytes are entropy unless they are text a
        // person could have written.
        PayloadKind::CompactSeedQr { .. } if !is_text(bytes) => {
            return Some(Secret::CompactSeedQr);
        }
        _ => {}
    }
    let text = std::str::from_utf8(bytes).ok()?;
    if bip39_words(text) {
        return Some(Secret::Bip39Words);
    }
    let trimmed = text.trim();
    if open
        && faraday_qr::envelope::looks_like(trimmed.as_bytes())
        && let Ok((_, data, _)) = faraday_qr::envelope::unpack(trimmed.as_bytes())
    {
        return secret_in(&data, false);
    }
    for line in std::iter::once(trimmed).chain(text.lines().map(str::trim)) {
        if line.is_empty() {
            continue;
        }
        if let Some(s) = ur_secret(line) {
            return Some(s);
        }
        let scheme = line.get(..20).unwrap_or(line).to_ascii_lowercase();
        if scheme.starts_with("otpauth-migration://") {
            return Some(Secret::TotpMigration);
        }
        if scheme.starts_with("otpauth://") {
            return Some(Secret::Totp);
        }
        match part_of(line) {
            Some(Part::Slip39(_)) => return Some(Secret::Slip39Share),
            Some(Part::Codex32(_)) => return Some(Secret::Codex32),
            None => {}
        }
    }
    if looks_like_entries(text) {
        return Some(Secret::VaultEntries);
    }
    let words = file_words(text);
    if bip39_words(&words) || bip39_words(&Zeroizing::new(words.to_lowercase())) {
        return Some(Secret::Bip39Words);
    }
    key_in(text)
}

/// `osk_codec::classify`, except for text it cannot take. At upstream
/// commit `d26b84e` its silent payment check slices the text at bytes 4
/// and 5, and on the end of a `bitcoin:` URI, without asking whether
/// those are a character's bounds, so text whose first characters are
/// not ASCII (a Chinese seed, for one) stops the program there. No
/// payload it names but words begins that way, and words are read here
/// ([`bip39_words`]), so such text is `Unknown` without being classified.
fn kind_of(bytes: &[u8]) -> PayloadKind {
    if let Ok(t) = std::str::from_utf8(bytes.trim_ascii()) {
        let t = t.trim();
        let cut = |n: usize| t.is_char_boundary(n.min(t.len()));
        let uri = t
            .get(..8)
            .is_some_and(|p| p.eq_ignore_ascii_case("bitcoin:"));
        if !t.is_ascii() && (!cut(4) || !cut(5) || uri) {
            return PayloadKind::Unknown;
        }
    }
    classify(bytes)
}

/// Whether `text` is 12 to 24 words all from one BIP-39 list, in any
/// language. Copied from `osk-codec/src/classify.rs`'s `words_kind` at
/// upstream commit `d26b84e`, where it is private, with one addition: a
/// word also matches in its composed (NFC) form, as Japanese and Korean
/// words are usually written, which that function does not read.
fn bip39_words(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    matches!(words.len(), 12 | 15 | 18 | 21 | 24)
        && Language::ALL.into_iter().any(|lang| {
            words.iter().all(|w| {
                osk_codec::classify::word_index(lang, w).is_some()
                    || lang.words_display().contains(w)
            })
        })
}

/// Text a person could have written: UTF-8 with no control character
/// but line ends and tabs.
fn is_text(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|t| {
        t.chars()
            .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
    })
}

/// The secret a UR names by its type, or holds: `None` for a UR that
/// is none, and for text that is not a UR.
fn ur_secret(text: &str) -> Option<Secret> {
    if !osk_codec::ur::is_ur(text) {
        return None;
    }
    let ur_type = text[3..].split('/').next()?.to_ascii_lowercase();
    if let Some(s) = secret_type(&ur_type) {
        return Some(s);
    }
    match osk_codec::ur::decode_single(text) {
        Ok(osk_codec::ur::Message::Other { ur_type, cbor }) => secret_ur(&ur_type, &cbor),
        _ => None,
    }
}

/// A UR type that is a secret whatever it holds.
fn secret_type(ur_type: &str) -> Option<Secret> {
    match ur_type {
        "crypto-seed" | "seed" => Some(Secret::UrSeed),
        "crypto-bip39" | "bip39" => Some(Secret::UrBip39),
        _ => None,
    }
}

/// Which secret a whole UR is, by its type and its CBOR: for a UR put
/// together from its parts, which [`secret`] cannot see whole.
pub fn secret_ur(ur_type: &str, cbor: &[u8]) -> Option<Secret> {
    let ur_type = ur_type.to_ascii_lowercase();
    if let Some(s) = secret_type(&ur_type) {
        return Some(s);
    }
    let holds_keys = matches!(
        ur_type.as_str(),
        "crypto-hdkey"
            | "hdkey"
            | "crypto-account"
            | "account-descriptor"
            | "crypto-output"
            | "output-descriptor"
    );
    if !holds_keys {
        return None;
    }
    let v = faraday_qr::cbor::Value::decode(cbor).ok()?;
    let top = ur_type.ends_with("hdkey");
    private_key_in(&v, top).then_some(Secret::UrPrivateKey)
}

/// The registry's tag for an `hdkey` (BCR-2020-007).
const HDKEY: u64 = 303;

/// Whether `v` is, or holds, an `hdkey` with a private key in it:
/// `is-master` or `is-private` true, or key data of 33 bytes that begins
/// `00`. `hdkey` is true when `v` itself is one, untagged.
fn private_key_in(v: &faraday_qr::cbor::Value, hdkey: bool) -> bool {
    use faraday_qr::cbor::Value;
    let (tag, inner) = v.untag();
    if hdkey || tag == Some(HDKEY) {
        let flag = |k| matches!(inner.get(k), Some(Value::Bool(true)));
        let private_data =
            matches!(inner.get(3), Some(Value::Bytes(k)) if k.len() == 33 && k[0] == 0);
        if flag(1) || flag(2) || private_data {
            return true;
        }
    }
    match inner {
        Value::Array(items) => items.iter().any(|i| private_key_in(i, false)),
        Value::Map(pairs) => pairs.iter().any(|(_, i)| private_key_in(i, false)),
        Value::Tag(_, i) => private_key_in(i, false),
        _ => false,
    }
}

/// An extended private key or a WIF private key anywhere in `text`: each
/// run of base58 characters long enough to be one is read with its
/// checksum. An extended key is private when its key data begins `00`,
/// whatever version it is written under, so every SLIP-132 prefix and
/// any other is caught.
fn key_in(text: &str) -> Option<Secret> {
    let base58_char = |c: char| c.is_ascii_alphanumeric() && !matches!(c, '0' | 'O' | 'I' | 'l');
    text.split(|c: char| !base58_char(c))
        .filter(|t| (50..=115).contains(&t.len()))
        .find_map(|t| {
            let data = Zeroizing::new(base58::decode_check(t).ok()?);
            match data.len() {
                78 if data[45] == 0 => Some(Secret::ExtendedPrivateKey),
                33 | 34
                    if matches!(data[0], 0x80 | 0xEF) && (data.len() == 33 || data[33] == 1) =>
                {
                    Some(Secret::Wif)
                }
                _ => None,
            }
        })
}
