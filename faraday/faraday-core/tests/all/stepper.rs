//! The stepper (`docs/DESIGN.md` §4.2), on Make codex32 shares' Scheme
//! card with Other chosen (`docs/CODEX32.md` §6.1) and Make a new
//! wallet's Quorum: its count is drawn between − and +, a press of either
//! moves it by one, the keyboard reaches + and Enter presses it, and the
//! pointer resting on − names it.

use faraday_core::codex32::{C32, SPLITS, Split};
use faraday_core::create::NewKind;
use faraday_core::keygen::{Start, kstep};
use faraday_core::strings::EN;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

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
    app
}

/// New key's Make codex32 shares under Reduce motion, Scheme open with
/// Other chosen: its two steppers drawn.
fn other(width: u16, height: u16) -> Faraday {
    other_with(width, height, true)
}

fn other_with(width: u16, height: u16, reduce: bool) -> Faraday {
    let mut app = on(width, height);
    app.press(Action::ReduceMotion(reduce));
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    app.press(Action::KStep(kstep::START));
    let _ = app.frame();
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let other = SPLITS.iter().position(|s| *s == Split::Other).unwrap();
    app.press(Action::Codex32(C32::Split(other as u8)));
    tick(&mut app, 1000);
    app.settle();
    let _ = app.frame();
    app
}

fn tick(app: &mut Faraday, ms: u64) {
    let now = app.now_ms() + ms;
    app.event(Event::Tick { now_ms: now });
}

/// The threshold and the number of strings the scheme holds.
fn k_n(app: &Faraday) -> (u8, u8) {
    app.keygen().unwrap().codex32.as_ref().unwrap().k_n()
}

/// A finger down and up where `action` is drawn.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app.where_offered(action).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

/// How many times `label` is drawn.
fn count(app: &mut Faraday, label: &str) -> usize {
    app.drawn_texts().iter().filter(|t| *t == label).count()
}

#[test]
fn the_count_is_drawn_and_moves_by_one_with_minus_and_plus() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let mut app = other(w, h);
        let (k, n) = k_n(&app);
        tap(&mut app, Action::Codex32(C32::N(1)));
        assert_eq!(k_n(&app), (k, n + 1), "{w}x{h}: + raised the strings");
        let _ = app.frame();
        let shown = (n + 1).to_string();
        assert!(
            app.drawn_texts().contains(&shown),
            "{w}x{h}: the new count is drawn"
        );
        tap(&mut app, Action::Codex32(C32::K(1)));
        assert_eq!(k_n(&app), (k + 1, n + 1), "{w}x{h}: + raised the threshold");
        tap(&mut app, Action::Codex32(C32::K(-1)));
        tap(&mut app, Action::Codex32(C32::N(-1)));
        assert_eq!(k_n(&app), (k, n), "{w}x{h}: − took each back");
    }
}

/// Make a new wallet's Quorum card, open on Multisig, Reduce motion on.
fn quorum() -> Faraday {
    let mut app = on(1280, 800);
    app.press(Action::ReduceMotion(true));
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap();
    app.press(Action::CKind(multi as u8));
    tick(&mut app, 1000);
    app.settle();
    let _ = app.frame();
    assert!(app.offers(Action::CN(1)), "the Keys stepper is drawn");
    app
}

/// The signatures needed and the keys of the wallet being made.
fn m_n(app: &Faraday) -> (usize, usize) {
    let c = app.create().unwrap();
    (c.m, c.n)
}

#[test]
fn the_keyboard_reaches_plus_and_enter_presses_it() {
    let mut app = quorum();
    let (m, n) = m_n(&app);
    let none = count(&mut app, EN.tip_increase);
    let mut on_plus = false;
    for _ in 0..200 {
        app.event(Event::Key(Key::Tab));
        tick(&mut app, 600);
        if count(&mut app, EN.tip_increase) > none {
            on_plus = true;
            break;
        }
    }
    assert!(on_plus, "Tab reaches a + and names it");
    assert_eq!(m_n(&app), (m, n), "moving the keyboard changes nothing");
    app.event(Event::Key(Key::Enter));
    let after = m_n(&app);
    assert!(
        after == (m + 1, n) || after == (m, n + 1),
        "Enter on + raised one count: {:?} to {after:?}",
        (m, n),
    );
}

