//! Typing the four non-Latin lists through the app: the kana grid, the
//! two-set jamo keyboard, and the pinyin and 注音 keyboards that look a
//! character up by its Mandarin reading. Keys are tapped one by one,
//! with the dead keys the candidates leave dead and the field reading
//! back what the fingers did.

mod common;

use common::{DESKTOP, Harness, PANEL, PHONE, TINY};
use opensigner_core::ScreenKind;
use opensigner_core::create::Step as CreateStep;
use opensigner_core::ids;
use opensigner_core::load::Step;
use opensigner_core::quiz::QuizState;
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{MasterKey, Network};
use osk_bip::{hangul, kana};
use osk_entropy::{DiceRolls, Strength};
use osk_shell_api::{App, Key};
use osk_ui::widgets::keyboard::{self, KeyInput, KeyboardKind, SMALL_KEY};
use serde_json::Value;

const JAPANESE_VECTORS: &str = include_str!("../../../tools/vectors/bip39/test_JP_BIP39.json");

/// The bip32JP passphrase in NFKD form, which is what BIP-39 salts with.
/// It is not ASCII, so it goes through the test-only `to_seed_unchecked`
/// as osk-bip's own vector run does; the device's passphrase keyboard is
/// ASCII (`docs/PLANNING.md` §16.2).
const JP_PASSPHRASE_NFKD: &str = "e383a1e383bce38388e383abe382abe38299e3838fe38299e382a6e38299e382a1e381afe3829ae381afe38299e3818fe38299e3829de38299e381a1e381a1e38299e58d81e4babae58d81e889b2";

/// The keys that type `word` (an NFKD wordlist word), in order. A small
/// kana is its full-size key and then 小; a doubled jamo is shift and
/// then the key under it.
fn taps(lang: Language, word: &str) -> Vec<KeyInput> {
    let mut out = Vec::new();
    match lang {
        Language::Japanese => {
            for c in word.chars() {
                if let Some(big) = kana::large(c) {
                    out.push(KeyInput::Char(big));
                    out.push(KeyInput::Char(SMALL_KEY));
                } else if c == kana::DAKUTEN {
                    out.push(KeyInput::Char(kana::DAKUTEN_KEY));
                } else if c == kana::HANDAKUTEN {
                    out.push(KeyInput::Char(kana::HANDAKUTEN_KEY));
                } else {
                    out.push(KeyInput::Char(c));
                }
            }
        }
        Language::Korean => {
            for jamo in word.chars() {
                for &key in hangul::keys_of(jamo).expect("a modern jamo") {
                    // The doubled consonants and the two shifted vowels
                    // are on the shift layer of their own key.
                    if keyboard::keycaps(KeyboardKind::Jamo)
                        .position(|k| k == key)
                        .is_some_and(|i| i >= 26)
                    {
                        out.push(KeyInput::Shift);
                    }
                    out.push(KeyInput::Char(key));
                }
            }
        }
        _ => panic!("this helper types the two non-Latin lists"),
    }
    out
}

/// Walks the wizard to the word step of `lang` with `count` words.
fn to_words(h: &mut Harness, lang: Language, count: u8) {
    h.open_load();
    h.tap(ids::LOAD_SOURCE_TYPE);
    h.tap(ids::LOAD_SOURCE_CONTINUE);
    let i = [12u8, 24, 15, 18, 21]
        .iter()
        .position(|c| *c == count)
        .expect("a listed count");
    h.tap(ids::at(ids::LOAD_COUNT_BASE, i));
    h.tap(ids::LOAD_COUNT_CONTINUE);
    let l = Language::ALL
        .iter()
        .position(|l| *l == lang)
        .expect("a wordlist");
    h.tap(ids::at(ids::LOAD_LANG_BASE, l));
    h.tap(ids::LOAD_LANG_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Words));
}

fn type_word(h: &mut Harness, lang: Language, word: &str) {
    for input in taps(lang, word) {
        h.pad(ids::LOAD_KEYBOARD, input);
    }
    // §16.118: the keys never take the word. A word typed out in full
    // is the first candidate, and the tap on it is what accepts.
    h.tap_candidate(0, word);
}

/// The first Japanese vector: its words, its passphrase and its seed.
fn first_japanese_vector() -> (Vec<String>, String, String) {
    let v: Value = serde_json::from_str(JAPANESE_VECTORS).expect("the vector file");
    let first = &v[0];
    let words = first["mnemonic"]
        .as_str()
        .expect("a sentence")
        .split_whitespace()
        .map(String::from)
        .collect();
    (
        words,
        String::from(first["passphrase"].as_str().expect("a passphrase")),
        String::from(first["seed"].as_str().expect("a seed")),
    )
}

