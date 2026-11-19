//! A lock whose save fails keeps the process (`Faraday::not_kept`): what a
//! tool held typed must be gone after `lock_finish`, with no later
//! screen change to drop it.

#[cfg(test)]
use super::lock_finish;
use crate::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event, Key};

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn tile(app: &mut Faraday, name: &str) {
    app.press(Action::Nav(Screen::Catalog));
    let i = crate::catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .unwrap();
    app.press(Action::Catalog(i as u8));
}

#[test]
fn a_lock_forgets_the_krux_code_tools_text() {
    let mut app = testkit::started();
    tile(&mut app, "Krux code");
    app.press(Action::KefTool(crate::kef_tool::KefToolAction::Pick(
        crate::kef_tool::Pick::Type,
    )));
    type_in(&mut app, "a note for later");
    assert!(crate::kef_tool::state(&app).is_some_and(|k| k.holds_secret()));
    lock_finish(&mut app);
    assert!(crate::kef_tool::state(&app).is_none());
}

#[test]
fn a_lock_forgets_the_core_wallet_tools_passphrase() {
    let mut app = testkit::started();
    tile(&mut app, "Read a Bitcoin Core wallet");
    app.press(Action::CoreWallet(
        crate::core_wallet::tool::CoreWalletAction::Focus,
    ));
    type_in(&mut app, "wallet passphrase");
    assert!(crate::core_wallet::tool::state(&app).is_some());
    lock_finish(&mut app);
    assert!(crate::core_wallet::tool::state(&app).is_none());
}

#[test]
fn a_lock_forgets_what_was_typed_for_lightning() {
    let mut app = testkit::started();
    app.press(Action::Lightning);
    app.press(Action::LAezeed);
    type_in(&mut app, "above judge emerge");
    assert!(app.lightning().is_some());
    lock_finish(&mut app);
    assert!(app.lightning().is_none());
}

#[test]
fn a_lock_forgets_the_vanity_search() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    type_in(&mut app, &testkit::test_words("bacon"));
    app.press(Action::EntryAdd);
    app.press(Action::Vanity(crate::vanity::VanityAction::Open));
    assert!(app.vanity().is_some());
    lock_finish(&mut app);
    assert!(app.vanity().is_none());
}

#[test]
fn a_lock_forgets_the_sweep_sheets_typed_key() {
    let mut app = testkit::started();
    tile(&mut app, "Sweep a private key");
    type_in(
        &mut app,
        "L1aW4aubDFB7yfras2S1mN3bqg9nwySY8nkoLmJebSLD5BWv3ENZ",
    );
    assert!(app.seed_read.holds_secret());
    lock_finish(&mut app);
    assert!(!app.seed_read.holds_secret());
}

#[test]
fn a_lock_forgets_the_calculators_typed_text() {
    let mut app = testkit::started();
    app.press(Action::Tools);
    app.press(Action::TTool(0));
    type_in(&mut app, "typed for the calculator");
    assert!(testkit::tools(&app).is_some_and(|t| !t.typed.is_empty()));
    lock_finish(&mut app);
    assert!(testkit::tools(&app).is_none());
}

#[test]
fn a_lock_forgets_the_totp_setup_code_the_camera_carried() {
    let mut app = testkit::started();
    app.camera
        .carry(faraday_text::Text::of("otpauth://totp/x?secret=ABC"));
    lock_finish(&mut app);
    assert!(app.camera.take_carried().is_none());
}

#[test]
fn a_lock_forgets_the_text_typed_in_a_spend_backup_box() {
    let mut app = testkit::started();
    app.spend_backup
        .seeds_mut()
        .cosigners
        .push(faraday_text::Text::of("abandon abandon abandon"));
    lock_finish(&mut app);
    assert!(app.spend_backup.state().seeds.cosigners.is_empty());
}
