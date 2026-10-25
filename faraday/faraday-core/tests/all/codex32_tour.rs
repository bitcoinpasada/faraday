//! The tour of the shares made and S past the last checksum
//! (`docs/CODEX32.md` §5.7), as a person stepping through it on New key
//! would notice it: Next at the last checksum box opens Share D's tab,
//! the switch beside the Working area turns the tour off, and the page
//! and the worksheet's view go to a step's cells.

use faraday_core::codex32::{C32, Codex32Make, TourStep};
use faraday_core::{Action, Faraday};
use faraday_shell_api::{App, Event};

fn c32(app: &mut Faraday, p: C32) {
    app.press(Action::Codex32(p));
}

fn make(app: &Faraday) -> &Codex32Make {
    app.keygen().unwrap().codex32.as_ref().unwrap()
}

/// `n` frames 16 ms apart, a tick before each, as a shell draws them.
fn frames(app: &mut Faraday, n: usize) {
    let mut t = app.now_ms();
    for _ in 0..n {
        t += 16;
        app.event(Event::Tick { now_ms: t });
        let _ = app.frame();
    }
}

/// Vector 2's A and C typed at `w` × `h`, the Working area stepped to
/// the last checksum box.
fn at_last_checksum(w: u16, h: u16) -> Faraday {
    let mut app = crate::codex32_make::both_typed_in(faraday_ui::Theme::default(), w, h);
    for _ in 0..200 {
        if make(&app).track_at_end() {
            break;
        }
        c32(&mut app, C32::Step(1));
    }
    frames(&mut app, 30);
    app
}

/// Next until the tour shows `want`, a frame after each.
fn to(app: &mut Faraday, want: TourStep) {
    for _ in 0..1200 {
        if make(app).tour_shown().is_some_and(|s| s.1 == want) {
            return;
        }
        c32(app, C32::Step(1));
        let _ = app.frame();
    }
    panic!("the tour never shows {want:?}");
}

#[test]
fn next_at_the_last_checksum_box_opens_share_ds_tab_and_the_switch_turns_it_off() {
    let mut app = at_last_checksum(1280, 800);
    let tour = Action::Codex32(C32::Tour);
    assert!(app.offers(tour), "the switch beside the Working area");
    c32(&mut app, C32::Step(1));
    let _ = app.frame();
    assert_eq!(make(&app).view(), 1, "Share D's tab");
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t.starts_with("Share A's translation symbol for D")),
        "its first step in the Working area"
    );
    c32(&mut app, C32::Step(-1));
    app.press(tour);
    c32(&mut app, C32::Step(1));
    assert_eq!(make(&app).tour_shown(), None, "off, no tour");
    assert_eq!(make(&app).view(), 0);
}

#[test]
fn a_late_check_row_is_brought_into_view() {
    let mut app = at_last_checksum(1280, 800);
    // The check's first cell, at the worksheet's top.
    let first = faraday_core::codex32::Step::Sum { j: 0, i: 0 };
    to(&mut app, TourStep::Check { t: 0, step: first });
    frames(&mut app, 40);
    // How far down the page and the worksheet's view stand together.
    let down = |app: &Faraday| app.keygen().unwrap().scroll.y + make(app).pan().y;
    let start = down(&app);
    let step = faraday_core::codex32::Step::Sum { j: 14, i: 3 };
    to(&mut app, TourStep::Check { t: 0, step });
    frames(&mut app, 40);
    let now = down(&app);
    assert!(
        now > start + 300.0,
        "the view went down to row 14: {start} → {now}"
    );
}