/// A Japanese mnemonic typed on the kana grid reaches the checksum, and
/// the key it makes is the key the published vector's seed makes.
#[test]
fn a_japanese_vector_types_on_the_kana_grid_and_makes_its_key() {
    let (words, _, seed_hex) = first_japanese_vector();
    assert_eq!(words.len(), 12);
    let mut h = Harness::new(PANEL);
    to_words(&mut h, Language::Japanese, 12);
    for word in &words {
        // The vector file is NFC; the list and the keys are NFKD.
        type_word(
            &mut h,
            Language::Japanese,
            Language::Japanese.word_nfkd(index_of(word)),
        );
    }
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    // The checksum passed: the wizard offers the way on rather than a
    // start-over.
    assert!(h.app.rect_of(ids::LOAD_CONTINUE).is_some());

    // The words that were typed are the vector's: with the vector's own
    // passphrase they stretch into the seed the file publishes.
    let nfkd: Vec<&str> = words
        .iter()
        .map(|w| Language::Japanese.word_nfkd(index_of(w)))
        .collect();
    let m = Mnemonic::parse(Language::Japanese, &nfkd.join(" ")).expect("a valid mnemonic");
    let seed = m
        .to_seed_unchecked(&unhex(JP_PASSPHRASE_NFKD))
        .expect("a seed");
    assert_eq!(hex(&seed.expose()[..]), seed_hex);

    // Finish the load with no passphrase; the key the wizard adds is the
    // one those same words make on their own.
    h.finish_load(None);
    let plain = m.to_seed(b"").expect("a seed");
    assert_eq!(
        h.app.fingerprints(),
        vec![MasterKey::from_seed(&plain, Network::Mainnet).fingerprint()]
    );
}

/// The index of a word written the way the vector file writes it (NFC).
fn index_of(word: &str) -> u16 {
    (0..2048u16)
        .find(|&i| Language::Japanese.word_display(i) == word)
        .unwrap_or_else(|| panic!("{word} is not in the Japanese list"))
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).expect("hex"))
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A Korean mnemonic types on the two-set keyboard, shift and all.
#[test]
fn a_korean_mnemonic_types_on_the_jamo_keyboard() {
    let entropy = [0x77u8; 16];
    let m = Mnemonic::from_entropy(Language::Korean, &entropy).expect("a mnemonic");
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&i| Language::Korean.word_nfkd(i))
        .collect();
    let mut h = Harness::new(PANEL);
    to_words(&mut h, Language::Korean, 12);
    for word in &words {
        type_word(&mut h, Language::Korean, word);
    }
    assert_eq!(h.app.load_step(), Some(Step::Checksum));
    assert!(h.app.rect_of(ids::LOAD_CONTINUE).is_some());
}

/// The candidate strip offers whole words in the form a reader writes
/// them, not the decomposed form the list is published in.
#[test]
fn the_candidate_strip_shows_words_as_they_are_written() {
    let mut h = Harness::new(PANEL);
    to_words(&mut h, Language::Japanese, 12);
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('\u{3042}'));
    let shown = h.app.candidates();
    assert!(!shown.is_empty());
    for word in &shown {
        assert!(
            !word
                .chars()
                .any(|c| c == kana::DAKUTEN || c == kana::HANDAKUTEN),
            "{word} carries a combining mark"
        );
        assert!(word.starts_with('\u{3042}'), "{word}");
    }
    let mut h = Harness::new(PANEL);
    to_words(&mut h, Language::Korean, 12);
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('\u{3131}'));
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('\u{314F}'));
    for word in h.app.candidates() {
        assert!(word.starts_with('\u{AC00}'), "{word} is not a syllable");
    }
}

