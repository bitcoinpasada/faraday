//! The Krux code tool's Contents step: where the pair stands the wallets
//! and then the keys are the pick's rows in a pane at the card's left,
//! headed by the question; other sizes keep the choices in the card.

use faraday_core::kef_tool::{self, KefToolAction as K, Pick};
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

const WORDS: &str = "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title";

/// The tool opens on a `w` × `h` window with a 24-word key loaded.
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
    app.session.keys.add_words(&c, WORDS, "Krux", None).unwrap();
    testkit::keep_published(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    assert_eq!(app.screen, Screen::KefTool);
    app
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
fn the_key_stands_in_a_pane_left_of_the_card_headed_by_the_question() {
    let mut app = on(1366, 768);
    assert!(shown(&mut app, "What is the code made of?"));
    let edge = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == "Contents")
        .map(|(_, r)| r.0)
        .min()
        .expect("the step's title");
    let (x, _, w, _) = app.hit_box(Action::KefTool(K::Key(0))).expect("a row");
    assert!(i32::from(x + w) < edge, "the row ends left of the card");
    // The choice's buttons are not in the card.
    assert!(app.hit_box(Action::KefTool(K::Pick(Pick::Key))).is_none());
}

#[test]
fn next_is_off_until_text_is_typed_and_a_press_on_a_row_picks() {
    let mut app = on(1920, 1080);
    let _ = app.frame();
    assert!(!app.offers(Action::KefTool(K::Next)));
    touch(&mut app, Action::KefTool(K::Pick(Pick::Type)));
    // Typing is picked: the card holds the field, Continue still off.
    let _ = app.frame();
    assert!(app.offers(Action::KefTool(K::Field)));
    touch(&mut app, Action::KefTool(K::Key(0)));
    let s = kef_tool::state(&app).unwrap();
    assert_eq!(s.key(&app.session.keys), Some(0));
    assert!(s.form().is_some());
}

#[test]
fn on_a_small_panel_the_card_keeps_its_choices() {
    let mut app = on(480, 640);
    assert!(!shown(&mut app, "What is the code made of?"));
    assert!(app.hit_box(Action::KefTool(K::Pick(Pick::Key))).is_some());
}

#[test]
fn a_key_that_cannot_be_a_code_stays_dimmed_beside_one_that_can() {
    let mut app = on(1366, 768);
    let c = testkit::compute();
    let words = format!("{} address", vec!["abandon"; 14].join(" "));
    app.session
        .keys
        .add_words(&c, &words, "Fifteen", None)
        .unwrap();
    let _ = app.frame();
    let has =
        |app: &mut Faraday, w: &str| app.drawn_text_boxes().iter().any(|(t, _)| t.contains(w));
    assert!(has(&mut app, "Fifteen"), "the 15-word key is listed");
    assert!(has(&mut app, "Krux"));
    // The 24-word key takes a press; the 15-word one does not.
    assert!(app.offers(Action::KefTool(K::Key(0))));
    assert!(!app.offers(Action::KefTool(K::Key(1))));
}