#[test]
fn the_pointer_resting_on_minus_names_it() {
    let mut app = quorum();
    let (x, y) = app.where_offered(Action::CN(-1)).unwrap();
    let before = count(&mut app, EN.tip_decrease);
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    tick(&mut app, 300);
    assert_eq!(count(&mut app, EN.tip_decrease), before + 1);
}

#[test]
fn enter_presses_the_focused_plus_on_new_key() {
    let mut app = other(1280, 800);
    let (k, n) = k_n(&app);
    let none = count(&mut app, EN.tip_increase);
    let mut on_plus = false;
    for _ in 0..200 {
        app.event(Event::Key(Key::Tab));
        tick(&mut app, 600);
        if count(&mut app, EN.tip_increase) > none {
            on_plus = true;
            break;
        }
    }
    assert!(on_plus, "Tab reaches a + on Scheme and names it");
    app.event(Event::Key(Key::Enter));
    let after = k_n(&app);
    assert!(
        after == (k + 1, n) || after == (k, n + 1),
        "Enter on + raised one count: {:?} to {after:?}",
        (k, n),
    );
}

#[test]
fn the_pointer_resting_on_schemes_minus_names_it() {
    let mut app = other(1280, 800);
    let (x, y) = app.where_offered(Action::Codex32(C32::K(-1))).unwrap();
    let before = count(&mut app, EN.tip_decrease);
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    tick(&mut app, 300);
    assert_eq!(count(&mut app, EN.tip_decrease), before + 1);
}

#[test]
fn schemes_tooltip_stands_at_its_button() {
    for (w, h) in [(1280, 800), (1920, 1080), (480, 640)] {
        let mut app = other(w, h);
        let (x, y) = app.where_offered(Action::Codex32(C32::K(-1))).unwrap();
        app.event(Event::Hover { x, y });
        tick(&mut app, 300);
        tick(&mut app, 300);
        let boxes = app.drawn_text_boxes();
        let tip = boxes.iter().find(|(t, _)| t == EN.tip_decrease).unwrap().1;
        eprintln!("hover ({x},{y}) tip {tip:?}");
        assert!(
            (tip.0 - i32::from(x)).abs() < 80 && (tip.1 - i32::from(y)).abs() < 80,
            "{w}x{h}"
        );
    }
}

#[test]
fn the_pointer_resting_on_schemes_minus_names_it_with_motion() {
    let mut app = other_with(1280, 800, false);
    for _ in 0..20 {
        tick(&mut app, 100);
        let _ = app.frame();
    }
    let (x, y) = app.where_offered(Action::Codex32(C32::K(-1))).unwrap();
    let before = count(&mut app, EN.tip_decrease);
    app.event(Event::Hover { x, y });
    tick(&mut app, 300);
    tick(&mut app, 300);
    assert_eq!(count(&mut app, EN.tip_decrease), before + 1);
}

/// The head's segmented pair ends left of the top strip's pills at every
/// size (`docs/LAYOUT.md` §15).
#[test]
fn the_head_pair_stays_left_of_the_pills() {
    for (w, h) in [(1280, 800), (1366, 768), (1920, 1080), (2560, 1440)] {
        let mut app = other(w, h);
        let _ = app.frame();
        let g = app.hit_box(Action::Guided(true)).unwrap();
        let q = app.hit_box(Action::Learn).unwrap().0;
        let p = app.hit_box(Action::ModeSheet).map_or(q, |b| b.0.min(q));
        assert!(
            g.0 + g.2 <= p,
            "{w}x{h}: Guided ends at {}, the pill at {}",
            g.0 + g.2,
            p
        );
    }
}
