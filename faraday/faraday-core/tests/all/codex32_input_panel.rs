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

use faraday_core::codex32::{C32, Card, DIE_SIZES, SPLITS, Split, Step, WAYS, WalkAct, Way};
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
    let mut app = on_entropy(width, height, dpi, roller);
    let w = WAYS.iter().position(|w| *w == way).unwrap();
    c32(&mut app, C32::Way(w as u8));
    c32(&mut app, C32::Next);
    assert_eq!(make(&app).open(), Some(Card::Enter));
    let _ = app.frame();
    app
}

/// Make codex32 shares on such a shell, 2 of 3 named `name`, on its
/// Entropy card.
fn on_entropy(width: u16, height: u16, dpi: u16, roller: bool) -> Faraday {
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
        // The roller is Demo's alone (`docs/DEMO.md` §14).
        faraday_core::testkit::into_demo(&mut app);
    }
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
    assert_eq!(make(&app).open(), Some(Card::Entropy));
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

/// The rolls' die names drawn by the pad: "Die 1" to "Die 5" within
/// `near` pixels of its key `key` up or down, in the window.
fn rolls_by(app: &mut Faraday, key: (i32, i32, i32, i32), near: i32, (w, h): (u16, u16)) -> bool {
    (1..=5).all(|d| {
        let name = format!("Die {d}");
        app.drawn_text_boxes()
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
    let sheet = text_at(&mut wide, "Die tracks").expect("the dice sheet");
    let key = wide.hit_box(face(3)).unwrap();
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
        let key = (
            i32::from(key.0),
            i32::from(key.1),
            i32::from(key.2),
            i32::from(key.3),
        );
        assert!(
            rolls_by(&mut app, key, 120, (w, h)),
            "{w}x{h}: the rolls with it"
        );
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
        // Outside Demo the strings made go on to the worksheets; Demo's
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
    // Testnet and in Demo.
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
fn at_1280_and_1366_the_rolls_stand_in_one_row_beside_a_pad_of_one_row() {
    for (w, h) in [(1280u16, 800u16), (1366, 768)] {
        let mut app = on_shell(w, h, 160, false, Way::Dice);
        let key = |app: &Faraday, f: u8| app.hit_box(face(f)).expect("a face's key");
        let one = key(&app, 1);
        // The six faces and Backspace on one row.
        for f in 2..=6 {
            assert_eq!(key(&app, f).1, one.1, "{w}x{h}: face {f} on the pad's row");
        }
        let back = app.hit_box(Action::Codex32(C32::Key(OskPress::Back)));
        assert_eq!(back.map(|b| b.1), Some(one.1), "{w}x{h}: Backspace too");
        // The five dice's names on one row, left of the pad, the row of
        // rolls level with it.
        let names: Vec<(i32, i32, i32, i32)> = (1..=5)
            .map(|d| {
                let name = format!("Die {d}");
                app.drawn_text_boxes()
                    .into_iter()
                    .filter(|(t, b)| *t == name && b.1 > i32::from(h) / 2)
                    .map(|(_, b)| b)
                    .next_back()
                    .unwrap_or_else(|| panic!("{w}x{h}: {name} in the panel"))
            })
            .collect();
        let level = names.iter().all(|b| (b.1 - names[0].1).abs() <= 2);
        assert!(level, "{w}x{h}: one row: {names:?}");
        assert!(names[4].0 < i32::from(one.0), "{w}x{h}: left of the pad");
        let rows = (names[0].1 - 8, names[0].1 + 80);
        assert!(
            (rows.0..rows.1).contains(&i32::from(one.1)),
            "{w}x{h}: the pad level with the rolls: {names:?} {one:?}"
        );
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
fn the_step_throughs_next_brings_the_die_it_rolls_into_view() {
    let mut app = on_shell(1280, 800, 160, true, Way::Dice);
    // The page at its top: the tracks are under the panel.
    scroll_page(&mut app, 1280 / 2, 800 / 3, -4000);
    for die in 0..5u8 {
        let step = |app: &Faraday| make(app).walk().shown().map(|(_, _, s)| s);
        for _ in 0..20 {
            c32(&mut app, C32::Walk(WalkAct::Next));
            for _ in 0..3 {
                let _ = app.frame();
                let now = app.now_ms() + 16;
                app.event(Event::Tick { now_ms: now });
            }
            if matches!(step(&app), Some(Step::First { die: d, .. }) if d == die) {
                break;
            }
        }
        assert!(
            matches!(step(&app), Some(Step::First { die: d, .. }) if d == die),
            "die {die}'s first roll: {:?}",
            step(&app)
        );
        let _ = app.frame();
        // Die n's track's name over the panel's own, both in the window.
        let name = format!("Die {}", die + 1);
        let mut at: Vec<(i32, i32, i32, i32)> = app
            .drawn_text_boxes()
            .into_iter()
            .filter(|(t, b)| *t == name && in_window(*b, 1280, 800))
            .map(|(_, b)| b)
            .collect();
        at.sort_by_key(|b| b.1);
        assert!(at.len() >= 2, "{name}: its track and the panel's: {at:?}");
        let (track, panel) = (at[0], at[at.len() - 1]);
        assert!(
            track.1 + track.3 < panel.1 - 8,
            "{name}'s track stands clear above the panel: {at:?}"
        );
    }
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

/// The panel's top at the foot: the highest of its "Working" and the
/// pad's keys, in pixels from the window's foot.
fn foot_h(app: &mut Faraday, h: u16) -> i32 {
    let mut top = i32::from(h);
    for (t, b) in app.drawn_text_boxes() {
        if t == "Working" && b.1 > i32::from(h) / 2 {
            top = top.min(b.1);
        }
    }
    for f in 1..=6 {
        if let Some(k) = app.hit_box(face(f)) {
            top = top.min(i32::from(k.1));
        }
    }
    i32::from(h) - top
}

#[test]
fn the_foot_panel_packs_the_rolls_the_working_area_and_the_pad_into_few_rows() {
    // Six- and twenty-faced dice, no button for their size, the Working
    // area one line where that is shorter: the owner's panel (960 × 580
    // dp at 1.5 px a dp: 132 dp, was 144, and 216), 1280 × 800 (108, was
    // 132, and 196), 1366 × 768 (108, was 132, and 160, was 176) and
    // 1920 × 1080 (88 in one row, was 104; 116, was 144): the panel's
    // parts begin no higher than its fewest rows allow. In dp.
    for (w, h, dpi, d6, d20) in [
        (1440u16, 870u16, 240u16, 132, 216),
        (1280, 800, 160, 108, 196),
        (1366, 768, 160, 108, 160),
        (1920, 1080, 160, 88, 116),
    ] {
        for (n, most) in [(6u8, d6), (20, d20)] {
            let mut app = on_shell(w, h, dpi, false, Way::Dice);
            choose_size(&mut app, n);
            let fh = foot_h(&mut app, h) * 160 / i32::from(dpi);
            assert!(
                fh > 0 && fh <= most,
                "{w}x{h} d{n}: the panel's parts {fh} dp from the foot"
            );
            pad_and_rolls_in_view(&mut app, w, h);
        }
    }
    // At 1920 × 1080 the six faces stand in one row with the rolls and
    // the Working area: the rolls, the pad, then the Working area.
    let mut app = on_shell(1920, 1080, 160, false, Way::Dice);
    let key = app.hit_box(face(1)).expect("the pad");
    let die5 = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(t, b)| t == "Die 5" && b.1 > 1080 / 2)
        .map(|(_, b)| b)
        .next_back()
        .expect("the rolls");
    let working = text_at(&mut app, "Working").expect("the Working area");
    assert!(die5.0 < i32::from(key.0), "the pad after the rolls");
    assert!(
        working.0 > i32::from(key.0),
        "the Working area after the pad"
    );
    assert!((working.1 - i32::from(key.1)).abs() < 24, "one row");
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

#[test]
fn the_tracks_and_the_tree_stand_side_by_side_where_they_fit() {
    // In Testnet, and in Demo beside the card's narrow column.
    for (w, h, dpi, demo) in [
        (1440u16, 870u16, 240u16, false),
        (1280, 800, 160, false),
        (1440, 870, 240, true),
        (1280, 800, 160, true),
    ] {
        let mut app = on_shell(w, h, dpi, demo, Way::Dice);
        let head = text_at(&mut app, "Dice de-biasing").expect("the dice sheet");
        let tracks = text_at(&mut app, "Die tracks").expect("the tracks");
        // Stacked, the tree (over 150 dp tall) would stand between them.
        let scale = i32::from(dpi) / 80;
        assert!(
            tracks.1 - head.1 < 70 * scale,
            "{w}x{h} demo {demo}: the tracks start at the sheet's top: {head:?} {tracks:?}"
        );
    }
}

#[test]
fn on_the_owners_panel_the_entry_takes_the_whole_width() {
    let (w, h) = (1440u16, 870u16);
    let mut app = on_shell(w, h, 240, true, Way::Dice);
    // The sheets start by the rail, not in a centred column.
    let head = text_at(&mut app, "Dice de-biasing").expect("the dice sheet");
    assert!(head.0 < 200, "the work starts by the rail: {head:?}");
    // No card beside it: the walk is the host strip's.
    assert!(text_at(&mut app, "Demo only").is_none());
    // The pad's keys at the foot, under the sheets.
    let key = app.hit_box(face(3)).expect("the pad");
    assert!(key.1 + key.3 > h - 200, "the pad at the foot: {key:?}");
}

/// The colour at (x, y) of the frame on screen, its three channels
/// summed.
fn ink_at(app: &mut Faraday, (x, y): (u16, u16)) -> i32 {
    let f = app.frame();
    let i = (usize::from(y) * f.width as usize + usize::from(x)) * 4;
    f.rgba
        .get(i..i + 3)
        .map_or(0, |p| i32::from(p[0]) + i32::from(p[1]) + i32::from(p[2]))
}

/// A point inside face `f`'s key, clear of its label and its edge.
fn in_key(app: &Faraday, f: u8) -> (u16, u16) {
    let k = app.hit_box(face(f)).expect("the key");
    (k.0 + k.2 / 2, k.1 + k.3 / 5)
}

#[test]
fn the_key_of_the_roll_shown_is_drawn_pressed_after_a_press_and_after_next() {
    // The person's press: their 3 drawn pressed, their 5 not.
    let mut app = on_shell(1280, 800, 160, true, Way::Dice);
    let (three, five) = (in_key(&app, 3), in_key(&app, 5));
    let rest = ink_at(&mut app, three);
    assert_eq!(rest, ink_at(&mut app, five), "two keys alike at rest");
    c32(&mut app, C32::Face(3));
    assert_eq!(make(&app).pressed_face(), Some(3));
    let pressed = ink_at(&mut app, three);
    assert_ne!(pressed, ink_at(&mut app, five), "3 drawn pressed");
    // The Demo walk's Next presses its die's face the same way.
    c32(&mut app, C32::Key(OskPress::Back));
    c32(&mut app, C32::Walk(WalkAct::Next));
    let f = make(&app).pressed_face().expect("Next rolled a face");
    let other = if f == 1 { 2 } else { 1 };
    let (on, off) = (in_key(&app, f), in_key(&app, other));
    assert_ne!(
        ink_at(&mut app, on),
        ink_at(&mut app, off),
        "Next's {f} drawn pressed"
    );
}

#[test]
fn the_pressed_key_is_emphasised_briefly_and_at_rest_under_reduce_motion() {
    for still in [false, true] {
        let mut app = on_shell(1280, 800, 160, false, Way::Dice);
        app.press(Action::ReduceMotion(still));
        let _ = app.frame();
        let three = in_key(&app, 3);
        c32(&mut app, C32::Face(3));
        let shown = |app: &Faraday| make(app).press_shown(app.now_ms(), still);
        let first = shown(&app);
        let ink_first = ink_at(&mut app, three);
        let mut seen = vec![first];
        for _ in 0..40 {
            let now = app.now_ms() + 16;
            app.event(Event::Tick { now_ms: now });
            seen.push(shown(&app));
        }
        let ink_late = ink_at(&mut app, three);
        if still {
            assert!(seen.iter().all(|s| *s == 0.0), "no emphasis: {seen:?}");
            assert_eq!(ink_first, ink_late, "the same highlight throughout");
        } else {
            assert!(first > 0.99, "emphasised as pressed: {seen:?}");
            assert!(
                seen.windows(2).all(|w| w[1] <= w[0]),
                "only eases: {seen:?}"
            );
            assert!(seen.last().is_some_and(|s| *s < 0.01), "to rest: {seen:?}");
            assert_ne!(ink_first, ink_late, "the emphasis drawn, then gone");
        }
        assert_eq!(make(&app).pressed_face(), Some(3), "still drawn pressed");
    }
}
