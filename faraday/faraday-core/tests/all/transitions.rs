//! How changes show (`docs/MOTION.md` §3.5): a new screen fades in over
//! the old one and is then still, and so does the guide sheet, its tabs
//! and its closing; at once with reduce motion only, however far apart a
//! machine shows a fade's steps, which is timed for the timings; a step
//! card grows open and comes to rest where it would have jumped to.

use faraday_core::ui::{Contrast, Mode, Theme, ThemeName};
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// Cobalt in Dark.
const DARK: Theme = Theme::new(ThemeName::Cobalt, Mode::Dark, Contrast::Standard);

/// Frost in Dark.
const FROST: Theme = Theme::new(ThemeName::Frost, Mode::Dark, Contrast::Standard);

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
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
    app.event(Event::Tick { now_ms: 1_000 });
    let _ = app.frame();
    app
}

/// Ticks every 16 ms from `from` to `to`, drawing each frame as a shell
/// does, and returns the last.
fn frames(app: &mut Faraday, from: u64, to: u64) -> Vec<u8> {
    let mut now = from;
    loop {
        app.event(Event::Tick { now_ms: now });
        while app.poll_command().is_some() {}
        let _ = app.frame();
        if now >= to {
            return app.frame().rgba.to_vec();
        }
        now = (now + 16).min(to);
    }
}

#[test]
fn a_new_screen_fades_in_over_the_old_and_is_then_still() {
    let mut app = shown();
    let home = app.frame().rgba.to_vec();
    app.press(Action::Nav(Screen::Settings));
    let first = frames(&mut app, 1_016, 1_016);
    let second = frames(&mut app, 1_032, 1_032);
    let rest = frames(&mut app, 1_048, 2_000);
    assert!(first == home, "the fade starts from the old screen");
    assert!(
        second != home && second != rest,
        "and the next frame is part of the way to the new"
    );
    assert!(frames(&mut app, 2_016, 2_400) == rest, "then it is still");
}

#[test]
fn with_reduce_motion_a_new_screen_is_there_at_once() {
    let mut app = shown();
    app.press(Action::ReduceMotion(true));
    let _ = frames(&mut app, 1_016, 1_400);
    app.press(Action::Nav(Screen::Settings));
    let first = frames(&mut app, 1_416, 1_416);
    let rest = frames(&mut app, 1_432, 2_400);
    assert!(first == rest, "the first frame is the screen at rest");
}

/// The screen at rest in `theme`, in its own run.
fn at_rest_in(theme: Theme) -> Vec<u8> {
    let mut app = shown();
    app.press(Action::Theme(theme));
    let _ = frames(&mut app, 1_016, 1_400);
    frames(&mut app, 1_416, 2_400)
}

#[test]
fn a_new_theme_fades_in_over_the_old_and_ends_in_the_new() {
    let mut app = shown();
    let old = app.frame().rgba.to_vec();
    app.press(Action::Theme(DARK));
    let first = frames(&mut app, 1_016, 1_016);
    assert!(first == old, "the fade starts from the old palette");
    let mid = frames(&mut app, 1_032, 1_150);
    let rest = frames(&mut app, 1_166, 2_000);
    assert!(mid != old && mid != rest, "part of the way, a while");
    assert!(rest == at_rest_in(DARK), "and ends in the new");
}

#[test]
fn with_reduce_motion_a_new_theme_is_there_at_once() {
    let mut app = shown();
    app.press(Action::ReduceMotion(true));
    let _ = frames(&mut app, 1_016, 1_400);
    app.press(Action::Theme(DARK));
    let first = frames(&mut app, 1_416, 1_416);
    assert!(first == frames(&mut app, 1_432, 2_400), "at rest at once");
}

#[test]
fn a_second_theme_pressed_mid_fade_ends_in_the_second() {
    let mut app = shown();
    app.press(Action::Theme(DARK));
    let _ = frames(&mut app, 1_016, 1_150);
    app.press(Action::Theme(FROST));
    let rest = frames(&mut app, 1_166, 2_400);
    assert!(rest == at_rest_in(FROST));
}

