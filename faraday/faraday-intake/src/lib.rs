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

/// The lists a word is on, as [`other_list_entries`] gives them: at most
/// `Entries::ROOM`, the spelled ones first.
pub struct Entries {
    n: u8,
    at: [(u8, u16); Entries::ROOM],
}

impl Entries {
    const ROOM: usize = 4;

    fn push(&mut self, e: (u8, u16)) {
        if usize::from(self.n) < Self::ROOM && !self.contains(&e) {
            self.at[usize::from(self.n)] = e;
            self.n += 1;
        }
    }
}

impl std::ops::Deref for Entries {
    type Target = [(u8, u16)];
    fn deref(&self) -> &[(u8, u16)] {
        &self.at[..usize::from(self.n)]
    }
}

/// An NFKD word of a Latin-script list without its combining marks
/// (U+0300 to U+036F), in lower case, in `out`: its length, or `None`
/// for a word with no mark, or that is not Latin letters and marks alone,
/// or does not fit.
fn unaccented(word: &str, out: &mut [u8; 24]) -> Option<usize> {
    if word.is_ascii() {
        return None;
    }
    let (mut n, mut marked) = (0, false);
    for c in word.chars() {
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            marked = true;
        } else if c.is_ascii_alphabetic() {
            *out.get_mut(n)? = c.to_ascii_lowercase() as u8;
            n += 1;
        } else {
            return None;
        }
    }
    marked.then_some(n)
}

/// FNV-1a of `bytes`: the keys are the published lists' words, fixed, and
/// a lookup is made for every word of a text read, so it is kept short.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The words of the Latin lists that have accents, by their accentless
/// form (the form the device's typed entry and its scanner take): a bit
/// table over the form's first three letters and length that most other
/// words fail at once, and, built the first time a word passes it, the
/// hashes of the forms, sorted.
struct Accentless {
    bits: Vec<u64>,
    forms: std::sync::OnceLock<Vec<(u64, u8, u16)>>,
}

impl Accentless {
    /// Whether the list `l` is read with accentless forms: the lists whose
    /// words hold accented Latin letters, French and Spanish (the other
    /// Latin lists spell theirs without, or with letters that are not a
    /// letter and a mark).
    fn reads(l: Language) -> bool {
        matches!(l, Language::French | Language::Spanish)
    }

    /// The bit of a form's first three letters (5 bits each, as the
    /// letters' low bits go) and length (3 bits), none for a shorter key.
    fn bit(key: &[u8]) -> Option<usize> {
        let [a, b, c, ..] = *key else { return None };
        let len = key.len().clamp(3, 10) - 3;
        Some(
            (len << 15)
                | (usize::from(a & 31) << 10)
                | (usize::from(b & 31) << 5)
                | usize::from(c & 31),
        )
    }

    /// Notes the word `nfkd` of a list read with accentless forms.
    fn note(bits: &mut [u64], nfkd: &str) {
        let mut buf = [0u8; 24];
        if let Some(n) = unaccented(nfkd, &mut buf)
            && let Some(b) = Self::bit(&buf[..n])
        {
            bits[b >> 6] |= 1 << (b & 63);
        }
    }

    fn forms(&self) -> &[(u64, u8, u16)] {
        self.forms.get_or_init(|| {
            let mut forms = Vec::new();
            for (at, &l) in Language::ALL.iter().enumerate() {
                if !Self::reads(l) {
                    continue;
                }
                for i in 0..2048u16 {
                    let mut buf = [0u8; 24];
                    if let Some(n) = unaccented(l.word_nfkd(i), &mut buf) {
                        forms.push((fnv(&buf[..n]), at as u8, i));
                    }
                }
            }
            forms.sort_unstable();
            forms
        })
    }

