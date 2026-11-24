//! A tool opened from Tools lights Tools and one opened from Check lights
//! Check; opened from anywhere else it lights what it always did.

use faraday_core::check::{Press, Row};
use faraday_core::convert::ConvertAction;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::App;

fn lit(app: &Faraday) -> Screen {
    testkit::lit_destination(app)
}

/// Each tool Tools opens lights Tools.
#[test]
fn a_tool_opened_from_tools_lights_tools() {
    for action in [
        Action::KeyGen(None),
        Action::Entry(None),
        Action::Convert(ConvertAction::Open),
        Action::Explore,
        Action::Lightning,
        Action::Bip85,
        Action::Silent,
        Action::Tools,
        Action::OpenCodes,
    ] {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::Catalog));
        assert_eq!(lit(&app), Screen::Catalog);
        app.press(action);
        let _ = app.frame();
        assert_ne!(app.screen, Screen::Catalog, "{action:?} opened");
        assert_eq!(lit(&app), Screen::Catalog, "{action:?} from Tools");
    }
}

/// Each tool Check opens lights Check.
#[test]
fn a_tool_opened_from_check_lights_check() {
    for row in [
        Row::SeedCheck,
        Row::Multisig,
        Row::Child,
        Row::Dice,
        Row::Address,
        Row::Transaction,
        Row::Signed,
        Row::Xpub,
    ] {
        let mut app = testkit::started();
        app.press(Action::Nav(Screen::Check));
        app.press(Action::Check(Press::Row(row)));
        let _ = app.frame();
        assert_ne!(app.screen, Screen::Check, "{row:?} opened");
        assert_eq!(lit(&app), Screen::Check, "{row:?} from Check");
    }
}

/// A tool opened from a tool opened from Tools still lights Tools.
#[test]
fn a_tool_opened_from_a_tool_keeps_tools() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Bip85);
    let _ = app.frame();
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(lit(&app), Screen::Catalog);
}

/// A tool begun from a wallet's page keeps Wallets, and a wallet's own
/// flow does too.
#[test]
fn a_flow_begun_from_wallets_keeps_wallets() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(lit(&app), Screen::WalletsStart);

    let mut app = testkit::started();
    app.press(Action::Nav(Screen::WalletsStart));
    app.press(Action::CreateWallet);
    let _ = app.frame();
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(lit(&app), Screen::WalletsStart);
}

/// Opened from Home, a tool lights what it lights today: Wallets.
#[test]
fn a_tool_opened_from_home_lights_what_it_did() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::KeyGen(None));
    let _ = app.frame();
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(lit(&app), Screen::WalletsStart);
}
