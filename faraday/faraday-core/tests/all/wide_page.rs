//! A wide window's page area (`docs/LAYOUT.md` §14, §15): what stands at
//! the right (the ?, Scan) stands 24 dp from the window's edge, and a
//! reading page sits centred in the room beside the navigation.

use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

const MARGIN: f32 = 24.0;
const SIDEBAR: f32 = 240.0;

fn shown(width: u16, height: u16) -> Faraday {
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
    app
}

#[test]
fn the_question_mark_and_scan_stand_24_dp_from_the_windows_edge() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (2560, 1440)] {
        let mut app = shown(w, h);
        app.press(Action::Nav(Screen::Home));
        let _ = app.frame();
        // The ? is a 28 dp circle in a 40 dp target.
        let q = app.hit_box(Action::Learn).expect("the ? on Home");
        let right = f32::from(q.0) + f32::from(q.2) / 2.0 + 14.0;
        assert!(
            (right - (f32::from(w) - MARGIN)).abs() <= 1.0,
            "{w}: the ? ends at {right}"
        );
        let scan = app.hit_box(Action::Scan).expect("Scan on Home");
        assert!(
            (f32::from(scan.0 + scan.2) - (f32::from(w) - MARGIN)).abs() <= 1.0,
            "{w}: Scan {scan:?}"
        );
        assert!(
            (f32::from(scan.1 + scan.3) - (f32::from(h) - MARGIN)).abs() <= 1.0,
            "{w}: Scan {scan:?}"
        );
    }
}

#[test]
fn set_the_time_sits_centred_in_the_room_beside_the_navigation_on_a_big_window() {
    for (w, h) in [(1920u16, 1080u16), (2560, 1440)] {
        let mut app = shown(w, h);
        app.press(Action::Nav(Screen::Settings));
        app.press(Action::OpenSetTime);
        let _ = app.frame();
        // Its 520 dp column, under the top app bar: the card of Now, its
        // label at its start and the time at its end, as far in.
        let (_, right) = page_span(&mut app);
        let middle = (text_left(&mut app, "Now") + right) / 2.0;
        let room = SIDEBAR + (f32::from(w) - SIDEBAR) / 2.0;
        assert!(
            (middle - room).abs() <= 2.0,
            "{w}: the column's middle {middle}, the room's {room}"
        );
    }
}

/// The left and right ends of what the page draws beside the
/// navigation, below the top app bar's 64 dp.
fn page_span(app: &mut Faraday) -> (f32, f32) {
    let ink = app.drawn_ink();
    let boxes = ink
        .iter()
        .map(|i| i.rect)
        .filter(|r| r.x as f32 > SIDEBAR && r.y >= 64);
    let left = boxes.clone().map(|r| r.x).min().expect("the page draws") as f32;
    let right = boxes.map(|r| r.x + r.w).max().expect("the page draws") as f32;
    (left, right)
}

/// Where a string of the page starts, by its first line on screen.
fn text_left(app: &mut Faraday, want: &str) -> f32 {
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == want)
        .map(|b| b.1)
        .min_by_key(|r| r.1)
        .unwrap_or_else(|| panic!("no {want} on the page"))
        .0 as f32
}

/// The middle of the room beside the navigation on a window `w` wide.
fn room_middle(w: u16) -> f32 {
    SIDEBAR + (f32::from(w) - SIDEBAR) / 2.0
}

#[test]
fn tools_stands_centred_in_the_room_on_a_big_window() {
    for (w, h) in [(1920u16, 1080u16), (2560, 1600)] {
        let mut app = shown(w, h);
        app.press(Action::Tools);
        let _ = app.frame();
        // From the column's start to the field's Clear.
        let (left, _) = page_span(&mut app);
        let clear = app.hit_box(Action::TClear).expect("the field's Clear");
        let right = f32::from(clear.0 + clear.2);
        let (before, after) = (left - SIDEBAR, f32::from(w) - right);
        assert!(
            (before - after).abs() <= 16.0,
            "Tools at {w}: {before} dp from the navigation, {after} from the edge"
        );
    }
}

#[test]
fn transfer_holds_send_and_receive_either_side_of_the_rooms_middle() {
    for (w, h) in [(1920u16, 1080u16), (2560, 1600)] {
        let mut app = shown(w, h);
        app.online = true;
        app.press(Action::Nav(Screen::Transfer));
        let _ = app.frame();
        assert_eq!(app.screen, Screen::Transfer);
        let middle = room_middle(w);
        let send = text_left(&mut app, "Send");
        let receive = text_left(&mut app, "Receive");
        assert!(
            send < middle,
            "{w}: Send starts at {send}, past the middle {middle}"
        );
        assert!(
            (0.0..=24.0).contains(&(receive - middle)),
            "{w}: Receive starts at {receive}, the room's middle is {middle}"
        );
    }
}

#[test]
fn a_stick_visits_two_lists_stand_centred_in_the_room_on_a_big_window() {
    use faraday_core::{StickInfo, StorageEvent};
    for (w, h) in [(1920u16, 1080u16), (2560, 1600)] {
        let mut app = shown(w, h);
        app.storage(StorageEvent::Sticks(vec![StickInfo {
            serial: None,
            id: "S".into(),
            label: "TESTSTICK".into(),
            boot: false,
            files: vec![("spend.psbt".into(), 300)],
        }]));
        app.press(Action::Nav(Screen::Visit));
        let _ = app.frame();
        assert_eq!(app.screen, Screen::Visit);
        // From the column's start to the end of the second list.
        let (left, _) = page_span(&mut app);
        let end = [Action::VisitInAll, Action::VisitOutAll]
            .into_iter()
            .filter_map(|a| app.hit_box(a))
            .map(|b| f32::from(b.0 + b.2))
            .fold(0.0, f32::max);
        let (before, after) = (left - SIDEBAR, f32::from(w) - end);
        assert!(
            (before - after).abs() <= 24.0,
            "{w}: the visit stands {before} dp from the navigation, {after} from the edge"
        );
    }
}

#[test]
fn the_tools_catalogue_takes_more_tiles_to_a_row_on_a_big_window() {
    let per_row = |w: u16, h: u16| {
        let mut app = shown(w, h);
        app.press(Action::Nav(Screen::Catalog));
        let _ = app.frame();
        let boxes: Vec<(u16, u16, u16, u16)> = (0..64u8)
            .filter_map(|i| app.hit_box(Action::Catalog(i)))
            .collect();
        let top = boxes
            .iter()
            .map(|b| b.1)
            .min()
            .expect("the catalogue's tiles");
        let row: Vec<_> = boxes.iter().filter(|b| b.1 == top).collect();
        let end = row.iter().map(|b| f32::from(b.0 + b.2)).fold(0.0, f32::max);
        (row.len(), end)
    };
    let (narrow, _) = per_row(1280, 800);
    assert_eq!(narrow, 3, "the catalogue at 1280 dp");
    for (w, h) in [(1920u16, 1080u16), (2560, 1600)] {
        let (n, end) = per_row(w, h);
        assert!(n > narrow, "{w}: {n} tiles to a row");
        // The row runs to the window's side, less the page's margins.
        assert!(
            f32::from(w) - end <= 4.0 * MARGIN,
            "{w}: the row ends {} dp from the edge",
            f32::from(w) - end
        );
    }
}
