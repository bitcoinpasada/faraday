//! The codex32 entry's input panel (`docs/LAYOUT.md` §16.3): the pad and
//! the rolls it fills are on screen together at every desktop size, the
//! panel beside the sheets on a 2560-wide window and pinned at the
//! window's foot at 1920 and 1280, staying where it is as the page
//! scrolls; a face pressed lands in the panel's rolls; the Pi panel's
//! entry is as before, its context over the pad.

use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;

/// Faraday Desktop at `width` × `height` and `dpi`, Reduce motion on,
/// on the codex32 entry by six-faced dice, a frame drawn.
fn on_entry(width: u16, height: u16, dpi: u16) -> Faraday {
    on_shell(width, height, dpi, true, Way::Dice)
}

/// The same on a shell that offers the computer's dice roller, or not
/// (the stick and the Pi), by `way`.
fn on_shell(width: u16, height: u16, dpi: u16, roller: bool, way: Way) -> Faraday {
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
    app.set_roller_offered(roller);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i * 7; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStep(kstep::START));
    app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(&mut app, C32::Split(split as u8));
    c32(&mut app, C32::Next);
    for ch in "name".chars() {
        c32(&mut app, C32::Key(OskPress::Char(ch)));
    }
    c32(&mut app, C32::Next);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(&mut app, C32::Way(w as u8));
    c32(&mut app, C32::Next);
    assert_eq!(make(&app).open(), Some(Card::Enter));
    let _ = app.frame();
    app
}

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn make(app: &Faraday) -> &faraday_core::codex32::Codex32Make {
    app.keygen().unwrap().codex32.as_ref().unwrap()
}

/// The pad's key for face `f`.
fn face(f: u8) -> Action {
    Action::Codex32(C32::Face(f))
}

/// Presses `action` where the last frame drew it, as a finger would.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
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

/// Where `text` is drawn, the first of it.
fn text_at(app: &mut Faraday, text: &str) -> Option<(i32, i32, i32, i32)> {
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == text)
        .map(|b| b.1)
}

/// Whether the box lies wholly in a window `w` × `h`.
fn in_window((x, y, bw, bh): (i32, i32, i32, i32), w: u16, h: u16) -> bool {
    x >= 0 && y >= 0 && x + bw <= i32::from(w) && y + bh <= i32::from(h)
}

/// The pad's 3 and the rolls ("First roll", "Second roll") both in the
/// window.
fn pad_and_rolls_in_view(app: &mut Faraday, w: u16, h: u16) {
    let key = app.hit_box(face(3)).expect("the pad's 3 is drawn");
    let key = (
        i32::from(key.0),
        i32::from(key.1),
        i32::from(key.2),
        i32::from(key.3),
    );
    assert!(in_window(key, w, h), "the pad's 3 in view: {key:?}");
    for head in ["First roll", "Second roll"] {
        let at = text_at(app, head).unwrap_or_else(|| panic!("{head} drawn"));
        assert!(in_window(at, w, h), "{head} in view: {at:?}");
    }
}

#[test]
fn the_pad_and_the_rolls_are_on_screen_together_and_stay_as_the_page_scrolls() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1440)] {
        let mut app = on_entry(w, h, 160);
        pad_and_rolls_in_view(&mut app, w, h);
        let key = app.hit_box(face(3));
        let page = |app: &Faraday| app.keygen().unwrap().scroll.y;
        let before = page(&app);
        // The pointer over the page's own sheets, right of the panel.
        scroll_page(&mut app, w / 2 + 100, h / 3, 400);
        assert!(page(&app) > before, "{w}x{h}: the page scrolled");
        assert_eq!(app.hit_box(face(3)), key, "{w}x{h}: the pad stayed");
        pad_and_rolls_in_view(&mut app, w, h);
    }
}

#[test]
fn at_2560_the_panel_stands_beside_the_sheets_and_elsewhere_at_the_foot() {
    // 2560: a column at the work area's start, below the top strip, the
    // sheets right of it.
    let mut wide = on_entry(2560, 1440, 160);
    let key = wide.hit_box(face(3)).unwrap();
    let sheet = text_at(&mut wide, "Die tracks").expect("the dice sheet");
    assert!(
        i32::from(key.0) + i32::from(key.2) < sheet.0,
        "the pad left of the sheets: {key:?} {sheet:?}"
    );
    assert!(key.1 < 1440 / 2, "not at the foot: {key:?}");
    // 1920 and 1280: pinned at the window's foot, under the sheets.
    for (w, h) in [(1920u16, 1080u16), (1280, 800)] {
        let mut app = on_entry(w, h, 160);
        let key = app.hit_box(face(3)).unwrap();
        assert!(
            key.1 + key.3 > h - 120 && key.1 + key.3 <= h,
            "{w}x{h}: at the foot: {key:?}"
        );
        let first = text_at(&mut app, "First roll").unwrap();
        assert!(first.1 > i32::from(h) / 2, "{w}x{h}: the rolls with it");
    }
}

#[test]
fn a_face_pressed_on_the_panel_lands_in_its_rolls() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1440)] {
        let mut app = on_entry(w, h, 160);
        // Die 1 rolls 3 then 5; dice 2 to 5 roll 1 first.
        for f in [3, 1, 1, 1, 1, 5] {
            tap(&mut app, face(f));
        }
        let d = make(&app).dice();
        assert_eq!((d.first(0), d.second(0)), (Some(3), Some(5)), "{w}x{h}");
        // The pair's bit is drawn in the panel, by the pad.
        let bit = text_at(&mut app, "higher · 1").expect("die 1's pair");
        let key = app.hit_box(face(3)).unwrap();
        assert!(in_window(bit, w, h), "{w}x{h}: {bit:?}");
        let near = (bit.1 - i32::from(key.1)).abs() < i32::from(h) / 3
            || (bit.0 - i32::from(key.0)).abs() < 424;
        assert!(near, "{w}x{h}: by the pad: {bit:?} {key:?}");
    }
}

#[test]
fn the_pi_panels_entry_keeps_its_context_over_the_pad() {
    let mut pi = on_shell(480, 640, 286, false, Way::Dice);
    // The six-face pad of the small panel, under the dice as rows of words.
    let key = pi
        .hit_box(Action::Codex32(C32::Key(OskPress::Char('3'))))
        .expect("the pad's 3");
    let die = text_at(&mut pi, "Die 1").expect("the dice rows");
    assert!(die.1 < i32::from(key.1), "the context over the pad");
    assert!(!pi.offers(face(3)), "no panel's pad on the Pi");
    assert!(i32::from(key.1) + i32::from(key.3) <= 640);
}

#[test]
fn characters_typed_take_the_codex32_keyboard_at_the_foot() {
    let mut app = on_shell(1280, 800, 160, true, Way::Typed);
    let q = Action::Codex32(C32::Key(OskPress::Char('q')));
    let key = app.hit_box(q).expect("the codex32 keyboard's q");
    assert!(key.1 > 800 / 2, "at the window's foot: {key:?}");
    scroll_page(&mut app, 1280 / 2, 800 / 3, 400);
    assert_eq!(app.hit_box(q), Some(key), "the keyboard stays");
    tap(&mut app, q);
    assert_eq!(make(&app).work().map(|w| w.entered(0)), Some(1));
}
