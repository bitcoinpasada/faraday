//! The secret a whole text is (`main`'s `scan_read::secret_kind` and `secret_text_kind`): a seed
//! in any of its forms, a share of one, words of a BIP-39 list that are no seed, a private key,
//! a paperkey, a TOTP setup code or vault entries.

use faraday_bip::bip39::Language;
use faraday_crypto::work::Compute;
use faraday_text::Text;

use crate::kind::SecretKind;

/// Costs a mnemonic's checksum, a codex32 and a SLIP-39 parse, a WIF's
/// checksum and a CRC-24 over a paperkey's lines. The secret kind one
/// code read is, if it is one: a seed in any of its forms, a share of
/// one, words of a BIP-39 list that are no seed, a private key, a
/// paperkey, a TOTP setup code or vault entries.
pub(crate) fn secret_kind(c: &Compute, bytes: &[u8]) -> Option<SecretKind> {
    if faraday_keys::gpg::is_paperkey(bytes) {
        return Some(SecretKind::Paperkey);
    }
    // A CompactSeedQR is 16 or 32 bytes of entropy that no text kind took.
    if faraday_codec::classify::is_compact_seedqr(bytes) {
        return Some(SecretKind::CompactSeedQr);
    }
    let text = Text::of(Text::from_utf8_lossy(bytes).trim());
    secret_text_kind(c, &text)
}

/// Costs what [`secret_kind`] costs but the paperkey's CRC. The secret
/// kind a code's text is, if it is one.
pub(crate) fn secret_text_kind(c: &Compute, text: &str) -> Option<SecretKind> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if matches!(t.len(), 48 | 96) && t.bytes().all(|b| b.is_ascii_digit()) {
        return Some(SecretKind::SeedQr);
    }
    if faraday_keys::identity::is_wif(t) {
        return Some(SecretKind::Wif);
    }
    if t.get(..5).is_some_and(|p| p.eq_ignore_ascii_case("nsec1")) {
        return Some(SecretKind::Nostr);
    }
    match faraday_intake::secret_text(c, t) {
        Some(faraday_intake::TextSecret::Words) => return Some(SecretKind::SeedWords),
        Some(faraday_intake::TextSecret::ExtendedPrivateKey) => return Some(SecretKind::Xprv),
        Some(faraday_intake::TextSecret::SilentSpendKey) => return Some(SecretKind::SpendKey),
        Some(faraday_intake::TextSecret::Wif) => return Some(SecretKind::Wif),
        None => {}
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    if matches!(words.len(), 12 | 15 | 18 | 21 | 24)
        && let Some(lang) = on_a_word_list(&words)
    {
        // A seed of a list other than English is a seed.
        if checksum_holds(lang, &words) {
            return Some(SecretKind::SeedWords);
        }
        // 24 words whose BIP-39 checksum fails may be an aezeed, which
        // has its own (`docs/SCANNING.md` §4.2).
        return Some(if is_aezeed(&words) {
            SecretKind::Aezeed
        } else {
            SecretKind::BadChecksum
        });
    }
    if let Ok(s) = faraday_bip::codex32::Codex32::parse(t) {
        return Some(if s.is_secret() {
            SecretKind::Codex32Secret
        } else {
            SecretKind::Codex32Share
        });
    }
    let lower = Text::lowercase(t);
    if faraday_bip::slip39::Share::parse(&lower).is_ok() {
        return Some(SecretKind::Slip39Share);
    }
    if lower.starts_with("ur:crypto-seed/") || lower.starts_with("ur:seed/") {
        return Some(SecretKind::UrSeed);
    }
    if faraday_intake::is_otpauth_migration(t) {
        return Some(SecretKind::Totp);
    }
    if faraday_codec::classify::looks_like_entries(t) {
        let one = t.lines().filter(|l| !l.trim().is_empty()).count() == 1;
        return Some(if one && faraday_codec::otpauth::is_otpauth(t) {
            SecretKind::Totp
        } else {
            SecretKind::Entries
        });
    }
    None
}

/// A lookup per word and a CRC-32C over 29 bytes. Whether the words are
/// an LND aezeed: 24 words of the English list, its version byte and its
/// CRC-32C holding (`faraday_bip::aezeed::checksum_holds`).
fn is_aezeed(words: &[&str]) -> bool {
    let mut idx = [0u16; faraday_bip::aezeed::NUM_WORDS];
    if words.len() != idx.len() {
        return false;
    }
    for (slot, w) in idx.iter_mut().zip(words) {
        match Language::English.index_of(&Text::lowercase(w)) {
            Some(i) => *slot = i,
            None => {
                faraday_crypto::Zeroize::zeroize(&mut idx);
                return false;
            }
        }
    }
    let holds = faraday_bip::aezeed::checksum_holds(&idx);
    faraday_crypto::Zeroize::zeroize(&mut idx);
    holds
}

/// The BIP-39 list Faraday reads that every word is on, in any case,
/// composed or decomposed, with or without accents; where several lists
/// have every word, the one whose checksum holds, else the first. A list
/// lookup per word and list, and a checksum.
pub(crate) fn on_a_word_list(words: &[&str]) -> Option<Language> {
    let mut first = None;
    for lang in Language::ALL {
        if !words.iter().all(|w| lang.find_in(w).is_some()) {
            continue;
        }
        if checksum_holds(lang, words) {
            return Some(lang);
        }
        first = first.or(Some(lang));
    }
    first
}

/// Whether `words`, all on `lang`'s list, are a seed of it by their
/// checksum. A lookup per word.
fn checksum_holds(lang: Language, words: &[&str]) -> bool {
    let mut idx = faraday_crypto::Zeroizing::new(Vec::with_capacity(words.len()));
    for w in words {
        match lang.find_in(w) {
            Some(i) => idx.push(i),
            None => return false,
        }
    }
    faraday_keys::words::from_indices(lang, &idx).is_ok()
}
