//! A guide's steps, drawn as the Companion draws them
//! (`docs/COMPANION.md` §4.7), in Faraday Starter's own Learn sheet: a
//! tap on a step's circle ticks it and a second clears it, and the ticks
//! survive leaving and reopening the page in the same session.

use faraday_core::Action;
use faraday_starter::{LearnId, Starter, StarterAction};
use osk_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

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

fn touch(starter: &mut Starter, x: u16, y: u16, phase: TouchPhase) {
    starter.event(Event::Touch { x, y, phase });
}

/// A finger lifted where it went down on step `n`'s circle.
fn tap_step(starter: &mut Starter, n: u32) {
    let _ = starter.drawn_texts();
    let (x, y) = starter
        .held()
        .where_offered(Action::LearnStep(n))
        .expect("the step's circle");
    touch(starter, x, y, TouchPhase::Down);
    touch(starter, x, y, TouchPhase::Up);
}

fn open_hp(starter: &mut Starter) {
    starter.press(StarterAction::LearnPage(LearnId::Maker("maker-hp")));
}

fn hp_page() -> &'static osk_learn::Page {
    faraday_guides::page("maker-hp").expect("the HP guide")
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

#[test]
fn a_tap_ticks_a_step_and_a_second_clears_it() {
    let mut starter = new_starter();
    open_hp(&mut starter);
    let page = hp_page();
    assert!(starter.held().learn_ticked(page).is_empty());

    tap_step(&mut starter, 1);
    assert_eq!(starter.held().learn_ticked(page), [1]);

    tap_step(&mut starter, 2);
    assert_eq!(starter.held().learn_ticked(page), [1, 2]);

    tap_step(&mut starter, 1);
    assert_eq!(starter.held().learn_ticked(page), [2]);
}

#[test]
fn ticks_survive_leaving_and_reopening_the_page() {
    let mut starter = new_starter();
    open_hp(&mut starter);
    let page = hp_page();
    tap_step(&mut starter, 1);
    tap_step(&mut starter, 3);
    assert_eq!(starter.held().learn_ticked(page), [1, 3]);

    // Leaving the sheet and opening it again on the same page.
    starter.held().press(Action::Cancel);
    open_hp(&mut starter);
    assert_eq!(starter.held().learn_ticked(page), [1, 3]);

    // A fresh session starts unticked.
    let mut again = new_starter();
    open_hp(&mut again);
    assert!(again.held().learn_ticked(page).is_empty());
}
