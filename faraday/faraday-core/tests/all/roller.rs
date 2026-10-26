//! The dice roller of Faraday Demo beside Generate with dice
//! (`docs/CODEX32.md` §5.6): a roll fills the rolls as the keys would,
//! its values are marked as this computer's, Rolls per click, Reduce
//! motion, and the roller is not offered on a device shell.

use faraday_bip::diceware::List;
use faraday_core::vault::VaultAction as V;
use faraday_core::vault::{RollerAct as Act, RollerPerClick as PerClick};
use faraday_core::{Action, Faraday};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

/// Create a vault with the dice open on passphrase 0, on a shell that
/// offers the roller (or not), its randomness answered.
fn rolling(offered: bool, reduce: bool) -> Faraday {
    let mut app = faraday_core::testkit::started();
    app.set_roller_offered(offered);
    if offered {
        // The roller is Practice's alone (`docs/DEMO.md` §14).
        faraday_core::testkit::into_demo(&mut app);
    }
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x41 + i * 7; 32])));
    }
    if reduce {
        app.press(Action::ReduceMotion(true));
    }
    app.press(Action::Vault(V::Create));
    app.press(Action::Vault(V::Dice(0)));
    app
}

fn roller(a: Act) -> Action {
    Action::Vault(V::Roller(a))
}

fn rolls(app: &Faraday) -> String {
    app.vaults()
        .dice
        .as_ref()
        .map(|d| d.1.text.to_string())
        .unwrap_or_default()
}

fn tick(app: &mut Faraday, now_ms: u64) {
    app.event(Event::Tick { now_ms });
}

#[test]
fn a_roll_fills_the_rolls_as_the_keys_would() {
    let mut app = rolling(true, true);
    app.press(roller(Act::Per(PerClick::Five)));
    app.press(roller(Act::Roll));
    let made = rolls(&app);
    assert_eq!(made.len(), 5);
    assert!(made.bytes().all(|b| (b'1'..=b'6').contains(&b)));
    app.press(Action::Vault(V::DiceUse));
    let rolled_phrase = app.vaults().create.as_ref().unwrap().phrases[0]
        .0
        .text
        .to_string();

    // The same five values typed on the keys make the same passphrase.
    let mut typed = rolling(true, true);
    for c in made.chars() {
        typed.event(Event::Key(Key::Char(c)));
    }
    typed.press(Action::Vault(V::DiceUse));
    let typed_phrase = typed.vaults().create.as_ref().unwrap().phrases[0]
        .0
        .text
        .to_string();
    assert_eq!(rolled_phrase, typed_phrase);
    // In Practice, where the roller is offered, both are `faraday` in place of
    // the words the rolls make (`docs/DEMO.md` §5.2); the words are the
    // rolls' as typed.
    assert_eq!(rolled_phrase, faraday_core::demo::PASSPHRASE);
    assert_eq!(
        faraday_core::vault::dice_words(&made, List::Large).len(),
        1,
        "five rolls make one word of the long list"
    );
}

/// The roller is Practice's alone, and Practice puts `faraday` in place of the
/// words the dice make, so no strength line speaks of the computer's
/// dice: rolled, rolled with typed, or typed, the passphrase is `faraday`
/// and is not measured (`docs/DEMO.md` §5.2, §14).
#[test]
fn rolled_or_typed_demo_puts_faraday_in_place_and_measures_nothing() {
    let mut cases: Vec<Faraday> = Vec::new();
    // All rolled.
    let mut app = rolling(true, true);
    app.press(roller(Act::Per(PerClick::Five)));
    app.press(roller(Act::Roll));
    cases.push(app);
    // One typed, four rolled.
    let mut app = rolling(true, true);
    app.event(Event::Key(Key::Char('3')));
    app.press(roller(Act::Per(PerClick::Five)));
    app.press(roller(Act::Roll));
    assert_eq!(rolls(&app).len(), 6);
    app.event(Event::Key(Key::Backspace));
    cases.push(app);
    // Typed alone.
    let mut app = rolling(true, true);
    for c in "12345".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    cases.push(app);
    for mut app in cases {
        app.press(Action::Vault(V::DiceUse));
        let c = app.vaults().create.as_ref().unwrap();
        assert_eq!(c.phrases[0].0.text.as_str(), faraday_core::demo::PASSPHRASE);
        assert_eq!(faraday_core::vault::Vaults::phrase_bits(c, 0), None);
    }
}

