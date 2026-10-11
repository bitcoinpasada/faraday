//! Words typed from another BIP-39 list: each word typed as its ASCII
//! fold (`ábaco` as `abaco`), the key the one that list's words give, not
//! the English words of the same entropy. Japanese, Korean and both
//! Chinese lists are typed on keyboards of their own, OpenSigner's, and
//! a word is taken by pressing it among those the keys can be.

use faraday_core::{Action, Faraday};
use osk_bip::bip39::{Language, Mnemonic};
use osk_shell_api::{App, Event, Key};

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
    let mut probe = faraday_core::wallet::Session::default();
    probe.add_mnemonic(m, "", "", None).unwrap().0
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
    assert_eq!(app.session.keys.len(), 1, "{:?}", app.entry.error);
    let key = &app.session.keys[0];
    assert_eq!(key.master.fingerprint().0, fingerprint(&es));
    assert_ne!(key.master.fingerprint().0, fingerprint(&en));
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
        app.entry
            .error
            .as_deref()
            .is_some_and(|e| e.contains("French"))
    );
}

#[test]
fn the_other_lists_are_behind_one_button_until_asked_for() {
    let mut app = faraday_core::testkit::started();
    app.event(osk_shell_api::Event::Display(osk_shell_api::DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
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
    use osk_bip::bip39::Script;
    use osk_ui::widgets::keyboard::ZHUYIN_MARKS;
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
    let before = app.entry.keys.as_ref().unwrap().committed_indices().count();
    for c in keys {
        app.press(Action::EntryKey(c));
    }
    let w = app.entry.keys.as_ref().unwrap();
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
            app.entry.keys.is_some(),
            "{lang:?} has a keyboard of its own"
        );
        for &i in m.indices() {
            type_on_keys(&mut app, lang, i);
        }
        let typed: Vec<u16> = app
            .entry
            .keys
            .as_ref()
            .unwrap()
            .committed_indices()
            .collect();
        assert_eq!(&typed[..], m.indices(), "{lang:?}");
        app.press(Action::EntryAdd);
        assert_eq!(app.session.keys.len(), 1, "{lang:?}: {:?}", app.entry.error);
        let key = &app.session.keys[0];
        assert_eq!(key.master.fingerprint().0, fingerprint(&m), "{lang:?}");
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
    assert!(app.entry.keys.as_ref().unwrap().prefix().is_empty());
    // "shi" and a tone are, typed on the computer's keyboard.
    for c in "shi4".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert!(app.entry.keys.as_ref().unwrap().candidates().count() > 1);
}
