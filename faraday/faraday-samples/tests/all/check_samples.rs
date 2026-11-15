//! Check's rows that open without a file in Practice: Check a multisig
//! setup and the dice entry of a seed made from dice rolls each hand out
//! a sample (`docs/MODES.md` §6.1).

use crate::common::{demo_app, strip, strip_offers};
use faraday_core::keygen::{Way, kstep};
use faraday_core::multisig_check::MsAction;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event};

#[test]
fn in_practice_check_a_multisig_setup_offers_a_sample_that_reads_as_a_setup() {
    let mut app = demo_app();
    app.press(Action::MultisigCheck(MsAction::Open));
    assert_eq!(app.screen, Screen::MultisigCheck);
    assert!(strip_offers(&mut app, Action::UseSample));
    app.press(Action::UseSample);
    let _ = app.frame();
    let texts = app.drawn_texts();
    for k in ["Key 1", "Key 2", "Key 3"] {
        assert!(texts.iter().any(|t| t == k), "{k}: {texts:?}");
    }
}

#[test]
fn in_practice_a_sample_setup_with_a_changed_checksum_is_refused() {
    let mut app = demo_app();
    app.press(Action::MultisigCheck(MsAction::Open));
    for _ in 0..8 {
        app.press(Action::UseSample);
    }
    // The card opens over a moment: let it finish before reading it.
    let at = app.now_ms();
    app.event(Event::Tick { now_ms: at + 2000 });
    let _ = app.frame();
    let lines: Vec<String> = app
        .drawn_texts()
        .iter()
        .map(|t| t.as_str().to_string())
        .collect();
    assert!(
        lines
            .iter()
            .any(|t| t.starts_with("The descriptor checksum")),
        "{lines:?}"
    );
}

#[test]
fn in_practice_the_dice_entry_offers_the_guides_rolls_and_makes_the_guides_key() {
    let mut app = demo_app();
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KNext);
    app.press(Action::KWay(Way::DiceWords.index()));
    assert_eq!(app.keygen().unwrap().open, Some(kstep::ENTER));
    assert!(strip_offers(&mut app, Action::UseSample));
    app.press(Action::UseSample);
    assert!(!strip_offers(&mut app, Action::UseSample));
    app.press(Action::KNext);
    assert_eq!(app.keygen().unwrap().open, Some(kstep::KEY));
    let lines = strip(&mut app);
    assert!(
        lines
            .iter()
            .any(|t| t.contains("These words are sample Guide key · 3dca6805")),
        "{lines:?}"
    );
}

#[test]
fn in_practice_the_xpub_calculator_offers_a_sample_that_it_reads() {
    let mut app = demo_app();
    app.press(Action::Tools);
    app.press(Action::TTool(3));
    assert_eq!(app.screen, Screen::Tools);
    assert!(strip_offers(&mut app, Action::UseSample));
    app.press(Action::UseSample);
    assert!(!strip_offers(&mut app, Action::UseSample));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.contains("pub")), "{texts:?}");
}
