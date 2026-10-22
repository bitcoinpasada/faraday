//! Load ticked with a stick attached: keys do not load while a stick is
//! in, so the Pull sheet asks for it out and the load goes on when it is
//! pulled.

use crate::common::demo_app;
use faraday_core::samples::Pick;
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent};
use faraday_samples::SAMPLES;
use faraday_shell_api::App;

fn savings() -> Pick {
    let at = SAMPLES
        .wallets
        .iter()
        .position(|w| w.name == "sample_savings");
    Pick::Wallet(at.expect("a sample wallet") as u8)
}

fn stick(app: &mut Faraday, attached: bool) {
    sticks_in(app, attached, false);
}

fn sticks_in(app: &mut Faraday, attached: bool, boot: bool) {
    let sticks = if attached {
        vec![StickInfo {
            serial: None,
            id: "stick".into(),
            label: "STICK".into(),
            boot,
            files: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    app.storage(StorageEvent::Sticks(sticks));
    let _ = app.frame();
}

/// The Load samples sheet with Savings ticked and Load ticked pressed.
fn load_savings(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app.press(Action::Samples);
    let _ = app.frame();
    app.press(Action::SampleTick(savings()));
    let _ = app.frame();
    app.press(Action::SamplesLoad);
}

#[test]
fn load_ticked_with_a_stick_in_asks_for_it_out_and_then_loads() {
    let mut app = demo_app();
    stick(&mut app, true);
    load_savings(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert!(app.session.wallets.is_empty(), "nothing loads with it in");
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("load the samples")),
        "{texts:?}"
    );
    stick(&mut app, false);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    assert_eq!(app.toast_text(), Some("Loaded 1 wallet and 3 keys"));
}

#[test]
fn not_now_on_the_pull_sheet_loads_nothing_even_when_the_stick_goes() {
    let mut app = demo_app();
    stick(&mut app, true);
    load_savings(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Pull));
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    stick(&mut app, false);
    assert!(app.session.wallets.is_empty());
    assert!(app.session.keys.is_empty());
}

#[test]
fn load_ticked_with_no_stick_loads_at_once() {
    let mut app = demo_app();
    load_savings(&mut app);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
}

fn says_pull(app: &mut Faraday) -> bool {
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app.press(Action::Samples);
    let _ = app.frame();
    app.drawn_texts()
        .iter()
        .any(|t| t.starts_with("Pull the") && t.contains("to load the samples"))
}

#[test]
fn the_sheet_says_to_pull_the_stick_while_one_is_in() {
    let mut app = demo_app();
    assert!(!says_pull(&mut app), "no stick, no line");
    stick(&mut app, true);
    assert!(says_pull(&mut app));
    app.press(Action::SampleTick(savings()));
    let _ = app.frame();
    assert!(
        app.hit_box(Action::SamplesLoad).is_some(),
        "Load ticked can be pressed with the stick in"
    );
    stick(&mut app, false);
    let _ = app.frame();
    assert!(!app.drawn_texts().iter().any(|t| t.starts_with("Pull the")));
    sticks_in(&mut app, true, true);
    assert!(says_pull(&mut app), "the boot stick too");
}

#[test]
fn load_ticked_with_the_boot_stick_in_asks_for_it_out_and_then_loads() {
    let mut app = demo_app();
    sticks_in(&mut app, true, true);
    load_savings(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert!(app.session.wallets.is_empty());
    sticks_in(&mut app, false, true);
    assert_eq!(app.sheet, None);
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    assert!(!app.session.keys.is_empty());
}
