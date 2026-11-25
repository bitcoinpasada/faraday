//! The codex32 entry's input panel (`docs/LAYOUT.md` §16.3): the pad and
//! the rolls it fills are on screen together at every desktop size, the
//! panel beside the sheets on a 2560-wide window and pinned at the
//! window's foot at 1920 and 1280, staying where it is as the page
//! scrolls; a face pressed lands in the panel's rolls; the Pi panel's
//! entry is as before, its context over the pad. The panel is short on
//! the smaller desktops: the rolls in one row beside a pad of one row,
//! the dice's size one button opening a menu, offered there alone, and
//! Continue in the pad's place once every character is in, the one way
//! on. The step-through's Next brings the die it rolls into view.

use faraday_core::codex32::{C32, Card, DIE_SIZES, SPLITS, Split, WAYS, Way};
use faraday_core::keygen::Start;
use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;

/// Faraday Demo at `width` × `height` and `dpi`, Reduce motion on,
/// on the codex32 entry by six-faced dice, a frame drawn.
fn on_entry(width: u16, height: u16, dpi: u16) -> Faraday {
    on_shell(width, height, dpi, true, Way::Dice)
}

/// The same on a shell that offers the computer's dice roller, or not
/// (the stick and the Pi), by `way`.
fn on_shell(width: u16, height: u16, dpi: u16, roller: bool, way: Way) -> Faraday {
    on_shell_len(width, height, dpi, (roller, way), 0)
}

/// The same with the length `len` chosen (0: 48 characters, 1: 74).
fn on_shell_len(width: u16, height: u16, dpi: u16, (roller, way): (bool, Way), len: u8) -> Faraday {
    let mut app = on_entropy_len(width, height, dpi, roller, len);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(&mut app, C32::Way(w as u8));
    c32(&mut app, C32::Next);
    assert_eq!(make(&app).open(), Some(Card::Enter));
    through_fade(&mut app);
    app
}

/// Make codex32 shares on such a shell, on its first card (Scheme).
fn on_scheme(width: u16, height: u16, dpi: u16, roller: bool) -> Faraday {
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
    if roller {
        // The roller is Practice's alone (`docs/DEMO.md` §14).
        faraday_core::testkit::into_demo(&mut app);
    }
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i * 7; 32])));
    }
    app.press(Action::ReduceMotion(true));
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(
        Start::Make(faraday_core::forms::Form::Codex32).index(),
    ));
    let _ = app.frame();
    app
}

/// Make codex32 shares on such a shell, 2 of 3 named `name`, on its
/// Entropy card.
fn on_entropy(width: u16, height: u16, dpi: u16, roller: bool) -> Faraday {
    on_entropy_len(width, height, dpi, roller, 0)
}

/// The same with the length `len` chosen (0: 48 characters, 1: 74).
fn on_entropy_len(width: u16, height: u16, dpi: u16, roller: bool, len: u8) -> Faraday {
    let mut app = on_scheme(width, height, dpi, roller);
    c32(&mut app, C32::Length(len));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c32(&mut app, C32::Split(split as u8));
    c32(&mut app, C32::Next);
    for ch in "name".chars() {
        c32(&mut app, C32::Key(OskPress::Char(ch)));
    }
    c32(&mut app, C32::Next);
    assert_eq!(make(&app).open(), Some(Card::Entropy));
    app
}

/// Frames drawn, a tick before each, until a step's fade has run out: the
/// step that changed shows as the card.
fn through_fade(app: &mut Faraday) {
    let mut t = app.now_ms();
    for _ in 0..40 {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
    }
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

/// The rolls' die names drawn by the pad: "Die 1" to "Die 5" within
/// `near` pixels of its key `key` up or down, in the window.
fn rolls_by(app: &mut Faraday, key: (i32, i32, i32, i32), near: i32, (w, h): (u16, u16)) -> bool {
    (1..=5).all(|d| {
        let name = format!("Die {d}");
        app.drawn_text_boxes()
            .plain()
            .into_iter()
            .any(|(t, b)| t == name && in_window(b, w, h) && (b.1 - key.1).abs() < near)
    })
}

/// The pad's 3 and the rolls, each die's name over them, both in the
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
    assert!(
        rolls_by(app, key, i32::from(h), (w, h)),
        "the rolls in view"
    );
}

