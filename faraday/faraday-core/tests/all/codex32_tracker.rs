//! The Working area of the codex32 entry (`docs/CODEX32.md` §5.7): the
//! person's own values, each with its step, beside the pad, as a person
//! entering their dice would notice it; outside Practice nothing rolls.

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, WalkAct, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

fn on(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app
}

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn key(app: &mut Faraday, ch: char) {
    c32(app, C32::Key(OskPress::Char(ch)));
}

/// New key, Codex32, 2 of 3, `name`, Roll dice: the entry open.
fn to_entry(app: &mut Faraday) {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(kstep::START));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(app, C32::Split(split as u8));
    c32(app, C32::Next);
    for ch in "name".chars() {
        key(app, ch);
    }
    c32(app, C32::Next);
    let w = WAYS.iter().position(|w| *w == Way::Dice).unwrap();
    c32(app, C32::Way(w as u8));
    c32(app, C32::Next);
    let m = app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.open(), Some(Card::Enter));
}

fn drawn(app: &mut Faraday, part: &str) -> bool {
    app.drawn_texts().iter().any(|t| t.contains(part))
}

#[test]
fn each_value_entered_shows_its_step_beside_the_pad_at_every_size() {
    for (w, h) in [(1280, 800), (1920, 1080), (480, 640)] {
        let mut app = on(w, h);
        to_entry(&mut app);
        // The small panel shows it as its context's fourth tile.
        c32(&mut app, C32::Panel(3));
        assert!(drawn(&mut app, "Next: die 1, first roll."), "{w}x{h}");
        key(&mut app, '1');
        assert!(
            drawn(&mut app, "Die 1, first roll: 1. Marker on pad 1."),
            "{w}x{h}"
        );
        // Vector 2's share A's first character, 3: bits 1 0 0 0 1.
        for f in "222121112".chars() {
            key(&mut app, f);
        }
        assert!(drawn(&mut app, "Bits 1 0 0 0 1"), "{w}x{h}");
        assert!(app.offers(Action::Codex32(C32::Step(1))), "{w}x{h}");
    }
}

#[test]
fn on_the_pi_panel_the_working_tile_stands_over_the_pad() {
    let mut pi = Faraday::new();
    pi.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    for i in 0..8u8 {
        pi.event(Event::Entropy(EntropyBytes::new([0x50 + i; 32])));
    }
    to_entry(&mut pi);
    c32(&mut pi, C32::Panel(3));
    key(&mut pi, '6');
    key(&mut pi, '4');
    let boxes = pi.drawn_text_boxes();
    let line = boxes
        .iter()
        .find(|(t, _)| t.starts_with("Die 2, first roll: 4."))
        .map(|b| b.1)
        .expect("the step is drawn");
    let pad = pi
        .hit_box(Action::Codex32(C32::Key(OskPress::Char('3'))))
        .expect("the pad's 3");
    assert!(line.1 >= 0 && line.1 < i32::from(pad.1), "{line:?} {pad:?}");
    let back = pi.hit_box(Action::Codex32(C32::Step(-1)));
    assert!(back.is_some_and(|b| b.1 + b.3 <= pad.1), "{back:?} {pad:?}");
}

#[test]
fn outside_demo_the_working_area_steps_but_never_rolls() {
    let mut app = on(1280, 800);
    to_entry(&mut app);
    for f in "1222121112".chars() {
        key(&mut app, f);
    }
    for act in [WalkAct::Next, WalkAct::Previous, WalkAct::Play] {
        assert!(!app.offers(Action::Codex32(C32::Walk(act))), "{act:?}");
    }
    assert!(!app.drawn_texts().iter().any(|t| t == "Play"));
    let entered = |app: &Faraday| {
        let m = app.keygen().unwrap().codex32.as_ref().unwrap();
        (m.work().unwrap().entered(0), m.seq_len())
    };
    let before = entered(&app);
    for _ in 0..8 {
        c32(&mut app, C32::Step(1));
    }
    assert!(drawn(&mut app, "Box 10, row g: "));
    assert_eq!(entered(&app), before, "Next enters nothing");
}
