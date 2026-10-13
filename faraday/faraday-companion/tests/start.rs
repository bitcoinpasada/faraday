//! Start from the stick (`docs/COMPANION.md` §4.7): it asks which
//! computer once, then the maker, remembers both through the shell, and
//! shows the pages for that computer.

mod common;

use common::*;
use faraday_companion::{Act, Companion, CompanionCommand, CompanionEvent, Computer, Doc, Page};
use faraday_strings::EN;

/// What the shell was last asked to keep for Start from the stick.
fn kept_start(c: &mut Companion) -> Option<(String, String)> {
    let (_, ours) = drain(c);
    ours.into_iter().rev().find_map(|o| match o {
        CompanionCommand::KeepStart { computer, maker } => Some((computer, maker)),
        _ => None,
    })
}

fn maker_at(id: &str) -> usize {
    faraday_learn::MAKERS
        .iter()
        .position(|m| m.id == id)
        .expect("a maker page")
}

fn restarted(computer: &str, maker: &str) -> Companion {
    let mut c = phone();
    c.companion_event(CompanionEvent::Start {
        computer: computer.to_string(),
        maker: maker.to_string(),
    });
    c
}

fn title(id: &str) -> &'static str {
    faraday_guides::page(id).expect("a guide").title
}

#[test]
fn the_first_visit_asks_which_computer() {
    let mut c = phone();
    c.press(Act::Start);
    assert_eq!(c.page, Page::StartComputer);
    assert!(shows(&mut c, EN.companion_start_question));
    assert!(shows(&mut c, EN.companion_start_windows));
    assert!(shows(&mut c, EN.companion_start_linux));
    assert_eq!(kept_start(&mut c), None);
}

#[test]
fn the_computer_chosen_is_kept_and_opens_its_page_after_a_restart() {
    let mut c = phone();
    c.press(Act::Start);
    c.press(Act::Computer(0));
    assert_eq!(c.computer, Some(Computer::Windows));
    assert_eq!(c.page, Page::Start);
    assert!(shows(&mut c, title("windows")));
    let (computer, maker) = kept_start(&mut c).expect("the shell is asked to keep it");

    let mut again = restarted(&computer, &maker);
    again.press(Act::Start);
    assert_eq!(again.page, Page::Start);
    assert!(shows(&mut again, EN.companion_start_windows));
    assert!(!shows(&mut again, EN.companion_start_question));

    // Back from the page goes to Home, not to the question.
    again.press(Act::Back);
    assert_eq!(again.page, Page::Home);
}

#[test]
fn the_maker_chosen_is_kept_and_shown_after_a_restart() {
    let mut c = restarted("linux", "");
    c.press(Act::Start);
    assert!(shows(&mut c, EN.companion_start_choose_maker));
    c.press(Act::ChooseMaker);
    assert_eq!(c.page, Page::StartMakers);
    // The makers' names alone, not their pages' titles.
    for name in [
        EN.companion_maker_dell,
        EN.companion_maker_hp,
        EN.companion_maker_lenovo_thinkpad,
        EN.companion_maker_lenovo_ideapad,
        EN.companion_maker_asus,
        EN.companion_maker_acer,
        EN.companion_maker_msi,
        EN.companion_maker_framework,
        EN.companion_maker_surface,
    ] {
        assert!(shows(&mut c, name), "{name}");
    }
    assert!(!shows(&mut c, title("maker-dell")));
    assert!(shows(&mut c, EN.companion_start_another_maker));
    c.press(Act::Maker(maker_at("maker-dell")));
    assert_eq!(c.page, Page::Start);
    assert!(shows(&mut c, EN.companion_maker_dell));
    assert!(shows(&mut c, title("maker-dell")));
    assert!(!shows(&mut c, EN.companion_start_choose_maker));
    let (computer, maker) = kept_start(&mut c).expect("the shell is asked to keep both");
    assert_eq!((computer.as_str(), maker.as_str()), ("linux", "maker-dell"));

    let mut again = restarted(&computer, &maker);
    again.press(Act::Start);
    assert!(shows(&mut again, title("maker-dell")));
    assert!(shows(&mut again, EN.companion_start_change));
    again.press(Act::Back);
    assert_eq!(again.page, Page::Home);
}