#[test]
fn the_pad_and_the_rolls_are_on_screen_together_and_stay_as_the_page_scrolls() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 800)] {
        let mut app = on_entry(w, h, 160);
        pad_and_rolls_in_view(&mut app, w, h);
        // The dice sheet closed (a share's end): no column, the page is
        // the step's and scrolls, the pad staying in view as it does.
        close_dice_sheet(&mut app);
        pad_and_rolls_in_view(&mut app, w, h);
        let key = app.hit_box(face(3));
        let page = |app: &Faraday| app.keygen().unwrap().scroll.y;
        let before = page(&app);
        // The pointer over the page's own sheets, right of the panel.
        scroll_page(&mut app, w / 2 + 100, h / 3, 400);
        assert!(page(&app) > before, "{w}x{h}: the page scrolled");
        // The panel stands under the card's head, which takes it up as it
        // scrolls away, to its place at the window's top; there it stays.
        let key = key.unwrap();
        let kept = app.hit_box(face(3)).unwrap();
        assert!(kept.1 <= key.1, "{w}x{h}: the pad never goes down");
        pad_and_rolls_in_view(&mut app, w, h);
        scroll_page(&mut app, w / 2 + 100, h / 3, 400);
        assert_eq!(app.hit_box(face(3)), Some(kept), "{w}x{h}: the pad stayed");
        pad_and_rolls_in_view(&mut app, w, h);
    }
}

#[test]
fn beside_the_sheets_a_panel_taller_than_the_window_scrolls_within_itself() {
    // A window wide enough for the panel beside the sheets but too low
    // for it whole with twenty-faced dice's three rows of keys: it
    // stands beside them still, and scrolls on its own.
    let (w, h) = (2560u16, 500u16);
    let mut app = on_entry(w, h, 160);
    choose_size(&mut app, 20);
    // Beside the rail at the work area's start, below the top strip,
    // not at the foot.
    let die = text_at(&mut app, "Die 1").expect("the rolls");
    assert!(
        die.0 > 100 && die.1 < i32::from(h) / 2,
        "the panel beside the sheets: {die:?}"
    );
    assert_eq!(make(&app).panel_offset(), 0.0);
    let page = app.keygen().unwrap().scroll.y;
    let (x, y) = (
        u16::try_from(die.0 + 20).unwrap(),
        u16::try_from(die.1).unwrap(),
    );
    scroll_page(&mut app, x, y, 600);
    assert!(
        make(&app).panel_offset() > 0.0,
        "the panel scrolled within itself"
    );
    assert_eq!(
        app.keygen().unwrap().scroll.y,
        page,
        "the page did not move"
    );
    let key = app.hit_box(face(3)).expect("the pad's 3 is drawn");
    let key = (
        i32::from(key.0),
        i32::from(key.1),
        i32::from(key.2),
        i32::from(key.3),
    );
    assert!(in_window(key, w, h), "the pad scrolled into view: {key:?}");
    tap(&mut app, face(3));
    assert_eq!(
        make(&app).dice().first(0),
        Some(3),
        "a face pressed there is a roll"
    );
}

