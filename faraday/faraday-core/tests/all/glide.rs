//! A change of page under the Motion and Speed settings
//! (`docs/MOTION.md` §3.5): Smooth motion fades the old page out and the
//! new one in and glides the session pane between its two places,
//! Crossfade blends the two, Off shows the new page at once; the Speed
//! sets how long it takes.

use faraday_core::stick::settings_choice::{MotionSpeed, MotionStyle};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const WIDTH: usize = 1280;

fn shown(style: MotionStyle, speed: MotionSpeed) -> Faraday {
    shown_at((WIDTH as u16, 800), style, speed)
}

fn shown_at((width, height): (u16, u16), style: MotionStyle, speed: MotionSpeed) -> Faraday {
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
    app.event(Event::Tick { now_ms: 1_000 });
    let _ = app.frame();
    app.press(Action::Motion(style));
    app.press(Action::Speed(speed));
    let _ = at(&mut app, 1_016);
    let _ = at(&mut app, 3_000);
    app
}

fn at(app: &mut Faraday, now: u64) -> Vec<u8> {
    app.event(Event::Tick { now_ms: now });
    while app.poll_command().is_some() {}
    let _ = app.frame();
    app.frame().rgba.to_vec()
}

/// The frames every 16 ms from `from` for `ms`, after a change of screen
/// pressed at `from`.
fn run(app: &mut Faraday, from: u64, ms: u64) -> Vec<Vec<u8>> {
    run_to(app, from, ms, None)
}

/// [`run`], the new page drawing its pane at `pane`.
fn run_to(
    app: &mut Faraday,
    from: u64,
    ms: u64,
    pane: Option<(i32, i32, i32, i32)>,
) -> Vec<Vec<u8>> {
    app.press(Action::Nav(Screen::Settings));
    app.force_pane(pane, false);
    (0..=ms / 16).map(|i| at(app, from + i * 16)).collect()
}

/// How many pixels differ from `bg`.
fn ink(frame: &[u8], bg: &[u8]) -> usize {
    let far = |p: &[u8]| p.iter().zip(bg).any(|(a, b)| a.abs_diff(*b) > 8);
    frame.chunks(4).filter(|p| far(p)).count()
}

/// The pixel the old page has most of.
fn background(frame: &[u8]) -> [u8; 4] {
    let mut seen: std::collections::HashMap<[u8; 4], usize> = Default::default();
    for p in frame.chunks(4) {
        *seen.entry([p[0], p[1], p[2], p[3]]).or_default() += 1;
    }
    seen.into_iter()
        .max_by_key(|(_, n)| *n)
        .map_or([0; 4], |(p, _)| p)
}

/// The tick at which the run's frames stop changing.
fn settled_at(frames: &[Vec<u8>]) -> usize {
    let rest = frames.last().unwrap();
    frames.iter().position(|f| f == rest).unwrap()
}

#[test]
fn smooth_motion_clears_the_page_between_the_old_and_the_new() {
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let old = at(&mut app, 3_016);
    let bg = background(&old);
    let frames = run(&mut app, 4_000, 800);
    let rest = frames.last().unwrap();
    let least = frames.iter().map(|f| ink(f, &bg)).min().unwrap();
    assert!(
        least * 4 < ink(&old, &bg).min(ink(rest, &bg)),
        "the page is nearly bare between the two"
    );
}

#[test]
fn crossfade_blends_the_old_page_into_the_new_without_clearing_it() {
    let mut app = shown(MotionStyle::Crossfade, MotionSpeed::Medium);
    let old = at(&mut app, 3_016);
    let bg = background(&old);
    let frames = run(&mut app, 4_000, 800);
    let rest = frames.last().unwrap();
    let least = frames.iter().map(|f| ink(f, &bg)).min().unwrap();
    assert!(
        least * 4 >= ink(&old, &bg).min(ink(rest, &bg)),
        "both pages are on screen throughout"
    );
    assert!(
        frames[2] != old && frames[2] != *rest,
        "and it is part of the way"
    );
}

#[test]
fn with_motion_off_the_new_page_is_there_at_once() {
    let mut app = shown(MotionStyle::Off, MotionSpeed::Medium);
    let old = at(&mut app, 3_016);
    let frames = run(&mut app, 4_000, 400);
    let last = frames.last().unwrap();
    assert!(frames[0] == *last, "the first frame is the final one");
    assert!(*last != old, "and the page changed");
}

