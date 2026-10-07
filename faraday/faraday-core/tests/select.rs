//! Selecting what is typed: a second click on a field, or a drag across
//! it, selects all of it; Backspace then empties it and a typed character
//! replaces it. A wallet's new name is typed without a stray caret
//! character in it.

use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn touch(app: &mut Faraday, x: u16, y: u16, phase: TouchPhase) {
    app.event(Event::Touch { x, y, phase });
}

/// A click at (x, y) at time `ms`.
fn click(app: &mut Faraday, (x, y): (u16, u16), ms: u64) {
    app.event(Event::Tick { now_ms: ms });
    touch(app, x, y, TouchPhase::Down);
    touch(app, x, y, TouchPhase::Up);
    let _ = app.frame();
}

/// The Unlock screen with a long passphrase typed.
fn typed_passphrase() -> (Faraday, (u16, u16)) {
    let mut app = device(vec![kit_file("vault.ofv")]);
    app.press(Action::Vault(V::Open(0)));
    assert_eq!(app.screen, Screen::Unlock);
    type_text(&mut app, "a long passphrase typed wrong somewhere");
    let _ = app.frame();
    let at = app
        .where_offered(Action::Vault(V::FocusPassphrase))
        .expect("the passphrase field");
    (app, at)
}

#[test]
fn a_double_click_selects_the_passphrase_and_backspace_empties_it() {
    let (mut app, at) = typed_passphrase();
    click(&mut app, at, 1_000);
    click(&mut app, at, 1_200);
    assert!(app.select_all);
    app.event(Event::Key(Key::Backspace));
    assert!(app.vaults.passphrase.text.is_empty());
    type_text(&mut app, "next");
    assert_eq!(
        &*app.vaults.passphrase.text, "next",
        "typing goes on as before"
    );
}

#[test]
fn a_drag_across_the_field_selects_it_and_typing_replaces_it() {
    let (mut app, (x, y)) = typed_passphrase();
    app.event(Event::Tick { now_ms: 1_000 });
    touch(&mut app, x - 100, y, TouchPhase::Down);
    touch(&mut app, x + 100, y, TouchPhase::Move);
    touch(&mut app, x + 100, y, TouchPhase::Up);
    assert!(app.select_all);
    type_text(&mut app, "z");
    assert_eq!(&*app.vaults.passphrase.text, "z");
}

#[test]
fn two_clicks_far_apart_in_time_select_nothing() {
    let (mut app, at) = typed_passphrase();
    click(&mut app, at, 1_000);
    click(&mut app, at, 3_000);
    assert!(!app.select_all);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(
        &*app.vaults.passphrase.text, "a long passphrase typed wrong somewher",
        "Backspace takes one character"
    );
}

#[test]
fn a_wallets_name_is_selected_and_typed_over() {
    let mut app = device(vec![kit_file("spending-wallet.txt")]);
    app.press(Action::LoadWallet(0));
    assert_eq!(app.screen, Screen::Wallets);
    app.press(Action::Rename);
    let _ = app.frame();
    let at = app.where_offered(Action::Rename).expect("the name field");
    click(&mut app, at, 1_000);
    click(&mut app, at, 1_200);
    app.event(Event::Key(Key::Backspace));
    assert_eq!(app.renaming.as_deref(), Some(""));
    type_text(&mut app, "Groceries");
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.session.wallets[0].name, "Groceries");
}