#[test]
fn a_face_pressed_on_the_panel_lands_in_its_rolls() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 800)] {
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

/// Size `i` of the Entropy card's choice of the dice's size.
fn size(i: usize) -> Action {
    Action::Codex32(C32::Faces(i as u8))
}

/// Whether any of the six sizes is offered on screen.
fn any_size(app: &mut Faraday) -> bool {
    let _ = app.frame();
    (0..DIE_SIZES.len()).any(|i| app.offers(size(i)))
}

#[test]
fn the_pad_has_no_done_and_continue_stands_in_its_place_once_every_character_is_in() {
    for roller in [false, true] {
        let mut app = on_shell(1280, 800, 160, roller, Way::Dice);
        let done = Action::Codex32(C32::Key(OskPress::Done));
        let next = Action::Codex32(C32::Next);
        assert!(!app.offers(done), "no Done on the pad");
        assert!(!app.offers(next), "no Continue while a character is left");
        // Both shares, each character five dice higher on their second
        // roll.
        while make(&app).front().is_some() {
            for f in [1, 1, 1, 1, 1, 2, 2, 2, 2, 2] {
                c32(&mut app, C32::Face(f));
            }
        }
        let _ = app.frame();
        assert!(!app.offers(face(3)), "the pad has gone");
        // Continue at the window's foot, wherever the page is scrolled.
        scroll_page(&mut app, 1280 / 2, 800 / 3, 4000);
        let at = app.hit_box(next).expect("Continue offered");
        assert!(at.1 > 800 - 80, "in the panel at the foot: {at:?}");
        // Outside Practice the strings made go on to the worksheets; Practice's
        // stand in the sample's place once it is loaded.
        if !roller {
            tap(&mut app, next);
            assert_eq!(make(&app).open(), Some(Card::Sheets), "it goes on");
        }
    }
}

#[test]
fn the_dice_size_is_chosen_on_the_entropy_card_and_the_panel_follows_it() {
    // The owner's panel (960 × 580), 1280 × 800 and 1920 × 1080, in
    // Testnet and in Practice.
    for (w, h, dpi) in [
        (1440u16, 870u16, 240u16),
        (1280, 800, 160),
        (1920, 1080, 160),
    ] {
        for roller in [false, true] {
            let mut app = on_entropy(w, h, dpi, roller);
            let way = WAYS.iter().position(|w| *w == Way::Dice).unwrap();
            c32(&mut app, C32::Way(way as u8));
            assert!(any_size(&mut app), "{w}x{h}: the sizes on the Entropy card");
            assert!(app.offers(Action::Codex32(C32::Colour(0))), "the colours");
            tap(&mut app, size(5));
            assert_eq!(make(&app).dice_choice().faces, 20, "{w}x{h}");
            c32(&mut app, C32::Next);
            assert_eq!(make(&app).open(), Some(Card::Enter));
            assert!(
                !any_size(&mut app),
                "{w}x{h}: no choice of size in the panel"
            );
            assert!(
                !app.drawn_texts().iter().any(|t| t == "d20" || t == "d6"),
                "{w}x{h}: no dice's button"
            );
            assert!(app.offers(face(20)), "{w}x{h}: a key per face");
            assert!(!app.offers(face(21)), "{w}x{h}: and no more");
        }
    }
}

#[test]
fn the_dice_sheet_stands_only_with_the_rolling_shares_checksum_and_closes_when_it_is_done() {
    let has = |app: &mut Faraday, s: &str| app.drawn_texts().iter().any(|t| t == s);
    let sheet = Action::Codex32(C32::DiceSheet);
    let mut app = on_shell(1280, 2400, 160, false, Way::Dice);
    c32(&mut app, C32::Face(3));
    assert!(
        has(&mut app, "Die tracks"),
        "with the share rolled's Checksum"
    );
    // The derived share's tab and the Secret's: their worksheets alone.
    for view in [1u8, 2] {
        c32(&mut app, C32::View(view));
        assert!(
            !has(&mut app, "Dice de-biasing"),
            "tab {view}: no dice sheet"
        );
    }
    c32(&mut app, C32::View(0));
    assert!(has(&mut app, "Die tracks"), "back on the Checksum tab");
    // Share A's last roll: the sheet closes to its head row.
    c32(&mut app, C32::Key(OskPress::Back));
    while make(&app).front().is_some_and(|f| f.0 == 0) {
        for f in [1, 1, 1, 1, 1, 2, 2, 2, 2, 2] {
            c32(&mut app, C32::Face(f));
        }
    }
    assert!(has(&mut app, "Dice de-biasing"), "its head row");
    assert!(!has(&mut app, "Die tracks"), "closed");
    // Its chevron opens it to the tracks as they were, and closes it.
    tap(&mut app, sheet);
    assert!(has(&mut app, "Die tracks"), "opened");
    tap(&mut app, sheet);
    assert!(!has(&mut app, "Die tracks"), "closed again");
    // The next share's first roll opens it by itself.
    c32(&mut app, C32::Face(4));
    assert!(has(&mut app, "Die tracks"), "a roll opens it");
}

#[test]
fn the_pi_panels_six_face_pad_has_no_done_and_the_flows_continue_goes_on() {
    let mut pi = on_shell(480, 640, 286, false, Way::Dice);
    let done = Action::Codex32(C32::Key(OskPress::Done));
    let next = Action::Codex32(C32::Next);
    let key = |c: char| Action::Codex32(C32::Key(OskPress::Char(c)));
    assert!(pi.hit_box(key('3')).is_some(), "the pad is up");
    assert!(!pi.offers(done), "no Done on the pad");
    while make(&pi).front().is_some() {
        for f in "1111122222".chars() {
            pi.press(key(f));
        }
    }
    let _ = pi.frame();
    assert!(pi.offers(next), "Continue once every character is in");
    assert!(!pi.offers(done), "still no Done");
    tap(&mut pi, next);
    assert_eq!(make(&pi).open(), Some(Card::Sheets), "it goes on");
}

/// Rolls share A's last character away, which closes the dice sheet.
fn close_dice_sheet(app: &mut Faraday) {
    while make(app).front().is_some_and(|f| f.0 == 0) {
        for f in [1, 1, 1, 1, 1, 2, 2, 2, 2, 2] {
            c32(app, C32::Face(f));
        }
    }
    let _ = app.frame();
}

#[test]
fn a_roll_opens_a_closed_dice_sheet_over_several_frames_and_at_once_under_reduce_motion() {
    for still in [false, true] {
        let mut app = on_shell(1280, 2400, 160, false, Way::Dice);
        app.press(Action::ReduceMotion(still));
        close_dice_sheet(&mut app);
        app.settle();
        let shown = |app: &Faraday| make(app).dice_shown(app.now_ms(), still);
        assert!(shown(&app) < 0.01, "closed to its head row");
        c32(&mut app, C32::Face(4));
        let mut seen = vec![shown(&app)];
        for _ in 0..30 {
            let _ = app.frame();
            let now = app.now_ms() + 16;
            app.event(Event::Tick { now_ms: now });
            seen.push(shown(&app));
        }
        let _ = app.frame();
        if still {
            assert!(seen.iter().all(|s| *s > 0.99), "at once: {seen:?}");
        } else {
            let part = seen.iter().filter(|s| **s > 0.01 && **s < 0.99).count();
            assert!(part >= 4, "grows over several frames: {seen:?}");
            assert!(
                seen.windows(2).all(|w| w[1] >= w[0]),
                "only grows: {seen:?}"
            );
            assert!(seen.last().is_some_and(|s| *s > 0.99), "to full: {seen:?}");
        }
    }
}

// ---- Make codex32 shares on medium panels (owner, 2026-10-25) ----

/// The dice of size `n` chosen, as the Entropy card chooses them.
fn choose_size(app: &mut Faraday, n: u8) {
    let i = DIE_SIZES.iter().position(|&s| s == n).unwrap();
    c32(app, C32::Faces(i as u8));
    let _ = app.frame();
}

/// How many times `text` is drawn.
fn drawn(app: &mut Faraday, text: &str) -> usize {
    app.drawn_texts().iter().filter(|t| *t == text).count()
}

#[test]
fn the_tracks_show_only_the_pads_the_dice_reach() {
    let mut app = on_shell(1920, 1080, 160, false, Way::Dice);
    assert_eq!(
        drawn(&mut app, "Die pads (d7+)"),
        0,
        "d6: no hexagons' head"
    );
    let (sevens, nines) = (drawn(&mut app, "7"), drawn(&mut app, "9"));
    choose_size(&mut app, 8);
    assert_eq!(
        drawn(&mut app, "Die pads (d7+)"),
        1,
        "d8: the hexagons' head"
    );
    // Each of the five tracks and the pad gain a 7, none a 9.
    assert_eq!(
        drawn(&mut app, "7"),
        sevens + 6,
        "d8: pad 7 on each track and its key"
    );
    assert_eq!(drawn(&mut app, "9"), nines, "d8: no pad 9");
    choose_size(&mut app, 10);
    assert_eq!(
        drawn(&mut app, "9"),
        nines + 6,
        "d10: pad 9 on each track and its key"
    );
    choose_size(&mut app, 6);
    assert_eq!(
        drawn(&mut app, "7"),
        sevens,
        "back to d6: the pads past 6 hidden"
    );
}

// ---- The view follows the work, calmly (owner, 2026-10-25) ----

/// `n` frames 16 ms apart, a tick before each, as a shell draws them.
fn frames(app: &mut Faraday, n: usize) {
    for _ in 0..n {
        let now = app.now_ms() + 16;
        app.event(Event::Tick { now_ms: now });
        let _ = app.frame();
    }
}

/// The page's scroll, the pan, and three things' tops.
type Spots = (f32, (f32, f32), Option<u16>, Option<i32>, Option<i32>);

/// Where things stand on the entry: the page's scroll, the worksheet's
/// pan, the pad's 3 and the dice sheet's "Die tracks" and "Working".
fn where_things_are(app: &mut Faraday) -> Spots {
    let page = app.keygen().unwrap().scroll.y;
    let pan = make(app).pan();
    let key = app.hit_box(face(3)).map(|b| b.1);
    let tracks = text_at(app, "Die tracks").map(|b| b.1);
    let working = text_at(app, "Working").map(|b| b.1);
    (page, (pan.x, pan.y), key, tracks, working)
}

/// Vector 2's share A begins `3`: its ten rolls on six-faced dice.
const THREE: [u8; 10] = [1, 2, 2, 2, 1, 2, 1, 1, 1, 2];

#[test]
fn a_roll_moves_nothing_and_a_character_in_its_box_comes_into_view() {
    for (w, h) in [(1280u16, 800u16), (960, 580)] {
        for practice in [false, true] {
            let mut app = on_shell(w, h, 160, practice, Way::Dice);
            app.press(Action::ReduceMotion(false));
            frames(&mut app, 40);
            // The page at its top: the worksheet's first box under the
            // panel at the foot.
            scroll_page(&mut app, w / 2, h / 3, -4000);
            frames(&mut app, 40);
            let at = where_things_are(&mut app);
            // The first nine rolls: nothing scrolls, pans or moves, at the
            // press or in the frames after it.
            for &f in &THREE[..9] {
                c32(&mut app, C32::Face(f));
                assert_eq!(where_things_are(&mut app), at, "{w}x{h} {practice}: at {f}");
                frames(&mut app, 30);
                assert_eq!(
                    where_things_are(&mut app),
                    at,
                    "{w}x{h} {practice}: after {f}"
                );
            }
            // The tenth puts the character in its box: the view goes to it
            // and comes to rest.
            c32(&mut app, C32::Face(THREE[9]));
            frames(&mut app, 40);
            let rest = where_things_are(&mut app);
            // It came to rest with the character's box above the panel.
            let panel = app.hit_box(face(3)).unwrap().1;
            let box10 = Action::Codex32(C32::Cell(faraday_core::codex32::Cell::Given {
                sheet: 0,
                b: 10,
            }));
            let at = app.hit_box(box10).expect("the character's box on screen");
            assert!(
                at.1 + at.3 <= panel,
                "{w}x{h} {practice}: {at:?} above {panel}"
            );
            // The next character's first roll moves nothing again.
            c32(&mut app, C32::Face(4));
            frames(&mut app, 30);
            let after = where_things_are(&mut app);
            assert_eq!((after.0, after.1), (rest.0, rest.1), "{w}x{h} {practice}");
        }
    }
}

/// The leftmost box drawn under the top bar by a text `is` accepts: the
/// card's head.
fn head_box(app: &mut Faraday, is: impl Fn(&str) -> bool) -> (i32, i32, i32, i32) {
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .filter(|(t, b)| is(t) && b.1 >= 64)
        .map(|(_, b)| b)
        .min_by_key(|b| b.0)
        .expect("the card's head is drawn")
}

fn overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

#[test]
fn the_dice_rolls_card_is_full_width_with_the_bar_carrying_your_choices() {
    let (w, h) = (2560u16, 1440u16);
    let mut app = on_scheme(w, h, 160, true);
    // The Start card stands beside Your choices.
    assert!(!app.offers(Action::Choices(faraday_core::ChoicesPress::Open)));
    assert!(crate::common::has(&mut app, "Your choices"));
    let mut app = on_entry(w, h, 160);
    assert!(app.offers(Action::Choices(faraday_core::ChoicesPress::Open)));
    assert!(
        !app.drawn_text_boxes()
            .plain()
            .iter()
            .any(|(t, b)| t == "Your choices" && b.1 >= 64 && b.0 > i32::from(w) / 2),
        "no Your choices pane at the right"
    );
}

#[test]
fn the_card_head_stands_at_the_same_x_on_the_start_card_and_the_dice_rolls_card() {
    let (w, h) = (2560u16, 1440u16);
    let mut app = on_scheme(w, h, 160, true);
    let scheme = head_box(&mut app, |t| t == "Scheme");
    let mut app = on_entry(w, h, 160);
    // The step is the bar's title; the card keeps its name.
    let rolls = head_box(&mut app, |t| t == "Characters");
    assert_eq!(scheme.0, rolls.0, "{scheme:?} {rolls:?}");
}

#[test]
fn the_input_panel_and_the_dice_column_stand_clear_of_the_card_head() {
    let (w, h) = (2560u16, 1440u16);
    let mut app = on_entry(w, h, 160);
    // The step is the bar's title; the card keeps its name.
    let rolls = head_box(&mut app, |t| t == "Characters");
    for f in 1..=6u8 {
        let k = app.hit_box(face(f)).expect("the pad's key");
        let k = (
            i32::from(k.0),
            i32::from(k.1),
            i32::from(k.2),
            i32::from(k.3),
        );
        assert!(!overlap(rolls, k), "face {f}: {rolls:?} over {k:?}");
    }
    // The dice sheet's column, where it stands, begins after the head.
    if let Some(col) = text_at(&mut app, "Dice de-biasing") {
        assert!(rolls.0 + rolls.2 <= col.0, "{rolls:?} over {col:?}");
    }
}

/// The cell press of the worksheet's given box `b` (share A), as a
/// finger finds it.
fn given(b: u8) -> Action {
    Action::Codex32(C32::Cell(faraday_core::codex32::Cell::Given {
        sheet: 0,
        b,
    }))
}

/// Where the character boxes' number `n` is drawn, left of `col`, above
/// the worksheet's head `sheet`, and at or right of `left` (the sheet's own
/// left edge, not the pad's keys beside it).
fn char_box(
    app: &mut Faraday,
    n: usize,
    (left, col, sheet): (i32, i32, i32),
) -> Option<(i32, i32, i32, i32)> {
    let name = n.to_string();
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .find(|(t, b)| *t == name && b.1 < sheet && b.0 >= left && b.0 < col)
        .map(|(_, b)| b)
}

/// The whole of the dice step on one window (owner, 2026-11-24): the
/// worksheet at the left, the dice sheet in a right column with its tree
/// over the die tracks, and the input panel's pad and rolls, every part
/// in the window at once, with a worksheet that pans in its view as well
/// as one drawn whole.
#[test]
fn the_dice_step_shows_the_worksheet_the_tree_the_tracks_and_the_pad_at_once() {
    let sizes = [
        (1366u16, 768u16, 0u8),
        (1440, 900, 0),
        (1536, 864, 0),
        (1920, 1080, 0),
        (2560, 1440, 0),
        (1920, 1080, 1),
        (2560, 1440, 1),
    ];
    for (w, h, len) in sizes {
        let chars = if len == 0 { 48 } else { 74 };
        let mut app = on_shell_len(w, h, 160, (true, Way::Dice), len);
        for f in THREE {
            c32(&mut app, C32::Face(f));
        }
        frames(&mut app, 40);
        let sheet = text_at(&mut app, "Checksum of Share A").expect("the worksheet is drawn");
        let head = text_at(&mut app, "Dice de-biasing").expect("the dice sheet is drawn");
        let tracks = text_at(&mut app, "Die tracks").expect("the tracks are drawn");
        let at = format!("{w}x{h} {chars}");
        assert!(in_window(sheet, w, h), "{at}: the worksheet in view");
        assert!(in_window(head, w, h), "{at}: the dice sheet in view");
        // On a window this low the column scrolls to reach its tracks.
        assert!(
            h < 800 || in_window(tracks, w, h),
            "{at}: the tracks in view"
        );
        assert!(
            head.0 > sheet.0 + sheet.2,
            "{at}: the dice sheet at the right"
        );
        assert!(tracks.1 > head.1, "{at}: the tree is over the tracks");
        // The characters' first and last box above the worksheet, and the
        // worksheet's first box under it, above the panel and left of the
        // column, in the same frame.
        for n in [1, chars] {
            let b = char_box(&mut app, n, (sheet.0, head.0, sheet.1))
                .unwrap_or_else(|| panic!("{at}: character box {n} is drawn"));
            assert!(in_window(b, w, h), "{at}: box {n} in view: {b:?}");
        }
        let pad = app.hit_box(face(3)).expect("the pad");
        let panel = i32::from(pad.1);
        let first = app.hit_box(given(10)).expect("the worksheet's box 10");
        let first = (
            i32::from(first.0),
            i32::from(first.1),
            i32::from(first.2),
            i32::from(first.3),
        );
        assert!(
            in_window(first, w, h)
                // Over the panel at the foot; beside it, the box is right of it.
                && (first.1 + first.3 <= panel || i32::from(pad.0 + pad.2) <= first.0)
                && first.0 + first.2 <= head.0,
            "{at}: the worksheet's first box in view over the panel: {first:?}"
        );
        // Where the room holds the whole sheet at a fitted cell (2560x1440,
        // 48 characters) it is drawn whole: a drag across it pans nothing.
        if (w, h, len) == (2560, 1440, 0) {
            let hrp = Action::Codex32(C32::Cell(faraday_core::codex32::Cell::Hrp {
                sheet: 0,
                i: 0,
            }));
            let (x, y, bw, bh) = app.hit_box(hrp).expect("the residue's first cell");
            let (x, y) = (x + bw / 2, y + bh / 2);
            for (px, py, phase) in [
                (x, y, TouchPhase::Down),
                (x - 60, y - 20, TouchPhase::Move),
                (x - 120, y - 40, TouchPhase::Move),
                (x - 120, y - 40, TouchPhase::Up),
            ] {
                app.event(Event::Touch {
                    x: px,
                    y: py,
                    phase,
                });
            }
            let _ = app.frame();
            let pan = make(&app).pan();
            assert_eq!(
                (pan.x, pan.y),
                (0.0, 0.0),
                "{at}: drawn whole, nothing pans"
            );
        }
        // The left side holds the window where the worksheet has room for
        // its rows; where under six rows are left, the page scrolls to the
        // view as before.
        let before = app.keygen().unwrap().scroll.y;
        scroll_page(&mut app, w / 2 - 100, h / 3, 400);
        let scrolled = app.keygen().unwrap().scroll.y > before;
        let short = len == 1 && h < 800;
        assert_eq!(scrolled, short, "{at}: the page scrolls only where short");
        if !scrolled {
            assert_eq!(
                text_at(&mut app, "Checksum of Share A"),
                Some(sheet),
                "{at}: the step stayed where it was"
            );
        }
        pad_and_rolls_in_view(&mut app, w, h);
    }
}

/// Where the worksheet has under six rows left beside the column, its
/// view is the class's and the page scrolls to it, the pad staying in
/// view.
#[test]
fn a_low_window_with_a_long_set_scrolls_the_page_to_the_class_view() {
    for (w, h) in [(1366u16, 768u16), (1280, 720)] {
        let mut app = on_shell_len(w, h, 160, (true, Way::Dice), 1);
        frames(&mut app, 3);
        assert!(
            text_at(&mut app, "Dice de-biasing").is_some(),
            "{w}x{h}: the column stands beside the worksheet"
        );
        let before = app.keygen().unwrap().scroll.y;
        scroll_page(&mut app, w / 2 - 100, h / 3, 600);
        assert!(
            app.keygen().unwrap().scroll.y > before,
            "{w}x{h}: the page scrolled to the sheet"
        );
        pad_and_rolls_in_view(&mut app, w, h);
    }
}

/// Where the column is lower than the window it scrolls within itself:
/// the tracks come into view above the panel, the page and the
/// worksheet staying where they are.
#[test]
fn the_column_scrolls_within_itself_to_its_tracks_and_the_page_stays() {
    let (w, h) = (1366u16, 768u16);
    let mut app = on_entry(w, h, 160);
    frames(&mut app, 3);
    let head = text_at(&mut app, "Dice de-biasing").expect("the column's head");
    let panel = i32::from(app.hit_box(face(3)).expect("the pad").1);
    let tracks_in_view = |app: &mut Faraday| {
        app.drawn_text_boxes()
            .plain()
            .into_iter()
            .filter(|(t, b)| t == "Die 5" && b.0 >= head.0 && b.1 + b.3 <= panel && b.1 >= 0)
            .count()
    };
    let (page, pan) = (app.keygen().unwrap().scroll.y, make(&app).pan());
    let was = tracks_in_view(&mut app);
    let (x, y) = (u16::try_from(head.0 + 20).unwrap(), h / 3);
    scroll_page(&mut app, x, y, 800);
    assert!(make(&app).pane_offset() > 0.0, "the column scrolled");
    assert!(
        tracks_in_view(&mut app) > was,
        "the fifth die's track came into view"
    );
    assert_eq!(app.keygen().unwrap().scroll.y, page, "the page stayed");
    assert_eq!(make(&app).pan(), pan, "the worksheet stayed");
}

/// The column's offset is kept within what it reaches now: scrolled to
/// its end on a low window, then the window made taller, its head row is
/// in view again. A regression check of the whole route (the resize's own
/// events also bring the page back); the clamp in drawing alone is not
/// isolated by it.
#[test]
fn the_columns_scroll_is_brought_back_where_its_reach_falls() {
    let (w, h) = (1366u16, 768u16);
    let mut app = on_entry(w, h, 160);
    choose_size(&mut app, 20);
    frames(&mut app, 3);
    let head = text_at(&mut app, "Dice de-biasing").expect("the column's head");
    let (x, y) = (u16::try_from(head.0 + 20).unwrap(), h / 3);
    scroll_page(&mut app, x, y, 4000);
    assert!(make(&app).pane_offset() > 0.0, "the column scrolled");
    // The window made taller: the column reaches nowhere now.
    let tall = 1600u16;
    app.event(Event::Display(DisplayInfo {
        width: w,
        height: tall,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    frames(&mut app, 3);
    let now = text_at(&mut app, "Dice de-biasing").expect("the column's head is drawn");
    assert!(
        now.1 >= 0 && in_window(now, w, tall),
        "the head row is in view again: {now:?}"
    );
}

/// Whether the worksheet's first box ("1") is in the window, left of the
/// dice column's head.
fn first_box_in_view(app: &mut Faraday, (w, h): (u16, u16)) -> bool {
    let col = text_at(app, "Dice de-biasing").map_or(i32::from(w), |b| b.0);
    let Some(sheet) = app
        .drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t.starts_with("Checksum of Share"))
        .map(|b| b.1)
    else {
        return false;
    };
    eprintln!(
        "DBG sheet {sheet:?} col {col} box {:?}",
        char_box(app, 1, (sheet.0, col, sheet.1))
    );
    char_box(app, 1, (sheet.0, col, sheet.1)).is_some_and(|b| in_window(b, w, h))
}

/// At the window where 48 characters stand on one screen (owner,
/// 2026-11-24), the page is at its start and stays there once the column
/// is back, however the page stood before.
fn assert_one_screen(app: &mut Faraday, (w, h): (u16, u16), why: &str) {
    app.settle();
    frames(app, 30);
    assert_eq!(app.keygen().unwrap().scroll.y, 0.0, "{why}: at the start");
    scroll_page(app, w / 2 - 100, h / 3, 600);
    assert_eq!(
        app.keygen().unwrap().scroll.y,
        0.0,
        "{why}: a wheel moves nothing"
    );
    assert!(first_box_in_view(app, (w, h)), "{why}: box 1 in view");
}

/// A share done and the finished sheet scrolled to its end, the next
/// share's first face brings the column back: the page is at its start,
/// nothing left to scroll, box 1 in view.
#[test]
fn a_share_done_and_scrolled_then_one_face_leaves_48_characters_on_one_screen() {
    let (w, h) = (1366u16, 768u16);
    let mut app = on_entry(w, h, 160);
    close_dice_sheet(&mut app);
    frames(&mut app, 30);
    scroll_page(&mut app, w / 2 - 100, h / 3, 4000);
    c32(&mut app, C32::Face(1));
    assert_one_screen(&mut app, (w, h), "a face after a scroll");
}

/// The dice sheet's head row pressed twice leaves the page where it was:
/// the page does not stay short from a follow of the regions drawn before.
#[test]
fn the_head_row_pressed_twice_leaves_48_characters_on_one_screen() {
    let (w, h) = (1366u16, 768u16);
    let mut app = on_entry(w, h, 160);
    frames(&mut app, 3);
    c32(&mut app, C32::DiceSheet);
    frames(&mut app, 30);
    c32(&mut app, C32::DiceSheet);
    assert_one_screen(&mut app, (w, h), "the head row twice");
}

/// The window `display` event at `w` x `h`, 160 dpi.
fn display(w: u16, h: u16) -> Event {
    Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// A tick, as the shell sends one, without a drawing.
fn tick(app: &mut Faraday) {
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
}

/// The head row closing the dice sheet and opening it again, Reduce motion
/// off, the device's order (tick, then frame): the page never goes past
/// where it rested while the sheet was closed, and ends at the start
/// (1366 x 768, 48 characters). Seen to fail only with both the glide wait
/// and `forget_drawn` of the head row removed; either alone leaves it
/// passing, so it holds the two together, not each.
#[test]
fn the_head_row_opening_the_sheet_again_never_sends_the_page_past_where_it_stood() {
    let mut app = on_entry(1366, 768, 160);
    app.press(Action::ReduceMotion(false));
    frames(&mut app, 3);
    c32(&mut app, C32::DiceSheet);
    frames(&mut app, 40);
    let closed = app.keygen().unwrap().scroll.y;
    c32(&mut app, C32::DiceSheet);
    for i in 0..30 {
        tick(&mut app);
        let y = app.keygen().unwrap().scroll.y;
        assert!(y <= closed, "tick {i}: {y} past where it stood, {closed}");
        let _ = app.frame();
        let y = app.keygen().unwrap().scroll.y;
        assert!(y <= closed, "frame {i}: {y} past where it stood, {closed}");
    }
    assert_eq!(app.keygen().unwrap().scroll.y, 0.0, "it ends at the start");
}

/// The page's scroll at its highest and at its end over 40 ticks after the
/// head row closes the sheet, with a frame drawn before the first tick or
/// not (Faraday Demo can draw one; the stick does not), Reduce motion off.
fn closing_by_the_head_row(h: u16, frame_first: bool) -> (f32, f32) {
    let mut app = on_entry(1366, h, 160);
    app.press(Action::ReduceMotion(false));
    frames(&mut app, 40);
    c32(&mut app, C32::DiceSheet);
    if frame_first {
        let _ = app.frame();
    }
    let mut most = 0f32;
    for _ in 0..40 {
        tick(&mut app);
        let _ = app.frame();
        most = most.max(app.keygen().unwrap().scroll.y);
    }
    (most, app.keygen().unwrap().scroll.y)
}

/// Closing the sheet with a frame drawn before the first tick (as Faraday
/// Demo can), Reduce motion off, ends where the stick's order ends and
/// never goes past it on the way: the page does not follow the worksheet
/// as it stands while the sheet is still whole. Fails without the wait for
/// the sheet's glide (1366 x 720 and 768).
#[test]
fn the_head_row_closing_the_sheet_ends_in_view_when_a_frame_comes_before_the_tick() {
    for h in [720u16, 768] {
        let (_, rest) = closing_by_the_head_row(h, false);
        let (most, end) = closing_by_the_head_row(h, true);
        assert_eq!(end, rest, "{h}: ends where the other order ends");
        assert!(most <= rest, "{h}: went to {most}, past {rest}");
    }
}

/// The fitted cell is the new window's after one drawing of the Display
/// event: the room moved, so a second drawing is asked for, and the
/// worksheet's boxes stand where a fresh window puts them.
#[test]
fn the_cell_is_the_new_windows_after_one_frame_of_a_resize() {
    let w = 2560u16;
    let mut app = on_entry(w, 1440, 160);
    frames(&mut app, 30);
    app.event(display(w, 1300));
    while app.poll_command().is_some() {}
    let _ = app.frame();
    let mut drawn_again = false;
    while let Some(c) = app.poll_command() {
        drawn_again |= matches!(c, Command::Draw);
    }
    assert!(drawn_again, "a second drawing is queued");
    let _ = app.frame();
    let mut fresh = on_entry(w, 1300, 160);
    frames(&mut fresh, 30);
    assert_eq!(
        app.hit_box(given(10)),
        fresh.hit_box(given(10)),
        "the worksheet's box 10 where a fresh window puts it"
    );
}