#[test]
fn the_speed_sets_how_long_a_change_of_page_takes() {
    let mut took = Vec::new();
    for speed in [
        MotionSpeed::Quick,
        MotionSpeed::Medium,
        MotionSpeed::Relaxed,
    ] {
        let mut app = shown(MotionStyle::Crossfade, speed);
        let frames = run(&mut app, 4_000, 1_200);
        took.push(settled_at(&frames) as u64 * 16);
    }
    assert!(took[0] < took[1] && took[1] < took[2], "{took:?}");
    for (took, speed) in took.iter().zip(MotionSpeed::ALL) {
        assert!(
            took.abs_diff(speed.ms()) <= 48,
            "{took} ms for {}",
            speed.ms()
        );
    }
}

/// How many pixels of the rows and columns differ between two frames.
fn differ(a: &[u8], b: &[u8], cols: std::ops::Range<usize>, rows: std::ops::Range<usize>) -> usize {
    let mut n = 0;
    for y in rows {
        for x in cols.clone() {
            let i = (y * WIDTH + x) * 4;
            n += (a[i..i + 4] != b[i..i + 4]) as usize;
        }
    }
    n
}

#[test]
fn the_pane_glides_between_its_two_places_and_ends_in_the_new_one() {
    let mut plain = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let none = run(&mut plain, 4_000, 800);
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    app.force_pane(Some((0, 0, 300, 300)), true);
    let with = run_to(&mut app, 4_000, 800, Some((800, 0, 300, 300)));
    // The middle column, where the pane is only on its way.
    let between = |i: usize| differ(&with[i], &none[i], 300..800, 0..300);
    assert!(
        (1..with.len() - 1).any(|i| between(i) > 0),
        "the pane is seen between its places"
    );
    assert_eq!(
        between(with.len() - 1),
        0,
        "and is gone from there at the end"
    );
    assert!(with.last() == none.last(), "the page ends as it is at rest");
    let beyond = |i: usize| differ(&with[i], &none[i], 1100..1280, 0..800);
    assert!(
        (0..with.len()).all(|i| beyond(i) == 0),
        "never beyond its places"
    );
}

#[test]
fn a_pane_on_one_side_only_does_not_glide() {
    let mut plain = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let none = run(&mut plain, 4_000, 800);
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    app.force_pane(Some((0, 0, 300, 300)), true);
    let with = run_to(&mut app, 4_000, 800, None);
    assert!(with == none, "the whole page fades through");
}

#[test]
fn a_pane_that_keeps_its_place_does_not_move() {
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    app.force_pane(Some((100, -100, 300, 300)), true);
    let old = at(&mut app, 3_016);
    let frames = run_to(&mut app, 4_000, 800, Some((100, -100, 300, 300)));
    let rest = frames.last().unwrap();
    // The pane shows only its two pages at their own place: a pixel of it
    // is never outside what the old and the new page have there.
    for (n, f) in frames.iter().enumerate() {
        let off = outside_both(f, &old, rest, 100..400, 0..200);
        assert_eq!(off, 0, "frame {n}");
    }
    assert!(old != *rest, "and the page changed");
}

/// How many channels of the pane's pixels in `f` lie beyond both `a`'s and
/// `b`'s at the same place, by more than a rounding.
fn outside_both(
    f: &[u8],
    a: &[u8],
    b: &[u8],
    cols: std::ops::Range<usize>,
    rows: std::ops::Range<usize>,
) -> usize {
    let mut n = 0;
    for y in rows {
        for i in cols
            .clone()
            .flat_map(|x| (y * WIDTH + x) * 4..(y * WIDTH + x) * 4 + 4)
        {
            let (low, high) = (a[i].min(b[i]), a[i].max(b[i]));
            n += (f[i] < low.saturating_sub(2) || f[i] > high.saturating_add(2)) as usize;
        }
    }
    n
}

#[test]
fn a_pane_on_the_new_page_only_does_not_glide() {
    let mut plain = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let none = run(&mut plain, 4_000, 800);
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    app.force_pane(None, true);
    let with = run_to(&mut app, 4_000, 800, Some((800, 0, 300, 300)));
    assert!(with == none, "the whole page fades through");
}