#[test]
fn change_asks_again_and_leaving_unanswered_keeps_what_was() {
    let mut c = restarted("windows", "maker-dell");
    c.press(Act::Start);
    c.press(Act::ChangeComputer);
    assert_eq!(c.page, Page::StartComputer);
    assert!(shows(&mut c, EN.companion_start_question));
    c.press(Act::Back);
    assert_eq!(c.computer, Some(Computer::Windows));
    assert_eq!(c.page, Page::Start);
    assert_eq!(kept_start(&mut c), None);

    c.press(Act::ChangeComputer);
    c.press(Act::Computer(1));
    assert_eq!(c.computer, Some(Computer::Linux));
    assert_eq!(c.page, Page::Start);
    assert!(shows(&mut c, title("linux")));
    // Another computer, so the maker is asked again.
    assert!(shows(&mut c, EN.companion_start_choose_maker));
    assert_eq!(
        kept_start(&mut c),
        Some((String::from("linux"), String::new()))
    );
    c.press(Act::Back);
    assert_eq!(c.page, Page::Home);
}

#[test]
fn changing_the_maker_lists_the_makers_again() {
    let mut c = restarted("windows", "maker-dell");
    c.press(Act::Start);
    c.press(Act::ChooseMaker);
    assert_eq!(c.page, Page::StartMakers);
    c.press(Act::Maker(maker_at("maker-asus")));
    assert!(shows(&mut c, title("maker-asus")));
    assert_eq!(
        kept_start(&mut c),
        Some((String::from("windows"), String::from("maker-asus")))
    );
}

#[test]
fn windows_shows_the_bitlocker_pages_and_linux_does_not() {
    let mut c = restarted("windows", "");
    c.press(Act::Start);
    assert!(shows(&mut c, EN.companion_start_bitlocker));
    assert!(shows(&mut c, title("bitlocker")));
    assert!(shows(&mut c, title("secure-boot-bitlocker")));
    assert!(shows(&mut c, EN.companion_start_trouble));
    assert!(shows(&mut c, title("not-starting")));

    let mut c = restarted("linux", "");
    c.press(Act::Start);
    assert!(!shows(&mut c, EN.companion_start_bitlocker));
    assert!(!shows(&mut c, title("bitlocker")));
    assert!(shows(&mut c, title("linux")));
    assert!(!shows(&mut c, title("windows")));
    assert!(shows(&mut c, title("not-starting")));
    assert!(shows(&mut c, title("screens-that-record")));
    assert!(shows(&mut c, title("replace-faraday")));
}

#[test]
fn another_maker_opens_the_systems_general_guide() {
    for (computer, id) in [("windows", "windows"), ("linux", "linux")] {
        let mut c = restarted(computer, "");
        c.press(Act::Start);
        c.press(Act::ChooseMaker);
        let general = faraday_guides::BOOT
            .iter()
            .position(|b| *b == id)
            .expect("a boot guide");
        c.press(Act::StartPage(general));
        assert_eq!(c.page, Page::Doc(Doc::Own(id)));
        assert_eq!(kept_start(&mut c), None);
        assert_eq!(c.maker, None);
    }
}

#[test]
fn a_guide_address_opens_its_page_and_changes_nothing_remembered() {
    let mut c = restarted("windows", "maker-dell");
    drain(&mut c);
    c.press(Act::Receive);
    paste(&mut c, &faraday_learn::guide_address("linux"));
    assert_eq!(c.page, Page::Doc(Doc::Own("linux")));
    assert_eq!(kept_start(&mut c), None);
    assert_eq!(c.computer, Some(Computer::Windows));
    assert_eq!(c.maker, Some("maker-dell"));
}

#[test]
fn a_kept_answer_this_build_does_not_know_asks_again() {
    let mut c = restarted("beos", "maker-dell");
    assert_eq!(c.computer, None);
    c.press(Act::Start);
    assert_eq!(c.page, Page::StartComputer);

    let c = restarted("linux", "maker-nobody");
    assert_eq!(c.computer, Some(Computer::Linux));
    assert_eq!(c.maker, None);
}
