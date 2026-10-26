//! Faraday Demo's dice rollers in New key (`docs/CODEX32.md` §5.5,
//! §5.6): the five dice of the codex32 entry's "Demo only" card, a step
//! at a time, and the one die beside the dice pad for BIP-39 words. The
//! values fill the entry as the pad would, what the computer made is
//! marked as this computer's, the result says "simulated dice", and a
//! device shell has neither.

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, WalkAct, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::testkit::five::Reach;
use faraday_core::vault::{RollerAct, RollerPerClick};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

/// The app with its randomness answered, on a shell that offers the
/// rollers (or not), Reduce motion on (or not).
fn app(offered: bool, reduce: bool) -> Faraday {
    let mut app = Faraday::new();
    // Tall enough for a whole card, the pad and the roller in view.
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.set_roller_offered(offered);
    if offered {
        // The roller is Practice's alone (`docs/DEMO.md` §14), with its
        // sample set, BIP 93's vector 2.
        faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
        app.session.set_demo(crate::codex32_rest::samples());
        app.samples = Some(crate::codex32_rest::samples());
    }
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x23 + i * 9; 32])));
    }
    if reduce {
        app.press(Action::ReduceMotion(true));
    }
    app
}

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

/// Play as far as `reach`, a step a frame, the shell answering each
/// request for randomness, until Play stops.
fn play(app: &mut Faraday, reach: Reach) {
    c32(app, C32::Walk(WalkAct::Reach(reach)));
    c32(app, C32::Walk(WalkAct::Speed(3)));
    c32(app, C32::Walk(WalkAct::Play));
    let mut t = app.now_ms();
    for i in 0..20_000u32 {
        app.event(Event::Entropy(EntropyBytes::new(
            [(i as u8).wrapping_mul(37); 32],
        )));
        t += 16;
        app.event(Event::Tick { now_ms: t });
        if !make(app).walk().playing() {
            break;
        }
    }
}

/// New key on its own, Make a new key's Codex32 at 2 of 3, `name`, on
/// the entry by dice.
fn on_dice_entry(app: &mut Faraday) {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(kstep::START));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(split as u8));
    c32(app, C32::Next);
    for ch in "name".chars() {
        c32(app, C32::Key(OskPress::Char(ch)));
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == Way::Dice).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, C32::Next);
    assert_eq!(make(app).open(), Some(Card::Enter));
}

fn make(app: &Faraday) -> &faraday_core::codex32::Codex32Make {
    app.keygen().unwrap().codex32.as_ref().unwrap()
}

/// Characters entered into share `s`.
fn entered(app: &Faraday, s: usize) -> usize {
    make(app).work().map_or(0, |w| w.entered(s))
}

/// From the finished entry to Strings made, each string's typing back
/// skipped.
fn to_made(app: &mut Faraday) {
    c32(app, C32::Next);
    let _ = app.frame();
    c32(app, C32::Next);
    for _ in 0..12 {
        if make(app).open() == Some(Card::Made) {
            break;
        }
        let _ = app.frame();
        c32(app, C32::Skip);
    }
    assert_eq!(make(app).open(), Some(Card::Made), "{:?}", make(app).note());
}

fn shows(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == text)
}

/// Whether Faraday Demo's host strip offers `text` for the app as it
/// stands (`docs/DESKTOP-HOST.md` §3.1).
fn in_strip(app: &mut Faraday, text: &str) -> bool {
    crate::common::strip(app).iter().any(|t| t == text)
}

#[test]
fn the_five_dice_are_offered_beside_your_dice_on_faraday_desktop_only() {
    let mut desk = app(true, true);
    on_dice_entry(&mut desk);
    assert!(in_strip(&mut desk, "Next") && in_strip(&mut desk, "Play"));
    assert!(shows(&mut desk, "Your dice"), "the pad for real dice stays");
    // Where the walk stands is the Working area's, beside the pad.
    assert!(shows(&mut desk, "Character 1"));
    assert!(
        !shows(&mut desk, "Next rolls character 1"),
        "not the card's"
    );

    let mut stick = app(false, true);
    on_dice_entry(&mut stick);
    assert!(!in_strip(&mut stick, "Next"));
    // A press that reaches a device shell anyway rolls nothing.
    c32(&mut stick, C32::Walk(WalkAct::Next));
    assert_eq!(make(&stick).seq_len(), 0);
}

#[test]
fn on_faraday_desktop_in_testnet_the_dice_step_offers_no_demo_aid() {
    // Faraday Demo, its roller set, but in Testnet: Practice is the one
    // condition for every aid (owner, 2026-10-24).
    let mut test = app(false, true);
    test.set_roller_offered(true);
    faraday_core::testkit::move_to(&mut test.session, faraday_core::testkit::NET).unwrap();
    on_dice_entry(&mut test);
    let s = crate::common::strip(&mut test);
    assert!(
        !s.iter()
            .any(|t| t == "Practice" || t == "Next" || t == "Use a sample"),
        "no aid: {s:?}"
    );
    assert!(shows(&mut test, "Your dice"), "the person's own dice stay");

    // In Practice the same step has them.
    let mut demo = app(true, true);
    on_dice_entry(&mut demo);
    assert!(in_strip(&mut demo, "Next") && in_strip(&mut demo, "Use a sample"));
}

