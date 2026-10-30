//! Words typed from another BIP-39 list: each word typed as its ASCII
//! fold (`ábaco` as `abaco`), the key the one that list's words give, not
//! the English words of the same entropy. Japanese, Korean and both
//! Chinese lists are typed on keyboards of their own, and
//! a word is taken by pressing it among those the keys can be.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::{Action, Faraday};
use faraday_shell_api::{App, Event, Key};

fn type_folded(app: &mut Faraday, m: &Mnemonic) {
    let lang = m.language();
    for (k, &i) in m.indices().iter().enumerate() {
        if k > 0 {
            app.event(Event::Key(Key::Char(' ')));
        }
        for c in lang.typed(i).unwrap().as_chars() {
            app.event(Event::Key(Key::Char(*c)));
        }
    }
}

fn fingerprint(m: &Mnemonic) -> [u8; 4] {
    let c = faraday_core::testkit::compute();
    let mut probe = faraday_core::wallet::Session::default();
    probe.keys.add_mnemonic(&c, m, "", "", None).unwrap().0
}

#[test]
fn spanish_words_typed_without_accents_make_the_spanish_key() {
    // Entropy whose Spanish words carry accents.
    let entropy: Vec<u8> = (0u8..16)
        .map(|i| i.wrapping_mul(37).wrapping_add(11))
        .collect();
    let es = Mnemonic::from_entropy(Language::Spanish, &entropy).unwrap();
    let en = Mnemonic::from_entropy(Language::English, &entropy).unwrap();
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LANGUAGES
        .iter()
        .position(|l| *l == Language::Spanish)
        .unwrap();
    app.press(Action::EntryLanguage(at as u8));
    type_folded(&mut app, &es);
    app.press(Action::EntryAdd);
    assert_eq!(
        app.session.keys.len(),
        1,
        "{:?}",
        faraday_core::testkit::entry(&app).error
    );
    let key = &app.session.keys[0];
    assert_eq!(key.fingerprint.0, fingerprint(&es));
    assert_ne!(key.fingerprint.0, fingerprint(&en));
    assert_eq!(key.language, Language::Spanish);
}

#[test]
fn a_word_from_another_list_is_named_as_not_on_this_one() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LANGUAGES
        .iter()
        .position(|l| *l == Language::French)
        .unwrap();
    app.press(Action::EntryLanguage(at as u8));
    for c in "zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra zebra".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(app.session.keys.is_empty());
    assert!(
        faraday_core::testkit::entry(&app)
            .error
            .as_deref()
            .is_some_and(|e| e.contains("French"))
    );
}

#[test]
fn the_other_lists_are_behind_one_button_until_asked_for() {
    let mut app = faraday_core::testkit::started();
    app.event(faraday_shell_api::Event::Display(
        faraday_shell_api::DisplayInfo {
            width: 1366,
            height: 768,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        },
    ));
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert!(app.offers(Action::EntryLanguages));
    assert!(
        !app.offers(Action::EntryLanguage(1)),
        "Spanish is shown before asking"
    );
    app.press(Action::EntryLanguages);
    let _ = app.frame();
    assert!(app.offers(Action::EntryLanguage(1)));
}

/// Types word `i` of `lang` on the list's own keyboard, as a person
/// would: its kana or jamo key by key, or its reading and then its tone;
/// then presses it among the words offered.
fn type_on_keys(app: &mut Faraday, lang: Language, i: u16) {
    use faraday_bip::bip39::Script;
    use faraday_draw::widgets::keyboard::ZHUYIN_MARKS;
    let keys: Vec<char> = match lang.script() {
        Script::Pinyin | Script::Zhuyin => {
            let (spelling, tone) = lang.word_readings(i).next().unwrap();
            let mut k: Vec<char> = spelling.chars().collect();
            k.push(if lang.script() == Script::Pinyin {
                char::from(b'0' + tone)
            } else {
                ZHUYIN_MARKS[usize::from(tone) - 1]
            });
            k
        }
        _ => lang.typed(i).unwrap().as_chars().to_vec(),
    };
    let before = faraday_core::testkit::entry(app)
        .keys
        .as_ref()
        .unwrap()
        .committed_indices()
        .count();
    for c in keys {
        app.press(Action::EntryKey(c));
    }
    let w = faraday_core::testkit::entry(app).keys.as_ref().unwrap();
    // A word with no other word starting the same way is taken as its
    // last key is typed, with no word pressed.
    if w.committed_indices().count() > before {
        assert_eq!(w.committed_indices().last(), Some(i));
        return;
    }
    let at = w
        .candidates()
        .position(|c| c == i)
        .unwrap_or_else(|| panic!("{} offered", lang.word(i)));
    app.press(Action::EntryCandidate(at as u8));
}

