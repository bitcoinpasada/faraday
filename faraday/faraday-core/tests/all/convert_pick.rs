//! Convert a seed's From step: where the pair stands the keys are the
//! pick's rows in a pane at the card's left, headed by the question, and
//! Next is off until a pick; other sizes keep the rows in the card.

use faraday_bip::bip39::Language;
use faraday_core::convert::{self, ConvertAction as C, cstep};
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// Convert opens on a `w` × `h` window with two keys loaded.
fn on(w: u16, h: u16) -> Faraday {
    let mut app = Faraday::new();
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
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(Vec::<(String, Vec<u8>)>::new()),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let c = testkit::compute();
    for i in 0..2u8 {
        let m = faraday_keys::words::from_entropy(Language::English, &[i + 1; 16]).unwrap();
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&w| Language::English.words()[usize::from(w)])
            .collect();
        app.session
            .keys
            .add_words(&c, &words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
    testkit::keep_published(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Convert a seed")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::Convert);
    app
}

fn fp(app: &Faraday, i: usize) -> [u8; 4] {
    app.session.keys.all().nth(i).unwrap().fingerprint.0
}

fn touch(app: &mut Faraday, a: Action) {
    let _ = app.frame();
    let (x, y) = app.where_offered(a).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

fn shown(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_text_boxes().iter().any(|(t, _)| t == text)
}

#[test]
fn the_keys_stand_in_a_pane_left_of_the_card_headed_by_the_question() {
    let mut app = on(1366, 768);
    assert!(shown(&mut app, "Which seed?"));
    assert!(!shown(&mut app, "Your choices"));
    let edge = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == "From")
        .map(|(_, r)| r.0)
        .min()
        .expect("the step's title");
    for i in 0..2 {
        let (x, _, w, _) = app
            .hit_box(Action::Convert(C::Key(fp(&app, i))))
            .expect("a row");
        assert!(i32::from(x + w) < edge, "the row ends left of the card");
    }
}

#[test]
fn next_is_off_until_a_pick_and_a_press_picks() {
    let mut app = on(1920, 1080);
    let _ = app.frame();
    assert!(!app.offers(Action::Convert(C::Step(cstep::TO))));
    let second = fp(&app, 1);
    touch(&mut app, Action::Convert(C::Key(second)));
    assert_eq!(convert::state(&app).and_then(|s| s.key), Some(second));
    // The pick opens To; From, opened again, has its pick and Next on.
    app.press(Action::Convert(C::Step(cstep::FROM)));
    let _ = app.frame();
    assert!(app.offers(Action::Convert(C::Step(cstep::TO))));
}

#[test]
fn on_a_small_panel_the_card_draws_the_rows_as_before() {
    let mut app = on(480, 640);
    assert!(!shown(&mut app, "Which seed?"));
    let first = fp(&app, 0);
    assert!(app.hit_box(Action::Convert(C::Key(first))).is_some());
    touch(&mut app, Action::Convert(C::Key(first)));
    assert_eq!(convert::state(&app).and_then(|s| s.key), Some(first));
}
