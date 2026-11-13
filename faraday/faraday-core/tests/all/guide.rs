//! The ? on a screen opens the guide at the section of what is on it: a
//! wallet being created opens Create a key, a new key opens Generate a
//! key, and Contents lists every page. On an extra-large window the guide
//! stands in a supporting pane beside the page (`docs/LAYOUT.md` §6),
//! follows the page, scrolls on its own and closes; below it, it is the
//! sheet.

use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// The place of the guide's page on show.
fn shown(app: &Faraday) -> &'static str {
    app.guide().shown
}

#[test]
fn the_question_mark_opens_the_guide_at_the_page_of_the_screen() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    crate::common::ckind(&mut app, 4);
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "create");
    assert!(
        app.guide().key.starts_with("create"),
        "at the section of the key of what is on screen"
    );
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);

    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::Guide);
    assert_eq!(shown(&app), "keygen");
}

#[test]
fn the_contents_lists_every_page_with_the_places_page_first() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Backups));
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "backups");
    assert!(!app.guide().contents);
    app.press(Action::GuideContents);
    frame(&mut app);
    assert!(app.guide().contents, "the contents is on show");
    assert!(
        app.offers(Action::GuideHere),
        "the place's page, first, under Here"
    );
    let rows: usize = faraday_guide::guide_groups()
        .iter()
        .map(|g| g.pages.len())
        .sum();
    assert_eq!(rows, faraday_guide::guide_places().len(), "every page");
}

#[test]
fn back_from_a_page_opened_in_the_contents_returns_to_the_contents_then_the_places_page() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Backups));
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    // The first row after Here is a page in the groups.
    app.press(Action::GuidePage(0));
    assert!(!app.guide().contents);
    let opened = shown(&app);
    assert_ne!(opened, "backups");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::Guide), "Back did not close it");
    assert!(app.guide().contents, "Back returned to the contents");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "backups", "then to the place's page");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None, "and then it closes");
}

#[test]
fn settings_opens_the_guide_at_the_settings_page() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Settings));
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "settings");
}

#[test]
fn the_vaults_screen_opens_the_vaults_page() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Vaults));
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "vaults");
}

/// The window, `width` × `height` at 160 dpi: one pixel a unit.
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

fn frame(app: &mut Faraday) {
    let _ = app.frame();
}

/// Make a new wallet's kinds, with the guide opened from its ?, on a window
/// `width` × `height`, drawn.
fn guide_on_create(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.press(Action::CreateWallet);
    app.press(Action::Guide);
    frame(&mut app);
    app
}

#[test]
fn at_1920_guide_opens_beside_the_page_whose_controls_stay_pressable() {
    let mut app = guide_on_create(1920, 1080);
    assert_eq!(app.sheet, None, "no sheet over the page");
    assert!(
        app.offers(Action::GuideClose),
        "the pane and its close button"
    );
    assert!(app.offers(Action::GuideTabGuide), "the pane's tabs");
    let first = shown(&app);
    assert_eq!(first, "create");
    // The page's kinds take a press beside the pane, none of them under it.
    let tab = app
        .hit_box(Action::GuideTabGuide)
        .expect("the pane's first tab");
    let kind = app.hit_box(Action::CKind(4)).expect("a kind on the page");
    assert!(
        u32::from(kind.0) + u32::from(kind.2) <= u32::from(tab.0),
        "the kind {kind:?} stands left of the pane's tab {tab:?}"
    );
    crate::common::ckind(&mut app, 4);
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "the pane stays open");
    assert_eq!(shown(&app), first, "and follows the multisig's step");
}

#[test]
fn at_1280_guide_opens_as_the_sheet() {
    let app = guide_on_create(1280, 800);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert!(!app.offers(Action::GuideClose), "no pane");
    assert!(!app.offers(Action::CKind(4)), "the page is under the sheet");
}

#[test]
fn a_pinned_pane_follows_the_step_and_the_screens_to_their_pages() {
    let mut app = guide_on_create(1920, 1080);
    // A multisig chosen: the pane shows the page of the screen.
    crate::common::ckind(&mut app, 4);
    frame(&mut app);
    assert_eq!(shown(&app), "create");
    assert!(app.offers(Action::GuideClose));
    app.press(Action::GuidePin);
    frame(&mut app);
    assert!(app.guide_pinned());
    // A new key: its page.
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    frame(&mut app);
    assert_eq!(shown(&app), "keygen");
    app.press(Action::Entry(None));
    frame(&mut app);
    assert_eq!(shown(&app), "entry", "the next screen's page");
    assert!(app.offers(Action::GuideClose));
}

