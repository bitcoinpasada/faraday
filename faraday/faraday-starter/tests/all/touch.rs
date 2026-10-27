//! The Starter under a finger (`docs/STARTER.md` §3.1d): the page and a
//! sheet scroll by a drag and coast after it, a drag that starts on a
//! control presses nothing, a tap presses, and once the motion settles
//! the Starter asks for no frame.

use faraday_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware, TouchPhase};
use faraday_starter::api::{Computer, Os, Payload, PayloadFile, StarterEvent};
use faraday_starter::decide::Item;
use faraday_starter::{Page, Starter, StarterAction};

fn display(width: u16, height: u16) -> Event {
    Event::Display(DisplayInfo {
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
    })
}

/// The Starter on a Linux computer in a window `width` × `height`, its
/// first frame drawn and what it asked for taken.
fn starter(width: u16, height: u16) -> Starter {
    let mut s = Starter::new();
    s.event(display(width, height));
    s.starter_event(StarterEvent::Computer(Computer {
        os: Os::Linux,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }));
    let file = |name: &str, n: char| PayloadFile {
        name: name.into(),
        sha256: n.to_string().repeat(64),
        inner: name.trim_end_matches(".xz").into(),
        inner_sha256: n.to_ascii_uppercase().to_string().repeat(64),
        inner_size: 4_000_000_000,
        build_id: "0.2.0".into(),
    };
    s.starter_event(StarterEvent::Payload(Some(Payload {
        version: "0.2.0".into(),
        commit: "4d0680b1a2b34d0680b1a2b34d0680b1a2b34d06".into(),
        files: vec![
            file("faraday-stick-0.2.0-x86_64-uefi.img.xz", 'a'),
            file("faraday-pi-0.2.0-pi3-waveshare-28dpi.img.xz", 'b'),
        ],
    })));
    s.starter_event(StarterEvent::Disks(Vec::new()));
    let _ = s.drawn_texts();
    while s.poll_command().is_some() {}
    s
}

fn at(s: &mut Starter, action: StarterAction) -> (u16, u16) {
    let _ = s.drawn_texts();
    s.where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not offered"))
}

/// Ticks `n` times 16 ms apart after `now`, a frame drawn for each that
/// asks for one; returns how many asked and the time of the last tick.
fn ticks(s: &mut Starter, now: u64, n: u64) -> (u64, u64) {
    let (mut asked, mut now) = (0, now);
    for _ in 0..n {
        now += 16;
        s.event(Event::Tick { now_ms: now });
        let mut draw = false;
        while let Some(c) = s.poll_command() {
            draw |= c == Command::Draw;
        }
        if draw {
            asked += 1;
            let _ = s.frame();
        }
    }
    (asked, now)
}

/// A finger down at `at`, moved `dy` pixels in `steps` moves a tick
/// apart, and lifted there; returns the time of the last tick.
fn drag(s: &mut Starter, (x, y): (u16, u16), dy: i32, steps: i32, now: u64) -> u64 {
    let mut now = now;
    s.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    let mut to = y;
    for i in 1..=steps {
        to = (i32::from(y) + dy * i / steps) as u16;
        now += 16;
        s.event(Event::Tick { now_ms: now });
        s.event(Event::Touch {
            x,
            y: to,
            phase: TouchPhase::Move,
        });
        let _ = s.frame();
    }
    s.event(Event::Touch {
        x,
        y: to,
        phase: TouchPhase::Up,
    });
    let _ = s.frame();
    now
}

fn tap(s: &mut Starter, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        s.event(Event::Touch { x, y, phase });
    }
}

const ROW: StarterAction = StarterAction::Item(Item::MakeStick);

#[test]
fn a_finger_drag_scrolls_home_and_presses_nothing_and_a_tap_presses() {
    let mut s = starter(400, 400);
    let (x, y0) = at(&mut s, ROW);
    // Up by 100 pixels, starting on the checklist's row.
    let now = drag(&mut s, (x, y0), -100, 5, 1000);
    assert_eq!(s.page, Page::Home, "the drag pressed nothing");
    assert!(s.scrolled() > 0.0, "the page scrolled");
    let (_, y1) = at(&mut s, ROW);
    assert!(y0 - y1 >= 90, "the row followed the finger: {y0} to {y1}");
    // The page comes to rest, and a tap on the row opens it.
    let _ = ticks(&mut s, now, 200);
    let at_rest = at(&mut s, ROW);
    tap(&mut s, at_rest);
    assert_eq!(s.page, Page::Disks);
}

#[test]
fn a_fling_on_home_coasts_then_settles_and_the_starter_asks_for_no_frame() {
    let mut s = starter(400, 300);
    let start = at(&mut s, ROW);
    // A flick: 16 pixels a frame, lifted while moving.
    let now = drag(&mut s, start, -48, 3, 1000);
    let lifted = s.scrolled();
    let (coasting, now) = ticks(&mut s, now, 5);
    assert_eq!(coasting, 5, "each tick of the coast draws");
    assert!(s.scrolled() > lifted, "it coasts on after the lift");
    let (_, now) = ticks(&mut s, now, 200);
    let settled = s.scrolled();
    let (asked, _) = ticks(&mut s, now, 200);
    assert_eq!(asked, 0, "at rest after the fling");
    assert_eq!(s.scrolled(), settled);
    assert_eq!(s.page, Page::Home);
}

#[test]
fn a_touch_on_home_while_it_coasts_stops_it_and_presses_nothing() {
    let mut s = starter(400, 300);
    let start = at(&mut s, ROW);
    let now = drag(&mut s, start, -48, 3, 1000);
    let (_, now) = ticks(&mut s, now, 1);
    let under = at(&mut s, StarterAction::Item(Item::MakeStick));
    tap(&mut s, under);
    let stopped = s.scrolled();
    let (asked, _) = ticks(&mut s, now, 60);
    assert!(asked <= 1, "it stopped: {asked} frames after");
    assert_eq!(s.scrolled(), stopped);
    assert_eq!(s.page, Page::Home, "the tap pressed nothing");
}

#[test]
fn a_finger_drag_scrolls_the_restart_sheet_and_the_page_under_it_stays() {
    // A window too short for the sheet: what is above its buttons
    // scrolls.
    let mut s = starter(400, 260);
    s.press(StarterAction::Start);
    let _ = s.drawn_texts();
    let page = s.scrolled();
    s.press(StarterAction::Restart);
    let cancel = at(&mut s, StarterAction::Cancel);
    assert_eq!(s.scrolled(), 0.0, "the sheet opens at its top");
    // Up from the sheet's middle, over its text.
    let now = drag(&mut s, (200, 130), -60, 4, 1000);
    let _ = ticks(&mut s, now, 200);
    assert!(s.scrolled() > 0.0, "the sheet scrolled");
    assert!(
        s.offers_action(StarterAction::Cancel),
        "the sheet is still open"
    );
    // Its buttons stay at its foot; Cancel closes it, the page as it was.
    assert_eq!(at(&mut s, StarterAction::Cancel), cancel);
    tap(&mut s, cancel);
    let _ = s.drawn_texts();
    assert!(!s.offers_action(StarterAction::Cancel), "Cancel closed it");
    assert_eq!(s.page, Page::Start);
    assert_eq!(s.scrolled(), page, "the page kept its place");
}
