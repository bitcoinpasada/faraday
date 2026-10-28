//! A turned panel as a person meets it (`docs/ORIENTATION.md` §9, tests
//! 1 to 5): a touch lands where the frame drew, the mouse moves the
//! arrow the way the hand goes, a turn rewrites the panel once, and a
//! caret's blink on a panel turned a quarter writes only its columns.

use std::sync::Mutex;

use faraday_shell_api::TouchPhase;

use crate::cursor::{Cursor, Pointed};
use crate::evdev::{ABS_X, ABS_Y, BTN_TOUCH, EV_ABS, EV_KEY, EV_SYN, RawEvent, SYN_REPORT};
use crate::fb::{self, Depth, Geometry};
use crate::orient::{Shared, Turn};
use crate::panel::{Panel, lock};
use crate::pointer::Motion;
use crate::present::Presenter;
use crate::touch::{Calibration, Parser, Touch};

const TURNS: [Turn; 4] = [Turn::R0, Turn::R90, Turn::R180, Turn::R270];

/// A 7-inch panel, landscape natively, and a 10-inch one, portrait.
const PANELS: [(u16, u16); 2] = [(1024, 600), (800, 1280)];

const RED: [u8; 4] = [0xff, 0x00, 0x00, 0xff];
const GROUND: [u8; 4] = [0x10, 0x10, 0x10, 0xff];

fn geometry((w, h): (u16, u16)) -> Geometry {
    Geometry {
        width: w,
        height: h,
        depth: Depth::Bgra8888,
        stride: usize::from(w) * 4,
    }
}

/// A frame of the turned size with a red block of `side` pixels at
/// `(u, v)`.
fn frame(size: (u16, u16), (u, v): (usize, usize), side: usize) -> Vec<u8> {
    let (fw, fh) = (usize::from(size.0), usize::from(size.1));
    let mut rgba = GROUND.repeat(fw * fh);
    for y in v..v + side {
        for x in u..u + side {
            rgba[(y * fw + x) * 4..][..4].copy_from_slice(&RED);
        }
    }
    rgba
}

