//! A start guide's steps, in the Starter's own sheet: the number in a
//! circle, and the text without its "n. " prefix.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_starter::{StartGuide, Starter, StarterAction};

fn new_starter() -> Starter {
    let mut starter = Starter::new();
    starter.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    starter
}

fn open_hp(starter: &mut Starter) {
    starter.press(StarterAction::StartGuide(StartGuide::Maker("maker-hp")));
}

#[test]
fn a_step_shows_its_circle_and_its_text_without_the_prefix() {
    let mut starter = new_starter();
    open_hp(&mut starter);
    let seen = starter.drawn_texts();
    // The circle's own label, drawn apart from the paragraph.
    assert!(seen.iter().any(|t| t == "1"), "{seen:?}");
    // No paragraph is shown with its raw "n. " prefix.
    for t in &seen {
        assert!(faraday_learn::step(t).is_none(), "{t}");
    }
    // The text after the step's number is still shown, just not as
    // "1. Plug the stick in.".
    assert!(seen.iter().any(|t| t == "Plug the stick in."), "{seen:?}");
}
