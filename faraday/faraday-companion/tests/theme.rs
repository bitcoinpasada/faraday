//! The theme chosen on About (`docs/COMPANION.md` §4.8): the shell is
//! asked to keep it, and a Companion started again with what was kept
//! draws in it.

mod common;

use common::*;
use faraday_companion::{Act, CompanionCommand, CompanionEvent, Page, Theme};

#[test]
fn the_theme_chosen_is_kept_and_comes_back_after_a_restart() {
    let mut c = phone();
    c.press(Act::About);
    assert_eq!(c.page, Page::About);
    let at = Theme::ALL
        .iter()
        .position(|t| *t == Theme::Gruvbox)
        .expect("Gruvbox is offered");
    c.press(Act::Theme(at));
    assert_eq!(c.theme, Theme::Gruvbox);
    let (_, ours) = drain(&mut c);
    let kept = ours.iter().find_map(|o| match o {
        CompanionCommand::KeepTheme { id } => Some(id.clone()),
        _ => None,
    });
    let kept = kept.expect("the shell is asked to keep the theme");

    let mut again = phone();
    assert_eq!(again.theme, Theme::default());
    again.companion_event(CompanionEvent::Theme { id: kept });
    assert_eq!(again.theme, Theme::Gruvbox);
}

#[test]
fn a_kept_theme_this_build_does_not_know_leaves_the_default() {
    let mut c = phone();
    c.companion_event(CompanionEvent::Theme {
        id: String::from("not-a-theme"),
    });
    assert_eq!(c.theme, Theme::default());
}
