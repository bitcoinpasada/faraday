//! Properties of the embedded wordlists.

use std::collections::HashSet;

use osk_bip::bip39::Language;
use osk_bip::wordlists;
use osk_crypto::sha256;

#[test]
fn each_list_has_2048_unique_words() {
    for lang in Language::ALL {
        let words = lang.words();
        let unique: HashSet<&str> = words.iter().copied().collect();
        assert_eq!(unique.len(), 2048, "{}", lang.name());
        assert!(
            words
                .iter()
                .all(|w| !w.is_empty() && !w.chars().any(char::is_whitespace))
        );
    }
}

#[test]
fn english_words_are_unique_by_first_four_chars() {
    let prefixes: HashSet<String> = Language::English
        .words()
        .iter()
        .map(|w| w.chars().take(4).collect())
        .collect();
    assert_eq!(prefixes.len(), 2048);
    assert!(Language::English.words().iter().all(|w| w.is_ascii()));
}

#[test]
fn every_word_round_trips_through_index_of() {
    for lang in Language::ALL {
        for i in 0..2048u16 {
            assert_eq!(lang.index_of(lang.word(i)), Some(i), "{} #{i}", lang.name());
            assert_eq!(lang.index_of(lang.word_nfkd(i)), Some(i));
        }
        assert_eq!(lang.index_of(""), None);
        assert_eq!(lang.index_of("not a word"), None);
    }
}

#[test]
fn sha256_constants_match_the_published_files() {
    // gen.py verified that each source file is exactly the 2048 words, each
    // followed by "\n"; rebuild those bytes and hash them.
    for lang in Language::ALL {
        let mut bytes = Vec::new();
        for w in lang.words() {
            bytes.extend_from_slice(w.as_bytes());
            bytes.push(b'\n');
        }
        let digest: String = sha256(&bytes).iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(digest, lang.wordlist_sha256(), "{}", lang.name());
    }
}

#[test]
fn no_word_is_longer_than_the_nfkd_bound() {
    for lang in Language::ALL {
        let longest = lang.words_nfkd().iter().map(|w| w.len()).max().unwrap();
        assert!(longest <= wordlists::MAX_NFKD_BYTES);
    }
}

#[test]
fn candidates_are_prefix_matches_in_order() {
    let en = Language::English;
    let abs: Vec<u16> = en.candidates("abs").collect();
    let expected: Vec<u16> = (0..2048u16)
        .filter(|&i| en.word(i).starts_with("abs"))
        .collect();
    assert_eq!(abs, expected);
    assert_eq!(
        abs.iter().map(|&i| en.word(i)).collect::<Vec<_>>(),
        ["absent", "absorb", "abstract", "absurd"]
    );
    assert_eq!(en.candidates("").count(), 2048);
    assert_eq!(en.candidates("zzz").count(), 0);
    assert_eq!(Language::Japanese.candidates("あい").count(), 3);
}

#[test]
fn japanese_separates_words_with_an_ideographic_space() {
    assert_eq!(Language::Japanese.separator(), "\u{3000}");
    for lang in Language::ALL {
        if lang != Language::Japanese {
            assert_eq!(lang.separator(), " ");
        }
    }
}
