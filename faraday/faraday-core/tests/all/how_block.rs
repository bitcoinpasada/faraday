//! How Faraday does it (`docs/CRYPTO-LEARN.md` §8): a screen that offers
//! a cryptographic action has the guide's Cryptography tab, which shows
//! the blocks of the screen's actions; the top app bar has no button
//! for it. Checked with the sample block, which stands
//! on Catalog; a screen without a block has no button and no
//! tab. The new guide pages open from their screens.

use crate::common::type_text;
use faraday_core::nav::{MenuPress, Overflow};
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
    app.guide().how && shows(app, "Sample action") && shows(app, EN.how_field_action)
}

/// Opens the guide on its Cryptography tab: the ?, then the tab.
fn open_how(app: &mut Faraday) {
    app.press(Action::Guide);
    let _ = app.frame();
    app.press(Action::GuideTabHow);
    let _ = app.frame();
}

#[test]
fn the_cryptography_tab_opens_the_blocks_at_every_size_and_the_bar_has_no_button() {
    for (name, (w, h, dpi)) in [
        ("480x640", (480, 640, 160)),
        ("1280x800", (1280, 800, 160)),
        ("the Pi's panel", (480, 640, 286)),
    ] {
        let mut app = app(w, h, dpi, false);
        go(&mut app, Screen::Catalog);
        assert!(
            app.offers(Action::Guide) || overflow(&app).is_some(),
            "{name}: the ?"
        );
        assert!(
            !app.drawn_texts().iter().any(|t| t == "Cryptography"),
            "{name}: no Cryptography text on the bar"
        );
        open_how(&mut app);
        assert_eq!(app.sheet, Some(Sheet::Guide), "{name}");
        assert!(how_tab_open(&mut app), "{name}: the block is on show");
        assert!(shows(&mut app, EN.guide_tab_how), "{name}: the tab");
        // The Guide tab shows the guide's page in its place.
        app.press(Action::GuideTabGuide);
        let _ = app.frame();
        assert!(!shows(&mut app, "Sample action"), "{name}: the guide tab");
        // Guide first, Cryptography beside it.
        let how = app
            .hit_box(Action::GuideTabHow)
            .expect("the Cryptography tab");
        let page = app.hit_box(Action::GuideTabGuide).expect("the Guide tab");
        assert!(page.0 < how.0, "{name}: Guide first");
        app.press(Action::GuideTabHow);
        let _ = app.frame();
        assert!(how_tab_open(&mut app), "{name}: back on the How tab");
    }
}

#[test]
fn the_question_mark_opens_the_guide_with_the_cryptography_tab_beside_it() {
    let mut app = app(480, 640, 160, false);
    go(&mut app, Screen::Catalog);
    app.press(Action::Guide);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Guide));
    assert!(!app.guide().how, "the page first");
    assert!(shows(&mut app, EN.guide_tab_how), "the tab is there");
    assert!(!shows(&mut app, "Sample action"));
}

#[test]
fn a_screen_without_a_block_has_no_tab() {
    let mut app = app(480, 640, 160, false);
    go(&mut app, Screen::SetTime);
    app.press(Action::Guide);
    let _ = app.frame();
    assert!(!shows(&mut app, EN.guide_tab_how));
}

#[test]
fn every_line_of_a_block_is_reached_by_scrolling_on_the_pi() {
    let mut app = app(480, 640, 286, false);
    go(&mut app, Screen::Catalog);
    open_how(&mut app);
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
            own: bits & 8 != 0,
            help: bits & 1 != 0,
            keyboard: bits & 2 != 0,
            keyboard_up: false,
            eye: bits & 4 != 0,
        };
        if app.offers(Action::NavMenu(MenuPress::Open(Some(held)))) {
            out = Some(held);
        }
    }
    out
}

#[test]
fn on_the_pi_the_overflow_menu_has_no_cryptography_row() {
    let mut app = app(480, 640, 286, true);
    go(&mut app, Screen::Entry);
    // The keyboard's button joins the tools on a page that types.
    app.press(Action::Osk(OskPress::Show));
    app.settle();
    let _ = app.frame();
    let held = overflow(&app).expect("a ⋮");
    app.press(Action::NavMenu(MenuPress::Open(Some(held))));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t == "Cryptography"),
        "no Cryptography row"
    );
}

#[test]
fn the_supporting_pane_opens_on_the_cryptography_tab() {
    let mut app = app(1920, 1080, 160, false);
    go(&mut app, Screen::Catalog);
    open_how(&mut app);
    assert!(app.offers(Action::GuideClose), "the pane, not the sheet");
    assert!(how_tab_open(&mut app));
    // On the Guide tab the blocks go and the page shows.
    app.press(Action::GuideTabGuide);
    let _ = app.frame();
    assert!(!app.guide().how);
    app.press(Action::GuideTabHow);
    let _ = app.frame();
    assert!(app.offers(Action::GuideClose) && app.guide().how);
    // The pane's close button puts it away.
    app.press(Action::GuideClose);
    let _ = app.frame();
    assert!(!app.offers(Action::GuideClose));
    // The ? opens the pane on the first page, with the How tab beside it.
    app.press(Action::Guide);
    let _ = app.frame();
    assert!(app.offers(Action::GuideClose) && !app.guide().how);
    assert!(shows(&mut app, EN.guide_tab_how));
}

/// The place of the guide's page the guide opens on the screen on show.
fn place(app: &mut Faraday) -> &'static str {
    app.press(Action::Guide);
    let _ = app.frame();
    let place = app.guide().shown;
    app.press(Action::Cancel);
    place
}

#[test]
fn explore_and_child_seeds_open_the_guide_at_their_own_pages() {
    let mut app = app(1280, 800, 160, false);
    go(&mut app, Screen::Explore);
    assert_eq!(place(&mut app), "explore");
    go(&mut app, Screen::Bip85);
    assert_eq!(place(&mut app), "bip85");
}

#[test]
fn a_vaults_secure_boot_keys_open_the_guide_at_the_vault_contents_page() {
    let mut app = app(1280, 800, 160, false);
    let vault = testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "vault.kdbx")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![vault]),
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
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
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
    assert_eq!(place(&mut app), "vault-contents");
    app.press(Action::Guide);
    let _ = app.frame();
    assert_eq!(app.guide().shown, "vault-contents");
}
