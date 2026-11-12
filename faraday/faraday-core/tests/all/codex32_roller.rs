//! Faraday Demo's dice rollers in New key (`docs/CODEX32.md` §5.5,
//! §5.6): the five dice of the codex32 entry's "Demo only" card, a step
//! at a time, and the one die beside the dice pad for BIP-39 words. The
//! values fill the entry as the pad would, what the computer made is
//! marked as this computer's, the result says "simulated dice", and a
//! device shell has neither.

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, WalkAct, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::testkit::five::Reach;
use faraday_core::vault::RollerAct;
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

fn shows(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == text)
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
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
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
fn a_device_shell_has_no_die_beside_new_keys_rolls() {
    let mut app = on_bip39_dice(false);
    assert!(!shows(&mut app, "Rolled by this computer"));
    app.press(Action::KRoller(RollerAct::Roll));
    assert_eq!(app.keygen().unwrap().dice.len(), 0);
}