#[test]
fn next_shows_a_step_at_a_time_and_marks_the_computers_dice() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    c32(&mut app, C32::Walk(WalkAct::Next));
    assert_eq!(make(&app).seq_len(), 1, "one face, die 1's first roll");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.starts_with("Step 1 of ")),
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t.starts_with("Die 1, first roll: ")));
    for _ in 0..4 {
        c32(&mut app, C32::Walk(WalkAct::Next));
    }
    assert!(
        shows(
            &mut app,
            "Die 1, Die 2, Die 3, Die 4, Die 5 rolled by this computer"
        ),
        "the first roll's five values, marked"
    );
    // Backspace takes the last value back: it is not the roller's, and
    // the walk shows the step it went back to, as Previous would
    // (owner, 2026-10-26).
    c32(&mut app, C32::Key(OskPress::Back));
    assert!(shows(
        &mut app,
        "Die 1, Die 2, Die 3, Die 4 rolled by this computer"
    ));
    assert!(matches!(make(&app).walk().shown(), Some((_, 4, _))));
    assert!(make(&app).walk_can_back(), "Previous still has a step");
}

#[test]
fn the_arrow_keys_step_through_on_faraday_desktop() {
    use faraday_shell_api::Key;
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    app.event(Event::Key(Key::Right));
    app.event(Event::Key(Key::Right));
    assert_eq!(make(&app).seq_len(), 2);
    app.event(Event::Key(Key::Left));
    assert_eq!(make(&app).seq_len(), 1);
}

#[test]
fn play_to_the_end_makes_the_strings_and_demo_puts_its_sample_in_their_place() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    play(&mut app, Reach::End);
    assert!(make(&app).strings().is_some(), "every share entered");
    assert!(make(&app).front().is_none());
    to_made(&mut app);
    // The roller is Practice's alone, so what the computer rolled never
    // leaves the entry (`docs/CODEX32.md` §12).
    assert!(make(&app).sample());
    assert!(in_strip(
        &mut app,
        "Practice puts sample NAME in place of these strings"
    ));
}

#[test]
fn faces_from_both_finish_the_entry_and_demo_puts_its_sample_in_their_place() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    // One character by hand: each die 4 then 3.
    for f in [4, 4, 4, 4, 4, 3, 3, 3, 3, 3] {
        c32(&mut app, C32::Key(OskPress::Char(char::from(b'0' + f))));
    }
    assert_eq!(entered(&app, 0), 1);
    assert!(!make(&app).char_rolled(0, 0), "a real die's, not marked");
    play(&mut app, Reach::End);
    to_made(&mut app);
    assert!(make(&app).sample());
    assert!(in_strip(
        &mut app,
        "Practice puts sample NAME in place of these strings"
    ));
}

#[test]
fn play_one_character_stops_after_its_last_step() {
    let mut app = app(true, false);
    on_dice_entry(&mut app);
    play(&mut app, Reach::Character);
    assert_eq!(entered(&app, 0), 1, "the character is in its box");
    let (ch, s, _) = make(&app).walk().shown().unwrap();
    assert_eq!(s, ch.steps().len(), "its last step shown");
    assert!(!make(&app).walk().playing());
}

/// New key opened on Add a key, 12 words, dice by the hashed procedure,
/// on the entries.
fn on_bip39_dice(offered: bool) -> Faraday {
    let mut app = app(offered, true);
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KNext);
    app.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    if app.keygen().unwrap().open != Some(kstep::ENTER) {
        app.press(Action::KNext);
    }
    assert_eq!(app.keygen().unwrap().open, Some(kstep::ENTER));
    app
}

#[test]
fn the_one_die_fills_new_keys_rolls_and_the_card_says_simulated_dice() {
    let mut app = on_bip39_dice(true);
    assert!(in_strip(&mut app, "Roll"), "the die in the strip");
    app.press(Action::KRoller(RollerAct::Per(RollerPerClick::Five)));
    app.press(Action::KRoller(RollerAct::Roll));
    assert_eq!(app.keygen().unwrap().dice.len(), 5);
    app.press(Action::KRoller(RollerAct::Per(RollerPerClick::ToEnd)));
    app.press(Action::KRoller(RollerAct::Roll));
    let k = app.keygen().unwrap();
    assert_eq!(k.progress().0, k.progress().1, "the rolls are full");
    app.press(Action::KNext);
    assert!(shows(&mut app, "Entropy · simulated dice"));
}

#[test]
fn a_device_shell_has_no_die_beside_new_keys_rolls() {
    let mut app = on_bip39_dice(false);
    assert!(!shows(&mut app, "Rolled by this computer"));
    app.press(Action::KRoller(RollerAct::Roll));
    assert_eq!(app.keygen().unwrap().dice.len(), 0);
}