#[test]
fn rolls_per_click_is_one_five_ten_a_typed_number_or_to_the_end() {
    for (per, want) in [(PerClick::One, 1), (PerClick::Five, 5), (PerClick::Ten, 10)] {
        let mut app = rolling(true, true);
        app.press(roller(Act::Per(per)));
        app.press(roller(Act::Roll));
        assert_eq!(rolls(&app).len(), want, "{per:?}");
    }
    // The default is one.
    let mut app = rolling(true, true);
    app.press(roller(Act::Roll));
    assert_eq!(rolls(&app).len(), 1);

    // A number typed.
    let mut app = rolling(true, true);
    app.press(roller(Act::Count));
    for c in ['7', 'x'] {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(roller(Act::Roll));
    assert_eq!(rolls(&app).len(), 7);

    // To the end: a passphrase of 77 bits on the long list, six words.
    let mut app = rolling(true, true);
    app.press(roller(Act::Per(PerClick::ToEnd)));
    app.press(roller(Act::Roll));
    let n = rolls(&app).len();
    assert_eq!(n, 30);
    let bits = List::Large.bits(n / List::Large.dice_per_word());
    assert!(bits >= faraday_core::vault::STRONG_BITS);
}

#[test]
fn reduce_motion_skips_the_tumble_and_the_pause() {
    // With motion: a click starts a tumble, nothing has landed yet, and
    // the values land one at a time, a pause apart.
    let mut app = rolling(true, false);
    app.press(roller(Act::Per(PerClick::Five)));
    let t0 = app.now_ms();
    app.press(roller(Act::Roll));
    assert!(rolls(&app).is_empty(), "no value before the tumble ends");
    tick(&mut app, t0 + 100);
    assert!(rolls(&app).is_empty());
    tick(&mut app, t0 + 1_000);
    assert_eq!(rolls(&app).len(), 1, "one lands when the tumble ends");
    tick(&mut app, t0 + 1_100);
    assert_eq!(rolls(&app).len(), 1, "and the next waits for its pause");
    for i in 1..=100u64 {
        tick(&mut app, t0 + 1_100 + i * 100);
    }
    assert_eq!(rolls(&app).len(), 5);

    // Reduce motion: all five at the click.
    let mut app = rolling(true, true);
    app.press(roller(Act::Per(PerClick::Five)));
    app.press(roller(Act::Roll));
    assert_eq!(rolls(&app).len(), 5);
}

#[test]
fn a_device_shell_has_no_roller() {
    let mut app = rolling(false, true);
    assert!(!app.roller_offered());
    app.press(roller(Act::Per(PerClick::Five)));
    app.press(roller(Act::Roll));
    assert!(rolls(&app).is_empty());
    // The pad's own entry is as it was.
    app.event(Event::Key(Key::Char('4')));
    assert_eq!(rolls(&app), "4");
}

#[test]
fn stop_ends_a_click_and_a_list_change_starts_the_rolls_over() {
    let mut app = rolling(true, false);
    app.press(roller(Act::Per(PerClick::Ten)));
    let t0 = app.now_ms();
    app.press(roller(Act::Roll));
    tick(&mut app, t0 + 1_000);
    assert_eq!(rolls(&app).len(), 1);
    app.press(roller(Act::Stop));
    tick(&mut app, t0 + 30_000);
    assert_eq!(rolls(&app).len(), 1);
    app.press(Action::Vault(V::DiceList(1)));
    assert!(rolls(&app).is_empty());
}
