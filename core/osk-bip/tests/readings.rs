//! The Mandarin readings the two Chinese keyboards look words up by, and
//! the composed forms the Japanese and Korean lists are read in.

use std::collections::BTreeMap;

use osk_bip::bip39::Language;
use osk_bip::wordlists::readings::{READINGS, SYLLABLES};

fn only_char(word: &str) -> char {
    let mut chars = word.chars();
    let c = chars.next().expect("a word is not empty");
    assert_eq!(chars.next(), None, "{word} is more than one character");
    c
}

#[test]
fn every_chinese_character_can_be_read_aloud() {
    for lang in [Language::ChineseSimplified, Language::ChineseTraditional] {
        for i in 0..2048u16 {
            let c = only_char(lang.word(i));
            let readings = lang.readings(c);
            assert!(!readings.is_empty(), "{c} has no reading");
            for &(syllable, tone) in readings {
                assert!(usize::from(syllable) < SYLLABLES.len(), "{c}");
                assert!((1..=5).contains(&tone), "{c} has tone {tone}");
            }
        }
    }
}

#[test]
fn a_reading_is_spelled_in_both_alphabets() {
    for s in SYLLABLES {
        assert!(!s.pinyin.is_empty());
        assert!(s.pinyin.bytes().all(|b| b.is_ascii_lowercase()));
        assert!(!s.zhuyin.is_empty(), "{} has no 注音", s.pinyin);
        assert!(
            s.zhuyin
                .chars()
                .all(|c| ('\u{3105}'..='\u{3129}').contains(&c)),
            "{} is spelled {} , which is not bopomofo",
            s.pinyin,
            s.zhuyin
        );
    }
    // No two syllables share a spelling in either alphabet, so a typed
    // prefix means one thing.
    let mut pinyin: Vec<&str> = SYLLABLES.iter().map(|s| s.pinyin).collect();
    let mut zhuyin: Vec<&str> = SYLLABLES.iter().map(|s| s.zhuyin).collect();
    pinyin.sort_unstable();
    zhuyin.sort_unstable();
    let unique = pinyin.len();
    pinyin.dedup();
    zhuyin.dedup();
    assert_eq!(pinyin.len(), unique);
    assert_eq!(zhuyin.len(), unique);
    assert!(READINGS.windows(2).all(|w| w[0].0 < w[1].0));
}

#[test]
fn typing_a_reading_finds_the_character_it_names() {
    let lang = Language::ChineseSimplified;
    let found = |prefix: &str, tone| -> Vec<char> {
        lang.candidates_pinyin(prefix, tone)
            .map(|i| only_char(lang.word(i)))
            .collect()
    };

    // `yi` is 一, and the list's order puts the commonest character first.
    let yi = found("yi", None);
    assert!(yi.contains(&'一'), "yi did not find 一");
    assert!(yi.contains(&'已'), "yi did not find 已");
    assert_eq!(yi[0], '一');
    // Its tone narrows the strip and keeps 一, which is first tone.
    assert!(found("yi", Some(1)).contains(&'一'));
    assert!(!found("yi", Some(3)).contains(&'一'));

    assert!(found("zhong", None).contains(&'中'));
    // A prefix, not a whole syllable: `zh` reaches every zh- syllable.
    assert!(found("zh", None).contains(&'中'));
    assert!(!found("zho", None).contains(&'张'));
    assert!(found("", None).len() == 2048);

    // 注音 finds the same character by the other spelling.
    let zhong: Vec<char> = lang
        .candidates_zhuyin("ㄓㄨㄥ", None)
        .map(|i| only_char(lang.word(i)))
        .collect();
    assert!(zhong.contains(&'中'));
    assert!(
        Language::ChineseTraditional
            .candidates_zhuyin("ㄧ", Some(1))
            .map(|i| only_char(Language::ChineseTraditional.word(i)))
            .any(|c| c == '一')
    );

    // The readings belong to the Chinese lists alone.
    assert_eq!(Language::English.candidates_pinyin("yi", None).count(), 0);
    assert!(Language::Japanese.readings('中').is_empty());
}

