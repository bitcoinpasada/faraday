//! A typed address is shown whole in Check an address' field: the field
//! grows to the lines it needs, the address in groups of four, and what
//! it holds, its caret and a paste stay the characters typed, with no
//! space in them (`docs/TEXT-CURSOR.md` §3).

use faraday_core::edit::Here;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{
    App, BootState, DisplayInfo, Event, FileKind, Key, SecureHardware, TouchPhase,
};

const SIZES: [(u16, u16); 3] = [(320, 240), (480, 640), (1920, 1080)];

/// 62 and 90 characters, none repeating a neighbour's place.
fn typed_of(n: usize) -> String {
    "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq02468acdefghjklmnpqrstuvwxyz13579acdefghjklmnpqrstuv"
        .chars()
        .take(n)
        .collect()
}

fn on_page(w: u16, h: u16) -> Faraday {
    let mut app = testkit::started();
    app.event(Event::Display(DisplayInfo {
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
    }));
    app.press(Action::Nav(Screen::Catalog));
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check an address")
        .unwrap();
    app.press(Action::Catalog(i as u8));
    assert_eq!(app.screen, Screen::CheckAddress);
    app
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
}

fn field(app: &Faraday) -> String {
    app.address_check_state()
        .unwrap()
        .typed()
        .as_str()
        .to_string()
}

/// The groups of the field drawn in the lines in view.
fn in_view(app: &mut Faraday, whole: &str, seen: &mut Vec<String>) {
    for _ in 0..30 {
        let _ = app.frame();
    }
    for t in app.drawn_texts() {
        let t = t.as_str().to_string();
        if t.replace(' ', "").len() >= 2 && whole.contains(&t.replace(' ', "")) {
            seen.push(t);
        }
    }
}

/// The lines of the field drawn, strings of groups of four. A small
/// screen's keyboard leaves room for a line or two: the page keeps the
/// line with the caret in view, so the lines are seen as a person sees
/// them, the caret moved up a line at a time.
fn grouped_lines(app: &mut Faraday, whole: &str) -> Vec<String> {
    let mut seen = Vec::new();
    in_view(app, whole, &mut seen);
    for _ in 0..12 {
        app.event(Event::Key(Key::Up));
        in_view(app, whole, &mut seen);
    }
    let mut lines: Vec<String> = Vec::new();
    let mut off = 0;
    while off < whole.len() {
        let Some(t) = seen
            .iter()
            .find(|t| whole[off..].starts_with(&t.replace(' ', "")))
        else {
            break;
        };
        off += t.replace(' ', "").len();
        lines.push(t.clone());
    }
    lines
}

#[test]
fn every_character_of_a_long_address_is_shown_in_groups_of_four() {
    for (w, h) in SIZES {
        for n in [62, 90] {
            let mut app = on_page(w, h);
            let a = typed_of(n);
            type_in(&mut app, &a);
            let lines = grouped_lines(&mut app, &a);
            assert_eq!(
                lines.concat().replace(' ', ""),
                a,
                "{w}x{h}, {n} characters"
            );
            for l in &lines {
                assert!(l.split(' ').all(|g| g.len() <= 4), "{l}");
            }
            assert_eq!(field(&app), a, "no space is in the text");
        }
    }
}

#[test]
fn the_caret_stays_between_the_characters_typed() {
    for (w, h) in SIZES {
        let mut app = on_page(w, h);
        let a = typed_of(62);
        type_in(&mut app, &a);
        for _ in 0..6 {
            app.event(Event::Key(Key::Left));
        }
        app.event(Event::Key(Key::Char('X')));
        app.event(Event::Key(Key::Backspace));
        app.event(Event::Key(Key::Backspace));
        app.event(Event::Key(Key::Delete));
        let mut want = a.clone();
        want.replace_range(55..57, "");
        assert_eq!(field(&app), want, "{w}x{h}");
        let lines = grouped_lines(&mut app, &want);
        assert_eq!(lines.concat().replace(' ', ""), want);
    }
}

#[test]
fn a_press_on_the_last_line_places_the_caret_there() {
    for (w, h) in SIZES {
        let mut app = on_page(w, h);
        let a = typed_of(90);
        type_in(&mut app, &a);
        let _ = app.frame();
        let _ = app.frame();
        let here = Action::TypeHere(Here::CheckAddress);
        let at = app.where_typed(here, 84);
        let (x, y) = at.expect("the edge drawn");
        assert!(
            app.keyboard_top().is_none_or(|k| y < k),
            "{w}x{h}: in view above the keyboard"
        );
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch { x, y, phase });
        }
        app.event(Event::Key(Key::Char('Z')));
        let mut want = a.clone();
        want.insert(84, 'Z');
        assert_eq!(field(&app), want, "{w}x{h}");
    }
}

#[test]
fn a_pasted_address_is_shown_whole_and_checked() {
    let mut app = on_page(480, 640);
    app.clipboard = true;
    let a = typed_of(62);
    app.press(Action::AddressCheck(
        faraday_core::address_check::AddressPress::Paste,
    ));
    app.event(Event::Clipboard {
        kind: FileKind::Text,
        text: faraday_text::Text::of(&a),
    });
    let lines = grouped_lines(&mut app, &a);
    assert_eq!(lines.concat().replace(' ', ""), a);
}

#[test]
fn up_and_down_move_the_caret_a_line_in_a_grown_field() {
    let mut app = on_page(320, 240);
    let a = typed_of(62);
    type_in(&mut app, &a);
    app.event(Event::Key(Key::Up));
    let _ = app.frame();
    app.event(Event::Key(Key::Char('Z')));
    let up = field(&app).find('Z').expect("typed");
    assert!((24..52).contains(&up), "a line up: {up}");
    app.event(Event::Key(Key::Down));
    let _ = app.frame();
    app.event(Event::Key(Key::Char('Y')));
    let down = field(&app).find('Y').expect("typed");
    assert!(down > up + 10, "a line down again: {down}");
}
