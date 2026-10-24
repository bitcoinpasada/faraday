//! Faraday Desktop's dice rollers in New key (`docs/CODEX32.md` §5.5,
//! §5.6): the five dice beside the codex32 dice pad and the one die
//! beside the dice pad for BIP-39 words. A click fills the entry as the
//! pad would, what the roller made is marked as this computer's, the
//! result says "simulated dice", and a device shell has no roller.

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::testkit::five::{Act, Reach};
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

/// Roll, and the ticks a long click takes: under Reduce motion the
/// faces land a batch a frame, and the shell answers each request for
/// randomness.
fn roll(app: &mut Faraday) {
    c32(app, C32::Roller(Act::Roll));
    let mut t = app.now_ms();
    for i in 0..60u8 {
        app.event(Event::Entropy(EntropyBytes::new([i.wrapping_mul(37); 32])));
        t += 16;
        app.event(Event::Tick { now_ms: t });
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
    assert_eq!(make(app).open(), Some(Card::Made));
}

fn shows(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == text)
}

#[test]
fn the_five_dice_stand_beside_your_dice_on_faraday_desktop_only() {
    let mut desk = app(true, true);
    on_dice_entry(&mut desk);
    assert!(shows(&mut desk, "Or let Faraday roll"));
    assert!(shows(&mut desk, "Your dice"), "the pad for real dice stays");
    assert!(desk.offers(Action::Codex32(C32::Roller(Act::Roll))));

    let mut stick = app(false, true);
    on_dice_entry(&mut stick);
    assert!(!shows(&mut stick, "Or let Faraday roll"));
    // A press that reaches a device shell anyway rolls nothing.
    c32(&mut stick, C32::Roller(Act::Roll));
    assert_eq!(entered(&stick, 0), 0);
}

#[test]
fn one_click_rolls_one_character_and_one_pass_marks_its_dice() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    c32(&mut app, C32::Roller(Act::Roll));
    assert_eq!(entered(&app, 0), 1, "One character, the default");
    assert!(make(&app).char_rolled(0, 0));

    c32(&mut app, C32::Roller(Act::Reach(Reach::Pass)));
    c32(&mut app, C32::Roller(Act::Roll));
    assert!(
        shows(
            &mut app,
            "Die 1, Die 2, Die 3, Die 4, Die 5 rolled by this computer"
        ),
        "the first roll's five values, marked"
    );
    // Backspace takes the last value back: it is not the roller's.
    c32(&mut app, C32::Key(OskPress::Back));
    assert!(shows(
        &mut app,
        "Die 1, Die 2, Die 3, Die 4 rolled by this computer"
    ));
}

#[test]
fn to_the_end_makes_the_strings_and_strings_made_says_simulated_dice() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    c32(&mut app, C32::Roller(Act::Reach(Reach::End)));
    roll(&mut app);
    assert!(make(&app).strings().is_some(), "every share entered");
    assert!(make(&app).front().is_none());
    to_made(&mut app);
    assert!(shows(&mut app, "simulated dice"));
}

#[test]
fn faces_from_both_say_simulated_dice_and_your_dice() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    // One character by hand: each die 4 then 3.
    for f in [4, 4, 4, 4, 4, 3, 3, 3, 3, 3] {
        c32(&mut app, C32::Key(OskPress::Char(char::from(b'0' + f))));
    }
    assert_eq!(entered(&app, 0), 1);
    assert!(!make(&app).char_rolled(0, 0), "a real die's, not marked");
    c32(&mut app, C32::Roller(Act::Reach(Reach::End)));
    roll(&mut app);
    to_made(&mut app);
    assert!(shows(&mut app, "simulated dice and your dice"));
}

#[test]
fn without_reduce_motion_the_dice_tumble_then_land() {
    let mut app = app(true, false);
    on_dice_entry(&mut app);
    c32(&mut app, C32::Roller(Act::Roll));
    assert_eq!(entered(&app, 0), 0, "nothing lands at the click");
    let start = app.now_ms();
    let mut t = start;
    while entered(&app, 0) == 0 && t < start + 20_000 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
    }
    assert_eq!(entered(&app, 0), 1, "the character lands by the ticks");
    assert!(!make(&app).roller().rolling());
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
    assert!(shows(&mut app, "Rolled by this computer"));
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