/// The panel pixels showing red: the first and last column and row.
fn red_on(panel: &Mutex<Panel<Vec<u8>>>, g: Geometry) -> (usize, usize, usize, usize) {
    let mut red = [0u8; 4];
    fb::convert_into(&RED, g.depth, &mut red);
    let p = lock(panel);
    let sink = p.sink();
    let w = usize::from(g.width);
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (i, px) in sink.chunks_exact(4).enumerate() {
        if px == red {
            let (x, y) = (i % w, i / w);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    (x0, y0, x1, y1)
}

/// One touch down at the controller's raw `(x, y)`, as the parser makes
/// it.
fn tap(parser: &mut Parser, (x, y): (i32, i32)) -> Touch {
    let ev = |kind, code, value| RawEvent {
        time_ms: 0,
        kind,
        code,
        value,
    };
    [
        ev(EV_KEY, BTN_TOUCH, 1),
        ev(EV_ABS, ABS_X, x),
        ev(EV_ABS, ABS_Y, y),
        ev(EV_SYN, SYN_REPORT, 0),
    ]
    .into_iter()
    .find_map(|e| parser.feed(e))
    .expect("a touch")
}

/// Draws a block on each panel at each turn and touches the controller
/// where the panel shows it, the controller's raw position worked out
/// from the panel pixel by `raw`; the touch must reach the frame inside
/// the block.
fn touch_lands_on_the_block(
    parser_for: impl Fn((u16, u16), Shared) -> Parser,
    raw: impl Fn((u16, u16), (usize, usize)) -> (i32, i32),
) {
    for size in PANELS {
        let g = geometry(size);
        for turn in TURNS {
            let fs = turn.frame_size(size);
            let at = (usize::from(fs.0) / 3, usize::from(fs.1) / 4);
            let panel = Mutex::new(Panel::new(Vec::new(), g));
            lock(&panel).set_turn(turn);
            Presenter::default()
                .present(&frame(fs, at, 8), g, turn, &panel, None)
                .expect("written");
            let (x0, y0, x1, y1) = red_on(&panel, g);
            assert_eq!((x1 - x0, y1 - y0), (7, 7), "{size:?} {turn:?}: one block");
            let middle = ((x0 + x1) / 2, (y0 + y1) / 2);
            let mut parser = parser_for(size, Shared::new(turn));
            let t = tap(&mut parser, raw(size, middle));
            assert_eq!(t.phase, TouchPhase::Down);
            let (u, v) = (usize::from(t.x), usize::from(t.y));
            assert!(
                (at.0..at.0 + 8).contains(&u) && (at.1..at.1 + 8).contains(&v),
                "{size:?} {turn:?}: touched {u},{v}, drawn at {at:?}"
            );
        }
    }
}

#[test]
fn a_touch_lands_where_it_is_drawn_at_each_turn() {
    // A controller reporting in a grid twice the panel's.
    touch_lands_on_the_block(
        |(w, h), turn| Parser::new(w, h, (w * 2, h * 2)).turned(turn),
        |_, (x, y)| (x as i32 * 2 + 1, y as i32 * 2 + 1),
    );
}

#[test]
fn a_touch_lands_through_a_hid_range_and_a_turned_controller() {
    // A HID controller whose range starts at 100 across, reporting with
    // its x along the panel's height, running bottom to top.
    let calibration = Calibration {
        swap: true,
        invert_x: true,
        invert_y: false,
    };
    touch_lands_on_the_block(
        |(w, h), turn| {
            let range = crate::hid::Range {
                x: (100, 100 + i32::from(h) - 1),
                y: (0, i32::from(w) - 1),
            };
            Parser::with_range(w, h, range)
                .calibrated(calibration)
                .turned(turn)
        },
        |(_, h), (x, y)| (100 + i32::from(h) - 1 - y as i32, x as i32),
    );
}

#[test]
fn a_mouse_moved_right_moves_the_arrow_right_at_each_turn() {
    for size in PANELS {
        let g = geometry(size);
        for turn in TURNS {
            let fs = turn.frame_size(size);
            let panel = Mutex::new(Panel::new(Vec::new(), g));
            lock(&panel).set_turn(turn);
            let mut p = Presenter::default();
            p.present(
                &GROUND.repeat(usize::from(fs.0) * usize::from(fs.1)),
                g,
                turn,
                &panel,
                None,
            )
            .expect("written");
            let mut cursor = Cursor::new(fs.0, fs.1, 1.0);
            let tip = |cursor: &Cursor, panel: &Mutex<Panel<Vec<u8>>>| {
                lock(panel).move_arrow(Some(cursor.at())).expect("moved");
                // The arrow's tip is white; nothing else on the panel is.
                let mut white = [0u8; 4];
                fb::convert_into(&[0xff; 4], g.depth, &mut white);
                let p = lock(panel);
                let w = usize::from(g.width);
                let found: Vec<(usize, usize)> = p
                    .sink()
                    .chunks_exact(4)
                    .enumerate()
                    .filter(|(_, px)| *px == white)
                    .map(|(i, _)| (i % w, i / w))
                    .collect();
                assert!(!found.is_empty(), "{size:?} {turn:?}: the arrow shows");
                found
            };
            let before = cursor.at();
            let Some(Pointed::Hover { x, y }) = cursor.apply(Motion::Move { dx: 40, dy: 0 }) else {
                panic!("a hover");
            };
            assert_eq!((x, y), (before.0 + 40, before.1), "right in the frame");
            // The arrow's tip is drawn where the core's hover is.
            let tip_at = turn.to_panel(
                (usize::from(size.0), usize::from(size.1)),
                (usize::from(x), usize::from(y)),
            );
            assert!(tip(&cursor, &panel).contains(&tip_at), "{size:?} {turn:?}");
        }
    }
}

#[test]
fn a_turn_writes_the_panel_once_and_a_frame_again_writes_nothing() {
    let size = (1280, 800);
    let g = geometry(size);
    let panel = Mutex::new(Panel::new(Vec::new(), g));
    let mut p = Presenter::default();
    let page = |turn: Turn| {
        let fs = turn.frame_size(size);
        frame(fs, (100, 200), 30)
    };
    p.present(&page(Turn::R0), g, Turn::R0, &panel, None)
        .expect("written");
    let landscape = p.counts(&panel);
    // Portrait chosen: the frame is the panel turned a quarter, and every
    // row of the panel is written once.
    let turn = Turn::R90;
    assert_eq!(turn.frame_size(size), (800, 1280), "the turned size");
    lock(&panel).set_turn(turn);
    p.present(&page(turn), g, turn, &panel, None)
        .expect("written");
    let turned = p.counts(&panel);
    assert_eq!(turned.written - landscape.written, u64::from(g.height));
    let mut want = vec![0u8; g.row_bytes() * usize::from(g.height)];
    fb::convert_turned(&page(turn), (0, 1279), g.depth, turn, size, &mut want);
    assert_eq!(
        *lock(&panel).sink(),
        want,
        "the panel shows the turned frame"
    );
    // The same frame drawn again: nothing written.
    p.present(&page(turn), g, turn, &panel, None)
        .expect("written");
    assert_eq!(p.counts(&panel).written, turned.written);
}

#[test]
fn a_carets_blink_at_a_quarter_turn_writes_only_its_columns() {
    let size = (1280, 800);
    let g = geometry(size);
    let turn = Turn::R90;
    let fs = turn.frame_size(size);
    let panel = Mutex::new(Panel::new(Vec::new(), g));
    lock(&panel).set_turn(turn);
    let mut p = Presenter::default();
    let shown = frame(fs, (0, 0), 0);
    p.present(&shown, g, turn, &panel, None).expect("written");
    let before = p.counts(&panel);
    // A caret two pixels wide and twenty tall, in the frame.
    let mut caret = shown.clone();
    let fw = usize::from(fs.0);
    for v in 300..320 {
        for u in 400..402 {
            caret[(v * fw + u) * 4..][..4].copy_from_slice(&RED);
        }
    }
    p.present(&caret, g, turn, &panel, None).expect("written");
    let after = p.counts(&panel);
    // Twenty of the core's rows are twenty of the panel's columns, and
    // the caret's two columns are two of the panel's rows: two short
    // writes, not twenty whole rows.
    assert_eq!(after.converted - before.converted, 20);
    assert_eq!(after.written - before.written, 2);
    let mut want = vec![0u8; g.row_bytes() * usize::from(g.height)];
    fb::convert_turned(
        &caret,
        (0, usize::from(fs.1) - 1),
        g.depth,
        turn,
        size,
        &mut want,
    );
    assert_eq!(*lock(&panel).sink(), want, "the panel shows the caret");
}