/// A polyphone is offered under each of its readings: 中 is `zhong` in
/// either tone, and 行 is both `xing` and `hang`.
#[test]
fn a_character_with_two_readings_is_found_under_both() {
    let lang = Language::ChineseSimplified;
    let has = |prefix: &str, tone, c: char| {
        lang.candidates_pinyin(prefix, tone)
            .any(|i| only_char(lang.word(i)) == c)
    };
    assert!(has("zhong", Some(1), '中'));
    assert!(has("zhong", Some(4), '中'));
    assert!(has("xing", None, '行'));
    assert!(has("hang", None, '行'));
}

/// The Japanese list is published as base kana plus combining voicing
/// marks and the Korean list as conjoining jamo; a reader sees them
/// composed. The composed forms here are written out by hand, so the test
/// does not need a normalizer, which the device does not have either.
#[test]
fn japanese_and_korean_words_are_read_composed() {
    const JAPANESE: [(u16, &str, &str); 10] = [
        (2, "\u{3042}\u{3044}\u{305f}\u{3099}", "あいだ"),
        (3, "\u{3042}\u{304a}\u{305d}\u{3099}\u{3089}", "あおぞら"),
        (6, "\u{3042}\u{3051}\u{304b}\u{3099}\u{305f}", "あけがた"),
        (42, "\u{3042}\u{3089}\u{305f}\u{3081}\u{308b}", "あらためる"),
        (100, "\u{3044}\u{3063}\u{305f}\u{3093}", "いったん"),
        (500, "\u{304f}\u{3081}\u{308b}", "くめる"),
        (1000, "\u{305d}\u{3046}\u{3053}\u{3099}", "そうご"),
        (1500, "\u{306d}\u{3093}\u{3072}\u{309a}", "ねんぴ"),
        (2000, "\u{308a}\u{308d}\u{3093}", "りろん"),
        (2047, "\u{308f}\u{308c}\u{308b}", "われる"),
    ];
    const KOREAN: [(u16, &str, &str); 10] = [
        (0, "\u{1100}\u{1161}\u{1100}\u{1167}\u{11a8}", "가격"),
        (1, "\u{1100}\u{1161}\u{1101}\u{1173}\u{11b7}", "가끔"),
        (7, "\u{1100}\u{1161}\u{1107}\u{1161}\u{11bc}", "가방"),
        (
            42,
            "\u{1100}\u{1161}\u{11bc}\u{1107}\u{116e}\u{11a8}",
            "강북",
        ),
        (
            100,
            "\u{1100}\u{1167}\u{11bc}\u{1112}\u{1165}\u{11b7}",
            "경험",
        ),
        (500, "\u{1106}\u{1169}\u{11a8}\u{1111}\u{116d}", "목표"),
        (
            1000,
            "\u{1109}\u{1175}\u{11ab}\u{1102}\u{1167}\u{11b7}",
            "신념",
        ),
        (
            1500,
            "\u{110c}\u{1165}\u{11af}\u{110b}\u{1163}\u{11a8}",
            "절약",
        ),
        (2000, "\u{1112}\u{116a}\u{1107}\u{116e}\u{11ab}", "화분"),
        (
            2047,
            "\u{1112}\u{1175}\u{11b7}\u{1101}\u{1165}\u{11ba}",
            "힘껏",
        ),
    ];

    for (lang, rows) in [
        (Language::Japanese, JAPANESE.as_slice()),
        (Language::Korean, KOREAN.as_slice()),
    ] {
        for &(i, published, read) in rows {
            assert_eq!(lang.word(i), published, "{} #{i}", lang.name());
            assert_eq!(lang.word_display(i), read, "{} #{i}", lang.name());
        }
    }

    // Every word is composed: no combining voicing mark and no conjoining
    // jamo survives into the display form, and the words stay distinct.
    for i in 0..2048u16 {
        assert!(
            Language::Japanese
                .word_display(i)
                .chars()
                .all(|c| !matches!(c, '\u{3099}' | '\u{309a}'))
        );
        assert!(
            Language::Korean
                .word_display(i)
                .chars()
                .all(|c| ('\u{ac00}'..='\u{d7a3}').contains(&c))
        );
    }
}

