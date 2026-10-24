//! How changes show (`docs/MOTION.md` §3.5): a new screen fades in over
//! the old one and is then still, and so does the Learn sheet, its tabs
//! and its closing; at once with reduce motion, and on a machine whose
//! fades come in steps too far apart to read as one, until a while
//! after; a slow moment, or a page slow to draw, leaves fades on; a
//! step card grows open and comes to rest where it would have jumped
//! to.

use faraday_core::ui::Preset;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

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
fn at_rest_in(theme: Preset) -> Vec<u8> {
    let mut app = shown();
    app.press(Action::Theme(theme));
    let _ = frames(&mut app, 1_016, 1_400);
    frames(&mut app, 1_416, 2_400)
}

#[test]
fn a_new_theme_fades_in_over_the_old_and_ends_in_the_new() {
    let mut app = shown();
    let old = app.frame().rgba.to_vec();
    app.press(Action::Theme(Preset::Dark));
    let first = frames(&mut app, 1_016, 1_016);
    assert!(first == old, "the fade starts from the old palette");
    let mid = frames(&mut app, 1_032, 1_150);
    let rest = frames(&mut app, 1_166, 2_000);
    assert!(mid != old && mid != rest, "part of the way, a while");
    assert!(rest == at_rest_in(Preset::Dark), "and ends in the new");
}

#[test]
fn with_reduce_motion_a_new_theme_is_there_at_once() {
    let mut app = shown();
    app.press(Action::ReduceMotion(true));
    let _ = frames(&mut app, 1_016, 1_400);
    app.press(Action::Theme(Preset::Dark));
    let first = frames(&mut app, 1_416, 1_416);
    assert!(first == frames(&mut app, 1_432, 2_400), "at rest at once");
}

#[test]
fn a_second_theme_pressed_mid_fade_ends_in_the_second() {
    let mut app = shown();
    app.press(Action::Theme(Preset::Dark));
    let _ = frames(&mut app, 1_016, 1_150);
    app.press(Action::Theme(Preset::Nord));
    let rest = frames(&mut app, 1_166, 2_400);
    assert!(rest == at_rest_in(Preset::Nord));
}

#[test]
fn the_learn_sheet_fades_in_though_the_frame_it_opens_on_is_slow() {
    let mut app = shown();
    let _ = frames(&mut app, 1_016, 1_400);
    let page = app.frame().rgba.to_vec();
    // The ? button.
    app.press(Action::Learn);
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
    app.press(Action::Learn);
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
fn a_machine_whose_fades_come_in_steps_gets_the_new_screen_at_once() {
    let mut app = shown();
    // Two fades whose steps come 40 ms apart, as on a slow machine.
    let mut now = 1_000;
    for screen in [Screen::Settings, Screen::Home] {
        app.press(Action::Nav(screen));
        now = every(&mut app, now + 40, now + 400, 40);
    }
    assert!(
        !fades(&mut app, Action::Nav(Screen::Settings), now),
        "no fade: it would show as steps"
    );
    assert!(
        !fades(&mut app, Action::Nav(Screen::Home), now + 2_000),
        "and still none at the next change"
    );
}

#[test]
fn a_slow_moment_does_not_leave_fades_off() {
    let mut app = shown();
    let mut now = 1_000;
    // A fade whose first frame is slow and a step of it slow again, as
    // when a sheet opens on a frosted page with the machine busy.
    app.press(Action::Learn);
    let _ = frames(&mut app, now + 16, now + 16);
    for gap in [120, 16, 90, 16, 16, 16, 16, 16, 16, 16, 16, 16] {
        now += gap;
        let _ = frames(&mut app, now, now);
    }
    app.press(Action::Cancel);
    let _ = frames(&mut app, now + 16, now + 600);
    now += 600;
    // A page that is slow to draw, scrolled: its frames come 50 ms
    // apart, which says what the page costs, not what a fade does.
    app.press(Action::Nav(Screen::Settings));
    let _ = frames(&mut app, now + 16, now + 600);
    now += 600;
    app.event(Event::Wheel {
        x: 700,
        y: 400,
        dy: 48,
    });
    now = every(&mut app, now + 50, now + 800, 50);
    assert!(
        fades(&mut app, Action::Nav(Screen::Home), now),
        "the next change of screen fades"
    );
}

#[test]
fn a_machine_busy_for_a_while_fades_again_once_it_is_past() {
    let mut app = shown();
    let mut now = 1_000;
    for screen in [Screen::Settings, Screen::Home] {
        app.press(Action::Nav(screen));
        now = every(&mut app, now + 40, now + 400, 40);
    }
    assert!(!fades(&mut app, Action::Nav(Screen::Settings), now));
    // Half a minute on, with nothing moving.
    now = every(&mut app, now + 1_050, now + 33_000, 50);
    assert!(
        fades(&mut app, Action::Nav(Screen::Home), now),
        "the slow steps are forgotten"
    );
}

#[test]
fn the_learn_sheet_fades_out_and_its_tabs_fade_between_pages() {
    let mut app = shown();
    let _ = frames(&mut app, 1_016, 1_400);
    assert!(
        fades(&mut app, Action::Learn, 1_400),
        "Learn opens with a fade"
    );
    assert!(
        fades(&mut app, Action::LearnPage(1), 2_400),
        "its next tab fades in"
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
fn a_step_card_grows_open_and_comes_to_rest() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(0));
    let _ = frames(&mut app, 1_016, 2_000);
    app.press(Action::BQ(faraday_core::qstep::BOARD));
    let early = frames(&mut app, 2_016, 2_048);
    let rest = frames(&mut app, 2_064, 2_600);
    assert!(early != rest, "part of the way open");
    assert!(frames(&mut app, 2_616, 3_200) == rest, "and then still");
    assert!(
        app.where_offered(Action::BQ(faraday_core::qstep::BOARD))
            .is_some(),
        "its header is still there to close it"
    );
}
