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
//! [`specter_diy`]),
//! whose refusals are Faraday's own lines (`faraday-strings`), a SLIP-132
//! key's among them (`faraday_bip::slip132` reads the key); the JSON those
//! readers read is `faraday_bip::json`.
//!
//! [`secret_buf`] holds text a person types that may be a secret, wiped
//! wherever it has been.

pub mod caravan;
pub mod electrum;
mod names;
pub mod secret_buf;
pub mod single_key;
pub mod specter_diy;
pub mod wallet;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::slip39;
use faraday_crypto::work::Compute;
use faraday_text::Text;
use zeroize::Zeroizing;

/// The room a seed phrase is built in, bytes: more than 24 words of the
/// longest BIP-39 list, so a seed phrase never moves.
pub const ROOM: usize = 512;

/// An empty [`Text`] with room for a whole seed phrase, for building one
/// a word at a time: a buffer that has to grow is wiped as it moves, but
/// with room it never moves.
pub fn room() -> Text {
    Text::with_room(ROOM)
}

/// The words a SeedQR holds, standard (digits) or compact (entropy), in
/// English; `None` for any other code.
#[expect(
    clippy::disallowed_methods,
    reason = "reads text as it arrives and keeps none of it (docs/KEYS.md section 2): faraday-keys depends on this crate"
)]
pub fn seedqr_words(payload: &[u8]) -> Option<Text> {
    let lang = Language::English;
    let digits = payload.iter().all(u8::is_ascii_digit);
    let m = if digits && matches!(payload.len(), 48 | 96) {
        faraday_codec::seedqr::from_digits(payload, lang).ok()?
    } else if faraday_codec::classify::is_compact_seedqr(payload) {
        faraday_codec::seedqr::from_entropy(payload, lang).ok()?
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

/// Costs what `parse` costs. The seed a code holds: a SeedQR, a
/// CompactSeedQR, or a seed's words in English, as words; `None` for
/// anything else.
#[expect(
    clippy::disallowed_methods,
    reason = "reads text as it arrives and keeps none of it (docs/KEYS.md section 2): faraday-keys depends on this crate"
)]
pub fn seed_in_code(c: &Compute, payload: &[u8]) -> Option<Text> {
    if let Some(words) = seedqr_words(payload) {
        return Some(words);
    }
    let text = Text::from_utf8_lossy(payload);
    let mut out = room();
    for (k, w) in text.split_whitespace().enumerate() {
        if k > 0 {
            out.push(' ');
        }
        out.push_str(w);
    }
    let words = out.split(' ').count();
    (matches!(words, 12 | 15 | 18 | 21 | 24)
        && Mnemonic::parse(c, Language::English, out.as_str()).is_ok())
    .then_some(out)
}

/// The secret a line of scanned text is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSecret {
    /// A seed's words: English, a valid checksum.
    Words,
    /// An extended private key, `xprv` or a SLIP-132 form of one.
    ExtendedPrivateKey,
    /// BIP-392's silent payments spend private key, `spspend1q…` or
    /// `tspspend1q…`, alone or in an `sp()` descriptor.
    SilentSpendKey,
    /// A WIF private key.
    Wif,
}

/// Costs a base58 checksum when the text has the shape of a WIF (51 or
/// 52 base58 characters, a version's first character), else nothing.
/// Whether `text` is a WIF private key, mainnet or test.
fn is_wif_key(text: &str) -> bool {
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let b = text.as_bytes();
    matches!(b.len(), 51 | 52)
        && matches!(b[0], b'5' | b'K' | b'L' | b'9' | b'c')
        && b.iter().all(|c| BASE58.contains(c))
        && faraday_bip::bitcoin::base58::decode_check(text).is_ok_and(|p| {
            let p = Zeroizing::new(p);
            matches!(p[0], 0x80 | 0xef) && (p.len() == 33 || (p.len() == 34 && p[33] == 1))
        })
}

/// Whether `text` holds BIP-392's silent payments spend private key
/// (`spspend1`, `tspspend1` contains it), in any case, compared without
/// a copy.
pub(crate) fn holds_spend_key(text: &str) -> bool {
    text.as_bytes()
        .windows(8)
        .any(|w| w.eq_ignore_ascii_case(b"spspend1"))
}

