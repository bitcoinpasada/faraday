//! The ? on a screen opens the Learn pages for what is on it: a
//! multisig being created opens Multisig, a new key opens Randomness, and
//! another tab shows the next page. On an extra-large window Learn stands
//! in a supporting pane beside the page (`docs/LAYOUT.md` §6), follows
//! the page, scrolls on its own and closes; below it, it is the sheet.

use faraday_core::{Action, Faraday, Sheet};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn shown(app: &Faraday) -> &'static str {
    app.learn().pages[app.learn().page].title
}

#[test]
fn the_question_mark_opens_the_pages_for_the_screen() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(4));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Multisig");
    app.press(Action::LearnPage(1));
    assert_eq!(shown(&app), "Xpubs and privacy");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);

    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    app.press(Action::Learn);
    assert_eq!(shown(&app), "Randomness");
}

#[test]
fn the_backups_list_names_the_page_on_a_backup_plans_check_after_backups() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Backups));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Backups");
    app.press(Action::LearnPage(1));
    assert_eq!(shown(&app), "A backup plan's check");
}

#[test]
fn settings_names_where_keys_are_kept() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Settings));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Where keys are kept");
}

#[test]
fn the_vaults_screen_names_the_vaults_page() {
    let mut app = faraday_core::testkit::started();
    app.press(Action::Nav(faraday_core::Screen::Vaults));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(shown(&app), "Vaults");
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

/// Make a new wallet's kinds, with Learn opened from its ?, on a window
/// `width` × `height`, drawn.
fn learn_on_create(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.press(Action::CreateWallet);
    app.press(Action::Learn);
    frame(&mut app);
    app
}

#[test]
fn at_1920_learn_opens_beside_the_page_whose_controls_stay_pressable() {
    let mut app = learn_on_create(1920, 1080);
    assert_eq!(app.sheet, None, "no sheet over the page");
    assert!(
        app.offers(Action::LearnClose),
        "the pane and its close button"
    );
    assert!(app.offers(Action::LearnPage(0)), "the pane's tabs");
    let first = shown(&app);
    // The page's kinds take a press beside the pane, none of them under it.
    let tab = app
        .hit_box(Action::LearnPage(0))
        .expect("the pane's first tab");
    let kind = app.hit_box(Action::CKind(4)).expect("a kind on the page");
    assert!(
        u32::from(kind.0) + u32::from(kind.2) <= u32::from(tab.0),
        "the kind {kind:?} stands left of the pane's tab {tab:?}"
    );
    app.press(Action::CKind(4));
    frame(&mut app);
    assert!(app.offers(Action::LearnClose), "the pane stays open");
    assert_ne!(shown(&app), first, "and shows the multisig's pages");
}

#[test]
fn at_1280_learn_opens_as_the_sheet() {
    let app = learn_on_create(1280, 800);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert!(!app.offers(Action::LearnClose), "no pane");
    assert!(!app.offers(Action::CKind(4)), "the page is under the sheet");
}

#[test]
fn a_pinned_pane_follows_the_step_and_the_screens_to_their_pages() {
    let mut app = learn_on_create(1920, 1080);
    // A multisig chosen: the pane shows the pages about it.
    app.press(Action::CKind(4));
    frame(&mut app);
    assert_eq!(shown(&app), "Multisig");
    assert!(app.offers(Action::LearnClose));
    app.press(Action::LearnPin);
    frame(&mut app);
    assert!(app.learn_pinned());
    // A new key: its pages; the tab on show stays where the next
    // screen's pages have it too.
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    frame(&mut app);
    assert_eq!(shown(&app), "Randomness");
    app.press(Action::LearnPage(1));
    let words = shown(&app);
    app.press(Action::Entry(None));
    frame(&mut app);
    assert_eq!(
        shown(&app),
        words,
        "the same page, on the next screen's tab"
    );
    assert_eq!(app.learn().page, 0);
    assert!(app.offers(Action::LearnClose));
}

#[test]
fn the_close_button_and_escape_put_the_pane_away() {
    let mut app = learn_on_create(1920, 1080);
    app.press(Action::LearnClose);
    frame(&mut app);
    assert!(!app.offers(Action::LearnClose), "the pane is away");
    assert!(!app.offers(Action::LearnPage(0)));
    assert_eq!(app.sheet, None);
    assert!(app.offers(Action::CKind(4)), "the page as it was");
    // Opened again, Escape puts it away and leaves the page on screen.
    app.press(Action::Learn);
    frame(&mut app);
    assert!(app.offers(Action::LearnClose));
    let screen = app.screen;
    app.event(Event::Key(Key::Escape));
    frame(&mut app);
    assert!(!app.offers(Action::LearnClose));
    assert_eq!(app.screen, screen, "Escape left the page where it was");
}

#[test]
fn a_long_page_scrolls_inside_the_pane_while_the_page_beside_stays() {
    let mut app = learn_on_create(1920, 600);
    let kind = app.hit_box(Action::CKind(4)).expect("a kind on the page");
    let tab = app.hit_box(Action::LearnPage(0)).expect("the pane's tab");
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
    assert!(app.learn().scroll > 0.0, "the pane's text moved");
    assert_eq!(
        app.hit_box(Action::LearnPage(0)),
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
fn resizing_across_1600_moves_learn_between_pane_and_sheet() {
    let mut app = learn_on_create(1920, 1080);
    app.press(Action::LearnPage(1));
    let page = shown(&app);
    window(&mut app, 1280, 800);
    frame(&mut app);
    assert_eq!(app.sheet, Some(Sheet::Learn), "the sheet below 1600");
    assert!(!app.offers(Action::LearnClose));
    assert_eq!(shown(&app), page, "on the page it showed");
    window(&mut app, 1920, 1080);
    frame(&mut app);
    assert_eq!(app.sheet, None, "the pane again from 1600");
    assert!(app.offers(Action::LearnClose));
    assert!(app.offers(Action::CKind(4)), "the page takes presses");
    assert_eq!(shown(&app), page);
}

/// Every line Learn draws on the Vaults page, scrolled from its top to its
/// foot, and whatever ran past a side on the way, at `width` × `height`.
fn vaults_page_read_to_its_foot(width: u16, height: u16) -> (Vec<String>, Vec<String>) {
    let mut app = faraday_core::testkit::started();
    window(&mut app, width, height);
    app.press(Action::Nav(faraday_core::Screen::Vaults));
    app.press(Action::Learn);
    frame(&mut app);
    assert_eq!(shown(&app), "Vaults");
    let tab = app.hit_box(Action::LearnPage(0)).expect("the page's tab");
    let (x, y) = (tab.0 + tab.2 / 2, tab.1 + tab.3 + 80);
    let (mut lines, mut past) = (Vec::new(), Vec::new());
    let mut last = -1.0;
    for _ in 0..400 {
        let (p, t) = app.drawn_past_sides_and_text();
        past.extend(p);
        lines.extend(t.into_iter().map(|(s, _)| s));
        if app.learn().scroll == last {
            break;
        }
        last = app.learn().scroll;
        app.event(Event::Scroll { x, y, dy: 120 });
        app.event(Event::ScrollEnd { x, y });
        for _ in 0..4 {
            frame(&mut app);
        }
    }
    (lines, past)
}

#[test]
fn the_vaults_page_goes_on_to_how_the_vault_is_built_and_passphrase_strength() {
    for (w, h) in [(1280, 800), (480, 640)] {
        let (lines, past) = vaults_page_read_to_its_foot(w, h);
        let has = |s: &str| lines.iter().any(|l| l == s);
        assert!(
            has("How the vault is built"),
            "{w}x{h}: the vault's account"
        );
        assert!(has("Passphrase strength"), "{w}x{h}: after it");
        assert!(
            has("The header") && has("Bytes"),
            "{w}x{h}: the header's table"
        );
        assert!(has("16 777 429"), "{w}x{h}: the largest vault's size");
        assert!(has("77.5"), "{w}x{h}: six words of the large list");
        assert!(
            !lines.iter().any(|l| l.starts_with("Planned, not built:")),
            "{w}x{h}: the vault's account has no work left to build"
        );
        assert!(past.is_empty(), "{w}x{h}: nothing past a side: {past:?}");
    }
}

/// Started on an extra-large window and taken round the destinations,
/// Learn's pane is never there until Learn or Cryptography is pressed;
/// Learn then opens it beside the page, and Cryptography on its tab.
#[test]
fn on_a_wide_window_the_pane_opens_only_when_learn_or_cryptography_is_pressed() {
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
            Screen::Family,
            Screen::Settings,
        ];
        for s in round {
            app.press(Action::Nav(s));
            frame(&mut app);
            assert!(
                !app.offers(Action::LearnClose),
                "no pane on {s:?} at {width}"
            );
            assert_eq!(app.sheet, None, "no Learn sheet on {s:?} at {width}");
        }
        app.press(Action::Learn);
        frame(&mut app);
        assert!(app.offers(Action::LearnClose), "Learn opened the pane");
        assert_eq!(shown(&app), "Where keys are kept");
        app.press(Action::LearnClose);
        frame(&mut app);
        assert!(!app.offers(Action::LearnClose));
        app.press(Action::How);
        frame(&mut app);
        assert!(app.offers(Action::LearnClose), "Cryptography opened it");
        assert!(app.learn().how, "on its Cryptography tab");
    }
}

/// On Settings at the narrowest and a wide extra-large window the pane's
/// tabs Cryptography, Key storage and Glossary show whole: each takes a
/// press side by side, none past the close button's end; every other
/// page's tab is reached from the row too.
#[test]
fn the_panes_tabs_on_settings_show_whole() {
    use faraday_core::Screen;
    for (width, height) in [(1600, 900), (2560, 1440)] {
        let mut app = faraday_core::testkit::started();
        window(&mut app, width, height);
        app.press(Action::Nav(Screen::Settings));
        app.press(Action::Learn);
        frame(&mut app);
        let close = app.hit_box(Action::LearnClose).expect("the pane");
        let end = u32::from(close.0) + u32::from(close.2);
        let tabs = [
            Action::LearnPage(faraday_core::learn::HOW_TAB),
            Action::LearnPage(0),
            Action::LearnPage(1),
        ];
        let mut last = 0;
        for tab in tabs {
            let b = app.hit_box(tab).expect("every tab takes a press");
            assert!(u32::from(b.0) >= last, "the tabs side by side");
            last = u32::from(b.0) + u32::from(b.2);
            assert!(last <= end, "{tab:?} at {width} runs past the pane");
        }
        for i in 0..app.learn().pages.len() {
            assert!(app.offers(Action::LearnPage(i as u8)), "page {i}'s tab");
        }
        assert_eq!(
            faraday_core::learn::tab_label(faraday_core::learn::keys_kept_page()),
            "Key storage"
        );
    }
}

/// Learn's pane offers **Spending, step by step** on the Spend tab only.
#[test]
fn the_pane_offers_spending_step_by_step_only_on_the_spend_tab() {
    use faraday_core::Screen;
    let mut app = faraday_core::testkit::started();
    window(&mut app, 1920, 1080);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Learn);
    frame(&mut app);
    assert!(app.offers(Action::LearnClose));
    assert!(!app.offers(Action::LearnSpend), "not on Settings");
    app.press(Action::LearnPin);
    app.press(Action::Nav(Screen::Family));
    frame(&mut app);
    assert!(app.offers(Action::LearnSpend), "on the Spend tab");
    app.press(Action::LearnSpend);
    frame(&mut app);
    assert!(
        !app.offers(Action::LearnClose),
        "Learn closed over the steps"
    );
    assert_eq!(app.screen, Screen::Family);
}

