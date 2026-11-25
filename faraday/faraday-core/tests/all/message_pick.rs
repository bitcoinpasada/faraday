//! Sign a message's wallet step: where the pair stands the wallets are the
//! pick's rows in a pane at the card's left, headed by the question; other
//! sizes keep the buttons in the card.

use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// Sign a message opens on a `w` × `h` window with a key and two wallets
/// loaded: a single-key one it signs for, and a multisig one.
fn on(w: u16, h: u16) -> Faraday {
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
    app.session = testkit::session();
    let c = testkit::compute();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    for id in ["spending", "savings"] {
        let kit = testkit::kits().into_iter().find(|k| k.id == id).unwrap();
        app.session
            .add_wallet_confirmed(
                &c,
                kit.name,
                &kit.descriptor,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .expect("the wallet");
    }
    testkit::keep_published(&mut app);
    app.press(Action::SignMessage);
    assert_eq!(app.screen, Screen::Message);
    app
}

fn shown(app: &mut Faraday, text: &str) -> bool {
    let _ = app.frame();
    app.drawn_text_boxes().iter().any(|(t, _)| t.contains(text))
}

fn wallet(app: &Faraday, kit: &str) -> usize {
    let name = testkit::kits()
        .into_iter()
        .find(|k| k.id == kit)
        .unwrap()
        .name;
    app.session
        .wallets
        .iter()
        .position(|w| w.name == name)
        .unwrap()
}

#[test]
fn the_wallets_stand_in_a_pane_left_of_the_card_headed_by_the_question() {
    let mut app = on(1366, 768);
    assert!(shown(&mut app, "Which wallet signs?"));
    let s = wallet(&app, "spending");
    let edge = app
        .drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == "Address")
        .map(|(_, r)| r.0)
        .min()
        .expect("the step's title");
    let (x, _, w, _) = app.hit_box(Action::MWallet(s)).expect("a row");
    assert!(i32::from(x + w) < edge, "the row ends left of the card");
}

#[test]
fn a_multisig_wallet_stays_dimmed_with_its_reason_and_takes_no_press() {
    let mut app = on(1366, 768);
    assert!(shown(&mut app, "Single-key wallets only"));
    let multi = wallet(&app, "savings");
    let single = wallet(&app, "spending");
    assert!(app.offers(Action::MWallet(single)));
    assert!(!app.offers(Action::MWallet(multi)));
}

#[test]
fn a_press_on_a_row_picks_the_wallet_and_moves_on() {
    let mut app = on(1920, 1080);
    let _ = app.frame();
    let s = wallet(&app, "spending");
    let (x, y) = app.where_offered(Action::MWallet(s)).expect("offered");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let m = testkit::message(&app).unwrap();
    assert_eq!(m.wallet, Some(s));
    assert!(m.done[usize::from(faraday_core::mstep::ADDRESS)]);
}

#[test]
fn on_a_small_panel_the_card_keeps_its_buttons() {
    let mut app = on(480, 640);
    assert!(!shown(&mut app, "Which wallet signs?"));
    let s = wallet(&app, "spending");
    assert!(app.hit_box(Action::MWallet(s)).is_some());
}
