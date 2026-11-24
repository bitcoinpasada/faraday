//! Check an address' scanner on the wide layout stands in a pane beside
//! its card, the page live; the sheet stays where there is no room for the
//! pair. The scan is one state either way: what is read is the same, and
//! every way out drops the camera, the frame and what was read.

use super::address_page as kit;
use faraday_core::{Action, Ask, Faraday, Screen, Sheet};
use faraday_shell_api::{App, Command, Event, Key};

fn frame_event() -> Event {
    Event::CameraFrame {
        width: 64,
        height: 48,
        luma: vec![120; 64 * 48],
        chroma: None,
    }
}

/// On Check an address with its scanner open and a preview frame in.
fn scanning() -> Faraday {
    let mut app = kit::on_page();
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_some_and(|s| s.frame.is_some()));
    while app.poll_command().is_some() {}
    app
}

fn gone(app: &Faraday) {
    assert!(app.scan().is_none(), "the scan is still held");
    assert_eq!(app.sheet, None);
}

fn cameras_off(app: &mut Faraday) {
    let sent: Vec<Command> = std::iter::from_fn(|| app.poll_command()).collect();
    assert!(sent.contains(&Command::CameraOff), "the camera is still on");
}

#[test]
fn the_scanner_stands_in_the_pane_with_the_page_live() {
    let mut app = kit::on_page();
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Scan an address"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Cancel"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == "Check an address"),
        "page not live: {texts:?}"
    );
}

#[test]
fn the_same_scan_from_the_sheet_covers_the_page() {
    let mut app = kit::on_page();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Scan an address"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Check an address"), "{texts:?}");
}

#[test]
fn a_code_read_in_the_pane_and_in_the_sheet_gives_the_same_result() {
    let read = |from_page: bool| {
        let mut app = kit::on_page();
        if !from_page {
            app.press(Action::Nav(Screen::Catalog));
        }
        app.press(Action::ScanFor(Ask::Address));
        app.event(frame_event());
        app.event(Event::Scanned {
            bytes: kit::address(4).into_bytes(),
        });
        let _ = app.frame();
        gone(&app);
        assert_eq!(app.screen, Screen::CheckAddress);
        kit::said(&app)
    };
    let (pane, sheet) = (read(true), read(false));
    assert!(pane.contains("Receive · index 4"), "{pane}");
    assert_eq!(pane, sheet);
}

#[test]
fn cancel_drops_the_frame_and_the_camera() {
    let mut app = scanning();
    app.press(Action::Cancel);
    gone(&app);
    cameras_off(&mut app);
}

#[test]
fn escape_drops_the_frame_and_the_camera() {
    let mut app = scanning();
    app.event(Event::Key(Key::Escape));
    gone(&app);
    cameras_off(&mut app);
}

#[test]
fn back_drops_the_frame_and_the_camera() {
    let mut app = scanning();
    app.press(Action::Nav(Screen::Catalog));
    gone(&app);
    cameras_off(&mut app);
}

#[test]
fn leaving_the_step_by_the_sidebar_drops_the_frame_and_the_camera() {
    let mut app = scanning();
    app.press(Action::Nav(Screen::Home));
    gone(&app);
    cameras_off(&mut app);
}

#[test]
fn done_after_a_read_leaves_nothing_of_the_scan() {
    let mut app = scanning();
    app.event(Event::Scanned {
        bytes: kit::address(4).into_bytes(),
    });
    let _ = app.frame();
    gone(&app);
    app.press(Action::Nav(Screen::Catalog));
    gone(&app);
}

fn display(app: &mut Faraday, width: u16, height: u16, dpi: u16) {
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
}

fn cameras(app: &mut Faraday, n: usize) {
    app.storage(faraday_core::StorageEvent::Cameras(
        (0..n)
            .map(|i| {
                (
                    faraday_text::Text::of(&format!("cam{i}")),
                    faraday_text::Text::of(&format!("Camera {i}")),
                )
            })
            .collect(),
    ));
}

#[test]
fn a_phone_keeps_the_sheet_and_covers_the_page() {
    let mut app = kit::on_page();
    display(&mut app, 1081, 2400, 420);
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Scan an address"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Check an address"), "{texts:?}");
    app.press(Action::Cancel);
    gone(&app);
}

#[test]
fn two_cameras_keep_the_sheet_with_its_chooser() {
    let mut app = kit::on_page();
    cameras(&mut app, 2);
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Camera 1"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Check an address"), "{texts:?}");
}

#[test]
fn one_camera_keeps_the_pane() {
    let mut app = kit::on_page();
    cameras(&mut app, 1);
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Check an address"), "{texts:?}");
}

/// Locks as the person does, and as the shell's next tick settles it.
fn locked(app: &mut Faraday) {
    app.press(Action::LockUnsealed);
    faraday_core::testkit::settle(app);
    let _ = app.frame();
}

/// What a lock leaves of the scan: no frame, decoder, card or camera.
fn nothing_of_the_scan(app: &mut Faraday) {
    assert!(
        app.scan().is_none(),
        "the scan is still held after the lock"
    );
    assert_ne!(app.sheet, Some(Sheet::Scan));
    cameras_off(app);
}

#[test]
fn lock_leaves_no_scan_when_the_pane_stood() {
    let mut app = scanning();
    locked(&mut app);
    nothing_of_the_scan(&mut app);
}