#[test]
fn a_page_opened_mid_fade_ends_on_the_last_page() {
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    app.press(Action::Nav(Screen::Settings));
    let _ = at(&mut app, 4_000);
    let _ = at(&mut app, 4_100);
    app.press(Action::Nav(Screen::Home));
    let mut last = Vec::new();
    for i in 0..=60 {
        last = at(&mut app, 4_116 + i * 16);
    }
    let mut direct = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let _ = at(&mut direct, 3_016);
    assert!(last == at(&mut direct, 6_000), "the fade ends on Home");
}

#[test]
fn a_full_frame_mid_fade_on_smooth_motion_still_ends_on_the_new_page() {
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let mut plain = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    let none = run(&mut plain, 4_000, 800);
    app.press(Action::Nav(Screen::Settings));
    let _ = at(&mut app, 4_000);
    let _ = at(&mut app, 4_100);
    app.event(Event::Display(DisplayInfo {
        width: WIDTH as u16,
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
    let mut last = Vec::new();
    for i in 0..=60 {
        last = at(&mut app, 4_116 + i * 16);
    }
    assert!(Some(&last) == none.last());
}

#[test]
fn leaving_a_page_after_a_secret_was_typed_keeps_no_image() {
    let mut app = shown(MotionStyle::Smooth, MotionSpeed::Medium);
    app.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Krux code")
        .unwrap();
    app.press(Action::Catalog(tile as u8));
    app.press(Action::KefTool(
        faraday_core::kef_tool::KefToolAction::Pick(faraday_core::kef_tool::Pick::Type),
    ));
    for c in "a note for later".chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    let _ = at(&mut app, 4_000);
    assert!(faraday_core::testkit::secret_typed(&app));
    app.press(Action::Nav(Screen::Settings));
    for i in 0..8 {
        let _ = at(&mut app, 4_016 + i * 16);
        assert!(app.kept_images().is_empty(), "an image is kept");
    }
}

/// The real windows the wide layout is judged at.
const WINDOWS: [(u16, u16); 2] = [(1920, 1080), (1366, 768)];

fn tile(name: &str) -> u8 {
    faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .unwrap() as u8
}

/// An app on Home at `win`, settled.
fn on_home(win: (u16, u16), style: MotionStyle) -> (Faraday, u64) {
    let mut app = shown_at(win, style, MotionSpeed::Medium);
    app.press(Action::Nav(Screen::Home));
    let _ = at(&mut app, 3_100);
    let _ = at(&mut app, 4_000);
    (app, 4_000)
}

/// Presses `go` and returns the frames every 16 ms for a second from `t`,
/// the new page's pane forced away when `no_pane`.
fn change(app: &mut Faraday, go: Action, t: u64, no_pane: bool) -> Vec<Vec<u8>> {
    app.press(go);
    if no_pane {
        app.force_pane(None, false);
    }
    (0..=1_000 / 16).map(|i| at(app, t + i * 16)).collect()
}

/// The frame a little under half way through the change.
const MID: usize = 14;

#[test]
fn home_to_tools_and_to_check_glides_the_pane_between_their_places() {
    for win in WINDOWS {
        for go in [Screen::Catalog, Screen::Check] {
            let (mut app, t) = on_home(win, MotionStyle::Smooth);
            let home = app.pane_place();
            assert!(home.is_some(), "{win:?}: Home reports its card");
            let frames = change(&mut app, Action::Nav(go), t + 100, false);
            let there = app.pane_place();
            assert!(there.is_some(), "{win:?} {go:?}: the page reports its pane");
            let (mut bare, t) = on_home(win, MotionStyle::Smooth);
            let fades = change(&mut bare, Action::Nav(go), t + 100, true);
            assert_ne!(
                frames[MID], fades[MID],
                "{win:?} {go:?}: the pane is drawn between its places, not faded with the page"
            );
        }
    }
}

#[test]
fn tools_to_a_tool_with_a_pair_and_back_glides_the_pane() {
    for win in WINDOWS {
        let (mut app, t) = on_home(win, MotionStyle::Smooth);
        let _ = change(&mut app, Action::Nav(Screen::Catalog), t, false);
        let tools = app.pane_place();
        assert!(tools.is_some(), "{win:?}: Tools reports its pane");
        let (mut bare, tb) = on_home(win, MotionStyle::Smooth);
        let _ = change(&mut bare, Action::Nav(Screen::Catalog), tb, false);
        let open = Action::Catalog(tile("Make a new wallet"));
        let frames = change(&mut app, open, t + 2_000, false);
        let flow = app.pane_place();
        let fades = change(&mut bare, open, tb + 2_000, true);
        assert!(
            flow.is_some(),
            "{win:?}: the pair's left part reports its place"
        );
        assert_ne!(flow, tools, "{win:?}: the pane moves into the flow's place");
        assert_ne!(
            frames[MID], fades[MID],
            "{win:?}: the pane glides into the flow"
        );
        // And back on leaving.
        let back = change(&mut app, Action::Nav(Screen::Catalog), t + 4_000, false);
        assert_eq!(app.pane_place(), tools, "{win:?}: back in Tools' place");
        let bare_back = change(&mut bare, Action::Nav(Screen::Catalog), tb + 4_000, true);
        assert_ne!(back[MID], bare_back[MID], "{win:?}: the pane glides back");
    }
}

#[test]
fn a_tool_whose_flow_has_no_pair_reports_no_pane() {
    for win in WINDOWS {
        let (mut app, t) = on_home(win, MotionStyle::Smooth);
        let _ = change(&mut app, Action::Nav(Screen::Catalog), t, false);
        let _ = change(
            &mut app,
            Action::Catalog(tile("Add a key")),
            t + 2_000,
            false,
        );
        assert_eq!(app.pane_place(), None, "{win:?}");
    }
}

#[test]
fn off_shows_the_end_at_once_and_crossfade_does_not_glide() {
    for win in WINDOWS {
        let (mut off, t) = on_home(win, MotionStyle::Off);
        let frames = change(&mut off, Action::Nav(Screen::Catalog), t + 100, false);
        assert_eq!(
            frames[0],
            *frames.last().unwrap(),
            "{win:?}: Off is final at once"
        );
        let (mut cross, t) = on_home(win, MotionStyle::Crossfade);
        let a = change(&mut cross, Action::Nav(Screen::Catalog), t + 100, false);
        let (mut cross, t) = on_home(win, MotionStyle::Crossfade);
        let b = change(&mut cross, Action::Nav(Screen::Catalog), t + 100, true);
        assert_eq!(a[MID], b[MID], "{win:?}: Crossfade blends whole frames");
    }
}

/// A flow with a pair, opened from Tools at 1920 x 1080 and settled, the
/// guide shut.
fn in_a_flow(style: MotionStyle) -> (Faraday, u64) {
    let (mut app, t) = on_home((1920, 1080), style);
    let _ = change(&mut app, Action::Nav(Screen::Catalog), t, false);
    let _ = change(
        &mut app,
        Action::Catalog(tile("Make a new wallet")),
        t + 2_000,
        false,
    );
    (app, t + 4_000)
}

#[test]
fn the_pair_slides_aside_for_the_guide_and_back_to_where_it_was() {
    let (mut app, t) = in_a_flow(MotionStyle::Smooth);
    let shut = at(&mut app, t);
    let place = app.pane_place();
    assert!(place.is_some(), "the flow's pair reports its place");
    let (mut bare, tb) = in_a_flow(MotionStyle::Smooth);
    let _ = at(&mut bare, tb);
    app.press(Action::Guide);
    bare.press(Action::Guide);
    bare.force_pane(None, false);
    let open: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    let still: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut bare, tb + 100 + i * 16))
        .collect();
    assert_ne!(app.pane_place(), place, "the guide moves the pair aside");
    assert_ne!(
        open[MID], still[MID],
        "the pair is drawn between its places"
    );
    app.press(Action::GuideClose);
    let _: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 2_000 + i * 16))
        .collect();
    assert_eq!(app.pane_place(), place, "the pair is back at its place");
    assert_eq!(at(&mut app, t + 4_000), shut, "and the page is as it was");
}