/// Unpinned, the pane goes when the person moves to another screen;
/// pinned, it stays open there; closed, it opens next time unpinned.
#[test]
fn the_pane_closes_at_another_screen_unless_pinned() {
    use faraday_core::Screen;
    let mut app = faraday_core::testkit::started();
    window(&mut app, 1920, 1080);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Learn);
    frame(&mut app);
    assert!(app.offers(Action::LearnPin), "the pane's pin");
    assert!(!app.learn_pinned());
    app.press(Action::Nav(Screen::Home));
    frame(&mut app);
    assert!(!app.offers(Action::LearnClose), "closed on Home");
    assert_eq!(app.sheet, None);

    app.press(Action::Learn);
    frame(&mut app);
    app.press(Action::LearnPin);
    frame(&mut app);
    assert!(app.learn_pinned());
    app.press(Action::Nav(Screen::Settings));
    frame(&mut app);
    assert!(app.offers(Action::LearnClose), "still open on Settings");
    assert_eq!(shown(&app), "Where keys are kept", "on Settings' pages");
    app.press(Action::Nav(Screen::Vaults));
    frame(&mut app);
    assert!(app.offers(Action::LearnClose), "and on Vaults");

    app.press(Action::LearnClose);
    frame(&mut app);
    assert!(!app.learn_pinned(), "closing it takes the pin off");
    app.press(Action::Learn);
    frame(&mut app);
    assert!(app.offers(Action::LearnClose));
    assert!(!app.learn_pinned(), "it opens unpinned");
    app.press(Action::Nav(Screen::Home));
    frame(&mut app);
    assert!(
        !app.offers(Action::LearnClose),
        "and closes at the next screen"
    );
}

/// Learn pressed on the next screen, before a frame has put the last
/// screen's pane away, opens the pane there rather than closing it.
#[test]
fn learn_pressed_straight_after_moving_on_opens_the_pane_there() {
    use faraday_core::Screen;
    let mut app = faraday_core::testkit::started();
    window(&mut app, 1920, 1080);
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::Learn);
    frame(&mut app);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Learn);
    frame(&mut app);
    assert!(app.offers(Action::LearnClose), "the pane on Vaults");
    assert_eq!(shown(&app), "Vaults");
}
