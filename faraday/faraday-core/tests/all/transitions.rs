//! How changes show (`docs/MOTION.md` §3.5): a new screen fades in over
//! the old one and is then still; at once with reduce motion, and on a
//! machine whose frames come too far apart for a fade to read as one; a
//! step card grows open and comes to rest where it would have jumped to.

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

#[test]
fn a_machine_whose_frames_come_far_apart_gets_the_new_screen_at_once() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Settings));
    // Settings is taller than the panel: a notch glides it, and the
    // frames of the glide come 40 ms apart, as on a slow machine.
    let _ = frames(&mut app, 1_016, 1_400);
    app.event(Event::Wheel {
        x: 700,
        y: 400,
        dy: 48,
    });
    let mut now = 1_400;
    for _ in 0..12 {
        now += 40;
        app.event(Event::Tick { now_ms: now });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    }
    app.press(Action::Nav(Screen::Home));
    let first = frames(&mut app, now + 40, now + 40);
    let rest = frames(&mut app, now + 80, now + 1_000);
    assert!(first == rest, "no fade: it would show as steps");
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