#[test]
fn the_idle_lock_leaves_no_scan_when_the_pane_stood() {
    const MIN: u64 = 60_000;
    let mut app = scanning();
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 30 * MIN });
    assert!(app.restart_requested());
    faraday_core::testkit::settle(&mut app);
    nothing_of_the_scan(&mut app);
}

#[test]
fn the_camera_counts_as_on_while_the_pane_scans() {
    let mut app = scanning();
    assert!(!app.clean(), "a frame may picture a seed");
    app.press(Action::Cancel);
    let _ = app.frame();
    assert!(app.clean());
}

#[test]
fn a_short_wide_window_keeps_the_sheet() {
    let mut app = kit::on_page();
    display(&mut app, 1500, 500, 160);
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Scan an address"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Check an address"), "{texts:?}");
}

/// A tap on the way back the last frame drew in the app bar.
fn tap_bar_back(app: &mut Faraday) {
    let _ = app.frame();
    let (x, y) = app.back_offered().expect("the bar draws a way back");
    for phase in [
        faraday_shell_api::TouchPhase::Down,
        faraday_shell_api::TouchPhase::Up,
    ] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

#[test]
fn the_bars_back_ends_the_scan_and_leaves_the_page() {
    let mut app = scanning();
    tap_bar_back(&mut app);
    gone(&app);
    cameras_off(&mut app);
    assert_ne!(app.screen, Screen::CheckAddress);
}

#[test]
fn the_bars_back_with_a_files_row_shown_ends_the_scan_too() {
    let text = format!("{}\n{}\n", kit::address(2), kit::address(7));
    let mut app = kit::on_page_with(vec![("list.txt", text.into_bytes())]);
    app.press(Action::AddressCheck(
        faraday_core::address_check::AddressPress::File(1),
    ));
    app.press(Action::AddressCheck(
        faraday_core::address_check::AddressPress::Row(1),
    ));
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    while app.poll_command().is_some() {}
    tap_bar_back(&mut app);
    gone(&app);
    cameras_off(&mut app);
    // One step back: the list, not the Catalog.
    assert_eq!(app.screen, Screen::CheckAddress);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("2 of 2 addresses read")),
        "the list is not shown: {texts:?}"
    );
}

#[test]
fn the_bars_back_from_the_pane_returns_to_the_home_search_results() {
    let mut app = faraday_core::testkit::started();
    for c in "address".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
    let _ = app.frame();
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Check an address")
        .unwrap() as u8;
    app.press(Action::Catalog(tile));
    assert_eq!(app.screen, Screen::CheckAddress);
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    tap_bar_back(&mut app);
    gone(&app);
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(faraday_core::testkit::home_find(&app), "address");
}

#[test]
fn the_guide_beside_the_scan_takes_typing_and_a_word_and_the_scan_goes_on() {
    let mut app = kit::on_page();
    display(&mut app, 1920, 1080, 160);
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    let _ = app.frame();
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    app.press(Action::TypeHere(faraday_core::edit::Here::GuideFind));
    for c in "quorum".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    assert_eq!(app.guide().find, "quorum");
    let i = faraday_guide::glossary()
        .iter()
        .position(|w| w.word == "Quorum")
        .unwrap();
    app.press(Action::GuideWord(u8::try_from(i).unwrap()));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the word ended the scan");
    assert!(app.scan().is_some());
    assert_eq!(app.guide().to, Some("glossary.quorum"));
}

#[test]
fn practice_offers_its_sample_and_the_scanner_its_paste_in_the_pane() {
    let mut app = kit::on_page();
    faraday_core::testkit::into_demo(&mut app);
    app.samples = Some(faraday_core::testkit::demo_samples());
    app.set_hosted(true);
    app.clipboard = true;
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Paste"), "{texts:?}");
    let _ = app.frame();
    assert!(
        app.sample_area().aids().iter().any(
            |i| matches!(i, faraday_ui::demo::DemoItem::Button(_, a) if *a == Action::ScanSample)
        ),
        "the sample is not offered beside the pane"
    );
    assert!(app.where_offered(Action::ScanPaste).is_some());
}

#[test]
fn the_guide_beside_the_scan_scrolls_by_wheel() {
    let mut app = kit::on_page();
    display(&mut app, 1920, 1080, 160);
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    let _ = app.frame();
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    let (x, y) = app
        .where_offered(Action::GuidePage(0))
        .expect("the guide's contents stand beside the scan");
    let before = faraday_core::testkit::guide_scroll_y(&app);
    for _ in 0..3 {
        app.event(Event::Wheel { x, y, dy: 48 });
        app.settle();
    }
    assert!(faraday_core::testkit::guide_scroll_y(&app) > before);
    assert_eq!(app.sheet, Some(Sheet::Scan));
}

#[test]
fn escape_ends_the_scan_with_a_word_typed_in_the_guides_find() {
    let mut app = kit::on_page();
    display(&mut app, 1920, 1080, 160);
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    let _ = app.frame();
    app.press(Action::ScanFor(Ask::Address));
    app.event(frame_event());
    let _ = app.frame();
    app.press(Action::TypeHere(faraday_core::edit::Here::GuideFind));
    for c in "quorum".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.event(Event::Key(Key::Escape));
    gone(&app);
    cameras_off(&mut app);
    assert_eq!(app.guide().find, "quorum", "Escape cleared the Find first");
}