/// §4.3's panel of the words accepted so far is drawn where the space
/// above the entry group has the height for this list's words: a Korean
/// word is two syllables and keeps two columns of six rows on a phone
/// and in a window, while a Japanese word can be seven kana, which
/// leaves one column of twelve rows that neither class has the height
/// for above a field and a keyboard. A 268 dp panel has the room for
/// neither, so it shows the word being typed and nothing else. Where the
/// panel is left off, so is the app bar's eye: it would reveal nothing.
#[test]
fn the_words_panel_is_drawn_where_the_list_fits_it() {
    let first = [
        (
            Language::Japanese,
            "\u{3042}\u{3044}\u{3053}\u{304f}\u{3057}\u{3093}",
        ),
        (Language::Korean, "\u{1100}\u{1161}\u{1100}\u{1167}\u{11a8}"),
    ];
    for (display, drawn) in [
        (PHONE, [false, true]),
        (DESKTOP, [false, true]),
        (PANEL, [false, false]),
        (TINY, [false, false]),
    ] {
        for ((lang, word), expected) in first.iter().zip(drawn) {
            let mut h = Harness::new(display);
            to_words(&mut h, *lang, 12);
            type_word(&mut h, *lang, word);
            assert_eq!(h.app.screen(), ScreenKind::Load);
            let panel = h.app.rect_of(ids::LOAD_WORDS_PANEL);
            assert_eq!(panel.is_some(), expected, "{lang:?} on {}", display.width);
            assert_eq!(
                h.app.rect_of(ids::SECRET_EYE).is_some(),
                expected,
                "the eye goes with the panel"
            );
            if let Some(r) = panel {
                assert!(r.w > 0 && r.h > 0);
                assert!(
                    r.right() <= h.app.frame().width as i32
                        && r.bottom() <= h.app.frame().height as i32,
                    "{lang:?}: the panel runs past the screen"
                );
            }
        }
    }
}

/// Fifty plausible rolls, the same run `tests/create.rs` uses.
const RANDOM_50: &str = "32461151351521144121541512665155412152342515356215";

/// A Korean key made from dice: the words screen shows syllables, and
/// the quiz offers and takes them in that form.
#[test]
fn a_korean_key_is_created_read_and_quizzed() {
    let mut h = Harness::new(PANEL);
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 0),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    // Korean is a row that takes now.
    let korean = Language::ALL
        .iter()
        .position(|l| *l == Language::Korean)
        .expect("a wordlist");
    h.choose(
        ids::at(ids::CREATE_LANG_BASE, korean),
        ids::CREATE_LANG_CONTINUE,
    );
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    h.type_text(RANDOM_50);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(CreateStep::Words));

    let mut dice = DiceRolls::new();
    for c in RANDOM_50.bytes() {
        dice.push(c - b'0');
    }
    let entropy = dice.entropy(Strength::Bits128).expect("128 bits");
    let m = Mnemonic::from_entropy(Language::Korean, entropy.as_bytes()).expect("a mnemonic");
    let words: Vec<String> = m
        .indices()
        .iter()
        .map(|&i| String::from(Language::Korean.word_display(i)))
        .collect();

    // The words screen shows syllables, not conjoining jamo.
    h.tap(ids::SECRET_EYE);
    let shown = h.app.texts().join(" ");
    assert!(
        shown.contains(words[0].as_str()),
        "{} is not on the screen",
        words[0]
    );
    assert!(
        !shown
            .chars()
            .any(|c| ('\u{1100}'..='\u{11C2}').contains(&c)),
        "a conjoining jamo reached the screen"
    );

    h.tap(ids::SECRET_EYE);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::QUIZ_START);
    for _ in 0..words.len() {
        let v = h.app.quiz_view().expect("a quiz on screen");
        assert_eq!(v.state, QuizState::Asking);
        let want = &words[v.word_number - 1];
        let slot = v
            .choices
            .iter()
            .position(|c| c == want)
            .unwrap_or_else(|| panic!("word {} not offered in {:?}", v.word_number, v.choices));
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.backup_verified(0), Some(true));
    assert_eq!(
        h.app.fingerprints(),
        vec![
            MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Mainnet).fingerprint()
        ]
    );
}

// --- Chinese: pinyin and 注音 ----------------------------------------

/// The two Chinese lists in the order the language step lists them.
fn language_row(lang: Language) -> usize {
    Language::ALL
        .iter()
        .position(|l| *l == lang)
        .expect("a wordlist")
}

/// The keys that type the first reading of the word at `idx`: the
/// syllable's spelling, then its tone — a digit in pinyin, a mark in
/// 注音, ˉ for the first tone.
fn reading_taps(lang: Language, idx: u16) -> Vec<KeyInput> {
    let (spelling, tone) = lang.word_readings(idx).next().expect("a reading");
    let mut out: Vec<KeyInput> = spelling.chars().map(KeyInput::Char).collect();
    if lang == Language::ChineseSimplified {
        out.push(KeyInput::Char(char::from(b'0' + tone)));
    } else {
        out.push(KeyInput::Char(
            keyboard::ZHUYIN_MARKS[usize::from(tone) - 1],
        ));
    }
    out
}