#[test]
fn with_motion_off_the_pair_is_aside_at_once() {
    let (mut app, t) = in_a_flow(MotionStyle::Off);
    let _ = at(&mut app, t);
    app.press(Action::Guide);
    let frames: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    assert_eq!(frames[0], *frames.last().unwrap(), "Off is final at once");
}

#[test]
fn the_guide_slides_in_from_the_right_edge_and_out_again() {
    let (mut app, t) = on_home((1920, 1080), MotionStyle::Smooth);
    let shut = at(&mut app, t);
    app.press(Action::Guide);
    let open: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    let rest = open.last().unwrap();
    assert_ne!(open[2], shut, "the first frames are not the shut page");
    assert_ne!(open[2], *rest, "nor the open one: the pane is between");
    assert_ne!(open[MID], *rest, "half way it is still on its way");
    assert_ne!(*rest, shut, "and it ends open");
    app.press(Action::GuideClose);
    let shutting: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 2_000 + i * 16))
        .collect();
    assert_ne!(shutting[2], *rest, "closing starts from the open page");
    assert_ne!(shutting[MID], shut, "and is between half way");
    assert_eq!(*shutting.last().unwrap(), shut, "pixel-equal after close");
}

/// How far the left part of the guide's rest rectangle in `frame` is
/// from the shut page `shut` towards the surface colour `s`, 0 to 1, read
/// at the pixel of the shut page that is furthest from it; and whether
/// every pixel of the part is at that same strength, so that the part is
/// the shut page faded to the surface as a whole.
fn strength(
    frame: &[u8],
    shut: &[u8],
    s: [u8; 4],
    xs: std::ops::Range<usize>,
    ys: std::ops::Range<usize>,
) -> (f32, f32) {
    let w = 1920usize;
    let mut best = (0i32, 0usize, 0usize);
    for y in ys.clone() {
        for x in xs.clone() {
            let i = (y * w + x) * 4;
            let d = (0..3)
                .map(|c| (shut[i + c] as i32 - s[c] as i32).abs())
                .max()
                .unwrap();
            if d > best.0 {
                best = (d, x, y);
            }
        }
    }
    let (d, bx, by) = best;
    if d == 0 {
        return (0.0, 0.0);
    }
    let i = (by * w + bx) * 4;
    let c = (0..3)
        .max_by_key(|c| (shut[i + c] as i32 - s[*c] as i32).abs())
        .unwrap();
    let k = (shut[i + c] as f32 - frame[i + c] as f32) / (shut[i + c] as f32 - s[c] as f32);
    let mut worst = 0f32;
    for y in ys {
        for x in xs.clone() {
            let i = (y * w + x) * 4;
            for c in 0..3 {
                let want = shut[i + c] as f32 + (s[c] as f32 - shut[i + c] as f32) * k;
                worst = worst.max((frame[i + c] as f32 - want).abs());
            }
        }
    }
    (k, worst)
}

