//! How Faraday does it (`docs/CRYPTO-LEARN.md` §8): a screen that offers
//! a cryptographic action carries the button in its top app bar beside
//! the ?, it opens Learn on a first tab of the blocks of the screen's
//! actions, and on the Pi's panel the button is the ⋮ menu's first row
//! when the bar has no room. Checked with the sample block, which stands
//! on Catalog; a screen without a block has no button and no
//! tab. The new Learn pages open from their screens.

use crate::common::type_text;
use faraday_core::nav::{BarTool, MenuPress, Overflow};
use faraday_core::strings::EN;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
use faraday_ui::keyboard::OskPress;

fn app(width: u16, height: u16, dpi: u16, testnet: bool) -> Faraday {
    let mut app = Faraday::new();
    if testnet {
        testkit::move_to(&mut app.session, testkit::NET).unwrap();
    }
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
    app
}

fn go(app: &mut Faraday, screen: Screen) {
    app.press(Action::Nav(screen));
    app.settle();
    let _ = app.frame();
}

fn shows(app: &mut Faraday, text: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == text)
}

/// Whether the tab strip names the How tab and the block's own text is
/// on show.
fn how_tab_open(app: &mut Faraday) -> bool {
    app.learn().how && shows(app, "Sample action") && shows(app, EN.how_field_action)
}

#[test]
fn the_button_opens_learn_on_the_first_tab_at_every_size() {
    for (name, (w, h, dpi)) in [
        ("480x640", (480, 640, 160)),
        ("1280x800", (1280, 800, 160)),
        ("the Pi's panel", (480, 640, 286)),
    ] {
        let mut app = app(w, h, dpi, false);
        go(&mut app, Screen::Home);
        assert!(!app.offers(Action::How), "{name}: not on Home");
        go(&mut app, Screen::Catalog);
        if dpi == 286 {
            // On the Pi's panel the ⋮ holds it, first, with the ?.
            assert!(!app.offers(Action::How), "{name}: not on the bar");
            let held = overflow(&app).expect("a ⋮");
            assert!(held.how && held.help, "{name}: the ⋮ holds both");
            app.press(Action::NavMenu(MenuPress::Open(Some(held))));
            let _ = app.frame();
            app.press(Action::NavMenu(MenuPress::Tool(BarTool::How)));
        } else {
            assert!(app.offers(Action::How), "{name}: the button is offered");
            assert!(app.offers(Action::Learn), "{name}: beside the ?");
            // Immediately left of the ?, and the two do not overlap.
            let (hx, _, hw, _) = app.hit_box(Action::How).unwrap();
            let (lx, ..) = app.hit_box(Action::Learn).unwrap();
            assert!(hx + hw <= lx, "{name}: left of the ?, clear of it");
            app.press(Action::How);
        }
        let _ = app.frame();
        assert_eq!(app.sheet, Some(Sheet::Learn), "{name}");
        assert!(how_tab_open(&mut app), "{name}: the block is on show");
        assert!(shows(&mut app, EN.learn_tab_how), "{name}: the tab");
        // The ?'s tabs: the pages follow the How tab.
        app.press(Action::LearnPage(0));
        let _ = app.frame();
        assert!(!shows(&mut app, "Sample action"), "{name}: a page's tab");
        assert!(shows(&mut app, EN.learn_tab_how), "{name}: How stays first");
        app.press(Action::LearnPage(faraday_core::learn::HOW_TAB));
        let _ = app.frame();
        assert!(how_tab_open(&mut app), "{name}: back on the How tab");
    }
}

#[test]
fn the_question_mark_opens_a_page_with_the_how_tab_first() {
    let mut app = app(480, 640, 160, false);
    go(&mut app, Screen::Catalog);
    app.press(Action::Learn);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert!(!app.learn().how, "the page first");
    assert!(shows(&mut app, EN.learn_tab_how), "the tab is there");
    assert!(!shows(&mut app, "Sample action"));
}

#[test]
fn a_screen_without_a_block_has_no_button_and_no_tab() {
    let mut app = app(480, 640, 160, false);
    go(&mut app, Screen::SetTime);
    assert!(!app.offers(Action::How));
    app.press(Action::Learn);
    let _ = app.frame();
    assert!(!shows(&mut app, EN.learn_tab_how));
}