/// Taps cell `n` of the candidate strip, counting across both rows: the
/// first `per_row` cells are the first row, the rest the second (§4.3,
/// §16.45).
fn tap_candidate(h: &mut Harness, n: usize, per_row: usize) {
    let (id, cell) = if n < per_row {
        (ids::LOAD_CANDIDATES, n)
    } else {
        (ids::LOAD_CANDIDATES_2, n - per_row)
    };
    // One candidate left is one chip rather than a strip of cells, and
    // it is tapped by its id.
    let Some(r) = h.app.candidate_rect(id, cell) else {
        assert_eq!(n, 0, "no candidate cell {n}");
        h.tap(id);
        return;
    };
    let c = r.center();
    h.tap_at(c.x as u16, c.y as u16);
}

/// How many cells a row of the strip has here: ten for a list whose
/// words are one character, the class's own count otherwise.
fn per_row(h: &Harness, lang: Language) -> usize {
    osk_ui::tokens::candidates_per_row(h.app.class(), lang.max_display_chars() == 1)
}

/// Takes candidate `n`: one tap where a tap accepts, two where the first
/// selects. Nothing is left half-taken either way.
fn accept_candidate(h: &mut Harness, lang: Language, n: usize) {
    let cells = per_row(h, lang);
    tap_candidate(h, n, cells);
    if !h.app.candidates().is_empty() {
        tap_candidate(h, n, cells);
    }
}

/// Types one character by its reading and takes it from the strip,
/// turning the page until it is offered. Mandarin is homophonic: most
/// characters share their syllable and tone with others, so the strip is
/// how one of them is accepted.
fn type_hanzi(h: &mut Harness, lang: Language, idx: u16) {
    for input in reading_taps(lang, idx) {
        h.pad(ids::LOAD_KEYBOARD, input);
    }
    let want = String::from(lang.word_display(idx));
    for _ in 0..64 {
        let shown = h.app.candidates();
        let more = h.app.candidate_more();
        if shown.len() == 1 && !more {
            // One candidate left: it is the one chip on the strip, and
            // the tap on it is what takes it (§16.118).
            assert_eq!(shown[0], want);
            accept_candidate(h, lang, 0);
            return;
        }
        let takeable = if more { shown.len() - 1 } else { shown.len() };
        if let Some(n) = shown[..takeable].iter().position(|w| *w == want) {
            accept_candidate(h, lang, n);
            return;
        }
        assert!(more, "{want} is not among {shown:?}");
        tap_candidate(h, takeable, per_row(h, lang));
    }
    panic!("{want} never appeared on the strip");
}

/// A Simplified mnemonic types on the pinyin keyboard, tone by tone, and
/// reaches the checksum; the Traditional list types the same characters
/// through 注音. The two lists share their index order, so the same
/// twelve indices are the same twelve words in either script.
#[test]
fn the_two_chinese_lists_type_by_reading_and_reach_the_checksum() {
    for lang in [Language::ChineseSimplified, Language::ChineseTraditional] {
        let m = Mnemonic::from_entropy(lang, &[0x42u8; 16]).expect("a mnemonic");
        let indices: Vec<u16> = m.indices().to_vec();
        let mut h = Harness::new(PHONE);
        to_words(&mut h, lang, 12);
        for &i in &indices {
            type_hanzi(&mut h, lang, i);
        }
        assert_eq!(h.app.load_step(), Some(Step::Checksum), "{lang:?}");
        assert!(h.app.rect_of(ids::LOAD_CONTINUE).is_some(), "{lang:?}");
        h.finish_load(None);
        assert_eq!(
            h.app.fingerprints(),
            vec![
                MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Mainnet)
                    .fingerprint()
            ],
            "{lang:?}"
        );
    }
}

