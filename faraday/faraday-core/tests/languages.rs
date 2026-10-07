//! Words typed from another BIP-39 list: each word typed as its ASCII
//! fold (`ábaco` as `abaco`), the key the one that list's words give, not
//! the English words of the same entropy.

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
    let mut app = Faraday::new();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LATIN
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
    let mut app = Faraday::new();
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LATIN
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
    let mut app = Faraday::new();
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
