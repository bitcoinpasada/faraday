//! The ? on a screen opens the guide at the section of what is on it: a
//! wallet being created opens Create a key, a new key opens Generate a
//! key, and Contents lists every page. On an extra-large window the guide
//! stands in a supporting pane beside the page (`docs/LAYOUT.md` §6),
//! follows the page, scrolls on its own and closes; below it, it is the
//! sheet.

use faraday_core::testkit::Plain;
use faraday_core::{Action, Faraday, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

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
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "vault.kdbx".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
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

/// A control of the page the pane stands beside: Settings' Lock after, in the
/// first column near the top at any width and height (Appearance moves
/// between the columns and may stand below the fold). Pressing it opens no
/// sheet.
fn page_control(app: &Faraday) -> Action {
    Action::IdleLock(app.idle_lock_min())
}

/// Settings, with the guide opened from its ?, on a window `width` ×
/// `height`, drawn. (A flow on the container keeps the pane's place for
/// Your choices: the guide is a sheet beside it.)
fn guide_on_settings(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.press(Action::Nav(faraday_core::Screen::Settings));
    app.press(Action::Guide);
    frame(&mut app);
    app
}

#[test]
fn at_1920_guide_opens_beside_the_page_whose_controls_stay_pressable() {
    let mut app = guide_on_settings(1920, 1080);
    assert_eq!(app.sheet, None, "no sheet over the page");
    assert!(
        app.offers(Action::GuideClose),
        "the pane and its close button"
    );
    assert!(app.offers(Action::GuideTabGuide), "the pane's tabs");
    let first = shown(&app);
    assert_eq!(first, "settings");
    // The page's sections take a press beside the pane, none of them under it.
    let tab = app
        .hit_box(Action::GuideTabGuide)
        .expect("the pane's first tab");
    let kind = app
        .hit_box(page_control(&app))
        .expect("a section on the page");
    assert!(
        u32::from(kind.0) + u32::from(kind.2) <= u32::from(tab.0),
        "the kind {kind:?} stands left of the pane's tab {tab:?}"
    );
    app.press(page_control(&app));
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "the pane stays open");
}

#[test]
fn at_1280_guide_opens_as_the_sheet() {
    let app = guide_on_settings(1280, 800);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert!(!app.offers(Action::GuideClose), "no pane");
    assert!(
        !app.offers(page_control(&app)),
        "the page is under the sheet"
    );
}

#[test]
fn the_open_pane_follows_the_step_and_the_screens_to_their_pages() {
    let mut app = guide_on_settings(1920, 1080);
    // The pane shows the page of the screen.
    assert_eq!(shown(&app), "settings");
    assert!(app.offers(Action::GuideClose));
    // A new key is a flow on the container: Your choices keeps the pane's
    // place, so the guide is the sheet there and the pane is away.
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    crate::common::through_fade(&mut app);
    assert!(
        !app.offers(Action::GuideClose),
        "Your choices has the place"
    );
    app.press(Action::Back);
    crate::common::through_fade(&mut app);
    app.press(Action::Entry(None));
    frame(&mut app);
    assert_eq!(shown(&app), "entry", "the next screen's page");
    assert!(app.offers(Action::GuideClose));
}

#[test]
fn the_close_button_and_escape_put_the_pane_away() {
    let mut app = guide_on_settings(1920, 1080);
    app.press(Action::GuideClose);
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose), "the pane is away");
    assert!(!app.offers(Action::GuideTabGuide));
    assert_eq!(app.sheet, None);
    assert!(app.offers(page_control(&app)), "the page as it was");
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
    let mut app = guide_on_settings(1920, 600);
    let kind = app
        .hit_box(page_control(&app))
        .expect("a section on the page");
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
        app.hit_box(page_control(&app)),
        Some(kind),
        "the page beside did not move"
    );
}

#[test]
fn resizing_across_1600_moves_guide_between_pane_and_sheet() {
    let mut app = guide_on_settings(1920, 1080);
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
    assert!(app.offers(page_control(&app)), "the page takes presses");
    assert_eq!(shown(&app), page);
}

