//! A guide's steps, ticked off (`docs/COMPANION.md` §4.7, "Steps are
//! ticked off"): a tap on a step's circle ticks it and a second clears
//! it, a drag that scrolls never ticks, the count under the title follows,
//! the ticks come back after a restart through the shell, and Start again
//! clears them.

mod common;

use common::*;
use faraday_companion::{Act, Companion, CompanionCommand, CompanionEvent, Doc, Page};
use faraday_strings::{EN, fill};
use osk_shell_api::{App, Event, TouchPhase};

/// The Windows guide, opened from Start from the stick.
fn guide(c: &mut Companion, id: &'static str) {
    c.press(Act::Start);
    if c.page == Page::StartComputer {
        c.press(Act::Computer(0));
    }
    let at = faraday_companion::start_page(id).expect("a guide Start lists");
    c.press(Act::StartPage(at));
    assert_eq!(c.page, Page::Doc(Doc::Own(id)));
    drain(c);
}

fn steps_of(id: &str) -> usize {
    faraday_learn::steps(faraday_guides::page(id).expect("a guide")).len()
}

fn count(done: usize, of: usize) -> String {
    fill(
        EN.companion_steps_done,
        &[&done.to_string(), &of.to_string()],
    )
}

/// What the shell was last asked to keep for a page's steps.
fn kept_steps(c: &mut Companion) -> Option<(String, Vec<u32>)> {
    let (_, ours) = drain(c);
    ours.into_iter().rev().find_map(|o| match o {
        CompanionCommand::KeepSteps { page, done } => Some((page, done)),
        _ => None,
    })
}

fn touch(c: &mut Companion, x: u16, y: u16, phase: TouchPhase) {
    c.event(Event::Touch { x, y, phase });
}

/// A finger lifted where it went down on step `n`'s circle.
fn tap_step(c: &mut Companion, n: u32) {
    let (x, y) = c.where_offered(Act::Step(n)).expect("the step's circle");
    touch(c, x, y, TouchPhase::Down);
    touch(c, x, y, TouchPhase::Up);
}

#[test]
fn a_tap_ticks_a_step_and_a_second_clears_it() {
    let mut c = phone();
    guide(&mut c, "windows");
    let n = steps_of("windows");
    assert!(shows(&mut c, &count(0, n)), "{:?}", c.texts());

    tap_step(&mut c, 1);
    assert!(shows(&mut c, &count(1, n)));
    assert_eq!(kept_steps(&mut c), Some((String::from("windows"), vec![1])));

    tap_step(&mut c, 2);
    assert!(shows(&mut c, &count(2, n)));
    assert_eq!(
        kept_steps(&mut c),
        Some((String::from("windows"), vec![1, 2]))
    );

    tap_step(&mut c, 1);
    assert!(shows(&mut c, &count(1, n)));
    assert_eq!(kept_steps(&mut c), Some((String::from("windows"), vec![2])));
}

#[test]
fn a_drag_that_scrolls_never_ticks() {
    let mut c = phone();
    guide(&mut c, "maker-dell");
    let n = steps_of("maker-dell");
    let (x, y) = c.where_offered(Act::Step(2)).expect("the step's circle");
    touch(&mut c, x, y, TouchPhase::Down);
    touch(&mut c, x, y - 200, TouchPhase::Move);
    touch(&mut c, x, y, TouchPhase::Move);
    touch(&mut c, x, y, TouchPhase::Up);
    assert!(shows(&mut c, &count(0, n)), "{:?}", c.texts());
    assert_eq!(kept_steps(&mut c), None);
}

#[test]
fn the_ticks_are_per_page_and_come_back_after_a_restart() {
    let mut c = phone();
    guide(&mut c, "maker-hp");
    tap_step(&mut c, 1);
    tap_step(&mut c, 3);
    let (page, done) = kept_steps(&mut c).expect("the shell is asked to keep them");
    assert_eq!((page.as_str(), done.as_slice()), ("maker-hp", &[1, 3][..]));

    // Another page has its own.
    c.press(Act::Back);
    guide(&mut c, "windows");
    assert!(shows(&mut c, &count(0, steps_of("windows"))));

    let mut again = phone();
    again.companion_event(CompanionEvent::Steps { page, done });
    guide(&mut again, "maker-hp");
    assert!(shows(&mut again, &count(2, steps_of("maker-hp"))));
    assert_eq!(again.ticked("maker-hp"), [1, 3]);
}

#[test]
fn kept_ticks_this_build_does_not_know_are_dropped() {
    let mut c = phone();
    c.companion_event(CompanionEvent::Steps {
        page: String::from("maker-hp"),
        done: vec![2, 999],
    });
    c.companion_event(CompanionEvent::Steps {
        page: String::from("no-such-page"),
        done: vec![1],
    });
    assert_eq!(c.ticked("maker-hp"), [2]);
    assert!(c.ticked("no-such-page").is_empty());
}

#[test]
fn start_again_clears_the_page_and_shows_only_once_one_is_ticked() {
    let mut c = phone();
    guide(&mut c, "linux");
    let n = steps_of("linux");
    let end = || Event::Scroll {
        x: 10,
        y: 10,
        dy: i16::MAX,
    };
    c.event(end());
    assert!(!shows(&mut c, EN.companion_steps_again));
    c.event(Event::Scroll {
        x: 10,
        y: 10,
        dy: i16::MIN,
    });
    tap_step(&mut c, 1);
    tap_step(&mut c, 2);
    c.event(end());
    assert!(shows(&mut c, EN.companion_steps_again), "{:?}", c.texts());
    drain(&mut c);
    c.press(Act::StartAgain);
    assert_eq!(kept_steps(&mut c), Some((String::from("linux"), vec![])));
    assert!(c.ticked("linux").is_empty());
    assert!(!shows(&mut c, EN.companion_steps_again));
    c.event(Event::Scroll {
        x: 10,
        y: 10,
        dy: i16::MIN,
    });
    assert!(shows(&mut c, &count(0, n)));
}

#[test]
fn no_step_is_shown_with_its_raw_number() {
    for id in faraday_guides::ids().filter(|id| steps_of(id) > 0) {
        let Some(_) = faraday_companion::start_page(id) else {
            continue;
        };
        let mut c = phone();
        guide(&mut c, id);
        let mut seen = Vec::new();
        for _ in 0..200 {
            seen.extend(c.texts());
            let before = c.texts();
            c.event(Event::Scroll {
                x: 10,
                y: 10,
                dy: 600,
            });
            if c.texts() == before {
                break;
            }
        }
        for t in &seen {
            assert!(
                faraday_learn::step(t).is_none() && !t.contains("::sketch"),
                "{id}: {t}"
            );
        }
        // Every step's circle was offered.
        assert!(seen.iter().any(|t| t == "1"), "{id}");
    }
}
