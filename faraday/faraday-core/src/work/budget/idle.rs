//! A screen at rest asks for no frame (`docs/SPEED.md` §8): at every
//! layout scale, scrolled to its top and to its end, 120 ticks of the
//! clock with nothing moving draw nothing. At a scale that is not whole
//! an offset rounded to whole pixels may land past the page's end; the
//! page must not clamp it back and draw again for ever.

use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware};

use super::walk;
use crate::Faraday;

/// The layout scales a shell gives: a desktop at 100 % to 300 %.
const SCALES: [f32; 7] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];

/// The pages checked: every flow page of the walk, and the main screens
/// that scroll.
const PAGES: [&str; 10] = [
    "Create",
    "Backup board",
    "Backup checklist",
    "GPG key flow",
    "Spend",
    "Home",
    "Wallets",
    "Files",
    "Backups",
    "Spend tab",
];

fn show(app: &mut Faraday, f: f32) {
    app.event(Event::Display(DisplayInfo {
        width: (1280.0 * f) as u16,
        height: (800.0 * f) as u16,
        dpi: (160.0 * f) as u16,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Whether the app asked for a frame since the last look, and what it
/// asked was drawn.
fn asked(app: &mut Faraday) -> bool {
    let mut draw = std::mem::take(&mut app.dirty);
    while let Some(c) = app.poll_command() {
        draw |= matches!(c, Command::Draw);
    }
    if draw {
        let _ = app.frame();
    }
    draw
}

/// The page at rest at `offset` (design units, past its end for the
/// end): drawn until it asks for nothing, then the ticks counted that
/// ask for a frame.
fn idle_at(app: &mut Faraday, offset: f32) -> (usize, Option<f32>) {
    app.toast = None;
    let _ = app.frame();
    if let Some(o) = app.scroll_slot() {
        *o = offset;
    }
    app.dirty = true;
    let _ = app.frame();
    app.settle();
    for _ in 0..4 {
        app.dirty = true;
        let _ = app.frame();
    }
    while app.poll_command().is_some() {}
    let base = app.now_ms;
    let mut drawn = 0;
    for i in 1..=120u64 {
        app.event(Event::Tick {
            now_ms: base + i * 16,
        });
        drawn += usize::from(asked(app));
    }
    (drawn, app.scroll_slot().map(|o| *o))
}

#[test]
fn a_page_at_rest_asks_for_no_frame_at_any_scale_at_its_top_or_its_end() {
    let c = crate::testkit::compute();
    let mut failures = Vec::new();
    let _ = walk(&c, &mut |app, name| {
        if !PAGES.contains(&name) {
            return;
        }
        for f in SCALES {
            show(app, f);
            for (end, offset) in [("top", 0.0), ("end", 1.0e6)] {
                let (drawn, at) = idle_at(app, offset);
                if drawn > 0 {
                    failures.push(format!(
                        "{name} at scale {f}, at its {end}: {drawn} of 120 idle ticks asked for a frame (offset {at:?})"
                    ));
                }
            }
        }
        show(app, 1.0);
        app.settle();
        let _ = app.frame();
    });
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
