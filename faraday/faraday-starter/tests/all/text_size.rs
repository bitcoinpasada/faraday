//! Text size in the Starter (F85, `docs/TEXT-SIZE.md` §6, tests 8 and
//! 9): chosen on About, the pages and the held Transfer drawn at it, and
//! kept by an installed Starter alone.

use faraday_draw::canvas::InkKind;
use faraday_shell_api::App;
use faraday_starter::api::{InstallItem, Installed, StarterCommand, StarterEvent};
use faraday_starter::{Page, Starter, StarterAction};
use faraday_ui::TextSize;

use super::starter::{display, linux, payload};

fn shown() -> Starter {
    let mut s = Starter::new();
    s.event(display(1280, 800, 160));
    s.starter_event(StarterEvent::Computer(linux(true)));
    s.starter_event(StarterEvent::Payload(Some(payload("0.2.0"))));
    s
}

fn installed() -> StarterEvent {
    StarterEvent::Installed(Some(Installed {
        version: "0.2.0".into(),
        items: vec![(InstallItem::App, "~/.local/bin/faraday-starter".into())],
    }))
}

/// About, opened from the ⋮ menu, and `text` pressed on it.
fn choose(s: &mut Starter, text: TextSize) {
    s.press(StarterAction::Menu);
    s.press(StarterAction::About);
    assert_eq!(s.page, Page::About);
    let _ = s.drawn_texts();
    assert!(s.offers_action(StarterAction::TextSize(text)), "{text:?}");
    s.press(StarterAction::TextSize(text));
    assert_eq!(s.text_size(), text);
    s.press(StarterAction::Back);
}

/// The height of `text`'s ink in the Starter's frame, pixels.
fn height(s: &mut Starter, text: &str) -> i32 {
    s.drawn_ink()
        .into_iter()
        .find_map(|i| match i.kind {
            InkKind::Text { text: t, .. } if t == text => Some(i.rect.h),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{text:?} drawn"))
}

/// The height of `text`'s ink in the held app's frame, pixels.
fn held_height(s: &mut Starter, text: &str) -> i32 {
    s.held()
        .drawn_text_boxes()
        .into_iter()
        .find_map(|(t, r)| (t == text).then_some(r.3))
        .unwrap_or_else(|| panic!("{text:?} drawn"))
}

/// What the Starter asked of the shell since it was last asked.
fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

#[test]
fn about_offers_every_text_size_and_larger_draws_home_and_transfer_larger() {
    let mut s = shown();
    assert_eq!(s.text_size(), TextSize::Standard);
    let home = height(&mut s, "This computer");
    s.press(StarterAction::ToolsPage);
    s.press(StarterAction::Transfer);
    assert!(s.showing_held());
    let send = held_height(&mut s, "Send");
    s.press(StarterAction::Back);
    s.press(StarterAction::Home);

    choose(&mut s, TextSize::Larger);
    assert_eq!(s.page, Page::Home);
    let larger = height(&mut s, "This computer");
    assert!(larger > home, "Home's label: {larger} at Larger, {home}");
    s.press(StarterAction::ToolsPage);
    s.press(StarterAction::Transfer);
    assert_eq!(s.held().text_size(), TextSize::Larger);
    let held = held_height(&mut s, "Send");
    assert!(held > send, "Transfer's Send: {held} at Larger, {send}");
}

#[test]
fn an_installed_starter_keeps_its_text_size_and_opens_at_it_again() {
    let mut s = shown();
    s.starter_event(installed());
    let _ = commands(&mut s);
    choose(&mut s, TextSize::Larger);
    assert_eq!(
        commands(&mut s),
        [StarterCommand::KeepTextSize(TextSize::Larger)]
    );
    // Chosen again, nothing new to keep.
    choose(&mut s, TextSize::Larger);
    assert_eq!(commands(&mut s), []);

    // Started again: the shell reads the file before anything is shown.
    let mut again = shown();
    let standard = height(&mut again, "This computer");
    again.starter_event(StarterEvent::TextKept(TextSize::Larger));
    again.starter_event(installed());
    assert_eq!(again.text_size(), TextSize::Larger);
    assert_eq!(again.held().text_size(), TextSize::Larger);
    assert!(height(&mut again, "This computer") > standard);
    assert_eq!(commands(&mut again), [], "nothing to write");
}

#[test]
fn a_starter_run_without_installing_keeps_its_text_size_for_the_run_only() {
    let mut s = shown();
    s.starter_event(StarterEvent::Installed(None));
    choose(&mut s, TextSize::Large);
    assert_eq!(s.text_size(), TextSize::Large);
    assert!(
        !commands(&mut s)
            .iter()
            .any(|c| matches!(c, StarterCommand::KeepTextSize(_))),
        "nothing written"
    );
    // Installed later in the run: the size chosen is kept from then on.
    s.starter_event(installed());
    assert_eq!(
        commands(&mut s),
        [StarterCommand::KeepTextSize(TextSize::Large)]
    );
    // Uninstalled: the file goes with it, and nothing is kept after.
    s.starter_event(StarterEvent::Installed(None));
    choose(&mut s, TextSize::Larger);
    assert_eq!(commands(&mut s), []);
}