/// Every line the guide draws on the Vaults page, scrolled from its top to its
/// foot, and whatever ran past a side on the way, at `width` × `height`.
fn vaults_page_read_to_its_foot(width: u16, height: u16) -> (Vec<String>, Vec<String>) {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.storage(faraday_core::StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            "vault.kdbx".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
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
        app.press(Action::Guide);
        app.press(Action::GuideTabHow);
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

/// The pane stays open as the person moves from page to page, on each
/// one's page of the guide; on a screen that keeps its place it waits and
/// comes back after it; closed, it stays away.
#[test]
fn the_pane_stays_open_across_screens_until_it_is_closed() {
    use faraday_core::Screen;
    let mut app = guide_on_settings(1920, 1080);
    for (screen, page) in [(Screen::Tools, "tools"), (Screen::Files, "files")] {
        app.press(Action::Nav(screen));
        frame(&mut app);
        assert!(app.offers(Action::GuideClose), "still open on {screen:?}");
        assert_eq!(app.sheet, None);
        assert_eq!(shown(&app), page, "on that screen's page");
    }
    // Back up keeps the place for its board's lists.
    app.press(Action::Nav(Screen::Backup));
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose), "the board has the place");
    assert_eq!(app.sheet, None, "and nothing comes over it");
    app.press(Action::Nav(Screen::Settings));
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "back after it");
    assert_eq!(shown(&app), "settings");

    app.press(Action::GuideClose);
    frame(&mut app);
    app.press(Action::Nav(Screen::Tools));
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose), "closed, it stays away");
    assert_eq!(app.sheet, None);
}

/// The guide pressed on the next screen, before a frame has drawn it, is
/// the open pane pressed again: it puts the pane away.
#[test]
fn guide_pressed_straight_after_moving_on_puts_the_open_pane_away() {
    use faraday_core::Screen;
    let mut app = guide_on_settings(1920, 1080);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Guide);
    frame(&mut app);
    assert!(!app.offers(Action::GuideClose), "the pane is away");
    assert_eq!(app.sheet, None);
    app.press(Action::Guide);
    frame(&mut app);
    assert!(app.offers(Action::GuideClose), "the pane on Files");
    assert_eq!(shown(&app), "files");
}

/// The contents' row of the page at `place`, as the Action that opens it.
fn row_of(place: &str) -> Action {
    let at = faraday_guide::guide_groups()
        .iter()
        .flat_map(|g| g.pages.iter())
        .position(|p| p.place == place)
        .expect("a page in the contents");
    Action::GuidePage(at as u8)
}

#[test]
fn guide_from_home_with_nothing_loaded_opens_first_start() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Guide);
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "first-start");
}

#[test]
fn guide_from_home_with_something_loaded_opens_homes_page() {
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::inbox_file(&mut app, "notes.txt", b"hello".to_vec());
    app.press(Action::Guide);
    assert_eq!(shown(&app), "home");
}

#[test]
fn typing_a_headings_word_in_the_contents_leaves_the_page_holding_it() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    frame(&mut app);
    assert!(app.offers(row_of("home")));
    for c in "formatting".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    frame(&mut app);
    assert!(app.offers(row_of("files")), "the page with that heading");
    assert!(!app.offers(row_of("home")), "a page without it is gone");
}

#[test]
fn a_find_with_no_page_says_so_and_clear_brings_the_contents_back() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    for c in "zzzzqq".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    frame(&mut app);
    assert!(!app.offers(row_of("files")));
    assert!(app.offers(Action::GuideContents), "Clear is offered");
    app.press(Action::GuideContents);
    frame(&mut app);
    assert!(app.offers(row_of("files")));
}