#[test]
fn the_close_button_and_escape_put_the_pane_away() {
    let mut app = guide_on_create(1920, 1080);
    app.press(Action::GuideClose);
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose), "the pane is away");
    assert!(!app.offers(Action::GuideTabGuide));
    assert_eq!(app.sheet, None);
    assert!(app.offers(Action::CKind(4)), "the page as it was");
    // Opened again, Escape puts it away and leaves the page on screen.
    app.press(Action::Guide);
    frame(&mut app);
    assert!(app.offers(Action::GuideClose));
    let screen = app.screen;
    app.event(Event::Key(Key::Escape));
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose));
    assert_eq!(app.screen, screen, "Escape left the page where it was");
}

#[test]
fn a_long_page_scrolls_inside_the_pane_while_the_page_beside_stays() {
    let mut app = guide_on_create(1920, 600);
    let kind = app.hit_box(Action::CKind(4)).expect("a kind on the page");
    let tab = app.hit_box(Action::GuideTabGuide).expect("the pane's tab");
    // Over the pane's text, under its tabs.
    let (x, y) = (tab.0 + tab.2 / 2, tab.1 + 200);
    for _ in 0..10 {
        app.event(Event::Scroll { x, y, dy: 40 });
        frame(&mut app);
    }
    app.event(Event::ScrollEnd { x, y });
    for _ in 0..30 {
        frame(&mut app);
    }
    assert!(app.guide().scroll > 0.0, "the pane's text moved");
    assert_eq!(
        app.hit_box(Action::GuideTabGuide),
        Some(tab),
        "the pane's tabs hold their place"
    );
    assert_eq!(
        app.hit_box(Action::CKind(4)),
        Some(kind),
        "the page beside did not move"
    );
}

#[test]
fn resizing_across_1600_moves_guide_between_pane_and_sheet() {
    let mut app = guide_on_create(1920, 1080);
    let page = shown(&app);
    window(&mut app, 1280, 800);
    frame(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Guide), "the sheet below 1600");
    assert!(!app.offers(Action::GuideClose));
    assert_eq!(shown(&app), page, "on the page it showed");
    window(&mut app, 1920, 1080);
    frame(&mut app);
    assert_eq!(app.sheet, None, "the pane again from 1600");
    assert!(app.offers(Action::GuideClose));
    assert!(app.offers(Action::CKind(4)), "the page takes presses");
    assert_eq!(shown(&app), page);
}

/// Every line the guide draws on the Vaults page, scrolled from its top to its
/// foot, and whatever ran past a side on the way, at `width` × `height`.
fn vaults_page_read_to_its_foot(width: u16, height: u16) -> (Vec<String>, Vec<String>) {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.press(Action::Nav(faraday_core::Screen::Vaults));
    app.press(Action::Guide);
    frame(&mut app);
    assert_eq!(shown(&app), "vaults");
    let tab = app.hit_box(Action::GuideTabGuide).expect("the page's tab");
    let (x, y) = (tab.0 + tab.2 / 2, tab.1 + tab.3 + 80);
    let (mut lines, mut past) = (Vec::new(), Vec::new());
    let mut last = -1.0;
    for _ in 0..400 {
        let (p, t) = app.drawn_past_sides_and_text().plain();
        past.extend(p);
        lines.extend(t.into_iter().map(|(s, _)| s));
        if app.guide().scroll == last {
            break;
        }
        last = app.guide().scroll;
        app.event(Event::Scroll { x, y, dy: 120 });
        app.event(Event::ScrollEnd { x, y });
        for _ in 0..4 {
            frame(&mut app);
        }
    }
    (lines, past)
}

#[test]
fn the_vaults_page_read_to_its_foot_shows_each_section() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let (lines, past) = vaults_page_read_to_its_foot(w, h);
        let has = |s: &str| lines.iter().any(|l| l == s);
        assert!(
            has("The list") && has("Seal count") && has("Load from"),
            "{w}x{h}: every section of the page"
        );
        assert!(past.is_empty(), "{w}x{h}: nothing past a side: {past:?}");
    }
}