/// The pinyin keyboard's dead keys: after `zh` only the letters that
/// continue some syllable are live, and the tone keys wake once the
/// spelling is a whole syllable. No candidate shows until the tone is
/// typed (§16.43). The field shows what was typed.
#[test]
fn the_pinyin_keys_follow_the_syllables_and_the_tones_end_them() {
    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::ChineseSimplified, 12);
    for c in "zh".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    let live = |h: &Harness, c: char| {
        h.app
            .key_rect(ids::LOAD_KEYBOARD, KeyInput::Char(c))
            .is_some()
    };
    for c in ['i', 'u'] {
        assert!(live(&h, c), "zh{c} is a syllable");
    }
    assert!(!live(&h, 'b'), "no syllable is zhb");
    assert!(!live(&h, '1'), "zh is not a syllable of its own");
    for c in "ong".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    assert!(live(&h, '1'), "zhong is a first-tone syllable");
    assert!(live(&h, '4'), "and a fourth-tone one");
    assert!(
        h.app.texts().iter().any(|t| t == "zhong"),
        "{:?}",
        h.app.texts()
    );
    // The reading is not complete, so nothing is offered — and no error
    // either: an incomplete reading is not a wrong one.
    assert!(
        h.app.candidates().is_empty(),
        "no candidate before the tone"
    );
    assert!(!h.app.texts().iter().any(|t| t.contains("No word")));
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('1'));
    assert!(h.app.texts().iter().any(|t| t == "zhong1"));
    // The tone ends the syllable: nothing follows it but a backspace.
    assert!(!live(&h, 'a') && !live(&h, '2'));
    assert!(
        h.app.candidates().contains(&String::from("\u{4e2d}")),
        "zhong1 offers 中"
    );
}

/// 注音 types the same word: the bopomofo keys narrow, the five marks
/// give the five tones, and the field shows the bopomofo and its mark.
/// ㄓㄨㄥ then ˉ is `zhong1` in the other script, so the two keyboards
/// reach the same characters.
#[test]
fn the_zhuyin_keys_spell_a_syllable_and_its_mark() {
    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::ChineseTraditional, 12);
    let live = |h: &Harness, c: char| {
        h.app
            .key_rect(ids::LOAD_KEYBOARD, KeyInput::Char(c))
            .is_some()
    };
    for c in "\u{3113}\u{3128}\u{3125}".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == "\u{3113}\u{3128}\u{3125}"),
        "the field shows ㄓㄨㄥ"
    );
    // ㄓㄨㄥ is as long as the syllable goes, so no bopomofo key is live;
    // the marks are, for the tones the syllable has.
    assert!(!live(&h, '\u{3128}'));
    assert!(live(&h, keyboard::TONE_FALLING), "ㄓㄨㄥˋ");
    assert!(live(&h, keyboard::TONE_HIGH), "ㄓㄨㄥˉ");
    assert!(
        h.app.candidates().is_empty(),
        "the reading is not complete until a mark is pressed"
    );
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(keyboard::TONE_HIGH));
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == "\u{3113}\u{3128}\u{3125}\u{02C9}"),
        "the field shows ㄓㄨㄥˉ"
    );
    let zhuyin = h.app.candidates();
    assert!(zhuyin.contains(&String::from("\u{4E2D}")), "{zhuyin:?}");

    // The same reading typed as pinyin offers the same characters, in
    // the same order: the two lists share their index order.
    let mut p = Harness::new(PHONE);
    to_words(&mut p, Language::ChineseSimplified, 12);
    for c in "zhong1".chars() {
        p.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    let simplified = p.app.candidates();
    assert_eq!(simplified.len(), zhuyin.len(), "{simplified:?} {zhuyin:?}");
    for (i, w) in simplified.iter().enumerate() {
        let a = Language::ChineseSimplified
            .index_of(w)
            .expect("a Simplified word");
        let b = Language::ChineseTraditional
            .index_of(&zhuyin[i])
            .expect("a Traditional word");
        assert_eq!(a, b, "{w} and {} are the same word", zhuyin[i]);
    }
}

/// A reading whose tone leaves one character offers that character and
/// waits for the tap that takes it, as a word typed out in full does
/// (§16.118). `shuo1` is 说 on the Simplified list and 說 on the
/// Traditional one, and no other character of either list is read that
/// way.
#[test]
fn a_reading_that_leaves_one_character_waits_for_its_tap() {
    for (lang, keys, word) in [
        (Language::ChineseSimplified, "shuo1", "\u{8BF4}"),
        (
            Language::ChineseTraditional,
            "\u{3115}\u{3128}\u{311B}\u{02C9}",
            "\u{8AAA}",
        ),
    ] {
        let mut h = Harness::new(PHONE);
        to_words(&mut h, lang, 12);
        let mut typed = String::new();
        for c in keys.chars() {
            h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
            typed.push(c);
        }
        assert_eq!(
            h.app.candidates().len(),
            1,
            "{lang:?}: one character is left"
        );
        assert_eq!(
            h.app.load_words_accepted(),
            0,
            "{lang:?}: the tone took the character"
        );
        assert!(
            h.app.texts().contains(&typed),
            "{typed}: the reading is still on the field"
        );
        accept_candidate(&mut h, lang, 0);
        assert!(
            !h.app.texts().contains(&typed),
            "{typed}: the field is clear"
        );
        h.tap(ids::SECRET_EYE);
        assert!(
            h.app.texts().iter().any(|t| t.contains(word)),
            "{typed}: {word} is the first word of the key"
        );
    }
}

