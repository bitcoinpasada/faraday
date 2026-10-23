//! Back lands exactly on the screen seen before the flow was entered
//! (F77): the same screen, the same wallet's card, the list scrolled as
//! far, the Learn sheet open on the same page. A press on the way back,
//! the mouse's back button and a flow's Cancel all land there, and the
//! way back is named after where it lands.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::catalog::{Go, TILES};
use faraday_core::create::NewKind;
use faraday_core::in_progress::Going;
use faraday_core::testkit::KindText;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
};

/// The app on a panel `width` × `height`, with a seed loaded and
/// `wallets` one-key wallets made from it, on Home.
fn with(wallets: usize, width: u16, height: u16) -> Faraday {
    let c = testkit::compute();
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
    for i in 0..4u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x30 + i; 32])));
    }
    let m = Mnemonic::from_entropy(Language::English, &[7; 16]).unwrap();
    let words: Vec<&str> = m
        .indices()
        .iter()
        .map(|&w| Language::English.words()[usize::from(w)])
        .collect();
    app.session
        .keys
        .add_words(&c, &words.join(" "), "Seed", None)
        .unwrap();
    let text = NewKind::NativeSegwit
        .key_text(&c, app.session.keys.master_for_tests(0))
        .unwrap();
    for a in 0..wallets {
        let d = format!("wpkh({}/<0;1>/*)", text.replace("/0h]", &format!("/{a}h]")));
        app.session
            .add_wallet(
                &c,
                &format!("Wallet {a}"),
                &d,
                "test",
                faraday_core::wallet::Origin::Named,
            )
            .unwrap_or_else(|e| panic!("{d}: {}", e.text()));
    }
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// The tile in Tools that goes to `go`.
fn tile(go: Go) -> Action {
    Action::Catalog(TILES.iter().position(|t| t.go == go).unwrap() as u8)
}

/// Tools, scrolled down the page.
fn tools_scrolled(app: &mut Faraday) -> f32 {
    app.press(Action::Nav(Screen::Catalog));
    let _ = app.frame();
    app.event(Event::Scroll {
        x: 600,
        y: 400,
        dy: 120,
    });
    let _ = app.frame();
    assert!(app.list_offset > 0.0, "Tools scrolls");
    app.list_offset
}

/// A tap on the way back the screen draws.
fn tap_back(app: &mut Faraday) {
    let _ = app.frame();
    let (x, y) = app.back_offered().expect("the screen draws a way back");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

#[test]
fn back_from_a_flow_opened_in_tools_lands_on_tools_scrolled_as_it_was() {
    let mut app = with(0, 1280, 800);
    let scrolled = tools_scrolled(&mut app);
    app.press(tile(Go::Create(NewKind::NativeSegwit)));
    assert_eq!(app.screen, Screen::Create);
    let _ = app.frame();
    assert!(
        app.drawn_texts().iter().any(|t| t == "Tools"),
        "the way back is named after Tools"
    );
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Catalog);
    assert_eq!(app.list_offset, scrolled);
}

#[test]
fn restore_from_tools_and_its_typed_seed_go_back_one_screen_at_a_time() {
    let mut app = with(0, 1280, 800);
    let scrolled = tools_scrolled(&mut app);
    app.press(tile(Go::Restore));
    assert_eq!(app.screen, Screen::Restore);
    app.press(Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Restore);
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Catalog);
    assert_eq!(app.list_offset, scrolled);
}

#[test]
fn back_from_a_flow_opened_on_a_wallets_card_lands_on_that_card() {
    let mut app = with(3, 1280, 800);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::OpenWallet(2));
    assert_eq!((app.screen, app.wallet), (Screen::Wallets, 2));
    app.press(Action::SignMessage);
    assert_eq!(app.screen, Screen::Message);
    let _ = app.frame();
    assert!(
        app.drawn_texts().iter().any(|t| t == "Wallet 2"),
        "the way back is named after the wallet"
    );
    tap_back(&mut app);
    assert_eq!((app.screen, app.wallet), (Screen::Wallets, 2));
}

