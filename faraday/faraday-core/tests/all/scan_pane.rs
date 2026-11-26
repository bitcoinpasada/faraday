//! Check an address' scanner on the wide layout stands in a pane beside
//! its card, the page live; the sheet stays where there is no room for the
//! pair. The scan is one state either way: what is read is the same, and
//! every way out drops the camera, the frame and what was read.

use super::address_page as kit;
use faraday_core::{Action, Ask, Faraday, Screen, Sheet};
use faraday_shell_api::{App, Command, Event, Key, TouchPhase};

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

// The sidebar's Scan (`docs/WORKSPACE.md` §4): the viewfinder in the
// session pane, the page beside it.

/// A session with a wallet, on `screen` at `width` by `height`.
fn beside(screen: Screen, (width, height): (u16, u16)) -> Faraday {
    let mut app = kit::on_page();
    display(&mut app, width, height, 160);
    app.press(Action::Nav(screen));
    let _ = app.frame();
    while app.poll_command().is_some() {}
    app
}

fn sidebar_scan(app: &mut Faraday) {
    app.press(Action::Scan);
    app.event(frame_event());
    let _ = app.frame();
}

/// Whether the sidebar beside the page takes a press: a frosted sheet
/// leaves nothing of the frame to press.
fn page_live(app: &mut Faraday) -> bool {
    let _ = app.frame();
    app.offers(Action::Nav(Screen::Catalog))
}

fn stick(app: &mut Faraday, on: bool) {
    let sticks = if on {
        vec![crate::common::Disk::empty("s1", "STICK").info()]
    } else {
        Vec::new()
    };
    app.storage(faraday_core::StorageEvent::Sticks(sticks));
    let _ = app.frame();
}

fn has_card(app: &Faraday) -> bool {
    app.scan().is_some_and(|s| s.card.is_some())
}

fn has(texts: &[faraday_text::Text], want: &str) -> bool {
    texts.iter().any(|t| t == want)
}

/// A setup code with no vault open: a read Home's Scan answers with its
/// card (`docs/SCANNING.md` §5.3).
fn read_totp(app: &mut Faraday) {
    app.event(Event::Scanned {
        bytes: b"otpauth://totp/Example:alice?secret=JBSWY3DPEHPK3PXP&issuer=Example".to_vec(),
    });
    let _ = app.frame();
}

#[test]
fn a_read_with_a_card_shows_it_in_the_pane() {
    for screen in [Screen::Home, Screen::Catalog, Screen::WalletsStart] {
        let mut app = beside(screen, (1920, 1080));
        sidebar_scan(&mut app);
        read_totp(&mut app);
        let texts = app.drawn_texts();
        assert_eq!(app.screen, screen);
        for want in ["Make a new vault", "Scan again", "Cancel"] {
            assert!(
                has(&texts, want),
                "{screen:?} {want} {:?}",
                texts.iter().map(|t| t.as_str()).collect::<Vec<_>>()
            );
        }
        assert!(!has(&texts, "This session"), "{screen:?} {texts:?}");
        assert!(page_live(&mut app), "{screen:?} the page is covered");
        app.press(Action::Cancel);
        let _ = app.frame();
        gone(&app);
        assert!(has(&app.drawn_texts(), "This session"), "{screen:?}");
    }
}

/// The sidebar's Scan in the pane, `kit::address(4)` read.
fn address_card(screen: Screen) -> Faraday {
    let mut app = beside(screen, (1920, 1080));
    sidebar_scan(&mut app);
    read_address(&mut app);
    app
}

fn read_address(app: &mut Faraday) {
    app.event(Event::Scanned {
        bytes: kit::address(4).into_bytes(),
    });
    let _ = app.frame();
}

/// The frame's text with no spaces: a whole address stands in groups of
/// four, on as many lines as it takes.
fn unspaced(app: &mut Faraday) -> String {
    let texts = app.drawn_texts();
    texts
        .iter()
        .flat_map(|t| t.chars())
        .filter(|c| !c.is_whitespace())
        .collect()
}