#[test]
fn the_pis_keyboard_leaves_the_guides_search_and_its_matches_in_view() {
    let mut app = faraday_core::testkit::started();
    window(&mut app, 480, 640);
    app.press(Action::Guide);
    app.press(Action::GuideContents);
    let find = Action::TypeHere(faraday_core::edit::Here::GuideFind);
    app.press(find);
    for c in "files".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    frame(&mut app);
    let top = app.keyboard_top().expect("the keyboard is up");
    for (what, action) in [("the field", find), ("the first match", row_of("files"))] {
        let (_, y, _, h) = app
            .hit_box(action)
            .unwrap_or_else(|| panic!("{what} is offered"));
        assert!(
            y + h <= top,
            "{what} ({y} to {}) is above the keyboard's top ({top})",
            y + h
        );
    }
}

/// The index of the glossary word headed `word`.
fn word_index(word: &str) -> u8 {
    let i = faraday_guide::glossary()
        .iter()
        .position(|w| w.word == word)
        .unwrap();
    u8::try_from(i).unwrap()
}

#[test]
fn a_word_pressed_on_a_page_shows_its_definition_and_back_returns_to_the_page() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Guide);
    let before = shown(&app);
    app.press(Action::GuideWord(word_index("Quorum")));
    assert_eq!(shown(&app), "glossary");
    assert_eq!(app.guide().to, Some("glossary.quorum"));
    app.press(Action::Cancel);
    assert_eq!(shown(&app), before);
    assert_eq!(app.sheet, Some(Sheet::Guide));
}

#[test]
fn a_word_found_by_a_search_opens_the_guide_at_its_definition() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::GuideWord(word_index("Seal")));
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "glossary");
    assert_eq!(app.guide().key, "glossary.seal");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
}

/// Types `text` and presses the glossary word `word` the search offers.
fn find_and_press_word(mut app: Faraday, text: &str, word: &str) -> Faraday {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    let _ = app.frame();
    let (x, y) = app
        .where_offered(Action::GuideWord(word_index(word)))
        .unwrap_or_else(|| panic!("{word} is not offered for {text:?}"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
    app
}

#[test]
fn typing_a_glossary_word_in_tools_or_home_offers_it_and_a_press_opens_its_definition() {
    use faraday_core::Screen;
    let mut tools = faraday_core::testkit::started();
    tools.press(Action::Nav(Screen::Catalog));
    let app = find_and_press_word(tools, "quorum", "Quorum");
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(shown(&app), "glossary");
    assert_eq!(app.guide().key, "glossary.quorum");

    let mut home = faraday_core::testkit::started();
    home.press(Action::Nav(Screen::Home));
    let app = find_and_press_word(home, "quorum", "Quorum");
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert_eq!(app.guide().key, "glossary.quorum");
    assert!(faraday_core::testkit::home_find(&app).is_empty());
}

/// Where `action`'s control stands on `screen` of a window `width` wide,
/// before the guide is opened there and with it open.
fn beside_guide(width: u16, screen: faraday_core::Screen, action: Action) -> (u16, u16) {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, 1440);
    app.press(Action::Nav(screen));
    crate::common::through_fade(&mut app);
    // Twice: the bar's pills, which decide the tile's ways, are noted by
    // the frame before.
    frame(&mut app);
    frame(&mut app);
    let before = app.hit_box(action).expect("the control").0;
    app.press(Action::Guide);
    crate::common::through_fade(&mut app);
    frame(&mut app);
    frame(&mut app);
    let pane = app.hit_box(Action::GuideTabGuide).expect("the pane");
    let after = app.hit_box(action).expect("the control, beside the pane");
    assert!(after.0 + after.2 <= pane.0, "{after:?} left of {pane:?}");
    (before, after.0)
}

#[test]
fn a_pair_that_leaves_the_guide_its_room_does_not_move_as_it_opens() {
    use faraday_core::Screen;
    let (before, after) = beside_guide(3840, Screen::Catalog, Action::Catalog(0));
    assert_eq!(before, after);
}

#[test]
fn a_pair_moves_left_where_the_guide_needs_the_room() {
    use faraday_core::Screen;
    let (before, after) = beside_guide(1920, Screen::WalletsStart, Action::Entry(None));
    assert!(after < before, "{after} left of {before}");
}