/// The versions of an extended private key, SLIP-132's included.
const PRIVATE_PREFIXES: [&str; 10] = [
    "xprv", "tprv", "yprv", "zprv", "Yprv", "Zprv", "uprv", "vprv", "Uprv", "Vprv",
];

/// Whether `text` starts with the version of an extended private key,
/// the 4 bytes `secret_text` names it by. Costs 10 comparisons of 4 bytes.
pub fn starts_with_private_version(text: &str) -> bool {
    PRIVATE_PREFIXES.iter().any(|p| text.starts_with(p))
}

/// Costs one scan of `text`: at most 111 bytes compared at each position.
/// Whether `text` holds an extended private key anywhere in it, in a
/// descriptor with no origin too: a private version followed by the
/// base58 characters of a whole key (111 in all).
fn holds_extended_private_key(text: &str) -> bool {
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    text.as_bytes().windows(111).any(|w| {
        PRIVATE_PREFIXES.iter().any(|p| w.starts_with(p.as_bytes()))
            && w[4..].iter().all(|c| BASE58.contains(c))
    })
}

/// The lists other than English that hold `word`, as (the list's place in
/// `Language::ALL`, the word's index), the word read as the list spells it
/// (decomposed, or composed as it is read) and in any case. Costs one hash
/// lookup for all lists at once (the index is built once, on first use,
/// from the published lists). Empty for a word on no list, and for one
/// longer than any listed word.
pub fn other_list_entries(word: &str) -> &'static [(u8, u16)] {
    use std::collections::HashMap;
    use std::hash::{BuildHasherDefault, Hasher};
    use std::sync::OnceLock;
    /// FNV-1a: the keys are the published lists' words, fixed, and a
    /// lookup is made for every word of a text read, so it is kept short.
    #[derive(Default)]
    struct Fnv(u64);
    impl Hasher for Fnv {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            let mut h = if self.0 == 0 {
                0xcbf2_9ce4_8422_2325
            } else {
                self.0
            };
            for &b in bytes {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
            self.0 = h;
        }
    }
    type Index = HashMap<&'static str, Vec<(u8, u16)>, BuildHasherDefault<Fnv>>;
    static INDEX: OnceLock<Index> = OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let mut m = Index::with_capacity_and_hasher(32768, Default::default());
        for (at, &l) in Language::ALL.iter().enumerate() {
            if l == Language::English {
                continue;
            }
            for i in 0..2048u16 {
                for w in [l.word_nfkd(i), l.word_display(i)] {
                    let e = m.entry(w).or_default();
                    if !e.contains(&(at as u8, i)) {
                        e.push((at as u8, i));
                    }
                }
            }
        }
        m
    });
    // The word in lower case, in a buffer on the stack.
    let mut key = [0u8; 64];
    let mut n = 0;
    if word.len() <= key.len() && word.is_ascii() {
        n = word.len();
        key[..n].copy_from_slice(word.as_bytes());
        key[..n].make_ascii_lowercase();
    } else {
        for c in word.chars().flat_map(char::to_lowercase) {
            if n + c.len_utf8() > key.len() {
                return &[];
            }
            n += c.encode_utf8(&mut key[n..]).len();
        }
    }
    match std::str::from_utf8(&key[..n]) {
        Ok(k) => index.get(k).map_or(&[], Vec::as_slice),
        Err(_) => &[],
    }
}

/// Costs a parse in English and in each other list that holds at least
/// half of the `words` words of `t`. Whether `t` is a mnemonic in any list.
#[expect(
    clippy::disallowed_methods,
    reason = "reads text as it arrives and keeps none of it (docs/KEYS.md section 2): faraday-keys depends on this crate"
)]
fn words_in_any_list(c: &Compute, t: &str, words: usize) -> bool {
    if Mnemonic::parse(c, Language::English, t).is_ok() {
        return true;
    }
    let mut on_list = vec![0usize; Language::ALL.len()];
    for w in t.split_whitespace() {
        for e in other_list_entries(w) {
            on_list[e.0 as usize] += 1;
        }
    }
    Language::ALL
        .iter()
        .enumerate()
        .filter(|&(at, &l)| l != Language::English && on_list[at] * 2 >= words)
        .any(|(_, &l)| Mnemonic::parse(c, l, t).is_ok())
}