#[test]
fn a_read_address_shows_its_card_in_the_pane() {
    for screen in [Screen::Home, Screen::Catalog, Screen::WalletsStart] {
        let mut app = address_card(screen);
        assert_eq!(app.screen, screen);
        assert_eq!(app.sheet, Some(Sheet::Scan));
        let texts = app.drawn_texts();
        for want in ["Check this address", "Scan another", "Close"] {
            assert!(has(&texts, want), "{screen:?} {want} {texts:?}");
        }
        assert!(!has(&texts, "This session"), "{screen:?} {texts:?}");
        assert!(unspaced(&mut app).contains(&kit::address(4)), "{texts:?}");
        for way in [Action::ReadGo(0), Action::ReadAgain, Action::Cancel] {
            assert!(app.where_offered(way).is_some(), "{screen:?} {way:?}");
        }
    }
}

#[test]
fn check_this_address_opens_the_check_with_that_address() {
    let mut app = address_card(Screen::Home);
    app.press(Action::ReadGo(0));
    let _ = app.frame();
    gone(&app);
    assert_eq!(app.screen, Screen::CheckAddress);
    // As a small panel's sidebar Scan opens it, with no card between.
    let mut direct = beside(Screen::Home, (480, 640));
    sidebar_scan(&mut direct);
    read_address(&mut direct);
    gone(&direct);
    assert_eq!(direct.screen, Screen::CheckAddress);
    assert!(kit::said(&app).contains("Receive · index 4"));
    assert_eq!(kit::said(&app), kit::said(&direct));
}

#[test]
fn scan_another_turns_the_panes_camera_back_on_after_an_address() {
    let mut app = address_card(Screen::Home);
    while app.poll_command().is_some() {}
    app.press(Action::ReadAgain);
    let sent: Vec<Command> = std::iter::from_fn(|| app.poll_command()).collect();
    assert!(sent.contains(&Command::CameraOn), "the camera stayed off");
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(has(&texts, "Scan a QR code"), "{texts:?}");
    assert!(!has(&texts, "Check this address"), "{texts:?}");
    assert!(!unspaced(&mut app).contains(&kit::address(4)), "{texts:?}");
    read_address(&mut app);
    assert!(has(&app.drawn_texts(), "Check this address"));
}

#[test]
fn close_and_escape_on_the_address_card_put_the_session_pane_back() {
    for escape in [false, true] {
        let mut app = address_card(Screen::Home);
        if escape {
            app.event(Event::Key(Key::Escape));
        } else {
            app.press(Action::Cancel);
        }
        let _ = app.frame();
        gone(&app);
        assert_eq!(app.screen, Screen::Home);
        let texts = app.drawn_texts();
        assert!(has(&texts, "This session"), "{escape} {texts:?}");
        assert!(!unspaced(&mut app).contains(&kit::address(4)), "{texts:?}");
    }
}

#[test]
fn lock_leaves_no_address_card() {
    let mut app = address_card(Screen::Home);
    while app.poll_command().is_some() {}
    locked(&mut app);
    assert!(
        app.scan().is_none(),
        "the card is still held after the lock"
    );
    assert_ne!(app.sheet, Some(Sheet::Scan));
    let texts = app.drawn_texts();
    assert!(!has(&texts, "Check this address"), "{texts:?}");
    assert!(!unspaced(&mut app).contains(&kit::address(4)), "{texts:?}");
}

#[test]
fn scan_again_turns_the_panes_camera_back_on() {
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    read_totp(&mut app);
    app.press(Action::ReadAgain);
    app.event(frame_event());
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(has(&texts, "Scan a QR code"), "{texts:?}");
    assert!(!has(&texts, "Scan again"), "{texts:?}");
    assert!(!has(&texts, "This session"), "{texts:?}");
    assert!(page_live(&mut app), "the page is covered");
}