/// Started on an extra-large window and taken round the destinations,
/// The guide's pane is never there until the guide or Cryptography is pressed;
/// The guide then opens it beside the page, and Cryptography on its tab.
#[test]
fn on_a_wide_window_the_pane_opens_only_when_guide_or_cryptography_is_pressed() {
    use faraday_core::Screen;
    for (width, height) in [(1600, 900), (1920, 1080), (2560, 1440)] {
        let mut app = faraday_core::testkit::started();
        window(&mut app, width, height);
        frame(&mut app);
        let round = [
            Screen::Home,
            Screen::WalletsStart,
            Screen::Backups,
            Screen::Vaults,
            Screen::Files,
            Screen::Catalog,
            Screen::SpendBackup,
            Screen::Settings,
        ];
        for s in round {
            app.press(Action::Nav(s));
            frame(&mut app);
            assert!(
                !app.offers(Action::GuideClose),
                "no pane on {s:?} at {width}"
            );
            assert_eq!(app.sheet, None, "no Learn sheet on {s:?} at {width}");
        }
        app.press(Action::Guide);
        frame(&mut app);
        assert!(app.offers(Action::GuideClose), "Learn opened the pane");
        assert_eq!(shown(&app), "settings");
        app.press(Action::GuideClose);
        frame(&mut app);
        assert!(!app.offers(Action::GuideClose));
        app.press(Action::How);
        frame(&mut app);
        assert!(app.offers(Action::GuideClose), "Cryptography opened it");
        assert!(app.guide().how, "on its Cryptography tab");
    }
}

/// On Settings at the narrowest and a wide extra-large window the pane's
/// tabs Guide and Cryptography show whole: each takes a press side by
/// side, none past the close button's end.
#[test]
fn the_panes_tabs_on_settings_show_whole() {
    use faraday_core::Screen;
    for (width, height) in [(1600, 900), (2560, 1440)] {
        let mut app = faraday_core::testkit::started();
        window(&mut app, width, height);
        app.press(Action::Nav(Screen::Settings));
        app.press(Action::Guide);
        frame(&mut app);
        // The pane stands at the window's right edge.
        let end = u32::from(width);
        let tabs = [Action::GuideTabGuide, Action::GuideTabHow];
        let mut last = 0;
        for tab in tabs {
            let b = app.hit_box(tab).expect("every tab takes a press");
            assert!(u32::from(b.0) >= last, "the tabs side by side");
            last = u32::from(b.0) + u32::from(b.2);
            assert!(last <= end, "{tab:?} at {width} runs past the pane");
        }
    }
}

/// The Cryptography tab is offered only where the screen has blocks.
#[test]
fn the_cryptography_tab_is_offered_only_where_the_screen_has_blocks() {
    use faraday_core::Screen;
    let mut app = faraday_core::testkit::started();
    window(&mut app, 1920, 1080);
    let how = Action::GuideTabHow;
    for s in [
        Screen::Home,
        Screen::Settings,
        Screen::Vaults,
        Screen::Backups,
    ] {
        app.press(Action::Nav(s));
        app.press(Action::Guide);
        frame(&mut app);
        assert_eq!(app.offers(how), app.how_offered(), "on {s:?}");
        assert_eq!(
            app.offers(Action::GuideTabGuide),
            app.how_offered(),
            "on {s:?}"
        );
        app.press(Action::GuideClose);
    }
}

/// Unpinned, the pane goes when the person moves to another screen;
/// pinned, it stays open there; closed, it opens next time unpinned.
#[test]
fn the_pane_closes_at_another_screen_unless_pinned() {
    use faraday_core::Screen;
    let mut app = faraday_core::testkit::started();
    window(&mut app, 1920, 1080);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Guide);
    frame(&mut app);
    assert!(app.offers(Action::GuidePin), "the pane's pin");
    assert!(!app.guide_pinned());
    app.press(Action::Nav(Screen::Home));
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose), "closed on Home");
    assert_eq!(app.sheet, None);

    app.press(Action::Guide);
    frame(&mut app);
    app.press(Action::GuidePin);
    frame(&mut app);
    assert!(app.guide_pinned());
    app.press(Action::Nav(Screen::Settings));
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "still open on Settings");
    assert_eq!(shown(&app), "settings", "on Settings' page");
    app.press(Action::Nav(Screen::Vaults));
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "and on Vaults");

    app.press(Action::GuideClose);
    frame(&mut app);
    assert!(!app.guide_pinned(), "closing it takes the pin off");
    app.press(Action::Guide);
    frame(&mut app);
    assert!(app.offers(Action::GuideClose));
    assert!(!app.guide_pinned(), "it opens unpinned");
    app.press(Action::Nav(Screen::Home));
    frame(&mut app);
    assert!(
        !app.offers(Action::GuideClose),
        "and closes at the next screen"
    );
}

/// The guide pressed on the next screen, before a frame has put the last
/// screen's pane away, opens the pane there rather than closing it.
#[test]
fn guide_pressed_straight_after_moving_on_opens_the_pane_there() {
    use faraday_core::Screen;
    let mut app = faraday_core::testkit::started();
    window(&mut app, 1920, 1080);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Guide);
    frame(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Guide);
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "the pane on Vaults");
    assert_eq!(shown(&app), "vaults");
}