/// `yi` is the crowded syllable: no candidate until a tone is typed,
/// then the whole of the largest tone group either list has — twenty
/// characters, on the screen at once, with nothing to page to.
#[test]
fn a_crowded_syllable_shows_its_whole_tone_group() {
    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::ChineseSimplified, 12);
    for c in "yi".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    let live = |h: &Harness, c: char| {
        h.app
            .key_rect(ids::LOAD_KEYBOARD, KeyInput::Char(c))
            .is_some()
    };
    assert!(h.app.candidates().is_empty(), "yi is not yet a reading");
    assert!(!h.app.candidate_more(), "an empty strip does not page");
    for c in "1234".chars() {
        assert!(live(&h, c), "yi has a {c} tone");
    }
    assert!(!live(&h, '5'), "no character of the list is read yi5");
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('4'));
    let shown = h.app.candidates();
    assert_eq!(shown.len(), 20, "{shown:?}");
    assert_eq!(shown[0], "\u{4E00}", "the list's order puts 一 first");
    assert!(!h.app.candidate_more(), "the whole group is on the strip");
    // The twentieth character is taken by a tap like any other.
    let last = shown[19].clone();
    tap_candidate(&mut h, 19, 10);
    assert_eq!(h.app.candidates().len(), 0, "the word was accepted");
    h.tap(ids::SECRET_EYE);
    assert!(
        h.app.texts().iter().any(|t| t.contains(&last)),
        "{last} is the first word of the key"
    );
}

/// `shi4` is seventeen characters: the first row is full, the second
/// carries the seven that are left and no more, and nothing pages.
#[test]
fn a_group_that_fills_a_row_and_a_half_leaves_the_second_row_short() {
    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::ChineseSimplified, 12);
    for input in "shi4".chars().map(KeyInput::Char) {
        h.pad(ids::LOAD_KEYBOARD, input);
    }
    let shown = h.app.candidates();
    assert_eq!(shown.len(), 17, "{shown:?}");
    assert!(!h.app.candidate_more(), "seventeen fit two rows of ten");
    assert!(h.app.candidate_rect(ids::LOAD_CANDIDATES, 9).is_some());
    assert!(h.app.candidate_rect(ids::LOAD_CANDIDATES_2, 6).is_some());
    assert!(
        h.app.candidate_rect(ids::LOAD_CANDIDATES_2, 7).is_none(),
        "the second row stops where the group does"
    );
    // A character in the short row is taken by a tap.
    let want = shown[16].clone();
    tap_candidate(&mut h, 16, 10);
    assert_eq!(h.app.candidates().len(), 0, "the word was accepted");
    h.tap(ids::SECRET_EYE);
    assert!(h.app.texts().iter().any(|t| t.contains(&want)), "{want}");
}

/// A spelled list is unchanged: its cell is as wide as the widest word
/// it can hold, so the strip offers eight at a time and `a` leaves more
/// than that.
#[test]
fn a_spelled_list_still_offers_eight_words() {
    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::English, 12);
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('a'));
    let shown = h.app.candidates();
    assert_eq!(shown.len(), 8, "{shown:?}");
    assert_eq!(shown[0], "abandon");
    assert!(
        !h.app.candidate_more(),
        "a spelled word is narrowed by another key, not by a page"
    );
}

