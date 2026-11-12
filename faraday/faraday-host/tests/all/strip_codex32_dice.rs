//! The strip's side of the codex32_dice tests: what Faraday Demo's host offers there.

use faraday_core::codex32::WalkAct;
use faraday_core::codex32::{C32, Card, DIE_SIZES, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::Start;
use faraday_core::testkit::five::Reach;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn key(app: &mut Faraday, ch: char) {
    c32(app, C32::Key(OskPress::Char(ch)));
}

fn make(app: &Faraday) -> &faraday_core::codex32::Codex32Make {
    app.keygen().unwrap().codex32.as_ref().unwrap()
}

/// The app on Faraday Demo (the roller offered) or the stick, Reduce
/// motion on, on the entry by dice of `faces` faces.
fn desktop(offered: bool, faces: u8) -> Faraday {
    let mut app = on(1366, 4000);
    app.set_roller_offered(offered);
    if offered {
        // The roller is Practice's alone (`docs/DEMO.md` §14).
        faraday_core::testkit::into_demo(&mut app);
    }
    app.press(Action::ReduceMotion(true));
    to_entry(&mut app, faces);
    app
}

fn on(width: u16, height: u16) -> Faraday {
    on_at(width, height, 160)
}

/// The app on a display of `width` × `height` pixels at `dpi`.
fn on_at(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
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
    app
}

/// The die of `faces` faces, as the Entropy card offers it.
fn die(faces: u8) -> C32 {
    C32::Faces(DIE_SIZES.iter().position(|&n| n == faces).unwrap() as u8)
}

/// New key, Codex32, 2 of 3, `name`, Roll dice on dice of `faces` faces:
/// the entry open.
fn to_entry(app: &mut Faraday, faces: u8) {
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
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
    c32(app, die(faces));
    c32(app, C32::Next);
    assert_eq!(make(app).open(), Some(Card::Enter));
}

#[test]
fn the_computers_dice_are_offered_in_the_host_strip_on_faraday_desktop_alone() {
    let mut app = desktop(true, 6);
    // The person's own entry in the app; the computer's walk outside it.
    assert!(app.drawn_texts().iter().any(|t| t == "Your dice"));
    assert!(!app.drawn_texts().iter().any(|t| t == "Demo only"));
    let s = crate::strips::strip(&mut app);
    for want in ["Rolls d6", "Previous", "Next", "Play"] {
        assert!(s.iter().any(|t| t == want), "{want}: {s:?}");
    }
    let mut stick = desktop(false, 6);
    let s = crate::strips::strip(&mut stick);
    assert!(!s.iter().any(|t| t == "Next" || t == "Rolls d6"), "{s:?}");
}

#[test]
fn twenty_faced_dice_are_rolled_and_padded_one_to_twenty() {
    let mut app = desktop(true, 20);
    assert!(
        crate::strips::strip(&mut app)
            .iter()
            .any(|t| t == "Rolls d20")
    );
    // The pad's keys 1 to 20.
    for n in 1..=20u8 {
        assert!(app.offers(Action::Codex32(C32::Face(n))), "key {n}");
    }
    assert!(!app.offers(Action::Codex32(C32::Face(21))));
    // A whole share rolled: every face 1 to 20, some past six.
    c32(&mut app, C32::Walk(WalkAct::Reach(Reach::Share)));
    c32(&mut app, C32::Walk(WalkAct::Speed(3)));
    c32(&mut app, C32::Walk(WalkAct::Play));
    let mut t = app.now_ms();
    for i in 0..4000u32 {
        let b = (i as u8).wrapping_mul(53).wrapping_add(7);
        app.event(Event::Entropy(EntropyBytes::new([b; 32])));
        t += 16;
        app.event(Event::Tick { now_ms: t });
        if !make(&app).walk().playing() {
            break;
        }
    }
    let m = make(&app);
    let mut seen = Vec::new();
    for n in 0..26 {
        let made = m.made_of(0, n).expect("the share is rolled");
        assert_eq!(made.faces(), 20);
        for r in made.rows() {
            seen.push(r.first);
            seen.push(r.second);
        }
    }
    assert!(seen.iter().all(|f| (1..=20).contains(f)));
    assert!(seen.iter().any(|&f| f > 6), "faces past six come up");
}
