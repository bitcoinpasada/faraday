//! Back and Forward within New key's steps (F104): Back opens the step
//! before with its choices as they were, Forward opens again the one Back
//! left, Make codex32 shares' own cards are steps too, a roller's click
//! under way takes no Back, and once the key is locked in Back goes no
//! further back than the step after Key.

use faraday_core::codex32::{C32, Card};
use faraday_core::keygen::{Start, Way, kstep};
use faraday_core::vault::RollerAct;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, EntropyBytes, Event};
use faraday_ui::keyboard::OskPress;

fn open(app: &Faraday) -> Option<u8> {
    app.keygen().and_then(|k| k.open)
}

fn button(app: &mut Faraday, e: Event) {
    let _ = app.frame();
    app.event(e);
    let _ = app.frame();
}

/// New key at 12 words on Rolls by the hashed dice procedure, chosen on
/// Randomness, which moves on by itself (F103).
fn on_rolls() -> Faraday {
    let mut app = crate::keygen::opened();
    app.press(Action::KStep(kstep::SOURCE));
    app.press(Action::KWay(Way::DiceHashed.index()));
    if open(&app) != Some(kstep::ENTER) {
        app.press(Action::KNext);
    }
    assert_eq!(open(&app), Some(kstep::ENTER));
    app
}

#[test]
fn back_opens_the_step_before_as_it_was_and_forward_opens_the_one_left() {
    let mut app = on_rolls();
    app.press(Action::KRoll(3));
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(kstep::SOURCE), "Randomness, before Rolls");
    assert_eq!(app.keygen().unwrap().way(), Some(Way::DiceHashed));
    button(&mut app, Event::Forward);
    assert_eq!(open(&app), Some(kstep::ENTER));
    assert_eq!(app.keygen().unwrap().dice.len(), 1, "the roll kept");
}

#[test]
fn escape_is_back_within_the_steps_and_leaves_from_start() {
    let mut app = on_rolls();
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(kstep::SOURCE));
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert_eq!(open(&app), Some(kstep::LENGTH));
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert_eq!(open(&app), Some(kstep::START));
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert_eq!(app.screen, Screen::Entry, "out to Add a key");
    assert!(app.keygen().is_none());
}

#[test]
fn make_codex32_shares_cards_are_steps_of_new_key() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(kstep::START));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    app.press(Action::Codex32(C32::Next));
    let card = |app: &Faraday| app.keygen().unwrap().codex32.as_ref().unwrap().open();
    assert_eq!(card(&app), Some(Card::Identifier));
    for ch in "name".chars() {
        app.press(Action::Codex32(C32::Key(OskPress::Char(ch))));
    }
    button(&mut app, Event::Back);
    assert_eq!(card(&app), Some(Card::Scheme));
    button(&mut app, Event::Forward);
    assert_eq!(card(&app), Some(Card::Identifier));
    assert_eq!(
        app.keygen().unwrap().codex32.as_ref().unwrap().identifier(),
        "name",
        "as it was typed"
    );
    button(&mut app, Event::Back);
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(kstep::START), "Scheme's Back is Start");
}

#[test]
fn a_rollers_click_is_done_at_once_so_back_goes_back() {
    let mut app = on_rolls();
    app.set_roller_offered(true);
    faraday_core::testkit::into_demo(&mut app);
    for i in 0..4u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i; 32])));
    }
    let before = app.keygen().unwrap().dice.len();
    app.press(Action::KRoller(RollerAct::Roll));
    assert_eq!(app.keygen().unwrap().dice.len(), before + 1, "rolled");
    button(&mut app, Event::Back);
    assert_eq!(open(&app), Some(kstep::SOURCE), "nothing under way");
}

#[test]
fn once_locked_in_back_from_words_leaves_new_key() {
    let mut app = crate::keygen::opened();
    app.press(Action::KStep(kstep::SOURCE));
    app.press(Action::KWay(Way::Device.index()));
    if open(&app) != Some(kstep::ENTER) {
        app.press(Action::KNext);
    }
    app.event(Event::Entropy(EntropyBytes::new([0x5a; 32])));
    app.press(Action::KNext);
    assert_eq!(open(&app), Some(kstep::KEY));
    app.press(Action::KLock);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(open(&app), Some(kstep::WORDS), "Lock in moves on");
    button(&mut app, Event::Back);
    assert_ne!(app.screen, Screen::KeyGen, "the key stays made: out");
}

/// How many times `text` is drawn on the screen.
fn drawn_count(app: &mut Faraday, text: &str) -> usize {
    let _ = app.frame();
    app.drawn_texts().iter().filter(|t| *t == text).count()
}

#[test]
fn the_arrow_names_the_entries_card_by_its_source() {
    use faraday_core::strings::EN;
    let mut app = crate::keygen::opened();
    app.press(Action::KStep(kstep::SOURCE));
    app.press(Action::KWay(Way::Device.index()));
    if open(&app) != Some(kstep::ENTER) {
        app.press(Action::KNext);
    }
    app.event(Event::Entropy(EntropyBytes::new([0x5a; 32])));
    app.press(Action::KNext);
    assert_eq!(open(&app), Some(kstep::KEY));
    assert_eq!(
        drawn_count(&mut app, EN.keygen_generator),
        1,
        "the card's title"
    );
    let back_to = format!("Back to {}", EN.keygen_generator);
    assert_eq!(drawn_count(&mut app, &back_to), 1, "the arrow back to it");
    assert_eq!(drawn_count(&mut app, EN.keygen_entries), 0);
}

#[test]
fn the_arrow_names_a_typed_form_by_the_form() {
    use faraday_core::forms::Form;
    use faraday_core::strings::EN;
    use faraday_shell_api::Key;
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(kstep::START));
    app.press(Action::KStart(Start::Codex32.index()));
    assert_eq!(open(&app), Some(kstep::TYPED));
    for share in [
        "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
        "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
    ] {
        for c in share.chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.event(Event::Key(Key::Enter));
    }
    if open(&app) != Some(kstep::KEY) {
        app.press(Action::KNext);
    }
    assert_eq!(open(&app), Some(kstep::KEY));
    assert_eq!(drawn_count(&mut app, Form::Codex32.name()), 1);
    let back_to = format!("Back to {}", Form::Codex32.name());
    assert_eq!(drawn_count(&mut app, &back_to), 1);
    assert_eq!(drawn_count(&mut app, EN.keygen_words), 0);
}