/// Opens the guide on `page` at 1920 x 1080, then shuts it, and checks
/// that in the left part of the pane's rest rectangle (which the sliding
/// pane has not reached) every frame is the page without the guide as the
/// page's own fade through the surface shows it: the whole part at one
/// strength, going out to the surface and staying there while the opening
/// page comes in, coming in from it while the closing page does.
fn under_the_pane_the_page_is_whole(page: Screen) {
    let (mut app, t) = on_home((1920, 1080), MotionStyle::Smooth);
    let _ = change(&mut app, Action::Nav(page), t, false);
    let t = t + 2_000;
    let shut = at(&mut app, t);
    app.press(Action::Guide);
    let open: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    let (x0, y0, pw, ph) = app.guide_rest_place().expect("the app has drawn");
    let (x0, y0) = (x0 as usize, y0 as usize);
    let (x1, y1) = (x0 + pw as usize, y0 + ph as usize);
    // The sliding pane has not reached the left part in the frames
    // looked at; the pane's shadow, beyond the rectangle, is left out.
    let (xs, ys) = (x0 + 24..x0 + (x1 - x0) / 4, y0 + 24..y1 - 24);
    // The surface: what the part holds most of once the page has gone
    // out of it, at the fifth frame.
    let crop: Vec<u8> = ys
        .clone()
        .flat_map(|y| open[5][(y * 1920 + xs.start) * 4..(y * 1920 + xs.end) * 4].to_vec())
        .collect();
    let surface = background(&crop);
    let look = |frames: &[Vec<u8>], which: std::ops::Range<usize>, what: &str| -> Vec<(f32, f32)> {
        which
            .map(|i| {
                let (k, worst) = strength(&frames[i], &shut, surface, xs.clone(), ys.clone());
                eprintln!("{what} frame {i}: strength {k:.3}, off by {worst}");
                (k, worst)
            })
            .collect()
    };
    // Where the part is one fade of the page, its strength: the sliding
    // pane covers it in the other frames.
    let whole = |v: Vec<(f32, f32)>| -> Vec<f32> {
        v.into_iter()
            .filter(|(_, w)| *w <= 8.0)
            .map(|(k, _)| k)
            .collect()
    };
    // Opening: out to the surface while the old page is there.
    let up = look(&open, 0..7, "opening");
    assert!(up.iter().all(|(_, w)| *w <= 8.0), "{up:?}");
    let up: Vec<f32> = up.into_iter().map(|(k, _)| k).collect();
    assert!(up.windows(2).all(|w| w[1] >= w[0] - 0.02), "{up:?}");
    assert!(
        up.iter().any(|k| *k > 0.3),
        "the page fades out under the guide: {up:?}"
    );
    app.press(Action::GuideClose);
    let shutting: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 2_000 + i * 16))
        .collect();
    // Closing: the surface first, then the page in from it; the first
    // frames that show the part at all are the surface.
    let down = whole(look(&shutting, 0..30, "closing"));
    eprintln!("{down:?}");
    assert!(
        down.first().is_some_and(|k| *k > 0.9),
        "the surface first: {down:?}"
    );
    assert!(down.windows(2).all(|w| w[1] <= w[0] + 0.02), "{down:?}");
}

