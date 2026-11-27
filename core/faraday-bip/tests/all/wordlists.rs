//! Properties of the embedded wordlists.

use faraday_bip::bip39::Language;
use faraday_bip::wordlists;

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

/// A text that is a listed word with something added or taken off is not
/// found, whatever its length.
#[test]
fn a_changed_word_is_not_found() {
    for lang in Language::ALL {
        for i in (0..2048u16).step_by(7) {
            let w = lang.word_nfkd(i);
            assert_eq!(lang.index_of(&format!("{w}\0")), None);
            assert_eq!(lang.index_of(&format!("{w}a")), None);
            assert_eq!(lang.index_of(&format!("{w}{w}{w}{w}{w}{w}")), None);
            if let Some((cut, _)) = w.char_indices().last() {
                assert_eq!(lang.index_of(&w[..cut]).filter(|&j| j == i), None);
            }
            // Every length around the longest word, with the word's own
            // bytes first and the rest zero or a letter.
            for len in (w.len() + 1).max(33)..=41 {
                let rest = len - w.len();
                assert_eq!(lang.index_of(&format!("{w}{}", "\0".repeat(rest))), None);
                assert_eq!(lang.index_of(&format!("{w}{}", "a".repeat(rest))), None);
            }
        }
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
