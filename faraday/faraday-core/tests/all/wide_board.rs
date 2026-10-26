//! The backup board on a large window (`docs/LAYOUT.md` §17): its places
//! stand side by side in columns, two at 1920 × 1080 and three at
//! 2560 × 1440, and a chip dragged into a place in the second column is
//! kept there; at 1280 × 800 and 480 × 640 the places stand one under
//! another, as before. Back up's and a wallet flow's cards stand where
//! they stand whether the board's card is open or not, and the open
//! board's rule runs over its columns.

use crate::backup_board::{sticks, three_on_an_empty_board, three_with_a_vault};
use crate::common::Shell;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Zone};
use faraday_core::plan::{self, At, What};
use faraday_core::{Action, Faraday, qstep};
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

/// Where Back up's card `q` heads the page.
fn plan_card(app: &Faraday, q: u8) -> (u16, u16, u16, u16) {
    app.hit_box(Action::BQ(q))
        .unwrap_or_else(|| panic!("card {q} drawn"))
}

/// Back up's board's card closed, the plan's other cards on the page.
fn board_closed(app: &mut Faraday) {
    app.press(Action::BQ(qstep::BOARD));
    app.settle();
    let _ = app.frame();
    assert!(!app.board_shown(), "the board's card closed");
}

#[test]
fn back_ups_cards_stand_still_when_the_boards_card_opens() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1600)] {
        let mut app = three_places(w, h, 160);
        board_closed(&mut app);
        let before = plan_card(&app, qstep::BOARD);
        let plan_before = plan_card(&app, qstep::PLAN);
        app.press(Action::BQ(qstep::BOARD));
        app.settle();
        let _ = app.frame();
        assert!(app.board_shown(), "{w}: the board's card open");
        let after = plan_card(&app, qstep::BOARD);
        let plan_after = plan_card(&app, qstep::PLAN);
        assert_eq!(
            (before.0, before.2),
            (after.0, after.2),
            "{w}: the board's card keeps its place and width"
        );
        assert_eq!(
            (plan_before.0, plan_before.2),
            (plan_after.0, plan_after.2),
            "{w}: The plan's card keeps its place and width"
        );
    }
}

#[test]
fn a_wallet_flows_cards_stand_still_when_the_boards_card_opens() {
    use faraday_core::backup_flow::Step;
    use faraday_core::backup_flow::steps::FlowAction as F;
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1600)] {
        let mut shell = Shell::new();
        sticks(&mut shell, 3);
        let (mut app, _) = three_with_a_vault(&mut shell);
        resize(&mut app, w, h, 160);
        let i = app.backup_flows().len() - 1;
        shell.flow(&mut app, F::Show(i));
        shell.flow(&mut app, F::ChangePlan);
        let board = F::Plan(faraday_core::PlanPress::Q(qstep::BOARD));
        if app.board_shown() {
            shell.flow(&mut app, board);
        }
        app.settle();
        let _ = app.frame();
        let head = Action::Flow(F::Open(Step::Where.n()));
        let before = app.hit_box(head).expect("the plan card drawn");
        assert!(!app.board_shown(), "{w}: the board's card closed");
        shell.flow(&mut app, board);
        app.settle();
        let _ = app.frame();
        assert!(app.board_shown(), "{w}: the board's card open");
        let after = app.hit_box(head).expect("the plan card drawn");
        assert_eq!(before.0, after.0, "{w}: the plan card keeps its place");
        // At 1280 the board's list takes a side panel, the column giving
        // it room, as before; on large windows the list is the supporting
        // pane and the column keeps its width.
        if w > 1280 {
            assert_eq!(before.2, after.2, "{w}: the plan card keeps its width");
        }
    }
}

/// Whether a line is drawn across x at row y (within a pixel): the
/// pixels there differ from the one `below` pixels under it.
fn line_across(app: &mut Faraday, x: u16, y: u16, below: u16) -> bool {
    let f = app.frame();
    let width = usize::from(f.width);
    let at = |px: u16, py: u16| -> [u8; 3] {
        let i = (usize::from(py) * width + usize::from(px)) * 4;
        [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2]]
    };
    let under = at(x, y + below);
    (y.saturating_sub(1)..=y + 1).any(|py| at(x, py) != under)
}

#[test]
fn on_large_windows_the_open_boards_rule_runs_over_its_columns() {
    for (w, h, side) in [(1920u16, 1080u16, 2usize), (2560, 1600, 3)] {
        let mut app = three_places(w, h, 160);
        let head = plan_card(&app, qstep::BOARD);
        let last = place(&app, side - 1).expect("the last place of the first line");
        let end = last.0 + last.2 - 4;
        assert!(end > head.0 + head.2, "{w}: the places run past the column");
        assert!(
            line_across(&mut app, end, head.1, 20),
            "{w}: the rule above the board's card runs over its last column"
        );
        // Closed, the card's rule keeps the column's width.
        board_closed(&mut app);
        let head = plan_card(&app, qstep::BOARD);
        assert!(
            !line_across(&mut app, end, head.1, 20),
            "{w}: a closed card's rule ends with the column"
        );
    }
}