/// A list whose published words are already composed reads exactly as it
/// is published, so nothing on screen moved for the other eight.
#[test]
fn the_other_lists_are_read_as_published() {
    for lang in Language::ALL {
        if matches!(
            lang,
            Language::Japanese | Language::Korean | Language::Spanish | Language::French
        ) {
            continue;
        }
        for i in 0..2048u16 {
            assert_eq!(lang.word_display(i), lang.word(i), "{} #{i}", lang.name());
        }
    }
    // Spanish and French are published decomposed too; composed, `ábaco`
    // is one character shorter than the `a` + accent it is published as.
    assert_eq!(Language::Spanish.word_display(0), "ábaco");
    assert_eq!(Language::Spanish.word(0).chars().count(), 6);
    assert_eq!(Language::Spanish.word_display(0).chars().count(), 5);
}

/// Every character of both lists can be typed: each of its readings,
/// with its tone, offers it. Mandarin is heavily homophonic, so most
/// characters share their syllable and tone with others a dictionary
/// lists under the same reading; the counts here are what the data
/// says, and the candidate strip is how one of a homophone group is
/// taken.
#[test]
fn every_character_is_reachable_by_each_of_its_readings() {
    for lang in [Language::ChineseSimplified, Language::ChineseTraditional] {
        // Syllable and tone to the characters read that way.
        let mut groups: BTreeMap<(&str, u8), Vec<u16>> = BTreeMap::new();
        for i in 0..2048u16 {
            for reading in lang.word_readings(i) {
                groups.entry(reading).or_default().push(i);
            }
        }
        for i in 0..2048u16 {
            let c = only_char(lang.word(i));
            for (spelling, tone) in lang.word_readings(i) {
                let found: Vec<u16> = if lang == Language::ChineseSimplified {
                    lang.candidates_pinyin(spelling, Some(tone)).collect()
                } else {
                    lang.candidates_zhuyin(spelling, Some(tone)).collect()
                };
                assert!(
                    found.contains(&i),
                    "{c} is not offered under {spelling} tone {tone}"
                );
                // The prefix search reaches longer syllables too; the
                // group is the characters read exactly this way.
                for j in &groups[&(spelling, tone)] {
                    assert!(found.contains(j), "{spelling} tone {tone} lost {j}");
                }
            }
        }
        let shared = groups.values().filter(|g| g.len() > 1).count();
        let only_way = (0..2048u16)
            .filter(|&i| lang.word_readings(i).all(|r| groups[&r].len() > 1))
            .count();
        let (want_shared, want_only_way) = match lang {
            Language::ChineseSimplified => (586, 1652),
            _ => (571, 1643),
        };
        assert_eq!(
            shared, want_shared,
            "{lang:?}: readings two characters share"
        );
        assert_eq!(
            only_way, want_only_way,
            "{lang:?}: characters every reading of which is shared"
        );
    }
}

/// The two spellings are two ways to the same word: every syllable
/// reaches the same characters typed as 注音 as it does typed as pinyin.
#[test]
fn a_syllable_finds_the_same_characters_in_either_spelling() {
    for lang in [Language::ChineseSimplified, Language::ChineseTraditional] {
        for s in SYLLABLES {
            for tone in [None, Some(1), Some(2), Some(3), Some(4), Some(5)] {
                let pinyin: Vec<u16> = lang.candidates_pinyin(s.pinyin, tone).collect();
                let zhuyin: Vec<u16> = lang.candidates_zhuyin(s.zhuyin, tone).collect();
                // A pinyin spelling is a prefix of longer ones (`zh` of
                // `zhong`), and so is a bopomofo one, but a whole
                // syllable's own characters are in both.
                for i in &pinyin {
                    if lang
                        .word_readings(*i)
                        .any(|(sp, t)| sp == s.pinyin && tone.is_none_or(|want| want == t))
                    {
                        assert!(zhuyin.contains(i), "{} is missing {}", s.zhuyin, i);
                    }
                }
                for i in &zhuyin {
                    if lang.word_readings(*i).any(|(sp, t)| {
                        (sp == s.pinyin || sp == s.zhuyin) && tone.is_none_or(|want| want == t)
                    }) {
                        assert!(pinyin.contains(i), "{} is missing {}", s.pinyin, i);
                    }
                }
            }
        }
    }
}