#[test]
fn back_from_a_backup_opened_in_tools_lands_on_tools_not_the_wallet() {
    let mut app = with(1, 1280, 800);
    let scrolled = tools_scrolled(&mut app);
    app.press(tile(Go::Backup));
    assert_eq!(app.screen, Screen::Backup);
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Catalog);
    assert_eq!(app.list_offset, scrolled);
}

#[test]
fn back_from_spend_opened_in_learn_opens_learn_again_on_the_same_page() {
    let mut app = with(1, 1280, 800);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::OpenWallet(0));
    app.press(Action::Learn);
    assert_eq!(app.sheet, Some(Sheet::Learn));
    app.press(Action::LearnPage(1));
    let _ = app.frame();
    let page = app.learn().page;
    assert_eq!(page, 1);
    app.press(Action::LearnSpend);
    assert_eq!((app.screen, app.sheet), (Screen::Family, None));
    tap_back(&mut app);
    assert_eq!((app.screen, app.wallet), (Screen::Wallets, 0));
    assert_eq!(app.sheet, Some(Sheet::Learn));
    assert_eq!(app.learn().page, page);
}

#[test]
fn set_the_time_goes_back_to_settings_scrolled_as_it_was() {
    let mut app = with(0, 480, 640);
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    app.event(Event::Scroll {
        x: 240,
        y: 400,
        dy: 200,
    });
    let _ = app.frame();
    let scrolled = app.list_offset;
    assert!(scrolled > 0.0, "Settings scrolls on a small panel");
    app.press(Action::OpenSetTime);
    assert_eq!(app.screen, Screen::SetTime);
    app.event(Event::Scroll {
        x: 240,
        y: 400,
        dy: 60,
    });
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.list_offset, scrolled);
}

#[test]
fn a_flows_cancel_lands_where_the_flow_was_opened() {
    let mut app = with(0, 1280, 800);
    let scrolled = tools_scrolled(&mut app);
    app.press(tile(Go::Create(NewKind::NativeSegwit)));
    assert_eq!(app.screen, Screen::Create);
    let _ = app.frame();
    assert!(app.offers(Action::CancelFlow), "the flow offers Cancel");
    app.press(Action::CancelFlow);
    if app.sheet == Some(Sheet::CancelFlow(Going::Create)) {
        app.press(Action::CancelFlowNow);
    }
    assert!(app.create.is_none(), "Cancel ends the flow");
    assert_eq!(app.screen, Screen::Catalog);
    assert_eq!(app.list_offset, scrolled);
}

#[test]
fn the_mouses_back_button_lands_where_the_way_back_does() {
    let mut app = with(2, 1280, 800);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::OpenWallet(1));
    app.press(Action::SignMessage);
    assert_eq!(app.screen, Screen::Message);
    let _ = app.frame();
    app.event(Event::Back);
    assert_eq!((app.screen, app.wallet), (Screen::Wallets, 1));
}

#[test]
fn on_a_small_panel_the_bar_goes_back_to_tools_not_home() {
    let mut app = with(0, 480, 640);
    let scrolled = tools_scrolled(&mut app);
    app.press(tile(Go::Restore));
    assert_eq!(app.screen, Screen::Restore);
    let _ = app.frame();
    assert!(
        app.drawn_texts().iter().any(|t| t == "Tools"),
        "the bar names Tools"
    );
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Catalog);
    assert_eq!(app.list_offset, scrolled);
}

#[test]
fn a_flow_reached_again_from_the_sidebar_goes_back_to_where_it_was_resumed() {
    let mut app = with(1, 1280, 800);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::SignMessage);
    app.press(Action::MNext(faraday_core::mstep::ADDRESS));
    app.press(Action::Nav(Screen::Files));
    app.press(Action::Resume(Going::Message));
    assert_eq!(app.screen, Screen::Message);
    tap_back(&mut app);
    assert_eq!(app.screen, Screen::Files);
}

#[test]
fn back_from_a_key_explored_from_a_wallets_chart_lands_on_that_chart() {
    let mut app = with(2, 1280, 800);
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::OpenWallet(1));
    app.press(Action::Glance(true));
    assert!(app.glance);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::ExploreKey(fp));
    assert_eq!(app.screen, Screen::Explore);
    tap_back(&mut app);
    assert_eq!((app.screen, app.wallet), (Screen::Wallets, 1));
    assert!(app.glance, "the chart, as it was left");
}
