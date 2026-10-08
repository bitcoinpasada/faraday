//! Scrolling as the shell API states it: `dy` pixels the content moves
//! up. A wheel notch (48 pixels) moves a page by 48, not half a page; a
//! list moves by the same pixels too, continuously, not a row at a time.

use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use osk_shell_api::{App, Event};

fn scroll(app: &mut Faraday, dy: i16) {
    app.event(Event::Scroll { x: 600, y: 400, dy });
}

#[test]
fn a_notch_moves_the_page_by_its_pixels_in_the_direction_asked() {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let mut app = Faraday::new();
    app.storage(StorageEvent::Restored {
        inbox: vec![("spending-wallet.txt".into(), kit.descriptor.into_bytes())],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    app.press(Action::Backup(0));
    assert_eq!(app.screen, Screen::Backup);
    // Content up: the page scrolls down by the notch.
    scroll(&mut app, 48);
    assert_eq!(app.backup.as_ref().unwrap().scroll.y, 48.0);
    // Content down: back up, and no further than the top.
    scroll(&mut app, -96);
    assert_eq!(app.backup.as_ref().unwrap().scroll.y, 0.0);
}

#[test]
fn a_list_scrolls_by_the_same_pixels_continuously() {
    let mut app = Faraday::new();
    app.press(Action::Nav(Screen::Files));
    for _ in 0..5 {
        scroll(&mut app, 10);
    }
    // No row-snapping: five small steps are already fifty pixels moved,
    // not zero waiting for a row's worth to build up.
    assert_eq!(app.list_offset, 50.0);
    scroll(&mut app, -1000);
    assert_eq!(app.list_offset, 0.0, "scrolling up stops at the top");
}

// ---------------------------------------------------------------------
// How it moves (docs/MOTION.md §3.3): a wheel glides, a pan follows the
// fingers and coasts when they lift, an end stretches and springs back.
// ---------------------------------------------------------------------

use osk_shell_api::{BootState, DisplayInfo, SecureHardware, TouchPhase};

/// Files on a 1280 × 800 panel, one pixel per unit, with an Inbox long
/// enough to scroll, drawn once.
fn long_files(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let descriptor = testkit::kits()[0].descriptor.clone();
    app.storage(StorageEvent::Restored {
        inbox: (0..16)
            .map(|i| {
                (
                    format!("wallet-{i:02}.txt"),
                    descriptor.clone().into_bytes(),
                )
            })
            .collect(),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    tick(&mut app, 1_000);
    let _ = app.frame();
    app
}

fn tick(app: &mut Faraday, now_ms: u64) {
    app.event(Event::Tick { now_ms });
    while app.poll_command().is_some() {}
}

/// Ticks every 16 ms from `from` for `ms`, drawing each frame as a shell
/// does.
fn run(app: &mut Faraday, from: u64, ms: u64) -> u64 {
    let mut now = from;
    while now < from + ms {
        now += 16;
        tick(app, now);
        let _ = app.frame();
    }
    now
}

fn wheel(app: &mut Faraday, dy: i16) {
    app.event(Event::Wheel { x: 700, y: 400, dy });
}

fn pixels(app: &mut Faraday) -> Vec<u8> {
    app.frame().rgba.to_vec()
}

#[test]
fn a_wheel_notch_glides_and_lands_on_its_pixels() {
    let mut app = long_files(1280, 800);
    wheel(&mut app, 48);
    assert_eq!(app.list_offset, 0.0, "nothing moves before the next frame");
    tick(&mut app, 1_016);
    let first = app.list_offset;
    assert!(
        first > 0.0 && first < 48.0,
        "the first frame is part of the way: {first}"
    );
    run(&mut app, 1_016, 400);
    assert_eq!(app.list_offset, 48.0);
}

#[test]
fn notches_in_quick_succession_add_up() {
    let mut app = long_files(1280, 800);
    wheel(&mut app, 48);
    tick(&mut app, 1_016);
    wheel(&mut app, 48);
    wheel(&mut app, 48);
    run(&mut app, 1_016, 500);
    assert_eq!(app.list_offset, 144.0);
}

#[test]
fn the_wheel_stops_hard_at_the_top() {
    let mut app = long_files(1280, 800);
    let rest = pixels(&mut app);
    wheel(&mut app, -48);
    tick(&mut app, 1_016);
    assert_eq!(app.list_offset, 0.0);
    assert!(
        pixels(&mut app) == rest,
        "a wheel at an end shows no stretch"
    );
}

#[test]
fn every_offset_is_a_whole_number_of_pixels() {
    // 1366 × 768 is 0.96 pixels a unit.
    let mut app = long_files(1366, 768);
    wheel(&mut app, 48);
    let mut now = 1_000;
    for _ in 0..20 {
        now += 16;
        tick(&mut app, now);
        let px = app.list_offset * 0.96;
        assert!((px - px.round()).abs() < 1e-3, "{px} pixels");
    }
    assert!((app.list_offset * 0.96 - 48.0).abs() < 1e-3);
}

#[test]
fn a_flick_coasts_on_after_the_fingers_lift_and_comes_to_rest() {
    let mut app = long_files(1280, 800);
    let mut now = 1_000;
    for _ in 0..6 {
        scroll(&mut app, 24);
        now += 16;
        tick(&mut app, now);
    }
    let lifted = app.list_offset;
    assert_eq!(lifted, 144.0, "a pan moves the content as the fingers do");
    app.event(Event::ScrollEnd { x: 600, y: 400 });
    now = run(&mut app, now, 200);
    let coasted = app.list_offset;
    assert!(coasted > lifted + 50.0, "it kept going: {coasted}");
    now = run(&mut app, now, 3_000);
    let settled = app.list_offset;
    run(&mut app, now, 500);
    assert_eq!(app.list_offset, settled, "and stopped");
}

#[test]
fn fingers_that_stop_before_they_lift_do_not_coast() {
    let mut app = long_files(1280, 800);
    let mut now = 1_000;
    for _ in 0..6 {
        scroll(&mut app, 24);
        now += 16;
        tick(&mut app, now);
    }
    now = run(&mut app, now, 200);
    app.event(Event::ScrollEnd { x: 600, y: 400 });
    run(&mut app, now, 500);
    assert_eq!(app.list_offset, 144.0);
}

#[test]
fn pulling_past_the_top_stretches_and_springs_back() {
    let mut app = long_files(1280, 800);
    let rest = pixels(&mut app);
    scroll(&mut app, -80);
    tick(&mut app, 1_016);
    assert_eq!(app.list_offset, 0.0);
    assert!(pixels(&mut app) != rest, "the list shows pulled down");
    app.event(Event::ScrollEnd { x: 600, y: 400 });
    // The spring and the scrollbar's fade are both over in two seconds.
    run(&mut app, 1_016, 2_000);
    assert!(pixels(&mut app) == rest, "and is back at its top");
}

#[test]
fn past_the_end_a_pan_back_moves_at_once() {
    let mut app = long_files(1280, 800);
    scroll(&mut app, 30_000);
    tick(&mut app, 1_016);
    app.event(Event::ScrollEnd { x: 600, y: 400 });
    run(&mut app, 1_016, 800);
    let end = app.list_offset;
    assert!(end > 0.0 && end < 30_000.0, "it stops at the end: {end}");
    scroll(&mut app, -10);
    assert_eq!(
        app.list_offset,
        end - 10.0,
        "with nothing to scroll back first"
    );
}

#[test]
fn a_finger_dragged_on_a_touchscreen_scrolls_and_presses_nothing() {
    let mut app = long_files(1280, 800);
    let touch = |app: &mut Faraday, y: u16, phase| {
        app.event(Event::Touch { x: 600, y, phase });
        while app.poll_command().is_some() {}
    };
    touch(&mut app, 500, TouchPhase::Down);
    touch(&mut app, 450, TouchPhase::Move);
    touch(&mut app, 400, TouchPhase::Move);
    assert_eq!(app.list_offset, 100.0);
    touch(&mut app, 400, TouchPhase::Up);
    assert_eq!(app.screen, Screen::Files, "the lift pressed nothing");
}

#[test]
fn a_scrollbar_shows_while_the_content_moves_and_then_fades() {
    let mut app = long_files(1280, 800);
    let rest = pixels(&mut app);
    wheel(&mut app, 48);
    let now = run(&mut app, 1_000, 300);
    let moved = pixels(&mut app);
    // The same list put at the same place and left to rest: no bar.
    let mut still = long_files(1280, 800);
    still.list_offset = 48.0;
    scroll(&mut still, 0);
    run(&mut still, 1_000, 1_500);
    let still = pixels(&mut still);
    assert!(moved != still, "a bar shows beside the moving list");
    run(&mut app, now, 1_500);
    let end = pixels(&mut app);
    assert!(end == still, "and is gone once it rests");
    assert!(rest != still);
}

#[test]
fn a_small_panel_shows_no_scrollbar_and_its_right_edge_scrolls_the_page() {
    let mut app = long_files(480, 640);
    assert!(app.is_compact());
    wheel(&mut app, 48);
    run(&mut app, 1_000, 300);
    let moved = pixels(&mut app);
    let mut still = long_files(480, 640);
    still.list_offset = 48.0;
    scroll(&mut still, 0);
    run(&mut still, 1_000, 300);
    assert!(moved == pixels(&mut still), "no bar beside the moving list");
    // A finger at the right edge drags the page as anywhere else does.
    let touch = |app: &mut Faraday, y: u16, phase| {
        app.event(Event::Touch { x: 476, y, phase });
        while app.poll_command().is_some() {}
    };
    let at = still.list_offset;
    touch(&mut still, 500, TouchPhase::Down);
    assert_eq!(still.list_offset, at, "a press there moves nothing");
    touch(&mut still, 450, TouchPhase::Move);
    touch(&mut still, 400, TouchPhase::Move);
    touch(&mut still, 400, TouchPhase::Up);
    assert_eq!(still.list_offset, at + 100.0);
}

#[test]
fn a_stretch_springs_back_even_when_no_end_is_said() {
    // A shell that never says the fingers lifted.
    let mut app = long_files(1280, 800);
    let rest = pixels(&mut app);
    scroll(&mut app, -80);
    tick(&mut app, 1_016);
    assert!(pixels(&mut app) != rest);
    run(&mut app, 1_016, 2_500);
    assert!(pixels(&mut app) == rest);
}

#[test]
fn with_reduce_motion_a_notch_moves_at_once_and_nothing_stretches() {
    let mut app = long_files(1280, 800);
    app.press(Action::ReduceMotion(true));
    let _ = app.frame();
    wheel(&mut app, 48);
    assert_eq!(app.list_offset, 48.0, "no glide");
    wheel(&mut app, -48);
    let rest = pixels(&mut app);
    scroll(&mut app, -80);
    tick(&mut app, 1_016);
    assert_eq!(app.list_offset, 0.0);
    assert!(
        pixels(&mut app) == rest,
        "a pull past the top shows nothing"
    );
}

/// Settings on a 1280 × 800 panel, taller than it: it scrolls.
fn settings() -> Faraday {
    let mut app = long_files(1280, 800);
    app.press(Action::Nav(Screen::Settings));
    tick(&mut app, 1_000);
    let _ = app.frame();
    app
}

#[test]
fn the_scrollbar_can_be_held_and_dragged_with_a_pointer_or_a_finger() {
    let mut app = settings();
    let touch = |app: &mut Faraday, y: u16, phase| {
        app.event(Event::Touch { x: 1274, y, phase });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    };
    // Pressed low on the track: the page goes there at once.
    touch(&mut app, 790, TouchPhase::Down);
    let low = app.list_offset;
    assert!(low > 0.0, "the page moved down");
    // Dragged to the top: it follows, and stops at the top.
    touch(&mut app, 400, TouchPhase::Move);
    assert!(app.list_offset < low);
    touch(&mut app, 0, TouchPhase::Move);
    assert_eq!(app.list_offset, 0.0);
    touch(&mut app, 0, TouchPhase::Up);
    assert_eq!(app.screen, Screen::Settings, "nothing under it was pressed");
}

#[test]
fn the_scrollbar_shows_while_the_pointer_is_over_it() {
    let mut app = settings();
    let rest = pixels(&mut app);
    app.event(Event::Hover { x: 1274, y: 400 });
    assert!(pixels(&mut app) != rest, "the bar shows");
    // Still there after it would have faded from a scroll.
    let now = run(&mut app, 1_000, 2_000);
    assert!(pixels(&mut app) != rest);
    app.event(Event::Hover { x: 700, y: 400 });
    run(&mut app, now, 2_000);
    assert!(
        pixels(&mut app) == rest,
        "and fades once the pointer leaves"
    );
}

/// Holds the bar at (x, from_y), drags it to the top, lets go; returns
/// where each step left `offset`.
fn drag_bar(
    app: &mut Faraday,
    x: u16,
    from_y: u16,
    offset: impl Fn(&Faraday) -> f32,
) -> (f32, f32) {
    let touch = |app: &mut Faraday, y: u16, phase| {
        app.event(Event::Touch { x, y, phase });
        while app.poll_command().is_some() {}
        let _ = app.frame();
    };
    touch(app, from_y, TouchPhase::Down);
    let low = offset(app);
    touch(app, 0, TouchPhase::Move);
    let top = offset(app);
    touch(app, 0, TouchPhase::Up);
    (low, top)
}

#[test]
fn the_word_list_scrollbar_can_be_held_and_dragged() {
    use faraday_core::wordlist::WordListAction as WL;
    let mut app = testkit::started();
    app.press(Action::WordList(WL::Open(0, None)));
    let _ = app.frame();
    // The sheet is 860 wide, centred: its right edge is at 1070.
    let scroll = |a: &Faraday| a.wordlist.as_ref().unwrap().scroll;
    let (low, top) = drag_bar(&mut app, 1066, 740, scroll);
    assert!(
        low > 1000.0,
        "pressed low on the track, the list went there"
    );
    assert_eq!(top, 0.0, "dragged to the top, it followed");
    assert_eq!(app.sheet, Some(faraday_core::Sheet::WordList));
    assert!(
        app.wordlist.as_ref().unwrap().mark.is_none(),
        "no word pressed"
    );
}

#[test]
fn the_learn_scrollbar_can_be_held_and_dragged() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Learn);
    let _ = app.frame();
    assert!(app.learn.max > 0.0, "the page is longer than the sheet");
    // The sheet is 860 wide, centred: its right edge is at 1070.
    let (low, top) = drag_bar(&mut app, 1066, 740, |a| a.learn.scroll);
    assert!(low > 0.0, "pressed low on the track, the page went there");
    assert_eq!(top, 0.0, "dragged to the top, it followed");
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Learn));
}