#[test]
fn every_line_of_a_block_is_reached_by_scrolling_on_the_pi() {
    let mut app = app(480, 640, 286, false);
    go(&mut app, Screen::Catalog);
    app.press(Action::How);
    let _ = app.frame();
    for first in ["Sample action", EN.how_field_standards] {
        assert!(shows(&mut app, first), "{first}");
    }
    let mut last = false;
    for _ in 0..40 {
        app.event(Event::Scroll {
            x: 240,
            y: 400,
            dy: 120,
        });
        app.settle();
        let _ = app.frame();
        if shows(&mut app, "no decision") {
            last = true;
            break;
        }
    }
    assert!(last, "the last of the Faraday's own calls is reached");
    assert!(shows(&mut app, EN.how_repository) || last);
}

/// The overflow the bar offers, if it has one.
fn overflow(app: &Faraday) -> Option<Overflow> {
    let mut out = None;
    for bits in 0u8..16 {
        let held = Overflow {
            how: bits & 1 != 0,
            help: bits & 2 != 0,
            keyboard: bits & 4 != 0,
            keyboard_up: false,
            eye: bits & 8 != 0,
        };
        if app.offers(Action::NavMenu(MenuPress::Open(Some(held)))) {
            out = Some(held);
        }
    }
    out
}

#[test]
fn on_the_pi_a_full_bar_puts_the_button_first_in_the_overflow_menu() {
    let mut app = app(480, 640, 286, true);
    go(&mut app, Screen::Entry);
    // The keyboard's button joins the tools on a page that types.
    app.press(Action::Osk(OskPress::Show));
    app.settle();
    let _ = app.frame();
    let held = overflow(&app).expect("the bar is full: a ⋮");
    assert!(held.how && held.help, "How goes in with the ?");
    assert!(!app.offers(Action::How), "no button on the bar");
    app.press(Action::NavMenu(MenuPress::Open(Some(held))));
    let _ = app.frame();
    let texts = app.drawn_texts();
    let row = texts
        .iter()
        .position(|t| t == EN.bar_menu_how)
        .expect("the row");
    let learn = texts
        .iter()
        .position(|t| t == EN.bar_menu_learn)
        .expect("Learn's row");
    assert!(row < learn, "above Learn");
    app.press(Action::NavMenu(MenuPress::Tool(BarTool::How)));
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert!(app.learn().how, "Learn opens on the How tab");
}

#[test]
fn the_supporting_pane_follows_the_button() {
    let mut app = app(1920, 1080, 160, false);
    go(&mut app, Screen::Catalog);
    app.press(Action::How);
    let _ = app.frame();
    assert!(app.offers(Action::LearnClose), "the pane, not the sheet");
    assert!(how_tab_open(&mut app));
    // On another tab, the button moves the pane to the How tab.
    app.press(Action::LearnPage(0));
    let _ = app.frame();
    assert!(!app.learn().how);
    app.press(Action::How);
    let _ = app.frame();
    assert!(app.offers(Action::LearnClose) && app.learn().how);
    // On the How tab, it puts the pane away.
    app.press(Action::How);
    let _ = app.frame();
    assert!(!app.offers(Action::LearnClose));
    // The ? opens the pane on the first page, with the How tab first.
    app.press(Action::Learn);
    let _ = app.frame();
    assert!(app.offers(Action::LearnClose) && !app.learn().how);
    assert!(shows(&mut app, EN.learn_tab_how));
}

/// The titles of the tabs Learn opens with on the screen on show.
fn tabs(app: &mut Faraday) -> Vec<&'static str> {
    app.press(Action::Learn);
    let _ = app.frame();
    let titles = app.learn().pages.iter().map(|p| p.title).collect();
    app.press(Action::Cancel);
    titles
}

#[test]
fn explore_and_child_seeds_list_the_pages_about_where_keys_are_kept_and_tools() {
    let mut app = app(1280, 800, 160, false);
    go(&mut app, Screen::Explore);
    let kept = faraday_learn::keys_kept_page().title;
    assert!(tabs(&mut app).contains(&kept), "Explore: {kept}");
    go(&mut app, Screen::Bip85);
    let tools = faraday_learn::pages::EN.tools.title;
    assert!(tabs(&mut app).contains(&tools), "Child seeds: {tools}");
}

#[test]
fn a_vaults_secure_boot_keys_list_the_page_about_them_and_it_opens() {
    let mut app = app(1280, 800, 160, false);
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.ofv")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_482_652,
    });
    testkit::vault_speed(&mut app, Some(180));
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..21 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        1,
        "{:?}",
        app.vaults().unlock_error
    );
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(5)));
    let page = faraday_learn::secure_boot_keys_page();
    assert!(tabs(&mut app).contains(&page.title), "{}", page.title);
    app.press(Action::Learn);
    app.press(Action::LearnPage(0));
    let _ = app.frame();
    assert!(shows(&mut app, page.title));
}