/// Costs what `parse` costs in English and in each list that holds half the words, plus one scan of the text for an extended
/// private key: at most 111 bytes compared at each position, so linear in
/// the text's length (by its loop, not timed). Whether scanned text is a secret, and
/// which: a seed's words, an extended private key, with or without its
/// origin, a silent payments spend key, or a WIF key (a base58 checksum).
pub fn secret_text(c: &Compute, text: &str) -> Option<TextSecret> {
    let t = text.trim();
    let words = t.split_whitespace().count();
    if matches!(words, 12 | 15 | 18 | 21 | 24) && words_in_any_list(c, t, words) {
        return Some(TextSecret::Words);
    }
    let key = t.split(']').next_back().unwrap_or(t);
    if PRIVATE_PREFIXES.iter().any(|p| key.starts_with(p)) || holds_extended_private_key(t) {
        return Some(TextSecret::ExtendedPrivateKey);
    }
    if holds_spend_key(t) {
        return Some(TextSecret::SilentSpendKey);
    }
    is_wif_key(t).then_some(TextSecret::Wif)
}

/// A whole transfer as it is kept: a name when it brought one, an
/// extension, and the bytes, in room that is wiped when dropped.
pub type Kept = (Option<Text>, &'static str, Zeroizing<Vec<u8>>);

/// What a whole transfer becomes when it is kept: a name when it brought
/// one, an extension, and the bytes, in room that is wiped when dropped
/// (a seed's words and an extended private key are among them).
pub fn arrival(a: faraday_qr::Arrived) -> Kept {
    use faraday_qr::Arrived;
    match a {
        Arrived::Psbt(p) => (None, "psbt", Zeroizing::new(p)),
        // A finished transaction is kept as hex, as one written here is,
        // written straight into wiping room.
        Arrived::Transaction(t) => {
            let mut hex = Zeroizing::new(vec![0u8; t.len() * 2]);
            faraday_crypto::hex::encode_lower(&t, &mut hex);
            (None, "txn", hex)
        }
        Arrived::File(name, data, _) => (Some(Text::of(&name)), "", Zeroizing::new(data)),
        Arrived::Keys(t) | Arrived::Descriptor(t) | Arrived::Text(t) => {
            (None, "txt", Zeroizing::new(t.into_bytes()))
        }
    }
}

/// The words in a file of them, as a person or a backup tool writes one:
/// lines starting `#` are notes, and a number before a word (`1.`, `2)`,
/// `03`) is its place, not a word.
pub fn file_words(text: &str) -> Text {
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
    let t = faraday_text::Text::lowercase(text.trim());
    if let Ok(s) = slip39::Share::parse(&t) {
        return Some(Part::Slip39(s));
    }
    // As given, not lowercased: BIP 93 refuses a string in mixed case.
    Codex32::parse(text.trim()).ok().map(Part::Codex32)
}

/// Whether the bytes begin as a KeePass database (KDBX) does but are not
/// a vault: a file of another version, or one of a length no vault has.
pub fn is_kdbx(bytes: &[u8]) -> bool {
    bytes.starts_with(&faraday_codec::vault_file::SIGNATURE)
        && !faraday_codec::vault_file::is_vault_file(bytes)
}

/// Whether the bytes begin as a KeePass database of version 3 or older,
/// which Faraday names and does not open.
pub fn is_older_kdbx(bytes: &[u8]) -> bool {
    faraday_codec::vault_file::is_older_kdbx(bytes)
}

/// Whether a line begins with the `otpauth://` scheme, in either case: the
/// spelling the TOTP reader accepts and an entry stores.
pub fn is_otpauth(line: &str) -> bool {
    line.get(..OTPAUTH.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(OTPAUTH))
}

const OTPAUTH: &str = "otpauth://";
const OTPAUTH_MIGRATION: &str = "otpauth-migration://";

/// Whether text begins with Google Authenticator's export scheme,
/// `otpauth-migration://`, in either case: it holds every TOTP secret of
/// the account. No more is read than the scheme.
pub fn is_otpauth_migration(text: &str) -> bool {
    text.trim()
        .get(..OTPAUTH_MIGRATION.len())
        .is_some_and(|p| p.eq_ignore_ascii_case(OTPAUTH_MIGRATION))
}