#[test]
fn the_cards_way_over_leaves_the_pane() {
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    read_totp(&mut app);
    app.press(Action::ReadGo(0));
    let _ = app.frame();
    assert_ne!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_none(), "the code is still held");
    assert_eq!(app.screen, Screen::CreateVault);
}

#[test]
fn leaving_the_page_ends_the_sidebar_scan() {
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    gone(&app);
    cameras_off(&mut app);
}

#[test]
fn a_small_panel_keeps_the_scan_sheet() {
    let mut app = beside(Screen::Home, (480, 640));
    sidebar_scan(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    let texts = app.drawn_texts();
    assert!(has(&texts, "Scan a QR code"), "{texts:?}");
    assert!(!has(&texts, "This session"), "{texts:?}");
}

#[test]
fn a_page_with_no_session_pane_keeps_the_scan_sheet() {
    let mut app = beside(Screen::Settings, (1920, 1080));
    let before = app.drawn_texts();
    sidebar_scan(&mut app);
    let texts = app.drawn_texts();
    assert!(has(&texts, "Scan a QR code"), "{texts:?}");
    assert!(!page_live(&mut app), "the sheet leaves the page live");
    let gone = before.iter().filter(|t| !texts.contains(t)).count();
    assert!(gone * 2 >= before.len(), "{texts:?}");
}

#[test]
fn lock_leaves_no_scan_when_the_session_pane_scans() {
    for read in [false, true] {
        let mut app = beside(Screen::Home, (1920, 1080));
        sidebar_scan(&mut app);
        if read {
            read_totp(&mut app);
            while app.poll_command().is_some() {}
        }
        locked(&mut app);
        nothing_of_the_scan(&mut app);
    }
}

/// A read card is held only under the sheets that wait over it: another
/// sheet the live page opens ends the scan at once.
#[test]
fn a_sheet_the_page_opens_ends_a_scan_with_a_card_up() {
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    read_totp(&mut app);
    assert!(has_card(&app));
    while app.poll_command().is_some() {}
    app.press(Action::PowerAsk);
    assert_eq!(app.sheet, Some(Sheet::Power));
    assert!(
        app.scan().is_none(),
        "the code is held under the Power sheet"
    );
    cameras_off(&mut app);
    app.press(Action::Cancel);
    let _ = app.frame();
    gone(&app);
    assert!(has(&app.drawn_texts(), "This session"));
}

/// A process that has held no secret is asked to pull the stick; one that
/// read a secret's card is asked to lock. The card waits under either. The
/// stick taken in for the pull opens Files, which has no session pane: the
/// camera comes back in the sheet there. The locked one stays in the pane.
#[test]
fn the_card_waits_under_scan_agains_sheet_and_the_camera_comes_back() {
    for (secret, sheet) in [(false, Sheet::Pull), (true, Sheet::Lock)] {
        let mut app = beside(Screen::Home, (1920, 1080));
        sidebar_scan(&mut app);
        if secret {
            read_totp(&mut app);
        } else {
            read_address(&mut app);
        }
        stick(&mut app, true);
        app.press(Action::ReadAgain);
        let _ = app.frame();
        assert_eq!(app.sheet, Some(sheet));
        assert!(has_card(&app), "{sheet:?} the card did not wait");
        while app.poll_command().is_some() {}
        stick(&mut app, false);
        let sent: Vec<Command> = std::iter::from_fn(|| app.poll_command()).collect();
        assert!(sent.contains(&Command::CameraOn), "{sheet:?} {sent:?}");
        assert_eq!(app.sheet, Some(Sheet::Scan));
        app.event(frame_event());
        let stays = if secret { Screen::Home } else { Screen::Files };
        assert_eq!(app.screen, stays);
        assert_eq!(page_live(&mut app), secret, "{sheet:?}");
        assert!(has(&app.drawn_texts(), "Scan a QR code"));
    }
}

#[test]
fn a_change_of_page_with_a_card_up_ends_the_sidebar_scan() {
    let mut app = address_card(Screen::Home);
    while app.poll_command().is_some() {}
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    gone(&app);
    cameras_off(&mut app);
    let texts = app.drawn_texts();
    assert!(!has(&texts, "Check this address"), "{texts:?}");
    assert!(!unspaced(&mut app).contains(&kit::address(4)), "{texts:?}");
}

#[test]
fn the_idle_warning_stands_over_the_pane_and_is_put_away_to_it() {
    const MIN: u64 = 60_000;
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    read_totp(&mut app);
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick {
        now_ms: 29 * MIN + 30_000,
    });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    assert!(has_card(&app), "the card did not wait under the warning");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x: 2, y: 2, phase });
    }
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(has_card(&app));
    assert!(
        page_live(&mut app),
        "the card did not come back to the pane"
    );
    assert!(has(&app.drawn_texts(), "Make a new vault"));
}

