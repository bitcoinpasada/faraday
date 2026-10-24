//! A fade's steps cost a blend, not a draw (`docs/MOTION.md` §3.5): once
//! the frame that shows a change has drawn the new screen, the steps of
//! the fade that nothing else asks a frame for blend it, kept aside, with
//! the old one, so a screen that is slow to draw, the Learn sheet over a
//! frosted page, fades as smoothly as any other.

use faraday_shell_api::{App, Event};

use crate::{Action, Faraday, Screen};

/// Ticks at `now`, then draws the frame as a shell does; returns the
/// lines of text it drew and its pixels.
fn step(app: &mut Faraday, now: u64) -> (u64, Vec<u8>) {
    let lines = |app: &Faraday| app.canvas.as_ref().map_or(0, |c| c.lines_drawn());
    app.event(Event::Tick { now_ms: now });
    while app.poll_command().is_some() {}
    let before = lines(app);
    let rgba = app.frame().rgba.to_vec();
    (lines(app) - before, rgba)
}

#[test]
fn the_steps_of_a_fade_draw_no_screen_again() {
    let mut app = crate::testkit::started();
    let mut now = 1_000;
    for (name, change) in [
        ("Settings", Action::Nav(Screen::Settings)),
        ("the Learn sheet", Action::Learn),
        ("the Learn sheet's next tab", Action::LearnPage(1)),
        ("Learn closed", Action::Cancel),
        (
            "a new theme",
            Action::Theme(crate::ui::Theme {
                mode: crate::ui::Mode::Dark,
                ..crate::ui::Theme::default()
            }),
        ),
    ] {
        for _ in 0..30 {
            now += 16;
            let _ = step(&mut app, now);
        }
        app.press(change);
        let (drawn, mut last) = step(&mut app, now + 16);
        assert!(drawn > 0, "{name}: the change is drawn");
        for i in 2..10 {
            let (drawn, rgba) = step(&mut app, now + 16 * i);
            assert_eq!(drawn, 0, "{name}: step {i} draws no text");
            assert!(rgba != last, "{name}: step {i} moves the fade on");
            last = rgba;
        }
        now += 16 * 10;
    }
}