#[test]
fn words_typed_on_the_kana_jamo_pinyin_and_zhuyin_keyboards_make_that_list_s_key() {
    let entropy: Vec<u8> = (0u8..16)
        .map(|i| i.wrapping_mul(53).wrapping_add(7))
        .collect();
    for lang in [
        Language::Japanese,
        Language::Korean,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
    ] {
        let m = Mnemonic::from_entropy(lang, &entropy).unwrap();
        let mut app = faraday_core::testkit::started();
        app.press(Action::Entry(None));
        let at = faraday_core::forms::LANGUAGES
            .iter()
            .position(|l| *l == lang)
            .unwrap();
        app.press(Action::EntryLanguage(at as u8));
        assert!(
            faraday_core::testkit::entry(&app).keys.is_some(),
            "{lang:?} has a keyboard of its own"
        );
        for &i in m.indices() {
            type_on_keys(&mut app, lang, i);
        }
        let typed: Vec<u16> = faraday_core::testkit::entry(&app)
            .keys
            .as_ref()
            .unwrap()
            .committed_indices()
            .collect();
        assert_eq!(&typed[..], m.indices(), "{lang:?}");
        app.press(Action::EntryAdd);
        assert_eq!(
            app.session.keys.len(),
            1,
            "{lang:?}: {:?}",
            faraday_core::testkit::entry(&app).error
        );
        let key = &app.session.keys[0];
        assert_eq!(key.fingerprint.0, fingerprint(&m), "{lang:?}");
        assert_eq!(key.language, lang);
    }
}

#[test]
fn a_key_that_leads_to_no_word_is_not_taken() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LANGUAGES
        .iter()
        .position(|l| *l == Language::ChineseSimplified)
        .unwrap();
    app.press(Action::EntryLanguage(at as u8));
    // No pinyin syllable starts with "v".
    app.event(Event::Key(Key::Char('v')));
    assert!(
        faraday_core::testkit::entry(&app)
            .keys
            .as_ref()
            .unwrap()
            .prefix()
            .is_empty()
    );
    // "shi" and a tone are, typed on the computer's keyboard.
    for c in "shi4".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(
        faraday_core::testkit::entry(&app)
            .keys
            .as_ref()
            .unwrap()
            .candidates()
            .count()
            > 1
    );
}

/// The words typed on a list's own keyboard: a press on a word taken
/// earlier opens it, its keys back in its cell; a key taken off it and
/// another word pressed among those its keys can be, that word stands in
/// its place and the words after it stay as they were; Left and Right
/// move the caret within its keys and on into the next cell
/// (`docs/TEXT-CURSOR.md`).
#[test]
fn a_word_taken_earlier_is_edited_in_its_cell_and_the_rest_kept() {
    let entropy: Vec<u8> = (0u8..16)
        .map(|i| i.wrapping_mul(29).wrapping_add(3))
        .collect();
    let lang = Language::Korean;
    let m = Mnemonic::from_entropy(lang, &entropy).unwrap();
    let mut app = faraday_core::testkit::started();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LANGUAGES
        .iter()
        .position(|l| *l == lang)
        .unwrap();
    app.press(Action::EntryLanguage(at as u8));
    let first: Vec<u16> = m.indices()[..3].to_vec();
    for &i in &first {
        type_on_keys(&mut app, lang, i);
    }
    let words = |app: &Faraday| {
        let w = faraday_core::testkit::entry(app).keys.as_ref().unwrap();
        (w.committed_indices().collect::<Vec<_>>(), w.is_open())
    };
    // Left from the empty cell opens the third word; Right past its end
    // takes it again, unchanged.
    app.event(Event::Key(Key::Left));
    assert_eq!(words(&app), (first[..2].to_vec(), true));
    app.event(Event::Key(Key::Right));
    assert_eq!(words(&app), (first.clone(), false));
    let _ = app.frame();
    // The second cell's end: cell 1, the cells 64 places apart in the
    // press's numbering, its end the last of them.
    let here = Action::TypeHere(faraday_core::edit::Here::Keyed);
    let (x, y) = app.where_typed(here, 64 + 63).expect("the cells drawn");
    for phase in [
        faraday_shell_api::TouchPhase::Down,
        faraday_shell_api::TouchPhase::Up,
    ] {
        app.event(Event::Touch { x, y, phase });
    }
    assert_eq!(words(&app), (first[..1].to_vec(), true), "the word opened");
    // Keys taken off until another word starts as they do.
    let others = |app: &Faraday| {
        let w = faraday_core::testkit::entry(app).keys.as_ref().unwrap();
        w.candidates().any(|c| c != first[1])
    };
    while !others(&app) {
        app.event(Event::Key(Key::Backspace));
    }
    let w = faraday_core::testkit::entry(&app).keys.as_ref().unwrap();
    let other = w
        .candidates()
        .position(|c| c != first[1])
        .expect("another word");
    let picked = w.candidates().nth(other).unwrap();
    app.press(Action::EntryCandidate(other as u8));
    assert_eq!(words(&app), (vec![first[0], picked, first[2]], false));
}