#[test]
fn a_stick_arriving_while_the_pane_scans_waits_for_the_scans_end() {
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    stick(&mut app, true);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert_eq!(app.screen, Screen::Home);
    assert!(app.scan().is_some());
    assert!(page_live(&mut app));
    while app.poll_command().is_some() {}
    app.press(Action::Cancel);
    let _ = app.frame();
    assert!(app.scan().is_none());
    cameras_off(&mut app);
    assert_eq!(app.screen, Screen::Files, "the stick waited for the scan");
}

#[test]
fn a_window_going_compact_sends_the_panes_scan_to_the_sheet() {
    let mut app = beside(Screen::Home, (1920, 1080));
    sidebar_scan(&mut app);
    assert!(page_live(&mut app));
    while app.poll_command().is_some() {}
    display(&mut app, 480, 640, 160);
    app.event(frame_event());
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_some());
    assert!(!page_live(&mut app), "the page is live beside no pane");
    assert!(has(&app.drawn_texts(), "Scan a QR code"));
    assert!(
        std::iter::from_fn(|| app.poll_command()).all(|c| c != Command::CameraOff),
        "the camera went off"
    );
}

/// Draws what the last press changed and says how many changes faded in
/// and how many were drawn at once for reduce motion.
fn fades(app: &mut Faraday) -> (u32, u32) {
    let _ = app.frame();
    let c = app.take_fade_counts();
    (c.started, c.reduced)
}

/// `docs/WORKSPACE.md` §6: the pane's content fades to the viewfinder and
/// back, at once with reduce motion.
#[test]
fn the_pane_fades_to_the_viewfinder_and_back() {
    for reduced in [false, true] {
        let mut app = beside(Screen::Home, (1920, 1080));
        if reduced {
            app.press(Action::ReduceMotion(true));
        }
        let _ = fades(&mut app);
        let want = if reduced { (0, 1) } else { (1, 0) };
        app.press(Action::Scan);
        assert_eq!(fades(&mut app), want, "to the viewfinder");
        app.press(Action::Cancel);
        assert_eq!(fades(&mut app), want, "back");
    }
}

// A stacked page (`docs/WORKSPACE.md` §4): This session stands over the
// page's tile, and the sidebar's Scan grows it to the viewfinder.

/// Where the line `text` is drawn, pixels (x, y, w, h).
fn line_box(app: &mut Faraday, text: &str) -> (i32, i32, i32, i32) {
    app.drawn_text_boxes()
        .into_iter()
        .find(|(t, _)| t == text)
        .unwrap_or_else(|| panic!("{text} is not drawn"))
        .1
}

/// The pane's box, settled, pixels (x, y, w, h).
fn pane(app: &mut Faraday) -> (i32, i32, i32, i32) {
    app.settle();
    let _ = app.frame();
    app.pane_place().expect("a pane")
}

