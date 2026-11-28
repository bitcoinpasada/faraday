//! The word lists read from a text: `main`'s `forms::typed_index`, `typed_mnemonic` and `other_list_index`.

use faraday_bip::bip39::{Language, Mnemonic};

/// The list's word a typed word is, matched on its ASCII fold.
pub(crate) fn typed_index(lang: Language, typed: &str) -> Option<u16> {
    // The English list is sorted and its typed form is the word itself:
    // a binary search finds the word in 11 comparisons, not 2048.
    if lang == Language::English {
        let (mut lo, mut hi) = (0u16, 2048u16);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            match lang.word(mid).cmp(typed) {
                std::cmp::Ordering::Less => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
                std::cmp::Ordering::Equal => return Some(mid),
            }
        }
        return None;
    }
    // The characters go into a wiping buffer sized once, so no grown-out
    // block is freed holding them; a word has no more characters than bytes.
    let mut chars = faraday_crypto::Zeroizing::new(Vec::<char>::with_capacity(typed.len()));
    chars.extend(typed.chars());
    lang.candidates_typed(&chars).find(|&i| {
        lang.typed(i)
            .is_some_and(|t| t.as_chars() == chars.as_slice())
    })
}

/// The index of `word` on `lang`'s list, a list other than English.
pub(crate) fn other_list_index(lang: Language, word: &str) -> Option<u16> {
    let at = Language::ALL.iter().position(|&l| l == lang)? as u8;
    other_list_entries(word)
        .iter()
        .find(|e| e.0 == at)
        .map(|e| e.1)
}

/// The typed words as a mnemonic in `lang`, when every one is a word and the checksum holds.
pub(crate) fn typed_mnemonic(lang: Language, typed: &str) -> Option<Mnemonic> {
    // The word numbers name the words: a wiping buffer sized once (it is wiped on every return,
    // the early one included).
    let mut idx =
        faraday_crypto::Zeroizing::new(Vec::<u16>::with_capacity(typed.split_whitespace().count()));
    for w in typed.split_whitespace() {
        idx.push(typed_index(lang, w)?);
    }
    faraday_keys::words::from_indices(lang, &idx).ok()
}

/// The lists other than English that hold `word` (see [`faraday_intake::other_list_entries`]).
pub(crate) fn other_list_entries(word: &str) -> faraday_intake::Entries {
    faraday_intake::other_list_entries(word)
}
