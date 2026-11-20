//! Make a new vault's steps fade through (`docs/FLOW-CONTAINER.md` §3): on
//! a wide window a step that moves on goes out to the page's surface and
//! the next comes in from it, 300 ms in all; with Reduce motion it is
//! there at once; a step with a passphrase typed swaps at once and no
//! image of it is kept.

use crate::common::type_text;
use faraday_core::testkit::Plain;
use faraday_core::vault::{VaultAction as V, vstep};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};

/// The app on Create a vault on a 1366 x 768 window, drawn, a tick taken
/// at `clock` (ms).
fn on_create_vault() -> (Faraday, u64) {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    let _ = app.frame();
    let mut clock = 10_000;
    tick(&mut app, &mut clock, 1);
    (app, clock)
}

/// `ms` more of time, as a shell ticks and draws it.
fn tick(app: &mut Faraday, clock: &mut u64, ms: u64) -> Vec<u8> {
    *clock += ms;
    app.event(Event::Tick { now_ms: *clock });
    while app.poll_command().is_some() {}
    app.frame().rgba.to_vec()
}

/// The frame once everything under way has finished.
fn at_rest(app: &mut Faraday, clock: &mut u64) -> Vec<u8> {
    for _ in 0..30 {
        tick(app, clock, 16);
    }
    tick(app, clock, 16)
}

/// How far pixel `i` of `frame` is from `surface`, summed over its colours.
fn away(frame: &[u8], i: usize, surface: &[u8]) -> i32 {
    (0..3)
        .map(|c| (i32::from(frame[i * 4 + c]) - i32::from(surface[c])).abs())
        .sum()
}

/// Where `text` is drawn, the leftmost of it: (x, y, w, h).
fn drawn_at(app: &mut Faraday, text: &str) -> (i32, i32, i32, i32) {
    let _ = app.frame();
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .filter(|(t, _)| t == text)
        .map(|b| b.1)
        .min_by_key(|b| b.0)
        .unwrap_or_else(|| panic!("{text} is drawn"))
}

/// The pixel inside `at` that is furthest from `surface`: where text is
/// inked.
fn inked(frame: &[u8], width: usize, at: (i32, i32, i32, i32), surface: &[u8]) -> usize {
    let (x, y, w, h) = at;
    let mut best = (0, 0);
    for py in y..y + h {
        for px in x..x + w {
            let i = py as usize * width + px as usize;
            let d = away(frame, i, surface);
            if d > best.0 {
                best = (d, i);
            }
        }
    }
    assert!(best.0 > 100, "the text is inked");
    best.1
}

/// Customise pressed, on Where will you open it?, at rest.
fn on_where() -> (Faraday, u64) {
    let (mut app, mut clock) = on_create_vault();
    app.press(Action::Vault(V::CCustomise));
    let _ = at_rest(&mut app, &mut clock);
    (app, clock)
}

#[test]
fn continue_fades_through_the_surface_in_300_ms() {
    let (mut app, mut clock) = on_where();
    let rest = app.frame().rgba.to_vec();
    let width = usize::from(app.frame().width);
    let old = drawn_at(&mut app, "Where will you open it?");
    // The page's surface, left of the card, in the empty margin.
    let surface = rest[(old.1 as usize * width + old.0 as usize) * 4..][..4].to_vec();
    let old_ink = inked(&rest, width, old, &surface);
    let at_first = away(&rest, old_ink, &surface);
    let _ = app.take_fade_counts();

    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let first = app.frame().rgba.to_vec();
    assert_eq!(app.take_fade_counts().started, 1, "Continue starts a fade");
    assert_eq!(first[old_ink * 4..][..4], rest[old_ink * 4..][..4]);
    let new = drawn_at(&mut app, "Unlock cost");
    // The clock starts at the first tick; 16 ms are taken as passed.
    let mut elapsed = 16;
    let mut seen = vec![(0, first)];
    tick(&mut app, &mut clock, 0);
    while elapsed < 320 {
        let frame = tick(&mut app, &mut clock, 5);
        elapsed += 5;
        seen.push((elapsed, frame));
    }
    let out: Vec<(u64, i32)> = seen
        .iter()
        .map(|(ms, f)| (*ms, away(f, old_ink, &surface)))
        .collect();
    // Part of the way out at about 50 ms.
    let (_, half) = *out.iter().find(|(ms, _)| *ms >= 50).unwrap();
    assert!(
        half > 0 && half < at_first,
        "the old text is closer to the surface at 50 ms: {half} of {at_first}"
    );
    // Gone, all of the card's place the surface, at about 105 ms.
    let gone = out
        .iter()
        .find(|(_, d)| *d == 0)
        .expect("the old text goes");
    assert!((90..=125).contains(&gone.0), "gone at {} ms", gone.0);
    // The new card comes in after, and the frame ends at the new step.
    let settled = tick(&mut app, &mut clock, 16);
    let new_ink = inked(&settled, width, new, &surface);
    let before_in = seen.iter().rev().find(|(ms, _)| *ms <= 100).unwrap();
    assert_eq!(away(&before_in.1, new_ink, &surface), 0, "nothing of the new yet");
    let later = seen.iter().find(|(ms, _)| *ms >= 200).unwrap();
    let d = away(&later.1, new_ink, &surface);
    assert!(
        d > 0 && d <= away(&settled, new_ink, &surface),
        "the new text is coming in at 200 ms"
    );
    let end = &seen.last().unwrap().1;
    assert!(
        end[new_ink * 4..][..4] == settled[new_ink * 4..][..4]
            && end[old_ink * 4..][..4] == settled[old_ink * 4..][..4],
        "300 ms on, the new step at rest"
    );
}

#[test]
fn with_reduce_motion_the_next_step_is_there_at_once() {
    let (mut app, mut clock) = on_where();
    app.press(Action::ReduceMotion(true));
    let _ = at_rest(&mut app, &mut clock);
    let _ = app.take_fade_counts();
    app.press(Action::Vault(V::CNext(vstep::WHERE)));
    let now = app.frame().rgba.to_vec();
    let counts = app.take_fade_counts();
    assert_eq!((counts.started, counts.reduced), (0, 1));
    let rest = tick(&mut app, &mut clock, 16);
    assert!(now == rest, "the step at rest");
}

#[test]
fn a_step_with_a_passphrase_typed_swaps_at_once_and_keeps_no_image() {
    let (mut app, mut clock) = on_create_vault();
    app.press(Action::Vault(V::CFocus(0, false)));
    type_text(&mut app, "test phrase");
    let _ = at_rest(&mut app, &mut clock);
    let _ = app.take_fade_counts();
    // The pane's Opens on row.
    app.press(Action::Vault(V::CStep(vstep::PRESET)));
    let now = app.frame().rgba.to_vec();
    assert_eq!(app.take_fade_counts().started, 0, "no fade");
    assert!(app.kept_images().is_empty(), "no image of the step is kept");
    assert!(now == tick(&mut app, &mut clock, 16), "the step at rest");
    assert!(app.kept_images().is_empty());
}