/// All ten wordlists are rows a person can choose, in each of the three
/// places the list is offered.
#[test]
fn the_chinese_rows_take_in_load_create_and_explore() {
    let simplified = language_row(Language::ChineseSimplified);
    let traditional = language_row(Language::ChineseTraditional);
    // The tone row of the pinyin keyboard and a bopomofo key say which
    // keyboard the wizard opened.
    let tone = KeyInput::Char('1');
    let bopomofo = KeyInput::Char('\u{3105}');

    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::ChineseSimplified, 12);
    for c in "zhong".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    assert!(h.app.key_rect(ids::LOAD_KEYBOARD, tone).is_some());

    let mut h = Harness::new(PHONE);
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 0),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_LANG_BASE, traditional),
        ids::CREATE_LANG_CONTINUE,
    );
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(CreateStep::Entropy));

    let mut h = Harness::new(PHONE);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_EXPLORER);
    h.tap(ids::EXPLORE_TYPE);
    h.tap(ids::SCAN_TYPE);
    h.choose(ids::at(ids::LOAD_COUNT_BASE, 0), ids::LOAD_COUNT_CONTINUE);
    h.choose(
        ids::at(ids::LOAD_LANG_BASE, traditional),
        ids::LOAD_LANG_CONTINUE,
    );
    assert_eq!(h.app.load_step(), Some(Step::Words));
    assert!(h.app.key_rect(ids::LOAD_KEYBOARD, bopomofo).is_some());
    assert_eq!(simplified + 1, traditional, "the two lists are neighbours");
}

/// A Traditional key made from dice: the words screen shows the
/// characters, and the quiz offers and takes them.
#[test]
fn a_traditional_key_is_created_read_and_quizzed() {
    let lang = Language::ChineseTraditional;
    let mut h = Harness::new(PHONE);
    h.open_create();
    h.choose(
        ids::at(ids::CREATE_SOURCE_BASE, 0),
        ids::CREATE_SOURCE_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_COUNT_BASE, 0),
        ids::CREATE_COUNT_CONTINUE,
    );
    h.choose(
        ids::at(ids::CREATE_LANG_BASE, language_row(lang)),
        ids::CREATE_LANG_CONTINUE,
    );
    h.tap(ids::CREATE_PROCEDURE_CONTINUE);
    h.type_text(RANDOM_50);
    h.key(Key::Enter);
    h.tap(ids::CREATE_CONTINUE);
    assert_eq!(h.app.create_step(), Some(CreateStep::Words));

    let mut dice = DiceRolls::new();
    for c in RANDOM_50.bytes() {
        dice.push(c - b'0');
    }
    let entropy = dice.entropy(Strength::Bits128).expect("128 bits");
    let m = Mnemonic::from_entropy(lang, entropy.as_bytes()).expect("a mnemonic");
    let words: Vec<String> = m
        .indices()
        .iter()
        .map(|&i| String::from(lang.word_display(i)))
        .collect();

    h.tap(ids::SECRET_EYE);
    let shown = h.app.texts().join(" ");
    assert!(
        shown.contains(words[0].as_str()),
        "{} is not on screen",
        words[0]
    );
    h.tap(ids::SECRET_EYE);
    h.tap(ids::CREATE_CONTINUE);
    h.tap(ids::QUIZ_START);
    for _ in 0..words.len() {
        let v = h.app.quiz_view().expect("a quiz on screen");
        assert_eq!(v.state, QuizState::Asking);
        let want = &words[v.word_number - 1];
        let slot = v
            .choices
            .iter()
            .position(|c| c == want)
            .unwrap_or_else(|| panic!("word {} not offered in {:?}", v.word_number, v.choices));
        h.choose(ids::at(ids::QUIZ_CHOICE_BASE, slot), ids::QUIZ_CONTINUE);
    }
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.backup_verified(0), Some(true));
    assert_eq!(
        h.app.fingerprints(),
        vec![
            MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Mainnet).fingerprint()
        ]
    );
}

/// On the 240 × 320 panel a candidate cell is 24 px and a fingertip
/// covers it, so a tap shows what it hit rather than taking it: the
/// first tap selects a candidate, the second accepts it, and a tap on
/// another cell moves the selection with nothing accepted.
#[test]
fn a_candidate_on_the_panel_is_selected_by_one_tap_and_accepted_by_the_next() {
    let mut h = Harness::new(PANEL);
    to_words(&mut h, Language::English, 12);
    for c in "ab".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    let offered = h.app.candidates();
    assert!(offered.len() > 1, "ab → several words: {offered:?}");

    // One tap: nothing is accepted and the strip still stands.
    tap_candidate(&mut h, 0, 3);
    assert_eq!(h.app.candidates(), offered, "the strip is unchanged");
    assert_eq!(h.app.load_words_accepted(), 0, "nothing was accepted");

    // A tap on another cell moves the selection; still nothing accepted.
    tap_candidate(&mut h, 1, 3);
    assert_eq!(h.app.candidates(), offered);
    assert_eq!(h.app.load_words_accepted(), 0);

    // The second tap on the selected cell accepts it.
    tap_candidate(&mut h, 1, 3);
    assert_eq!(h.app.load_words_accepted(), 1, "the word was accepted");
    assert!(h.app.candidates().is_empty(), "the next word starts empty");
    // What was accepted is the cell that was tapped twice: backspace
    // un-commits it into the field, and its own spelling is what shows.
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Backspace);
    assert!(
        h.app.texts().iter().any(|t| *t == offered[1]),
        "{} is not in the field",
        offered[1]
    );
}

