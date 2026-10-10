//! The Create vault Passphrases step: choosing one of the three EFF dice
//! lists changes the words the same rolls spell and starts the rolls
//! over, the passphrase fields stay masked until the person asks to see
//! them, a passphrase added by mistake can be taken back out, and a
//! passphrase the dice made has a strength only while it is unchanged. Every
//! passphrase field, Unlock's too, has an eye that shows what is typed.

use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday};
use osk_bip::diceware::List;
use osk_shell_api::App;

/// The Create vault form, open on the Passphrases step, rolling for
/// passphrase 0.
fn rolling() -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    app
}

#[test]
fn the_long_list_is_where_rolling_starts() {
    let app = rolling();
    let (_, _, list) = app.vaults.dice.as_ref().unwrap();
    assert_eq!(*list, List::Large);
}

#[test]
fn the_same_rolls_spell_different_words_on_different_lists() {
    let mut app = rolling();
    // Five dice: a whole word on the long list, one short of a short
    // list's four.
    for c in ['1', '2', '3', '4', '5'] {
        app.event(osk_shell_api::Event::Key(osk_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::DiceUse));
    let long_word = app.vaults.create.as_ref().unwrap().phrases[0]
        .0
        .text
        .clone();
    assert!(!long_word.is_empty());

    let mut app = rolling();
    app.press(Action::Vault(V::DiceList(1))); // Short1
    let (_, _, list) = app.vaults.dice.as_ref().unwrap();
    assert_eq!(*list, List::Short1);
    for c in ['1', '2', '3', '4'] {
        app.event(osk_shell_api::Event::Key(osk_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::DiceUse));
    let short_word = app.vaults.create.as_ref().unwrap().phrases[0]
        .0
        .text
        .clone();
    assert!(!short_word.is_empty());
    assert_ne!(
        *long_word, *short_word,
        "the lists must not share a word for these rolls"
    );
}

#[test]
fn dice_words_land_in_the_passphrase_field_with_no_spaces() {
    let mut app = rolling();
    for c in ['1', '2', '3', '4', '5', '6', '1', '2', '3', '4'] {
        app.event(osk_shell_api::Event::Key(osk_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::DiceUse));
    let phrase = app.vaults.create.as_ref().unwrap().phrases[0]
        .0
        .text
        .clone();
    assert!(
        !phrase.contains(' '),
        "a diceware passphrase is one unbroken string of words, not space-separated: {phrase:?}"
    );
}

#[test]
fn choosing_a_list_clears_rolls_already_typed() {
    let mut app = rolling();
    for c in ['1', '2', '3'] {
        app.event(osk_shell_api::Event::Key(osk_shell_api::Key::Char(c)));
    }
    assert_eq!(app.vaults.dice.as_ref().unwrap().1.text.len(), 3);
    app.press(Action::Vault(V::DiceList(1)));
    assert!(
        app.vaults.dice.as_ref().unwrap().1.text.is_empty(),
        "switching lists mid-roll must not spell a word under the wrong list"
    );
}

#[test]
fn passphrases_are_masked_until_shown() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Vault(V::Create));
    assert!(!app.vaults.create.as_ref().unwrap().shown);
    app.press(Action::Vault(V::CShow));
    assert!(app.vaults.create.as_ref().unwrap().shown);
    app.press(Action::Vault(V::CShow));
    assert!(!app.vaults.create.as_ref().unwrap().shown);
}

#[test]
fn a_second_passphrase_added_by_mistake_can_be_removed() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Vault(V::Create));
    assert_eq!(app.vaults.create.as_ref().unwrap().phrases.len(), 1);
    app.press(Action::Vault(V::CAddPhrase));
    assert_eq!(app.vaults.create.as_ref().unwrap().phrases.len(), 2);
    app.press(Action::Vault(V::CRemovePhrase(1)));
    assert_eq!(
        app.vaults.create.as_ref().unwrap().phrases.len(),
        1,
        "the flow would otherwise be stuck on an extra, empty passphrase"
    );
}

fn shown() -> Faraday {
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
    app
}

#[test]
fn unlock_has_an_eye_that_shows_the_passphrase_typed() {
    let mut app = shown();
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: vec![(
            "vault.ofv".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Vault(V::Open(0)));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::ShowTyped)), "no eye on Unlock");
    assert!(!app.vaults.typed_shown);
    app.press(Action::Vault(V::ShowTyped));
    assert!(app.vaults.typed_shown);
}

#[test]
fn create_vault_passphrase_fields_have_the_eye() {
    let mut app = shown();
    // Create opens on Name and passphrases.
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::CShow)));
}

#[test]
fn the_dice_panel_says_how_many_words_make_a_strong_passphrase() {
    use faraday_core::vaults::dice_aim;
    assert!(dice_aim(List::Large).starts_with("Aim for 6 words"));
    assert!(dice_aim(List::Short1).starts_with("Aim for 8 words"));
    assert!(dice_aim(List::Large).contains("eff.org/dice"));
}

#[test]
fn a_dice_passphrase_has_a_strength_until_it_is_changed() {
    use faraday_core::vaults::Vaults;
    use osk_shell_api::{Event, Key};
    let mut app = rolling();
    // Six words of the long list: thirty dice.
    for c in "123451234512345123451234512345".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::DiceUse));
    let c = app.vaults.create.as_ref().unwrap();
    let own = Vaults::phrase_bits(c, 0).expect("the dice's bits");
    assert!((own - 6.0 * List::Large.bits_per_word()).abs() < 1e-3);

    // Changed by hand, it is a typed passphrase: no strength is claimed.
    app.press(Action::Vault(V::CFocus(0, false)));
    app.event(Event::Key(Key::Char('x')));
    let c = app.vaults.create.as_ref().unwrap();
    assert!(c.phrases[0].0.text.ends_with('x'));
    assert_eq!(Vaults::phrase_bits(c, 0), None);
}