#[test]
fn the_guide_sheet_fades_in_though_the_frame_it_opens_on_is_slow() {
    let mut app = shown();
    let _ = frames(&mut app, 1_016, 1_400);
    let page = app.frame().rgba.to_vec();
    // The ? button.
    app.press(Action::Guide);
    let first = frames(&mut app, 1_416, 1_416);
    assert!(first == page, "the fade starts from the page");
    // Frosting the page under a sheet makes its first frame the slowest
    // there is: the next tick comes 90 ms later.
    let second = frames(&mut app, 1_506, 1_506);
    let rest = frames(&mut app, 1_522, 2_400);
    assert!(
        second != page && second != rest,
        "the slow frame neither ends the fade nor uses it up"
    );
    // And so does the next sheet, once the slow frame is past.
    app.press(Action::Cancel);
    let _ = frames(&mut app, 2_416, 3_000);
    let page = app.frame().rgba.to_vec();
    app.press(Action::Guide);
    let first = frames(&mut app, 3_016, 3_016);
    let second = frames(&mut app, 3_032, 3_032);
    assert!(first == page && second != page, "the next one fades too");
}

/// Ticks every `step` ms from `from` to `to`, drawing each frame as a
/// shell does; returns the time of the last tick.
fn every(app: &mut Faraday, from: u64, to: u64, step: u64) -> u64 {
    let mut now = from;
    while now <= to {
        app.event(Event::Tick { now_ms: now });
        while app.poll_command().is_some() {}
        let _ = app.frame();
        now += step;
    }
    now - step
}

/// Whether a change of screen made by `press` at `now` fades: the frame
/// after the one that shows it is part of the way.
fn fades(app: &mut Faraday, press: Action, now: u64) -> bool {
    let page = app.frame().rgba.to_vec();
    app.press(press);
    let first = frames(app, now + 16, now + 16);
    let second = frames(app, now + 32, now + 32);
    let rest = frames(app, now + 48, now + 1_000);
    first == page && second != page && second != rest
}

#[test]
fn a_machine_whose_fades_come_in_steps_still_fades_and_times_the_steps() {
    let mut app = shown();
    let _ = app.take_fade_counts();
    // Fades whose steps come 70 ms apart, past four refreshes at 60 Hz,
    // as on a slow machine.
    let mut now = 1_000;
    for screen in [
        Screen::Settings,
        Screen::Home,
        Screen::Settings,
        Screen::Home,
    ] {
        app.press(Action::Nav(screen));
        now = every(&mut app, now + 70, now + 420, 70);
    }
    let counts = app.take_fade_counts();
    assert!(counts.gaps_known > 0, "the steps' gaps are timed");
    assert_eq!(counts.median_gap_ms, Some(70));
    assert!(
        fades(&mut app, Action::Nav(Screen::Settings), now),
        "the next change of screen fades"
    );
}

#[test]
fn the_guide_sheet_fades_out_and_its_tabs_fade_between_pages() {
    let mut app = shown();
    let _ = frames(&mut app, 1_016, 1_400);
    assert!(
        fades(&mut app, Action::Guide, 1_400),
        "Learn opens with a fade"
    );
    assert!(
        fades(&mut app, Action::GuideContents, 2_400),
        "the contents fades in"
    );
    assert!(
        fades(&mut app, Action::Cancel, 3_400),
        "and it closes with a fade"
    );
    assert!(
        fades(&mut app, Action::Nav(Screen::Settings), 4_400),
        "and the next tab of the app fades too"
    );
}

#[test]
fn a_step_card_opens_at_once_and_stays_at_rest() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Backup(0));
    let _ = frames(&mut app, 1_016, 2_000);
    app.press(Action::BQ(faraday_core::qstep::BOARD));
    let early = frames(&mut app, 2_016, 2_048);
    let rest = frames(&mut app, 2_064, 2_600);
    assert!(early == rest, "open at once, not growing");
    assert!(frames(&mut app, 2_616, 3_200) == rest, "and then still");
}