/// A reading that leaves one character waits for its tap on every
/// class: a lone candidate is already selected where the strip is
/// tapped twice, so one tap takes it there as it does on the phone
/// (§16.118).
#[test]
fn a_lone_chinese_candidate_waits_for_a_tap() {
    for display in [PANEL, PHONE] {
        let mut h = Harness::new(display);
        to_words(&mut h, Language::ChineseSimplified, 12);
        for c in "le5".chars() {
            h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
        }
        assert_eq!(h.app.candidates().len(), 1, "le5 leaves one character");
        assert_eq!(h.app.load_words_accepted(), 0, "it waits for the tap");
        tap_candidate(&mut h, 0, 10);
        assert_eq!(h.app.load_words_accepted(), 1, "one tap takes it");
    }
}

/// On the phone a candidate is taken by the tap that hits it: the cell
/// is 6.5 mm and the finger does not hide it.
#[test]
fn a_candidate_on_the_phone_is_accepted_by_one_tap() {
    let mut h = Harness::new(PHONE);
    to_words(&mut h, Language::English, 12);
    for c in "ab".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    let offered = h.app.candidates();
    assert!(offered.len() > 1, "ab → several words: {offered:?}");
    tap_candidate(&mut h, 1, 4);
    assert_eq!(h.app.load_words_accepted(), 1);
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Backspace);
    assert!(h.app.texts().iter().any(|t| *t == offered[1]));
}

/// A key typed after a candidate was selected takes the selection with
/// it: the strip is a new list, and nothing is accepted that the typist
/// last saw somewhere else. Backspace does the same.
#[test]
fn a_key_clears_a_selected_candidate() {
    let mut h = Harness::new(PANEL);
    to_words(&mut h, Language::English, 12);
    for c in "ab".chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
    tap_candidate(&mut h, 0, 3);
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Backspace);
    h.pad(ids::LOAD_KEYBOARD, KeyInput::Char('b'));
    // The same cell as before: with the selection gone this tap only
    // selects, and the word is still not accepted.
    tap_candidate(&mut h, 0, 3);
    assert_eq!(h.app.load_words_accepted(), 0);
    tap_candidate(&mut h, 0, 3);
    assert_eq!(h.app.load_words_accepted(), 1);
}

/// Twelve words typed and taken from the strip make the key those words
/// make, on the panel and on the phone, on a spelled list and on a read
/// one.
#[test]
fn a_mnemonic_taken_from_the_strip_makes_its_key_on_both_classes() {
    for display in [PANEL, PHONE] {
        for lang in [Language::English, Language::ChineseSimplified] {
            let m = Mnemonic::from_entropy(lang, &[0x42u8; 16]).expect("a mnemonic");
            let indices: Vec<u16> = m.indices().to_vec();
            let mut h = Harness::new(display);
            to_words(&mut h, lang, 12);
            for &i in &indices {
                if lang == Language::ChineseSimplified {
                    type_hanzi(&mut h, lang, i);
                } else {
                    take_from_strip(&mut h, lang, i);
                }
            }
            assert_eq!(h.app.load_step(), Some(Step::Checksum), "{lang:?}");
            h.finish_load(None);
            assert_eq!(
                h.app.fingerprints(),
                vec![
                    MasterKey::from_seed(&m.to_seed(b"").expect("a seed"), Network::Mainnet)
                        .fingerprint()
                ],
                "{lang:?}"
            );
        }
    }
}

/// Types a spelled word letter by letter and takes it from the strip as
/// soon as it is offered.
fn take_from_strip(h: &mut Harness, lang: Language, idx: u16) {
    let want = String::from(lang.word_display(idx));
    for c in want.chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
        let shown = h.app.candidates();
        if shown.is_empty() {
            return; // the whole word was typed and it committed itself
        }
        if let Some(n) = shown.iter().position(|w| *w == want) {
            accept_candidate(h, lang, n);
            return;
        }
    }
    panic!("{want} was never on the strip");
}
