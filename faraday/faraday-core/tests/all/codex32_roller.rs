//! Faraday Desktop's dice rollers in New key (`docs/CODEX32.md` §5.5,
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
    assert!(shows(&mut desk, "Next rolls character 1"));
    for act in [WalkAct::Next, WalkAct::Play] {
        assert!(desk.offers(Action::Codex32(C32::Walk(act))));
    }

    let mut stick = app(false, true);
    on_dice_entry(&mut stick);
    assert!(!shows(&mut stick, "Or let Faraday roll"));
    assert!(!stick.offers(Action::Codex32(C32::Walk(WalkAct::Next))));
    // A press that reaches a device shell anyway rolls nothing.
    c32(&mut stick, C32::Walk(WalkAct::Next));
    assert_eq!(make(&stick).seq_len(), 0);
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
    // the steps are forgotten.
    c32(&mut app, C32::Key(OskPress::Back));
    assert!(shows(
        &mut app,
        "Die 1, Die 2, Die 3, Die 4 rolled by this computer"
    ));
    assert!(make(&app).walk().shown().is_none());
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
fn play_to_the_end_makes_the_strings_and_strings_made_says_simulated_dice() {
    let mut app = app(true, true);
    on_dice_entry(&mut app);
    play(&mut app, Reach::End);
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
    play(&mut app, Reach::End);
    to_made(&mut app);
    assert!(shows(&mut app, "simulated dice and your dice"));
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

/// The app on Faraday Desktop at `width` × `height` and `dpi`, Reduce
/// motion on, on the codex32 entry by dice, a frame drawn.
fn sized(width: u16, height: u16, dpi: u16) -> Faraday {
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
    app.set_roller_offered(true);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i * 7; 32])));
    }
    app.press(Action::ReduceMotion(true));
    on_dice_entry(&mut app);
    let _ = app.frame();
    app
}

/// The page scrolled down by `dy` with the pointer at (x, y), a frame
/// after.
fn scroll_page(app: &mut Faraday, x: u16, y: u16, dy: i16) {
    app.event(Event::Hover { x, y });
    app.event(Event::Scroll { x, y, dy });
    app.event(Event::ScrollEnd { x, y });
    app.settle();
    let _ = app.frame();
}

#[test]
fn the_demo_only_card_holds_its_place_while_the_page_scrolls() {
    let next = Action::Codex32(C32::Walk(WalkAct::Next));
    // Wide: a pane at the right, below the top strip, the page scrolling
    // beside it.
    let mut wide = sized(1280, 800, 160);
    let at = wide.hit_box(next).expect("Next in the pane");
    assert!(at.0 > 1280 / 2, "at the right: {at:?}");
    let page = |app: &Faraday| app.keygen().unwrap().scroll.y;
    let before = page(&wide);
    scroll_page(&mut wide, 500, 400, 300);
    assert!(page(&wide) > before, "the page scrolled");
    assert_eq!(wide.hit_box(next), Some(at), "the card did not");
    let top = wide
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == "Demo only")
        .map(|b| b.1)
        .expect("the card's heading is in view");
    assert!(top.1 >= 96, "below the pill and the eye: {top:?}");

    // Narrow: a strip pinned at the window's foot.
    let mut narrow = sized(480, 640, 286);
    let at = narrow.hit_box(next).expect("Next in the strip");
    assert!(at.1 > 640 / 2, "at the foot: {at:?}");
    scroll_page(&mut narrow, 240, 200, 200);
    assert_eq!(narrow.hit_box(next), Some(at), "the strip stays");
    // More opens the whole card over the page; Less closes it.
    c32(&mut narrow, C32::Pane);
    assert!(make(&narrow).pane_open());
    assert!(shows(&mut narrow, "Rolls d6"), "the whole card");
    c32(&mut narrow, C32::Pane);
    assert!(!make(&narrow).pane_open());
}

#[test]
fn next_stays_where_it_is_at_every_step_and_character() {
    let next = Action::Codex32(C32::Walk(WalkAct::Next));
    // The pane at the right of a wide window, the strip at a narrow one's
    // foot.
    for (w, h, dpi) in [(1280, 800, 160), (480, 640, 286)] {
        let mut app = sized(w, h, dpi);
        let at = app.hit_box(next).expect("Next is drawn");
        // Past a character's last step, into the next character's, and
        // a step back.
        for i in 0..45 {
            c32(&mut app, C32::Walk(WalkAct::Next));
            if i % 7 == 6 {
                c32(&mut app, C32::Walk(WalkAct::Previous));
            }
            let _ = app.frame();
            assert_eq!(
                app.hit_box(next),
                Some(at),
                "Next moved at press {i} on {w}x{h}"
            );
        }
    }
}

#[test]
fn the_demo_only_card_stands_24_dp_from_the_windows_right_edge_on_a_big_window() {
    let next = Action::Codex32(C32::Walk(WalkAct::Next));
    for (w, h) in [(1920u16, 1080u16), (2560, 1440)] {
        let mut app = sized(w, h, 160);
        let at = app.hit_box(next).expect("Next in the pane");
        assert!(
            i32::from(at.0 + at.2) <= i32::from(w) - 24,
            "{w}: Next {at:?} reaches into the margin"
        );
        let card = app
            .drawn_text_boxes()
            .into_iter()
            .find(|(t, _)| t == "Demo only")
            .map(|b| b.1)
            .expect("the card's heading");
        // The card is 400 dp wide; its heading starts inside its left edge.
        let left = i32::from(w) - 24 - 400;
        assert!(
            (left..left + 48).contains(&card.0),
            "{w}: heading at {}",
            card.0
        );
    }
}
