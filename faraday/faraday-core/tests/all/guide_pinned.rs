//! The guide shows the reader where they are: its page's title is pinned
//! under the tabs wherever the page is scrolled, its scrollbar is always
//! shown, and no "Guide" heading stands over it (F308).

use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

fn window(app: &mut Faraday, width: u16, height: u16) {
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
}

fn settle(app: &mut Faraday) {
    for _ in 0..40 {
        let _ = app.frame();
    }
}

/// The guide on the Vaults page, on a window `width` x `height`.
fn vaults_guide(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Guide);
    settle(&mut app);
    app
}

/// Where the lines drawn with `text` stand, as (x, y).
fn places(app: &mut Faraday, text: &str) -> Vec<(i32, i32)> {
    let (_, lines) = app.drawn_past_sides_and_text().plain();
    lines
        .into_iter()
        .filter(|(s, _)| s == text)
        .map(|(_, r)| (r.0, r.1))
        .collect()
}

#[test]
fn the_pages_title_stays_in_one_place_from_the_top_to_the_end() {
    for (w, h) in [(1280, 800), (480, 640), (1920, 1080)] {
        let mut app = vaults_guide(w, h);
        let title = faraday_guide::guide_page(app.guide().shown)
            .expect("the page")
            .title
            .clone();
        // Beside a flow's card the pane stands at the right; the flow's own
        // heading of the same name is not the guide's.
        let from = if w >= 1536 { f32::from(w) - 424.0 } else { 0.0 };
        let top: Vec<(i32, i32)> = places(&mut app, &title)
            .into_iter()
            .filter(|p| p.0 as f32 >= from)
            .collect();
        let dbg = app.drawn_past_sides_and_text().plain().1;
        assert_eq!(
            top.len(),
            1,
            "{w}x{h}: the title {title:?} is drawn once: {top:?} {dbg:?}"
        );
        for y in [60.0, 150.0, 100_000.0] {
            faraday_core::testkit::guide_scroll_to(&mut app, y);
            settle(&mut app);
            assert!(app.guide().scroll > 0.0, "{w}x{h}: scrolled to {y}");
            // (Another line of the page may read the same words.)
            assert!(
                places(&mut app, &title).contains(&top[0]),
                "{w}x{h}: the title at scroll {y}"
            );
        }
    }
}

#[test]
fn no_guide_heading_stands_over_the_sheet_or_the_pane() {
    for (w, h) in [(1280, 800), (480, 640), (1920, 1080)] {
        let mut app = vaults_guide(w, h);
        assert!(
            places(&mut app, "Guide").len() <= 1,
            "{w}x{h}: at most the tab reads Guide"
        );
        let tab = app.hit_box(Action::GuideTabGuide);
        let found = places(&mut app, "Guide");
        // The only "Guide" a person sees is a tab's label, inside its box.
        for (x, y) in found {
            let (tx, ty, tw, th) = tab.expect("a tab holds it");
            let (tx, ty, tw, th) = (i32::from(tx), i32::from(ty), i32::from(tw), i32::from(th));
            assert!(
                x >= tx && x <= tx + tw && y >= ty && y <= ty + th,
                "{w}x{h}: a heading Guide at {x},{y}"
            );
        }
    }
}

/// Whether the sheet's right margin shows a scrollbar: a column, beside
/// Done's right edge, where a tall run differs from its neighbour.
fn bar_shown(app: &mut Faraday) -> bool {
    let done = app
        .hit_box(Action::GuideClose)
        .or_else(|| app.hit_box(Action::Cancel));
    let d = done.expect("Done");
    let f = app.frame();
    let (fw, fh) = (usize::from(f.width), usize::from(f.height));
    let px = |x: usize, y: usize| &f.rgba[(y * fw + x) * 4..(y * fw + x) * 4 + 3];
    let x0 = (d.0 + d.2) as usize + 1;
    for x in x0 + 1..(x0 + 12).min(fw) {
        let mut run = 0;
        for y in 0..fh {
            if px(x, y) != px(x0, y) {
                run += 1;
                if run >= 30 {
                    return true;
                }
            } else {
                run = 0;
            }
        }
    }
    false
}

#[test]
fn the_sheets_scrollbar_is_shown_at_the_top_partway_and_at_the_end() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let mut app = vaults_guide(w, h);
        for y in [0.0, 150.0, 100_000.0] {
            faraday_core::testkit::guide_scroll_to(&mut app, y);
            settle(&mut app);
            assert!(bar_shown(&mut app), "{w}x{h}: the scrollbar at {y}");
        }
    }
}