/// A session with a wallet on `screen`, reached from Home (a page left
/// for a tool stays side by side until Home is shown), in the first of
/// these windows tall enough to stack it, with that height: Tools, the
/// longer page, needs more than Check.
fn stacked(screen: Screen) -> (Faraday, i32) {
    for height in [2000, 2600, 3200, 3800, 4400] {
        let mut app = beside(Screen::Home, (1920, height));
        app.press(Action::Nav(screen));
        // Wider than the pair's list pane: This session over the page.
        if pane(&mut app).2 > faraday_draw::tokens::LIST_PANE as i32 {
            return (app, i32::from(height));
        }
    }
    panic!("{screen:?} stacks in no window tried");
}

fn escape(app: &mut Faraday) {
    app.event(Event::Key(Key::Escape));
}

/// On `screen`, tall enough to stack: the sidebar's Scan takes This
/// session's place in a box as wide as the tile's columns, the page under
/// it, and `close` puts both back.
fn scan_over_a_stacked_page(screen: Screen, first_line: &str, close: fn(&mut Faraday)) {
    let (mut app, height) = stacked(screen);
    let strip = pane(&mut app);
    let line = line_box(&mut app, first_line);
    let (_, strip_y, _, strip_h) = strip;
    assert!(
        line.1 > strip_y + strip_h,
        "not stacked: {strip:?} {line:?}"
    );

    sidebar_scan(&mut app);
    let boxed = pane(&mut app);
    assert!(has(&app.drawn_texts(), "Scan a QR code"));
    assert!(!has(&app.drawn_texts(), "This session"));
    // A padding in from each side of the strip: the tile's columns.
    let pad = faraday_draw::tokens::PAD as i32;
    assert_eq!(boxed.0, strip.0 + pad, "{boxed:?} {strip:?}");
    assert_eq!(boxed.2, strip.2 - 2 * pad, "{boxed:?} {strip:?}");
    assert_eq!(boxed.1, strip.1);
    assert!(boxed.3 > strip.3, "the strip did not grow: {boxed:?}");
    assert!(boxed.1 + boxed.3 <= height, "the box is cut: {boxed:?}");
    let moved = line_box(&mut app, first_line);
    assert!(moved.1 > boxed.1 + boxed.3, "{moved:?} {boxed:?}");
    assert_eq!(moved.0, line.0);
    assert_eq!(moved.1 - line.1, boxed.3 - strip.3);

    close(&mut app);
    let _ = app.frame();
    gone(&app);
    assert_eq!(app.screen, screen);
    assert_eq!(pane(&mut app), strip);
    assert_eq!(line_box(&mut app, first_line), line);
    assert!(has(&app.drawn_texts(), "This session"));
}

#[test]
fn scan_over_the_stacked_check_page_takes_the_columns_and_moves_the_page_down() {
    scan_over_a_stacked_page(
        Screen::Check,
        faraday_core::strings::EN.check_group_wallet,
        escape,
    );
}

#[test]
fn scan_over_the_stacked_tools_page_takes_the_columns_and_moves_the_page_down() {
    scan_over_a_stacked_page(
        Screen::Catalog,
        faraday_core::strings::EN.catalog_common,
        escape,
    );
}

#[test]
fn cancel_puts_a_stacked_pages_strip_back() {
    scan_over_a_stacked_page(
        Screen::Check,
        faraday_core::strings::EN.check_group_wallet,
        |app| {
            app.press(Action::Cancel);
        },
    );
}

#[test]
fn a_read_over_a_stacked_page_shows_its_card_in_the_box() {
    let (mut app, _) = stacked(Screen::Check);
    let strip = pane(&mut app);
    sidebar_scan(&mut app);
    let finder = pane(&mut app);
    read_totp(&mut app);
    let card = pane(&mut app);
    assert!(has(&app.drawn_texts(), "Make a new vault"));
    assert_eq!((card.0, card.1, card.2), (finder.0, finder.1, finder.2));
    app.press(Action::Cancel);
    let _ = app.frame();
    gone(&app);
    assert_eq!(pane(&mut app), strip);
}