#[test]
fn the_page_fading_through_shows_nothing_of_the_guide_at_its_rest_place() {
    under_the_pane_the_page_is_whole(Screen::Home);
}

#[test]
fn a_card_under_the_guides_rest_place_is_whole_while_it_slides() {
    under_the_pane_the_page_is_whole(Screen::Catalog);
}

/// Across the guide's rest rectangle's left edge, every frame of the
/// guide's slide in a flow shows no seam: the step between the columns
/// either side of the edge is no larger than the steps between the
/// columns beside them (a seam is a card cut or doubled there).
#[test]
fn a_flows_card_has_no_seam_at_the_guides_rest_edge_while_it_slides() {
    let (mut app, t) = in_a_flow(MotionStyle::Smooth);
    let _ = at(&mut app, t);
    app.press(Action::Guide);
    let open: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    let (x0, y0, _, ph) = app.guide_rest_place().expect("the app has drawn");
    let (x0, y0, y1) = (x0 as usize, y0 as usize, (y0 + ph) as usize);
    app.press(Action::GuideClose);
    let shutting: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 2_000 + i * 16))
        .collect();
    let step = |f: &[u8], x: usize| -> i64 {
        (y0 + 24..y1 - 24)
            .map(|y| {
                let (a, b) = ((y * 1920 + x - 1) * 4, (y * 1920 + x) * 4);
                (0..3)
                    .map(|c| (f[a + c] as i64 - f[b + c] as i64).abs())
                    .sum::<i64>()
            })
            .sum()
    };
    for (what, frames) in [("opening", &open), ("closing", &shutting)] {
        for (i, f) in frames.iter().enumerate().take(8) {
            let seam = step(f, x0);
            let beside = (x0 - 14..x0 - 1)
                .chain(x0 + 2..x0 + 14)
                .map(|x| step(f, x))
                .max()
                .unwrap();
            eprintln!("{what} frame {i}: seam {seam}, beside {beside}");
            assert!(
                seam <= beside * 3 / 2 + 2_000,
                "{what} frame {i}: a seam at x {x0}: {seam} against {beside}"
            );
        }
    }
}

#[test]
fn with_motion_off_the_guide_is_there_and_gone_at_once() {
    let (mut app, t) = on_home((1920, 1080), MotionStyle::Off);
    let shut = at(&mut app, t);
    app.press(Action::Guide);
    let open: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    assert_eq!(open[0], *open.last().unwrap(), "Off is final at once");
    app.press(Action::GuideClose);
    assert_eq!(at(&mut app, t + 2_000), shut, "and closing is at once");
}

#[test]
fn crossfade_keeps_the_guides_own_fade_with_no_slide() {
    let (mut app, t) = on_home((1920, 1080), MotionStyle::Crossfade);
    let _ = at(&mut app, t);
    app.press(Action::Guide);
    let open: Vec<_> = (0..=1_000 / 16)
        .map(|i| at(&mut app, t + 100 + i * 16))
        .collect();
    // The short fade (200 ms) is over by the frame at 300 ms, whatever
    // the Speed: a slide at Medium would still be moving.
    assert_eq!(open[14], *open.last().unwrap());
}