    /// The lists with a word whose accentless form is `key`.
    fn of(&self, key: &[u8]) -> impl Iterator<Item = (u8, u16)> + '_ {
        let h = fnv(key);
        let forms = self.forms();
        let from = forms.partition_point(|f| f.0 < h);
        forms[from..]
            .iter()
            .take_while(move |f| f.0 == h)
            .map(|f| (f.1, f.2))
    }

    /// Whether `key` may be one of the forms: its first three letters and
    /// length are those of one.
    fn may_hold(&self, key: &[u8]) -> bool {
        Self::bit(key).is_some_and(|b| self.bits[b >> 6] >> (b & 63) & 1 == 1)
    }
}

/// The lists other than English that hold `word`, as (the list's place in
/// `Language::ALL`, the word's index), the word read as the list spells it
/// (decomposed, or composed as it is read) or without its accents, as the
/// device's typed entry and its scanner take it, and in any case. Costs one
/// hash lookup for all lists at once, and for a word that may be an
/// accentless form a second (the index is built once, on first use, from
/// the published lists). Empty for a word on no list, and for one longer
/// than any listed word.
pub fn other_list_entries(word: &str) -> Entries {
    use std::collections::HashMap;
    use std::hash::{BuildHasherDefault, Hasher};
    use std::sync::OnceLock;
    #[derive(Default)]
    struct Fnv(u64);
    impl Hasher for Fnv {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            let h = if self.0 == 0 {
                0xcbf2_9ce4_8422_2325
            } else {
                self.0
            };
            self.0 = bytes
                .iter()
                .fold(h, |h, &b| (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3));
        }
    }
    type Index = HashMap<&'static str, Vec<(u8, u16)>, BuildHasherDefault<Fnv>>;
    static INDEX: OnceLock<(Index, Accentless)> = OnceLock::new();
    let (index, plain) = INDEX.get_or_init(|| {
        let mut m = Index::with_capacity_and_hasher(32768, Default::default());
        let mut bits = vec![0u64; 1 << 12];
        for (at, &l) in Language::ALL.iter().enumerate() {
            if l == Language::English {
                continue;
            }
            for i in 0..2048u16 {
                let (nfkd, shown) = (l.word_nfkd(i), l.word_display(i));
                if Accentless::reads(l) {
                    Accentless::note(&mut bits, nfkd);
                }
                // A word spelled alike in both forms is entered once.
                for w in [nfkd, shown]
                    .into_iter()
                    .take(if nfkd == shown { 1 } else { 2 })
                {
                    let e = m.entry(w).or_default();
                    if !e.contains(&(at as u8, i)) {
                        e.push((at as u8, i));
                    }
                }
            }
        }
        (
            m,
            Accentless {
                bits,
                forms: Default::default(),
            },
        )
    });
    let mut out = Entries {
        n: 0,
        at: [(0, 0); Entries::ROOM],
    };
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
                return out;
            }
            n += c.encode_utf8(&mut key[n..]).len();
        }
    }
    if let Ok(k) = std::str::from_utf8(&key[..n])
        && let Some(v) = index.get(k)
    {
        // A word is spelled alike on a few lists at most.
        let v = &v[..v.len().min(Entries::ROOM)];
        out.at[..v.len()].copy_from_slice(v);
        out.n = v.len() as u8;
    }
    // A word with accents is read without them as its NFKD form folded, so
    // a word with only some of its accents, or composed and decomposed
    // characters mixed, is found as well.
    let mut folded = [0u8; 24];
    let mut f = 0;
    if !word.is_ascii() {
        for c in faraday_bip::bip39::nfkd_lower(word) {
            if ('\u{0300}'..='\u{036F}').contains(&c) {
                continue;
            }
            match folded.get_mut(f) {
                Some(b) if c.is_ascii_alphabetic() => *b = c as u8,
                _ => return out,
            }
            f += 1;
        }
        key[..f].copy_from_slice(&folded[..f]);
        n = f;
    }
    if plain.may_hold(&key[..n]) {
        for e in plain.of(&key[..n]) {
            out.push(e);
        }
    }
    out
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
        for e in other_list_entries(w).iter() {
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
