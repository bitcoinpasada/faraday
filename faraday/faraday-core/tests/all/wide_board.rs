//! The backup board on a large window (`docs/LAYOUT.md` §17): its places
//! stand side by side in columns, two at 1920 × 1080 and three at
//! 2560 × 1440, and a chip dragged into a place in the second column is
//! kept there; at 1280 × 800 and 480 × 640 the places stand one under
//! another, as before.

use crate::backup_board::three_on_an_empty_board;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// The window made `width` × `height` at `dpi`.
fn resize(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.settle();
    let _ = app.frame();
}

/// A 2-of-3's board with three empty places, on a window this size.
fn three_places(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = three_on_an_empty_board();
    app.press(Action::Board(BP::AddPlace));
    app.press(Action::Board(BP::AddPlace));
    resize(&mut app, width, height, dpi);
    app.settle();
    let _ = app.frame();
    app
}

/// Where place `p` is drawn, if it is on screen.
fn place(app: &Faraday, p: usize) -> Option<(u16, u16, u16, u16)> {
    app.hit_box(Action::Board(BP::Zone(Zone::Card(p))))
}

/// What place `p` keeps on the plan's map.
fn kept_at(app: &Faraday, p: usize) -> Vec<What> {
    let c = faraday_core::testkit::compute();
    let b = app.backup().expect("Back up");
    let shape = app.plan_shape_now(b.wallet);
    plan::map(&c, &shape, &b.answers)
        .iter()
        .filter(|s| s.at == At::Place(p))
        .flat_map(|s| s.holds.iter().map(|(w, _)| *w))
        .collect()
}

#[test]
fn on_large_windows_the_places_stand_side_by_side_in_columns() {
    // (window, places in the first line of columns)
    for (w, h, side) in [(1920u16, 1080u16, 2usize), (2560, 1440, 3)] {
        let app = three_places(w, h, 160);
        let boxes: Vec<_> = (0..3)
            .map(|p| place(&app, p).unwrap_or_else(|| panic!("{w}: Place {} drawn", p + 1)))
            .collect();
        for p in 1..side {
            let (a, b) = (boxes[p - 1], boxes[p]);
            assert_eq!(a.1, b.1, "{w}: Place {} beside Place {}", p + 1, p);
            assert!(a.0 + a.2 < b.0, "{w}: Place {} right of Place {}", p + 1, p);
            assert_eq!(a.3, b.3, "{w}: places in a line are as tall as each other");
        }
        if side < 3 {
            let (first, next) = (boxes[0], boxes[side]);
            assert_eq!(
                first.0,
                next.0,
                "{w}: Place {} starts the next line",
                side + 1
            );
            assert!(next.1 >= first.1 + first.3, "{w}: under the first line");
        }
        // The list stands at the window's right, beside the places.
        let list = app
            .hit_box(Action::Board(BP::Zone(Zone::List)))
            .expect("the board's list");
        let last = boxes[side - 1];
        assert!(
            last.0 + last.2 < list.0,
            "{w}: the places end before the list"
        );
        assert!(
            f32::from(list.0 + list.2) <= f32::from(w) - 23.0,
            "{w}: the list's pane ends 24 dp from the window's edge: {list:?}"
        );
    }
}

#[test]
fn a_seed_dragged_into_the_second_column_is_kept_at_that_place() {
    let mut app = three_places(1920, 1080, 160);
    let start = app
        .where_offered(Action::Board(BP::Chip(Chip::List(Thing::Seed(0)))))
        .expect("Seed A in the list");
    let end = app
        .where_offered(Action::Board(BP::Zone(Zone::Paper(1))))
        .expect("Place 2's Paper");
    let first = place(&app, 0).expect("Place 1");
    assert!(
        end.0 > first.0 + first.2,
        "Place 2's Paper is in the second column"
    );
    for (at, phase) in [
        (start, TouchPhase::Down),
        ((start.0 - 12, start.1 + 4), TouchPhase::Move),
        (end, TouchPhase::Move),
        (end, TouchPhase::Up),
    ] {
        app.event(Event::Touch {
            x: at.0,
            y: at.1,
            phase,
        });
        let _ = app.frame();
    }
    app.settle();
    assert_eq!(kept_at(&app, 1), vec![What::Words(0)], "kept at Place 2");
    assert!(kept_at(&app, 0).is_empty(), "nothing at Place 1");
}

#[test]
fn on_smaller_windows_the_places_stand_one_under_another() {
    for (w, h, dpi) in [(1280u16, 800u16, 160u16), (480, 640, 286)] {
        let app = three_places(w, h, dpi);
        let first = place(&app, 0).unwrap_or_else(|| panic!("{w}: Place 1 drawn"));
        // Place 2 is under Place 1, or below the fold.
        if let Some(next) = place(&app, 1) {
            assert_eq!(first.0, next.0, "{w}: Place 2 starts where Place 1 does");
            assert!(next.1 >= first.1 + first.3, "{w}: Place 2 under Place 1");
        }
    }
}
