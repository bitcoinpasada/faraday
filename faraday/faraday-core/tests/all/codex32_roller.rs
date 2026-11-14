//! Faraday Demo's dice roller in New key (`docs/CODEX32.md` §5.6): the
//! one die beside the dice pad for BIP-39 words; a device shell has no
//! such die.

use faraday_core::keygen::{Start, kstep};
use faraday_core::vault::RollerAct;
use faraday_core::{Action, Faraday};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// The app with its randomness answered, on a shell that offers the
/// rollers (or not), Reduce motion on (or not).
fn app(offered: bool, reduce: bool) -> Faraday {
    let mut app = Faraday::new();
    // Tall enough for a whole card, the pad and the roller in view.
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.set_roller_offered(offered);
    if offered {
        // The roller is Practice's alone (`docs/DEMO.md` §14), with its
        // sample set, BIP 93's vector 2.
        faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
        app.session.set_demo(crate::codex32_rest::samples());
        app.samples = Some(crate::codex32_rest::samples());
    }
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x23 + i * 9; 32])));
    }
    if reduce {
        app.press(Action::ReduceMotion(true));
    }
    app
}

fn shows(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == text)
}

/// New key opened on Add a key, 12 words, dice by the hashed procedure,
/// on the entries.
fn on_bip39_dice(offered: bool) -> Faraday {
    let mut app = app(offered, true);
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KNext);
    app.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    if app.keygen().unwrap().open != Some(kstep::ENTER) {
        app.press(Action::KNext);
    }
    assert_eq!(app.keygen().unwrap().open, Some(kstep::ENTER));
    app
}

#[test]
fn a_device_shell_has_no_die_beside_new_keys_rolls() {
    let mut app = on_bip39_dice(false);
    assert!(!shows(&mut app, "Rolled by this computer"));
    app.press(Action::KRoller(RollerAct::Roll));
    assert_eq!(app.keygen().unwrap().dice.len(), 0);
}
